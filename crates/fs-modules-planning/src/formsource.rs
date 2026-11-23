//! `FormSource` of the planning module: the read side for drawing by a form (`cards-in-columns@1.0`) and the writing of one command
//! (`move`), next to the contract `tasks.capnp` on the same socket.
//!
//! Pattern: Adapter (the board layout and the tasks of the core, as records by the field names of `catalog/forms.json`). The one capability that is `Tasks` and `FormSource` is in `both.rs`. Everything the person may do is checked by the core as the caller of the connection; `invoke` only knows the commands
//! of `catalog/commands.json`, and the values of a command are text (the wire of `FormSource` has no other kind).

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use fs_interfaces::common_capnp::cursor;
use fs_interfaces::forms::FORM_SOURCE_LEVEL;
use fs_interfaces::formsource_capnp::{form_source, record_view, value};
use fs_rpc::{Problem, run_to_end};
use fs_tasks_core::{BoardEntry, Boards, TaskItems, TaskRepo};

use crate::errors::rpc_error;
use crate::wire::count;

/// Who the service says it is in `hello`.
pub const PROVIDER: &str = "planning";

/// The one form this module draws today, as `name@Major.Minor` (a key of `schema/forms.txt`).
pub const FORM_CARDS: &str = "cards-in-columns@1.0";

/// The most records of one page when the caller names no limit (or a bigger one).
pub const PAGE_MAX: u32 = 500;

const UNKNOWN_FORM: &str = "form-source-unknown-form";
const UNKNOWN_SCOPE: &str = "form-source-unknown-scope";
const UNKNOWN_COMMAND: &str = "form-source-unknown-command";
const BAD_ARGS: &str = "form-source-bad-arguments";
const UNKNOWN_ARG: &str = "form-source-unknown-argument";
const DUPLICATE_ARG: &str = "form-source-duplicate-argument";
const INTERNAL: &str = "form-source-internal";

/// The longest `record` (uid of a card) `invoke` takes.
const UID_MAX: usize = 255;

pub(crate) struct FormSourceImpl {
    pub(crate) repo: Arc<TaskRepo>,
}

type Done = Result<(), capnp::Error>;

fn problem(kind: &str, detail: impl Into<String>, status: u16) -> Problem {
    Problem::new(kind, kind, detail, status)
}

/// The text values of a request by path. A path that is not in `allowed` or that comes twice is refused with its own problem and
/// the reason, never silently dropped.
fn text_args(
    list: capnp::struct_list::Reader<'_, value::Owned>,
    allowed: &[&str],
) -> Result<BTreeMap<String, String>, Box<Problem>> {
    let mut out = BTreeMap::new();
    for v in list {
        let path = v
            .get_path()
            .and_then(|p| p.to_string().map_err(capnp::Error::from))
            .map_err(bad_wire)?;
        let text = v
            .get_text()
            .and_then(|t| t.to_string().map_err(capnp::Error::from))
            .map_err(bad_wire)?;
        if !allowed.contains(&path.as_str()) {
            let why = format!("no argument {path:?} (allowed: {})", allowed.join(", "));
            return Err(boxed(UNKNOWN_ARG, why, 400));
        }
        if out.insert(path.clone(), text).is_some() {
            return Err(boxed(
                DUPLICATE_ARG,
                format!("argument {path:?} twice"),
                400,
            ));
        }
    }
    Ok(out)
}

fn set_value(mut out: value::Builder<'_>, path: &str, text: &str) {
    out.set_path(path);
    out.set_text(text);
}

/// The board for `scope`: the id wins; a name that more than one board has is refused (the caller uses the id).
fn board_named<'a>(boards: &'a [BoardEntry], scope: &str) -> Result<&'a BoardEntry, Box<Problem>> {
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

impl FormSourceImpl {
    /// One record per card of the boards in `scope` (empty = every board), in the order of the layout: column by column.
    async fn records(&self, scope: &str) -> capnp::Result<Vec<Vec<(&'static str, String)>>> {
        let boards = self.repo.boards().await.map_err(rpc_error)?;
        let chosen: Vec<&BoardEntry> = if scope.is_empty() {
            boards.iter().collect()
        } else {
            vec![board_named(&boards, scope).map_err(|p| p.into_error())?]
        };
        let mut out = Vec::new();
        for entry in chosen {
            let layout = self.repo.board_layout(entry.id).await.map_err(rpc_error)?;
            for column in &layout.columns {
                for uid in &column.cards {
                    let Some(task) = self.repo.get_by_uid(uid).await.map_err(rpc_error)? else {
                        continue;
                    };
                    let task = &task.task;
                    let mut row = vec![
                        ("uid", uid.clone()),
                        ("board", entry.board.name.clone()),
                        ("column", column.column.id.clone()),
                        ("title", task.title.clone().unwrap_or_default()),
                    ];
                    if let Some(due) = &task.due {
                        row.push(("due", due.clone()));
                    }
                    if let Some(progress) = &task.progress {
                        row.push(("progress", progress.clone()));
                    }
                    row.extend(crate::facts::of(task));
                    out.push(row);
                }
            }
        }
        Ok(out)
    }
}

impl form_source::Server for FormSourceImpl {
    async fn hello(
        self: Rc<Self>,
        _: form_source::HelloParams,
        mut results: form_source::HelloResults,
    ) -> Done {
        let mut out = results.get();
        out.set_level(FORM_SOURCE_LEVEL);
        out.set_provider(PROVIDER);
        Ok(())
    }

