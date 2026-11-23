//! Finding a piece of text in a script and replacing it. Places are
//! character indices, as the text edit's cursors count them.

use std::ops::Range;

/// What is looked for in a text.
#[derive(Debug, Clone, Copy)]
pub(super) struct Query<'query> {
    pub(super) text: &'query str,
    /// Whether upper and lower case letters are told apart.
    pub(super) match_case: bool,
}

impl Query<'_> {
    /// Where the query is in `text`, in order and without overlapping.
    /// Nothing for an empty query.
    #[must_use]
    pub(super) fn matches_in(self, text: &str) -> Vec<Range<usize>> {
        let wanted: Vec<char> = self.text.chars().collect();
        if wanted.is_empty() {
            return Vec::new();
        }
        let characters: Vec<char> = text.chars().collect();
        let mut found = Vec::new();
        let mut start = 0;
        while start + wanted.len() <= characters.len() {
            let candidate = &characters[start..start + wanted.len()];
            if self.same_characters(candidate, &wanted) {
                found.push(start..start + wanted.len());
                start += wanted.len();
            } else {
                start += 1;
            }
        }
        found
    }

    fn same_characters(self, found: &[char], wanted: &[char]) -> bool {
        found.iter().zip(wanted).all(|(found, wanted)| {
            found == wanted || (!self.match_case && found.to_lowercase().eq(wanted.to_lowercase()))
        })
    }
}

/// Writes `replacement` in place of the characters `range` of `text`.
pub(super) fn replace_range(text: &mut String, range: Range<usize>, replacement: &str) {
    let start = byte_of(text, range.start);
    let end = byte_of(text, range.end);
    text.replace_range(start..end, replacement);
}

/// Writes `replacement` in place of every match of `query` in `text`.
/// Returns how many were replaced.
pub(super) fn replace_all(text: &mut String, query: Query<'_>, replacement: &str) -> usize {
    let matches = query.matches_in(text);
    // From the last one, so the places of the others stay where they were.
    for range in matches.iter().rev() {
        replace_range(text, range.clone(), replacement);
    }
    matches.len()
}

/// The place, among `matches`, of the first one that starts at the
/// character `cursor` or after it; the first one if none does.
#[must_use]
pub(super) fn first_from(matches: &[Range<usize>], cursor: usize) -> usize {
    matches
        .iter()
        .position(|range| range.start >= cursor)
        .unwrap_or(0)
}

/// The byte where the character at `index` of `text` starts; the end of
/// the text past its last character.
fn byte_of(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(byte, _)| byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(text: &str) -> Query<'_> {
        Query {
            text,
            match_case: false,
        }
    }

    fn exact(text: &str) -> Query<'_> {
        Query {
            text,
            match_case: true,
        }
    }

    #[test]
    fn matches_are_character_ranges() {
        assert_eq!(query("ño").matches_in("año niño"), vec![1..3, 6..8]);
    }

    #[test]
    fn empty_queries_match_nothing() {
        assert_eq!(query("").matches_in("anything"), Vec::<Range<usize>>::new());
    }

    #[test]
    fn matches_do_not_overlap() {
        assert_eq!(query("aa").matches_in("aaaaa"), vec![0..2, 2..4]);
    }

    #[test]
    fn case_is_ignored_unless_asked() {
        let text = "Rock rock ROCK";

        assert_eq!(query("rock").matches_in(text).len(), 3);
        assert_eq!(exact("rock").matches_in(text), vec![5..9]);
    }

    #[test]
    fn letters_with_accents_ignore_case_too() {
        assert_eq!(query("ÁRBOL").matches_in("un árbol"), vec![3..8]);
    }

    #[test]
    fn a_range_is_replaced_by_characters() {
        let mut text = "año = 1".to_owned();

        replace_range(&mut text, 0..3, "year");

        assert_eq!(text, "year = 1");
    }

    #[test]
    fn every_match_is_replaced() {
        let mut text = "var size = size + Size".to_owned();

        let replaced = replace_all(&mut text, query("size"), "scale");

        assert_eq!(replaced, 3);
        assert_eq!(text, "var scale = scale + scale");
    }

    #[test]
    fn replacements_may_contain_the_query() {
        let mut text = "a a".to_owned();

        replace_all(&mut text, exact("a"), "aa");

        assert_eq!(text, "aa aa");
    }

    #[test]
    fn the_first_match_from_the_cursor_is_chosen() {
        let matches = [0..2, 5..7, 9..11];

        assert_eq!(first_from(&matches, 0), 0);
        assert_eq!(first_from(&matches, 3), 1);
        assert_eq!(first_from(&matches, 10), 0);
    }
}
