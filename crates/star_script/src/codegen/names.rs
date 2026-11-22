//! Rust names for StarScript names.
//!
//! Every StarScript name is kept as written when possible, so the generated
//! code stays readable. Two rules make the translation safe:
//!
//! - Rust keywords become raw identifiers: `type` → `r#type`.
//! - A name ending in `_`, or one that Rust cannot use at all (`crate`,
//!   `Self`, `None`, …), gets an extra `_`: `count_` → `count__`.
//!
//! So the translation never gives two names the same Rust name, and the
//! generator's own names (`this_`, `object_`, `field0_`, …) end in a single
//! `_`, which only the names Rust cannot use (`crate_`, `None_`, …) also do:
//! they never collide.

/// Strict and reserved Rust keywords that raw identifiers can escape.
const RUST_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "static", "struct", "trait", "true", "try", "type", "typeof", "union", "unsafe", "unsized",
    "use", "virtual", "where", "while", "yield",
];

/// Names Rust cannot use as identifiers, or that mean something else in a
/// pattern such as `let None = …`.
const UNUSABLE_NAMES: &[&str] = &[
    "crate", "self", "Self", "super", "Some", "None", "Ok", "Err",
];

/// The Rust identifier for a StarScript name.
#[must_use]
pub(super) fn ident(name: &str) -> String {
    if name.ends_with('_') || UNUSABLE_NAMES.contains(&name) {
        format!("{name}_")
    } else if RUST_KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_names_are_kept() {
        assert_eq!(ident("speed"), "speed");
        assert_eq!(ident("Player"), "Player");
    }

    #[test]
    fn rust_keywords_become_raw_identifiers() {
        assert_eq!(ident("type"), "r#type");
        assert_eq!(ident("match"), "r#match");
    }

    #[test]
    fn names_rust_cannot_use_get_an_underscore() {
        assert_eq!(ident("crate"), "crate_");
        assert_eq!(ident("None"), "None_");
    }

    #[test]
    fn names_ending_in_underscore_cannot_collide() {
        assert_eq!(ident("this_"), "this__");
        assert_eq!(ident("crate_"), "crate__");
        assert_eq!(ident("_"), "__");
        assert_ne!(ident("crate"), ident("crate_"));
    }
}
