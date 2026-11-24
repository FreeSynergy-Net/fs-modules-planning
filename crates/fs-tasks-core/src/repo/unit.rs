//! One command, one unit: the writes of a command are ONE `write_by` (one transaction, one undo unit), recorded with the
//! person, the origin and the step of this handle.
//!
//! Pattern: Unit of Work (the writes of one command go into the store together or not at all) over the Parameter Object `By`
//! (who, for whom, from where, which step). A handle for one call is made with [`TaskRepo::with_origin`] and
//! [`TaskRepo::with_step`]; the store is shared.

use fs_objects::{ActivityFilter, ActivityId, Origin, Outcome, Recorded, StoredObject, Write};
use tokio::sync::broadcast;

use super::handle::TaskRepo;
use crate::error::{Error, Result};
use crate::interface::{TaskChange, TaskEntry};
use crate::model::{Board, Task};
use crate::revision::Revision;

/// What one card command writes besides the board.
#[derive(Clone, Debug)]
pub enum CardOp {
    /// A new task, written together with the board.
    Create(Box<Task>),
    /// The task `uid` stands for the move (it is written together with the board, so the change is about the record);
    /// `expect` = the `rev` of the task the caller saw (compare and set), none = no check.
    Move {
        /// The `uid` of the card.
        uid: String,
        /// The revision (`objects.rev`) of the task the caller saw.
        expect: Option<u64>,
    },
}

impl TaskRepo {
    /// The person, owner, origin and step this handle writes with.
    #[must_use]
    pub fn by(&self) -> &fs_objects::By {
        &self.by
    }

    /// The same tasks, written from `origin` (program, instance, device); the person stays.
    #[must_use]
    pub fn with_origin(&self, origin: Origin) -> Self {
        let mut by = self.by.clone();
        by.origin = origin;
        Self { by, ..self.clone() }
    }

    /// The same tasks, written as the step `step` of the run `run`: the same pair acts once.
    #[must_use]
    pub fn with_step(&self, run: &str, step: &str) -> Self {
        Self {
            by: self.by.clone().step(run, step),
            ..self.clone()
        }
    }

    /// The writes as ONE unit; a refusal of the store that is not a clash is an error.
    pub(super) async fn write_unit(&self, writes: Vec<Write>) -> Result<Outcome> {
        Ok(self.objects.write_by(&self.by, writes).await?)
    }

    /// One object as its own unit; anything but `Done` is an error (no `expect` is given here, so a clash cannot be).
    pub(super) async fn put_one(&self, object: StoredObject) -> Result<Recorded> {
        match self
            .write_unit(vec![Write::Put {
                object,
                expect: None,
            }])
            .await?
        {
            Outcome::Done(done) => Ok(done),
            other => Err(refused(&other)),
        }
    }

    /// Delete the task `id` as one unit; `expect` = the `rev` the caller saw. A clash is the answer, not an error.
    ///
    /// # Errors
    /// No such task, or the store failed.
    pub async fn delete_checked(
        &self,
        id: fs_objects::ObjectId,
        expect: Option<u64>,
    ) -> Result<Outcome> {
        let stored = self.stored(id).await?.ok_or(Error::NotFound(id))?;
        let gone = super::crud::entry_of(stored).ok();
        let outcome = self.write_unit(vec![Write::Delete { id, expect }]).await?;
        if matches!(outcome, Outcome::Done(_)) {
            self.feed.notify(&TaskChange::Deleted(id));
            if let Some(gone) = gone {
                self.queue_unblocked(&gone.task, None);
            }
        }
        Ok(outcome)
    }

