//! The whole sample in one value, and the `uid` rule.
//!
//! Pattern: Facade over the builders (`three_tasks`, `board_example`, `mini_plan`).

use fs_tasks_core::model::{Board, Task};

use crate::{board_example, mini_plan, three_tasks};

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
