//! The planning module as a `ModuleCore` for `fs_rpc::Guarded`: reading, changing, taking back and following, and nothing else.
//!
//! Pattern: Template Method (steps) - this delivers only the steps; check -> change -> record -> report stands once in
//! `Guarded<PlanningCore>`. Adapter: the bodies are the ones of the level 1 service (`formsource.rs`, `level2*.rs`), read through the
//! values of `fs_interfaces::module`. Rights are NOT asked here (`rules.rs`). Who writes: the actor is the IRI of the system id of the
//! connection (`cx.caller`, the same text as `actor_of`), the origin is what the program said of itself (`cx.claimed`), the owner is
//! the owner of the store (the repo says it); a module keeps no "current user".
//!
//! Not wired yet (B4 switches `root.rs` over and deletes the old ways). `feed` needs the `LocalSet` of the program (it spawns a local task).

#![allow(clippy::result_large_err)] // `Refusal` is the one refusal type of the contract `ModuleCore`

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use fs_interfaces::module::{
    Applied, Blob, Caller, ChangeFeed, ChangePage, ChangesQuery, Context, Follow, Invocation,
    ModuleCore, Page, PageQuery, RecordView, Refusal, RightsRules, StepRef, Value,
};
use fs_objects::{ActivityId, Origin};
use fs_tasks_core::{TaskItems, TaskRepo};
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;

use crate::args::Pairs;
use crate::commands::dispatch;
use crate::core_conv::{applied, change_ref, clash, refusal_of, refusal_of_error};
use crate::formsource::{
    FormSourceImpl, LEVEL, PAGE_MAX, PROVIDER, UNKNOWN_FORM, offset_of, serves,
    unwatchable_form_why,
};
use crate::level2::{parse_expect, parse_step, seen};
use crate::level2_calls::{Reverted, revert_of};
use crate::root::actor_iri;
use crate::rules::PlanningRules;
use crate::scope::{ScopeMemo, scopes_of};
use crate::write::{Ran, known, problem};

/// The planning module over one store.
pub struct PlanningCore {
    repo: Arc<TaskRepo>,
    rules: PlanningRules,
    memo: Rc<ScopeMemo>,
    feeds: RefCell<Vec<JoinHandle<()>>>,
}

impl PlanningCore {
    /// The module over `repo` (its owner is the owner of the store); the rules say what the provider of `repo` supports.
    #[must_use]
    pub fn new(repo: Arc<TaskRepo>) -> Self {
        let rules = PlanningRules::new(repo.capabilities());
        Self::with_rules(repo, rules)
    }

    /// The module with other rules (a provider with fewer commands, or a test).
    #[must_use]
    pub fn with_rules(repo: Arc<TaskRepo>, rules: PlanningRules) -> Self {
        Self {
            repo,
            rules,
            memo: Rc::default(),
            feeds: RefCell::default(),
        }
    }

    /// The store handle of ONE connection: written as its actor, from the origin it claimed, as the step if there is one.
    fn handle(&self, cx: &Context, step: Option<&(String, String)>) -> Result<TaskRepo, Refusal> {
        let Caller::Connection { system_id, .. } = &cx.caller else {
            return Err(Refusal::forbidden("no-connection"));
        };
        let claimed = &cx.claimed;
        let origin = Origin::new(&claimed.program, &claimed.instance, &claimed.device);
        let mut repo = self.repo.as_actor(actor_iri(system_id)).with_origin(origin);
        if let Some((run, step)) = step {
            repo = repo.with_step(run, step);
        }
        Ok(repo)
    }

    /// The readers of the level 1 service over the handle of this connection (one body of reading, not two).
    fn source(&self, cx: &Context) -> Result<FormSourceImpl, Refusal> {
        let origin = Origin::new(
            &cx.claimed.program,
            &cx.claimed.instance,
            &cx.claimed.device,
        );
        Ok(FormSourceImpl {
            repo: Arc::new(self.handle(cx, None)?),
            origin: Rc::new(RefCell::new(origin)),
        })
    }
}

impl Drop for PlanningCore {
    fn drop(&mut self) {
        for task in self.feeds.borrow().iter() {
            task.abort();
        }
    }
}

fn limit_of(limit: u32) -> usize {
    (match limit {
        0 => PAGE_MAX,
        n => n.min(PAGE_MAX),
    }) as usize
}

fn pairs(args: &[Value]) -> Vec<(String, String)> {
    args.iter()
        .map(|v| (v.path.clone(), v.text.clone()))
        .collect()
}

impl ModuleCore for PlanningCore {
    fn level(&self) -> u32 {
        LEVEL
    }

    fn provider(&self) -> &str {
        PROVIDER
    }

    fn rules(&self) -> &dyn RightsRules {
        &self.rules
    }

