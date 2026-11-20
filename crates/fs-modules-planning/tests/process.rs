//! The program itself: it serves, keeps its data where the decision says, runs once, and says what it was asked.
//! Folders are temporary (XDG variables point there); every child is ended at the end of its test, also when the test fails.
//! Unix only: the folders need Unix permissions; the client tests (transport and board) are the ones that run on Windows too.
#![cfg(unix)]

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use fs_interfaces::tasks_capnp::tasks;
use tokio::task::LocalSet;

const BIN: &str = env!("CARGO_BIN_EXE_fs-modules-planning");

struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start(home: &Path, socket: &Path) -> Running {
    let child = Command::new(BIN)
        .args(["serve", "--socket"])
        .arg(socket)
        .env_clear()
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_RUNTIME_DIR", home.join("run"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    Running(child)
}

fn private_dirs() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::Builder::new().prefix("pm").tempdir().unwrap();
    for sub in ["data", "run"] {
        let dir = home.path().join(sub);
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    home
}

#[tokio::test(flavor = "current_thread")]
async fn it_serves_keeps_its_data_below_xdg_and_runs_once() {
    let home = private_dirs();
    let socket = home.path().join("run").join("p.sock");
    let first = start(home.path(), &socket);
    LocalSet::new()
        .run_until(async {
            let deadline = Instant::now() + Duration::from_secs(20);
            let connection = loop {
                match fs_rpc::connect::<tasks::Client>(&socket).await {
                    Ok(c) => break c,
                    Err(e) => {
                        assert!(
                            Instant::now() < deadline,
                            "the service did not come up: {e}"
                        );
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            };
            let answer = connection
                .client
                .capabilities_request()
                .send()
                .promise
                .await
                .unwrap();
            let ids: Vec<String> = answer
                .get()
                .unwrap()
                .get_capabilities()
                .unwrap()
                .get_ids()
                .unwrap()
                .iter()
                .map(|i| i.unwrap().to_string().unwrap())
                .collect();
            assert!(
                ids.iter().any(|i| i == "board.open") && ids.iter().any(|i| i == "card.move"),
                "{ids:?}"
            );
        })
        .await;
    let data = home.path().join("data/freesynergy/modules/planning");
    assert!(
        data.join("objects.db").exists(),
        "the data lives in {}",
        data.display()
    );

    // A second service on the same socket does not start: it is served by somebody else.
    let mut second = start(home.path(), &socket);
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = second.0.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "the second service kept running");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(
        !status.success(),
        "the second one must fail, it said {status}"
    );
    drop(first);
}

#[test]
fn it_says_what_it_does_and_what_was_wrong() {
    let run = |args: &[&str]| {
        Command::new(BIN)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let help = run(&["--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&help.stdout).contains("tasks.capnp"));
    let bad = run(&["--no-such-flag"]);
    assert_eq!(bad.status.code(), Some(2));
}
