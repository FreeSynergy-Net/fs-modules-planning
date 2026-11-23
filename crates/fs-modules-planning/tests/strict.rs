//! `FormSource` strictness over a real service, driven by the file `catalog/commands.json` itself: every command is served, every
//! `params` entry is taken, and an unknown, a doubled or a missing argument is refused with its reason and changes nothing. Also
//! `may` and the paging by cursor.
#![cfg(unix)]

mod common;

use common::{board_with, call, page, serve, titles};
use fs_interfaces::formsource_capnp::form_source;
use fs_modules_planning::FORM_CARDS;
use tokio::task::LocalSet;

#[test]
fn the_form_names_at_most_four_facts_and_the_capabilities_are_the_contract_ids() {
    let forms: serde_json::Value =
        serde_json::from_str(include_str!("../../../catalog/forms.json")).unwrap();
    // cards-in-columns@1.0 allows facts 0..4 (fs-interfaces schema/forms.txt)
    assert!(forms[0]["map"]["facts"].as_array().unwrap().len() <= 4);
    // the channel of the form names the key of the contract, not a name of the host
    assert_eq!(forms[0]["map"]["move"], "card.move");
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

/// Arguments that make `key` succeed on the board `work` (`record` is the card to move or delete, empty for a create).
fn valid(key: &str) -> Vec<(&'static str, &'static str)> {
    match key {
        "card.move" => vec![("board", "work"), ("column", "doing"), ("index", "0")],
        "task.create" => vec![
            ("title", "Probe"),
            ("board", "work"),
            ("column", "doing"),
            ("index", "0"),
        ],
        _ => vec![],
    }
}

fn record_of(key: &str, uid: &str) -> String {
    if key == "task.create" {
        String::new()
    } else {
        uid.to_owned()
    }
}

#[tokio::test(flavor = "current_thread")]
async fn every_command_of_the_file_is_served_and_its_arguments_are_strict() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let uids = board_with(&socket, &source.client, &["one", "two", "three"]).await;
            let source = source.client;
            let commands: serde_json::Value =
                serde_json::from_str(include_str!("../../../catalog/commands.json")).unwrap();
            let bad = "form-source-bad-arguments";
            for c in commands["commands"].as_array().unwrap() {
                let key = c["capability"].as_str().unwrap();
                let rec = record_of(key, &uids[0]);
                // the file and the service say the same: the arguments of `valid` are exactly the params of the file
                let params: Vec<&str> = c["params"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p["path"].as_str().unwrap())
                    .collect();
                let required: Vec<&str> = c["params"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|p| p["required"] == true)
                    .map(|p| p["path"].as_str().unwrap())
                    .collect();
                let base = valid(key);
                assert!(
                    required.iter().all(|r| base.iter().any(|(k, _)| k == r)),
                    "{key}"
                );
                assert!(base.iter().all(|(k, _)| params.contains(k)), "{key}");
                assert!(
                    params.iter().all(|p| base.iter().any(|(k, _)| k == p)),
                    "{key}: a param no test runs"
                );
                let before = titles(&source).await;

                // an argument that the file does not name: refused with the word of the reason, nothing happens
                let mut extra = base.clone();
                extra.push(("colour", "red"));
                let (kind, why) = call(&source, key, &rec, &extra).await.unwrap_err();
                assert_eq!(kind, bad, "{key}");
                assert!(why.contains("unknown argument"), "{key}: {why}");
                // a doubled argument: the first no longer wins silently
                if let Some(first) = base.first() {
                    let mut twice = base.clone();
                    twice.push((first.0, "other"));
                    let (kind, why) = call(&source, key, &rec, &twice).await.unwrap_err();
                    assert_eq!(kind, bad, "{key}");
                    assert!(why.contains("twice"), "{key}: {why}");
                }
                // a required argument that is missing
                for gone in &required {
                    let less: Vec<_> = base.iter().copied().filter(|(k, _)| k != gone).collect();
                    let (kind, why) = call(&source, key, &rec, &less).await.unwrap_err();
                    assert_eq!(kind, bad, "{key} without {gone}");
                    assert!(why.contains("missing"), "{key}: {why}");
                }
                assert_eq!(
                    titles(&source).await,
                    before,
                    "{key}: a refused call changed something"
                );

                // and with every argument of the file it runs (an optional one dropped too)
                call(&source, key, &rec, &base)
                    .await
                    .unwrap_or_else(|e| panic!("{key}: {e:?}"));
                let without_optional: Vec<_> = base
                    .iter()
                    .copied()
                    .filter(|(k, _)| required.contains(k))
                    .collect();
                if key != "task.delete" {
                    call(&source, key, &rec, &without_optional)
                        .await
                        .unwrap_or_else(|e| panic!("{key}: {e:?}"));
                }
            }
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_page_says_what_may_be_done_and_pages_by_cursor() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let _uids = board_with(&socket, &source.client, &["one", "two", "three"]).await;
            let source = source.client;
            // pages by cursor: one record each, in the order of the layout, until the cursor is empty
            let mut seen = Vec::new();
            let mut cursor = Vec::new();
            loop {
                let mut request = source.page_request();
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
                // what the provider supports: on the set, a new card; on a record, move and delete; keys of the contract
                let on_set: Vec<_> = page
                    .get_may()
                    .unwrap()
                    .iter()
                    .map(|k| k.unwrap().to_string().unwrap())
                    .collect();
                assert_eq!(on_set, ["task.create"]);
                for r in page.get_records().unwrap() {
                    let may: Vec<_> = r
                        .get_may()
                        .unwrap()
                        .iter()
                        .map(|k| k.unwrap().to_string().unwrap())
                        .collect();
                    assert_eq!(may, ["card.move", "task.delete"]);
                    seen.push(r.get_id().unwrap().to_string().unwrap());
                    assert!(seen.len() <= 3, "the cursor never ends");
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
            let all = page(&source, FORM_CARDS, "work").await.unwrap();
            assert_eq!(all.iter().map(|r| r.0.clone()).collect::<Vec<_>>(), seen);
        })
        .await;
}
