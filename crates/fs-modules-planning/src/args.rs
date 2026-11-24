//! Reading the arguments of a command: text values by path, strict (unknown, doubled, missing or a number place is refused with its reason).
//!
//! Pattern: Adapter (the wire values of `FormSource.invoke` as a checked map of text). Every value of a command is text.

use std::collections::BTreeMap;

use fs_interfaces::formsource_capnp::value;

use crate::write::{BAD_ARGS, Failure, UID_MAX, boxed};

/// A place by a number is never taken (S1 E3): the caller names the neighbour cards (`after`, `before`).
const NUMBER_PLACES: [&str; 2] = ["order", "index"];

/// The text values of a request by path. A path that is not in `allowed` or that comes twice is refused with the reason, never
/// silently dropped.
pub(crate) fn text_args(
    list: capnp::struct_list::Reader<'_, value::Owned>,
    allowed: &[&str],
) -> Result<BTreeMap<String, String>, Failure> {
    let mut out = BTreeMap::new();
    for v in list {
        let path = v
            .get_path()
            .and_then(|p| p.to_string().map_err(capnp::Error::from))
            .map_err(|e| bad_wire(&e))?;
        let text = v
            .get_text()
            .and_then(|t| t.to_string().map_err(capnp::Error::from))
            .map_err(|e| bad_wire(&e))?;
        if NUMBER_PLACES.contains(&path.as_str()) {
            let why = format!(
                "{path:?} is not taken: the place is given by the neighbour cards `after` and `before` (record ids), never by a number"
            );
            return Err(boxed(BAD_ARGS, why, 400));
        }
        if !allowed.contains(&path.as_str()) {
            let why = format!(
                "unknown argument {path:?} (allowed: {})",
                allowed.join(", ")
            );
            return Err(boxed(BAD_ARGS, why, 400));
        }
        if out.insert(path.clone(), text).is_some() {
            return Err(boxed(
                BAD_ARGS,
                format!("argument {path:?} given twice"),
                400,
            ));
        }
    }
    Ok(out)
}

pub(crate) fn take(got: &mut BTreeMap<String, String>, path: &str) -> Result<String, Failure> {
    got.remove(path)
        .ok_or_else(|| boxed(BAD_ARGS, format!("`{path}` is missing"), 400))
}

/// A neighbour card (record id) or none: an empty text is none (a card at the top or the end has no neighbour on that side).
pub(crate) fn take_neighbour(got: &mut BTreeMap<String, String>, path: &str) -> Option<String> {
    got.remove(path).filter(|t| !t.is_empty())
}

pub(crate) fn check_uid(uid: &str) -> Result<(), Failure> {
    if uid.is_empty() || uid.len() > UID_MAX {
        let why = format!("`record` must be a uid of 1 to {UID_MAX} bytes");
        return Err(boxed(BAD_ARGS, why, 400));
    }
    Ok(())
}

#[allow(clippy::unnecessary_box_returns)] // the error type of the argument readers is boxed (a `Problem` is large)
fn bad_wire(e: &capnp::Error) -> Failure {
    boxed(BAD_ARGS, e.to_string(), 400)
}
