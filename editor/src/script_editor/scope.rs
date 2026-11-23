//! What names mean at a place of a script being written: the class being
//! written, what the chain of values before a `.` is, and the types of
//! variables. Suggestions, “Go to definition” and the types under the
//! pointer all ask it.
//!
//! The text is usually half written, so it is never parsed: only the
//! compiler's lexer (which keeps going after mistakes), the class table of
//! the last check that could read the program, and the types of the
//! variables that check found are used.

use std::ops::Range;

use star_script::semantic::{ClassId, ClassTable, LocalInfo, Type, basic_type, builtin_method};
use star_script::{Keyword, Token, TokenKind, scan};

use crate::project_check::ScriptLocals;

/// What the last check that could read the program knows, for one script.
#[derive(Clone, Copy, Default)]
pub(crate) struct Known<'known> {
    /// The classes of the program and the engine.
    pub(crate) classes: Option<&'known ClassTable>,
    /// The local variables of the script.
    pub(crate) locals: Option<&'known ScriptLocals>,
}

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

/// The tokens of `text`, without the end of the file the lexer adds.
fn tokens_of(text: &str) -> Vec<Token> {
    let mut tokens = scan(text).tokens;
    if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::EndOfFile)
    {
        tokens.pop();
    }
    tokens
}

/// A link of the chain of values before a `.`.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Link {
    /// A name: `rock`, `self`.
    Name(String),
    /// A call: `make(…)`.
    Call(String),
    /// An element of a list: `[…]`.
    Index,
    /// A value written as it is: `"a"`, `1.5`, `true`.
    Literal(Type),
}

/// The links of the chain before the `.` (or `?.`) that ends `tokens`, if
/// it ends with one: `self.friend?.make()[0].` is `self`, `friend`,
/// `make(…)` and an element. Empty if what is before the `.` is not a chain
/// whose value can be known (as `(a + b).`).
fn chain_before_dot(tokens: &[Token]) -> Option<Vec<Link>> {
    let (dot, mut rest) = tokens.split_last()?;
    if !matches!(dot.kind, TokenKind::Dot | TokenKind::QuestionDot) {
        return None;
    }
    let mut links = Vec::new();
    loop {
        let Some((link, before)) = last_link(rest) else {
            return Some(Vec::new());
        };
        let ends = matches!(link, Link::Literal(_));
        let indexed = link == Link::Index;
        links.push(link);
        rest = before;
        if ends {
            break;
        }
        if indexed {
            continue;
        }
        match rest.split_last() {
            Some((token, before))
                if matches!(token.kind, TokenKind::Dot | TokenKind::QuestionDot) =>
            {
                rest = before;
            }
            _ => break,
        }
    }
    links.reverse();
    Some(links)
}

/// The link that ends `tokens`, and the tokens before it.
fn last_link(tokens: &[Token]) -> Option<(Link, &[Token])> {
    let (last, before) = tokens.split_last()?;
    let link = match &last.kind {
        TokenKind::Identifier(name) => Link::Name(name.clone()),
        TokenKind::Keyword(Keyword::SelfValue) => Link::Name("self".to_owned()),
        TokenKind::Keyword(Keyword::Super) => Link::Name("super".to_owned()),
        TokenKind::Keyword(Keyword::True | Keyword::False) => Link::Literal(Type::Bool),
        TokenKind::String(_) => Link::Literal(Type::String),
        // Not integers: `1.` is how a decimal number starts being written.
        TokenKind::Float(_) => Link::Literal(Type::Float),
        TokenKind::RightParen => {
            let open = matching_open(before, &TokenKind::LeftParen, &TokenKind::RightParen)?;
            let (callee, before) = before[..open].split_last()?;
            let TokenKind::Identifier(name) = &callee.kind else {
                return None;
            };
            return Some((Link::Call(name.clone()), before));
        }
        TokenKind::RightBracket => {
            let open = matching_open(before, &TokenKind::LeftBracket, &TokenKind::RightBracket)?;
            let indexed = before[..open].last().is_some_and(|token| {
                matches!(
                    token.kind,
                    TokenKind::Identifier(_)
                        | TokenKind::RightParen
                        | TokenKind::RightBracket
                        | TokenKind::Keyword(Keyword::SelfValue)
                )
            });
            // Otherwise it is a list written as it is: `[1, 2].`.
            return indexed.then(|| (Link::Index, &before[..open]));
        }
        _ => return None,
    };
    Some((link, before))
}

