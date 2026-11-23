//! The command catalog file of the module: command id to contract call, and the duties of the host towards the file.

use fs_modules_planning_client::Catalog;

#[test]
fn the_shipped_catalog_maps_the_commands_to_the_calls_of_the_decision() {
    let catalog = Catalog::bundled();
    let call = |id: &str| {
        catalog
            .command(id)
            .map(|c| (c.call.as_str(), c.capability.as_str()))
    };
    assert_eq!(
        call("planning.create"),
        Some(("Items.create", "planning.task.create"))
    );
    assert_eq!(
        call("planning.move"),
        Some(("Boards.moveCard", "planning.card.move"))
    );
    assert_eq!(
        call("planning.column.create"),
        Some(("Boards.editColumns", "planning.column.create"))
    );
    assert_eq!(
        call("planning.delete"),
        Some(("Items.delete", "planning.task.delete"))
    );
    // claim, release and note are NOT in the file: a command without a service behind it would be a lie
    assert_eq!(catalog.ids().count(), 4);
    for id in [
        "planning.claim",
        "planning.release",
        "planning.note",
        "claim",
    ] {
        assert!(catalog.command(id).is_none(), "{id}");
    }
}

#[test]
fn a_broken_catalog_is_refused_with_the_reason() {
    let twice = r#"{"commands":[{"id":"a","call":"X.y","capability":"c"},{"id":"a","call":"X.z","capability":"c"}]}"#;
    assert!(Catalog::parse(twice).unwrap_err().contains("twice"));
    let no_call = r#"{"commands":[{"id":"a","capability":"c"}]}"#;
    assert!(Catalog::parse(no_call).unwrap_err().contains("call"));
    assert!(Catalog::parse("{}").is_err());
    assert!(Catalog::parse("not json").is_err());
}

/// The duties the host (fs-starter v0.0.5, `registry/read.rs` + `registry/names.rs`) puts on the file; a stand-in until the one
/// conformance test of every module exists (card "fs-conformance" in the report).
#[test]
fn the_file_keeps_the_duties_of_the_host() {
    let text = include_str!("../../../catalog/commands.json");
    let doc: serde_json::Value = serde_json::from_str(text).unwrap();
    let module = fs_modules_planning_client::SOCKET_PROGRAM;
    assert_eq!(doc["module"], module);
    assert_eq!(doc["contract"], doc["module"]);
    assert_eq!(doc["scope"], "own-contract");
    assert_eq!(doc["version"], "0.0.1");
    let word = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
    };
    let commands = doc["commands"].as_array().unwrap();
    assert!(commands.len() >= 4);
    let space = format!("{module}.");
    for c in commands {
        let id = c["id"].as_str().unwrap();
        assert!(id.starts_with(&space), "{id}");
        assert!(
            id.split('.').count() >= 2 && id.split('.').all(word),
            "{id}"
        );
        assert!(
            c["capability"].as_str().unwrap().starts_with(&space),
            "{id}"
        );
        assert!(
            ["read", "write", "delete"].contains(&c["access"].as_str().unwrap()),
            "{id}"
        );
        for key in [
            &c["help"]["topic"],
            &c["help"]["text"]["en"],
            &c["help"]["text"]["de"],
        ] {
            assert!(!key.as_str().unwrap_or("").is_empty(), "{id}");
        }
    }
}
