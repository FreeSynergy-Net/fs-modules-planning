//! `rows@1.0.0` of the planning module: the lanes of a swim-lane view (type `Lane`, scope `lanes:<axis>[:<board>]`, also the empty
//! ones stored at the board) and the tasks as rows with typed columns (type `Task`, the classic list).
//!
//! Pattern: Projection (the module lists the lanes and the values of the standard; the frontend groups and names them).
#![cfg(unix)]

mod common;

use common::{field, may_of, page, serve_repo};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use fs_tasks_core::model::ActorType;
use fs_tasks_core::{Boards, Task};
use tokio::task::LocalSet;

const ROWS: &str = "rows@1.0.0";
const ADA: &str = "freesynergy.net:person:ada";
const BOB: &str = "freesynergy.net:person:bob";
const ZED: &str = "freesynergy.net:person:zed";

async fn ids(source: &form_source::Client, scope: &str) -> Vec<String> {
    let rows = page(source, ROWS, scope).await.unwrap();
    rows.iter().map(|r| r.0.clone()).collect()
}

#[allow(clippy::too_many_lines)] // one board with tasks, then every axis read once: the setup is shared on purpose
#[tokio::test(flavor = "current_thread")]
async fn lanes_come_from_the_axis_and_the_stored_ones_come_first() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(Box::pin(async {
            let (socket, _stop, repo) = serve_repo(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let tasks = fs_rpc::connect::<tasks::Client>(&socket).await.unwrap();
            let boards = tasks.client.boards_request().send().pipeline.get_boards();
            for name in ["work", "home"] {
                let mut create = boards.create_request();
                create.get().set_name(name);
                create.send().promise.await.unwrap();
            }
            let items = tasks.client.items_request().send().pipeline.get_items();
            let mut one = Task::new("One");
            one.assign_actor(ADA, ActorType::Person, "Ada");
            let mut both = Task::new("Both");
            both.assign_actor(ADA, ActorType::Person, "Ada");
            both.assign_actor(BOB, ActorType::Person, "Bob");
            let mut failed = Task::new("Failed");
            failed.progress = Some("failed".into());
            for task in [one, both, failed, Task::new("Nobody")] {
                let mut add = items.create_request();
                add.get()
                    .set_jscalendar(task.to_value().to_string().as_str());
                add.send().promise.await.unwrap();
            }
            // an empty lane of the board: what the contract cannot write, so it is stored through the repository
            let entry = repo
                .boards()
                .await
                .unwrap()
                .into_iter()
                .find(|b| b.board.name == "work")
                .unwrap();
            let mut board = entry.board.clone();
            board.lanes.insert("assignee".into(), vec![ZED.into()]);
            repo.board_edit(entry.id, entry.revision(), board)
                .await
                .unwrap();

            // status: the five words of the RFC in order
            let five = [
                "needs-action",
                "in-process",
                "completed",
                "failed",
                "cancelled",
            ];
            assert_eq!(ids(&source.client, "lanes:state").await, five);
            // people: the stored lane first (empty), then everyone in charge of a task of the board, each once
            assert_eq!(
                ids(&source.client, "lanes:assignee:work").await,
                [ZED, ADA, BOB]
            );
            // no board named = every board: the stored lane of `work` is still there
            assert_eq!(ids(&source.client, "lanes:assignee").await, [ZED, ADA, BOB]);
            // boards: all, or the one named
            assert_eq!(ids(&source.client, "lanes:lane").await.len(), 2);
            assert_eq!(ids(&source.client, "lanes:lane:home").await, ["home"]);
            // the record: name, place and axis
            let rows = page(&source.client, ROWS, "lanes:assignee:work")
                .await
                .unwrap();
            assert_eq!(field(&rows[0], "label"), ZED);
            assert_eq!(field(&rows[2], "order"), "2");
            assert_eq!(field(&rows[2], "axis"), "assignee");
            // a lane is not a card: nothing may be done at a lane record or at the list of lanes
            for scope in ["lanes:state", "lanes:assignee:work", "lanes:lane"] {
                let (set, records) = may_of(&source.client, ROWS, scope).await;
                assert_eq!(set, Vec::<String>::new(), "{scope} set");
                assert_ne!(records.len(), 0, "{scope}");
                assert!(records.iter().all(|(_, m)| m.is_empty()), "{scope}");
            }
            // the lanes stored at the board for the axes `state` and `lane` come first
            let entry = repo
                .boards()
                .await
                .unwrap()
                .into_iter()
                .find(|b| b.board.name == "work")
                .unwrap();
            let mut board = entry.board.clone();
            board
                .lanes
                .insert("state".into(), vec!["acme.example:waiting".into()]);
            board.lanes.insert("lane".into(), vec!["archive".into()]);
            repo.board_edit(entry.id, entry.revision(), board)
                .await
                .unwrap();
            assert_eq!(
                ids(&source.client, "lanes:state:work").await[..2],
                ["acme.example:waiting", "needs-action"]
            );
            assert_eq!(
                ids(&source.client, "lanes:lane:work").await,
                ["archive", "work"]
            );
            // an axis that does not exist, and a form that is not served
            assert!(page(&source.client, ROWS, "lanes:colour").await.is_err());
            assert!(
                page(&source.client, ROWS, "lanes:assignee:nowhere")
                    .await
                    .is_err()
            );
            assert!(
                page(&source.client, "rows@2.0.0", "lanes:state")
                    .await
                    .is_err()
            );
        }))
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_task_is_a_row_with_the_standard_columns() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(Box::pin(async {
            let (socket, _stop, _repo) = serve_repo(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let tasks = fs_rpc::connect::<tasks::Client>(&socket).await.unwrap();
            let boards = tasks.client.boards_request().send().pipeline.get_boards();
            let mut create = boards.create_request();
            create.get().set_name("work");
            create.send().promise.await.unwrap();
            let items = tasks.client.items_request().send().pipeline.get_items();

            let mut full = Task::new("Full");
            full.due = Some("2026-11-26T09:00:00".into());
            full.time_zone = Some("Europe/Berlin".into());
            // a recurrence needs a start (JSCalendar 2.0)
            full.start = Some("2026-11-26T08:00:00".into());
            full.priority = Some(2);
            full.percent_complete = Some(40);
            full.estimated_duration = Some("PT2H30M".into());
            full.privacy = Some("private".into());
            full.set_task_status("PENDING");
            full.recurrence_rule = Some(
                serde_json::from_value(serde_json::json!({
                    "frequency": "monthly", "interval": 2, "count": 3,
                    "byDay": [{"day": "mo"}, {"day": "fr", "nthOfPeriod": 2}]
                }))
                .unwrap(),
            );
            full.related_to.insert(
                "urn:uuid:a".into(),
                serde_json::json!({"relation": {"parent": true}}),
            );
            full.related_to.insert(
                "urn:uuid:b".into(),
                serde_json::json!({"relation": {"depends-on": true}}),
            );
            full.assign_actor(ADA, ActorType::Person, "Ada");
            for task in [full, Task::new("Bare")] {
                let mut add = items.create_request();
                add.get()
                    .set_jscalendar(task.to_value().to_string().as_str());
                add.send().promise.await.unwrap();
            }

            let mut odd = Task::new("Odd");
            odd.set_task_status("MAYBE");
            let mut add = items.create_request();
            add.get()
                .set_jscalendar(odd.to_value().to_string().as_str());
            add.send().promise.await.unwrap();

            // the tasks of the list may be moved and deleted, and the list may get a new task
            let (set, records) = may_of(&source.client, ROWS, "work").await;
            assert!(set.iter().any(|k| k.contains("create")), "{set:?}");
            assert!(records.iter().all(|(_, m)| !m.is_empty()));

            let rows = page(&source.client, ROWS, "work").await.unwrap();
            let all = |title: &str, path: &str| -> Vec<String> {
                let row = rows.iter().find(|r| field(r, "title") == title).unwrap();
                row.2
                    .iter()
                    .filter(|(p, _)| p == path)
                    .map(|(_, v)| v.clone())
                    .collect()
            };
            assert_eq!(all("Full", "due"), ["2026-11-26T08:00:00Z"]);
            assert_eq!(all("Full", "timeZone"), ["Europe/Berlin"]);
            assert_eq!(all("Full", "priority"), ["2"]);
            assert_eq!(all("Full", "percentComplete"), ["40"]);
            assert_eq!(all("Full", "estimatedDuration"), ["PT2H30M"]);
            assert_eq!(all("Full", "progress"), ["needs-action"]);
            assert_eq!(all("Full", "taskStatus"), ["PENDING"]);
            assert_eq!(all("Full", "privacy"), ["private"]);
            assert_eq!(all("Full", "participants"), ["Ada"]);
            // the contract has no `repeats: relatedTo` for rows yet: at most the first related task is sent
            assert_eq!(all("Full", "relatedTo"), ["urn:uuid:a"]);
            // the rule is not sent yet (no conversion wire form -> typed rule in fs-jscalendar): nothing is written or guessed
            assert_eq!(all("Full", "recurrenceRule").len(), 0);
            // a status word that is not one of the six of RFC 10051 is not sent
            assert_eq!(all("Odd", "taskStatus").len(), 0);
            // what the type does not name stays out (a card's `column`, the `description` ...); a bare task has only what is always there
            assert_eq!(all("Full", "column").len(), 0);
            for path in [
                "due",
                "timeZone",
                "priority",
                "taskStatus",
                "recurrenceRule",
                "relatedTo",
                "privacy",
                "percentComplete",
            ] {
                assert_eq!(all("Bare", path).len(), 0, "{path}");
            }
            assert_eq!(all("Bare", "progress"), ["needs-action"]);
        }))
        .await;
}
