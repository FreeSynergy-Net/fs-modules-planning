//! The optional facts of a card, as the text values of the form `cards-in-columns@1.4`: `participants`, `keywords`, `description`, and the own
//! channels `priority`, `color`, `estimated`. They come from the JSCalendar task the contract carries (RFC 8984), a fact the task does not have is not sent.
//!
//! Pattern: Adapter (typed task member to the text of a form field; nothing is invented, `note` has no member in the contract). `description` goes out as it is, whatever its `descriptionContentType` (text/html stays raw).

use fs_tasks_core::Task;
use fs_tasks_core::model::actor_of;

/// The own channels of `cards-in-columns@1.4.0` that are set on `task`: `priority` (RFC 8984 section 4.4.1, 1..9; 0 is "not set", other
/// integers are reserved and not sent), `color` (section 4.2.11, as the member `color` the task carries; not typed in the model, so it is read as the
/// standard member of the open map), `estimated` (section 5.2.3 `estimatedDuration`, ISO 8601 as stored), `state` (section 5.2.5 `progress`,
/// the valid word of `effective_progress()`; a missing `progress` is derived after section 5.2.5), `time-zone` (section 4.7.1 `timeZone`, only when set; `due` stays an instant with `Z`) and `assignee` (see
/// [`assignees`]). The values go out under the field names of the catalog (`estimatedDuration`, `progress`, `timeZone`, `assignee`).
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
        out.push(("estimatedDuration", span.to_owned()));
    }
    out.extend(state_word(task).map(|word| ("progress", word)));
    if let Some(zone) = task.time_zone.as_deref().filter(|z| !z.is_empty()) {
        out.push(("timeZone", zone.to_owned()));
    }
    out.extend(assignees(task).into_iter().map(|iri| ("assignee", iri)));
    out
}

/// The status word of the task (RFC 8984 section 5.2.5, `effective_progress()`): one of the five words of the RFC, or a vendor word
/// `<domain>:<name>`. A stored word that is neither is NOT sent (no value; nothing is guessed or mapped to another status).
pub(crate) fn state_word(task: &Task) -> Option<String> {
    const WORDS: [&str; 5] = [
        "needs-action",
        "in-process",
        "completed",
        "failed",
        "cancelled",
    ];
    let word = task.effective_progress();
    let vendor = word
        .split_once(':')
        .is_some_and(|(domain, name)| !domain.is_empty() && !name.is_empty());
    (WORDS.contains(&word) || vendor).then(|| word.to_owned())
}

