//! The tasks and the lanes as records of the form `rows@1.0.0`: one row per task with typed columns (type `Task`, the list of the
//! classic frontend), or one row per lane (type `Lane`, scope `lanes:<axis>[:<board>]`, see `lanes.rs`).
//!
//! Pattern: Projection (the same tasks as the cards, as rows: the module sends the values of the standard, the frontend chooses the
//! columns); the scope decides the type, as the form `rows@1.0.0` leaves its meaning to the module.

use fs_tasks_core::TaskItems;

use crate::cards::Card;
use crate::errors::rpc_error;
use crate::formsource::FormSourceImpl;
use crate::lanes::LANES;

/// One record: its id (`uid`) first, then the values by the field names of the catalog.
pub(crate) type Row = Vec<(&'static str, String)>;

/// The words of `taskStatus` (RFC 10051 section 11.2, STATUS of a task).
const TASK_STATUS: [&str; 6] = [
    "NEEDS-ACTION",
    "IN-PROCESS",
    "COMPLETED",
    "CANCELLED",
    "PENDING",
    "FAILED",
];

/// The fields of the type `Task` of `rows@1.0.0` that are sent (`catalog/forms.json`: `label`, `order`, the columns): a value of the
/// shared set (`FormSourceImpl::shared`) that is not one of them stays out.
const TASK_FIELDS: [&str; 14] = [
    "title",
    "index",
    "due",
    "timeZone",
    "priority",
    "progress",
    "taskStatus",
    "percentComplete",
    "estimatedDuration",
    "keywords",
    "participants",
    "recurrenceRule",
    "relatedTo",
    "privacy",
];

impl FormSourceImpl {
    /// One page of `rows@1.0.0` in `scope`: the lanes for `lanes:...`, else the tasks of the board (empty = every board). Returns the
    /// total and the rows from `skip`, at most `limit`.
    pub(crate) async fn list_page(
        &self,
        scope: &str,
        skip: usize,
        limit: usize,
    ) -> capnp::Result<(usize, Vec<Row>)> {
        if let Some(lanes) = scope.strip_prefix(LANES) {
            let lanes = self.lanes(lanes).await?;
            let rows = lanes.iter().enumerate().skip(skip).take(limit);
            return Ok((lanes.len(), rows.map(|(at, lane)| lane.row(at)).collect()));
        }
        let cards = self.unique_cards(scope).await?;
        let mut rows = Vec::new();
        for card in cards.iter().skip(skip).take(limit) {
            rows.extend(self.task_row(card).await?);
        }
        Ok((cards.len(), rows))
    }

    /// The record of one task by the field names of the type `Task`; `None` when the task is gone in the meantime.
    async fn task_row(&self, card: &Card) -> capnp::Result<Option<Row>> {
        let Some(entry) = self.repo.get_by_uid(&card.uid).await.map_err(rpc_error)? else {
            return Ok(None);
        };
        let task = &entry.task;
        let mut row = vec![
            ("uid", card.uid.clone()),
            ("title", task.title.clone().unwrap_or_default()),
            ("index", card.at.to_string()),
        ];
        row.extend(self.shared(task)?);
        // only the six words of RFC 10051 section 11.2 (STATUS of a task); another word is NOT sent
        row.extend(
            task.task_status()
                .filter(|s| TASK_STATUS.contains(s))
                .map(|s| ("taskStatus", s.to_owned())),
        );
        // `recurrenceRule` is not sent yet: it comes as soon as fs-jscalendar has the conversion wire form -> typed rule (then
        // `to_rrule_in(zone)`, UNTIL in UTC); no text is written or guessed here (fail-closed).
        // `relatedTo`: the contract (`rows@1.0.0`) has no `repeats: relatedTo` yet, so at most ONE value (the first) is sent until
        // the addition of the contract (fs-interfaces) lands.
        row.extend(
            task.related_to
                .keys()
                .next()
                .map(|uid| ("relatedTo", uid.clone())),
        );
        row.extend(
            task.privacy
                .as_deref()
                .filter(|p| ["public", "private", "secret"].contains(p))
                .map(|p| ("privacy", p.to_owned())),
        );
        row.retain(|(path, _)| *path == "uid" || TASK_FIELDS.contains(path));
        Ok(Some(row))
    }
}
