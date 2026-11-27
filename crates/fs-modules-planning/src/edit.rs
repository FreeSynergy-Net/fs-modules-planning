//! `task.edit`, the fourth write command of `FormSource.invoke` (split from `write.rs`).
//!
//! Pattern: Command (one function over the core) + Optimistic Lock (the task is written with the revision it was read with).

use fs_rpc::run_to_end;
use fs_tasks_core::{Revision, TaskItems, TaskRepo};

use crate::args::{Pairs, check_uid, text_args};
use crate::write::{
    BAD_ARGS, Done, Failure, INTERNAL, Ran, UNSUPPORTED, boxed, task_of, to_problem,
};

const EDIT_ARGS: [&str; 4] = ["title", "description", "due", "priority"];

/// `task.edit`: change the fields of the task `uid` that are given (`title`, `description`, `due`, `priority`), the others stay. An
/// empty `description`, `due` or `priority` removes it; a `title` cannot be empty. `due` and `priority` are judged by the core as for any
/// task (a local date-time, a whole number 0 to 9), a wrong value is `rejected` and nothing is written. The card stays where it is.
/// No compare-and-set on the `rev` yet: an `expect` is refused (never silently dropped).
pub(crate) async fn edit_task(
    repo: &TaskRepo,
    uid: &str,
    args: &Pairs,
    expect: Option<u64>,
) -> Result<Ran, Failure> {
    let mut got = text_args(args, &EDIT_ARGS)?;
    check_uid(uid)?;
    if got.is_empty() {
        let why = format!("no field to change (allowed: {})", EDIT_ARGS.join(", "));
        return Err(boxed(BAD_ARGS, why, 400));
    }
    if expect.is_some() {
        let why = "task.edit has no compare-and-set on the rev yet: `expect` is not taken";
        return Err(boxed(UNSUPPORTED, why, 501));
    }
    let entry = task_of(repo, uid).await?;
    let mut task = entry.task.clone();
    // an empty text removes an optional field; the title is never removed
    let some = |text: String| Some(text).filter(|t| !t.trim().is_empty());
    if let Some(title) = got.remove("title") {
        match some(title) {
            Some(t) => task.title = Some(t.trim().to_owned()),
            None => return Err(boxed(BAD_ARGS, "`title` is empty", 400)),
        }
    }
    if let Some(text) = got.remove("description") {
        task.description = some(text);
    }
    if let Some(text) = got.remove("due") {
        task.due = some(text).map(|t| t.trim().to_owned());
    }
    if let Some(text) = got.remove("priority") {
        task.priority =
            match some(text) {
                None => None,
                Some(t) => Some(t.trim().parse::<i32>().map_err(|_| {
                    boxed(BAD_ARGS, "`priority` is a whole number from 0 to 9", 400)
                })?),
            };
    }
    // written with the revision that was read: a change by someone else in between is a conflict (409), never a lost update
    let (repo, id, read) = (repo.clone(), entry.id, Revision::of(&entry));
    run_to_end(async move { repo.edit_checked(id, read, task).await })
        .await
        .map_err(|e| boxed(INTERNAL, e.to_string(), 500))?
        .map_err(to_problem)?;
    Ok(Ran::Done(Done {
        record: uid.to_owned(),
        text: "task-edited",
        head: None,
    }))
}
