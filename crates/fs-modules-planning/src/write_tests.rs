//! Unit tests of `may` and `permit` with a provider that supports less than the real one (the real one supports all three).
//!
//! Pattern: table test (a fake `Capabilities` per row).

use crate::write::{FORBIDDEN, RECORD_COMMANDS, SET_COMMANDS, UNKNOWN_COMMAND, may, permit};
use fs_interfaces::Capabilities;

fn caps(supported: &[&str]) -> Capabilities {
    let mut c = Capabilities::new("test");
    for s in supported {
        c.supported(*s);
    }
    c
}

#[test]
fn may_is_what_the_provider_supports_and_nothing_else() {
    let all = caps(&["card.move", "task.delete", "task.create"]);
    assert_eq!(may(&all, &RECORD_COMMANDS), ["card.move", "task.delete"]);
    assert_eq!(may(&all, &SET_COMMANDS), ["task.create"]);
    let some = caps(&["card.move"]);
    assert_eq!(may(&some, &RECORD_COMMANDS), ["card.move"]);
    assert_eq!(may(&some, &SET_COMMANDS), Vec::<&str>::new());
    assert_eq!(may(&caps(&[]), &RECORD_COMMANDS), Vec::<&str>::new());
}

#[test]
fn invoke_refuses_what_may_does_not_say() {
    let some = caps(&["card.move"]);
    assert!(permit(&some, "card.move").is_ok());
    let no = permit(&some, "task.delete").unwrap_err();
    assert_eq!((no.problem_type.as_str(), no.status), (FORBIDDEN, 403));
    let named = permit(&some, "planning.card.move").unwrap_err();
    assert_eq!(named.problem_type, UNKNOWN_COMMAND);
}
