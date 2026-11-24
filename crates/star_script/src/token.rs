//! The smallest meaningful pieces of StarScript code.

use std::fmt;

use crate::Span;

/// A piece of source code classified by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// What the piece of code is.
    pub kind: TokenKind,
    /// Where it is in the source.
    pub span: Span,
}

impl Token {
    /// Creates a token.
    #[must_use]
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Every kind of token, with the value of literals already decoded.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// A name such as `speed` or `Player`.
    Identifier(String),
    /// A word with a meaning in the language, such as `class`.
    Keyword(Keyword),
    /// Integer literal: `42`, `0xFF`, `0b1010`.
    Integer(i64),
    /// Floating-point literal: `3.14`, `1e3`.
    Float(f64),
    /// Text literal with its escapes resolved.
    String(String),

    /// `(`
    LeftParen,
    /// `)`
    RightParen,
    /// `{`
    LeftBrace,
    /// `}`
    RightBrace,
    /// `[`
    LeftBracket,
    /// `]`
    RightBracket,
    /// `,`
    Comma,
    /// `.`
    Dot,
    /// `..`
    DotDot,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `->`
    Arrow,
    /// `@`: starts a decorator.
    At,
    /// `?`
    Question,
    /// `?.`
    QuestionDot,
    /// `??`
    QuestionQuestion,

    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `!`
    Bang,
    /// `&&`
    AndAnd,
    /// `||`
    OrOr,

    /// `==`
    EqualEqual,
    /// `!=`
    BangEqual,
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,

    /// `=`
    Equal,
    /// `+=`
    PlusEqual,
    /// `-=`
    MinusEqual,
    /// `*=`
    StarEqual,
    /// `/=`
    SlashEqual,
    /// `%=`
    PercentEqual,

    /// End of a statement (one or more line breaks).
    Newline,
    /// End of the source; always the last token.
    EndOfFile,
}

impl TokenKind {
    /// The fixed spelling of punctuation and operators, `None` for the rest.
    #[must_use]
    pub const fn symbol(&self) -> Option<&'static str> {
        let symbol = match self {
            Self::LeftParen => "(",
            Self::RightParen => ")",
            Self::LeftBrace => "{",
            Self::RightBrace => "}",
            Self::LeftBracket => "[",
            Self::RightBracket => "]",
            Self::Comma => ",",
            Self::Dot => ".",
            Self::DotDot => "..",
            Self::Colon => ":",
            Self::Semicolon => ";",
            Self::Arrow => "->",
            Self::At => "@",
            Self::Question => "?",
            Self::QuestionDot => "?.",
            Self::QuestionQuestion => "??",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::Percent => "%",
            Self::Bang => "!",
            Self::AndAnd => "&&",
            Self::OrOr => "||",
            Self::EqualEqual => "==",
            Self::BangEqual => "!=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Equal => "=",
            Self::PlusEqual => "+=",
            Self::MinusEqual => "-=",
            Self::StarEqual => "*=",
            Self::SlashEqual => "/=",
            Self::PercentEqual => "%=",
            Self::Identifier(_)
            | Self::Keyword(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::String(_)
            | Self::Newline
            | Self::EndOfFile => return None,
        };
        Some(symbol)
    }
}

impl fmt::Display for TokenKind {
    /// Describes the token as error messages show it, e.g. ``identifier `x` ``.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identifier(name) => write!(formatter, "identifier `{name}`"),
            Self::Keyword(keyword) => write!(formatter, "keyword `{keyword}`"),
            Self::Integer(value) => write!(formatter, "integer `{value}`"),
            Self::Float(value) => write!(formatter, "number `{value:?}`"),
            Self::String(value) => write!(formatter, "string {value:?}"),
            Self::Newline => formatter.write_str("line break"),
            Self::EndOfFile => formatter.write_str("end of file"),
            symbol => write!(formatter, "`{}`", symbol.symbol().unwrap_or("?")),
        }
    }
}

