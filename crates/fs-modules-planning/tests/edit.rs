//! `task.edit` over a real service in a temporary folder: the given fields are set and the others stay, an empty text removes an
//! optional field, wrong input is refused with its own kind and writes nothing, a stranger is refused. Each test names what a wrong
//! implementation would get wrong.
//!
//! Pattern: Scenario test (given a board with tasks, when the owner edits, then the core shows exactly that) + Oracle (the task as
//! the core reads it back, not the answer of the command).
#![cfg(unix)]

mod common;

use common::{board_with, call, invoke, serve_for, serve_repo, titles};
use fs_interfaces::formsource_capnp::form_source;
use fs_tasks_core::{TaskItems, TaskRepo};
use tokio::task::LocalSet;

const BAD: &str = "form-source-bad-arguments";

async fn task(repo: &TaskRepo, uid: &str) -> fs_tasks_core::Task {
    repo.get_by_uid(uid)
        .await
        .unwrap()
        .expect("the task is there")
        .task
}

/// All fields at once, then one field alone (the others stay), then an empty text removes an optional field; the title stays
/// through all of it, and the card does not move.
#[tokio::test(flavor = "current_thread")]
async fn the_given_fields_are_set_and_the_others_stay() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop, repo) = serve_repo(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let uids = board_with(&socket, &source.client, &["a", "b"]).await;
            let source = source.client;
            let before = titles(&source).await;
            let done = invoke(
                &source,
                "task.edit",
                &uids[0],
                &[
                    ("title", "  A renamed "),
                    ("description", "details"),
                    ("due", "2026-12-01T09:00:00"),
                    ("priority", "3"),
                ],
            )
            .await;
            assert!(!done.failed, "{done:?}");
            assert_eq!(
                (done.record.as_str(), done.text.as_str()),
                (uids[0].as_str(), "task-edited")
            );
            let t = task(&repo, &uids[0]).await;
            assert_eq!(t.title.as_deref(), Some("A renamed"));
            assert_eq!(t.description.as_deref(), Some("details"));
            assert_eq!(t.due.as_deref(), Some("2026-12-01T09:00:00"));
            assert_eq!(t.priority, Some(3));
            assert_eq!(
                t.sequence,
                Some(1),
                "a new due is a scheduling change: the sequence moves on"
            );
            // the card stays in its column, at its place; the other task is untouched
            assert_eq!(titles(&source).await, ["todo/A renamed", "todo/b"]);
            assert_ne!(before, titles(&source).await);
            assert_eq!(task(&repo, &uids[1]).await.title.as_deref(), Some("b"));

            // one field alone: the rest stays
            call(&source, "task.edit", &uids[0], &[("priority", "5")])
                .await
                .unwrap();
            let t = task(&repo, &uids[0]).await;
            assert_eq!(
                (t.priority, t.title.as_deref(), t.description.as_deref()),
                (Some(5), Some("A renamed"), Some("details"))
            );
            assert_eq!(t.due.as_deref(), Some("2026-12-01T09:00:00"));

            // an empty text removes an optional field, and only that one
            call(
                &source,
                "task.edit",
                &uids[0],
                &[("description", ""), ("due", "")],
            )
            .await
            .unwrap();
            let t = task(&repo, &uids[0]).await;
            assert_eq!((t.description, t.due), (None, None));
            assert_eq!(
                (t.priority, t.title.as_deref()),
                (Some(5), Some("A renamed"))
            );
            call(&source, "task.edit", &uids[0], &[("priority", " ")])
                .await
                .unwrap();
            assert_eq!(task(&repo, &uids[0]).await.priority, None);
        })
        .await;
}

/// Refused: the task is as it was, each with its own kind.
#[tokio::test(flavor = "current_thread")]
async fn wrong_input_is_refused_and_writes_nothing() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop, repo) = serve_repo(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let uids = board_with(&socket, &source.client, &["a"]).await;
            let source = source.client;
            let before = task(&repo, &uids[0]).await;
            let refused = [
                (uids[0].clone(), vec![], BAD),
                (uids[0].clone(), vec![("title", "")], BAD),
                (uids[0].clone(), vec![("title", "   ")], BAD),
                (uids[0].clone(), vec![("priority", "high")], BAD),
                (uids[0].clone(), vec![("colour", "red")], BAD),
                (uids[0].clone(), vec![("title", "x"), ("title", "y")], BAD),
                (
                    uids[0].clone(),
                    vec![("priority", "10")],
                    "form-source-rejected",
                ),
                (
                    uids[0].clone(),
                    vec![("priority", "-1")],
                    "form-source-rejected",
                ),
                (
                    uids[0].clone(),
                    vec![("due", "tomorrow")],
                    "form-source-rejected",
                ),
                (
                    "no-such-uid".to_owned(),
                    vec![("title", "x")],
                    "form-source-unknown-record",
                ),
                ("u".repeat(300), vec![("title", "x")], BAD),
                (String::new(), vec![("title", "x")], BAD),
            ];
            for (uid, args, kind) in &refused {
                let (got, why) = call(&source, "task.edit", uid, args).await.unwrap_err();
                assert_eq!(&got, kind, "{args:?}: {why}");
            }
            // one wrong value among right ones: nothing of it is written
            let (kind, _) = call(
                &source,
                "task.edit",
                &uids[0],
                &[("title", "New"), ("priority", "99")],
            )
            .await
            .unwrap_err();
            assert_eq!(kind, "form-source-rejected");
            assert_eq!(
                task(&repo, &uids[0]).await,
                before,
                "a refused call changed the task"
            );
        })
        .await;
}

/// The first task of an empty database is made with `task.create` and then edited; and the editing of a task that is gone is told.
#[tokio::test(flavor = "current_thread")]
async fn the_first_task_of_an_empty_database_can_be_made_and_edited() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop, repo) = serve_repo(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            board_with(&socket, &source.client, &[]).await;
            let source = source.client;
            assert_eq!(
                repo.list().await.unwrap().len(),
                0,
                "the database starts empty"
            );
            let made = invoke(
                &source,
                "task.create",
                "",
                &[("title", "First"), ("column", "todo")],
            )
            .await;
            assert!(!made.failed, "{made:?}");
            let edited = invoke(
                &source,
                "task.edit",
                &made.record,
                &[("description", "now with a note")],
            )
            .await;
            assert!(!edited.failed, "{edited:?}");
            let t = task(&repo, &made.record).await;
            assert_eq!(
                (t.title.as_deref(), t.description.as_deref()),
                (Some("First"), Some("now with a note"))
            );
            let gone = invoke(&source, "task.delete", &made.record, &[]).await;
            assert!(!gone.failed);
            let (kind, _) = call(&source, "task.edit", &made.record, &[("title", "x")])
                .await
                .unwrap_err();
            assert_eq!(kind, "form-source-unknown-record");
        })
        .await;
}

/// A connection for which the service knows no person is refused `task.edit` (403) and the task stays as it was.
#[tokio::test(flavor = "current_thread")]
async fn a_stranger_may_not_edit() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop, repo) = serve_for(dir.path(), false).await;
            // a task made behind the service, as the owner would have
            let made = repo.create(fs_tasks_core::Task::new("Mine")).await.unwrap();
            let uid = made.task.uid.clone();
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let (kind, _) = call(&source.client, "task.edit", &uid, &[("title", "Hacked")])
                .await
                .unwrap_err();
            assert_eq!(kind, "form-source-forbidden");
            assert_eq!(task(&repo, &uid).await.title.as_deref(), Some("Mine"));
        })
        .await;
}
