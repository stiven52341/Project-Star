//! Blocks and statements.

use super::{ParseError, ParseErrorKind, ParseResult, Parser};
use crate::ast::{
    AssignOperator, Block, ElseBranch, Expr, ExprKind, IfStatement, Statement, StatementKind,
};
use crate::{Keyword, TokenKind};

impl Parser<'_> {
    /// `{ statements }`. The `{` may be on the next line.
    pub(super) fn block(&mut self) -> ParseResult<Block> {
        self.skip_newlines();
        self.nested(|parser| {
            let start = parser.start();
            parser.expect(TokenKind::LeftBrace)?;
            let statements = parser.statements();
            parser.expect(TokenKind::RightBrace)?;
            Ok(Block {
                statements,
                span: parser.span_from(start),
            })
        })
    }

    fn statements(&mut self) -> Vec<Statement> {
        let mut statements = Vec::new();
        loop {
            self.skip_separators();
            if self.at(&TokenKind::RightBrace) || self.at(&TokenKind::EndOfFile) {
                return statements;
            }
            match self.statement() {
                Ok(statement) => statements.push(statement),
                Err(error) => self.recover_from(error),
            }
        }
    }

    fn statement(&mut self) -> ParseResult<Statement> {
        let start = self.start();
        let kind = match self.peek() {
            TokenKind::Keyword(Keyword::Var | Keyword::Const) => {
                StatementKind::Variable(self.variable_declaration()?)
            }
            TokenKind::Keyword(Keyword::If) => StatementKind::If(self.if_statement()?),
            TokenKind::Keyword(Keyword::While) => self.while_statement()?,
            TokenKind::Keyword(Keyword::For) => self.for_statement()?,
            TokenKind::Keyword(Keyword::Return) => self.return_statement()?,
            TokenKind::Keyword(Keyword::Break) => {
                self.advance();
                StatementKind::Break
            }
            TokenKind::Keyword(Keyword::Continue) => {
                self.advance();
                StatementKind::Continue
            }
            _ => self.expression_or_assignment()?,
        };
        let span = self.span_from(start);
        if !ends_with_block(&kind) {
            self.end_statement();
        }
        Ok(Statement { kind, span })
    }

    /// `if condition { … } [else if … | else { … }]`. The `else` may be on the
    /// line after the `}`.
    fn if_statement(&mut self) -> ParseResult<IfStatement> {
        let start = self.start();
        self.expect_keyword(Keyword::If)?;
        let condition = self.condition()?;
        let then_block = self.block()?;
        let else_branch = if *self.peek_past_newlines() == TokenKind::Keyword(Keyword::Else) {
            self.skip_newlines();
            self.advance();
            Some(if self.at_keyword(Keyword::If) {
                ElseBranch::If(Box::new(self.if_statement()?))
            } else {
                ElseBranch::Block(self.block()?)
            })
        } else {
            None
        };
        Ok(IfStatement {
            condition,
            then_block,
            else_branch,
            span: self.span_from(start),
        })
    }

    fn while_statement(&mut self) -> ParseResult<StatementKind> {
        self.expect_keyword(Keyword::While)?;
        let condition = self.condition()?;
        let body = self.block()?;
        Ok(StatementKind::While { condition, body })
    }

    /// `for variable in iterable { … }`
    fn for_statement(&mut self) -> ParseResult<StatementKind> {
        self.expect_keyword(Keyword::For)?;
        let variable = self.expect_name()?;
        self.expect_keyword(Keyword::In)?;
        let iterable = self.expression()?;
        let body = self.block()?;
        Ok(StatementKind::For {
            variable,
            iterable,
            body,
        })
    }

    fn return_statement(&mut self) -> ParseResult<StatementKind> {
        self.expect_keyword(Keyword::Return)?;
        let value = if self.at_end_of_statement() {
            None
        } else {
            Some(self.expression()?)
        };
        Ok(StatementKind::Return(value))
    }

    /// The condition of `if` or `while`, with a clear error for `if x = 3`.
    fn condition(&mut self) -> ParseResult<Expr> {
        let condition = self.expression()?;
        if self.at(&TokenKind::Equal) {
            return Err(ParseError::new(
                ParseErrorKind::AssignmentInCondition,
                self.current().span,
            ));
        }
        Ok(condition)
    }

    fn expression_or_assignment(&mut self) -> ParseResult<StatementKind> {
        let target = self.expression()?;
        let Some(operator) = assign_operator(self.peek()) else {
            return Ok(StatementKind::Expression(target));
        };
        if !is_assignable(&target) {
            return Err(ParseError::new(
                ParseErrorKind::InvalidAssignmentTarget,
                target.span,
            ));
        }
        self.advance();
        self.skip_newlines();
        let value = self.expression()?;
        Ok(StatementKind::Assignment {
            target,
            operator,
            value,
        })
    }
}

/// Statements ending in a block need no line break or `;` after them.
const fn ends_with_block(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::If(_) | StatementKind::While { .. } | StatementKind::For { .. }
    )
}

const fn assign_operator(kind: &TokenKind) -> Option<AssignOperator> {
    match kind {
        TokenKind::Equal => Some(AssignOperator::Assign),
        TokenKind::PlusEqual => Some(AssignOperator::Add),
        TokenKind::MinusEqual => Some(AssignOperator::Subtract),
        TokenKind::StarEqual => Some(AssignOperator::Multiply),
        TokenKind::SlashEqual => Some(AssignOperator::Divide),
        TokenKind::PercentEqual => Some(AssignOperator::Remainder),
        _ => None,
    }
}

const fn is_assignable(target: &Expr) -> bool {
    matches!(
        target.kind,
        ExprKind::Name(_) | ExprKind::Member { .. } | ExprKind::Index { .. }
    )
}
