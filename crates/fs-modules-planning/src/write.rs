//! The three write commands of `FormSource.invoke`, by the capability key of the contract (`card.move`, `task.create`, `task.delete`),
//! and the rule of what the asking person `may` do.
//!
//! Pattern: Command (one method per command over the core) + Adapter (core errors as problems of the form source). The wire name of a
//! command is the key of `fs-interfaces schema/capabilities.txt`, never a name with the prefix of the module (the host adds that). Every
//! value of a command is text; an argument that is not named in `catalog/commands.json` or that comes twice is refused with its reason.
//! Rights, honestly: the core has no rights per person. Only the owner is admitted at the socket (fs-rpc: Unix mode 0600, Windows
//! `only_me` and no remote clients), and he may do all with his data; `invoke` gives no right that `Tasks` on the same socket does not
//! give already (`Items.create`/`delete`, `Boards.moveCard`). So `may` ("this person may" in the contract) is what the provider reports
//! as supported (`TaskItems::capabilities`), nothing more is claimed; rights per person (groups, several users) come later. A command
//! the provider cannot do is `form-source-forbidden` (403): `formsource.capnp` names no kinds for `invoke`, and the host (SW-5) tells them apart only later, so no new kind `unsupported` (501) is started here.

use fs_interfaces::Capabilities;
use fs_interfaces::formsource_capnp::value;
use fs_objects::{Activity, Outcome};
use fs_rpc::{Problem, run_to_end};
use fs_tasks_core::{BoardEntry, Boards, Error, TaskEntry, TaskItems, TaskRepo};

use crate::args::{check_uid, take, take_neighbour, text_args};
use crate::errors::rpc_error;
use crate::place::{create_placed, place};

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

const MOVE_ARGS: [&str; 4] = ["board", "column", "after", "before"];
const CREATE_ARGS: [&str; 5] = ["board", "column", "title", "after", "before"];

/// What a command made or changed: the record (empty for a delete), a short code for people, and the head of the unit of change it wrote.
pub(crate) struct Done {
    pub(crate) record: String,
    pub(crate) text: &'static str,
    pub(crate) head: Option<Box<Activity>>,
}

/// What a command came to: it was done, or the `expect` of the caller did not match (nothing was written): the last change of the
/// record and the paths that changed.
pub(crate) enum Ran {
    Done(Done),
    Clash(Box<Activity>, Vec<String>),
}

impl Ran {
    /// The answer of the store for the command `record`/`text`; a unit that is gone or cannot be taken back is a refusal.
    fn of(outcome: Outcome, record: String, text: &'static str) -> Result<Self, Failure> {
        match outcome {
            Outcome::Done(done) => Ok(Self::Done(Done {
                record,
                text,
                head: Some(Box::new(done.head)),
            })),
            Outcome::Conflict { by, fields } => Ok(Self::Clash(
                by,
                fields.into_iter().map(|f| f.path).collect(),
            )),
            Outcome::NotRevertible(why) => Err(boxed(REJECTED, why, 422)),
            _ => Err(boxed(UNKNOWN_RECORD, "the record is gone", 404)),
        }
    }
}

pub(crate) type Failure = Box<Problem>;

/// The title is text for people made from the kind (`form-source-unknown-command` is "Unknown command"), never the kind itself.
fn title_of(kind: &str) -> String {
    let words = kind
        .strip_prefix("form-source-")
        .unwrap_or(kind)
        .replace('-', " ");
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

pub(crate) fn problem(kind: &str, detail: impl Into<String>, status: u16) -> Problem {
    Problem::new(kind, title_of(kind), detail, status)
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

/// `card.move`: the card `uid` into `column` of the board `board`, right after the card `after` and right before the card `before` (record ids; none = the end).
pub(crate) async fn move_card(
    repo: &TaskRepo,
    uid: String,
    args: capnp::struct_list::Reader<'_, value::Owned>,
    expect: Option<u64>,
) -> Result<Ran, Failure> {
    let mut got = text_args(args, &MOVE_ARGS)?;
    check_uid(&uid)?;
    let (board, column) = (take(&mut got, "board")?, take(&mut got, "column")?);
    let (after, before) = (
        take_neighbour(&mut got, "after"),
        take_neighbour(&mut got, "before"),
    );
    let boards = repo.boards().await.map_err(to_problem)?;
    let id = board_named(&boards, &board)?.id;
    task_of(repo, &uid).await?;
    let (repo, moved) = (repo.clone(), uid.clone());
    let outcome = run_to_end(async move {
        let near = (after.as_deref(), before.as_deref());
        place(&repo, id, &moved, &column, near, expect).await
    })
    .await
    .map_err(|e| boxed(INTERNAL, e.to_string(), 500))?
    .map_err(to_problem)?;
    Ran::of(outcome, uid, "card-moved")
}

/// `task.create`: a task with `title`, put into `column` of `board` between `after` and `before` (none = the end). Board and column are checked before
/// anything is written; if the placing still fails (a conflict, a column or board that vanished in between), the new task is deleted
/// again (`create_placed`). A refused command leaves nothing behind, so a retry never makes the task twice.
pub(crate) async fn create_task(
    repo: &TaskRepo,
    record: &str,
    args: capnp::struct_list::Reader<'_, value::Owned>,
) -> Result<Ran, Failure> {
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
    let column = take(&mut got, "column")?;
    let board = take_neighbour(&mut got, "board");
    let (after, before) = (
        take_neighbour(&mut got, "after"),
        take_neighbour(&mut got, "before"),
    );
    let boards = repo.boards().await.map_err(to_problem)?;
    // `lane` is optional on `create` (forms.txt): without it exactly ONE board is meant; none or several is refused with the reason
    let board = match board {
        Some(name) => name,
        None => match boards.as_slice() {
            [only] => only.board.name.clone(),
            [] => {
                return Err(boxed(
                    BAD_ARGS,
                    "`board` is missing and there is no board to put the card on",
                    400,
                ));
            }
            _ => {
                let names: Vec<&str> = boards.iter().map(|b| b.board.name.as_str()).collect();
                let why = format!(
                    "`board` is missing and there are several boards ({}): name one",
                    names.join(", ")
                );
                return Err(boxed(BAD_ARGS, why, 400));
            }
        },
    };
    let entry = board_named(&boards, &board)?;
    if !entry.board.columns.iter().any(|c| c.id == column) {
        return Err(to_problem(Error::Rejected(format!("no column {column:?}"))));
    }
    let (id, repo) = (entry.id, repo.clone());
    let (uid, outcome) = run_to_end(async move {
        let near = (after.as_deref(), before.as_deref());
        create_placed(&repo, id, title, &column, near).await
    })
    .await
    .map_err(|e| boxed(INTERNAL, e.to_string(), 500))?
    .map_err(to_problem)?;
    Ran::of(outcome, uid, "task-created")
}

/// `task.delete`: the task `uid` and nothing else; it takes no argument.
pub(crate) async fn delete_task(
    repo: &TaskRepo,
    uid: &str,
    args: capnp::struct_list::Reader<'_, value::Owned>,
    expect: Option<u64>,
) -> Result<Ran, Failure> {
    text_args(args, &[])?;
    let id = task_of(repo, uid).await?.id;
    let repo = repo.clone();
    let outcome = run_to_end(async move { repo.delete_checked(id, expect).await })
        .await
        .map_err(|e| boxed(INTERNAL, e.to_string(), 500))?
        .map_err(to_problem)?;
    Ran::of(outcome, String::new(), "task-deleted")
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
