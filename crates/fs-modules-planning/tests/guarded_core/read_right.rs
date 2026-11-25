//! The read right (`form.read`, asked by `fs_rpc::Guarded` for every record of a page): the rules of planning allow it on purpose, so a
//! page over `Guarded<PlanningCore>` is not empty, even for a provider that supports no command and for a caller who is not a person.
//!
//! Pattern: Strategy (`PlanningRules` is the Strategy under test; the wrapper only asks). There are no rights per person yet (only the
//! owner reaches the socket), so a caller other than the owner gets what the rules say: the same as the owner. Rights per person
//! replace the Strategy later and change this test then.
use fs_interfaces::Capabilities;
use fs_interfaces::module::Actor;
use fs_interfaces::owner::fake::person;
use fs_modules_planning::{PlanningCore, PlanningRules};
use fs_rpc::{Caller as Peer, Guarded, Ports};
use tokio::task::LocalSet;

use super::fixture::{FORM, world};
use super::{OWNER, open};

async fn rows_for(actor: fn() -> Actor, caps: Capabilities) -> Vec<String> {
    let w = world().await;
    let core = PlanningCore::with_rules(w.repo.clone(), PlanningRules::new(caps));
    let ports = Ports::new(move |_: &Peer| actor(), |_: &Peer| Some(person(OWNER)));
    let mut g = Guarded::new(core, ports);
    let rows = super::common::page(&open(&mut g), FORM, "work")
        .await
        .unwrap();
    rows.into_iter().map(|r| r.0).collect()
}

#[tokio::test(flavor = "current_thread")]
async fn the_page_is_readable_through_guarded_and_not_empty() {
    LocalSet::new()
        .run_until(async {
            let w = world().await;
            let ids = rows_for(|| Actor::Person, Capabilities::new("fs-tasks")).await;
            assert!(
                !ids.is_empty(),
                "form.read must be allowed: the page is empty"
            );
            for uid in &w.uids {
                assert!(ids.contains(uid), "{uid} missing in {ids:?}");
            }
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_caller_that_is_not_a_person_reads_what_the_rules_say_the_same_as_the_owner() {
    LocalSet::new()
        .run_until(async {
            let owner = rows_for(|| Actor::Person, Capabilities::new("fs-tasks")).await;
            let bot = rows_for(|| Actor::Bot, Capabilities::new("fs-tasks")).await;
            assert!(!bot.is_empty());
            let mut a = owner.clone();
            let mut b = bot.clone();
            a.sort();
            b.sort();
            assert_eq!(a, b);
        })
        .await;
}
