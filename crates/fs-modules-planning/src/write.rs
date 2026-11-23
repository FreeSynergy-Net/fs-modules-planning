//! The three write commands of `FormSource.invoke`, by the capability key of the contract (`card.move`, `task.create`, `task.delete`),
//! and the rule of what the asking person `may` do.
//!
//! Pattern: Command (one method per command over the core) + Adapter (core errors as problems of the form source). The wire name of a
//! command is the key of `fs-interfaces schema/capabilities.txt`, never a name with the prefix of the module (the host adds that). Every
//! value of a command is text; an argument that is not named in `catalog/commands.json` or that comes twice is refused with its reason.
//! The core of this module has no rights of its own (the socket admits the person of the data folder, the actor is that person): `may` is
//! what the provider reports as supported (`TaskItems::capabilities`), nothing more is claimed.

use std::collections::BTreeMap;

use fs_interfaces::Capabilities;
use fs_interfaces::formsource_capnp::value;
use fs_objects::ObjectId;
use fs_rpc::{Problem, run_to_end};
use fs_tasks_core::{BoardEntry, Boards, Error, Task, TaskEntry, TaskItems, TaskRepo};

use crate::errors::rpc_error;

pub(crate) const UNKNOWN_SCOPE: &str = "form-source-unknown-scope";
pub(crate) const UNKNOWN_COMMAND: &str = "form-source-unknown-command";
pub(crate) const BAD_ARGS: &str = "form-source-bad-arguments";
const UNKNOWN_RECORD: &str = "form-source-unknown-record";
const REJECTED: &str = "form-source-rejected";
const CONFLICT: &str = "form-source-conflict";
pub(crate) const FORBIDDEN: &str = "form-source-forbidden";
const INTERNAL: &str = "form-source-internal";

pub(crate) const UID_MAX: usize = 255;

/// The commands a person may run on ONE record, and on the SET of records (a new card). Keys of the contract.
pub(crate) const RECORD_COMMANDS: [&str; 2] = ["card.move", "task.delete"];
pub(crate) const SET_COMMANDS: [&str; 1] = ["task.create"];

const MOVE_ARGS: [&str; 3] = ["board", "column", "index"];
const CREATE_ARGS: [&str; 4] = ["board", "column", "title", "index"];

/// What a command made or changed: the record (empty for a delete) and a short code for people.
pub(crate) struct Done {
    pub(crate) record: String,
    pub(crate) text: &'static str,
}

pub(crate) type Failure = Box<Problem>;

pub(crate) fn problem(kind: &str, detail: impl Into<String>, status: u16) -> Problem {
    Problem::new(kind, kind, detail, status)
}

pub(crate) fn boxed(kind: &str, detail: impl Into<String>, status: u16) -> Failure {
    Box::new(problem(kind, detail, status))
}

