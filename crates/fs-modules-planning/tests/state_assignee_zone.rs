//! The channels `state`, `assignee` and `time-zone` of `cards-in-columns@1.4.0` over a real service: the status word of the five (RFC 8984
//! 5.2.5), one value per person in charge (the value repeats, same path), and the zone beside `due`, which stays an instant with `Z`.
//!
//! Pattern: Projection (the module sends the superset in the values of the standard; grouping by state or person is the frontend's choice).
#![cfg(unix)]

mod common;

use common::{field, page, serve};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use fs_modules_planning::FORM_CARDS;
use fs_tasks_core::Task;
use fs_tasks_core::model::ActorType;
use tokio::task::LocalSet;

const FIVE: [&str; 5] = [
    "needs-action",
    "in-process",
    "completed",
    "failed",
    "cancelled",
];

#[tokio::test(flavor = "current_thread")]
async fn state_assignee_and_time_zone_follow_the_task() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let tasks = fs_rpc::connect::<tasks::Client>(&socket).await.unwrap();
            let boards = tasks.client.boards_request().send().pipeline.get_boards();
            let mut create = boards.create_request();
            create.get().set_name("work");
            create.send().promise.await.unwrap();
            let items = tasks.client.items_request().send().pipeline.get_items();

            let mut berlin = Task::new("Zoned");
            berlin.due = Some("2026-11-26T09:00:00".into());
            berlin.time_zone = Some("Europe/Berlin".into());
            berlin.progress = Some("in-process".into());
            let mut floating = Task::new("Floating");
            floating.due = Some("2026-11-26T09:00:00".into());
            let mut two = Task::new("Two");
            two.assign_actor("freesynergy.net:person:ada", ActorType::Person, "Ada");
            two.assign_actor("freesynergy.net:person:bob", ActorType::Person, "Bob");
            let mut tasks_in = vec![berlin, floating, two, Task::new("Nobody")];
            for (i, word) in FIVE.iter().enumerate() {
                let mut t = Task::new(format!("State {i}"));
                t.progress = Some((*word).into());
                tasks_in.push(t);
            }
            for task in tasks_in {
                let mut add = items.create_request();
                add.get()
                    .set_jscalendar(task.to_value().to_string().as_str());
                add.send().promise.await.unwrap();
            }

            let rows = page(&source.client, FORM_CARDS, "work").await.unwrap();
            let row = |title: &str| rows.iter().find(|r| field(r, "title") == title).unwrap();
            let all = |title: &str, path: &str| -> Vec<String> {
                row(title)
                    .2
                    .iter()
                    .filter(|(p, _)| p == path)
                    .map(|(_, v)| v.clone())
                    .collect()
            };
            // 09:00 in Berlin in November is 08:00 UTC, and the zone is named beside it
            assert_eq!(field(row("Zoned"), "due"), "2026-11-26T08:00:00Z");
            assert_eq!(all("Zoned", "timeZone"), ["Europe/Berlin"]);
            // no zone = floating: the instant in the floating zone, no value for the zone
            assert_eq!(field(row("Floating"), "due"), "2026-11-26T09:00:00Z");
            assert_eq!(all("Floating", "timeZone").len(), 0);
            // one value per person, same path; nobody = no value
            assert_eq!(
                all("Two", "assignee"),
                ["freesynergy.net:person:ada", "freesynergy.net:person:bob"]
            );
            assert_eq!(all("Nobody", "assignee").len(), 0);
            // the state is one of the five words
            assert_eq!(all("Zoned", "progress"), ["in-process"]);
            assert_eq!(all("Nobody", "progress"), ["needs-action"]);
            for (i, word) in FIVE.iter().enumerate() {
                assert_eq!(all(&format!("State {i}"), "progress"), [*word]);
            }
        })
        .await;
}
