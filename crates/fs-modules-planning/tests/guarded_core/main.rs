//! The suite `module_v1` (level 2) over `Guarded<PlanningCore>` in this process, the check that a command the provider cannot do is
//! refused before `apply`, and the scope of `follows` (`scope.rs`). The service (`Planning`) serves exactly this wrapper.
//!
//! Pattern: Template Method (`run_conformance` is the order of the rules; this file plugs the subject in, as `fs-rpc` `tests/guarded_g8.rs`
//! does for `FakeCore`). Counter-proof by Decorator: the same module with rules "everything is allowed" (a wrong Strategy).
#![cfg(unix)]
#![allow(clippy::unused_async_trait_impl, clippy::result_large_err)]

#[path = "../common/mod.rs"]
mod common;
mod core_calls;
mod fixture;
mod scope;

use std::cell::Cell;
use std::rc::Rc;

use capnp::capability::FromClientHook;
use common::suite::Planning;
use fixture::{FORM, World, world};
use fs_interfaces::Capabilities;
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::module::{
    Actor, Applied, Ask, Blob, Caller, ChangeFeed, ChangePage, ChangesQuery, Context, FeedBreak,
    Follow, HiddenReason, Invocation, ModuleCore, Page, PageQuery, Refusal, RightsRules, StepRef,
    Verdict,
};
use fs_interfaces::owner::fake::person;
use fs_module::conformance::run_conformance;
use fs_modules_planning::{PlanningCore, PlanningRules};
use fs_rpc::{Caller as Peer, Guarded, PerConnection, Ports};
use tokio::task::LocalSet;

const OWNER: &str = "urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e01";

/// The skips that are right by design (the same reasons as `tests/conformance.rs`): "11 forbidden" (no rights per person: nothing is
/// locked in planning), "L2-9 gone" (the contract has no redaction), "20 blob" (planning has no image field).
const ALLOWED_SKIPS: [&str; 3] = ["11 forbidden", "L2-9 gone", "20 blob"];

fn ports() -> Ports {
    Ports::new(|_: &Peer| Actor::Person, |_: &Peer| Some(person(OWNER)))
}

fn open<M: ModuleCore>(g: &mut Guarded<M>) -> form_source::Client {
    FromClientHook::new(g.bootstrap(&Peer::new("1000", None)).hook)
}

/// Pattern: Strategy (the wrong variant of `RightsRules`) - says yes to everything.
struct AllowAll;

impl RightsRules for AllowAll {
    fn may(&self, _: &Ask<'_>) -> Verdict {
        Verdict::Allow
    }
    fn hidden(&self, _: &Context, _: &str, _: &str) -> Option<HiddenReason> {
        None
    }
}

/// Pattern: Decorator - the planning module, counting `apply`, with its own rules or with `AllowAll`.
struct Counting {
    inner: PlanningCore,
    lax: Option<AllowAll>,
    applied: Rc<Cell<u32>>,
}

impl ModuleCore for Counting {
    fn level(&self) -> u32 {
        self.inner.level()
    }
    fn provider(&self) -> &str {
        self.inner.provider()
    }
    fn rules(&self) -> &dyn RightsRules {
        match &self.lax {
            Some(lax) => lax,
            None => self.inner.rules(),
        }
    }
    async fn page(&self, cx: &Context, q: PageQuery) -> Result<Page, Refusal> {
        self.inner.page(cx, q).await
    }
    async fn blob(&self, cx: &Context, r: &str, f: &str) -> Result<Blob, Refusal> {
        self.inner.blob(cx, r, f).await
    }
    async fn apply(&self, cx: &Context, c: Invocation) -> Result<Applied, Refusal> {
        self.applied.set(self.applied.get() + 1);
        self.inner.apply(cx, c).await
    }
    async fn revert(&self, cx: &Context, c: &str, s: StepRef) -> Result<Applied, Refusal> {
        self.inner.revert(cx, c, s).await
    }
    async fn changes(&self, cx: &Context, q: ChangesQuery) -> Result<ChangePage, Refusal> {
        self.inner.changes(cx, q).await
    }
    fn feed(&self, sink: ChangeFeed) {
        self.inner.feed(sink);
    }
    fn on_feed_break(&self, sink: FeedBreak) {
        self.inner.on_feed_break(sink);
    }
    async fn resolve_scope(
        &self,
        cx: &Context,
        form: &str,
        scope: &str,
    ) -> Result<String, Refusal> {
        self.inner.resolve_scope(cx, form, scope).await
    }
    fn follows(&self, cx: &Context, form: &str, scope: &str) -> Result<Follow, Refusal> {
        self.inner.follows(cx, form, scope)
    }
}

