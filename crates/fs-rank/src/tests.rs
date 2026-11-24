use crate::{Error, Key};

fn text_sorted(keys: &[Key]) -> bool {
    keys.windows(2).all(|w| w[0].to_text() < w[1].to_text())
}

#[test]
fn between_before_after_keep_order() {
    let a = Key::first();
    let c = Key::after(&a);
    let b = Key::between(&a, &c).unwrap();
    let top = Key::before(&a);
    assert!(top < a && a < b && b < c);
    assert_eq!(Key::place(None, None).unwrap().to_text().is_empty(), false);
    assert!(Key::place(Some(&a), Some(&c)).unwrap() > a);
}

#[test]
fn many_inserts_at_the_same_place_stay_ordered() {
    let low = Key::first();
    let mut high = Key::after(&low);
    let mut keys = vec![low.clone(), high.clone()];
    for _ in 0..300 {
        high = Key::between(&low, &high).unwrap();
        keys.insert(1, high.clone());
    }
    assert!(keys.windows(2).all(|w| w[0] < w[1]));
    assert!(text_sorted(&keys));
}

#[test]
fn two_writers_with_the_same_neighbours_get_different_keys() {
    let a = Key::first();
    let b = Key::after(&a);
    let one = Key::between(&a, &b).unwrap();
    let two = Key::between(&a, &b).unwrap();
    assert_ne!(one, two);
    let end_one = Key::after(&b);
    let end_two = Key::after(&b);
    assert_ne!(end_one, end_two);
}

#[test]
fn text_sorts_like_the_keys() {
    // Mixed: random places in a growing list; the byte order of fracindex must equal the text order.
    let mut keys = vec![Key::first()];
    let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
    for _ in 0..500 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let at = (x % (keys.len() as u64 + 1)) as usize;
        let lower = at.checked_sub(1).map(|i| &keys[i]);
        let upper = keys.get(at);
        let k = Key::place(lower, upper).unwrap();
        keys.insert(at, k);
    }
    assert!(keys.windows(2).all(|w| w[0] < w[1]));
    assert!(text_sorted(&keys));
}

#[test]
fn text_roundtrip() {
    let a = Key::first();
    let k = Key::between(&a, &Key::after(&a)).unwrap();
    assert_eq!(Key::from_text(&k.to_text()).unwrap(), k);
    assert_eq!(Key::from_text(&a.to_text()).unwrap(), a);
}

#[test]
fn bad_text_is_an_error() {
    for t in ["", "abc", "zz", "AB"] {
        assert!(matches!(Key::from_text(t), Err(Error::BadText(_))), "{t}");
    }
}

#[test]
fn lower_not_smaller_than_upper_is_an_error() {
    let a = Key::first();
    let b = Key::after(&a);
    assert_eq!(Key::between(&b, &a), Err(Error::NotOrdered));
    assert_eq!(Key::between(&a, &a), Err(Error::NotOrdered));
}
