//! The five calls of `FormSource` level 2, one function each (the trait impl in `formsource.rs` only hands over).
//!
//! Pattern: Command (a call is read, run through the store as one unit and answered) + Adapter (outcomes of the store as the problems
//! and the typed conflict of `formsource.capnp`). A refusal is in the answer (`failed = true`), an exception means a broken line.

use fs_interfaces::formsource_capnp::form_source;
use fs_objects::{ActivityId, Outcome};

use crate::errors::rpc_error;
use crate::formsource::FormSourceImpl;
use crate::formsource::{PAGE_MAX, read_offset, serves};
use crate::level2::{
    GONE, NOT_REVERTIBLE, Seen, UNKNOWN_CHANGE, clash_problem, parse_expect, parse_step, read_uuid,
    seen, write_change, write_clash, write_done, write_problem,
};
use crate::level2_watch::start_watch;
use crate::write::{Ran, boxed, problem, to_problem};

type Reply = Result<(), capnp::Error>;

/// The record id of the head of a unit that was written, for the `ChangeRef` of the answer.
pub(crate) async fn head_seen(me: &FormSourceImpl, done: &crate::write::Done) -> Seen {
    match &done.head {
        Some(head) => seen(&me.repo, head).await,
        None => Seen {
            object: String::new(),
            undoes: None,
        },
    }
}

/// `invokeIf`: the command of `invoke`, with the `rev` the caller saw and the step that makes a repeat act once.
pub(crate) async fn invoke_if(
    me: &FormSourceImpl,
    params: form_source::InvokeIfParams,
    mut results: form_source::InvokeIfResults,
) -> Reply {
    let p = params.get()?;
    let command = p.get_command()?.to_string()?;
    let record = p.get_record()?.to_string()?;
    let wanted = parse_expect(p.get_expect()?.to_str()?);
    let step = p.get_step()?;
    let step = parse_step(step.get_run()?.to_str()?, step.get_step()?.to_str()?);
    let mut out = results.get();
    let ran = match (wanted, step) {
        (Ok(expect), Ok(step)) => me.run(&command, record, p.get_args()?, expect, step).await,
        (Err(why), _) | (_, Err(why)) => Err(why),
    };
    match ran {
        Ok(Ran::Done(done)) => {
            let seen = head_seen(me, &done).await;
            write_done(out.init_result(), &done, &seen);
        }
        Ok(Ran::Clash(by, paths)) => {
            out.set_failed(true);
            write_problem(out.reborrow().init_problem(), &clash_problem());
            let seen = seen(&me.repo, &by).await;
            write_clash(out.init_conflict(), (&by, &seen), &paths);
        }
        Err(why) => {
            out.set_failed(true);
            write_problem(out.init_problem(), &why);
        }
    }
    Ok(())
}

/// `revert`: write the before-images of the unit as a new `Undo` unit, as the step; a clash, a redacted or an unknown change is told.
pub(crate) async fn revert(
    me: &FormSourceImpl,
    params: form_source::RevertParams,
    mut results: form_source::RevertResults,
) -> Reply {
    let p = params.get()?;
    let bits = read_uuid(p.get_change()?);
    let step = p.get_step()?;
    let step = parse_step(step.get_run()?.to_str()?, step.get_step()?.to_str()?);
    let mut out = results.get();
    let id = ActivityId::from_uuid(uuid::Uuid::from_u128(bits)).ok();
    let step = match (id, step) {
        (Some(id), Ok(step)) => Ok((id, step)),
        (None, _) => Err(boxed(UNKNOWN_CHANGE, "no such change", 404)),
        (_, Err(why)) => Err(why),
    };
    let (id, step) = match step {
        Ok(ok) => ok,
        Err(why) => {
            out.set_failed(true);
            write_problem(out.init_problem(), &why);
            return Ok(());
        }
    };
    match me.handle(step.as_ref()).revert_change(id).await {
        Ok(Outcome::Done(done)) => {
            let made = crate::write::Done {
                record: done.head.object.to_string(),
                text: "change-reverted",
                head: Some(Box::new(done.head)),
            };
            let seen = head_seen(me, &made).await;
            write_done(out.init_result(), &made, &seen);
        }
        Ok(Outcome::Conflict { by, fields }) => {
            let paths: Vec<String> = fields.into_iter().map(|f| f.path).collect();
            out.set_failed(true);
            write_problem(out.reborrow().init_problem(), &clash_problem());
            let seen = seen(&me.repo, &by).await;
            write_clash(out.init_conflict(), (&by, &seen), &paths);
        }
        Ok(Outcome::NotRevertible(why)) => {
            out.set_failed(true);
            write_problem(out.init_problem(), &problem(NOT_REVERTIBLE, why, 422));
        }
        Ok(_) => {
            out.set_failed(true);
            let known = me.repo.activity_of(id).await.ok().flatten().is_some();
            let why = if known {
                problem(
                    GONE,
                    "the change was blacked out or its record is gone",
                    410,
                )
            } else {
                problem(UNKNOWN_CHANGE, "no such change", 404)
            };
            write_problem(out.init_problem(), &why);
        }
        Err(e) => {
            out.set_failed(true);
            write_problem(out.init_problem(), &to_problem(e));
        }
    }
    Ok(())
}

/// `changes`: the heads of the changes of the own program (run empty) or of one run, newest first.
pub(crate) async fn changes(
    me: &FormSourceImpl,
    params: form_source::ChangesParams,
    mut results: form_source::ChangesResults,
) -> Reply {
    let p = params.get()?;
    let scope = p.get_scope()?.to_string()?;
    let run = p.get_run()?.to_string()?;
    let skip = read_offset(p.get_cursor()?)?;
    let limit = match p.get_limit() {
        0 => PAGE_MAX,
        n => n.min(PAGE_MAX),
    } as usize;
    let (list, more) = me.changes_page(&scope, &run, skip, limit).await?;
    let mut seen_all = Vec::with_capacity(list.len());
    for a in &list {
        seen_all.push(seen(&me.repo, a).await);
    }
    let mut out = results.get();
    let mut items = out
        .reborrow()
        .init_items(u32::try_from(list.len()).unwrap_or(0));
    for (i, a) in (0u32..).zip(&list) {
        write_change(items.reborrow().get(i), a, &seen_all[i as usize]);
    }
    if more {
        out.init_next()
            .set_opaque((skip + list.len()).to_string().as_bytes());
    }
    Ok(())
}

/// `watch`: every change in the scope, by whomever, after its commit, until the subscription is dropped.
pub(crate) async fn watch(
    me: &FormSourceImpl,
    params: form_source::WatchParams,
    mut results: form_source::WatchResults,
) -> Reply {
    let p = params.get()?;
    let form = p.get_form()?.to_str()?;
    if !serves(form) {
        let why = crate::formsource::unwatchable_form_why();
        return Err(problem(crate::formsource::UNKNOWN_FORM, why, 404).into_error());
    }
    let scope = p.get_scope()?.to_string()?;
    let board = me.board_of(&scope).await?;
    let feed = me.repo.subscribe_changes().map_err(rpc_error)?;
    let subscription = start_watch(me.repo.clone(), board, p.get_sink()?, feed);
    results.get().set_subscription(subscription);
    Ok(())
}
