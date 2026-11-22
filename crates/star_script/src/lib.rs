//! Compiler of StarScript, the object-oriented language of ProjectStar games.
//!
//! The compiler turns `.star` source files into Rust code, which is then built
//! like any other crate. It is split into stages, each usable and tested on
//! its own:
//!
//! 1. [`lexer`]: source text → [`Token`]s.
//! 2. [`parser`]: tokens → [syntax tree](ast).
//! 3. [`semantic`]: syntax tree → [`Analysis`](semantic::Analysis): types,
//!    names and null safety checked.
//!
//! [`check_source`] runs them all. The language is specified in
//! `docs/starscript.md`. This crate has no dependencies, so the editor and
//! command-line tools can embed it freely.

pub mod ast;
mod diagnostic;
pub mod lexer;
pub mod parser;
pub mod semantic;
mod source;
mod token;

pub use diagnostic::CompileError;
pub use lexer::{LexError, LexErrorKind, tokenize};
pub use parser::{ParseError, ParseErrorKind, parse};
pub use source::{LineIndex, Position, Span};
pub use token::{Keyword, Token, TokenKind};

/// Reads the source of a `.star` file into its syntax tree.
///
/// # Errors
///
/// Returns every mistake found, in source order. Lexical mistakes are
/// reported alone: parsing broken tokens would only add confusing errors.
pub fn parse_source(source: &str) -> Result<ast::SourceFile, Vec<CompileError>> {
    let tokens = tokenize(source).map_err(|errors| {
        errors
            .into_iter()
            .map(CompileError::from)
            .collect::<Vec<_>>()
    })?;
    parse(&tokens).map_err(|errors| errors.into_iter().map(CompileError::from).collect())
}

/// A source file whose meaning has been checked.
#[derive(Debug, Clone)]
pub struct CheckedSource {
    /// Its syntax tree.
    pub syntax: ast::SourceFile,
    /// What semantic analysis learned about it.
    pub analysis: semantic::Analysis,
}

/// Reads and checks the source of a `.star` file.
///
/// # Errors
///
/// Returns every mistake found by the first stage that finds any; later
/// stages would only add errors caused by the earlier ones.
pub fn check_source(source: &str) -> Result<CheckedSource, Vec<CompileError>> {
    let syntax = parse_source(source)?;
    let analysis = semantic::check(&syntax).map_err(|errors| {
        errors
            .into_iter()
            .map(CompileError::from)
            .collect::<Vec<_>>()
    })?;
    Ok(CheckedSource { syntax, analysis })
}
