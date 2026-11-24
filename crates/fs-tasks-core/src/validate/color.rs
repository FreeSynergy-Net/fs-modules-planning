//! The `color` member of a task (RFC 8984 section 4.2.11): a CSS color name or an RGB value in hexadecimal notation.
//!
//! Pattern: Specification (one rule, one predicate) — the names come from the crate `csscolorparser`, nothing is listed here.

/// True if `s` is a CSS3 color name (section 4.3) or `#rgb` / `#rrggbb` (section 4.2.1).
///
/// Functional forms (`rgb(...)`, `hsl(...)`), `transparent` and a hex form without `#` are not named by the RFC and are refused,
/// although the parser would read them. Names are case-insensitive, as in CSS.
pub(crate) fn is_valid(s: &str) -> bool {
    if let Some(hex) = s.strip_prefix('#') {
        return matches!(hex.len(), 3 | 6) && hex.bytes().all(|b| b.is_ascii_hexdigit());
    }
    !s.is_empty()
        && s.bytes().all(|b| b.is_ascii_alphabetic())
        && !s.eq_ignore_ascii_case("transparent")
        && csscolorparser::parse(s).is_ok()
}