/// Of `wanted`, the commands the provider reports as supported: the answer of `may`.
pub(crate) fn may(caps: &Capabilities, wanted: &[&'static str]) -> Vec<&'static str> {
    wanted
        .iter()
        .copied()
        .filter(|key| caps.is_supported(key))
        .collect()
}

/// `invoke` refuses what `may` does not say, so the two never disagree.
pub(crate) fn permit(caps: &Capabilities, command: &str) -> Result<(), Failure> {
    let known = RECORD_COMMANDS.iter().chain(&SET_COMMANDS);
    if !known.clone().any(|k| *k == command) {
        return Err(boxed(
            UNKNOWN_COMMAND,
            format!("no command {command:?}"),
            404,
        ));
    }
    if caps.is_supported(command) {
        return Ok(());
    }
    Err(boxed(
        FORBIDDEN,
        format!("{command:?} is not possible here"),
        403,
    ))
}

/// The text values of a request by path. A path that is not in `allowed` or that comes twice is refused with the reason, never
/// silently dropped.
pub(crate) fn text_args(
    list: capnp::struct_list::Reader<'_, value::Owned>,
    allowed: &[&str],
) -> Result<BTreeMap<String, String>, Failure> {
    let mut out = BTreeMap::new();
    for v in list {
        let path = v
            .get_path()
            .and_then(|p| p.to_string().map_err(capnp::Error::from))
            .map_err(|e| bad_wire(&e))?;
        let text = v
            .get_text()
            .and_then(|t| t.to_string().map_err(capnp::Error::from))
            .map_err(|e| bad_wire(&e))?;
        if !allowed.contains(&path.as_str()) {
            let why = format!(
                "unknown argument {path:?} (allowed: {})",
                allowed.join(", ")
            );
            return Err(boxed(BAD_ARGS, why, 400));
        }
        if out.insert(path.clone(), text).is_some() {
            return Err(boxed(
                BAD_ARGS,
                format!("argument {path:?} given twice"),
                400,
            ));
        }
    }
    Ok(out)
}

fn take(got: &mut BTreeMap<String, String>, path: &str) -> Result<String, Failure> {
    got.remove(path)
        .ok_or_else(|| boxed(BAD_ARGS, format!("`{path}` is missing"), 400))
}

fn take_index(got: &mut BTreeMap<String, String>) -> Result<Option<usize>, Failure> {
    got.remove("index")
        .map(|t| {
            t.parse::<usize>()
                .map_err(|_| boxed(BAD_ARGS, "`index` is no number", 400))
        })
        .transpose()
}

fn check_uid(uid: &str) -> Result<(), Failure> {
    if uid.is_empty() || uid.len() > UID_MAX {
        let why = format!("`record` must be a uid of 1 to {UID_MAX} bytes");
        return Err(boxed(BAD_ARGS, why, 400));
    }
    Ok(())
}

/// The board for `scope`: the id wins; a name that more than one board has is refused (the caller uses the id).
pub(crate) fn board_named<'a>(
    boards: &'a [BoardEntry],
    scope: &str,
) -> Result<&'a BoardEntry, Failure> {
    if let Some(b) = boards.iter().find(|b| b.id.to_string() == scope) {
        return Ok(b);
    }
    let mut named = boards.iter().filter(|b| b.board.name == scope);
    match (named.next(), named.next()) {
        (Some(b), None) => Ok(b),
        (Some(_), Some(_)) => {
            let why = format!("more than one board is named {scope:?}, use its id");
            Err(boxed(UNKNOWN_SCOPE, why, 409))
        }
        _ => Err(boxed(UNKNOWN_SCOPE, format!("no board {scope:?}"), 404)),
    }
}

/// The task with this uid; a uid the core does not know is told, never created.
async fn task_of(repo: &TaskRepo, uid: &str) -> Result<TaskEntry, Failure> {
    check_uid(uid)?;
    repo.get_by_uid(uid)
        .await
        .map_err(to_problem)?
        .ok_or_else(|| boxed(UNKNOWN_RECORD, format!("no task {uid:?}"), 404))
}

/// Put the card `uid` into `column` at `index` COUNTED AMONG THE CARDS AS THE LAYOUT SHOWS THEM. The core counts only the cards placed by
/// hand (the others stand after them), so a plain `board_move_card` would put "second place" before cards the person sees above it.
/// So the order of every column is first written as shown (nothing moves on the screen), then the card is placed; a board changed by
/// someone else in between is a conflict (checked edit), never a lost write.
async fn place(
    repo: &TaskRepo,
    id: ObjectId,
    uid: &str,
    column: &str,
    index: Option<usize>,
) -> fs_tasks_core::Result<()> {
    let entry = repo.board_get(id).await?.ok_or(Error::NotFound(id))?;
    let layout = repo.board_layout(id).await?;
    let mut board = entry.board.clone();
    board.order = layout
        .columns
        .into_iter()
        .filter(|c| !c.cards.is_empty())
        .map(|c| (c.column.id, c.cards))
        .collect();
    board
        .move_card(uid, column, index)
        .map_err(Error::Rejected)?;
    repo.board_edit(id, entry.revision(), board).await?;
    Ok(())
}

