//! The small builder every sample task is made with.
//!
//! Pattern: Builder (consuming, chained); the fixed `created`/`updated` instant replaces `Task::new` (which takes
//! the clock and a fresh `uid`).

use fs_tasks_core::model::{Relation, Task};

use crate::all::uid_of;

/// The instant all sample tasks were created and last changed: "today" of the other samples (fs-events), 16.11.2026.
pub(crate) const STAMP: &str = "2026-11-16T00:00:00Z";

pub(crate) struct B(pub(crate) Task);

pub(crate) fn task(slug: &str, title: &str) -> B {
    let mut t = Task {
        uid: uid_of(slug),
        title: Some(title.into()),
        created: Some(STAMP.into()),
        updated: Some(STAMP.into()),
        progress: Some("needs-action".into()),
        ..Task::default()
    };
    t.normalise();
    B(t)
}

impl B {
    pub(crate) fn start(mut self, local: &str) -> Self {
        self.0.start = Some(local.into());
        self
    }
    pub(crate) fn due(mut self, local: &str) -> Self {
        self.0.due = Some(local.into());
        self
    }
    pub(crate) fn zone(mut self, zone: &str) -> Self {
        self.0.time_zone = Some(zone.into());
        self
    }
    pub(crate) fn all_day(mut self) -> Self {
        self.0.show_without_time = Some(true);
        self
    }
    pub(crate) fn est(mut self, iso: &str) -> Self {
        self.0.estimated_duration = Some(iso.into());
        self
    }
    pub(crate) fn prio(mut self, p: i32) -> Self {
        self.0.priority = Some(p);
        self
    }
    pub(crate) fn state(mut self, progress: &str, percent: u8) -> Self {
        self.0.progress = Some(progress.into());
        self.0.percent_complete = Some(percent);
        self
    }
    pub(crate) fn keyword(mut self, word: &str) -> Self {
        self.0.keywords.insert(word.into(), true);
        self
    }
    /// This task (the holder) is the predecessor of `slug`; `gap` is the lag as ISO 8601 (empty = none).
    ///
    /// # Panics
    /// If the relation or the gap of the sample is malformed (a typo in this crate).
    pub(crate) fn then(mut self, slug: &str, relation: Relation, gap: &str) -> Self {
        self.0
            .relate_with_gap(&uid_of(slug), relation, gap)
            .expect("a relation of the sample");
        self
    }
    pub(crate) fn done(self) -> Task {
        self.0
    }
}