/// The provider that cannot delete: every command but `task.delete`.
fn caps_without_delete() -> Capabilities {
    let mut caps = Capabilities::new("fs-tasks");
    for id in ["card.move", "task.create"] {
        caps.supported(id);
    }
    caps
}

#[tokio::test(flavor = "current_thread")]
async fn the_suite_of_module_v1_passes_over_guarded_planning_core() {
    LocalSet::new()
        .run_until(async {
            let w = world().await;
            let mut g = Guarded::new(PlanningCore::new(w.repo.clone()), ports());
            let fx = Planning {
                client: open(&mut g),
                other: open(&mut g),
                uids: w.uids.clone(),
            };
            let report = run_conformance(&fx).await;
            assert!(report.failures.is_empty(), "{:#?}", report.failures);
            assert!(
                report
                    .skipped
                    .iter()
                    .all(|s| ALLOWED_SKIPS.iter().any(|a| s.starts_with(a))),
                "{:#?}",
                report.skipped
            );
            assert!(report.checked > 30, "{}", report.checked);
            eprintln!("skipped: {:#?}", report.skipped);
        })
        .await;
}

/// `task.delete` against a provider that cannot delete: 403 in the answer, `apply` never ran, the card is still there.
/// Returns whether that held (the counter-proof flips the rules and expects `false`).
async fn unsupported_is_refused_before_apply(lax: bool) -> bool {
    let w: World = world().await;
    let rules = PlanningRules::new(caps_without_delete());
    let applied = Rc::new(Cell::new(0));
    let core = Counting {
        inner: PlanningCore::with_rules(w.repo.clone(), rules),
        lax: lax.then_some(AllowAll),
        applied: applied.clone(),
    };
    let mut g = Guarded::new(core, ports());
    let client = open(&mut g);
    let a = common::invoke(&client, "task.delete", &w.uids[1], &[]).await;
    let still = common::page(&client, FORM, "work")
        .await
        .unwrap()
        .iter()
        .any(|r| r.0 == w.uids[1]);
    a.failed && a.kind == "form-source-forbidden" && applied.get() == 0 && still
}

#[tokio::test(flavor = "current_thread")]
async fn a_command_the_provider_cannot_do_is_403_and_apply_is_not_called() {
    LocalSet::new()
        .run_until(async { assert!(unsupported_is_refused_before_apply(false).await) })
        .await;
}

/// Counter-proof: with rules "everything is allowed" the same check fails (the ban comes from `PlanningRules`).
#[tokio::test(flavor = "current_thread")]
async fn counter_proof_rules_that_allow_everything_let_apply_run() {
    LocalSet::new()
        .run_until(async { assert!(!unsupported_is_refused_before_apply(true).await) })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn the_page_offers_only_what_the_provider_supports() {
    LocalSet::new()
        .run_until(async {
            let w = world().await;
            let core =
                PlanningCore::with_rules(w.repo.clone(), PlanningRules::new(caps_without_delete()));
            let mut g = Guarded::new(core, ports());
            let (set, records) = common::may_of(&open(&mut g), FORM, "work").await;
            assert_eq!(set, ["task.create"]);
            assert!(
                records.iter().all(|(_, may)| may == &["card.move"]),
                "{records:?}"
            );
        })
        .await;
}

fn cx() -> Context {
    let who = Caller::Connection {
        system_id: "1000".into(),
        system_name: None,
    };
    Context::from_connection(who, Actor::Person, person(OWNER))
}

fn inv(command: &str, record: &str, args: &[(&str, &str)]) -> Invocation {
    let mut i = Invocation::default();
    i.command = command.into();
    i.record = (!record.is_empty()).then(|| record.to_owned());
    i.args = args
        .iter()
        .map(|(p, t)| fs_interfaces::module::Value::new(p, t))
        .collect();
    i
}
