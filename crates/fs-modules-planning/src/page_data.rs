//! What one page of a form is made of, before it is written to any wire: the rows, the total and what may be done.
//!
//! Pattern: Adapter (the board layout and the tasks of the core as rows by the field names of `catalog/forms.json`); the ONE
//! reading `PlanningCore::page` uses (the wire of `FormSource` is `Guarded`).

use fs_interfaces::Capabilities;
use fs_tasks_core::TaskItems as _;

use crate::forms::{Reading, UNKNOWN_FORM, limit_of, serves, serves_rows, unknown_form_why};
use crate::list::Row;
use crate::write::{RECORD_COMMANDS, SET_COMMANDS, may, problem};

/// One page: the rows from `skip`, the total, the end of the window and what the provider supports (for a record, and for the set).
pub(crate) struct PageData {
    pub(crate) total: usize,
    pub(crate) rows: Vec<Row>,
    pub(crate) end: usize,
    pub(crate) on_record: Vec<&'static str>,
    pub(crate) on_set: Vec<&'static str>,
}

impl Reading {
    /// The page of `form` in `scope` from `skip`, at most `limit` records (0 = the most). A form this module does not draw is
    /// `unknown-form` 404.
    pub(crate) async fn page_data(
        &self,
        form: &str,
        scope: &str,
        skip: usize,
        limit: u32,
    ) -> capnp::Result<PageData> {
        let rows_form = serves_rows(form);
        if !rows_form && !serves(form) {
            return Err(problem(UNKNOWN_FORM, unknown_form_why(), 404).into_error());
        }
        let limit = limit_of(limit);
        let (total, rows) = if rows_form {
            self.list_page(scope, skip, limit).await?
        } else {
            let cards = self.cards(scope).await?;
            let mut rows = Vec::new();
            for card in cards.iter().skip(skip).take(limit) {
                rows.extend(self.row(card).await?);
            }
            (cards.len(), rows)
        };
        let end = (skip + limit).min(total);
        // what the provider supports, for a record and for the set (a new card); the same table `invoke` is held to
        let caps: Capabilities = self.repo.capabilities();
        // the lanes are not cards: no command is offered at a lane record or at the list of lanes
        let (on_record, on_set) = if rows_form && scope.starts_with(crate::lanes::LANES) {
            (Vec::new(), Vec::new())
        } else {
            (may(&caps, &RECORD_COMMANDS), may(&caps, &SET_COMMANDS))
        };
        Ok(PageData {
            total,
            rows,
            end,
            on_record,
            on_set,
        })
    }
}
