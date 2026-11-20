//! The command catalog file of the module: command id to contract call.

use fs_modules_planning_client::Catalog;

#[test]
fn the_shipped_catalog_maps_the_commands_to_the_calls_of_the_decision() {
    let catalog = Catalog::bundled();
    let call = |id: &str| catalog.command(id).map(|c| (c.call.as_str(), c.capability.as_str()));
    assert_eq!(call("create"), Some(("Items.create", "task.create")));
    assert_eq!(call("move"), Some(("Boards.moveCard", "card.move")));
    assert_eq!(call("column.create"), Some(("Boards.editColumns", "column.create")));
    assert_eq!(call("delete"), Some(("Items.delete", "task.delete")));
    assert_eq!(catalog.ids().count(), 4, "claim, release, note come later");
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
