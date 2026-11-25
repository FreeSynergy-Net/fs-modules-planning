//! What `PlanningCore` says to the wrapper about scopes and conflicts (Stufe 4 B4a), called directly (the core, not a socket):
//! `resolve_scope` and `follows` with the board ids, `scopes` of every change of the feed, and the record a conflict names.
//!
//! Pattern: Scenario tests (given a store, when a change is made, then what the core reports).
#![allow(clippy::too_many_lines)]

use std::cell::RefCell;
use std::rc::Rc;

use super::fixture::{FORM, empty, world};
use super::scope::{card, wait_for};
use super::{cx, inv};
use fs_interfaces::module::ModuleCore;
use fs_modules_planning::PlanningCore;
use fs_tasks_core::{Boards, TaskItems};
use tokio::task::LocalSet;

/// A conflict names the record its fields belong to (else the wrapper blanks `current`: it cannot tell what is hidden there).
#[tokio::test(flavor = "current_thread")]
async fn a_conflict_names_its_record() {
    LocalSet::new()
        .run_until(Box::pin(async {
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
        }))
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
