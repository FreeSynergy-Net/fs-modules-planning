//! Reading the arguments of a command: text values by path, strict (unknown, doubled, missing or a number place is refused with its reason).
//!
//! Pattern: Adapter (the values of an `Invocation` as a checked map of text). Every value of a command is text.

use std::collections::BTreeMap;

use crate::write::{BAD_ARGS, Failure, UID_MAX, boxed};

/// A place by a number is never taken (S1 E3): the caller names the neighbour cards (`after`, `before`).
const NUMBER_PLACES: [&str; 2] = ["order", "index"];

/// The values of a request as (path, text) pairs: what `Guarded` hands to the commands (the `Invocation`).
pub(crate) type Pairs = [(String, String)];

/// The text values of a request by path. A path that is not in `allowed` or that comes twice is refused with the reason, never
/// silently dropped.
pub(crate) fn text_args(
    list: &Pairs,
    allowed: &[&str],
) -> Result<BTreeMap<String, String>, Failure> {
    let mut out = BTreeMap::new();
    for (path, text) in list {
        let (path, text) = (path.clone(), text.clone());
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

/// `expect` as the text of a revision (empty = any).
pub(crate) fn parse_expect(text: &str) -> Result<Option<u64>, Failure> {
    if text.is_empty() {
        return Ok(None);
    }
    text.parse()
        .map(Some)
        .map_err(|_| boxed(BAD_ARGS, "`expect` is not a revision (decimal text)", 400))
}

/// `step` as (run, step): both empty = no step; one of them empty is refused.
pub(crate) fn parse_step(run: &str, step: &str) -> Result<Option<(String, String)>, Failure> {
    match (run.is_empty(), step.is_empty()) {
        (true, true) => Ok(None),
        (false, false) => Ok(Some((run.to_owned(), step.to_owned()))),
        _ => Err(boxed(
            BAD_ARGS,
            "`step.run` and `step.step` are both given or both empty",
            400,
        )),
    }
}
