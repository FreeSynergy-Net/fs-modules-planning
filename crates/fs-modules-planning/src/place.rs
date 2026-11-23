//! Placing a card at an index, and creating a task together with its place.
//!
//! Pattern: Command over the core (one checked edit of the board). The core counts `index` only among the cards placed by hand in
//! `Board.order`; the cards that stand in a column by their `progress` come after them. So for the index a person sees, exactly the
//! shown cards BEFORE the insertion point of the TARGET column are pinned (written into `order`, where they stand now): nothing moves
//! on the screen, no other column is touched, and the cards behind the insertion point keep following their `progress`. That pinning is
//! unavoidable (a place among cards that have none needs those cards to have one); it is the whole price of the index.

use fs_objects::ObjectId;
use fs_tasks_core::model::{Board, Layout};
use fs_tasks_core::{Boards, Error, Task, TaskItems, TaskRepo};

/// Write into `board.order[column]` the cards of the target column that must stand before the insertion point: the placed ones (dead
/// uids go), then the unplaced shown ones up to `index` (`None` = the end = all). The moved card is not among them (the core inserts it).
pub(crate) fn pin_before(
    board: &mut Board,
    layout: &Layout,
    uid: &str,
    column: &str,
    index: Option<usize>,
) {
    let Some(target) = layout.columns.iter().find(|c| c.column.id == column) else {
        return; // no such column: the core refuses the move
    };
    let shown: Vec<&String> = target.cards.iter().filter(|u| *u != uid).collect();
    let placed = board.order.get(column);
    let is_placed = |u: &&String| placed.is_some_and(|p| p.contains(*u));
    let stay = shown.iter().filter(|u| is_placed(u)).count();
    let upto = index.map_or(shown.len(), |i| i.min(shown.len())).max(stay);
    let pinned: Vec<String> = shown.iter().take(upto).map(|u| (*u).clone()).collect();
    if pinned.is_empty() {
        board.order.remove(column);
    } else {
        board.order.insert(column.to_owned(), pinned);
    }
}

/// Put the card `uid` into `column` at `index` COUNTED AMONG THE CARDS AS THE LAYOUT SHOWS THEM (see the head of this file). A board
/// changed by someone else in between is a conflict (checked edit), never a lost write.
pub(crate) async fn place(
    repo: &TaskRepo,
    id: ObjectId,
    uid: &str,
    column: &str,
    index: Option<usize>,
) -> fs_tasks_core::Result<()> {
    let entry = repo.board_get(id).await?.ok_or(Error::NotFound(id))?;
    let layout = repo.board_layout(id).await?;
    let mut board = entry.board.clone();
    pin_before(&mut board, &layout, uid, column, index);
    board
        .move_card(uid, column, index)
        .map_err(Error::Rejected)?;
    repo.board_edit(id, entry.revision(), board).await?;
    Ok(())
}

/// A new task with `title`, placed. If the placing fails (conflict, a column or board that vanished), the task just made is deleted
/// again and the placing error is returned: a failed command leaves nothing behind, so a retry cannot make it twice.
pub(crate) async fn create_placed(
    repo: &TaskRepo,
    id: ObjectId,
    title: String,
    column: &str,
    index: Option<usize>,
) -> fs_tasks_core::Result<String> {
    let made = repo.create(Task::new(title)).await?;
    let (uid, task_id) = (made.task.uid.clone(), made.id);
    match place(repo, id, &uid, column, index).await {
        Ok(()) => Ok(uid),
        Err(placing) => match repo.delete(task_id).await {
            Ok(()) => Err(placing),
            Err(undo) => Err(Error::Rejected(format!(
                "task {uid:?} was made but could not be placed ({placing}) nor taken back ({undo})"
            ))),
        },
    }
}
