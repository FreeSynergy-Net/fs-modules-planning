//! The coverage rule of the mini plan: what a plan must hold to be a good picture (Gantt, graph, calendar).
//!
//! Pattern: Specification (one rule over a plan; the first thing missing is named in the error).

use chrono::Duration;
use fs_tasks_core::model::Task;
use fs_tasks_core::plan::{EdgeKind, edges_of};

/// All four kinds of dependency, one lag, one milestone (`PT0S`, start = due): the first thing missing is the error.
///
/// # Errors
/// Names the missing kind (`FinishToStart`, `StartToStart`, `FinishToFinish`, `StartToFinish`), the gap or the milestone.
pub fn coverage(plan: &[Task]) -> Result<(), String> {
    let edges = edges_of(plan);
    for kind in [
        EdgeKind::FinishToStart,
        EdgeKind::StartToStart,
        EdgeKind::FinishToFinish,
        EdgeKind::StartToFinish,
    ] {
        if !edges.iter().any(|e| e.kind == kind) {
            return Err(format!("no dependency of kind {kind:?}"));
        }
    }
    if !edges.iter().any(|e| e.gap > Duration::zero()) {
        return Err("no gap (lag)".into());
    }
    let milestone = plan.iter().any(|t| {
        t.estimated_duration.as_deref() == Some("PT0S") && t.start == t.due && t.start.is_some()
    });
    if !milestone {
        return Err("no milestone (PT0S, start = due)".into());
    }
    Ok(())
}
