//! The command catalog file of the module: command id to contract call.

use fs_modules_planning_client::Catalog;

#[test]
fn the_shipped_catalog_maps_the_commands_to_the_calls_of_the_decision() {
    let catalog = Catalog::bundled();
    let call = |id: &str| {
        catalog
            .command(id)
            .map(|c| (c.call.as_str(), c.capability.as_str()))
    };
    assert_eq!(call("create"), Some(("Items.create", "task.create")));
    assert_eq!(call("move"), Some(("Boards.moveCard", "card.move")));
    assert_eq!(
        call("column.create"),
        Some(("Boards.editColumns", "column.create"))
    );
    assert_eq!(call("delete"), Some(("Items.delete", "task.delete")));
    assert_eq!(catalog.ids().count(), 7);
    for id in ["claim", "release", "note"] {
        assert_eq!(call(id).map(|c| c.0), Some("FormSource.invoke"), "{id}");
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

#[test]
fn the_catalog_names_the_module_by_the_one_constant_and_carries_the_version() {
    let text = include_str!("../../../catalog/commands.json");
    let doc: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(doc["module"], fs_modules_planning_client::SOCKET_PROGRAM);
    assert_eq!(doc["version"], "0.0.1");
}
