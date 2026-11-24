//! The optional facts of a card, as the text values of the form `cards-in-columns@1.3`: `participants`, `keywords`, `description`, and the own
//! channels `priority`, `color`, `estimated`. They come from the JSCalendar task the contract carries (RFC 8984), a fact the task does not have is not sent.
//!
//! Pattern: Adapter (typed task member to the text of a form field; nothing is invented, `note` has no member in the contract). `description` goes out as it is, whatever its `descriptionContentType` (text/html stays raw).

use fs_tasks_core::Task;

/// The own channels of `cards-in-columns@1.3.0` that are set on `task`: `priority` (RFC 8984 section 4.4.1, 1..9; 0 is "not set", other
/// integers are reserved and not sent), `color` (section 4.2.11, as the member `color` the task carries; not typed in the model, so it is read as the
/// standard member of the open map) and `estimated` (section 4.2.2 `estimatedDuration`, ISO 8601 as stored).
pub(crate) fn channels(task: &Task) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    if let Some(priority) = task.priority.filter(|p| (1..=9).contains(p)) {
        out.push(("priority", priority.to_string()));
    }
    if let Some(color) = task
        .vendor("color")
        .and_then(|v| v.as_str())
        .filter(|c| !c.is_empty())
    {
        out.push(("color", color.to_owned()));
    }
    if let Some(span) = task.estimated_duration.as_deref().filter(|d| !d.is_empty()) {
        out.push(("estimated", span.to_owned()));
    }
    out
}

/// The facts that are set on `task`, in the order of the form's field list.
pub(crate) fn of(task: &Task) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let names: Vec<&str> = task
        .participants
        .values()
        .filter_map(|p| p.name.as_deref().or(p.email.as_deref()))
        .collect();
    if !names.is_empty() {
        out.push(("participants", names.join(", ")));
    }
    let words: Vec<&str> = task
        .keywords
        .iter()
        .filter(|(_, on)| **on)
        .map(|(k, _)| k.as_str())
        .collect();
    if !words.is_empty() {
        out.push(("keywords", words.join(", ")));
    }
    if let Some(text) = task.description.as_deref().filter(|d| !d.is_empty()) {
        out.push(("description", text.to_owned()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::of;
    use fs_tasks_core::{Participant, Task};

    #[test]
    fn a_bare_task_has_no_facts_and_a_full_one_has_all_four() {
        let mut task = Task::new("t");
        assert_eq!(of(&task).len(), 0);
        task.priority = Some(2);
        task.description = Some("why".into());
        task.keywords.insert("a".into(), true);
        task.keywords.insert("b".into(), false);
        task.keywords.insert("c".into(), true);
        let who = Participant {
            name: Some("Ada".into()),
            ..Default::default()
        };
        task.participants.insert("p1".into(), who);
        assert_eq!(
            of(&task),
            vec![
                ("participants", "Ada".to_owned()),
                ("keywords", "a, c".to_owned()),
                ("description", "why".to_owned()),
            ]
        );
    }
}

#[cfg(test)]
mod channel_tests {
    use super::channels;
    use fs_tasks_core::Task;

    #[test]
    fn each_own_channel_is_sent_only_when_the_task_has_it() {
        let mut task = Task::new("t");
        assert_eq!(channels(&task).len(), 0);
        task.priority = Some(0);
        assert_eq!(channels(&task).len(), 0, "0 = not set");
        task.priority = Some(10);
        assert_eq!(channels(&task).len(), 0, "10 is reserved by RFC 8984");
        task.priority = Some(1);
        task.estimated_duration = Some("PT1H30M".into());
        task.set_vendor("color", serde_json::json!("#ff8800"));
        assert_eq!(
            channels(&task),
            vec![
                ("priority", "1".to_owned()),
                ("color", "#ff8800".to_owned()),
                ("estimated", "PT1H30M".to_owned()),
            ]
        );
        task.set_vendor("color", serde_json::json!(""));
        assert_eq!(channels(&task).len(), 2, "an empty colour is not sent");
    }
}
