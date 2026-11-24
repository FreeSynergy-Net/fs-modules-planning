//! The program `fs-modules-planning`: the service of the planning module. All of it is in the library.
//!
//! Pattern: Facade — one call, with the source of the local owner (`LocalOwner`, Strategy). The provider that reads the owner from the module
//! `person` is the next package (S3-7b); until it is connected this binary says so and does not start, never with a made-up owner.

use fs_interfaces::owner::{LocalOwner, OwnerProblem};
use fs_objects::PersonRef;

/// The owner source of this build: the module `person` is not connected yet.
struct NotConnected;

impl LocalOwner for NotConnected {
    fn owner(&self) -> Result<PersonRef, OwnerProblem> {
        Err(OwnerProblem::Unreachable {
            reason: "the module person is not connected in this build".into(),
        })
    }
}

fn main() {
    std::process::exit(fs_modules_planning::serve_command(
        std::env::args_os().skip(1),
        &NotConnected,
    ));
}
