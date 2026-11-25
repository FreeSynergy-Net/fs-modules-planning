//! Which changes belong to the scope of a board (`watch`, `changes`).
//!
//! Pattern: Specification (a change is in scope when it is about the board, about a card the board shows now, or about a card that
//! belonged to it BEFORE the change: a deleted card is no longer in the layout, so its before-image decides).

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, VecDeque};

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

/// The text ids a board is known by in a scope: its id and its name (a caller names a scope by either).
fn names_of(board: &fs_tasks_core::BoardEntry) -> [String; 2] {
    [board.id.to_string(), board.board.name.clone()]
}

/// Every scope (board id and board name) the head activity `a` is a change in, decided now by [`Scope::holds`].
pub(crate) async fn scopes_of(repo: &TaskRepo, a: &Activity) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for board in repo.boards().await.unwrap_or_default() {
        if Scope(board.id).holds(repo, a).await {
            out.extend(names_of(&board));
        }
    }
    out
}

/// The most changes whose scopes are remembered.
const MEMO_MAX: usize = 4096;

/// Pattern: Memoization (closest to a Cache; deviation: it is filled by the feed and by `changes`, never by a test) - which scopes
/// a change is in, by its activity id. `Follow` is a SYNCHRONOUS test of a `ChangeRef`, while the scope of a change needs the store
/// (the layout, the before-image); so the async side decides once when it hands the change out, and `Follow` only looks it up.
/// A change that is not remembered is not in any scope (fail closed).
#[derive(Default)]
pub(crate) struct ScopeMemo {
    inner: RefCell<Inner>,
}

#[derive(Default)]
struct Inner {
    order: VecDeque<String>,
    scopes: HashMap<String, BTreeSet<String>>,
}

impl ScopeMemo {
    /// Remember the scopes of the change `activity` (the oldest is forgotten past [`MEMO_MAX`]).
    pub(crate) fn put(&self, activity: &str, scopes: BTreeSet<String>) {
        let mut inner = self.inner.borrow_mut();
        if inner.scopes.insert(activity.to_owned(), scopes).is_none() {
            inner.order.push_back(activity.to_owned());
        }
        while inner.order.len() > MEMO_MAX {
            if let Some(old) = inner.order.pop_front() {
                inner.scopes.remove(&old);
            }
        }
    }

    /// Whether the change `activity` is remembered as being in `scope`.
    pub(crate) fn holds(&self, activity: &str, scope: &str) -> bool {
        self.inner
            .borrow()
            .scopes
            .get(activity)
            .is_some_and(|s| s.contains(scope))
    }
}
