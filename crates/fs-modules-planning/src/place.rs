//! Placing a card between neighbours, and creating a task together with its place.
//!
//! Pattern: Command over the core (one checked edit of the board as ONE unit of work). The place is never a number: the caller names the card it
//! comes right after (`after`) and the card it comes right before (`before`), as the column shows them; the core (`Board::place_card`) makes
//! the rank key between theirs and writes only the place of the moved card (and, where a card before it never had a place, that
//! card's place, where it stands now, so nothing moves on the screen). The cards behind it keep following their `progress`.
//! The task and the board are written by ONE `write_by` (`TaskRepo::write_card`): a failed command leaves nothing behind, and one
//! undo takes both back.

use fs_objects::{ObjectId, Outcome};
use fs_tasks_core::{Boards, CardOp, Error, Task, TaskRepo};

/// Put the card `uid` into `column` between `after` and `before` (record ids of the neighbour cards; `None` = no card on that side; if
/// they are no longer neighbours `after` is the anchor, see `Board::place_card`). A board changed by someone else in between is a
/// conflict (checked edit), never a lost write. `expect` = the `rev` of the task the caller saw (a clash is the answer).
pub(crate) async fn place(
    repo: &TaskRepo,
    id: ObjectId,
    uid: &str,
    column: &str,
    (after, before): (Option<&str>, Option<&str>),
    expect: Option<u64>,
) -> fs_tasks_core::Result<Outcome> {
    let (board, revision) = placed(repo, id, uid, column, after, before).await?;
    let op = CardOp::Move {
        uid: uid.to_owned(),
        expect,
    };
    repo.write_card(op, id, revision, board).await
}

/// A new task with `title`, placed: the task and the place are ONE unit, so a placing that fails (conflict, a column or board that
/// vanished) leaves no task behind and a retry cannot make it twice. Returns the `uid` and what the store made of the unit.
pub(crate) async fn create_placed(
    repo: &TaskRepo,
    id: ObjectId,
    title: String,
    column: &str,
    (after, before): (Option<&str>, Option<&str>),
) -> fs_tasks_core::Result<(String, Outcome)> {
    let task = Task::new(title);
    let uid = task.uid.clone();
    let (board, revision) = placed(repo, id, &uid, column, after, before).await?;
    let outcome = repo
        .write_card(CardOp::Create(Box::new(task)), id, revision, board)
        .await?;
    Ok((uid, outcome))
}

/// The board with the card `uid` placed, and the revision the board had when it was read.
async fn placed(
    repo: &TaskRepo,
    id: ObjectId,
    uid: &str,
    column: &str,
    after: Option<&str>,
    before: Option<&str>,
) -> fs_tasks_core::Result<(fs_tasks_core::model::Board, fs_tasks_core::Revision)> {
    let entry = repo.board_get(id).await?.ok_or(Error::NotFound(id))?;
    let layout = repo.board_layout(id).await?;
    let mut board = entry.board.clone();
    board
        .place_card(&layout, uid, column, after, before)
        .map_err(Error::Rejected)?;
    Ok((board, entry.revision()))
}