    async fn page(
        self: Rc<Self>,
        params: form_source::PageParams,
        mut results: form_source::PageResults,
    ) -> Done {
        let p = params.get()?;
        if p.get_form()?.to_str()? != FORM_CARDS {
            return Err(
                problem(UNKNOWN_FORM, "this module draws cards-in-columns@1.0", 404).into_error(),
            );
        }
        let scope = p.get_scope()?.to_string()?;
        let skip = read_offset(p.get_cursor()?)?;
        let limit = match p.get_limit() {
            0 => PAGE_MAX,
            n => n.min(PAGE_MAX),
        } as usize;
        let rows = self.records(&scope).await?;
        let total = rows.len();
        let rows: Vec<_> = rows.into_iter().skip(skip).take(limit).collect();
        let end = skip + rows.len();
        let mut out = results.get();
        {
            let mut page = out.reborrow().init_page();
            page.set_total(total as u64);
            page.set_has_total(true);
            let mut list = page.init_records(count(rows.len())?);
            for (i, row) in (0u32..).zip(&rows) {
                let mut view: record_view::Builder<'_> = list.reborrow().get(i);
                view.set_id(row[0].1.as_str());
                // what the commands of this module allow in general; the rights of the person are checked by the core at `invoke`
                view.reborrow().init_may(1).set(0, "planning.move");
                let mut values = view.init_values(count(row.len() - 1)?);
                for (j, (path, text)) in (0u32..).zip(&row[1..]) {
                    set_value(values.reborrow().get(j), path, text);
                }
            }
        }
        if end < total {
            out.init_next().set_opaque(end.to_string().as_bytes());
        }
        Ok(())
    }

    async fn blob(self: Rc<Self>, _: form_source::BlobParams, _: form_source::BlobResults) -> Done {
        Err(problem(
            "https://freesynergy.net/problems/unsupported",
            "this module has no image or file field",
            501,
        )
        .into_error())
    }

    async fn invoke(
        self: Rc<Self>,
        params: form_source::InvokeParams,
        mut results: form_source::InvokeResults,
    ) -> Done {
        let p = params.get()?;
        let command = p.get_command()?.to_string()?;
        let result = match command.as_str() {
            "planning.move" => {
                self.move_card(p.get_record()?.to_string()?, p.get_args()?)
                    .await
            }
            other => Err(boxed(UNKNOWN_COMMAND, format!("no command {other:?}"), 404)),
        };
        let mut out = results.get();
        if let Err(why) = result {
            out.set_failed(true);
            why.write(out.init_problem());
        }
        Ok(())
    }
}

impl FormSourceImpl {
    /// `move`: the card `uid` into `column` of the board `board` at `index` (text of a number; none = the end).
    async fn move_card(
        &self,
        uid: String,
        args: capnp::struct_list::Reader<'_, value::Owned>,
    ) -> Result<(), Box<Problem>> {
        let mut got = text_args(args, &["board", "column", "index"])?;
        if uid.is_empty() || uid.len() > UID_MAX {
            let why = format!("`record` must be a uid of 1 to {UID_MAX} bytes");
            return Err(boxed(BAD_ARGS, why, 400));
        }
        let (Some(board), Some(column)) = (got.remove("board"), got.remove("column")) else {
            return Err(boxed(BAD_ARGS, "move needs `board` and `column`", 400));
        };
        let index = match got.remove("index") {
            None => None,
            Some(t) => Some(
                t.parse::<usize>()
                    .map_err(|_| boxed(BAD_ARGS, "`index` is no number", 400))?,
            ),
        };
        let boards = self.repo.boards().await.map_err(to_problem)?;
        let entry = board_named(&boards, &board)?;
        if self
            .repo
            .get_by_uid(&uid)
            .await
            .map_err(to_problem)?
            .is_none()
        {
            return Err(boxed(BAD_ARGS, format!("no task {uid:?}"), 404));
        }
        let (id, repo) = (entry.id, self.repo.clone());
        run_to_end(async move { repo.board_move_card(id, &uid, &column, index).await })
            .await
            .map_err(|e| boxed(INTERNAL, e.to_string(), 500))?
            .map(|_| ())
            .map_err(to_problem)
    }
}

#[allow(clippy::unnecessary_box_returns)] // the error type of `move_card` is boxed (a `Problem` is large)
fn to_problem(e: fs_tasks_core::Error) -> Box<Problem> {
    let text = e.to_string();
    Box::new(Problem::from_error(&rpc_error(e)).unwrap_or_else(|| problem(INTERNAL, text, 500)))
}

fn read_offset(c: cursor::Reader<'_>) -> capnp::Result<usize> {
    let bytes = c.get_opaque()?;
    if bytes.is_empty() {
        return Ok(0);
    }
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|t| t.parse().ok())
        .ok_or_else(|| problem(BAD_ARGS, "the cursor is not one of this service", 400).into_error())
}

fn bad_wire(e: capnp::Error) -> Box<Problem> {
    boxed(BAD_ARGS, e.to_string(), 400)
}

fn boxed(kind: &str, detail: impl Into<String>, status: u16) -> Box<Problem> {
    Box::new(problem(kind, detail, status))
}
