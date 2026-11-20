//! A planning service inside the test, on a socket in a temporary folder (never the user's runtime folder).
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use fs_modules_planning::Planning;
use fs_objects::{Store, TursoStore};
use fs_tasks_core::TaskRepo;
use tokio::sync::oneshot;

/// The actor the store itself is opened with; a connection must never write as this one.
pub const STORE_ACTOR: &str = "https://local.invalid/users/owner";

/// A running service; dropping it (or `stop`) ends it and removes the socket.
pub struct Service {
    pub socket: PathBuf,
    stop: Option<oneshot::Sender<()>>,
    pub dir: tempfile::TempDir,
}

impl Service {
    pub fn stop(&mut self) {
        drop(self.stop.take());
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Start a service on a fresh store; must run inside a `LocalSet`.
pub async fn start() -> Service {
    let dir = tempfile::Builder::new().prefix("pl").tempdir().unwrap();
    let store = TursoStore::open(dir.path().join("objects.db"))
        .await
        .unwrap();
    let store: Arc<dyn Store> = Arc::new(store);
    let repo = Arc::new(TaskRepo::new(store, STORE_ACTOR));
    let socket = dir.path().join("planning.sock");
    let bound = fs_rpc::bind_socket(&socket).await.unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    tokio::task::spawn_local(async move {
        let _ = fs_rpc::serve_callers(bound, Planning::new(repo, |_| {}), async {
            let _ = stopped.await;
        })
        .await;
    });
    Service {
        socket,
        stop: Some(stop),
        dir,
    }
}