    /// A card command as ONE unit: the task (created, or the one that moves) and the board `board` (its new content, checked
    /// against `revision` as `board_edit`). The task comes first, so the head of the unit is about the record. A clash of
    /// `expect` is the answer (`Outcome::Conflict`), a board changed in between is [`Error::Changed`].
    ///
    /// # Errors
    /// No such board or task, a task or board that is not valid, the store failed.
    pub async fn write_card(
        &self,
        op: CardOp,
        board_id: fs_objects::ObjectId,
        revision: Revision,
        board: Board,
    ) -> Result<Outcome> {
        let _one_at_a_time = self.writes.lock().await;
        let board_stored = self
            .board_object(board_id)
            .await?
            .ok_or(Error::NotFound(board_id))?;
        if super::boards::read(board_stored.clone())?.revision() != revision {
            return Err(Error::Changed(board_id));
        }
        let board_put = Self::board_put(board_stored, &board)?;
        let (first, created) = match op {
            CardOp::Create(task) => {
                if let Some(existing) = self.stored_by_uid(&task.uid).await? {
                    return Err(Error::Exists(existing.envelope.id));
                }
                let (object, id) = self.new_object(*task, None).await?;
                (
                    Write::Put {
                        object,
                        expect: None,
                    },
                    Some(id),
                )
            }
            CardOp::Move { uid, expect } => {
                let stored = self
                    .stored_by_uid(&uid)
                    .await?
                    .ok_or_else(|| Error::Rejected(format!("no task {uid:?}")))?;
                (
                    Write::Put {
                        object: stored,
                        expect,
                    },
                    None,
                )
            }
        };
        let id = match &first {
            Write::Put { object, .. } => object.envelope.id,
            _ => board_id,
        };
        let outcome = self
            .write_unit(vec![
                first,
                Write::Put {
                    object: board_put,
                    expect: None,
                },
            ])
            .await?;
        if matches!(outcome, Outcome::Done(_)) {
            self.feed.notify(&match created {
                Some(id) => TaskChange::Created(id),
                None => TaskChange::Updated(id),
            });
        }
        Ok(outcome)
    }

    /// Take back the unit `change` as one new unit (`revert_by` of the store), with the origin and step of this handle.
    ///
    /// # Errors
    /// The store failed, or it keeps no origin.
    pub async fn revert_change(&self, change: ActivityId) -> Result<Outcome> {
        let store = self.store.as_ref().ok_or_else(no_store)?;
        Ok(store.revert_by(change, &self.by).await?)
    }

    /// The heads of the units of change that match `filter`, ascending.
    ///
    /// # Errors
    /// The store failed, or it keeps no origin.
    pub async fn changes_of(
        &self,
        filter: &fs_objects::ChangeFilter,
    ) -> Result<Vec<fs_objects::Activity>> {
        let store = self.store.as_ref().ok_or_else(no_store)?;
        Ok(store.changes_by(filter).await?)
    }

    /// The log entry with this id (a change that is gone is told apart from one that never was).
    ///
    /// # Errors
    /// The store failed.
    pub async fn activity_of(&self, id: ActivityId) -> Result<Option<fs_objects::Activity>> {
        let store = self.store.as_ref().ok_or_else(no_store)?;
        let before = uuid::Uuid::from_u128(id.as_uuid().as_u128().saturating_sub(1));
        let filter = ActivityFilter {
            after: ActivityId::from_uuid(before).ok(),
            limit: Some(1),
            ..ActivityFilter::default()
        };
        Ok(store
            .activities(&filter)
            .await?
            .into_iter()
            .find(|a| a.id == id))
    }

    /// Every committed activity of the store, after its commit; a slow receiver gets `Lagged`.
    ///
    /// # Errors
    /// There is no whole store behind this handle.
    pub fn subscribe_changes(&self) -> Result<broadcast::Receiver<fs_objects::Activity>> {
        Ok(self.store.as_ref().ok_or_else(no_store)?.subscribe())
    }

    /// The object of a task with this uid (id and `rev`), for a caller that wants to say what it saw.
    ///
    /// # Errors
    /// The store failed.
    pub async fn rev_of(&self, uid: &str) -> Result<Option<(TaskEntry, u64)>> {
        let Some(stored) = self.stored_by_uid(uid).await? else {
            return Ok(None);
        };
        let rev = stored.rev;
        Ok(Some((super::crud::entry_of(stored)?, rev)))
    }
}

fn no_store() -> Error {
    Error::Unsupported(fs_interfaces::Unsupported::new(
        "change",
        "this handle has no whole store behind it",
    ))
}

/// A unit that did not go through, as an error (no `expect` was given, so a clash is strange but still told).
pub(super) fn refused(outcome: &Outcome) -> Error {
    match outcome {
        Outcome::Done(_) => Error::Conflict("written".into()),
        Outcome::Conflict { by, .. } => Error::Conflict(format!(
            "changed since it was read (by {}); nothing was written",
            by.actor
        )),
        Outcome::NotRevertible(why) => Error::Conflict(format!("cannot be taken back: {why}")),
        _ => Error::Conflict("the object is gone".into()),
    }
}