    async fn page(&self, cx: &Context, q: PageQuery) -> Result<Page, Refusal> {
        let me = self.source(cx)?;
        let skip = offset_of(q.cursor.as_bytes()).map_err(|e| refusal_of_error(&e))?;
        let data = me
            .page_data(&q.form, &q.scope, skip, q.limit)
            .await
            .map_err(|e| refusal_of_error(&e))?;
        let mut page = Page::default();
        page.records = data
            .rows
            .iter()
            .map(|row| {
                let mut view = RecordView::default();
                view.id.clone_from(&row[0].1);
                view.may = data.on_record.iter().map(|c| (*c).to_owned()).collect();
                view.values = row[1..].iter().map(|(p, t)| Value::new(p, t)).collect();
                view
            })
            .collect();
        page.total = Some(data.total as u64);
        page.may = data.on_set.iter().map(|c| (*c).to_owned()).collect();
        if data.end < data.total {
            page.next = data.end.to_string();
        }
        Ok(page)
    }

    async fn blob(&self, _cx: &Context, _record: &str, _field: &str) -> Result<Blob, Refusal> {
        Err(refusal_of(&problem(
            "form-source-unsupported",
            "this module has no image or file field",
            501,
        )))
    }

    async fn apply(&self, cx: &Context, c: Invocation) -> Result<Applied, Refusal> {
        let plain = |p: Box<fs_rpc::Problem>| refusal_of(&p);
        let expect = parse_expect(c.expect.as_deref().unwrap_or("")).map_err(plain)?;
        let step = match &c.step {
            Some(s) => parse_step(&s.run, &s.step).map_err(plain)?,
            None => None,
        };
        known(&c.command).map_err(plain)?;
        let repo = self.handle(cx, step.as_ref())?;
        let args = pairs(&c.args);
        let args: &Pairs = &args;
        let record = c.record.unwrap_or_default();
        match dispatch(&repo, &c.command, record, args, expect).await {
            Ok(Ran::Done(done)) => {
                let head = match &done.head {
                    Some(h) => seen(&repo, h).await,
                    None => crate::level2::Seen {
                        object: String::new(),
                        undoes: None,
                    },
                };
                Ok(applied(&done, &head))
            }
            Ok(Ran::Clash(by, paths)) => Err(clash(&by, &seen(&repo, &by).await, &paths)),
            Err(why) => Err(plain(why)),
        }
    }

    async fn revert(&self, cx: &Context, change: &str, step: StepRef) -> Result<Applied, Refusal> {
        let unknown = || {
            refusal_of(&problem(
                "form-source-unknown-change",
                "no such change",
                404,
            ))
        };
        let id = uuid::Uuid::parse_str(change)
            .ok()
            .and_then(|u| ActivityId::from_uuid(u).ok())
            .ok_or_else(unknown)?;
        let step = parse_step(&step.run, &step.step).map_err(|p| refusal_of(&p))?;
        let repo = self.handle(cx, step.as_ref())?;
        match revert_of(&repo, id).await {
            Reverted::Done(made) => {
                let head = match &made.head {
                    Some(h) => seen(&repo, h).await,
                    None => crate::level2::Seen {
                        object: String::new(),
                        undoes: None,
                    },
                };
                Ok(applied(&made, &head))
            }
            Reverted::Clash(by, paths) => Err(clash(&by, &seen(&repo, &by).await, &paths)),
            Reverted::Failed(why) => Err(refusal_of(&why)),
        }
    }

    async fn changes(&self, cx: &Context, q: ChangesQuery) -> Result<ChangePage, Refusal> {
        let me = self.source(cx)?;
        let skip = offset_of(q.cursor.as_bytes()).map_err(|e| refusal_of_error(&e))?;
        let (list, more) = me
            .changes_page(&q.scope, &q.run, skip, limit_of(q.limit))
            .await
            .map_err(|e| refusal_of_error(&e))?;
        let mut page = ChangePage::default();
        for a in &list {
            let change = change_ref(a, &seen(&me.repo, a).await);
            if !q.scope.is_empty() {
                // `follows` is synchronous: what the store says about the scope is decided here, once
                self.memo
                    .put(&change.activity, scopes_of(&me.repo, a).await);
            }
            page.changes.push(change);
        }
        if more {
            page.next = (skip + list.len()).to_string();
        }
        Ok(page)
    }

    fn feed(&self, sink: ChangeFeed) {
        let Ok(mut rx) = self.repo.subscribe_changes() else {
            return;
        };
        let (repo, memo) = (self.repo.clone(), self.memo.clone());
        let task = tokio::task::spawn_local(async move {
            loop {
                match rx.recv().await {
                    // the head of a unit only (its members are not changes of their own)
                    Ok(a) if a.unit.is_none() => {
                        let change = change_ref(&a, &seen(&repo, &a).await);
                        memo.put(&change.activity, scopes_of(&repo, &a).await);
                        sink(&change);
                    }
                    // `ChangeFeed` has no way to say "you missed some" (card): the next change goes on
                    Ok(_) | Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => break,
                }
            }
        });
        self.feeds.borrow_mut().push(task);
    }

    fn follows(&self, _cx: &Context, form: &str, scope: &str) -> Result<Follow, Refusal> {
        if !form.is_empty() && !serves(form) {
            return Err(refusal_of(&problem(
                UNKNOWN_FORM,
                unwatchable_form_why(),
                404,
            )));
        }
        if scope.is_empty() {
            return Ok(Follow::all());
        }
        let (memo, scope) = (self.memo.clone(), scope.to_owned());
        Ok(Follow::by(move |c| memo.holds(&c.activity, &scope)))
    }
}
