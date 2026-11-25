//! What the planning module draws and says of itself: the forms it serves (`cards-in-columns@1.4`, and 1.3, 1.2, 1.1 and 1.0 for
//! reading, `rows@1.0`), its provider name and level, and the reader of its store that `PlanningCore` pages through.
//!
//! Pattern: Adapter (the board layout and the tasks of the core, as records by the field names of `catalog/forms.json`). The wire of
//! `FormSource` is not here: `fs_rpc::Guarded<PlanningCore>` serves it, next to the contract `tasks.capnp` on the same socket
//! (`root.rs`). The wire name of a command is its capability key of the contract; the values of a command are text.

use std::sync::Arc;

use fs_interfaces::forms::parse_key;
use fs_objects::Origin;
use fs_tasks_core::TaskRepo;

use crate::write::{BAD_ARGS, problem};

/// Who the service says it is in `hello`.
pub const PROVIDER: &str = "planning";

/// The level of `FormSource` this service speaks (`hello`): 2 = compare-and-set, undo, `introduce`, `changes` and `watch`. It is its OWN number, not
/// `fs_interfaces::forms::FORM_SOURCE_LEVEL`: levels are cumulative and that constant rises with the host, but this service says what it does.
pub const LEVEL: u32 = 2;

/// The form this module draws and writes, as `name@x.y.z` (a key of `schema/forms.txt`): `cards-in-columns@1.4.0` has the channels (as 1.3.0, 1.2.0) `order`
/// (read only: the sequence), `after`, `before`, `create` and `remove` and the `writes` of `move`, plus `state`, `assignee` (repeats) and `time-zone`.
/// `catalog/forms.json` declares this version as `1.4.0` (`x.y.z`).
pub const FORM_CARDS: &str = "cards-in-columns@1.4.0";

/// The name of the form and the older minors of it that are still read (`1.1`, `1.0`: the same cards, a subset of the channels); both ways
/// of writing the key (`x.y`, `x.y.z`) are taken, the patch does not count.
pub const FORM_NAME: &str = "cards-in-columns";

/// The second form: `rows@1.0.0`, with the type `Task` (the tasks as rows) or `Lane` (the lanes of a swim-lane view, scope `lanes:<axis>[:<board>]`).
pub const FORM_ROWS: &str = "rows@1.0.0";
const MINORS_SERVED: [u64; 5] = [0, 1, 2, 3, 4];

/// The most records of one page when the caller names no limit (or a bigger one).
pub const PAGE_MAX: u32 = 500;

pub(crate) const UNKNOWN_FORM: &str = "form-source-unknown-form";

/// The text of the problem for a form `page` does not serve.
pub(crate) fn unknown_form_why() -> String {
    format!("this module draws {FORM_CARDS} (and 1.3, 1.2, 1.1, 1.0 for reading) and {FORM_ROWS}")
}

/// The text of the problem for a form `watch` and `changes` do not serve (they follow the cards only, not `rows@1.0.0`).
pub(crate) fn unwatchable_form_why() -> String {
    format!("this module lets you follow {FORM_CARDS} (and 1.3, 1.2, 1.1, 1.0), not {FORM_ROWS}")
}

/// The reader of the store for ONE call: the handle written as the connection, and the origin it claimed (`changes` of the own program).
pub(crate) struct Reading {
    pub(crate) repo: Arc<TaskRepo>,
    pub(crate) origin: Origin,
}

/// Whether `form` is `rows@1.0.0` (`x.y` and `x.y.z`).
pub(crate) fn serves_rows(form: &str) -> bool {
    parse_key(form).is_some_and(|(name, v)| name == "rows" && v.major == 1 && v.minor == 0)
}

/// Whether `form` is a key of the form this module serves: the name, major 1 and a minor of `MINORS_SERVED` (`x.y` and `x.y.z`).
pub(crate) fn serves(form: &str) -> bool {
    parse_key(form).is_some_and(|(name, v)| {
        name == FORM_NAME && v.major == 1 && MINORS_SERVED.contains(&v.minor)
    })
}

/// The offset a cursor of this service says (empty = the start); anything else is `bad-arguments`.
pub(crate) fn offset_of(bytes: &[u8]) -> capnp::Result<usize> {
    if bytes.is_empty() {
        return Ok(0);
    }
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|t| t.parse().ok())
        .ok_or_else(|| problem(BAD_ARGS, "the cursor is not one of this service", 400).into_error())
}

/// The number of records of one page: no limit (0) or a bigger one than `PAGE_MAX` is `PAGE_MAX`.
pub(crate) fn limit_of(limit: u32) -> usize {
    (match limit {
        0 => PAGE_MAX,
        n => n.min(PAGE_MAX),
    }) as usize
}
