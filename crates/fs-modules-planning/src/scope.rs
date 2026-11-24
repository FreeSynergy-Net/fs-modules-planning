//! Which changes belong to the scope of a board (`watch`, `changes`).
//!
//! Pattern: Specification (a change is in scope when it is about the board, about a card the board shows now, or about a card that
//! belonged to it BEFORE the change: a deleted card is no longer in the layout, so its before-image decides).

use fs_objects::{Activity, ObjectId};
use fs_tasks_core::{Boards, TaskItems, TaskRepo};

/// The scope of one board.
#[derive(Clone, Copy)]
pub(crate) struct Scope(pub(crate) ObjectId);

impl Scope {
    /// Whether the head activity `a` is a change in this scope.
    pub(crate) async fn holds(self, repo: &TaskRepo, a: &Activity) -> bool {
        if a.object == self.0 {
            return true;
        }
        let Ok(Some(board)) = repo.board_get(self.0).await else {
            return false;
        };
        // a card that exists: by the layout (its uid, whatever it looks like)
        if let Ok(Some(entry)) = repo.get(a.object).await {
            return repo.board_layout(self.0).await.is_ok_and(|l| {
                l.columns
                    .iter()
                    .flat_map(|c| c.cards.iter())
                    .any(|uid| *uid == entry.task.uid)
            });
        }
        // a card that is gone (deleted, or made not to be by an undo): the image it had before decides
        let Some(before) = a.change_set().ok().and_then(|c| c.before) else {
            return false;
        };
        let Ok(task) = fs_tasks_core::Task::from_value(before.content) else {
            return false;
        };
        let b = &board.board;
        b.place.contains_key(&task.uid)
            || b.label
                .as_ref()
                .is_none_or(|l| task.keywords.get(l).copied().unwrap_or(false))
    }
}
