//! `fs-tasks-sample`: the fixed sample data of the planning module, as the module holds them (JSCalendar `Task`, RFC 8984,
//! with `relatedTo` of RFC 9253), for the frontends and for every test that needs "the same tasks".
//!
//! Pattern: Object Mother (one well-known set, built by pure functions: no clock, no randomness, the same `uid`s and
//! times on every run). Contract used as it is: `fs-tasks-core::Task`, `Board`, `Relation`; nothing new is invented.
//!
//! * [`three_tasks`]: the three example tasks of `TESTDATEN-AUFGABEN.md` (due with duration, undated, all-day due).
//! * [`board_example`]: the board "Beispiel" (columns "Offen", "In Arbeit") they stand on.
//! * [`mini_plan`]: six tasks tied by all four kinds of dependency, one lag and one milestone (`PT0S`).
//! * [`sample`]: all of it in one value; people come from `fs-person-directory` (`people.rs`).

mod all;
mod build;
mod coverage;
mod people;
mod plan;
mod three;

pub use all::{Sample, sample, uid_of};
pub use coverage::coverage;
pub use plan::{mini_plan, plan_uid};
pub use three::{board_example, three_tasks};
