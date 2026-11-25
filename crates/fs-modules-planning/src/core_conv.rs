//! The values of the module contract (`fs_interfaces::module`) from what the store and the commands say.
//!
//! Pattern: Adapter (log entry -> `ChangeRef`, problem -> `Refusal`, outcome -> `Applied`); no behaviour of its own.

use fs_interfaces::module::{
    Applied, ChangeKind, ChangeRef, Conflict, FieldClash, Origin, Refusal,
};
use fs_objects::Activity;
use fs_rpc::Problem;
use fs_types::{BasicValue as _, Utc};

use crate::changes::Seen;
use crate::write::Done;

const PREFIX: &str = "form-source-";

/// A problem of the commands as the refusal of the contract: the kind without the prefix of the wire (the wrapper adds it again);
/// a type that is no kind of this contract (a store failure) is `internal`, its status and text stay.
pub(crate) fn refusal_of(p: &Problem) -> Refusal {
    let mut r = Refusal::default();
    p.problem_type
        .strip_prefix(PREFIX)
        .unwrap_or("internal")
        .clone_into(&mut r.kind);
    r.status = p.status;
    r.detail.clone_from(&p.detail);
    r
}

/// An RPC error of a read (a problem inside, or a broken store) as the refusal.
pub(crate) fn refusal_of_error(e: &capnp::Error) -> Refusal {
    Problem::from_error(e).map_or_else(
        || {
            let mut r = Refusal::default();
            "internal".clone_into(&mut r.kind);
            r.status = 500;
            r.detail = e.to_string();
            r
        },
        |p| refusal_of(&p),
    )
}

fn kind_of(a: &Activity) -> ChangeKind {
    match a.kind.as_str() {
        "Create" => ChangeKind::Create,
        "Delete" => ChangeKind::Delete,
        "Undo" => ChangeKind::Undo,
        _ => ChangeKind::Update,
    }
}

/// A head activity as the `ChangeRef`: the record it is about (`seen`), the revisions, who and from where, what and when.
pub(crate) fn change_ref(a: &Activity, seen: &Seen) -> ChangeRef {
    let before = if a.kind.as_str() == "Create" {
        String::new()
    } else {
        a.revision.saturating_sub(1).to_string()
    };
    let person = a
        .owner
        .as_ref()
        .map_or_else(|| a.actor.clone(), |p| p.id.canonical());
    let origin = Origin::new(
        &person,
        &a.origin.program,
        &a.origin.instance,
        &a.origin.device,
    );
    let at = Utc::from_millis(a.at.timestamp_millis());
    let change = ChangeRef::new(
        &a.id.as_uuid().to_string(),
        &seen.object,
        &a.revision.to_string(),
        &before,
        origin,
        kind_of(a),
        at,
    );
    match seen.undoes {
        Some(bits) => change.undoing(&uuid::Uuid::from_u128(bits).to_string()),
        None => change,
    }
}

/// What a command made, as `Applied`.
pub(crate) fn applied(done: &Done, seen: &Seen) -> Applied {
    let mut a = Applied::default();
    a.record.clone_from(&done.record);
    done.text.clone_into(&mut a.text);
    a.change = done.head.as_ref().map(|head| change_ref(head, seen));
    a
}

/// The conflict of an `expect` or an undo: 409 with the last change and the paths that changed since.
pub(crate) fn clash(by: &Activity, seen: &Seen, paths: &[String]) -> Refusal {
    let fields = paths
        .iter()
        .map(|p| FieldClash::new(p, "", "").on(&seen.object))
        .collect();
    Refusal::conflict(
        "the record was changed since it was read",
        Conflict::new(change_ref(by, seen), fields),
    )
}
