//! `FormSource` of the planning module: the read side for drawing by a form (`cards-in-columns@1.3`, and 1.2, 1.1 and 1.0 for reading) and the writing of the commands
//! of `catalog/commands.json` (`card.move`, `task.create`, `task.delete`, see `write.rs`), next to the contract `tasks.capnp` on the
//! same socket.
//!
//! Pattern: Adapter (the board layout and the tasks of the core, as records by the field names of `catalog/forms.json`). The one capability that is `Tasks` and `FormSource` is in `both.rs`. The wire name of a command is its capability key of the contract; `invoke` only knows the commands
//! of `catalog/commands.json`, and the values of a command are text (the wire of `FormSource` has no other kind). `may` says what the provider supports (the core has no rights per person, see `write.rs`).

use std::rc::Rc;
use std::sync::Arc;

use fs_interfaces::common_capnp::cursor;
use fs_interfaces::forms::{FORM_SOURCE_LEVEL, parse_key};
use fs_interfaces::formsource_capnp::{form_source, record_view, value};
use fs_tasks_core::{BoardEntry, Boards, TaskItems, TaskRepo};

use crate::errors::rpc_error;
use crate::wire::count;
use crate::write::{
    BAD_ARGS, Done, Failure, RECORD_COMMANDS, SET_COMMANDS, UNKNOWN_COMMAND, board_named, boxed,
    create_task, delete_task, may, move_card, permit, problem,
};

/// Who the service says it is in `hello`.
pub const PROVIDER: &str = "planning";

/// The form this module draws and writes, as `name@x.y.z` (a key of `schema/forms.txt`): `cards-in-columns@1.3.0` has the channels (as 1.2.0) `order`
/// (read only: the sequence), `after`, `before`, `create` and `remove` and the `writes` of `move`. `catalog/forms.json` declares this version
/// as `1.3` (the host reads `x.y` until it takes `x.y.z`).
pub const FORM_CARDS: &str = "cards-in-columns@1.3.0";

/// The name of the form and the older minors of it that are still read (`1.1`, `1.0`: the same cards, a subset of the channels); both ways
/// of writing the key (`x.y`, `x.y.z`) are taken, the patch does not count.
pub const FORM_NAME: &str = "cards-in-columns";
const MINORS_SERVED: [u64; 4] = [0, 1, 2, 3];

/// The most records of one page when the caller names no limit (or a bigger one).
pub const PAGE_MAX: u32 = 500;

const UNKNOWN_FORM: &str = "form-source-unknown-form";

pub(crate) struct FormSourceImpl {
    pub(crate) repo: Arc<TaskRepo>,
}

type Reply = Result<(), capnp::Error>;

/// One card of the layout: its board, column, uid and its position in the column (the read-only `order`).
struct Card {
    board: String,
    column: String,
    label: String,
    /// The WIP limit of the column (`column-limit`); the component counts, the module only reports it.
    limit: Option<u32>,
    uid: String,
    at: usize,
}

/// Whether `form` is a key of the form this module serves: the name, major 1 and a minor of `MINORS_SERVED` (`x.y` and `x.y.z`).
fn serves(form: &str) -> bool {
    parse_key(form).is_some_and(|(name, v)| {
        name == FORM_NAME && v.major == 1 && MINORS_SERVED.contains(&v.minor)
    })
}

impl FormSourceImpl {
    /// The cards of the boards in `scope` (empty = every board), in the order of the layout: column by column, as `(board name,
    /// column id, uid)`. Only the layout is read here; the tasks of one page are read by [`Self::row`].
    async fn cards(&self, scope: &str) -> capnp::Result<Vec<Card>> {
        let boards = self.repo.boards().await.map_err(rpc_error)?;
        let chosen: Vec<&BoardEntry> = if scope.is_empty() {
            boards.iter().collect()
        } else {
            vec![board_named(&boards, scope).map_err(|p| p.into_error())?]
        };
        let mut out = Vec::new();
        for entry in chosen {
            let layout = self.repo.board_layout(entry.id).await.map_err(rpc_error)?;
            for column in layout.columns {
                let (board, id) = (&entry.board.name, &column.column.id);
                let label = &column.column.name;
                let limit = column.column.wip_limit;
                out.extend(column.cards.into_iter().enumerate().map(|(at, uid)| Card {
                    board: board.clone(),
                    column: id.clone(),
                    label: label.clone(),
                    limit,
                    uid,
                    at,
                }));
            }
        }
        Ok(out)
    }

