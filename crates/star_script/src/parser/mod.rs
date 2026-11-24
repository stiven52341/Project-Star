//! Second compiler stage: builds the [syntax tree](crate::ast) from tokens.
//!
//! A recursive-descent parser. After a mistake it skips to the end of the
//! statement or member and carries on, so every mistake is reported at once.

mod declaration;
mod error;
mod expression;
mod statement;

pub use error::{Expected, ParseError, ParseErrorKind};

use crate::ast::{Expr, ExprId, ExprKind, Name, SourceFile, TypeRef, TypeRefKind};
use crate::{Keyword, Span, Token, TokenKind};

/// Deeper nesting is rejected rather than risking a stack overflow.
const MAX_NESTING_DEPTH: usize = 128;

/// Returned when the token list is empty, so the parser always sees an end.
static END_OF_FILE: Token = Token::new(TokenKind::EndOfFile, Span::new(0, 0));

/// Builds the syntax tree of a file from its tokens.
///
/// # Errors
///
/// Returns every syntax mistake found, in source order.
pub fn parse(tokens: &[Token]) -> Result<SourceFile, Vec<ParseError>> {
    parse_numbering_from(tokens, 0).map(|(file, _)| file)
}

/// Builds the syntax tree of one file of a program, numbering its
/// expressions from `first_expression_id`, so the files of a program never
/// share an [`ExprId`]. Returns the tree and the first identifier left free.
///
/// # Errors
///
/// As [`parse`].
pub(crate) fn parse_numbering_from(
    tokens: &[Token],
    first_expression_id: u32,
) -> Result<(SourceFile, u32), Vec<ParseError>> {
    let mut parser = Parser::new(tokens, first_expression_id);
    let file = parser.source_file();
    if parser.errors.is_empty() {
        Ok((file, parser.next_expression_id))
    } else {
        Err(parser.errors)
    }
}

type ParseResult<T> = Result<T, ParseError>;

struct Parser<'tokens> {
    tokens: &'tokens [Token],
    position: usize,
    errors: Vec<ParseError>,
    depth: usize,
    next_expression_id: u32,
}

impl<'tokens> Parser<'tokens> {
    const fn new(tokens: &'tokens [Token], first_expression_id: u32) -> Self {
        Self {
            tokens,
            position: 0,
            errors: Vec::new(),
            depth: 0,
            next_expression_id: first_expression_id,
        }
    }

    fn source_file(&mut self) -> SourceFile {
        let mut classes = Vec::new();
        loop {
            self.skip_separators();
            if self.at(&TokenKind::EndOfFile) {
                return SourceFile {
                    classes,
                    next_expression_id: self.next_expression_id,
                };
            }
            if self.at_keyword(Keyword::Class) {
                match self.class_declaration() {
                    Ok(class) => classes.push(class),
                    Err(error) => self.recover_from(error),
                }
            } else {
                let error = self.unexpected(Expected::Class);
                // Consume at least one token: a stray `}` would stop recovery.
                self.advance();
                self.recover_from(error);
            }
        }
    }

    // --- Looking at tokens ---

