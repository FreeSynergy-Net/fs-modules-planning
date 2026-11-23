//! `FormSource` strictness over a real service: every command of the catalog is served, paging by cursor, `index`, and the arguments
//! of `invoke` (unknown, doubled, no such task) are refused with their reason.
#![cfg(unix)]

mod common;

use common::{field, page, serve};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use fs_modules_planning::FORM_CARDS;
use fs_tasks_core::Task;
use tokio::task::LocalSet;

fn set_args(
    invoke: &mut capnp::capability::Request<
        form_source::invoke_params::Owned,
        form_source::invoke_results::Owned,
    >,
    command: &str,
    uid: &str,
    pairs: &[(&str, &str)],
) {
    let mut p = invoke.get();
    p.set_command(command);
    p.set_record(uid);
    let mut args = p.init_args(u32::try_from(pairs.len()).unwrap());
    for (i, (k, v)) in pairs.iter().enumerate() {
        let mut a = args.reborrow().get(u32::try_from(i).unwrap());
        a.set_path(k);
        a.set_text(v);
    }
}

/// `Ok(())` when the command ran, else the type of the problem.
async fn call(
    source: &form_source::Client,
    command: &str,
    uid: &str,
    pairs: &[(&str, &str)],
) -> Result<(), String> {
    let mut invoke = source.invoke_request();
    set_args(&mut invoke, command, uid, pairs);
    let answer = invoke.send().promise.await.unwrap();
    let answer = answer.get().unwrap();
    if !answer.get_failed() {
        return Ok(());
    }
    Err(answer
        .get_problem()
        .unwrap()
        .get_type()
        .unwrap()
        .to_string()
        .unwrap())
}

#[test]
fn the_form_names_at_most_four_facts_and_the_capabilities_are_the_contract_ids() {
    let forms: serde_json::Value =
        serde_json::from_str(include_str!("../../../catalog/forms.json")).unwrap();
    // cards-in-columns@1.0 allows facts 0..4 (fs-interfaces schema/forms.txt)
    assert!(forms[0]["map"]["facts"].as_array().unwrap().len() <= 4);
    for id in fs_modules_planning::CAPABILITIES {
        assert!(!id.starts_with("planning."), "{id}");
    }
    // the basic type words the host reader knows (fs-starter `sources/declared.rs` basic(), words of fs-forms-bind); a stand-in for
    // the host check until the one conformance test exists (card fs-conformance)
    let known = [
        "text",
        "email",
        "number",
        "bool",
        "date",
        "date-time",
        "choice",
        "reference",
        "keywords",
        "link",
        "image",
    ];
    for (name, kind) in forms[0]["fields"].as_object().unwrap() {
        assert!(known.contains(&kind.as_str().unwrap()), "{name}: {kind}");
    }
}

#[tokio::test(flavor = "current_thread")]
#[allow(clippy::too_many_lines)] // one service, one story: paging, index, strict arguments
async fn every_command_of_the_catalog_is_served_and_the_arguments_are_strict() {
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
            for title in ["one", "two", "three"] {
                let mut add = items.create_request();
                add.get()
                    .set_jscalendar(Task::new(title).to_value().to_string().as_str());
                add.send().promise.await.unwrap();
            }

            // a command of the file that the service does not serve is a lie: none may answer `unknown-command`
            let commands: serde_json::Value =
                serde_json::from_str(include_str!("../../../catalog/commands.json")).unwrap();
            for c in commands["commands"].as_array().unwrap() {
                let id = c["id"].as_str().unwrap();
                let got = call(&source.client, id, "x", &[]).await;
                assert_ne!(got, Err("form-source-unknown-command".to_owned()), "{id}");
            }

            // pages by cursor: one record each, in the order of the layout, until the cursor is empty
            let mut seen = Vec::new();
            let mut cursor = Vec::new();
            loop {
                let mut request = source.client.page_request();
                {
                    let mut p = request.get();
                    p.set_form(FORM_CARDS);
                    p.set_scope("work");
                    p.set_limit(1);
                    p.init_cursor().set_opaque(&cursor);
                }
                let answer = request.send().promise.await.unwrap();
                let page = answer.get().unwrap().get_page().unwrap();
                assert_eq!(page.get_total(), 3);
                for r in page.get_records().unwrap() {
                    assert_eq!(
                        r.get_may().unwrap().get(0).unwrap().to_str().unwrap(),
                        "planning.move"
                    );
                    seen.push(r.get_id().unwrap().to_string().unwrap());
                }
                cursor = if answer.get().unwrap().has_next() {
                    answer
                        .get()
                        .unwrap()
                        .get_next()
                        .unwrap()
                        .get_opaque()
                        .unwrap()
                        .to_vec()
                } else {
                    Vec::new()
                };
                if cursor.is_empty() {
                    break;
                }
            }
            assert_eq!(seen.len(), 3);
            let all = page(&source.client, FORM_CARDS, "work").await.unwrap();
            assert_eq!(all.iter().map(|r| r.0.clone()).collect::<Vec<_>>(), seen);

            // `index`: the third card goes to the front of its column
            let (first, last) = (seen[0].clone(), seen[2].clone());
            call(
                &source.client,
                "planning.move",
                &last,
                &[("board", "work"), ("column", "todo"), ("index", "0")],
            )
            .await
            .unwrap();
            let all = page(&source.client, FORM_CARDS, "work").await.unwrap();
            assert_eq!(all[0].0, last);
            assert_eq!(all[1].0, first);

            // strict arguments: an unknown one, a double one, a wrong uid are told, nothing is silently ignored
            let ok = [("board", "work"), ("column", "done")];
            let with = |extra: (&'static str, &'static str)| [ok[0], ok[1], extra];
            assert_eq!(
                call(
                    &source.client,
                    "planning.move",
                    &first,
                    &with(("colour", "red"))
                )
                .await,
                Err("form-source-unknown-argument".into())
            );
            assert_eq!(
                call(
                    &source.client,
                    "planning.move",
                    &first,
                    &with(("column", "todo"))
                )
                .await,
                Err("form-source-duplicate-argument".into())
            );
            assert_eq!(
                call(&source.client, "planning.move", "no-such-uid", &ok).await,
                Err("form-source-bad-arguments".into())
            );
            let long = "u".repeat(300);
            assert_eq!(
                call(&source.client, "planning.move", &long, &ok).await,
                Err("form-source-bad-arguments".into())
            );
            // none of the refused calls moved anything
            let all = page(&source.client, FORM_CARDS, "work").await.unwrap();
            assert!(all.iter().all(|r| field(r, "column") == "todo"));
        })
        .await;
}
