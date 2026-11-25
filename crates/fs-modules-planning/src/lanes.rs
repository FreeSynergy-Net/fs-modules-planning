//! The lanes of a board as records of the form `rows@1.0.0` (type `Lane`): which lanes a swim-lane view has, also the empty ones.
//!
//! Pattern: Projection (a second source over the same data as the cards: the module lists the lanes of an axis in the values of the
//! standard, grouping and naming are the frontend's). The scope is `lanes:<axis>` or `lanes:<axis>:<board>`; its meaning belongs to the
//! module (`FormSource.page`), so `rows@1.0.0` stays as it is.
//!
//! The axes: `state` (the five words of RFC 8984 5.2.5 in the order of the RFC, then the vendor words the tasks in scope carry),
//! `assignee` (every person in charge of a task in scope, see `Task::assignees()`) and `lane` (the boards). On every axis the lanes
//! stored at the board (`Board::lanes`, axis to ids in the order shown) come first: that is how an empty lane exists.

use std::collections::BTreeSet;

use fs_tasks_core::{BoardEntry, Boards, Task};

use crate::errors::rpc_error;
use crate::facts::{STATES, assignees, state_word};
use crate::forms::Reading;
use crate::write::{UNKNOWN_SCOPE, board_named, problem};

/// The scope prefix that selects the lane list of `rows@1.0.0`.
pub(crate) const LANES: &str = "lanes:";

/// What the lanes of a list are grouped by.
#[derive(Clone, Copy)]
pub(crate) enum Axis {
    State,
    Assignee,
    Lane,
}

impl Axis {
    /// The name of the axis: in the scope, in `Board::lanes` and as the `state` of a lane record.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::State => "state",
            Self::Assignee => "assignee",
            Self::Lane => "lane",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        [Self::State, Self::Assignee, Self::Lane]
            .into_iter()
            .find(|a| a.name() == word)
    }
}

/// One lane: the value the cards are grouped by (also the record id), its name, and the axis it belongs to.
pub(crate) struct Lane {
    pub(crate) id: String,
    pub(crate) axis: Axis,
}

impl Lane {
    /// The record of the lane at place `at` by the field names of `catalog/forms.json`. The name is the value itself: a person is named by
    /// the reference (the name comes from the module of the people), a status by its word, a board by its name.
    pub(crate) fn row(&self, at: usize) -> Vec<(&'static str, String)> {
        vec![
            ("uid", self.id.clone()),
            ("label", self.id.clone()),
            ("order", at.to_string()),
            ("axis", self.axis.name().to_owned()),
        ]
    }
}

/// `<axis>` or `<axis>:<board>` (what follows `lanes:`); the board is a name or an id.
fn split(scope: &str) -> capnp::Result<(Axis, Option<&str>)> {
    let (word, board) = scope
        .split_once(':')
        .map_or((scope, None), |(a, b)| (a, Some(b)));
    let axis = Axis::parse(word).ok_or_else(|| {
        let why = format!("no lanes {word:?}: the axes are state, assignee and lane");
        problem(UNKNOWN_SCOPE, why, 404).into_error()
    })?;
    Ok((axis, board.filter(|b| !b.is_empty())))
}

impl Reading {
    /// The lanes of the scope `lanes:<axis>[:<board>]` (the part after `lanes:`), in the order shown, none twice.
    pub(crate) async fn lanes(&self, scope: &str) -> capnp::Result<Vec<Lane>> {
        let (axis, board) = split(scope)?;
        let boards = self.repo.boards().await.map_err(rpc_error)?;
        let chosen: Vec<&BoardEntry> = match board {
            None => boards.iter().collect(),
            Some(name) => vec![board_named(&boards, name).map_err(|p| p.into_error())?],
        };
        let stored = chosen
            .iter()
            .filter_map(|e| e.board.lanes.get(axis.name()))
            .flatten()
            .cloned();
        let found: Vec<String> = match axis {
            Axis::Lane => chosen.iter().map(|e| e.board.name.clone()).collect(),
            Axis::State | Axis::Assignee => {
                let tasks = self.tasks_of(board.unwrap_or("")).await?;
                if matches!(axis, Axis::State) {
                    STATES
                        .map(str::to_owned)
                        .into_iter()
                        .chain(vendor_states(tasks.iter().map(|t| &t.task)))
                        .collect()
                } else {
                    tasks.iter().flat_map(|t| assignees(&t.task)).collect()
                }
            }
        };
        let mut seen = BTreeSet::new();
        Ok(stored
            .chain(found)
            .filter(|id| seen.insert(id.clone()))
            .map(|id| Lane { id, axis })
            .collect())
    }
}

/// The vendor status words (`<domain>:<name>`) of the tasks, sorted: the five of the RFC are listed first by the caller.
fn vendor_states<'a>(tasks: impl Iterator<Item = &'a Task>) -> BTreeSet<String> {
    tasks
        .filter_map(state_word)
        .filter(|w| !STATES.contains(&w.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pattern: Object Mother (tasks with a status word, built in memory: the validator of the store takes no vendor word).
    fn with(progress: &str) -> Task {
        let mut task = Task::new("t");
        task.progress = Some(progress.into());
        task
    }

    #[test]
    fn vendor_words_are_listed_once_and_the_rest_is_left_out() {
        let tasks = [
            with("acme.example:review"),
            with("acme.example:review"),
            with("completed"),
            with("bogus"),
            with("acme.example:"),
            with("zeta.example:late"),
        ];
        let got: Vec<String> = vendor_states(tasks.iter()).into_iter().collect();
        assert_eq!(got, ["acme.example:review", "zeta.example:late"]);
    }
}
