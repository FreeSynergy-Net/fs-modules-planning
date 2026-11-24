//! Reading the cards of the boards as records of the form `cards-in-columns`.
//!
//! Pattern: Adapter (the board layout and the tasks of the core, as records by the field names of `catalog/forms.json`).

use fs_tasks_core::{BoardEntry, Boards, TaskItems};

use crate::errors::rpc_error;
use crate::formsource::FormSourceImpl;
use crate::write::board_named;

/// One card of the layout: its board, column, uid and its position in the column (the read-only `order`).
pub(crate) struct Card {
    pub(crate) board: String,
    pub(crate) column: String,
    pub(crate) label: String,
    /// The WIP limit of the column (`column-limit`); the component counts, the module only reports it.
    pub(crate) limit: Option<u32>,
    pub(crate) uid: String,
    pub(crate) at: usize,
}

impl FormSourceImpl {
    /// The cards of the boards in `scope` (empty = every board), in the order of the layout: column by column, as `(board name,
    /// column id, uid)`. Only the layout is read here; the tasks of one page are read by [`Self::row`].
    pub(crate) async fn cards(&self, scope: &str) -> capnp::Result<Vec<Card>> {
        let boards = self.repo.boards().await.map_err(rpc_error)?;
        let chosen: Vec<&BoardEntry> = if scope.is_empty() {
            boards.iter().collect()
        } else {
            vec![board_named(&boards, scope).map_err(|p| p.into_error())?]
        };
        let mut out = Vec::new();
        for entry in chosen {
            let layout = self.repo.board_layout(entry.id).await.map_err(rpc_error)?;
            for column in layout.columns {
                let (board, id) = (&entry.board.name, &column.column.id);
                let label = &column.column.name;
                let limit = column.column.wip_limit;
                out.extend(column.cards.into_iter().enumerate().map(|(at, uid)| Card {
                    board: board.clone(),
                    column: id.clone(),
                    label: label.clone(),
                    limit,
                    uid,
                    at,
                }));
            }
        }
        Ok(out)
    }

    /// The record of one card by the field names of `catalog/forms.json`; `None` when the task is gone in the meantime.
    pub(crate) async fn row(
        &self,
        Card {
            board,
            column,
            label,
            limit,
            uid,
            at,
        }: &Card,
    ) -> capnp::Result<Option<Vec<(&'static str, String)>>> {
        let Some(task) = self.repo.get_by_uid(uid).await.map_err(rpc_error)? else {
            return Ok(None);
        };
        let task = &task.task;
        let mut row = vec![
            ("uid", uid.clone()),
            ("board", board.clone()),
            ("column", column.clone()),
            ("title", task.title.clone().unwrap_or_default()),
            ("index", at.to_string()),
        ];
        // RFC 3339 date-time with offset (form art `time`), not the naive JSCalendar LocalDateTime
        if let Some(due) = self.repo.due_instant(task).map_err(rpc_error)? {
            row.push(("due", due));
        }
        row.push(("column-label", label.clone()));
        if let Some(limit) = limit {
            row.push(("column-limit", limit.to_string()));
        }
        if let Some(percent) = task.percent_complete {
            row.push(("percent", percent.to_string()));
        }
        row.extend(crate::facts::channels(task));
        row.extend(crate::facts::of(task));
        Ok(Some(row))
    }
}
