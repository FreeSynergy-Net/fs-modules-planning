//! The placing of a card and the creation with a place, over a real store in a temporary folder: which cards the index pins (only the
//! shown cards before the insertion point of the target column), that the others keep following their `progress`, and that a
//! creation whose placing fails leaves no task behind.
//!
//! Pattern: table-free scenario tests over the core (`TaskRepo`), no socket.

use std::sync::Arc;

use fs_objects::{Store, TursoStore};
use fs_tasks_core::{Boards, Task, TaskItems, TaskRepo};

use crate::place::{create_placed, place};

struct Fixture {
    _dir: tempfile::TempDir,
    repo: TaskRepo,
    board: fs_objects::ObjectId,
    uids: Vec<String>,
}

/// A board `work` (standard columns todo/doing/done) and the tasks `titles`, all unplaced (they stand in `todo` by their progress).
async fn fixture(titles: &[&str]) -> Fixture {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    let store = TursoStore::open(dir.path().join("objects.db"))
        .await
        .unwrap();
    let store: Arc<dyn Store> = Arc::new(store);
    let repo = TaskRepo::new(store, "https://local.invalid/users/owner");
    let board = repo.board_create("work", None).await.unwrap().id;
    let mut uids = Vec::new();
    for t in titles {
        uids.push(repo.create(Task::new(*t)).await.unwrap().task.uid);
    }
    Fixture {
        _dir: dir,
        repo,
        board,
        uids,
    }
}

/// `column/uid-index` as the layout shows the cards, column by column.
async fn shown(f: &Fixture) -> Vec<String> {
    let layout = f.repo.board_layout(f.board).await.unwrap();
    layout
        .columns
        .iter()
        .flat_map(|c| {
            c.cards.iter().map(|u| {
                let at = f.uids.iter().position(|x| x == u).unwrap();
                format!("{}/{}", c.column.id, ["a", "b", "c", "d"][at])
            })
        })
        .collect()
}

/// Set the progress of a task like another program does (an edit of the task, not of the board).
async fn progress(f: &Fixture, at: usize, value: &str) {
    let entry = f.repo.get_by_uid(&f.uids[at]).await.unwrap().unwrap();
    let mut task = entry.task;
    task.progress = Some(value.into());
    f.repo.edit(entry.id, task).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn a_move_pins_only_the_cards_before_the_insertion_point_of_the_target_column() {
    let f = fixture(&["a", "b", "c", "d"]).await;
    // d to place 1 of `todo`: only `a` (before the point) is pinned with it; b and c are not touched
    place(&f.repo, f.board, &f.uids[3], "todo", Some(1))
        .await
        .unwrap();
    assert_eq!(shown(&f).await, ["todo/a", "todo/d", "todo/b", "todo/c"]);
    let order = f
        .repo
        .board_get(f.board)
        .await
        .unwrap()
        .unwrap()
        .board
        .order;
    assert_eq!(order["todo"], [f.uids[0].clone(), f.uids[3].clone()]);
    assert_eq!(order.len(), 1, "no other column is pinned: {order:?}");
    // a card that was never touched still follows its progress (b is done elsewhere, the card changes column on its own)
    progress(&f, 1, "completed").await;
    progress(&f, 2, "in-process").await;
    assert_eq!(shown(&f).await, ["todo/a", "todo/d", "doing/c", "done/b"]);
}

#[tokio::test(flavor = "current_thread")]
async fn a_move_to_another_column_leaves_the_untouched_columns_to_their_progress() {
    let f = fixture(&["a", "b", "c"]).await;
    progress(&f, 2, "in-process").await; // c in `doing`
    place(&f.repo, f.board, &f.uids[0], "doing", Some(0))
        .await
        .unwrap();
    assert_eq!(shown(&f).await, ["todo/b", "doing/a", "doing/c"]);
    // `b` was not placed by anyone: when it is completed it goes to `done`
    progress(&f, 1, "completed").await;
    assert_eq!(shown(&f).await, ["doing/a", "doing/c", "done/b"]);
}

#[tokio::test(flavor = "current_thread")]
async fn the_end_pins_the_whole_target_column_and_an_index_past_the_end_clamps() {
    let f = fixture(&["a", "b", "c"]).await;
    place(&f.repo, f.board, &f.uids[0], "todo", None)
        .await
        .unwrap();
    assert_eq!(shown(&f).await, ["todo/b", "todo/c", "todo/a"]);
    place(&f.repo, f.board, &f.uids[1], "todo", Some(99))
        .await
        .unwrap();
    assert_eq!(shown(&f).await, ["todo/c", "todo/a", "todo/b"]);
}

#[tokio::test(flavor = "current_thread")]
async fn a_failed_placing_of_a_new_task_takes_the_task_back() {
    let f = fixture(&["a"]).await;
    let before = f.repo.list().await.unwrap().len();
    // a column that is not there (removed in the meantime): the placing is refused after the task was made
    let e = create_placed(&f.repo, f.board, "new".into(), "nowhere", None)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("nowhere"), "{e}");
    assert_eq!(f.repo.list().await.unwrap().len(), before);
    // a board that vanished in the meantime
    f.repo.board_delete(f.board).await.unwrap();
    create_placed(&f.repo, f.board, "new".into(), "todo", None)
        .await
        .unwrap_err();
    assert_eq!(f.repo.list().await.unwrap().len(), before);
    // the retry that works makes it once
    let g = fixture(&["a"]).await;
    let uid = create_placed(&g.repo, g.board, "new".into(), "todo", Some(0))
        .await
        .unwrap();
    assert_eq!(g.repo.list().await.unwrap().len(), 2);
    assert_eq!(
        g.repo.board_layout(g.board).await.unwrap().columns[0].cards[0],
        uid
    );
}

#[tokio::test(flavor = "current_thread")]
async fn the_cards_already_placed_in_the_target_column_stay_placed_when_the_index_is_above_them() {
    let f = fixture(&["a", "b", "c", "d"]).await;
    // a to the end: todo is b, c, a (all three pinned in that order)
    place(&f.repo, f.board, &f.uids[0], "todo", None)
        .await
        .unwrap();
    // d to the top: the placed cards b, c, a keep their places behind it (not back to the order of their creation)
    place(&f.repo, f.board, &f.uids[3], "todo", Some(0))
        .await
        .unwrap();
    assert_eq!(shown(&f).await, ["todo/d", "todo/b", "todo/c", "todo/a"]);
}
