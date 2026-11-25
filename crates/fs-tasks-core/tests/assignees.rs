//! `assignees()` (role `chair`), `assign` and `assign_actor` write `chair` (not `owner`), `Board.lanes` is appended.

use fs_jscalendar::model::Participant;
use fs_tasks_core::Task;
use fs_tasks_core::model::{ActorType, Board};

#[test]
fn assign_actor_sets_chair_and_not_owner() {
    let mut t = Task::new("T");
    let k = t.assign_actor("https://x.invalid/a", ActorType::Person, "Ada");
    let p = &t.participants[&k];
    assert_eq!(p.roles.get("chair"), Some(&true));
    assert_eq!(p.roles.get("owner"), None);
    assert_eq!(t.assignees().len(), 1);
}

#[test]
fn assign_sets_chair_and_not_owner() {
    let mut t = Task::new("T");
    let k = t.assign("Ben", "ben@example.org");
    assert_eq!(t.participants[&k].roles.get("chair"), Some(&true));
    assert_eq!(t.participants[&k].roles.get("owner"), None);
    // An existing participant (owner only) gets chair added when assigned.
    let mut u = Task::new("U");
    u.participants.insert(
        "p1".into(),
        Participant::person("Cy", "cy@example.org", "owner"),
    );
    let k = u.assign("Cy", "cy@example.org");
    assert_eq!(u.participants[&k].roles.get("chair"), Some(&true));
}

#[test]
fn an_owner_only_participant_is_not_an_assignee_and_actors_is_unchanged() {
    let mut t = Task::new("T");
    t.participants.insert(
        "p1".into(),
        Participant::person("Own", "own@example.org", "owner"),
    );
    t.assign_actor("https://x.invalid/a", ActorType::Application, "Runner");
    let names: Vec<_> = t
        .assignees()
        .iter()
        .filter_map(|p| p.name.clone())
        .collect();
    assert_eq!(names, vec!["Runner".to_string()]);
    assert_eq!(t.actors(), vec!["https://x.invalid/a"]);
}

#[test]
fn a_board_without_lanes_loads_and_lanes_round_trip() {
    let plain = serde_json::to_value(Board::standard("x")).unwrap();
    assert!(plain.get("lanes").is_none(), "empty lanes are not written");
    let b: Board = serde_json::from_value(plain).unwrap();
    assert!(b.lanes.is_empty());
    let mut with = Board::standard("x");
    with.lanes
        .insert("assignee".into(), vec!["b".into(), "a".into()]);
    let back: Board = serde_json::from_str(&serde_json::to_string(&with).unwrap()).unwrap();
    assert_eq!(back, with);
}
