//! Unit test of `may` with a provider that supports less than the real one (the real one supports all three).
//!
//! Pattern: table test (a fake `Capabilities` per row).

use crate::write::{RECORD_COMMANDS, SET_COMMANDS, may};
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
    let all = caps(&["card.move", "task.delete", "task.edit", "task.create"]);
    assert_eq!(
        may(&all, &RECORD_COMMANDS),
        ["card.move", "task.delete", "task.edit"]
    );
    // only what the provider supports: without task.edit it is not offered
    let no_edit = caps(&["card.move", "task.delete"]);
    assert_eq!(
        may(&no_edit, &RECORD_COMMANDS),
        ["card.move", "task.delete"]
    );
    assert_eq!(may(&all, &SET_COMMANDS), ["task.create"]);
    let some = caps(&["card.move"]);
    assert_eq!(may(&some, &RECORD_COMMANDS), ["card.move"]);
    assert_eq!(may(&some, &SET_COMMANDS), Vec::<&str>::new());
    assert_eq!(may(&caps(&[]), &RECORD_COMMANDS), Vec::<&str>::new());
}
