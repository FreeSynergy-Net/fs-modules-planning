//! The scope of `watch` and `changes` over `Guarded<PlanningCore>` (Stufe 4 B4a): the scope is resolved to the fixed id of the board BEFORE
//! `follows`, so a name survives a rename; an unknown scope is 404; a feed that lost changes ends the watches; a conflict names its record.
//!
//! Pattern: Scenario tests (given a store with boards, when a card or a board changes, then what a subscriber hears) through the wrapper,
//! not through the core: the wrapper is what a program meets.
#![allow(clippy::too_many_lines)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use super::fixture::{FORM, World, empty, world};
use super::{cx, inv, open, ports};
use fs_interfaces::common_capnp::cursor;
use fs_interfaces::formsource_capnp::{change_ref, change_sink, form_source};
use fs_interfaces::module::ModuleCore;
use fs_modules_planning::PlanningCore;
use fs_rpc::Guarded;
use fs_tasks_core::{Boards, Task, TaskItems};
use tokio::task::LocalSet;

/// The client side of a `watch`: the records heard, and whether the hub ended the subscription.
struct Heard {
    seen: Rc<RefCell<Vec<String>>>,
    ended: Rc<Cell<bool>>,
}

impl change_sink::Server for Heard {
    async fn changed(
        self: Rc<Self>,
        params: change_sink::ChangedParams,
    ) -> Result<(), capnp::Error> {
        let c: change_ref::Reader<'_> = params.get()?.get_change()?;
        self.seen.borrow_mut().push(c.get_object()?.to_string()?);
        Ok(())
    }
    async fn ended(
        self: Rc<Self>,
        _: change_sink::EndedParams,
        _: change_sink::EndedResults,
    ) -> Result<(), capnp::Error> {
        self.ended.set(true);
        Ok(())
    }
}

type Watched = (
    Rc<RefCell<Vec<String>>>,
    Rc<Cell<bool>>,
    Result<capnp::capability::Response<form_source::watch_results::Owned>, capnp::Error>,
);

async fn watch(client: &form_source::Client, scope: &str) -> Watched {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let ended = Rc::new(Cell::new(false));
    let mut req = client.watch_request();
    req.get().set_form(FORM);
    req.get().set_scope(scope);
    req.get().set_sink(capnp_rpc::new_client(Heard {
        seen: seen.clone(),
        ended: ended.clone(),
    }));
    (seen, ended, req.send().promise.await)
}

