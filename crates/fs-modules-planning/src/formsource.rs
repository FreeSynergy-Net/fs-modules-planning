//! `FormSource` of the planning module: the read side for drawing by a form (`cards-in-columns@1.2`, and 1.1 and 1.0 for reading) and the writing of the commands
//! of `catalog/commands.json` (`card.move`, `task.create`, `task.delete`, see `write.rs`), next to the contract `tasks.capnp` on the
//! same socket.
//!
//! Pattern: Adapter (the board layout and the tasks of the core, as records by the field names of `catalog/forms.json`). The one capability that is `Tasks` and `FormSource` is in `both.rs`. The wire name of a command is its capability key of the contract; `invoke` only knows the commands
//! of `catalog/commands.json`, and the values of a command are text (the wire of `FormSource` has no other kind). `may` says what the provider supports (the core has no rights per person, see `write.rs`).

use std::rc::Rc;
use std::sync::Arc;

use fs_interfaces::common_capnp::cursor;
use fs_interfaces::forms::parse_key;
use fs_interfaces::formsource_capnp::{form_source, record_view, value};
use fs_tasks_core::{TaskItems, TaskRepo};

use crate::level2::{clash_problem, write_done, write_problem};
use crate::level2_calls;
use crate::wire::count;
use crate::write::{
    BAD_ARGS, Failure, RECORD_COMMANDS, Ran, SET_COMMANDS, UNKNOWN_COMMAND, boxed, create_task,
    delete_task, may, move_card, permit, problem,
};
use fs_objects::Origin;

/// Who the service says it is in `hello`.
pub const PROVIDER: &str = "planning";

/// The level of `FormSource` this service speaks (`hello`): 2 = compare-and-set, undo, `introduce`, `changes` and `watch`. It is its OWN number, not
/// `fs_interfaces::forms::FORM_SOURCE_LEVEL`: levels are cumulative and that constant rises with the host, but this service says what it does.
pub const LEVEL: u32 = 2;

/// The form this module draws and writes, as `name@x.y.z` (a key of `schema/forms.txt`): `cards-in-columns@1.2.0` has the channels `order`
/// (read only: the sequence), `after`, `before`, `create` and `remove` and the `writes` of `move`. `catalog/forms.json` declares this version
/// as `1.2` (the host reads `x.y` until it takes `x.y.z`).
pub const FORM_CARDS: &str = "cards-in-columns@1.2.0";

/// The name of the form and the older minors of it that are still read (`1.1`, `1.0`: the same cards, a subset of the channels); both ways
/// of writing the key (`x.y`, `x.y.z`) are taken, the patch does not count.
pub const FORM_NAME: &str = "cards-in-columns";
const MINORS_SERVED: [u64; 3] = [0, 1, 2];

/// The most records of one page when the caller names no limit (or a bigger one).
pub const PAGE_MAX: u32 = 500;

pub(crate) const UNKNOWN_FORM: &str = "form-source-unknown-form";

pub(crate) struct FormSourceImpl {
    pub(crate) repo: Arc<TaskRepo>,
    /// Where the changes of THIS connection come from (`introduce`); one per connection, made by `bootstrap`.
    pub(crate) origin: crate::level2::Context,
}

type Reply = Result<(), capnp::Error>;

/// Whether `form` is a key of the form this module serves: the name, major 1 and a minor of `MINORS_SERVED` (`x.y` and `x.y.z`).
pub(crate) fn serves(form: &str) -> bool {
    parse_key(form).is_some_and(|(name, v)| {
        name == FORM_NAME && v.major == 1 && MINORS_SERVED.contains(&v.minor)
    })
}

impl form_source::Server for FormSourceImpl {
    async fn hello(
        self: Rc<Self>,
        _: form_source::HelloParams,
        mut results: form_source::HelloResults,
    ) -> Reply {
        let mut out = results.get();
        out.set_level(LEVEL);
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
            let why = format!("this module draws {FORM_CARDS} (and 1.1, 1.0 for reading)");
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
        // level 1: no expectation and no step
        let result = self.run(&command, record, p.get_args()?, None, None).await;
        let mut out = results.get();
        match result {
            Ok(Ran::Done(done)) => {
                let seen = level2_calls::head_seen(&self, &done).await;
                write_done(out.init_result(), &done, &seen);
            }
            Ok(Ran::Clash(..)) => {
                out.set_failed(true);
                write_problem(out.init_problem(), &clash_problem());
            }
            Err(why) => {
                out.set_failed(true);
                write_problem(out.init_problem(), &why);
            }
        }
        Ok(())
    }

    async fn invoke_if(
        self: Rc<Self>,
        params: form_source::InvokeIfParams,
        results: form_source::InvokeIfResults,
    ) -> Reply {
        level2_calls::invoke_if(&self, params, results).await
    }

    async fn revert(
        self: Rc<Self>,
        params: form_source::RevertParams,
        results: form_source::RevertResults,
    ) -> Reply {
        level2_calls::revert(&self, params, results).await
    }

    async fn changes(
        self: Rc<Self>,
        params: form_source::ChangesParams,
        results: form_source::ChangesResults,
    ) -> Reply {
        level2_calls::changes(&self, params, results).await
    }

    async fn introduce(
        self: Rc<Self>,
        params: form_source::IntroduceParams,
        _: form_source::IntroduceResults,
    ) -> Reply {
        let p = params.get()?;
        let origin = Origin::new(
            p.get_program()?.to_string()?,
            p.get_instance()?.to_string()?,
            p.get_device()?.to_string()?,
        );
        *self.origin.borrow_mut() = origin;
        Ok(())
    }

    async fn watch(
        self: Rc<Self>,
        params: form_source::WatchParams,
        results: form_source::WatchResults,
    ) -> Reply {
        level2_calls::watch(&self, params, results).await
    }
}

impl FormSourceImpl {
    /// One command by the capability key of the contract: known and possible first, then its arguments. `expect` is the `rev` of
    /// the record the caller saw, `step` the (run, step) that makes a repeat act once; the writes carry the origin of the connection.
    pub(crate) async fn run(
        &self,
        command: &str,
        record: String,
        args: capnp::struct_list::Reader<'_, value::Owned>,
        expect: Option<u64>,
        step: Option<(String, String)>,
    ) -> Result<Ran, Failure> {
        permit(&self.repo.capabilities(), command)?;
        let repo = self.handle(step.as_ref());
        match command {
            "card.move" => move_card(&repo, record, args, expect).await,
            "task.create" => create_task(&repo, &record, args).await,
            "task.delete" => delete_task(&repo, &record, args, expect).await,
            other => Err(boxed(UNKNOWN_COMMAND, format!("no command {other:?}"), 404)),
        }
    }
}

fn set_may(mut out: capnp::text_list::Builder<'_>, keys: &[&str]) {
    for (i, key) in (0u32..).zip(keys) {
        out.set(i, key);
    }
}

pub(crate) fn read_offset(c: cursor::Reader<'_>) -> capnp::Result<usize> {
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
