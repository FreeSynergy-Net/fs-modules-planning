//! Values are checked when they are written (RFC 8984): `color` and `priority`, next to `estimatedDuration`.
//!
//! Pattern: Specification tests — one valid and one invalid case per rule, the error is the usual `Invalid` with a path.

use fs_tasks_core::Task;
use fs_tasks_core::validate::{issues, validate};
use serde_json::json;

fn with(key: &str, v: serde_json::Value) -> Task {
    let mut t = Task::new("T");
    t.extra.insert(key.into(), v);
    t
}

fn paths(t: &Task) -> Vec<String> {
    issues(t).into_iter().map(|i| i.path).collect()
}

#[test]
fn color_names_and_hex_pass() {
    for c in ["red", "Red", "darkgoldenrod", "#fa0", "#FFAA00", "#00ff7f"] {
        validate(&with("color", json!(c))).unwrap_or_else(|e| panic!("{c}: {e}"));
    }
}

#[test]
fn color_other_forms_are_refused_with_path() {
    for c in [
        json!("banana"),
        json!("#12"),
        json!("#12345"),
        json!("#gggggg"),
        json!("ffaa00"),
        json!("rgb(1,2,3)"),
        json!("transparent"),
        json!(""),
        json!(5),
    ] {
        let t = with("color", c.clone());
        assert_eq!(paths(&t), ["/color"], "{c}");
        assert!(validate(&t).is_err(), "{c}");
    }
}

#[test]
fn priority_is_checked_on_write() {
    for p in 0..=9 {
        let mut t = Task::new("T");
        t.priority = Some(p);
        validate(&t).unwrap();
    }
    for p in [-1, 10, 99] {
        let mut t = Task::new("T");
        t.priority = Some(p);
        assert_eq!(paths(&t), ["/priority"], "{p}");
        assert!(validate(&t).is_err(), "{p}");
    }
}

#[test]
fn estimated_stays_checked() {
    let mut t = Task::new("T");
    t.estimated_duration = Some("1h30".into());
    assert_eq!(paths(&t), ["/estimatedDuration"]);
    t.estimated_duration = Some("PT1H30M".into());
    validate(&t).unwrap();
}
