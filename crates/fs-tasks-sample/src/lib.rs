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

mod build;
mod people;
mod plan;
mod three;

pub use plan::{mini_plan, plan_uid};
pub use three::{board_example, three_tasks};

use fs_tasks_core::model::{Board, Task};

/// Everything the sample holds.
#[derive(Clone, Debug)]
pub struct Sample {
    /// The three example tasks, in a fixed order.
    pub three: Vec<Task>,
    /// The board they stand on.
    pub board: Board,
    /// The mini plan, in a fixed order (the milestone last).
    pub plan: Vec<Task>,
}

/// The whole sample.
#[must_use]
pub fn sample() -> Sample {
    Sample {
        three: three_tasks(),
        board: board_example(),
        plan: mini_plan(),
    }
}

/// The `uid` of a sample task: `urn:fs-testdata:<slug>` (the pattern of the other samples).
#[must_use]
pub fn uid_of(slug: &str) -> String {
    format!("urn:fs-testdata:{slug}")
}
