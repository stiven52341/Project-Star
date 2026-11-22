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
//! 4. [`codegen`]: checked program → Rust code, which depends on the
//!    `star_script_runtime` crate.
//!
//! [`check_source`] runs the first three and [`compile_source`] all of them;
//! their `_with` versions also take the project's [`GameSettings`]
//! (e.g. its input actions). [`check_program_with`] and
//! [`compile_program_with`] do the same for a program of several files, a
//! [`SourceSet`]. The language is specified in
//! `docs/starscript.md`. This crate has no dependencies, so the editor and
//! command-line tools can embed it freely.

pub mod ast;
pub mod codegen;
mod diagnostic;
pub mod lexer;
pub mod package;
pub mod parser;
pub mod semantic;
pub mod settings;
mod source;
mod source_set;
mod token;

pub use diagnostic::CompileError;
pub use lexer::{LexError, LexErrorKind, tokenize};
pub use parser::{ParseError, ParseErrorKind, parse};
pub use settings::{GameSettings, InputAction};
pub use source::{LineIndex, Position, Span};
pub use source_set::{SourceLocation, SourceSet};
pub use token::{Keyword, Token, TokenKind};

/// Reads the source of a `.star` file into its syntax tree.
///
/// # Errors
///
/// Returns every mistake found, in source order. Lexical mistakes are
/// reported alone: parsing broken tokens would only add confusing errors.
pub fn parse_source(source: &str) -> Result<ast::SourceFile, Vec<CompileError>> {
    parse_file(source, 0)
}

/// Reads every file of a program into one syntax tree with all their
/// classes, in the order the files were added.
///
/// # Errors
///
/// Returns the mistakes of every file, as [`parse_source`] finds them in
/// each one: a file's lexical mistakes do not hide another file's syntax
/// mistakes.
pub fn parse_program(sources: &SourceSet) -> Result<ast::SourceFile, Vec<CompileError>> {
    let mut classes = Vec::new();
    let mut errors = Vec::new();
    for (text, start) in sources.texts() {
        match parse_file(text, start) {
            Ok(file) => classes.extend(file.classes),
            Err(file_errors) => errors.extend(file_errors),
        }
    }
    if errors.is_empty() {
        Ok(ast::SourceFile { classes })
    } else {
        Err(errors)
    }
}

/// Parses one file placed at offset `start` of its program, so every span
/// it reports is an offset of the program.
fn parse_file(source: &str, start: usize) -> Result<ast::SourceFile, Vec<CompileError>> {
    let mut tokens = tokenize(source).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                CompileError::from(LexError {
                    span: error.span.shifted(start),
                    ..error
                })
            })
            .collect::<Vec<_>>()
    })?;
    for token in &mut tokens {
        token.span = token.span.shifted(start);
    }
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
    check_source_with(source, &GameSettings::default())
}

/// Reads and checks the source of a `.star` file of a game with `settings`.
///
/// # Errors
///
/// As [`check_source`].
pub fn check_source_with(
    source: &str,
    settings: &GameSettings,
) -> Result<CheckedSource, Vec<CompileError>> {
    check_syntax(parse_source(source)?, settings)
}

/// Reads and checks every file of a program of a game with `settings`, as
/// one program: a class of one file can use the classes of the others.
///
/// # Errors
///
/// As [`check_source`]; the spans of the mistakes are located with
/// [`SourceSet::locate`].
pub fn check_program_with(
    sources: &SourceSet,
    settings: &GameSettings,
) -> Result<CheckedSource, Vec<CompileError>> {
    check_syntax(parse_program(sources)?, settings)
}

fn check_syntax(
    syntax: ast::SourceFile,
    settings: &GameSettings,
) -> Result<CheckedSource, Vec<CompileError>> {
    let analysis = semantic::check_with(&syntax, settings).map_err(|errors| {
        errors
            .into_iter()
            .map(CompileError::from)
            .collect::<Vec<_>>()
    })?;
    Ok(CheckedSource { syntax, analysis })
}

