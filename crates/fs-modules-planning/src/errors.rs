//! Errors of the core as RPC errors.
//!
//! Pattern: Adapter — a core error becomes the problem the contract names for it: a reason starting `conflict:` (the board
//! changed since it was read, nothing was written), `rejected:` (the board or the task is not valid), an RFC 9457 problem for the
//! rest. The text of a store failure is cut, never the cause hidden.

use fs_rpc::{Problem, failed};
use fs_tasks_core::Error;

const NOT_FOUND: &str = "https://freesynergy.net/problems/tasks/not-found";
const UNSUPPORTED: &str = "https://freesynergy.net/problems/unsupported";

/// A core error as an RPC error.
#[allow(clippy::needless_pass_by_value)] // used as `map_err(rpc_error)`
pub(crate) fn rpc_error(e: Error) -> capnp::Error {
    match &e {
        // Their own words already start with `conflict:` and `rejected:`.
        Error::Changed(_) | Error::Rejected(_) => capnp::Error::failed(e.to_string()),
        Error::Conflict(_) | Error::Exists(_) | Error::Cycle(_) => {
            capnp::Error::failed(format!("conflict: {e}"))
        }
        Error::Invalid(_) | Error::Time(_) | Error::ICalendar(_) => {
            capnp::Error::failed(format!("rejected: {e}"))
        }
        Error::NotFound(_) => {
            Problem::new(NOT_FOUND, "Task not found", e.to_string(), 404).into_error()
        }
        Error::Unsupported(not_possible) => {
            Problem::new(UNSUPPORTED, "Not possible", not_possible.to_string(), 501).into_error()
        }
        _ => failed(e),
    }
}

/// The request itself is not acceptable (a text that is no task, a missing id).
pub(crate) fn rejected(detail: impl std::fmt::Display) -> capnp::Error {
    capnp::Error::failed(format!("rejected: {detail}"))
}
