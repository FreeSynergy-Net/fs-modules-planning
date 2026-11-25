//! The sample is the same on every run, matches the table and the plan, and the plan holds what it promises.
//!
//! Pattern: Specification (one `coverage` rule over the plan; a plan without a dependency kind, the gap or the
//! milestone must fail it, proven below) and Golden comparison against the sample itself (parity, derived).

use chrono::{Duration, NaiveDateTime};
use fs_person_directory::{sample_address, sample_name, sample_ref};
use fs_tasks_core::model::{BoardTask, Task};
use fs_tasks_core::plan::{Edge, EdgeKind, edges_of};
use fs_tasks_core::validate;
use fs_tasks_sample::{mini_plan, plan_uid, sample, three_tasks, uid_of};
use fs_types::BasicValue;

fn local(text: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S").expect("a local time")
}

/// All four kinds, one lag, one milestone (`PT0S`, start = due): the first thing missing is the error.
fn coverage(plan: &[Task]) -> Result<(), String> {
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

#[test]
fn deterministic_and_equal_in_both_ways_of_asking() {
    assert_eq!(mini_plan(), mini_plan());
    assert_eq!(three_tasks(), three_tasks());
    let s = sample();
    assert_eq!(s.plan, mini_plan());
    assert_eq!(s.three, three_tasks());
    assert_eq!(s.board, fs_tasks_sample::board_example());
}

#[test]
fn the_three_tasks_are_the_table() {
    let t = three_tasks();
    let [report, shopping, tax] = [&t[0], &t[1], &t[2]];
    assert_eq!(report.uid, "urn:fs-testdata:task-report");
    assert_eq!(report.title.as_deref(), Some("Bericht schreiben"));
    assert_eq!(report.due.as_deref(), Some("2026-11-20T17:00:00"));
    assert_eq!(report.time_zone.as_deref(), Some("Europe/Berlin"));
    assert_eq!(report.estimated_duration.as_deref(), Some("PT3H"));
    assert_eq!(
        (report.priority, report.percent_complete),
        (Some(1), Some(40))
    );
    assert_eq!(report.progress.as_deref(), Some("in-process"));
    // start = due - estimatedDuration (the bar of the calendar): 14:00 to 17:00.
    assert_eq!(
        local(report.start.as_deref().unwrap()) + Duration::hours(3),
        local(report.due.as_deref().unwrap())
    );
    assert_eq!(shopping.uid, uid_of("task-shopping"));
    assert!(
        shopping.due.is_none() && shopping.start.is_none() && shopping.estimated_duration.is_none()
    );
    assert_eq!(
        (shopping.priority, shopping.progress.as_deref()),
        (Some(5), Some("needs-action"))
    );
    assert_eq!(tax.due.as_deref(), Some("2026-11-30T00:00:00"));
    assert_eq!(
        (tax.show_without_time, tax.time_zone.clone()),
        (Some(true), None)
    );
    assert_eq!(tax.priority, Some(3));
    for (task, word) in [
        (report, "Arbeit"),
        (shopping, "Haushalt"),
        (tax, "Finanzen"),
    ] {
        assert!(task.keywords.get(word) == Some(&true), "{word}");
    }
}

#[test]
fn board_puts_the_cards_in_their_columns() {
    let s = sample();
    let cards: Vec<BoardTask> = s
        .three
        .iter()
        .map(|t| BoardTask {
            uid: t.uid.clone(),
            progress: t.effective_progress().to_owned(),
        })
        .collect();
    let layout = s.board.layout(&cards);
    let names: Vec<(&str, &[String])> = layout
        .columns
        .iter()
        .map(|c| (c.column.name.as_str(), c.cards.as_slice()))
        .collect();
    assert_eq!(
        names[0],
        ("Offen", &[uid_of("task-shopping"), uid_of("task-tax")][..])
    );
    assert_eq!(names[1], ("In Arbeit", &[uid_of("task-report")][..]));
}

#[test]
fn every_task_is_valid_and_ids_are_unique() {
    let s = sample();
    let all: Vec<&Task> = s.three.iter().chain(&s.plan).collect();
    for t in &all {
        assert!(
            validate::issues(t).is_empty(),
            "{}: {:?}",
            t.uid,
            validate::issues(t)
        );
    }
    let mut uids: Vec<&str> = all.iter().map(|t| t.uid.as_str()).collect();
    uids.sort_unstable();
    uids.dedup();
    assert_eq!(uids.len(), all.len());
}

#[test]
fn the_plan_has_all_four_kinds_a_gap_and_a_milestone() {
    let plan = mini_plan();
    assert_eq!(plan.len(), 6);
    coverage(&plan).expect("the sample plan");
}

#[test]
fn coverage_fails_when_a_kind_the_gap_or_the_milestone_is_missing() {
    let plan = mini_plan();
    let without = |slug: &str| -> Vec<Task> {
        plan.iter()
            .filter(|t| t.uid != plan_uid(slug))
            .cloned()
            .collect()
    };
    // No milestone.
    assert!(
        coverage(&without("freigabe"))
            .unwrap_err()
            .contains("milestone")
    );
    // Strip the relations of one holder at a time: each of these removes a kind or the gap.
    let strip = |slug: &str| -> Vec<Task> {
        let mut p = plan.clone();
        p.iter_mut()
            .filter(|t| t.uid == plan_uid(slug))
            .for_each(|t| t.related_to.clear());
        p
    };
    assert!(
        coverage(&strip("texte"))
            .unwrap_err()
            .contains("FinishToFinish")
    );
    assert!(
        coverage(&strip("konzept")).is_ok(),
        "FS is still there (design to aufbau, design to freigabe)"
    );
    let mut lacks_sf = strip("aufbau");
    lacks_sf.retain(|t| t.uid != plan_uid("freigabe"));
    assert!(coverage(&lacks_sf).is_err());
    let mut lacks_ss = plan.clone();
    lacks_ss
        .iter_mut()
        .filter(|t| t.uid == plan_uid("design"))
        .for_each(|t| t.related_to.clear());
    assert!(coverage(&lacks_ss).unwrap_err().contains("StartToStart"));
    // No gap: all relations without a lag.
    let mut no_gap = plan;
    for t in &mut no_gap {
        let uids: Vec<String> = t.related_to.keys().cloned().collect();
        for u in uids {
            if let Some(e) = t.related_to.get_mut(&u).and_then(|v| v.as_object_mut()) {
                e.remove(fs_tasks_core::model::GAP_KEY);
            }
        }
    }
    assert!(coverage(&no_gap).unwrap_err().contains("gap"));
}

#[test]
fn the_times_keep_every_tie_of_the_plan() {
    let plan = mini_plan();
    let find = |uid: &str| plan.iter().find(|t| t.uid == uid).expect("a plan task");
    let start = |t: &Task| local(t.start.as_deref().unwrap());
    let due = |t: &Task| local(t.due.as_deref().unwrap());
    for Edge {
        from,
        to,
        kind,
        gap,
    } in edges_of(&plan)
    {
        let (a, b) = (find(&from), find(&to));
        let (earliest, actual) = match kind {
            EdgeKind::FinishToStart => (due(a), start(b)),
            EdgeKind::StartToStart => (start(a), start(b)),
            EdgeKind::FinishToFinish => (due(a), due(b)),
            EdgeKind::StartToFinish => (start(a), due(b)),
        };
        assert!(actual >= earliest + gap, "{from} {kind:?} {to}");
    }
    for t in &plan {
        let est =
            fs_jscalendar::time::Dur::parse(t.estimated_duration.as_deref().unwrap()).unwrap();
        let secs = est.days * 86_400 + est.secs;
        assert_eq!(start(t) + Duration::seconds(secs), due(t), "{}", t.uid);
    }
}

#[test]
fn people_come_from_the_person_directory_sample() {
    let plan = mini_plan();
    let mut seen = 0;
    for t in &plan {
        for p in t.participants.values() {
            let iri = fs_tasks_core::model::actor_of(p).expect("an actor");
            let n = (1..=12u8)
                .find(|n| sample_ref(*n).is_some_and(|r| r.id.canonical() == iri))
                .expect("a person of the sample");
            assert_eq!(p.name.as_deref(), sample_name(n));
            assert_eq!(p.calendar_address.as_deref(), sample_address(n));
            seen += 1;
        }
    }
    assert_eq!(seen, 4);
    assert!(
        three_tasks().iter().all(|t| t.participants.is_empty()),
        "the table names no one"
    );
}
