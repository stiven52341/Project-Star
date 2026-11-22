//! First compiler stage: splits source text into [`Token`]s.
//!
//! The lexer reports every mistake it finds instead of stopping at the first
//! one, so the developer can fix them all at once.

mod cursor;
mod error;
mod number;
mod string;

pub use error::{LexError, LexErrorKind};

use crate::{Keyword, Span, Token, TokenKind};
use cursor::Cursor;

const BYTE_ORDER_MARK: char = '\u{FEFF}';

/// Splits `source` into tokens, ending with [`TokenKind::EndOfFile`].
///
/// # Errors
///
/// Returns every lexical mistake found, in source order.
pub fn tokenize(source: &str) -> Result<Vec<Token>, Vec<LexError>> {
    Lexer::new(source).run()
}

/// Brackets that are open at the current point of the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Delimiter {
    Paren,
    Bracket,
    Brace,
}

struct Lexer<'source> {
    cursor: Cursor<'source>,
    tokens: Vec<Token>,
    errors: Vec<LexError>,
    open_delimiters: Vec<Delimiter>,
}

impl<'source> Lexer<'source> {
    const fn new(source: &'source str) -> Self {
        Self {
            cursor: Cursor::new(source),
            tokens: Vec::new(),
            errors: Vec::new(),
            open_delimiters: Vec::new(),
        }
    }

    fn run(mut self) -> Result<Vec<Token>, Vec<LexError>> {
        self.cursor.eat(BYTE_ORDER_MARK);
        while let Some(character) = self.cursor.peek() {
            self.lex_next(character);
        }
        let end = self.cursor.position();
        self.push(TokenKind::EndOfFile, end);
        if self.errors.is_empty() {
            Ok(self.tokens)
        } else {
            Err(self.errors)
        }
    }

    fn lex_next(&mut self, character: char) {
        let start = self.cursor.position();
        match character {
            '\n' => {
                self.cursor.advance();
                self.push_newline(start);
            }
            '/' if self.cursor.peek_second() == Some('/') => self.skip_line_comment(),
            '/' if self.cursor.peek_second() == Some('*') => self.skip_block_comment(),
            '"' => self.lex_string(start),
            _ if character.is_whitespace() => {
                self.cursor.advance();
            }
            _ if character.is_ascii_digit() => self.lex_number(start),
            _ if is_identifier_start(character) => self.lex_word(start),
            _ => self.lex_symbol(start),
        }
    }

    fn skip_line_comment(&mut self) {
        self.cursor.eat_while(|character| character != '\n');
    }

    /// Skips a possibly nested `/* … */`. A comment spanning several lines
    /// still ends the statement before it, like the line break it contains.
    fn skip_block_comment(&mut self) {
        let start = self.cursor.position();
        self.cursor.advance();
        self.cursor.advance();
        let mut depth = 1_usize;
        let mut contains_newline = false;
        while depth > 0 {
            match self.cursor.advance() {
                None => {
                    let span = Span::new(start, self.cursor.position());
                    self.error(LexErrorKind::UnterminatedBlockComment, span);
                    return;
                }
                Some('/') if self.cursor.eat('*') => depth += 1,
                Some('*') if self.cursor.eat('/') => depth -= 1,
                Some('\n') => contains_newline = true,
                Some(_) => {}
            }
        }
        if contains_newline {
            self.push_newline(start);
        }
    }

    fn lex_string(&mut self, start: usize) {
        match string::lex_string(&mut self.cursor) {
            Ok(value) => self.push(TokenKind::String(value), start),
            Err(error) => self.errors.push(error),
        }
    }

    fn lex_number(&mut self, start: usize) {
        match number::lex_number(&mut self.cursor) {
            Ok(kind) => self.push(kind, start),
            Err(kind) => self.error(kind, Span::new(start, self.cursor.position())),
        }
    }

    fn lex_word(&mut self, start: usize) {
        self.cursor.eat_while(is_identifier_continue);
        let word = self.cursor.text_since(start);
        let kind = Keyword::from_word(word).map_or_else(
            || TokenKind::Identifier(word.to_owned()),
            TokenKind::Keyword,
        );
        self.push(kind, start);
    }

