//! The optional facts of a card, as the text values of the form `cards-in-columns@1.1`: `priority`, `participants`, `keywords`,
//! `description`. They come from the JSCalendar task the contract carries (RFC 8984), a fact the task does not have is not sent.
//!
//! Pattern: Adapter (typed task member to the text of a form field; nothing is invented, `note` has no member in the contract). `description` goes out as it is, whatever its `descriptionContentType` (text/html stays raw).

use fs_tasks_core::Task;

/// The facts that are set on `task`, in the order of the form's field list.
pub(crate) fn of(task: &Task) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    // RFC 8984 section 4.4.1: priority 0 means "undefined", so it is no fact
    if let Some(priority) = task.priority.filter(|p| *p != 0) {
        out.push(("priority", priority.to_string()));
    }
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
        task.priority = Some(0);
        assert_eq!(of(&task).len(), 0, "priority 0 is undefined, no fact");
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
                ("priority", "2".to_owned()),
                ("participants", "Ada".to_owned()),
                ("keywords", "a, c".to_owned()),
                ("description", "why".to_owned()),
            ]
        );
    }
}
