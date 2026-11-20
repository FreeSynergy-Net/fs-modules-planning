//! `fs-modules-planning serve`: the options, the data folder, the store, the serving.
//!
//! Pattern: Template method over fs-rpc `run_until_signal_callers` — registers the signals, binds the socket (a second server on
//! the same socket does not start: it is served by somebody else), opens the store, serves until a signal or `Control.stop`. The data
//! lives in `$XDG_DATA_HOME/freesynergy/modules/planning/` (0700). Returns the exit code.

use std::ffi::OsString;
use std::io;
use std::sync::Arc;

use fs_objects::{Store, TursoStore};
use fs_rpc::{ServerIdentity, ServerOptions, default_data_dir, prepare_data_dir, run_until_signal_callers};
use fs_tasks_core::TaskRepo;

use crate::root::Planning;

/// The name of the program for the socket (`$XDG_RUNTIME_DIR/freesynergy/modules-planning.sock`) and `Control.version`.
pub const PROGRAM: &str = "modules-planning";

/// The data folder below `$XDG_DATA_HOME/freesynergy/`.
const DATA_BELOW: &str = "modules/planning";

const USAGE: &str = "fs-modules-planning [serve] [--socket PATH] [--data DIR] [--instance DOMAIN]\nDefaults: $XDG_RUNTIME_DIR/freesynergy/modules-planning.sock, $XDG_DATA_HOME/freesynergy/modules/planning\nServes the contract tasks.capnp (items, boards); the actor of a request is the user of the connection.";

/// The words of `serve` with the data folder of this module where none was given; a leading `serve` is dropped.
fn words(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut args: Vec<OsString> = args.into_iter().collect();
    if args.first().is_some_and(|a| a == "serve") {
        args.remove(0);
    }
    if !args.iter().any(|a| a == "--data")
        && let Some(dir) = default_data_dir(DATA_BELOW)
    {
        args.push("--data".into());
        args.push(dir.into_os_string());
    }
    args
}

/// Run the server with `args`; the exit code: 0 after a signal or `--help`, 2 for a mistake in the words, 1 when serving failed.
#[must_use]
pub fn serve_command(args: impl IntoIterator<Item = OsString>) -> i32 {
    let options = match ServerOptions::parse(PROGRAM, words(args)) {
        Ok(Some(options)) => options,
        Ok(None) => {
            println!("{USAGE}");
            return 0;
        }
        Err(problem) => {
            eprintln!("fs-modules-planning: {problem}");
            return 2;
        }
    };
    let ServerOptions {
        socket,
        data,
        actor,
        instance,
        ..
    } = options;
    let identity = ServerIdentity::new("fs-modules-planning", env!("CARGO_PKG_VERSION"));
    let served = run_until_signal_callers(&socket, identity, || async move {
        let data = prepare_data_dir(&data)?;
        let store = TursoStore::open(data.join("objects.db"))
            .await
            .map_err(io::Error::other)?;
        let store: Arc<dyn Store> = Arc::new(store);
        // The actor of the store itself; every connection writes as its own caller (`Planning::bootstrap`).
        let mut repo = TaskRepo::new(store, actor);
        if let Some(instance) = instance {
            repo = repo.with_instance(instance);
        }
        Ok(Planning::new(Arc::new(repo), |why| {
            eprintln!("fs-modules-planning: connection dropped, the system did not say who it is: {why:?}");
        }))
    });
    match served {
        Ok(()) => 0,
        Err(e) if e.kind() == io::ErrorKind::Interrupted => {
            eprintln!("fs-modules-planning: {e}");
            0
        }
        Err(e) => {
            eprintln!("fs-modules-planning: {e}");
            1
        }
    }
}