    fn current(&self) -> &'tokens Token {
        self.tokens
            .get(self.position)
            .or_else(|| self.tokens.last())
            .unwrap_or(&END_OF_FILE)
    }

    fn peek(&self) -> &'tokens TokenKind {
        &self.current().kind
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn at_keyword(&self, keyword: Keyword) -> bool {
        *self.peek() == TokenKind::Keyword(keyword)
    }

    /// Whether the statement ends here: `}` and the end of the file also end it.
    fn at_end_of_statement(&self) -> bool {
        matches!(
            self.peek(),
            TokenKind::Newline
                | TokenKind::Semicolon
                | TokenKind::RightBrace
                | TokenKind::EndOfFile
        )
    }

    /// The next token that is not a line break.
    fn peek_past_newlines(&self) -> &'tokens TokenKind {
        self.tokens
            .get(self.position..)
            .unwrap_or_default()
            .iter()
            .map(|token| &token.kind)
            .find(|kind| **kind != TokenKind::Newline)
            .unwrap_or(&TokenKind::EndOfFile)
    }

    // --- Consuming tokens ---

    /// Consumes the current token. The end of the file is never consumed.
    fn advance(&mut self) -> &'tokens Token {
        let token = self.current();
        if token.kind != TokenKind::EndOfFile {
            self.position += 1;
        }
        token
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        let matches = self.at(kind);
        if matches {
            self.advance();
        }
        matches
    }

    fn eat_keyword(&mut self, keyword: Keyword) -> bool {
        self.eat(&TokenKind::Keyword(keyword))
    }

    fn expect(&mut self, kind: TokenKind) -> ParseResult<Span> {
        if self.at(&kind) {
            Ok(self.advance().span)
        } else {
            Err(self.unexpected(Expected::Token(kind)))
        }
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> ParseResult<Span> {
        self.expect(TokenKind::Keyword(keyword))
    }

    fn expect_name(&mut self) -> ParseResult<Name> {
        let token = self.current();
        let TokenKind::Identifier(text) = &token.kind else {
            return Err(self.unexpected(Expected::Name));
        };
        self.advance();
        Ok(Name {
            text: text.clone(),
            span: token.span,
        })
    }

    /// `Name`, `[Element]`, optionally followed by `?`.
    fn type_ref(&mut self) -> ParseResult<TypeRef> {
        let start = self.start();
        let kind = match self.peek() {
            TokenKind::Identifier(name) => {
                self.advance();
                TypeRefKind::Named(name.clone())
            }
            TokenKind::LeftBracket => {
                self.advance();
                let element = self.nested(Self::type_ref)?;
                self.expect(TokenKind::RightBracket)?;
                TypeRefKind::List(Box::new(element))
            }
            _ => return Err(self.unexpected(Expected::Type)),
        };
        let type_ref = TypeRef {
            kind,
            span: self.span_from(start),
        };
        if !self.eat(&TokenKind::Question) {
            return Ok(type_ref);
        }
        Ok(TypeRef {
            kind: TypeRefKind::Nullable(Box::new(type_ref)),
            span: self.span_from(start),
        })
    }

    /// Creates an expression node with a fresh identifier.
    const fn expression_node(&mut self, kind: ExprKind, span: Span) -> Expr {
        let id = ExprId::new(self.next_expression_id);
        self.next_expression_id += 1;
        Expr { id, kind, span }
    }

    fn skip_newlines(&mut self) {
        while self.eat(&TokenKind::Newline) {}
    }

    fn skip_separators(&mut self) {
        while self.eat(&TokenKind::Newline) || self.eat(&TokenKind::Semicolon) {}
    }

    /// Consumes the line break or `;` that ends a statement; a mistake here
    /// does not discard the statement that was already read.
    fn end_statement(&mut self) {
        if self.eat(&TokenKind::Newline) || self.eat(&TokenKind::Semicolon) {
            return;
        }
        if !self.at_end_of_statement() {
            let error = self.unexpected(Expected::EndOfStatement);
            self.recover_from(error);
        }
    }

    // --- Spans ---

    fn start(&self) -> usize {
        self.current().span.start()
    }

    /// From `start` to the end of the last consumed token.
    fn span_from(&self, start: usize) -> Span {
        let end = self
            .position
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map_or(start, |token| token.span.end());
        Span::new(start, end)
    }

    // --- Errors ---

    fn unexpected(&self, expected: Expected) -> ParseError {
        let token = self.current();
        ParseError::new(
            ParseErrorKind::Unexpected {
                expected,
                found: token.kind.clone(),
            },
            token.span,
        )
    }

    /// Records `error` and skips to the end of the broken statement or member:
    /// past the next line break or `;`, or up to the `}` that closes the
    /// enclosing block. Nested blocks are skipped whole.
    fn recover_from(&mut self, error: ParseError) {
        self.errors.push(error);
        let mut depth = 0_usize;
        loop {
            match self.peek() {
                TokenKind::EndOfFile => return,
                TokenKind::Newline | TokenKind::Semicolon if depth == 0 => {
                    self.advance();
                    return;
                }
                TokenKind::RightBrace if depth == 0 => return,
                TokenKind::LeftBrace => depth += 1,
                TokenKind::RightBrace => depth -= 1,
                _ => {}
            }
            self.advance();
        }
    }

    /// Runs `parse` one nesting level deeper, refusing absurdly deep code.
    fn nested<T>(&mut self, parse: impl FnOnce(&mut Self) -> ParseResult<T>) -> ParseResult<T> {
        if self.depth >= MAX_NESTING_DEPTH {
            return Err(ParseError::new(
                ParseErrorKind::TooDeeplyNested,
                self.current().span,
            ));
        }
        self.depth += 1;
        let result = parse(self);
        self.depth -= 1;
        result
    }
}

#[cfg(test)]
mod tests;