/// `card.move`: the card `uid` into `column` of the board `board` at `index` (text of a number; none = the end).
pub(crate) async fn move_card(
    repo: &TaskRepo,
    uid: String,
    args: capnp::struct_list::Reader<'_, value::Owned>,
) -> Result<Done, Failure> {
    let mut got = text_args(args, &MOVE_ARGS)?;
    check_uid(&uid)?;
    let (board, column) = (take(&mut got, "board")?, take(&mut got, "column")?);
    let index = take_index(&mut got)?;
    let boards = repo.boards().await.map_err(to_problem)?;
    let id = board_named(&boards, &board)?.id;
    task_of(repo, &uid).await?;
    let (repo, moved) = (repo.clone(), uid.clone());
    run_to_end(async move { place(&repo, id, &moved, &column, index).await })
        .await
        .map_err(|e| boxed(INTERNAL, e.to_string(), 500))?
        .map_err(to_problem)?;
    Ok(Done {
        record: uid,
        text: "card-moved",
    })
}

/// `task.create`: a task with `title`, put into `column` of `board` at `index` (none = the end). Board and column are checked before
/// anything is written, so a refused command leaves nothing behind; only a board that vanishes between the check and the placing
/// (a race) leaves the new task standing in the column its progress names.
pub(crate) async fn create_task(
    repo: &TaskRepo,
    record: &str,
    args: capnp::struct_list::Reader<'_, value::Owned>,
) -> Result<Done, Failure> {
    let mut got = text_args(args, &CREATE_ARGS)?;
    if !record.is_empty() {
        return Err(boxed(
            BAD_ARGS,
            "`record` must be empty for task.create",
            400,
        ));
    }
    let title = take(&mut got, "title")?.trim().to_owned();
    if title.is_empty() {
        return Err(boxed(BAD_ARGS, "`title` is empty", 400));
    }
    let (board, column) = (take(&mut got, "board")?, take(&mut got, "column")?);
    let index = take_index(&mut got)?;
    let boards = repo.boards().await.map_err(to_problem)?;
    let entry = board_named(&boards, &board)?;
    if !entry.board.columns.iter().any(|c| c.id == column) {
        return Err(to_problem(Error::Rejected(format!("no column {column:?}"))));
    }
    let (id, repo) = (entry.id, repo.clone());
    let uid = run_to_end(async move {
        let uid = repo.create(Task::new(title)).await?.task.uid;
        place(&repo, id, &uid, &column, index).await?;
        Ok(uid)
    })
    .await
    .map_err(|e| boxed(INTERNAL, e.to_string(), 500))?
    .map_err(to_problem)?;
    Ok(Done {
        record: uid,
        text: "task-created",
    })
}

/// `task.delete`: the task `uid` and nothing else; it takes no argument.
pub(crate) async fn delete_task(
    repo: &TaskRepo,
    uid: &str,
    args: capnp::struct_list::Reader<'_, value::Owned>,
) -> Result<Done, Failure> {
    text_args(args, &[])?;
    let id = task_of(repo, uid).await?.id;
    let repo = repo.clone();
    run_to_end(async move { repo.delete(id).await })
        .await
        .map_err(|e| boxed(INTERNAL, e.to_string(), 500))?
        .map_err(to_problem)?;
    Ok(Done {
        record: String::new(),
        text: "task-deleted",
    })
}

/// A core error as the problem the caller can tell apart: a refusal (rejected, conflict, no such record) is not a break of the module.
#[allow(clippy::unnecessary_box_returns)] // the error type of the commands is boxed (a `Problem` is large)
pub(crate) fn to_problem(e: Error) -> Failure {
    let text = e.to_string();
    match &e {
        Error::Rejected(_) | Error::Invalid(_) | Error::Time(_) | Error::ICalendar(_) => {
            boxed(REJECTED, text, 422)
        }
        Error::Changed(_) | Error::Conflict(_) | Error::Exists(_) | Error::Cycle(_) => {
            boxed(CONFLICT, text, 409)
        }
        Error::NotFound(_) => boxed(UNKNOWN_RECORD, text, 404),
        _ => Box::new(
            Problem::from_error(&rpc_error(e)).unwrap_or_else(|| problem(INTERNAL, text, 500)),
        ),
    }
}

#[allow(clippy::unnecessary_box_returns)] // the error type of the argument readers is boxed (a `Problem` is large)
fn bad_wire(e: &capnp::Error) -> Failure {
    boxed(BAD_ARGS, e.to_string(), 400)
}