/// Where the `open` token that the `close` token after `tokens` closes is.
fn matching_open(tokens: &[Token], open: &TokenKind, close: &TokenKind) -> Option<usize> {
    let mut depth = 0_usize;
    for (index, token) in tokens.iter().enumerate().rev() {
        if token.kind == *close {
            depth += 1;
        } else if token.kind == *open {
            if depth == 0 {
                return Some(index);
            }
            depth -= 1;
        }
    }
    None
}

/// What is known where a word is written.
pub(super) struct Scope<'text> {
    pub(super) classes: Option<&'text ClassTable>,
    locals: Option<&'text ScriptLocals>,
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

/// What the chain before a `.` is.
pub(super) enum Chained {
    /// An object of a class, or the class itself.
    Class(Receiver),
    /// A value of a basic type or a list, with methods of the language.
    Basic(Type),
}

impl<'text> Scope<'text> {
    /// What is known after `before`, from what the last check `knows`.
    pub(super) fn of(before: &str, knows: Known<'text>) -> Self {
        let tokens = tokens_of(before);
        let current = knows.classes.and_then(|classes| {
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
            classes: knows.classes,
            locals: knows.locals,
            tokens,
            current,
        }
    }

    /// Whether the word comes right after a `.` (or `?.`).
    pub(super) fn is_after_dot(&self) -> bool {
        self.tokens
            .last()
            .is_some_and(|token| matches!(token.kind, TokenKind::Dot | TokenKind::QuestionDot))
    }

    /// What the chain of values before the `.` the word comes after is, if
    /// it can be known.
    pub(super) fn chained(&self) -> Option<Chained> {
        let links = chain_before_dot(&self.tokens)?;
        let (first, rest) = links.split_first()?;
        let mut chained = self.first_link(first)?;
        for link in rest {
            chained = self.next_link(chained, link)?;
        }
        Some(chained)
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

    /// The local `name` the last check found declared at the byte `byte`
    /// of the text, if the text still declares that name there.
    pub(super) fn local_at(&self, byte: usize, name: &str) -> Option<&'text LocalInfo> {
        self.locals?.get(&byte).filter(|local| local.name == name)
    }

    /// The type of the variable `name` of the function being written: as
    /// the last check found it or, if the text changed since, as the text
    /// writes it.
    pub(super) fn variable_type(&self, name: &str) -> Option<Type> {
        let declaration = self.declaration_of(name, true)?;
        self.local_at(declaration, name)
            .map(|local| local.ty.clone())
            .or_else(|| self.written_type_of(name))
    }

    /// The type the text writes for the variable `name`, last before the
    /// word: `name: Type` (a variable or parameter) or `name = Class(…)`.
    fn written_type_of(&self, name: &str) -> Option<Type> {
        let tokens = &self.tokens;
        (0..tokens.len()).rev().find_map(|index| {
            if !matches!(&tokens[index].kind, TokenKind::Identifier(found) if found == name) {
                return None;
            }
            match tokens.get(index + 1)?.kind {
                TokenKind::Colon => self.written_type(tokens.get(index + 2..)?),
                TokenKind::Equal => match (tokens.get(index + 2), tokens.get(index + 3)) {
                    (
                        Some(Token {
                            kind: TokenKind::Identifier(class),
                            ..
                        }),
                        Some(Token {
                            kind: TokenKind::LeftParen,
                            ..
                        }),
                    ) => Some(Type::Class(self.classes?.find(class)?)),
                    _ => None,
                },
                _ => None,
            }
        })
    }

    /// The type written at the start of `tokens`: `int`, `Rock` or
    /// `[Rock]` (whether it may be `null` does not matter here).
    fn written_type(&self, tokens: &[Token]) -> Option<Type> {
        match &tokens.first()?.kind {
            TokenKind::Identifier(name) => {
                basic_type(name).or_else(|| Some(Type::Class(self.classes?.find(name)?)))
            }
            TokenKind::LeftBracket => Some(Type::list(self.written_type(&tokens[1..])?)),
            _ => None,
        }
    }

    /// What the first link of a chain is.
    fn first_link(&self, link: &Link) -> Option<Chained> {
        match link {
            Link::Literal(ty) => self.chained_value(ty),
            Link::Index => None,
            Link::Name(name) => self.value_named(name),
            Link::Call(name) => {
                let classes = self.classes?;
                if let Some(class) = classes.find(name) {
                    return self.chained_value(&Type::Class(class));
                }
                self.chained_value(member_type(classes, self.current?, name)?)
            }
        }
    }

    /// What the name at the start of a chain is.
    fn value_named(&self, name: &str) -> Option<Chained> {
        let classes = self.classes;
        match name {
            "self" => Some(self.object_of(self.current?)),
            "super" => Some(self.object_of(classes?.class(self.current?).base?)),
            _ => {
                if let Some(ty) = self.variable_type(name) {
                    return self.chained_value(&ty);
                }
                let classes = classes?;
                if let Some(class) = classes.find(name) {
                    return Some(Chained::Class(Receiver {
                        class,
                        statics: true,
                        private: Some(class) == self.current,
                    }));
                }
                self.chained_value(member_type(classes, self.current?, name)?)
            }
        }
    }

    /// What the link after `chained` in a chain is.
    fn next_link(&self, chained: Chained, link: &Link) -> Option<Chained> {
        match (chained, link) {
            (Chained::Class(receiver), Link::Name(name) | Link::Call(name)) => {
                self.chained_value(member_type(self.classes?, receiver.class, name)?)
            }
            (Chained::Basic(ty), Link::Call(name)) => {
                self.chained_value(&builtin_method(&ty, name)?.return_type)
            }
            (Chained::Basic(Type::List(element)), Link::Index) => self.chained_value(&element),
            _ => None,
        }
    }

    /// A value of type `ty`, `null` or not, if its members can be listed.
    fn chained_value(&self, ty: &Type) -> Option<Chained> {
        match ty {
            Type::Class(class) => Some(self.object_of(*class)),
            Type::Nullable(inner) => self.chained_value(inner),
            Type::Int | Type::Float | Type::Bool | Type::String | Type::List(_) => {
                Some(Chained::Basic(ty.clone()))
            }
            _ => None,
        }
    }

    /// An object of `class`.
    fn object_of(&self, class: ClassId) -> Chained {
        Chained::Class(Receiver {
            class,
            statics: false,
            private: Some(class) == self.current,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn chain(before: &str) -> Option<Vec<Link>> {
        chain_before_dot(&tokens_of(before))
    }

    fn name(text: &str) -> Link {
        Link::Name(text.to_owned())
    }

    #[test]
    fn chains_of_names_are_read() {
        assert_eq!(
            chain("x = self.friend?."),
            Some(vec![name("self"), name("friend")])
        );
    }

    #[test]
    fn calls_and_elements_are_links() {
        assert_eq!(
            chain("make(a, f(b)).items[i + 1]."),
            Some(vec![
                Link::Call("make".to_owned()),
                name("items"),
                Link::Index
            ])
        );
    }

    #[test]
    fn literals_start_chains() {
        assert_eq!(chain("\"a\"."), Some(vec![Link::Literal(Type::String)]));
        assert_eq!(chain("1.5."), Some(vec![Link::Literal(Type::Float)]));
    }

    #[test]
    fn unknown_values_before_a_dot_give_an_empty_chain() {
        assert_eq!(chain("(a + b)."), Some(Vec::new()));
        assert_eq!(chain("[1, 2]."), Some(Vec::new()));
        // A decimal number being written.
        assert_eq!(chain("x = 1."), Some(Vec::new()));
    }

    #[test]
    fn words_not_after_a_dot_have_no_chain() {
        assert_eq!(chain("var a = "), None);
    }
}
