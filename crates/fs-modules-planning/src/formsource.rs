//! `FormSource` of the planning module: the read side for drawing by a form (`cards-in-columns@1.0`) and the writing of the commands
//! of `catalog/commands.json` (`card.move`, `task.create`, `task.delete`, see `write.rs`), next to the contract `tasks.capnp` on the
//! same socket.
//!
//! Pattern: Adapter (the board layout and the tasks of the core, as records by the field names of `catalog/forms.json`). The one capability that is `Tasks` and `FormSource` is in `both.rs`. The wire name of a command is its capability key of the contract; `invoke` only knows the commands
//! of `catalog/commands.json`, and the values of a command are text (the wire of `FormSource` has no other kind). `may` says what the provider supports (the core has no rights per person, see `write.rs`).

use std::rc::Rc;
use std::sync::Arc;

use fs_interfaces::common_capnp::cursor;
use fs_interfaces::forms::FORM_SOURCE_LEVEL;
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

/// The one form this module draws today, as `name@Major.Minor` (a key of `schema/forms.txt`).
pub const FORM_CARDS: &str = "cards-in-columns@1.0";

/// The most records of one page when the caller names no limit (or a bigger one).
pub const PAGE_MAX: u32 = 500;

const UNKNOWN_FORM: &str = "form-source-unknown-form";

pub(crate) struct FormSourceImpl {
    pub(crate) repo: Arc<TaskRepo>,
}

type Reply = Result<(), capnp::Error>;

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
                why.write(out.init_problem());
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
