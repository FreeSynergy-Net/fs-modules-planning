//! The people of the sample tasks: taken from the person directory sample, never written down here.
//!
//! Pattern: Adapter (stub) over `fs_person_directory::{sample_ref, sample_name, sample_address}`: the ONE place of the
//! sample names and addresses; a task only holds the reference (the actor IRI) and the name the directory shows.

use fs_person_directory::{sample_address, sample_name, sample_ref};
use fs_tasks_core::model::{ActorType, Task};
use fs_types::BasicValue;

/// Entry `n` of the person directory sample is responsible for `task` (role `chair`): its reference as the actor IRI,
/// its name, its address (`mailto:`), as the directory has them.
///
/// # Panics
/// If `n` is not an entry of the sample (a typo in this crate).
pub(crate) fn assign(task: &mut Task, n: u8) {
    let who = sample_ref(n).expect("an entry of the person sample");
    let name = sample_name(n).expect("an entry of the person sample");
    let key = task.assign_actor(&who.id.canonical(), ActorType::Person, name);
    if let (Some(address), Some(p)) = (sample_address(n), task.participants.get_mut(&key)) {
        p.calendar_address = Some(address.into());
        p.email = address.strip_prefix("mailto:").map(Into::into);
    }
}
