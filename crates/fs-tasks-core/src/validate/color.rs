//! The `color` member of a task (RFC 8984 section 4.2.11): a CSS3 color name (section 4.3) or an RGB value in hexadecimal notation.
//!
//! Pattern: Specification (one rule, one predicate) — the same rule as rule 22 of the conformance suite of fs-interfaces, checked
//! here by the shape itself and a fixed list of names, so it does not depend on how loosely a parser reads.

use super::css_names::NAMES;

/// True if `s` is `#` + 3 or 6 hex digits (section 4.2.1) or one of the 147 names (names compared in lower case, as in CSS).
///
/// Everything else is refused: functional forms, `transparent`, `rebeccapurple` (CSS4), hex without `#`, 8 digits, blanks.
pub(crate) fn is_valid(s: &str) -> bool {
    match s.strip_prefix('#') {
        Some(hex) => matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit()),
        None => NAMES.contains(&s.to_ascii_lowercase().as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::{NAMES, is_valid};
    /// The rule of rule 22 of the conformance suite (fs-interfaces e6f69213 `rules_values.rs`, `is_color`), mirrored: that code is `pub(crate)`.
    fn rule_22(t: &str) -> bool {
        match t.strip_prefix('#') {
            Some(hex) => matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit()),
            None => NAMES.contains(&t.to_ascii_lowercase().as_str()),
        }
    }

    /// Both rules answer every string the same: all 147 names (plain, upper, with a blank, with `#`), hex forms and the loose forms a parser reads.
    #[test]
    fn same_answer_as_rule_22_for_names_and_hex_forms() {
        let mut samples: Vec<String> = Vec::new();
        for n in NAMES {
            samples.push((*n).to_owned());
            samples.push(n.to_ascii_uppercase());
            samples.push(format!(" {n}"));
            samples.push(format!("#{n}"));
        }
        for h in [
            "",
            "#",
            "#f",
            "#ff",
            "#fff",
            "#FFF",
            "#ffff",
            "#fffff",
            "#ffffff",
            "#FfAa00",
            "#fffffff",
            "#ffaa00ff",
            "#ggg",
            "cafe",
            "bad",
            "fff",
            "facade",
            "DeadBeef",
            "ffaa00",
            "abc",
            "rebeccapurple",
            "transparent",
            "currentcolor",
            "rgb(1,2,3)",
        ] {
            samples.push(h.to_owned());
        }
        for i in 0..4096u32 {
            samples.push(format!("#{i:03x}"));
            samples.push(format!("{i:03x}"));
            samples.push(format!("#{:06x}", i * 4093));
        }
        assert_eq!(NAMES.len(), 147);
        for s in &samples {
            assert_eq!(is_valid(s), rule_22(s), "{s:?}");
        }
    }
}
