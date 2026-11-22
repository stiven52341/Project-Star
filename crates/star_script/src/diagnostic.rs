//! Errors of any compiler stage, in a single type.

use std::error::Error;
use std::fmt;

use crate::codegen::CodegenError;
use crate::semantic::SemanticError;
use crate::{LexError, LineIndex, ParseError, Position, Span};

/// A mistake found by any stage of the compiler.
#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    /// Found while splitting the text into tokens.
    Lex(LexError),
    /// Found while reading the structure of the code.
    Parse(ParseError),
    /// Found while checking the meaning of the code.
    Semantic(SemanticError),
    /// Found while translating the code to Rust.
    Codegen(CodegenError),
}

impl CompileError {
    /// The code that is wrong.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Lex(error) => error.span,
            Self::Parse(error) => error.span,
            Self::Semantic(error) => error.span,
            Self::Codegen(error) => error.span,
        }
    }

    /// One-line report in the usual compiler format:
    /// `file:line:column: error: message`.
    #[must_use]
    pub fn report(&self, file_name: &str, lines: &LineIndex<'_>) -> String {
        format_report(file_name, lines.position(self.span().start()), self)
    }
}

/// `file:line:column: error: message`, the format of every report.
pub(crate) fn format_report(file_name: &str, position: Position, error: &CompileError) -> String {
    format!(
        "{file_name}:{}:{}: error: {error}",
        position.line, position.column
    )
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lex(error) => error.fmt(formatter),
            Self::Parse(error) => error.fmt(formatter),
            Self::Semantic(error) => error.fmt(formatter),
            Self::Codegen(error) => error.fmt(formatter),
        }
    }
}

impl Error for CompileError {}

impl From<LexError> for CompileError {
    fn from(error: LexError) -> Self {
        Self::Lex(error)
    }
}

impl From<SemanticError> for CompileError {
    fn from(error: SemanticError) -> Self {
        Self::Semantic(error)
    }
}

impl From<CodegenError> for CompileError {
    fn from(error: CodegenError) -> Self {
        Self::Codegen(error)
    }
}

impl From<ParseError> for CompileError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenize;

    #[test]
    fn report_shows_file_line_and_column() {
        let source = "class A {\n  var x = @\n}";
        let error = CompileError::from(tokenize(source).expect_err("has errors").remove(0));

        let report = error.report("a.star", &LineIndex::new(source));

        assert_eq!(report, "a.star:2:11: error: unexpected character `@`");
    }
}
