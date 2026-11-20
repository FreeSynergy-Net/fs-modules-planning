//! Ids, envelopes and tasks on the wire.
//!
//! Pattern: Adapter — `ObjectId`, `Envelope`, `TaskEntry` between Rust and `common.capnp` / `tasks.capnp`. The JSCalendar JSON stays
//! authoritative for the content. fs-rpc writes envelopes for its own fs-objects; the core of this repo is on another one, so the
//! few lines are here.

use fs_interfaces::common_capnp::{envelope, uuid};
use fs_interfaces::tasks_capnp::task;
use fs_objects::{Envelope, ObjectId};
use fs_rpc::{check_intake, read_uuid, to_millis, write_uuid};
use fs_tasks_core::{Revision, Task, TaskEntry};

use crate::errors::rejected;

/// Read an object id; none or not a valid id is a rejected request.
pub(crate) fn read_id(r: uuid::Reader<'_>) -> capnp::Result<ObjectId> {
    let id = read_uuid(r).ok_or_else(|| rejected("id is missing"))?;
    ObjectId::from_uuid(id).map_err(rejected)
}

/// Write an object id.
pub(crate) fn write_id(out: uuid::Builder<'_>, id: ObjectId) {
    write_uuid(out, Some(id.as_uuid()));
}

fn texts(mut out: capnp::text_list::Builder<'_>, items: &[String]) {
    for (i, s) in (0u32..).zip(items) {
        out.set(i, s.as_str());
    }
}

fn len(items: &[String]) -> u32 {
    u32::try_from(items.len()).unwrap_or(u32::MAX)
}

/// Write the envelope of an object.
pub(crate) fn write_envelope(mut out: envelope::Builder<'_>, e: &Envelope) {
    write_id(out.reborrow().init_id(), e.id);
    out.set_iri(e.iri.as_deref().unwrap_or(""));
    out.set_type(e.object_type.as_str());
    out.set_name(e.name.as_str());
    out.set_attributed_to(e.attributed_to.as_deref().unwrap_or(""));
    texts(out.reborrow().init_to(len(&e.to)), &e.to);
    texts(out.reborrow().init_cc(len(&e.cc)), &e.cc);
    texts(out.reborrow().init_bto(len(&e.bto)), &e.bto);
    texts(out.reborrow().init_bcc(len(&e.bcc)), &e.bcc);
    texts(out.reborrow().init_audience(len(&e.audience)), &e.audience);
    out.set_context(e.context.as_deref().unwrap_or(""));
    out.set_in_reply_to(e.in_reply_to.as_deref().unwrap_or(""));
    out.reborrow()
        .init_published()
        .set_unix_millis(to_millis(e.published));
    out.reborrow()
        .init_updated()
        .set_unix_millis(to_millis(e.updated));
    texts(out.reborrow().init_sensitive(len(&e.sensitive)), &e.sensitive);
    out.set_describes(e.describes.as_deref().unwrap_or(""));
    if let Some(t) = e.start_time {
        out.reborrow().init_start_time().set_unix_millis(to_millis(t));
    }
    if let Some(t) = e.end_time {
        out.reborrow().init_end_time().set_unix_millis(to_millis(t));
    }
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
