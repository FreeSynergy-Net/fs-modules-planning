//! Two small writers of `common.capnp` that fs-rpc has, for the `fs-interfaces` of THIS crate.
//!
//! Pattern: Adapter. Why a copy: fs-rpc 0.1.4 (4f02342) still takes the types of `fs-interfaces` 12142143 (0.1.1), this crate those of
//! 766f94c (0.2.0, level 2); the lock holds both, and a `Builder` of one is not a `Builder` of the other. The file goes as soon as fs-rpc takes the same
//! pin (then `fs_rpc::write_envelope`, `fs_rpc::read_uuid` and `Problem::write` are used again).

use fs_interfaces::common_capnp::{envelope as wire, uuid as wire_uuid};
use fs_objects::Envelope;

fn set_list(mut out: capnp::text_list::Builder<'_>, items: &[String]) {
    for (i, s) in (0u32..).zip(items) {
        out.set(i, s.as_str());
    }
}

fn len(items: &[String]) -> u32 {
    u32::try_from(items.len()).unwrap_or(u32::MAX)
}

/// Read a uuid (all zero = `None`).
pub(crate) fn read_uuid(r: wire_uuid::Reader<'_>) -> Option<::uuid::Uuid> {
    let bits = (u128::from(r.get_hi()) << 64) | u128::from(r.get_lo());
    (bits != 0).then(|| ::uuid::Uuid::from_u128(bits))
}

/// Write the envelope of an object.
pub(crate) fn write_envelope(mut out: wire::Builder<'_>, e: &Envelope) {
    crate::level2::write_uuid(out.reborrow().init_id(), e.id.as_uuid().as_u128());
    out.set_iri(e.iri.as_deref().unwrap_or(""));
    out.set_type(e.object_type.as_str());
    out.set_name(e.name.as_str());
    out.set_attributed_to(e.attributed_to.as_deref().unwrap_or(""));
    set_list(out.reborrow().init_to(len(&e.to)), &e.to);
    set_list(out.reborrow().init_cc(len(&e.cc)), &e.cc);
    set_list(out.reborrow().init_bto(len(&e.bto)), &e.bto);
    set_list(out.reborrow().init_bcc(len(&e.bcc)), &e.bcc);
    set_list(out.reborrow().init_audience(len(&e.audience)), &e.audience);
    out.set_context(e.context.as_deref().unwrap_or(""));
    out.set_in_reply_to(e.in_reply_to.as_deref().unwrap_or(""));
    out.reborrow()
        .init_published()
        .set_unix_millis(e.published.timestamp_millis());
    out.reborrow()
        .init_updated()
        .set_unix_millis(e.updated.timestamp_millis());
    set_list(
        out.reborrow().init_sensitive(len(&e.sensitive)),
        &e.sensitive,
    );
    out.set_describes(e.describes.as_deref().unwrap_or(""));
    if let Some(t) = e.start_time {
        out.reborrow()
            .init_start_time()
            .set_unix_millis(t.timestamp_millis());
    }
    if let Some(t) = e.end_time {
        out.reborrow()
            .init_end_time()
            .set_unix_millis(t.timestamp_millis());
    }
}
