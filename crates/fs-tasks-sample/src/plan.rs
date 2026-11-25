//! The mini plan: six tasks, all four dependencies of RFC 9253, one lag, one milestone.
//!
//! Pattern: Object Mother (fixed data). The holder of a relation is the predecessor (`fs-tasks-core::Relation`).
//! Every task has `start`, `due` (= `start` + `estimatedDuration`) in Europe/Berlin:
//!
//! ```text
//! konzept   16.11. 09:00 PT8H   --FS-->   design   17.11. 09:00 PT24H
//! design    --SS lag PT4H-->      texte    17.11. 13:00 PT20H
//! design    --FS lag P1D-->       aufbau   19.11. 09:00 PT48H
//! texte     --FF-->               aufbau
//! aufbau    --SF-->               altseite 18.11. 09:00 PT48H
//! aufbau    --FS-->               freigabe 21.11. 09:00 PT0S (milestone)
//! ```

use fs_tasks_core::model::{Relation, Task};

use crate::build::{B, task};
use crate::people::assign;
use crate::uid_of;

const ZONE: &str = "Europe/Berlin";

fn at(b: B, start: &str, due: &str, est: &str) -> B {
    b.start(start).due(due).zone(ZONE).est(est)
}

/// The `uid` of a plan task by its slug (`plan-konzept` and so on).
#[must_use]
pub fn plan_uid(slug: &str) -> String {
    uid_of(&format!("plan-{slug}"))
}

/// The six tasks, the milestone last.
#[must_use]
pub fn mini_plan() -> Vec<Task> {
    use Relation::{
        FinishToFinish as FF, FinishToStart as FS, StartToFinish as SF, StartToStart as SS,
    };
    let to = |slug: &str| format!("plan-{slug}");
    let mut konzept = at(
        task("plan-konzept", "Konzept schreiben"),
        "2026-11-16T09:00:00",
        "2026-11-16T17:00:00",
        "PT8H",
    )
    .state("completed", 100)
    .prio(2)
    .then(&to("design"), FS, "")
    .done();
    let mut design = at(
        task("plan-design", "Entwurf zeichnen"),
        "2026-11-17T09:00:00",
        "2026-11-18T09:00:00",
        "PT24H",
    )
    .state("in-process", 50)
    .prio(2)
    .then(&to("texte"), SS, "PT4H")
    .then(&to("aufbau"), FS, "P1D")
    .done();
    let mut texte = at(
        task("plan-texte", "Texte schreiben"),
        "2026-11-17T13:00:00",
        "2026-11-18T09:00:00",
        "PT20H",
    )
    .prio(3)
    .then(&to("aufbau"), FF, "")
    .done();
    let mut aufbau = at(
        task("plan-aufbau", "Seite aufbauen"),
        "2026-11-19T09:00:00",
        "2026-11-21T09:00:00",
        "PT48H",
    )
    .prio(3)
    .then(&to("altseite"), SF, "")
    .then(&to("freigabe"), FS, "")
    .done();
    let altseite = at(
        task("plan-altseite", "Alte Seite weiterbetreiben"),
        "2026-11-18T09:00:00",
        "2026-11-20T09:00:00",
        "PT48H",
    )
    .prio(5)
    .done();
    let freigabe = at(
        task("plan-freigabe", "Freigabe (Meilenstein)"),
        "2026-11-21T09:00:00",
        "2026-11-21T09:00:00",
        "PT0S",
    )
    .prio(1)
    .done();
    assign(&mut konzept, 1);
    assign(&mut design, 1);
    assign(&mut texte, 12);
    assign(&mut aufbau, 12);
    vec![konzept, design, texte, aufbau, altseite, freigabe]
}