/// Words with a meaning in the language. They cannot be used as names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    /// `class`
    Class,
    /// `extends`
    Extends,
    /// `init`
    Init,
    /// `self`
    SelfValue,
    /// `super`
    Super,
    /// `override`
    Override,
    /// `private`
    Private,
    /// `static`
    Static,
    /// `var`
    Var,
    /// `const`
    Const,
    /// `func`
    Func,
    /// `if`
    If,
    /// `else`
    Else,
    /// `while`
    While,
    /// `for`
    For,
    /// `in`
    In,
    /// `break`
    Break,
    /// `continue`
    Continue,
    /// `return`
    Return,
    /// `as`
    As,
    /// `is`
    Is,
    /// `true`
    True,
    /// `false`
    False,
    /// `null`
    Null,
    /// `async`: a method that can wait across frames.
    Async,
    /// `await`: waits for a method that can wait.
    Await,
    /// `parallel`: a function that runs on another thread.
    Parallel,
    /// `export`: a field of a node that scenes and the inspector set.
    Export,
    /// `interface`, reserved for the future.
    Interface,
    /// `implements`, reserved for the future.
    Implements,
    /// `abstract`, reserved for the future.
    Abstract,
    /// `enum`, reserved for the future.
    Enum,
    /// `match`, reserved for the future.
    Match,
    /// `signal`, reserved for the future.
    Signal,
}

impl Keyword {
    /// Every keyword, in the order the specification lists them.
    pub const ALL: [Self; 34] = [
        Self::Class,
        Self::Extends,
        Self::Init,
        Self::SelfValue,
        Self::Super,
        Self::Override,
        Self::Private,
        Self::Static,
        Self::Var,
        Self::Const,
        Self::Func,
        Self::If,
        Self::Else,
        Self::While,
        Self::For,
        Self::In,
        Self::Break,
        Self::Continue,
        Self::Return,
        Self::As,
        Self::Is,
        Self::True,
        Self::False,
        Self::Null,
        Self::Async,
        Self::Await,
        Self::Parallel,
        Self::Export,
        Self::Interface,
        Self::Implements,
        Self::Abstract,
        Self::Enum,
        Self::Match,
        Self::Signal,
    ];

    /// The keyword spelled `word`, if there is one.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|keyword| keyword.as_str() == word)
    }

    /// How the keyword is written.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Extends => "extends",
            Self::Init => "init",
            Self::SelfValue => "self",
            Self::Super => "super",
            Self::Override => "override",
            Self::Private => "private",
            Self::Static => "static",
            Self::Var => "var",
            Self::Const => "const",
            Self::Func => "func",
            Self::If => "if",
            Self::Else => "else",
            Self::While => "while",
            Self::For => "for",
            Self::In => "in",
            Self::Break => "break",
            Self::Continue => "continue",
            Self::Return => "return",
            Self::As => "as",
            Self::Is => "is",
            Self::True => "true",
            Self::False => "false",
            Self::Null => "null",
            Self::Async => "async",
            Self::Await => "await",
            Self::Parallel => "parallel",
            Self::Export => "export",
            Self::Interface => "interface",
            Self::Implements => "implements",
            Self::Abstract => "abstract",
            Self::Enum => "enum",
            Self::Match => "match",
            Self::Signal => "signal",
        }
    }

    /// Whether the keyword is reserved but has no meaning yet.
    #[must_use]
    pub const fn is_reserved(self) -> bool {
        matches!(
            self,
            Self::Interface
                | Self::Implements
                | Self::Abstract
                | Self::Enum
                | Self::Match
                | Self::Signal
        )
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_keyword_round_trips_through_its_spelling() {
        for keyword in Keyword::ALL {
            assert_eq!(Keyword::from_word(keyword.as_str()), Some(keyword));
        }
    }

    #[test]
    fn ordinary_words_are_not_keywords() {
        assert_eq!(Keyword::from_word("speed"), None);
        assert_eq!(Keyword::from_word("int"), None);
        assert_eq!(Keyword::from_word("Class"), None);
    }

    #[test]
    fn only_future_keywords_are_reserved() {
        assert!(Keyword::Match.is_reserved());
        assert!(!Keyword::Class.is_reserved());
    }

    #[test]
    fn tokens_describe_themselves_for_error_messages() {
        assert_eq!(
            TokenKind::Identifier("speed".into()).to_string(),
            "identifier `speed`"
        );
        assert_eq!(
            TokenKind::Keyword(Keyword::Func).to_string(),
            "keyword `func`"
        );
        assert_eq!(TokenKind::Arrow.to_string(), "`->`");
        assert_eq!(TokenKind::EndOfFile.to_string(), "end of file");
    }
}
