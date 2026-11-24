//! Expressions, by precedence climbing (Pratt parsing).

use super::{Expected, ParseError, ParseErrorKind, ParseResult, Parser};
use crate::ast::{BinaryOperator, Expr, ExprKind, Literal, Name, UnaryOperator};
use crate::{Keyword, TokenKind};

/// Precedence of `||`, the operator that binds the loosest.
const LOWEST_PRECEDENCE: u8 = 1;

/// Name of the constructor when called explicitly, as in `super.init(…)`.
const CONSTRUCTOR_NAME: &str = "init";

/// Operators written after a value. `is` and `as` take a type, not a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InfixOperator {
    Binary(BinaryOperator),
    TypeTest,
    Cast,
}

impl InfixOperator {
    const fn from_token(kind: &TokenKind) -> Option<Self> {
        let binary = match kind {
            TokenKind::OrOr => BinaryOperator::Or,
            TokenKind::AndAnd => BinaryOperator::And,
            TokenKind::EqualEqual => BinaryOperator::Equal,
            TokenKind::BangEqual => BinaryOperator::NotEqual,
            TokenKind::Less => BinaryOperator::Less,
            TokenKind::LessEqual => BinaryOperator::LessEqual,
            TokenKind::Greater => BinaryOperator::Greater,
            TokenKind::GreaterEqual => BinaryOperator::GreaterEqual,
            TokenKind::QuestionQuestion => BinaryOperator::Coalesce,
            TokenKind::DotDot => BinaryOperator::Range,
            TokenKind::Plus => BinaryOperator::Add,
            TokenKind::Minus => BinaryOperator::Subtract,
            TokenKind::Star => BinaryOperator::Multiply,
            TokenKind::Slash => BinaryOperator::Divide,
            TokenKind::Percent => BinaryOperator::Remainder,
            TokenKind::Keyword(Keyword::Is) => return Some(Self::TypeTest),
            TokenKind::Keyword(Keyword::As) => return Some(Self::Cast),
            _ => return None,
        };
        Some(Self::Binary(binary))
    }

    /// Higher binds tighter. Matches the table in `docs/starscript.md`.
    const fn precedence(self) -> u8 {
        match self {
            Self::Binary(BinaryOperator::Or) => 1,
            Self::Binary(BinaryOperator::And) => 2,
            Self::Binary(BinaryOperator::Equal | BinaryOperator::NotEqual) => 3,
            Self::Binary(
                BinaryOperator::Less
                | BinaryOperator::LessEqual
                | BinaryOperator::Greater
                | BinaryOperator::GreaterEqual,
            )
            | Self::TypeTest => 4,
            Self::Binary(BinaryOperator::Coalesce) => 5,
            Self::Binary(BinaryOperator::Range) => 6,
            Self::Binary(BinaryOperator::Add | BinaryOperator::Subtract) => 7,
            Self::Binary(
                BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Remainder,
            ) => 8,
            Self::Cast => 9,
        }
    }

    /// `a < b < c` almost never means what it looks like, so it is rejected.
    const fn is_chainable(self) -> bool {
        !matches!(self.precedence(), 3 | 4 | 6)
    }

    /// `a ?? b ?? c` is `a ?? (b ?? c)`: the first value that is not `null`.
    const fn groups_right(self) -> bool {
        matches!(self, Self::Binary(BinaryOperator::Coalesce))
    }
}

