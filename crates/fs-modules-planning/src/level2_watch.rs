//! The push of `watch`: every head change in the scope of a board goes to the sink of the caller.
//!
//! Pattern: Observer (the subscription is a capability; dropping it ends the task that pushes, as `Common.Subscription` says).

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::subscription;
use fs_interfaces::formsource_capnp::change_sink;
use fs_objects::Activity;

use crate::scope::Scope;
use tokio::sync::broadcast::error::RecvError;

use crate::level2::{FEED_ENDED, seen, write_change, write_problem};
use crate::write::problem as new_problem;

/// The subscription of one `watch`: dropping the capability ends the task that pushes.
struct Subscription(tokio::task::JoinHandle<()>);

impl subscription::Server for Subscription {}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Push every head change in `scope` to `sink` until the receiver is gone, too slow or the subscription dropped.
pub(crate) fn start_watch(
    repo: std::sync::Arc<fs_tasks_core::TaskRepo>,
    board: Option<Scope>,
    sink: change_sink::Client,
    mut feed: tokio::sync::broadcast::Receiver<Activity>,
) -> subscription::Client {
    let task = tokio::task::spawn_local(async move {
        let ended = |why: &str| {
            let mut request = sink.ended_request();
            write_problem(
                request.get().init_problem(),
                &new_problem(FEED_ENDED, why, 410),
            );
            request.send().promise
        };
        loop {
            match feed.recv().await {
                Ok(a) if a.unit.is_none() && in_scope(&repo, board, &a).await => {
                    let seen = seen(&repo, &a).await;
                    let mut request = sink.changed_request();
                    write_change(request.get().init_change(), &a, &seen);
                    if request.send().await.is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(RecvError::Lagged(n)) => {
                    let _ = ended(&format!("the receiver was too slow, {n} changes missed")).await;
                    break;
                }
                Err(RecvError::Closed) => {
                    let _ = ended("the store was closed").await;
                    break;
                }
            }
        }
    });
    new_client(Subscription(task))
}

/// Whether the change is in the scope; no scope = every change.
async fn in_scope(repo: &fs_tasks_core::TaskRepo, scope: Option<Scope>, a: &Activity) -> bool {
    match scope {
        Some(scope) => scope.holds(repo, a).await,
        None => true,
    }
}
