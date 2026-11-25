//! The three example tasks (`TESTDATEN-AUFGABEN.md`) and the board they stand on.
//!
//! Pattern: Object Mother (fixed data, one builder chain per task). Slugs and fields as the table says; the table names
//! no assignee, so none is invented here.

use fs_tasks_core::model::{Board, Column, Task};

use crate::build::task;

/// Report (due with duration), shopping (no date), tax (all-day due): in this order.
#[must_use]
pub fn three_tasks() -> Vec<Task> {
    vec![
        task("task-report", "Bericht schreiben")
            .start("2026-11-20T14:00:00")
            .due("2026-11-20T17:00:00")
            .zone("Europe/Berlin")
            .est("PT3H")
            .prio(1)
            .state("in-process", 40)
            .keyword("Arbeit")
            .done(),
        task("task-shopping", "Einkaufen")
            .prio(5)
            .state("needs-action", 0)
            .keyword("Haushalt")
            .done(),
        task("task-tax", "Steuererklärung")
            .due("2026-11-30T00:00:00")
            .all_day()
            .prio(3)
            .state("needs-action", 0)
            .keyword("Finanzen")
            .done(),
    ]
}

/// The board "Beispiel": "Offen" (`needs-action`) and "In Arbeit" (`in-process`); a card stands where its `progress`
/// says, so no place is stored.
#[must_use]
pub fn board_example() -> Board {
    Board {
        columns: vec![
            Column::new("offen", "Offen", &["needs-action"]),
            Column::new("in-arbeit", "In Arbeit", &["in-process"]),
        ],
        ..Board::standard("Beispiel")
    }
}
