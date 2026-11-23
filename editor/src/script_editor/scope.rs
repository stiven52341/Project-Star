//! What names mean at a place of a script being written: the class being
//! written, what is before a `.`, and the types of variables as the text
//! declares them. Suggestions and “Go to definition” both ask it.
//!
//! The text is usually half written, so it is never parsed: only the
//! compiler's lexer (which keeps going after mistakes) and the class table
//! of the last check that could read the program are used.

use std::ops::Range;

use star_script::semantic::{ClassId, ClassTable, Type};
use star_script::{Keyword, Token, TokenKind, scan};

/// The name characters at the end of `text`.
pub(super) fn trailing_word(text: &str) -> &str {
    let start = text
        .char_indices()
        .rev()
        .take_while(|(_, character)| is_name_character(*character))
        .last()
        .map_or(text.len(), |(index, _)| index);
    &text[start..]
}

/// The bytes of the name around the character `cursor` of `text` (before
/// it, after it, or both), if there is one: numbers are not names.
pub(super) fn name_around(text: &str, cursor: usize) -> Option<Range<usize>> {
    let byte = text
        .char_indices()
        .nth(cursor)
        .map_or(text.len(), |(byte, _)| byte);
    let start = byte - trailing_word(&text[..byte]).len();
    let end = byte
        + text[byte..]
            .find(|character: char| !is_name_character(character))
            .unwrap_or(text.len() - byte);
    let name = &text[start..end];
    let is_name =
        !name.is_empty() && !name.starts_with(|character: char| character.is_ascii_digit());
    is_name.then_some(start..end)
}

pub(super) const fn is_name_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

/// The names of `a.b.` (or `a?.b?.`) at the end of `before`, if it ends
/// with a `.` after names; empty if what is before the `.` is not a name.
pub(super) fn receiver_chain(before: &str) -> Option<Vec<&str>> {
    let mut rest = before.strip_suffix('.')?;
    let mut chain = Vec::new();
    loop {
        rest = rest.strip_suffix('?').unwrap_or(rest);
        let name = trailing_word(rest);
        if name.is_empty() {
            break;
        }
        chain.push(name);
        rest = &rest[..rest.len() - name.len()];
        match rest.strip_suffix('.') {
            Some(shorter) => rest = shorter,
            None => break,
        }
    }
    chain.reverse();
    Some(chain)
}

/// What is known where a word is written.
pub(super) struct Scope<'text> {
    pub(super) classes: Option<&'text ClassTable>,
    /// The tokens before the word.
    tokens: Vec<Token>,
    /// The class being written, if known.
    pub(super) current: Option<ClassId>,
}

/// The class whose members a `.` lists, and which of them.
#[derive(Clone, Copy)]
pub(super) struct Receiver {
    pub(super) class: ClassId,
    /// Whether it is the class itself (`Rock.`), whose static members are
    /// listed, or one of its objects.
    pub(super) statics: bool,
    /// Whether its private members are seen.
    pub(super) private: bool,
}

impl<'text> Scope<'text> {
    /// What is known after `before`, with the program's `classes`.
    pub(super) fn of(before: &str, classes: Option<&'text ClassTable>) -> Self {
        let tokens = scan(before).tokens;
        let current = classes.and_then(|classes| {
            let name = tokens.windows(2).rev().find_map(|pair| match pair {
                [
                    Token {
                        kind: TokenKind::Keyword(Keyword::Class),
                        ..
                    },
                    Token {
                        kind: TokenKind::Identifier(name),
                        ..
                    },
                ] => Some(name),
                _ => None,
            })?;
            classes.find(name)
        });
        Self {
            classes,
            tokens,
            current,
        }
    }

