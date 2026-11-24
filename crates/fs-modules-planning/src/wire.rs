//! Ids, envelopes and tasks on the wire.
//!
//! Pattern: Adapter — `ObjectId` and `TaskEntry` between Rust and `common.capnp` / `tasks.capnp`. The JSCalendar JSON stays
//! authoritative for the content. Envelopes and ids are written by fs-rpc (one fs-objects for the core and fs-rpc).

use fs_interfaces::common_capnp::uuid;
use fs_interfaces::tasks_capnp::task;
use fs_objects::ObjectId;
use fs_rpc::check_intake;

use crate::compat::{read_uuid, write_envelope};
use fs_tasks_core::{Revision, Task, TaskEntry};

use crate::errors::rejected;

/// Read an object id; none or not a valid id is a rejected request.
pub(crate) fn read_id(r: uuid::Reader<'_>) -> capnp::Result<ObjectId> {
    let id = read_uuid(r).ok_or_else(|| rejected("id is missing"))?;
    ObjectId::from_uuid(id).map_err(rejected)
}

/// Write one task.
pub(crate) fn write_task(mut out: task::Builder<'_>, entry: &TaskEntry) {
    write_envelope(out.reborrow().init_envelope(), &entry.envelope);
    out.set_jscalendar(entry.task.to_value().to_string().as_str());
    out.set_revision(Revision::of(entry).get());
}

/// Read the JSCalendar text of a request.
pub(crate) fn read_task(text: capnp::text::Reader<'_>) -> capnp::Result<Task> {
    let text = text.to_str()?;
    check_intake(text.len())?;
    let value: serde_json::Value = serde_json::from_str(text).map_err(rejected)?;
    Task::from_value(value).map_err(|e| crate::errors::rpc_error(e.into()))
}

/// A required text of a request, not empty.
pub(crate) fn required<'a>(text: capnp::text::Reader<'a>, what: &str) -> capnp::Result<&'a str> {
    let text = text.to_str()?;
    if text.is_empty() {
        return Err(rejected(format!("{what} must not be empty")));
    }
    check_intake(text.len())?;
    Ok(text)
}

pub(crate) fn count(n: usize) -> capnp::Result<u32> {
    u32::try_from(n).map_err(|_| capnp::Error::failed("list too long".into()))
}
