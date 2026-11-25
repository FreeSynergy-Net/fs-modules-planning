//! The commands of `catalog/commands.json` by their key, on a handle that already writes as the connection.
//!
//! Pattern: Command (a key picks one command over the core; Factory Method by key). Both ways in use it: the level 1 service
//! (`FormSourceImpl::run`, after its own `permit`) and `PlanningCore::apply` (the rights were asked by `Guarded` before).

use fs_tasks_core::TaskRepo;

use crate::args::Pairs;
use crate::write::{Failure, Ran, UNKNOWN_COMMAND, boxed, create_task, delete_task, move_card};

/// Run `command` for `record` with `args` (read strictly here); `expect` is the `rev` the caller saw. Rights are not asked here.
pub(crate) async fn dispatch(
    repo: &TaskRepo,
    command: &str,
    record: String,
    args: &Pairs,
    expect: Option<u64>,
) -> Result<Ran, Failure> {
    match command {
        "card.move" => move_card(repo, record, args, expect).await,
        "task.create" => create_task(repo, &record, args).await,
        "task.delete" => delete_task(repo, &record, args, expect).await,
        other => Err(boxed(UNKNOWN_COMMAND, format!("no command {other:?}"), 404)),
    }
}