async fn wait_for(done: impl Fn() -> bool) {
    for _ in 0..150 {
        if done() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// A card that belongs to the board of the label `label` only.
fn labelled(title: &str, label: &str) -> Task {
    let mut task = Task::new(title);
    task.keywords.insert(label.to_owned(), true);
    task
}

/// A store with the board `work` (label `x`) and the board `other` (label `y`), and a watch on `work` by its NAME. Returns after the
/// first change of the setup was heard (so a later assert about "nothing" is about later changes).
async fn named_watch() -> (World, Guarded<PlanningCore>, Watched, form_source::Client) {
    let w = empty(1024).await;
    w.repo.board_create("work", Some("x")).await.unwrap();
    w.repo.board_create("other", Some("y")).await.unwrap();
    let mut g = Guarded::new(PlanningCore::new(w.repo.clone()), ports());
    let client = open(&mut g);
    let watched = watch(&client, "work").await;
    (w, g, watched, client)
}

/// Makes a card of the board with the label `label`; its id as the form shows it.
async fn card(w: &World, title: &str, label: &str) -> String {
    w.repo
        .create(labelled(title, label))
        .await
        .unwrap()
        .task
        .uid
}

/// A watch by the name of a board hears its board after the board was renamed (the subscription hangs on the id), and still not the others.
#[tokio::test(flavor = "current_thread")]
async fn a_watch_by_name_goes_on_after_the_board_is_renamed() {
    LocalSet::new()
        .run_until(async {
            let (w, _g, (seen, _, answer), _client) = named_watch().await;
            let _sub = answer.unwrap();
            let work = w
                .repo
                .boards()
                .await
                .unwrap()
                .into_iter()
                .find(|b| b.board.name == "work")
                .unwrap();
            let mut edited = work.board.clone();
            edited.name = "renamed".into();
            w.repo
                .board_edit(work.id, work.revision(), edited)
                .await
                .unwrap();
            let mine = card(&w, "mine", "x").await;
            wait_for(|| seen.borrow().contains(&mine)).await;
            let theirs = card(&w, "theirs", "y").await;
            tokio::time::sleep(Duration::from_millis(150)).await;
            let seen = seen.borrow();
            assert!(
                seen.contains(&mine),
                "the renamed board is still heard: {seen:?}"
            );
            assert!(
                !seen.contains(&theirs),
                "another board is not heard: {seen:?}"
            );
        })
        .await;
}

/// A NEW board that takes the old name is another scope: the subscription of the old board does not listen to it.
#[tokio::test(flavor = "current_thread")]
async fn a_new_board_with_the_old_name_is_not_heard() {
    LocalSet::new()
        .run_until(async {
            let (w, _g, (seen, _, answer), _client) = named_watch().await;
            let _sub = answer.unwrap();
            let work = w
                .repo
                .boards()
                .await
                .unwrap()
                .into_iter()
                .find(|b| b.board.name == "work")
                .unwrap();
            let mut edited = work.board.clone();
            edited.name = "renamed".into();
            w.repo
                .board_edit(work.id, work.revision(), edited)
                .await
                .unwrap();
            w.repo.board_create("work", Some("z")).await.unwrap();
            let in_new = card(&w, "new", "z").await;
            let in_old = card(&w, "old", "x").await;
            wait_for(|| seen.borrow().contains(&in_old)).await;
            tokio::time::sleep(Duration::from_millis(150)).await;
            let seen = seen.borrow();
            assert!(seen.contains(&in_old), "the old board is heard: {seen:?}");
            assert!(
                !seen.contains(&in_new),
                "the new board of the same name is not: {seen:?}"
            );
        })
        .await;
}

/// An unknown scope is 404 at `watch` and at `changes` (never "heard nothing" and never an empty page); an empty scope is everything.
#[tokio::test(flavor = "current_thread")]
async fn an_unknown_scope_is_404_at_watch_and_changes() {
    LocalSet::new()
        .run_until(async {
            let w = world().await;
            let mut g = Guarded::new(PlanningCore::new(w.repo.clone()), ports());
            let client = open(&mut g);
            let (_, _, answer) = watch(&client, "nowhere").await;
            let Err(err) = answer else {
                panic!("an unknown scope must not subscribe");
            };
            assert!(
                err.to_string().contains("form-source-unknown-scope"),
                "{err}"
            );
            assert!(err.to_string().contains("404"), "{err}");
            let mut req = client.changes_request();
            {
                let mut p = req.get();
                p.set_scope("nowhere");
                let _: cursor::Builder<'_> = p.init_cursor();
            }
            let Err(err) = req.send().promise.await else {
                panic!("an unknown scope must not give an empty page");
            };
            assert!(
                err.to_string().contains("form-source-unknown-scope"),
                "{err}"
            );
            assert!(
                watch(&client, "work").await.2.is_ok(),
                "a board by its name"
            );
            assert!(watch(&client, "").await.2.is_ok(), "no scope is all");
        })
        .await;
}

/// A store that drops changes before the feed reads them (a channel of ONE) ends every subscription, with `ended`; a new `watch` listens.
#[tokio::test(flavor = "current_thread")]
async fn a_lagged_feed_ends_the_watches() {
    LocalSet::new()
        .run_until(async {
            let w = empty(1).await;
            w.repo.board_create("work", Some("x")).await.unwrap();
            let mut g = Guarded::new(PlanningCore::new(w.repo.clone()), ports());
            let client = open(&mut g);
            let (_, ended, answer) = watch(&client, "work").await;
            let _sub = answer.unwrap();
            for i in 0..20 {
                w.repo
                    .create(labelled(&format!("c{i}"), "x"))
                    .await
                    .unwrap();
            }
            wait_for(|| ended.get()).await;
            assert!(ended.get(), "the watch was ended, not left deaf");
            let (seen, ended, answer) = watch(&client, "work").await;
            let _again = answer.unwrap();
            let uid = card_in(&w).await;
            wait_for(|| seen.borrow().contains(&uid)).await;
            assert!(seen.borrow().contains(&uid), "a new watch listens again");
            assert!(!ended.get());
        })
        .await;
}

async fn card_in(w: &World) -> String {
    tokio::time::sleep(Duration::from_millis(100)).await; // the feed has caught up with the burst
    card(w, "later", "x").await
}

/// A conflict names the record its fields belong to (else the wrapper blanks `current`: it cannot tell what is hidden there).
#[tokio::test(flavor = "current_thread")]
async fn a_conflict_names_its_record() {
    LocalSet::new()
        .run_until(async {
            let w = world().await;
            let core = PlanningCore::new(w.repo.clone());
            let cx = cx();
            let uid = &w.uids[0];
            // somebody else changes the title after this caller read the card (revision `seen`)
            let (entry, seen) = w.repo.rev_of(uid).await.unwrap().unwrap();
            let mut edited = entry.task.clone();
            edited.title = Some("changed meanwhile".into());
            w.repo.edit(entry.id, edited).await.unwrap();
            let mut stale = inv("card.move", uid, &[("board", "work"), ("column", "doing")]);
            stale.expect = Some(seen.to_string());
            let refused = core.apply(&cx, stale).await.unwrap_err();
            let conflict = refused.conflict.expect("a conflict");
            assert!(!conflict.fields.is_empty(), "{conflict:?}");
            for field in &conflict.fields {
                assert_eq!(&field.record, uid, "{field:?}");
                assert_eq!(field.record, conflict.by.object, "{field:?}");
            }
        })
        .await;
}

/// `follows` takes the id `resolve_scope` gave: it admits the changes of its board and no others, and every change says its boards.
#[tokio::test(flavor = "current_thread")]
async fn follows_admits_the_changes_of_the_resolved_board_only() {
    LocalSet::new()
        .run_until(async {
            let w = empty(1024).await;
            w.repo.board_create("work", Some("x")).await.unwrap();
            w.repo.board_create("other", Some("y")).await.unwrap();
            let core = PlanningCore::new(w.repo.clone());
            let heard: Rc<RefCell<Vec<fs_interfaces::module::ChangeRef>>> = Rc::default();
            let sink = heard.clone();
            core.feed(Box::new(move |c| sink.borrow_mut().push(c.clone())));
            let cx = cx();
            let bad = core.follows(&cx, "rows@1.0.0", "").unwrap_err();
            assert_eq!((bad.kind.as_str(), bad.status), ("unknown-form", 404));
            let nowhere = core.resolve_scope(&cx, FORM, "nowhere").await.unwrap_err();
            assert_eq!(
                (nowhere.kind.as_str(), nowhere.status),
                ("unknown-scope", 404)
            );
            let work_id = core.resolve_scope(&cx, FORM, "work").await.unwrap();
            assert_eq!(
                core.resolve_scope(&cx, FORM, &work_id).await.unwrap(),
                work_id
            );
            let other_id = core.resolve_scope(&cx, FORM, "other").await.unwrap();
            assert_ne!(work_id, other_id);
            assert_eq!(core.resolve_scope(&cx, FORM, "").await.unwrap(), "");
            let work = core.follows(&cx, FORM, &work_id).unwrap();
            let other = core.follows(&cx, FORM, &other_id).unwrap();
            let all = core.follows(&cx, "", "").unwrap();
            let _ = card(&w, "n", "x").await;
            wait_for(|| !heard.borrow().is_empty()).await;
            let heard = heard.borrow();
            assert!(!heard.is_empty(), "the feed told the change");
            assert!(
                heard.iter().all(|c| c.scopes == [work_id.clone()]),
                "{heard:?}"
            );
            assert!(heard.iter().all(|c| work.admits(c)), "{heard:?}");
            assert!(heard.iter().all(|c| !other.admits(c)), "{heard:?}");
            assert!(heard.iter().all(|c| all.admits(c)));
        })
        .await;
}
