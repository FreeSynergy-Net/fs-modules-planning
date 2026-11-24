//! `FormSource` level 2 of the planning module: compare-and-set (`invokeIf`), undo (`revert`), the history (`changes`), who is asking
//! (`introduce`) and the push of changes (`watch`), over the units of work of the store (`write_by`, `revert_by`, `changes_by`).
//!
//! Pattern: Command with an expectation (`invokeIf`), Compensating Transaction (`revert`), Observer (`changes` queries, `watch` pushes),
//! Context Object (`introduce`: the origin of THIS connection, in one `Rc<RefCell<Origin>>` that `bootstrap` makes per connection, never global).

use std::cell::RefCell;
use std::rc::Rc;

use fs_interfaces::common_capnp::{problem, uuid};
use fs_interfaces::formsource_capnp::{change_ref, conflict, origin as origin_wire, write_result};
use fs_objects::{Activity, ChangeFilter, Origin};

use crate::scope::Scope;
use fs_rpc::Problem;
use fs_types::BasicValue as _;

use crate::errors::rpc_error;
use crate::formsource::FormSourceImpl;
use crate::write::{Done, Failure, boxed, problem as new_problem};

pub(crate) const UNKNOWN_CHANGE: &str = "form-source-unknown-change";
const CONFLICT: &str = "form-source-conflict";
pub(crate) const NOT_REVERTIBLE: &str = "form-source-not-revertible";
pub(crate) const GONE: &str = "form-source-gone";
/// Problem type of `ChangeSink.ended` when the receiver was too slow.
pub(crate) const FEED_ENDED: &str = "https://freesynergy.net/problems/feed-ended";

/// The origin of one connection: given by `introduce`, empty until then.
pub(crate) type Context = Rc<RefCell<Origin>>;

/// Write a `Common.Problem` (fs-rpc does the work: the detail is cut, the extensions are admitted).
pub(crate) fn write_problem(out: problem::Builder<'_>, p: &Problem) {
    p.write(out);
}

/// The 128 bits of a UUID as the wire has them.
pub(crate) fn write_uuid(mut out: uuid::Builder<'_>, bits: u128) {
    out.set_hi(u64::try_from(bits >> 64).unwrap_or(0));
    out.set_lo(u64::try_from(bits & u128::from(u64::MAX)).unwrap_or(0));
}

pub(crate) fn read_uuid(r: uuid::Reader<'_>) -> u128 {
    (u128::from(r.get_hi()) << 64) | u128::from(r.get_lo())
}

/// A head activity as `ChangeRef`: the record it is about, the revision after and before, where it came from, what and when.
pub(crate) fn write_change(mut out: change_ref::Builder<'_>, a: &Activity, seen: &Seen) {
    write_uuid(out.reborrow().init_activity(), a.id.as_uuid().as_u128());
    out.set_object(seen.object.as_str());
    out.set_revision(a.revision.to_string().as_str());
    let before = if a.kind.as_str() == "Create" {
        String::new()
    } else {
        a.revision.saturating_sub(1).to_string()
    };
    out.set_before(before.as_str());
    {
        let mut o: origin_wire::Builder<'_> = out.reborrow().init_origin();
        let person = a
            .owner
            .as_ref()
            .map_or_else(|| a.actor.clone(), |p| p.id.canonical());
        o.set_person(person.as_str());
        o.set_program(a.origin.program.as_str());
        o.set_instance(a.origin.instance.as_str());
        o.set_device(a.origin.device.as_str());
    }
    out.set_kind(a.kind.as_str());
    out.reborrow()
        .init_at()
        .set_unix_millis(a.at.timestamp_millis());
    if let Some(undoes) = seen.undoes {
        write_uuid(out.init_undoes(), undoes);
    }
}

/// What a command made or changed, for `WriteResult`.
pub(crate) fn write_done(mut out: write_result::Builder<'_>, done: &Done, seen: &Seen) {
    out.set_record(done.record.as_str());
    out.set_text(done.text);
    if let Some(head) = &done.head {
        write_change(out.init_change(), head, seen);
    }
}

/// The conflict of an `expect` or an undo: the last change and the paths that changed since.
pub(crate) fn write_clash(
    mut out: conflict::Builder<'_>,
    (by, seen): (&Activity, &Seen),
    paths: &[String],
) {
    write_change(out.reborrow().init_by(), by, seen);
    let mut list = out.init_fields(u32::try_from(paths.len()).unwrap_or(0));
    for (i, path) in (0u32..).zip(paths) {
        list.reborrow().get(i).set_path(path.as_str());
    }
}

/// The problem of a clash: 409, `issue=changed-since`.
pub(crate) fn clash_problem() -> Problem {
    new_problem(CONFLICT, "the record was changed since it was read", 409)
        .with_ext("issue", "changed-since")
}

/// `expect` as the text of a revision (empty = any).
pub(crate) fn parse_expect(text: &str) -> Result<Option<u64>, Failure> {
    if text.is_empty() {
        return Ok(None);
    }
    text.parse().map(Some).map_err(|_| {
        boxed(
            crate::write::BAD_ARGS,
            "`expect` is not a revision (decimal text)",
            400,
        )
    })
}

/// `step` as (run, step): both empty = no step; one of them empty is refused.
pub(crate) fn parse_step(run: &str, step: &str) -> Result<Option<(String, String)>, Failure> {
    match (run.is_empty(), step.is_empty()) {
        (true, true) => Ok(None),
        (false, false) => Ok(Some((run.to_owned(), step.to_owned()))),
        _ => Err(boxed(
            crate::write::BAD_ARGS,
            "`step.run` and `step.step` are both given or both empty",
            400,
        )),
    }
}

impl FormSourceImpl {
    /// This connection's repo for one call: written from the introduced origin, as the given step.
    pub(crate) fn handle(&self, step: Option<&(String, String)>) -> fs_tasks_core::TaskRepo {
        let mut repo = self.repo.with_origin(self.origin.borrow().clone());
        if let Some((run, step)) = step {
            repo = repo.with_step(run, step);
        }
        repo
    }

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
            filter.program = Some(self.origin.borrow().program.clone());
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
pub(crate) async fn seen(repo: &fs_tasks_core::TaskRepo, a: &Activity) -> Seen {
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

/// The id of the record a change is about, as the form shows it: the `uid` of the task (`urn:uuid:` and the id for a task made here), else the id of the object.
pub(crate) async fn record_of(repo: &fs_tasks_core::TaskRepo, a: &Activity) -> String {
    match fs_tasks_core::TaskItems::get(repo, a.object).await {
        Ok(Some(entry)) => entry.task.uid,
        Ok(None) if a.kind.as_str() == "Delete" => format!("urn:uuid:{}", a.object),
        _ => a.object.to_string(),
    }
}
