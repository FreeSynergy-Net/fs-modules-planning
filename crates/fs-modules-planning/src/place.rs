//! Placing a card between neighbours, and creating a task together with its place.
//!
//! Pattern: Command over the core (one checked edit of the board). The place is never a number: the caller names the card it comes
//! right after (`after`) and the card it comes right before (`before`), as the column shows them; the core (`Board::place_card`) makes
//! the rank key between theirs and writes only the place of the moved card (and, where a card before it never had a place, that
//! card's place, where it stands now, so nothing moves on the screen). The cards behind it keep following their `progress`.

use fs_objects::ObjectId;
use fs_tasks_core::{Boards, Error, Task, TaskItems, TaskRepo};

/// Put the card `uid` into `column` between `after` and `before` (record ids of the neighbour cards; `None` = no card on that side; if
/// they are no longer neighbours `after` is the anchor, see `Board::place_card`). A board changed by someone else in between is a
/// conflict (checked edit), never a lost write.
pub(crate) async fn place(
    repo: &TaskRepo,
    id: ObjectId,
    uid: &str,
    column: &str,
    after: Option<&str>,
    before: Option<&str>,
) -> fs_tasks_core::Result<()> {
    let entry = repo.board_get(id).await?.ok_or(Error::NotFound(id))?;
    let layout = repo.board_layout(id).await?;
    let mut board = entry.board.clone();
    board
        .place_card(&layout, uid, column, after, before)
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
    after: Option<&str>,
    before: Option<&str>,
) -> fs_tasks_core::Result<String> {
    let made = repo.create(Task::new(title)).await?;
    let (uid, task_id) = (made.task.uid.clone(), made.id);
    match place(repo, id, &uid, column, after, before).await {
        Ok(()) => Ok(uid),
        Err(placing) => match repo.delete(task_id).await {
            Ok(()) => Err(placing),
            Err(undo) => Err(Error::Rejected(format!(
                "task {uid:?} was made but could not be placed ({placing}) nor taken back ({undo})"
            ))),
        },
    }
}