impl Parser<'_> {
    /// Any expression.
    pub(super) fn expression(&mut self) -> ParseResult<Expr> {
        self.binary_expression(LOWEST_PRECEDENCE)
    }

    /// An expression whose operators bind at least as tight as `min_precedence`.
    /// Most operators of equal precedence group to the left: `a - b - c` is
    /// `(a - b) - c`.
    fn binary_expression(&mut self, min_precedence: u8) -> ParseResult<Expr> {
        let mut left = self.unary()?;
        while let Some(operator) = InfixOperator::from_token(self.peek()) {
            if operator.precedence() < min_precedence {
                break;
            }
            self.advance();
            left = self.infix(left, operator)?;
            if !operator.is_chainable() {
                self.reject_chain(operator)?;
            }
        }
        Ok(left)
    }

    fn infix(&mut self, left: Expr, operator: InfixOperator) -> ParseResult<Expr> {
        let start = left.span.start();
        let kind = match operator {
            InfixOperator::TypeTest => ExprKind::TypeTest {
                type_ref: self.type_ref()?,
                value: Box::new(left),
            },
            InfixOperator::Cast => ExprKind::Cast {
                type_ref: self.type_ref()?,
                value: Box::new(left),
            },
            InfixOperator::Binary(binary) => {
                // An operator at the end of a line continues on the next one.
                self.skip_newlines();
                let right_precedence = if operator.groups_right() {
                    operator.precedence()
                } else {
                    operator.precedence() + 1
                };
                let right = self.binary_expression(right_precedence)?;
                ExprKind::Binary {
                    operator: binary,
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
        };
        let span = self.span_from(start);
        Ok(self.expression_node(kind, span))
    }

    fn reject_chain(&self, previous: InfixOperator) -> ParseResult<()> {
        match InfixOperator::from_token(self.peek()) {
            Some(next) if next.precedence() == previous.precedence() => Err(ParseError::new(
                ParseErrorKind::ChainedOperators,
                self.current().span,
            )),
            _ => Ok(()),
        }
    }

    /// `-value`, `!value`, `await value` or a postfix expression.
    fn unary(&mut self) -> ParseResult<Expr> {
        let operator = match self.peek() {
            TokenKind::Minus => UnaryOperator::Negate,
            TokenKind::Bang => UnaryOperator::Not,
            TokenKind::Keyword(Keyword::Await) => return self.await_expression(),
            _ => return self.nested(Self::postfix),
        };
        let start = self.advance().span.start();
        let operand = self.nested(Self::unary)?;
        let kind = ExprKind::Unary {
            operator,
            operand: Box::new(operand),
        };
        let span = self.span_from(start);
        Ok(self.expression_node(kind, span))
    }

    /// `await value`. It binds like `-`: `await a.b() + 1` waits for
    /// `a.b()` and then adds.
    fn await_expression(&mut self) -> ParseResult<Expr> {
        let start = self.advance().span.start();
        let value = self.nested(Self::unary)?;
        let span = self.span_from(start);
        Ok(self.expression_node(ExprKind::Await(Box::new(value)), span))
    }

    /// A primary expression followed by calls, member accesses and indexing.
    fn postfix(&mut self) -> ParseResult<Expr> {
        let mut expression = self.primary()?;
        loop {
            let start = expression.span.start();
            let kind = match self.peek() {
                TokenKind::LeftParen => {
                    self.advance();
                    ExprKind::Call {
                        arguments: self.expression_list(TokenKind::RightParen)?,
                        callee: Box::new(expression),
                    }
                }
                TokenKind::Dot | TokenKind::QuestionDot => {
                    let null_safe = self.advance().kind == TokenKind::QuestionDot;
                    self.skip_newlines();
                    ExprKind::Member {
                        name: self.member_name()?,
                        object: Box::new(expression),
                        null_safe,
                    }
                }
                TokenKind::LeftBracket => {
                    self.advance();
                    let index = self.expression()?;
                    self.expect(TokenKind::RightBracket)?;
                    ExprKind::Index {
                        object: Box::new(expression),
                        index: Box::new(index),
                    }
                }
                _ => return Ok(expression),
            };
            let span = self.span_from(start);
            expression = self.expression_node(kind, span);
        }
    }

    /// The name after `.`; `init` is accepted so `super.init(…)` can be written.
    fn member_name(&mut self) -> ParseResult<Name> {
        if self.at_keyword(Keyword::Init) {
            let span = self.advance().span;
            return Ok(Name {
                text: CONSTRUCTOR_NAME.to_owned(),
                span,
            });
        }
        self.expect_name()
    }

    fn primary(&mut self) -> ParseResult<Expr> {
        let token = self.current();
        let kind = match &token.kind {
            TokenKind::Integer(value) => ExprKind::Literal(Literal::Integer(*value)),
            TokenKind::Float(value) => ExprKind::Literal(Literal::Float(*value)),
            TokenKind::String(value) => ExprKind::Literal(Literal::String(value.clone())),
            TokenKind::Keyword(Keyword::True) => ExprKind::Literal(Literal::Bool(true)),
            TokenKind::Keyword(Keyword::False) => ExprKind::Literal(Literal::Bool(false)),
            TokenKind::Keyword(Keyword::Null) => ExprKind::Literal(Literal::Null),
            TokenKind::Keyword(Keyword::SelfValue) => ExprKind::SelfValue,
            TokenKind::Keyword(Keyword::Super) => ExprKind::Super,
            TokenKind::Identifier(name) => ExprKind::Name(name.clone()),
            TokenKind::LeftParen => return self.parenthesized(),
            TokenKind::LeftBracket => return self.array(),
            _ => return Err(self.unexpected(Expected::Expression)),
        };
        self.advance();
        Ok(self.expression_node(kind, token.span))
    }

    /// `(expression)`: only changes grouping, so no node of its own.
    fn parenthesized(&mut self) -> ParseResult<Expr> {
        let start = self.start();
        self.expect(TokenKind::LeftParen)?;
        let inner = self.expression()?;
        self.expect(TokenKind::RightParen)?;
        Ok(Expr {
            span: self.span_from(start),
            ..inner
        })
    }

    /// `[a, b, c]`
    fn array(&mut self) -> ParseResult<Expr> {
        let start = self.start();
        self.expect(TokenKind::LeftBracket)?;
        let elements = self.expression_list(TokenKind::RightBracket)?;
        let span = self.span_from(start);
        Ok(self.expression_node(ExprKind::Array(elements), span))
    }

    /// Comma-separated expressions up to `close`, a trailing comma allowed.
    /// The opening bracket must already be consumed.
    pub(super) fn expression_list(&mut self, close: TokenKind) -> ParseResult<Vec<Expr>> {
        let mut expressions = Vec::new();
        while !self.at(&close) {
            expressions.push(self.expression()?);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(close)?;
        Ok(expressions)
    }
}
