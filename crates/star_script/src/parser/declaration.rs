//! Classes, members and parameters.

use super::{Expected, ParseError, ParseErrorKind, ParseResult, Parser};
use crate::ast::{
    ClassDecl, ConstructorDecl, Decorator, Member, MemberKind, MethodDecl, Modifier, ModifierKind,
    Parameter, VariableDecl, VariableKind,
};
use crate::{Keyword, TokenKind};

impl Parser<'_> {
    /// `class Name [extends Base] { members }`
    pub(super) fn class_declaration(&mut self) -> ParseResult<ClassDecl> {
        let start = self.start();
        self.expect_keyword(Keyword::Class)?;
        let name = self.expect_name()?;
        let base = if self.eat_keyword(Keyword::Extends) {
            Some(self.expect_name()?)
        } else {
            None
        };
        self.skip_newlines();
        self.expect(TokenKind::LeftBrace)?;
        let members = self.class_members();
        self.expect(TokenKind::RightBrace)?;
        Ok(ClassDecl {
            name,
            base,
            members,
            span: self.span_from(start),
        })
    }

    fn class_members(&mut self) -> Vec<Member> {
        let mut members = Vec::new();
        loop {
            self.skip_separators();
            if self.at(&TokenKind::RightBrace) || self.at(&TokenKind::EndOfFile) {
                return members;
            }
            match self.member() {
                Ok(member) => members.push(member),
                Err(error) => self.recover_from(error),
            }
        }
    }

    fn member(&mut self) -> ParseResult<Member> {
        let start = self.start();
        let decorators = self.decorators()?;
        let modifiers = self.modifiers();
        let kind = match self.peek() {
            TokenKind::Keyword(Keyword::Var | Keyword::Const) => {
                MemberKind::Variable(self.variable_declaration()?)
            }
            TokenKind::Keyword(Keyword::Func) => MemberKind::Method(self.method()?),
            TokenKind::Keyword(Keyword::Init) => MemberKind::Constructor(self.constructor()?),
            _ => return Err(self.unexpected(Expected::ClassMember)),
        };
        let span = self.span_from(start);
        if matches!(kind, MemberKind::Variable(_)) {
            self.end_statement();
        }
        Ok(Member {
            decorators,
            modifiers,
            kind,
            span,
        })
    }

    /// Reads `@name` and `@name(arguments)`, each followed by line breaks
    /// or not. Which decorators fit which members is checked later.
    fn decorators(&mut self) -> ParseResult<Vec<Decorator>> {
        let mut decorators = Vec::new();
        while self.at(&TokenKind::At) {
            let start = self.start();
            self.expect(TokenKind::At)?;
            let name = self.expect_name()?;
            let arguments = if self.eat(&TokenKind::LeftParen) {
                self.expression_list(TokenKind::RightParen)?
            } else {
                Vec::new()
            };
            decorators.push(Decorator {
                name,
                arguments,
                span: self.span_from(start),
            });
            self.skip_newlines();
        }
        Ok(decorators)
    }

    /// Reads `private`, `static`, `override`, `async`, `parallel` and
    /// `export` in any order. Which members
    /// accept which modifiers is checked later, by semantic analysis.
    fn modifiers(&mut self) -> Vec<Modifier> {
        let mut modifiers: Vec<Modifier> = Vec::new();
        while let TokenKind::Keyword(keyword) = self.peek() {
            let Some(kind) = ModifierKind::from_keyword(*keyword) else {
                break;
            };
            let span = self.advance().span;
            if modifiers.iter().any(|modifier| modifier.kind == kind) {
                let error = ParseErrorKind::DuplicateModifier(kind);
                self.errors.push(ParseError::new(error, span));
            } else {
                modifiers.push(Modifier { kind, span });
            }
        }
        modifiers
    }

    /// `var name [: Type] [= value]` or `const NAME [: Type] = value`, without
    /// the end of the statement. Shared by fields and local variables.
    pub(super) fn variable_declaration(&mut self) -> ParseResult<VariableDecl> {
        let kind = if self.eat_keyword(Keyword::Const) {
            VariableKind::Const
        } else {
            self.expect_keyword(Keyword::Var)?;
            VariableKind::Var
        };
        let name = self.expect_name()?;
        let type_annotation = if self.eat(&TokenKind::Colon) {
            Some(self.type_ref()?)
        } else {
            None
        };
        let initializer = if self.eat(&TokenKind::Equal) {
            self.skip_newlines();
            Some(self.expression()?)
        } else {
            None
        };
        if kind == VariableKind::Const && initializer.is_none() {
            return Err(ParseError::new(
                ParseErrorKind::ConstantWithoutValue,
                name.span,
            ));
        }
        Ok(VariableDecl {
            kind,
            name,
            type_annotation,
            initializer,
        })
    }

    /// `func name(parameters) [-> Type] { body }`
    fn method(&mut self) -> ParseResult<MethodDecl> {
        self.expect_keyword(Keyword::Func)?;
        let name = self.expect_name()?;
        let parameters = self.parameters()?;
        let return_type = if self.eat(&TokenKind::Arrow) {
            Some(self.type_ref()?)
        } else {
            None
        };
        let body = self.block()?;
        Ok(MethodDecl {
            name,
            parameters,
            return_type,
            body,
        })
    }

    /// `init(parameters) { body }`
    fn constructor(&mut self) -> ParseResult<ConstructorDecl> {
        self.expect_keyword(Keyword::Init)?;
        let parameters = self.parameters()?;
        let body = self.block()?;
        Ok(ConstructorDecl { parameters, body })
    }

    /// `(name: Type, …)`, a trailing comma allowed.
    fn parameters(&mut self) -> ParseResult<Vec<Parameter>> {
        self.expect(TokenKind::LeftParen)?;
        let mut parameters = Vec::new();
        while !self.at(&TokenKind::RightParen) {
            let start = self.start();
            let name = self.expect_name()?;
            self.expect(TokenKind::Colon)?;
            let type_annotation = self.type_ref()?;
            parameters.push(Parameter {
                name,
                type_annotation,
                span: self.span_from(start),
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RightParen)?;
        Ok(parameters)
    }
}
