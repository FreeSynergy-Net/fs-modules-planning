//! The own channels of `cards-in-columns@1.4.0` over a real service: `description`, `color`, `estimatedDuration`, `priority` of a task (JSCalendar
//! RFC 8984) and `column-limit` of a column, each sent only when it is set, `priority` not as a fact.
//!
//! Pattern: Projection (the module sends the superset in the values of the standard; what is shown is the frontend's choice).
#![cfg(unix)]

mod common;

use common::{board_with, field, page, seed_channels, serve};
use fs_interfaces::formsource_capnp::form_source;
use fs_modules_planning::FORM_CARDS;
use tokio::task::LocalSet;

type Row = (String, String, Vec<(String, String)>);

fn has(row: &Row, name: &str) -> bool {
    row.2.iter().any(|(p, _)| p == name)
}

#[tokio::test(flavor = "current_thread")]
async fn each_channel_is_sent_when_set_and_left_out_when_not() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            // `plain` has none of the channels; `doing` stands in a column without a limit
            board_with(&socket, &source.client, &["plain"]).await;
            seed_channels(&socket).await;

            let rows = page(&source.client, FORM_CARDS, "work").await.unwrap();
            let of = |title: &str| rows.iter().find(|r| field(r, "title") == title).unwrap();
            let full = of("full");
            assert_eq!(field(full, "priority"), "1");
            assert_eq!(field(full, "color"), "#ff8800");
            assert_eq!(field(full, "estimatedDuration"), "PT1H30M");
            assert_eq!(field(full, "description"), "why");
            assert_eq!(field(full, "column-limit"), "3");
            let plain = of("plain");
            for name in [
                "priority",
                "color",
                "estimatedDuration",
                "description",
                "timeZone",
                "assignee",
            ] {
                assert!(!has(plain, name), "{name} is not set");
            }
            assert_eq!(
                field(plain, "column-limit"),
                "3",
                "every card of the column carries its limit"
            );
            for row in &rows {
                // `progress` is now the status word of the channel `state` (RFC 8984 5.2.5), never the percent
                assert_eq!(field(row, "progress"), "needs-action");
            }
        })
        .await;
}

#[test]
fn the_catalog_declares_the_channels_with_their_types() {
    let forms: serde_json::Value =
        serde_json::from_str(include_str!("../../../catalog/forms.json")).unwrap();
    let form = &forms[0];
    assert_eq!(form["version"], "1.4.0");
    for (channel, kind) in [
        ("description", "text"),
        ("color", "text"),
        ("priority", "number"),
        ("column-limit", "number"),
    ] {
        assert_eq!(form["map"][channel], channel, "{channel}");
        assert_eq!(form["fields"][channel], kind, "{channel}");
    }
    // the channels that carry the name of the standard in the module (card: standard names in the field map)
    for (channel, name, kind) in [
        ("estimated", "estimatedDuration", "text"),
        ("progress", "percentComplete", "number"),
        ("state", "progress", "text"),
        ("assignee", "assignee", "reference"),
        ("time-zone", "timeZone", "text"),
    ] {
        assert_eq!(form["map"][channel], name, "{channel}");
        assert_eq!(form["fields"][name], kind, "{name}");
    }
    let facts: Vec<&str> = form["map"]["facts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert!(
        !facts.contains(&"priority"),
        "priority is a channel of its own, no fact"
    );
}