    /// Where the text before the word declares the variable `name` last,
    /// as a byte offset: after `var`, `const` or `for`, or as a parameter
    /// (after `(` or `,`, and before `:`). With `in_function`, only inside
    /// the function or constructor being written.
    pub(super) fn declaration_of(&self, name: &str, in_function: bool) -> Option<usize> {
        let first = if in_function {
            self.tokens.iter().rposition(|token| {
                matches!(
                    token.kind,
                    TokenKind::Keyword(Keyword::Func | Keyword::Init)
                )
            })?
        } else {
            0
        };
        let tokens = &self.tokens[first..];
        (1..tokens.len()).rev().find_map(|index| {
            let token = &tokens[index];
            if !matches!(&token.kind, TokenKind::Identifier(found) if found == name) {
                return None;
            }
            let after_keyword = matches!(
                tokens[index - 1].kind,
                TokenKind::Keyword(Keyword::Var | Keyword::Const | Keyword::For)
            );
            let parameter = matches!(
                tokens[index - 1].kind,
                TokenKind::LeftParen | TokenKind::Comma
            ) && tokens
                .get(index + 1)
                .is_some_and(|next| next.kind == TokenKind::Colon);
            (after_keyword || parameter).then_some(token.span.start())
        })
    }

    /// What `chain` names, if its type is known.
    pub(super) fn receiver_of_chain(&self, chain: &[&str]) -> Option<Receiver> {
        let classes = self.classes?;
        let (first, rest) = chain.split_first()?;
        let mut receiver = self.receiver_named(classes, first)?;
        for name in rest {
            let member = classes.find_member(receiver.class, name)?;
            let ty = member_type(classes, member.class, name)?;
            receiver = Receiver {
                class: class_of(ty)?,
                statics: false,
                private: Some(member.class) == self.current,
            };
        }
        Some(receiver)
    }

    /// What the name at the start of a chain is.
    pub(super) fn receiver_named(&self, classes: &ClassTable, name: &str) -> Option<Receiver> {
        let object = |class: ClassId| Receiver {
            class,
            statics: false,
            private: Some(class) == self.current,
        };
        match name {
            "self" => self.current.map(object),
            "super" => self
                .current
                .and_then(|current| classes.class(current).base)
                .map(object),
            _ => {
                if let Some(class) = self.declared_class_of(classes, name) {
                    return Some(object(class));
                }
                if let Some(class) = classes.find(name) {
                    return Some(Receiver {
                        class,
                        statics: true,
                        private: Some(class) == self.current,
                    });
                }
                let current = self.current?;
                let ty = member_type(classes, current, name)?;
                class_of(ty).map(object)
            }
        }
    }

    /// The class of the variable `name` as the text declares it last before
    /// the word: `name: Class` (a variable or parameter) or
    /// `var name = Class(…)`.
    pub(super) fn declared_class_of(&self, classes: &ClassTable, name: &str) -> Option<ClassId> {
        let is_name =
            |token: &Token| matches!(&token.kind, TokenKind::Identifier(found) if found == name);
        self.tokens.windows(4).rev().find_map(|window| {
            let class = match window {
                [
                    variable,
                    colon,
                    Token {
                        kind: TokenKind::Identifier(class),
                        ..
                    },
                    _,
                ] if is_name(variable) && colon.kind == TokenKind::Colon => class,
                [
                    variable,
                    Token {
                        kind: TokenKind::Equal,
                        ..
                    },
                    Token {
                        kind: TokenKind::Identifier(class),
                        ..
                    },
                    Token {
                        kind: TokenKind::LeftParen,
                        ..
                    },
                ] if is_name(variable) => class,
                _ => return None,
            };
            classes.find(class)
        })
    }
}

/// The type of the member `name` of `class` (or its bases): a field's or
/// constant's type, or what a method gives.
pub(super) fn member_type<'table>(
    classes: &'table ClassTable,
    class: ClassId,
    name: &str,
) -> Option<&'table Type> {
    classes.ancestors(class).find_map(|id| {
        let class = classes.class(id);
        class
            .fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| &field.ty)
            .or_else(|| {
                class
                    .constants
                    .iter()
                    .find(|constant| constant.name == name)
                    .map(|constant| &constant.ty)
            })
            .or_else(|| {
                class
                    .methods
                    .iter()
                    .find(|method| method.name == name)
                    .map(|method| &method.return_type)
            })
    })
}

/// The class of objects of `ty`, `null` or not.
pub(super) fn class_of(ty: &Type) -> Option<ClassId> {
    match ty {
        Type::Class(class) => Some(*class),
        Type::Nullable(inner) => class_of(inner),
        _ => None,
    }
}
