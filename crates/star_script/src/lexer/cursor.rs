//! Character-by-character reading of the source.

/// Reads the source one character at a time, tracking the byte offset.
pub(super) struct Cursor<'source> {
    source: &'source str,
    position: usize,
}

impl<'source> Cursor<'source> {
    pub(super) const fn new(source: &'source str) -> Self {
        Self {
            source,
            position: 0,
        }
    }

    /// Byte offset of the next character.
    pub(super) const fn position(&self) -> usize {
        self.position
    }

    pub(super) fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    pub(super) fn peek_second(&self) -> Option<char> {
        self.remaining().chars().nth(1)
    }

    pub(super) fn advance(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.position += character.len_utf8();
        Some(character)
    }

    /// Consumes the next character if it is `expected`.
    pub(super) fn eat(&mut self, expected: char) -> bool {
        let matches = self.peek() == Some(expected);
        if matches {
            self.advance();
        }
        matches
    }

    pub(super) fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while self.peek().is_some_and(&mut predicate) {
            self.advance();
        }
    }

    /// The code read since byte offset `start`.
    pub(super) fn text_since(&self, start: usize) -> &'source str {
        self.source.get(start..self.position).unwrap_or_default()
    }

    fn remaining(&self) -> &'source str {
        self.source.get(self.position..).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advancing_moves_by_whole_characters() {
        let mut cursor = Cursor::new("ñx");

        assert_eq!(cursor.advance(), Some('ñ'));
        assert_eq!(cursor.position(), 2);
        assert_eq!(cursor.advance(), Some('x'));
        assert_eq!(cursor.advance(), None);
    }

    #[test]
    fn eat_only_consumes_the_expected_character() {
        let mut cursor = Cursor::new("+=");

        assert!(!cursor.eat('='));
        assert!(cursor.eat('+'));
        assert!(cursor.eat('='));
    }

    #[test]
    fn text_since_returns_what_was_read() {
        let mut cursor = Cursor::new("speed = 2");

        cursor.eat_while(|character| character.is_ascii_alphabetic());

        assert_eq!(cursor.text_since(0), "speed");
        assert_eq!(cursor.peek_second(), Some('='));
    }
}
