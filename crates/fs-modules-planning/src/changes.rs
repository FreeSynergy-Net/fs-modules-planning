//! The history of the planning module: what a change is about (`Seen`), the changes of a scope (`changes_page`), the scope a name
//! stands for (`board_of`) and taking a change back (`revert_of`), over the units of work of the store (`changes_of`, `revert_change`).
//!
//! Pattern: Compensating Transaction (`revert_of`), Repository (`board_of`: lookup by key), Observer queries (`changes_page`). The
//! bodies of `PlanningCore`; what `Guarded` does around them (checks, record, report) is not here.

use fs_objects::{Activity, ActivityId, ChangeFilter, Outcome};
use fs_tasks_core::TaskRepo;

use crate::errors::rpc_error;
use crate::forms::Reading;
use crate::scope::Scope;
use crate::write::{Done, problem, to_problem};

pub(crate) const UNKNOWN_CHANGE: &str = "form-source-unknown-change";
pub(crate) const NOT_REVERTIBLE: &str = "form-source-not-revertible";
pub(crate) const GONE: &str = "form-source-gone";

impl Reading {
    /// The scope a name stands for (empty = the whole module); a name that is no board is `form-source-unknown-scope`.
    pub(crate) async fn board_of(&self, scope: &str) -> capnp::Result<Option<Scope>> {
        if scope.is_empty() {
            return Ok(None);
        }
        let boards = fs_tasks_core::Boards::boards(&*self.repo)
            .await
            .map_err(rpc_error)?;
        let board = crate::write::board_named(&boards, scope).map_err(|p| p.into_error())?;
        Ok(Some(Scope(board.id)))
    }

    /// The heads of the changes that match: of the own program (`run` empty) or of one run, newest first, with the cursor of this service.
    pub(crate) async fn changes_page(
        &self,
        scope: &str,
        run: &str,
        skip: usize,
        limit: usize,
    ) -> capnp::Result<(Vec<Activity>, bool)> {
        let mut filter = ChangeFilter::default();
        if run.is_empty() {
            filter.program = Some(self.origin.program.clone());
        } else {
            filter.run = Some(run.to_owned());
        }
        let board = self.board_of(scope).await?;
        let mut all = self.repo.changes_of(&filter).await.map_err(rpc_error)?;
        if let Some(board) = board {
            let mut kept = Vec::with_capacity(all.len());
            for a in all {
                if board.holds(&self.repo, &a).await {
                    kept.push(a);
                }
            }
            all = kept;
        }
        all.reverse();
        let more = all.len() > skip + limit;
        Ok((all.into_iter().skip(skip).take(limit).collect(), more))
    }
}

/// What a `ChangeRef` says about the record: its id as the form shows it, and for an undo the head of the unit it takes back.
pub(crate) struct Seen {
    pub(crate) object: String,
    pub(crate) undoes: Option<u128>,
}

/// A change as the caller sees it: an undo is about the record of the unit it took back (its own head is the inverse of the last member).
pub(crate) async fn seen(repo: &TaskRepo, a: &Activity) -> Seen {
    if let Some(target) = a.target {
        let member = repo.activity_of(target).await.ok().flatten();
        let head = match member {
            Some(m) => match m.unit {
                Some(u) => repo.activity_of(u).await.ok().flatten(),
                None => Some(m),
            },
            None => None,
        };
        if let Some(h) = head {
            return Seen {
                object: record_of(repo, &h).await,
                undoes: Some(h.id.as_uuid().as_u128()),
            };
        }
    }
    Seen {
        object: record_of(repo, a).await,
        undoes: a.target.map(|t| t.as_uuid().as_u128()),
    }
}

/// What the head of a unit that was written is about, for the `ChangeRef` of the answer; a unit without a head is about no record.
pub(crate) async fn seen_of_head(repo: &TaskRepo, head: Option<&Activity>) -> Seen {
    match head {
        Some(head) => seen(repo, head).await,
        None => Seen {
            object: String::new(),
            undoes: None,
        },
    }
}

/// The id of the record a change is about, as the form shows it: the `uid` of the task (`urn:uuid:` and the id for a task made here), else the id of the object.
pub(crate) async fn record_of(repo: &TaskRepo, a: &Activity) -> String {
    match fs_tasks_core::TaskItems::get(repo, a.object).await {
        Ok(Some(entry)) => entry.task.uid,
        Ok(None) if a.kind.as_str() == "Delete" => format!("urn:uuid:{}", a.object),
        _ => a.object.to_string(),
    }
}

/// What taking a change back came to.
pub(crate) enum Reverted {
    /// Taken back: the unit written.
    Done(Done),
    /// The record changed since: the last change and the paths.
    Clash(Box<Activity>, Vec<String>),
    /// Refused (not revertible, blacked out, unknown) or broken.
    Failed(fs_rpc::Problem),
}

/// Take the unit `id` back on `repo` (origin and step already set): the `revert_by` of the store, its outcome as `Reverted`.
pub(crate) async fn revert_of(repo: &TaskRepo, id: ActivityId) -> Reverted {
    match repo.revert_change(id).await {
        Ok(Outcome::Done(done)) => Reverted::Done(Done {
            record: done.head.object.to_string(),
            text: "change-reverted",
            head: Some(Box::new(done.head)),
        }),
        Ok(Outcome::Conflict { by, fields }) => {
            Reverted::Clash(by, fields.into_iter().map(|f| f.path).collect())
        }
        Ok(Outcome::NotRevertible(why)) => Reverted::Failed(problem(NOT_REVERTIBLE, why, 422)),
        Ok(_) => {
            let known = repo.activity_of(id).await.ok().flatten().is_some();
            Reverted::Failed(if known {
                problem(
                    GONE,
                    "the change was blacked out or its record is gone",
                    410,
                )
            } else {
                problem(UNKNOWN_CHANGE, "no such change", 404)
            })
        }
        Err(e) => Reverted::Failed(*to_problem(e)),
    }
}
