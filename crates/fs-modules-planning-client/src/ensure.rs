//! `ensure_service`: one planning service per person.
//!
//! Pattern: Facade over fs-rpc `ensure_single` (as `ensure_service` of the layouts). The program file `fs-modules-planning` is looked
//! for next to the running program, then in `/usr/bin` — never on `PATH`: a planted file in a folder of the search path must not
//! become the server. The host calls it before its own runtime exists (it builds one and panics inside an async context).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use fs_interfaces::tasks_capnp::tasks;
use fs_rpc::{CommandStarter, Expected, SingleTimes, ensure_single};

use crate::model::{Error, Kind};

/// The name of the program of the service (the name it tells in `Control.version`).
pub const SERVICE_PROGRAM: &str = "fs-modules-planning";

/// The name of the socket program (`$XDG_RUNTIME_DIR/freesynergy/modules-planning.sock`), the same the service uses.
pub const SOCKET_PROGRAM: &str = "modules-planning";

/// The socket of the service for this person, if the system has a place for it.
#[must_use]
pub fn default_socket() -> Option<PathBuf> {
    fs_rpc::default_socket(SOCKET_PROGRAM)
}

/// Where the service program is: next to `running`, then in `usr_bin`; `None` if there is no regular file in either.
#[must_use]
pub fn find_service_program(running: &Path, usr_bin: &Path) -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "fs-modules-planning.exe"
    } else {
        SERVICE_PROGRAM
    };
    [
        running.parent().map(|d| d.join(name)),
        Some(usr_bin.join(name)),
    ]
    .into_iter()
    .flatten()
    .find(|p| std::fs::metadata(p).is_ok_and(|m| m.is_file()))
}

/// Make sure the service listens on `socket`: reach the one that runs, or start `<program> serve --socket S`.
///
/// # Errors
/// [`Kind::Unreachable`]; the text names the socket and the cause (`no-server` when there is no program file to start).
pub fn ensure_service(
    socket: &Path,
    expected: &Expected,
    program: &Path,
    times: &SingleTimes,
) -> Result<(), Error> {
    let unreachable =
        |cause: &str| Error::new(Kind::Unreachable, format!("{}: {cause}", socket.display()));
    let starter = CommandStarter::new(
        program,
        [
            OsStr::new("serve"),
            OsStr::new("--socket"),
            socket.as_os_str(),
        ],
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| unreachable("runtime"))?;
    tokio::task::LocalSet::new()
        .block_on(
            &runtime,
            ensure_single::<tasks::Client>(socket, expected, &starter, times, Some(program)),
        )
        .map(drop)
        .map_err(|e| unreachable(fs_rpc::RpcError::from(&e).code().as_str()))
}

/// [`ensure_service`] with the program file looked for next to `running` and in `/usr/bin`.
///
/// # Errors
/// As for [`ensure_service`].
pub fn ensure_service_found(
    socket: &Path,
    expected: &Expected,
    running: &Path,
    times: &SingleTimes,
) -> Result<(), Error> {
    let usr_bin = if cfg!(windows) {
        Path::new("")
    } else {
        Path::new("/usr/bin")
    };
    match find_service_program(running, usr_bin) {
        Some(program) => ensure_service(socket, expected, &program, times),
        None => Err(Error::new(
            Kind::Unreachable,
            format!("{}: no-server", socket.display()),
        )),
    }
}
