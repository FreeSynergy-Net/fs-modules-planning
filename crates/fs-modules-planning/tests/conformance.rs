//! The conformance suite of the contract `module_v1` (crate `fs-module` of fs-interfaces v0.1.0) against the real service
//! (`FormSourceImpl` behind `Planning`) in this process with a temporary store, and the case T21 (move by neighbours, never a number).
//!
//! Pattern: Template Method (the rules are the suite's; this file is the fixture: client, declared forms and commands from the
//! catalog files, valid calls). A rule the fixture cannot set up is skipped by the suite, never passed.
#![cfg(unix)]

mod common;

use common::{board_with, call, field, page, serve};
use fs_interfaces::formsource_capnp::form_source;
use fs_module::conformance::run_conformance;
use fs_module::{
    CommandKind, DeclaredCommand, DeclaredForm, FormSourceFixture, Param, Sample, Verify,
};
use tokio::task::LocalSet;

const FORM: &str = "cards-in-columns@1.2.0";

struct Planning {
    client: form_source::Client,
    uids: Vec<String>,
}

impl FormSourceFixture for Planning {
    fn client(&self) -> form_source::Client {
        self.client.clone()
    }

    fn provider(&self) -> Option<&str> {
        Some("planning")
    }

    fn forms(&self) -> Vec<DeclaredForm> {
        let forms: serde_json::Value =
            serde_json::from_str(include_str!("../../../catalog/forms.json")).unwrap();
        vec![DeclaredForm {
            key: FORM.into(),
            scope: "work".into(),
            fields: forms[0]["fields"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect(),
        }]
    }

    fn commands(&self) -> Vec<DeclaredCommand> {
        let file: serde_json::Value =
            serde_json::from_str(include_str!("../../../catalog/commands.json")).unwrap();
        file["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                let key = c["capability"].as_str().unwrap().to_owned();
                DeclaredCommand {
                    kind: if key == "task.create" {
                        CommandKind::Set
                    } else {
                        CommandKind::Record
                    },
                    params: c["params"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|p| Param {
                            name: p["path"].as_str().unwrap().to_owned(),
                            required: p["required"] == true,
                        })
                        .collect(),
                    key,
                }
            })
            .collect()
    }

    fn samples(&self) -> Vec<Sample> {
        let a = |k: &str, v: &str| (k.to_owned(), v.to_owned());
        let value = |path: &str, text: &str| Verify::Value {
            form: FORM.into(),
            record: None,
            path: path.into(),
            text: text.into(),
        };
        vec![
            Sample {
                command: "card.move".into(),
                record: self.uids[2].clone(),
                args: vec![
                    a("board", "work"),
                    a("column", "doing"),
                    a("after", ""),
                    a("before", ""),
                ],
                makes_record: true,
                verify: value("column", "doing"),
            },
            Sample {
                command: "task.create".into(),
                record: String::new(),
                args: vec![
                    a("title", "Made"),
                    a("board", "work"),
                    a("column", "todo"),
                    a("before", &self.uids[0]),
                ],
                makes_record: true,
                verify: value("title", "Made"),
            },
            Sample {
                command: "task.delete".into(),
                record: self.uids[1].clone(),
                args: vec![],
                makes_record: false,
                verify: Verify::Gone {
                    form: FORM.into(),
                    record: self.uids[1].clone(),
                },
            },
        ]
    }

    fn strict_scope(&self) -> bool {
        true
    }
}

#[tokio::test(flavor = "current_thread")]
async fn the_suite_of_module_v1_passes_on_the_planning_service() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let uids = board_with(&socket, &source.client, &["a", "b", "c"]).await;
            let fixture = Planning {
                client: source.client.clone(),
                uids,
            };
            let report = run_conformance(&fixture).await;
            assert!(report.failures.is_empty(), "{:#?}", report.failures);
            // only the rule about a locked record is skipped: the service has no rights per person (see `write.rs`)
            assert!(
                report.skipped.iter().all(|s| s.starts_with("11 forbidden")),
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
