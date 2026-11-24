//! The precision of the scope of a board for a DELETED card: the before-image decides, so a card of another label is no false positive.
//!
//! Pattern: Scenario tests (given a store, when a card changes, then the scope holds) over the core (`TaskRepo`), no socket.

use std::sync::Arc;

use fs_objects::{ChangeFilter, Store, TursoStore};
use fs_tasks_core::{Boards, Task, TaskItems, TaskRepo};

use crate::scope::Scope;

async fn repo(dir: &std::path::Path) -> TaskRepo {
    let store = TursoStore::open_with(dir.join("objects.db"), fs_objects::testing::options())
        .await
        .unwrap();
    let store: Arc<dyn Store> = Arc::new(store);
    TaskRepo::new(
        store,
        "https://local.invalid/users/owner",
        fs_objects::testing::owner(),
    )
}

fn labelled(title: &str, label: &str) -> Task {
    let mut task = Task::new(title);
    task.keywords.insert(label.to_owned(), true);
    task
}

#[tokio::test]
async fn a_deleted_card_is_in_the_scope_of_its_label_board_only() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    let repo = repo(dir.path()).await;
    let lab = repo.board_create("lab", Some("x")).await.unwrap().id;
    let other = repo.board_create("other", Some("y")).await.unwrap().id;
    let t1 = repo.create(labelled("one", "x")).await.unwrap().id;
    let t2 = repo.create(labelled("two", "y")).await.unwrap().id;
    repo.delete_checked(t1, None).await.unwrap();
    repo.delete_checked(t2, None).await.unwrap();
    let all = repo.changes_of(&ChangeFilter::default()).await.unwrap();
    let deleted = |id| {
        all.iter()
            .find(|a| a.object == id && a.change_set().is_ok_and(|c| c.after.is_none()))
            .expect("the delete is in the log")
    };
    assert!(Scope(lab).holds(&repo, deleted(t1)).await);
    assert!(!Scope(other).holds(&repo, deleted(t1)).await);
    assert!(Scope(other).holds(&repo, deleted(t2)).await);
    assert!(!Scope(lab).holds(&repo, deleted(t2)).await);
}