/// Compiles the source of a `.star` file into Rust code.
///
/// See [`codegen::generate`] for the shape of the result.
///
/// # Errors
///
/// Returns the mistakes found, as [`check_source`] does, or the construct
/// that cannot be translated to Rust yet.
pub fn compile_source(source: &str) -> Result<codegen::GeneratedProgram, Vec<CompileError>> {
    compile_source_with(source, &GameSettings::default())
}

/// Compiles the source of a `.star` file of a game with `settings`.
///
/// # Errors
///
/// As [`compile_source`].
pub fn compile_source_with(
    source: &str,
    settings: &GameSettings,
) -> Result<codegen::GeneratedProgram, Vec<CompileError>> {
    generate(&check_source_with(source, settings)?)
}

/// Compiles every file of a program of a game with `settings` into one
/// piece of Rust code.
///
/// # Errors
///
/// As [`compile_source`].
pub fn compile_program_with(
    sources: &SourceSet,
    settings: &GameSettings,
) -> Result<codegen::GeneratedProgram, Vec<CompileError>> {
    generate(&check_program_with(sources, settings)?)
}

fn generate(checked: &CheckedSource) -> Result<codegen::GeneratedProgram, Vec<CompileError>> {
    codegen::generate(&checked.syntax, &checked.analysis)
        .map_err(|error| vec![CompileError::Codegen(error)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(files: &[(&str, &str)]) -> SourceSet {
        let mut sources = SourceSet::new();
        for (name, text) in files {
            sources.add(*name, *text);
        }
        sources
    }

    fn reports(sources: &SourceSet, errors: &[CompileError]) -> Vec<String> {
        errors.iter().map(|error| sources.report(error)).collect()
    }

    #[test]
    fn classes_of_one_file_use_those_of_another() {
        let sources = program(&[
            (
                "main.star",
                "class Main {\n  static func main() {\n    Debug.print(Rock().size.to_string())\n  }\n}\n",
            ),
            ("rock.star", "class Rock {\n  var size = 3\n}\n"),
        ]);

        let checked = check_program_with(&sources, &GameSettings::default());

        assert!(checked.is_ok(), "{:?}", checked.err());
    }

    #[test]
    fn mistakes_are_reported_in_their_own_file() {
        let sources = program(&[
            ("a.star", "class A {\n  var x = @\n}\n"),
            ("b.star", "class B {\n  var y =\n}\n"),
        ]);

        let errors = parse_program(&sources).expect_err("both files are wrong");

        let reports = reports(&sources, &errors);
        assert_eq!(reports.len(), 2, "{reports:?}");
        assert!(
            reports[0].starts_with("a.star:2:11: error: "),
            "{reports:?}"
        );
        assert!(reports[1].starts_with("b.star:3:1: error: "), "{reports:?}");
    }

    #[test]
    fn an_unclosed_class_does_not_swallow_the_next_file() {
        let sources = program(&[("a.star", "class A {\n"), ("b.star", "class B {}\n")]);

        let errors = parse_program(&sources).expect_err("a.star is unclosed");

        assert!(
            reports(&sources, &errors)
                .iter()
                .all(|report| report.starts_with("a.star:")),
            "{errors:?}"
        );
    }

    #[test]
    fn semantic_mistakes_name_their_file() {
        let sources = program(&[("a.star", "class Same {}\n"), ("b.star", "class Same {}\n")]);

        let errors = check_program_with(&sources, &GameSettings::default())
            .expect_err("the class is declared twice");

        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            sources
                .report(&errors[0])
                .starts_with("b.star:1:7: error: ")
        );
    }

    #[test]
    fn a_program_of_one_file_compiles_as_that_file() {
        let source = "class Main {\n  static func main() {\n    Debug.print(\"1\")\n  }\n}\n";

        let from_file = compile_source_with(source, &GameSettings::default())
            .expect("valid program")
            .code;
        let from_program =
            compile_program_with(&program(&[("main.star", source)]), &GameSettings::default())
                .expect("valid program")
                .code;

        assert_eq!(from_file, from_program);
    }
}