    fn lex_symbol(&mut self, start: usize) {
        let Some(character) = self.cursor.advance() else {
            return;
        };
        let Some(kind) = self.symbol_starting_with(character) else {
            let kind = if character.is_alphabetic() {
                LexErrorKind::NonAsciiIdentifier(character)
            } else {
                LexErrorKind::UnexpectedCharacter(character)
            };
            self.error(kind, Span::new(start, self.cursor.position()));
            return;
        };
        self.track_delimiter(&kind);
        self.push(kind, start);
    }

    /// The symbol whose first character was just read, consuming the rest of it.
    fn symbol_starting_with(&mut self, first: char) -> Option<TokenKind> {
        let kind = match first {
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            '.' => self.either('.', TokenKind::DotDot, TokenKind::Dot),
            '?' if self.cursor.eat('.') => TokenKind::QuestionDot,
            '?' => self.either('?', TokenKind::QuestionQuestion, TokenKind::Question),
            '-' if self.cursor.eat('>') => TokenKind::Arrow,
            '-' => self.either('=', TokenKind::MinusEqual, TokenKind::Minus),
            '+' => self.either('=', TokenKind::PlusEqual, TokenKind::Plus),
            '*' => self.either('=', TokenKind::StarEqual, TokenKind::Star),
            '/' => self.either('=', TokenKind::SlashEqual, TokenKind::Slash),
            '%' => self.either('=', TokenKind::PercentEqual, TokenKind::Percent),
            '=' => self.either('=', TokenKind::EqualEqual, TokenKind::Equal),
            '!' => self.either('=', TokenKind::BangEqual, TokenKind::Bang),
            '<' => self.either('=', TokenKind::LessEqual, TokenKind::Less),
            '>' => self.either('=', TokenKind::GreaterEqual, TokenKind::Greater),
            '&' if self.cursor.eat('&') => TokenKind::AndAnd,
            '|' if self.cursor.eat('|') => TokenKind::OrOr,
            _ => return None,
        };
        Some(kind)
    }

    /// `long` if the next character is `second` (consuming it), else `short`.
    fn either(&mut self, second: char, long: TokenKind, short: TokenKind) -> TokenKind {
        if self.cursor.eat(second) { long } else { short }
    }

    fn track_delimiter(&mut self, kind: &TokenKind) {
        let (delimiter, opens) = match kind {
            TokenKind::LeftParen => (Delimiter::Paren, true),
            TokenKind::RightParen => (Delimiter::Paren, false),
            TokenKind::LeftBracket => (Delimiter::Bracket, true),
            TokenKind::RightBracket => (Delimiter::Bracket, false),
            TokenKind::LeftBrace => (Delimiter::Brace, true),
            TokenKind::RightBrace => (Delimiter::Brace, false),
            _ => return,
        };
        if opens {
            self.open_delimiters.push(delimiter);
        } else if self.open_delimiters.last() == Some(&delimiter) {
            // A mismatched closing bracket is left for the parser to report.
            self.open_delimiters.pop();
        }
    }

    /// Ends the statement, unless inside `( … )` or `[ … ]` or right after
    /// another line break or the start of the file.
    fn push_newline(&mut self, start: usize) {
        let inside_expression = matches!(
            self.open_delimiters.last(),
            Some(Delimiter::Paren | Delimiter::Bracket)
        );
        let follows_statement = self
            .tokens
            .last()
            .is_some_and(|token| token.kind != TokenKind::Newline);
        if follows_statement && !inside_expression {
            self.push(TokenKind::Newline, start);
        }
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        let span = Span::new(start, self.cursor.position());
        self.tokens.push(Token::new(kind, span));
    }

    fn error(&mut self, kind: LexErrorKind, span: Span) {
        self.errors.push(LexError::new(kind, span));
    }
}

const fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

const fn is_identifier_continue(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

#[cfg(test)]
mod tests;
