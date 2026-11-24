//! The hand order of the cards: a place per card (column + rank key), neighbours instead of a position number.

use std::collections::BTreeMap;

use fs_tasks_core::model::{Board, BoardTask};

fn tasks(uids: &[&str]) -> Vec<BoardTask> {
    uids.iter()
        .map(|u| BoardTask {
            uid: (*u).into(),
            progress: "needs-action".into(),
        })
        .collect()
}

fn todo(board: &Board, t: &[BoardTask]) -> Vec<String> {
    board.layout(t).columns[0].cards.clone()
}

#[test]
fn a_card_put_between_two_neighbours_writes_only_its_own_place() {
    let t = tasks(&["a", "b", "c"]);
    let mut board = Board::standard("x");
    board
        .place_card(&board.layout(&t), "c", "todo", None, Some("a"))
        .unwrap();
    assert_eq!(
        todo(&board, &t),
        ["c", "a", "b"],
        "unplaced cards after the placed one"
    );
    let before = board.place.clone();
    // c between a and b: a is pinned (it stands before the place), b keeps following its progress
    let mut board = Board::standard("x");
    board
        .place_card(&board.layout(&t), "c", "todo", Some("a"), Some("b"))
        .unwrap();
    assert_eq!(todo(&board, &t), ["a", "c", "b"]);
    assert_eq!(
        board.place.len(),
        2,
        "a is pinned, c is placed, b is not written"
    );
    let keys = board.place.clone();
    board
        .place_card(&board.layout(&t), "b", "todo", Some("c"), None)
        .unwrap();
    assert_eq!(todo(&board, &t), ["a", "c", "b"]);
    assert_eq!(board.place["a"], keys["a"], "no other card is rewritten");
    assert_eq!(board.place["c"], keys["c"]);
    assert_ne!(before, keys);
}

#[test]
fn after_is_the_anchor_when_the_neighbours_are_no_longer_neighbours() {
    let t = tasks(&["a", "b", "c", "d"]);
    let mut board = Board::standard("x");
    // after a, before d, but b and c stand between them now: right behind a
    board
        .place_card(&board.layout(&t), "d", "todo", Some("a"), Some("c"))
        .unwrap();
    assert_eq!(todo(&board, &t), ["a", "d", "b", "c"]);
    // a neighbour that is gone or in another column: before is the anchor, else the end
    board
        .place_card(&board.layout(&t), "a", "todo", Some("zz"), Some("c"))
        .unwrap();
    assert_eq!(todo(&board, &t), ["d", "b", "a", "c"]);
    board
        .place_card(&board.layout(&t), "d", "todo", Some("zz"), None)
        .unwrap();
    assert_eq!(todo(&board, &t).last().unwrap(), "d");
    assert!(
        board
            .place_card(&board.layout(&t), "a", "nowhere", None, None)
            .is_err()
    );
}

#[test]
fn the_index_counts_among_the_placed_cards_and_a_card_is_in_one_place_only() {
    let t = tasks(&["a", "b", "c"]);
    let mut board = Board::standard("x");
    board.move_card("a", "todo", None).unwrap();
    board.move_card("b", "todo", None).unwrap();
    board.move_card("c", "todo", Some(0)).unwrap();
    assert_eq!(todo(&board, &t), ["c", "a", "b"]);
    board.move_card("c", "doing", None).unwrap();
    assert_eq!(board.order()["doing"], ["c"]);
    assert_eq!(board.order()["todo"], ["a", "b"]);
}

#[test]
fn an_order_is_stored_as_places_and_written_back_unchanged() {
    let mut board = Board::standard("x");
    let order = BTreeMap::from([("todo".to_owned(), vec!["b".to_owned(), "a".to_owned()])]);
    board.set_order(&order).unwrap();
    assert_eq!(board.order(), order);
    let keys = board.place.clone();
    board.set_order(&order).unwrap();
    assert_eq!(board.place, keys, "the order read back does not renumber");
    let json = serde_json::to_value(&board).unwrap();
    assert!(json.get("order").is_none() && json["place"]["a"]["column"] == "todo");
}

#[test]
fn an_old_board_with_a_list_order_is_read_without_it() {
    let old = r#"{"name":"x","columns":[{"id":"todo","name":"To do"}],"order":{"todo":["a"]}}"#;
    let board: Board = serde_json::from_str(old).unwrap();
    assert!(board.place.is_empty() && board.problem().is_none());
}
