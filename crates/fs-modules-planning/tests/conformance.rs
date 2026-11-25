//! The conformance suite of the contract `module_v1`, level 2 (crate `fs-module` of fs-interfaces v0.2.0) against the real service
//! (`Guarded<PlanningCore>` behind `Planning`, over the socket) in this process with a temporary store, and the case T21 (move by neighbours, never a number).
//!
//! Pattern: Template Method (the rules are the suite's; the fixture is `common::suite::Planning`, the same one `guarded_core` uses).
//! A rule the fixture cannot set up is skipped by the suite, never passed.
#![cfg(unix)]

mod common;

use common::{board_with, call, field, page, seed_channels, serve};
use fs_interfaces::formsource_capnp::form_source;
use fs_module::conformance::run_conformance;
use tokio::task::LocalSet;

use common::suite::{FORM, Planning};

#[tokio::test(flavor = "current_thread")]
async fn the_suite_of_module_v1_passes_on_the_planning_service() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let other = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let uids = board_with(&socket, &source.client, &["a", "b", "c"]).await;
            seed_channels(&socket).await;
            let fixture = Planning {
                client: source.client.clone(),
                other: other.client.clone(),
                uids,
            };
            let report = run_conformance(&fixture).await;
            assert!(report.failures.is_empty(), "{:#?}", report.failures);
            // only the rules about a locked record (no rights per person, see `write.rs`), a blacked-out change and an image field (there is none) are skipped
            assert!(
                report.skipped.iter().all(|s| s.starts_with("11 forbidden")
                    || s.starts_with("L2-9 gone")
                    || s.starts_with("20 blob")),
                "{:#?}",
                report.skipped
            );
            assert!(report.checked > 30, "{}", report.checked);
        })
        .await;
}

/// T21: a card moved between two neighbours stands there, `order` (read only) counts 0, 1, 2, and a place by a number is refused.
#[tokio::test(flavor = "current_thread")]
async fn t21_move_orders_by_neighbours_and_refuses_a_number() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let uids = board_with(&socket, &source.client, &["A", "B", "C"]).await;
            let source = source.client;
            let to = [
                ("board", "work"),
                ("column", "todo"),
                ("after", &uids[0]),
                ("before", &uids[1]),
            ];
            call(&source, "card.move", &uids[2], &to).await.unwrap();
            let rows = page(&source, FORM, "work").await.unwrap();
            let seen: Vec<_> = rows
                .iter()
                .map(|r| (field(r, "title"), field(r, "index")))
                .collect();
            assert_eq!(seen, [("A", "0"), ("C", "1"), ("B", "2")]);
            for number in [("index", "1"), ("order", "1")] {
                let args = [("board", "work"), ("column", "todo"), number];
                let (kind, _) = call(&source, "card.move", &uids[2], &args)
                    .await
                    .unwrap_err();
                assert_eq!(kind, "form-source-bad-arguments", "{number:?}");
            }
            assert_eq!(
                page(&source, FORM, "work").await.unwrap(),
                rows,
                "a refused call changed something"
            );
        })
        .await;
}
