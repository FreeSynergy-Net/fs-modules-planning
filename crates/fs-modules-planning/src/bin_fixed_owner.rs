//! The service with a FIXED owner, for the tests that run it as a process (feature `fake-owner`, never shipped).
//!
//! Pattern: Facade — the same call as `main.rs`, with the test source of the owner (`FixedOwner`, Strategy).

use fs_interfaces::owner::fake::{FixedOwner, sample};

fn main() {
    std::process::exit(fs_modules_planning::serve_command(
        std::env::args_os().skip(1),
        &FixedOwner(sample()),
    ));
}
