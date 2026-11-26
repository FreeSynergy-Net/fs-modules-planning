//! Helpers of the person-owner tests: the real person binary, private folders, the end of what a test started.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub const BIN: &str = env!("CARGO_BIN_EXE_fs-modules-planning");

/// The service of the module person next to the program of the test (the place the program looks in first): built once from the
/// pinned source of `fs-person-service` (a dependency, so its binary is not built by `cargo test` itself).
pub fn person_binary() -> &'static Path {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
        let planning = Path::new(BIN);
        let dir = planning.parent().unwrap();
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let status = Command::new(cargo)
            .args([
                "build",
                "-p",
                "fs-person-service",
                "--bin",
                "fs-person-service",
            ])
            .arg("--target-dir")
            .arg(dir.parent().unwrap())
            .status()
            .unwrap();
        assert!(status.success(), "building fs-person-service failed");
        let built = dir.join("fs-person-service");
        assert!(built.is_file(), "{} is missing", built.display());
        built
    })
}

/// Private folders for one test (XDG variables point there) and the end of what it started.
pub struct Home(pub tempfile::TempDir);

impl Home {
    pub fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let home = tempfile::Builder::new().prefix("po").tempdir().unwrap();
        for sub in ["data", "run"] {
            let dir = home.path().join(sub);
            std::fs::create_dir(&dir).unwrap();
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        Self(home)
    }

    pub fn person_socket(&self) -> PathBuf {
        self.0.path().join("run/freesynergy/person.sock")
    }

    pub fn planning_socket(&self) -> PathBuf {
        self.0.path().join("run/freesynergy/planning.sock")
    }

    pub fn command(&self) -> Command {
        person_binary();
        let mut command = Command::new(BIN);
        command
            .arg("serve")
            .env_clear()
            .env("HOME", self.0.path())
            .env("XDG_DATA_HOME", self.0.path().join("data"))
            .env("XDG_RUNTIME_DIR", self.0.path().join("run"))
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        command
    }
}

impl Drop for Home {
    /// Stop the person service the program started and wait for its end (a folder removed under it would hang it).
    fn drop(&mut self) {
        let socket = self.person_socket();
        let _ =
            std::thread::spawn(move || local(async move { fs_rpc::stop_server(&socket).await }))
                .join();
        let want = self.0.path().to_string_lossy().into_owned();
        let running = || {
            std::fs::read_dir("/proc").unwrap().flatten().any(|e| {
                let line = std::fs::read(e.path().join("cmdline")).unwrap_or_default();
                String::from_utf8_lossy(&line).contains(&want)
            })
        };
        let until = Instant::now() + Duration::from_secs(8);
        while running() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

pub fn local<T>(work: impl std::future::Future<Output = T>) -> T {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    tokio::task::LocalSet::new().block_on(&runtime, work)
}