    /// The record of one card by the field names of `catalog/forms.json`; `None` when the task is gone in the meantime.
    async fn row(
        &self,
        Card {
            board,
            column,
            label,
            limit,
            uid,
            at,
        }: &Card,
    ) -> capnp::Result<Option<Vec<(&'static str, String)>>> {
        let Some(task) = self.repo.get_by_uid(uid).await.map_err(rpc_error)? else {
            return Ok(None);
        };
        let task = &task.task;
        let mut row = vec![
            ("uid", uid.clone()),
            ("board", board.clone()),
            ("column", column.clone()),
            ("title", task.title.clone().unwrap_or_default()),
            ("index", at.to_string()),
        ];
        // RFC 3339 date-time with offset (form art `time`), not the naive JSCalendar LocalDateTime
        if let Some(due) = self.repo.due_instant(task).map_err(rpc_error)? {
            row.push(("due", due));
        }
        row.push(("column-label", label.clone()));
        if let Some(limit) = limit {
            row.push(("column-limit", limit.to_string()));
        }
        if let Some(percent) = task.percent_complete {
            row.push(("percent", percent.to_string()));
        }
        row.extend(crate::facts::channels(task));
        row.extend(crate::facts::of(task));
        Ok(Some(row))
    }
}

impl form_source::Server for FormSourceImpl {
    async fn hello(
        self: Rc<Self>,
        _: form_source::HelloParams,
        mut results: form_source::HelloResults,
    ) -> Reply {
        let mut out = results.get();
        out.set_level(FORM_SOURCE_LEVEL);
        out.set_provider(PROVIDER);
        Ok(())
    }

    async fn page(
        self: Rc<Self>,
        params: form_source::PageParams,
        mut results: form_source::PageResults,
    ) -> Reply {
        let p = params.get()?;
        let form = p.get_form()?.to_str()?;
        if !serves(form) {
            let why = format!("this module draws {FORM_CARDS} (and 1.2, 1.1, 1.0 for reading)");
            return Err(problem(UNKNOWN_FORM, why, 404).into_error());
        }
        let scope = p.get_scope()?.to_string()?;
        let skip = read_offset(p.get_cursor()?)?;
        let limit = match p.get_limit() {
            0 => PAGE_MAX,
            n => n.min(PAGE_MAX),
        } as usize;
        let cards = self.cards(&scope).await?;
        let total = cards.len();
        let mut rows = Vec::new();
        for card in cards.iter().skip(skip).take(limit) {
            rows.extend(self.row(card).await?);
        }
        let end = (skip + limit).min(total);
        // what the provider supports, for a record and for the set (a new card); the same table `invoke` is held to
        let caps = self.repo.capabilities();
        let (on_record, on_set) = (may(&caps, &RECORD_COMMANDS), may(&caps, &SET_COMMANDS));
        let mut out = results.get();
        {
            let mut page = out.reborrow().init_page();
            page.set_total(total as u64);
            page.set_has_total(true);
            set_may(page.reborrow().init_may(count(on_set.len())?), &on_set);
            let mut list = page.init_records(count(rows.len())?);
            for (i, row) in (0u32..).zip(&rows) {
                let mut view: record_view::Builder<'_> = list.reborrow().get(i);
                view.set_id(row[0].1.as_str());
                set_may(
                    view.reborrow().init_may(count(on_record.len())?),
                    &on_record,
                );
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

    async fn blob(
        self: Rc<Self>,
        _: form_source::BlobParams,
        _: form_source::BlobResults,
    ) -> Reply {
        Err(problem(
            "form-source-unsupported",
            "this module has no image or file field",
            501,
        )
        .into_error())
    }

    async fn invoke(
        self: Rc<Self>,
        params: form_source::InvokeParams,
        mut results: form_source::InvokeResults,
    ) -> Reply {
        let p = params.get()?;
        let command = p.get_command()?.to_string()?;
        let record = p.get_record()?.to_string()?;
        let result = self.run(&command, record, p.get_args()?).await;
        let mut out = results.get();
        match result {
            Ok(done) => {
                let mut made = out.init_result();
                made.set_record(done.record.as_str());
                made.set_text(done.text);
            }
            Err(why) => {
                out.set_failed(true);
                crate::compat::write_problem(out.init_problem(), &why);
            }
        }
        Ok(())
    }
}

impl FormSourceImpl {
    /// One command by the capability key of the contract: known and possible first, then its arguments.
    async fn run(
        &self,
        command: &str,
        record: String,
        args: capnp::struct_list::Reader<'_, value::Owned>,
    ) -> Result<Done, Failure> {
        permit(&self.repo.capabilities(), command)?;
        match command {
            "card.move" => move_card(&self.repo, record, args).await,
            "task.create" => create_task(&self.repo, &record, args).await,
            "task.delete" => delete_task(&self.repo, &record, args).await,
            other => Err(boxed(UNKNOWN_COMMAND, format!("no command {other:?}"), 404)),
        }
    }
}

fn set_may(mut out: capnp::text_list::Builder<'_>, keys: &[&str]) {
    for (i, key) in (0u32..).zip(keys) {
        out.set(i, key);
    }
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

fn set_value(mut out: value::Builder<'_>, path: &str, text: &str) {
    out.set_path(path);
    out.set_text(text);
}
