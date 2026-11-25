//! The pump from the change stream of the store to the feed of the module: heads of units only, each with its boards, and a loss told
//! to the wrapper instead of dropped.
//!
//! Pattern: Observer (the store broadcasts, the pump is the subscriber); second event `FeedLoss` like `onError` in Reactive Streams.

use std::sync::Arc;

use fs_interfaces::module::{ChangeFeed, FeedLoss};
use fs_objects::Activity;
use fs_tasks_core::TaskRepo;
use tokio::sync::broadcast::{Receiver, error::RecvError};
use tokio::task::JoinHandle;

use crate::changes::seen;
use crate::core_conv::change_ref;
use crate::scope::scopes_of;

/// Hand every head activity of `rx` to `sink` as a `ChangeRef` with its `scopes` (board ids), and report a loss to `lost`.
///
/// `Lagged` is reported and the pump goes on (the wrapper ends the subscriptions, a new one listens again); `Closed` is reported once and
/// the pump ends.
///
/// # Panics
/// Outside a `LocalSet`: the pump is a local task (`tokio::task::spawn_local`).
pub(crate) fn pump(
    repo: Arc<TaskRepo>,
    mut rx: Receiver<Activity>,
    sink: ChangeFeed,
    lost: impl Fn(FeedLoss) + 'static,
) -> JoinHandle<()> {
    tokio::task::spawn_local(async move {
        loop {
            match rx.recv().await {
                // the head of a unit only (its members are not changes of their own)
                Ok(a) if a.unit.is_none() => {
                    let change = change_ref(&a, &seen(&repo, &a).await)
                        .in_scopes(scopes_of(&repo, &a).await);
                    sink(&change);
                }
                Ok(_) => {}
                Err(RecvError::Lagged(n)) => lost(FeedLoss::Lagged { missed: n }),
                Err(RecvError::Closed) => {
                    lost(FeedLoss::Closed);
                    break;
                }
            }
        }
    })
}