/// The IRI of each person in charge (`Task::assignees()`, the one place that says "responsible"), in the order of the participant keys (a map: lexicographic, `p10` before `p2`): the actor IRI
/// (`freesynergy.net:actor`), else `mailto:` + the email (RFC 6068), else no value for that participant.
pub(crate) fn assignees(task: &Task) -> Vec<String> {
    task.assignees()
        .into_iter()
        .filter_map(|p| {
            actor_of(p)
                .filter(|a| !a.is_empty())
                .map(str::to_owned)
                .or_else(|| {
                    // `email` may already carry `mailto:` (as `calendar_address` does): never doubled
                    p.email
                        .as_deref()
                        .or(p.calendar_address.as_deref())
                        .map(|e| e.trim().trim_start_matches("mailto:"))
                        .filter(|e| !e.is_empty())
                        .map(|e| format!("mailto:{e}"))
                })
        })
        .collect()
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
        let bare = vec![("progress", "needs-action".to_owned())];
        assert_eq!(channels(&task), bare);
        task.priority = Some(0);
        assert_eq!(channels(&task), bare, "0 = not set");
        task.priority = Some(10);
        assert_eq!(channels(&task), bare, "10 is reserved by RFC 8984");
        task.priority = Some(1);
        task.estimated_duration = Some("PT1H30M".into());
        task.set_vendor("color", serde_json::json!("#ff8800"));
        assert_eq!(
            channels(&task),
            vec![
                ("priority", "1".to_owned()),
                ("color", "#ff8800".to_owned()),
                ("estimatedDuration", "PT1H30M".to_owned()),
                ("progress", "needs-action".to_owned()),
            ]
        );
        task.set_vendor("color", serde_json::json!(""));
        assert_eq!(channels(&task).len(), 3, "an empty colour is not sent");
    }

    fn named(task: &Task, path: &str) -> Vec<String> {
        channels(task)
            .into_iter()
            .filter(|(p, _)| *p == path)
            .map(|(_, v)| v)
            .collect()
    }

    #[test]
    fn the_zone_is_sent_only_when_set_and_due_keeps_its_instant() {
        let mut task = Task::new("t");
        assert_eq!(named(&task, "timeZone").len(), 0, "floating: no value");
        task.time_zone = Some("Europe/Berlin".into());
        assert_eq!(named(&task, "timeZone"), ["Europe/Berlin"]);
    }

    #[test]
    fn the_state_is_one_of_the_five_words_or_a_vendor_word() {
        let mut task = Task::new("t");
        assert_eq!(named(&task, "progress"), ["needs-action"]);
        for word in [
            "needs-action",
            "in-process",
            "completed",
            "failed",
            "cancelled",
            "freesynergy.net:blocked",
        ] {
            task.progress = Some(word.into());
            assert_eq!(named(&task, "progress"), [word]);
        }
        for bad in ["done", "COMPLETED", "x:", ":y"] {
            task.progress = Some(bad.into());
            assert_eq!(
                named(&task, "progress").len(),
                0,
                "{bad} is not a state, so it is not sent"
            );
        }
        // no stored word: derived after RFC 8984 5.2.5 from the participants
        task.progress = None;
        let mut done = fs_tasks_core::Participant::default();
        done.extra
            .insert("progress".into(), serde_json::json!("completed"));
        task.participants.insert("p1".into(), done);
        assert_eq!(named(&task, "progress"), ["completed"]);
    }

    #[test]
    fn each_person_in_charge_is_one_value_and_nobody_is_none() {
        let mut task = Task::new("t");
        assert_eq!(named(&task, "assignee").len(), 0);
        task.assign_actor(
            "freesynergy.net:person:ada",
            fs_tasks_core::model::ActorType::Person,
            "Ada",
        );
        task.assign_actor(
            "freesynergy.net:person:bob",
            fs_tasks_core::model::ActorType::Person,
            "Bob",
        );
        assert_eq!(
            named(&task, "assignee"),
            ["freesynergy.net:person:ada", "freesynergy.net:person:bob"]
        );
        // a participant without the role `chair` is no assignee; one with only an email goes out as `mailto:`; one with neither gets no value
        let mut mail = fs_tasks_core::Participant {
            email: Some("cy@example.org".into()),
            ..Default::default()
        };
        mail.roles.insert("chair".into(), true);
        let mut nothing = fs_tasks_core::Participant::default();
        nothing.roles.insert("chair".into(), true);
        let mut watcher = fs_tasks_core::Participant {
            email: Some("w@example.org".into()),
            ..Default::default()
        };
        watcher.roles.insert("attendee".into(), true);
        task.participants.insert("p8".into(), mail);
        task.participants.insert("p9".into(), nothing);
        task.participants.insert("p10".into(), watcher);
        let mut prefixed = fs_tasks_core::Participant {
            email: Some("mailto:dee@example.org".into()),
            ..Default::default()
        };
        prefixed.roles.insert("chair".into(), true);
        task.participants.insert("p11".into(), prefixed);
        let got = named(&task, "assignee");
        assert_eq!(got.len(), 4);
        assert!(got.contains(&"mailto:cy@example.org".to_owned()));
        assert!(
            got.contains(&"mailto:dee@example.org".to_owned()),
            "no doubled prefix"
        );
    }
}
