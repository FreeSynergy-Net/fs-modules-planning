//! The tasks and the lanes as records of the form `rows@1.0.0`: one row per task with typed columns (type `Task`, the list of the
//! classic frontend), or one row per lane (type `Lane`, scope `lanes:<axis>[:<board>]`, see `lanes.rs`).
//!
//! Pattern: Projection (the same tasks as the cards, as rows: the module sends the values of the standard, the frontend chooses the
//! columns); the scope decides the type, as the form `rows@1.0.0` leaves its meaning to the module.

use fs_tasks_core::{RecurrenceRule, TaskItems};

use crate::cards::Card;
use crate::errors::rpc_error;
use crate::formsource::FormSourceImpl;
use crate::lanes::LANES;

/// One record: its id (`uid`) first, then the values by the field names of the catalog.
pub(crate) type Row = Vec<(&'static str, String)>;

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
        let mut seen = std::collections::BTreeSet::new();
        let cards: Vec<Card> = self
            .cards(scope)
            .await?
            .into_iter()
            .filter(|c| seen.insert(c.uid.clone()))
            .collect();
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
        row.extend(task.task_status().map(|s| ("taskStatus", s.to_owned())));
        row.extend(
            task.recurrence_rule
                .as_ref()
                .map(|r| ("recurrenceRule", rrule(r))),
        );
        row.extend(task.related_to.keys().map(|uid| ("relatedTo", uid.clone())));
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

/// The RRULE text (RFC 5545 3.3.10, with `RSCALE` and `SKIP` of RFC 7529) of the rule of the task. The task keeps the rule as the
/// JSCalendar object of RFC 8984 (the wire form of `fs-jscalendar`), and no original text beside it, so the text is written from it; nothing
/// is read back or changed.
fn rrule(r: &RecurrenceRule) -> String {
    fn list<T: ToString>(out: &mut Vec<String>, name: &str, items: &[T]) {
        if !items.is_empty() {
            let joined: Vec<String> = items.iter().map(ToString::to_string).collect();
            out.push(format!("{name}={}", joined.join(",")));
        }
    }
    let mut parts = Vec::new();
    parts.extend(
        r.rscale
            .as_ref()
            .map(|v| format!("RSCALE={}", v.to_uppercase())),
    );
    parts.push(format!("FREQ={}", r.frequency.to_uppercase()));
    parts.extend(
        r.skip
            .as_ref()
            .map(|v| format!("SKIP={}", v.to_uppercase())),
    );
    parts.extend(r.interval.map(|v| format!("INTERVAL={v}")));
    parts.extend(r.count.map(|v| format!("COUNT={v}")));
    parts.extend(
        r.until
            .as_ref()
            .map(|v| format!("UNTIL={}", v.replace(['-', ':'], ""))),
    );
    let days: Vec<String> = r
        .by_day
        .iter()
        .map(|d| {
            format!(
                "{}{}",
                d.nth_of_period.map_or_else(String::new, |n| n.to_string()),
                d.day.to_uppercase()
            )
        })
        .collect();
    list(&mut parts, "BYDAY", &days);
    list(&mut parts, "BYMONTHDAY", &r.by_month_day);
    list(&mut parts, "BYMONTH", &r.by_month);
    list(&mut parts, "BYYEARDAY", &r.by_year_day);
    list(&mut parts, "BYWEEKNO", &r.by_week_no);
    list(&mut parts, "BYHOUR", &r.by_hour);
    list(&mut parts, "BYMINUTE", &r.by_minute);
    list(&mut parts, "BYSECOND", &r.by_second);
    list(&mut parts, "BYSETPOS", &r.by_set_position);
    parts.extend(
        r.first_day_of_week
            .as_ref()
            .map(|v| format!("WKST={}", v.to_uppercase())),
    );
    parts.join(";")
}
