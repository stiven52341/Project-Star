//! Suggestions while writing a script: the names that may complete the
//! word at the cursor. After a `.`, the members of what is before it (its
//! type found from the classes of the last check and the declarations in
//! the text); elsewhere, the names of the text, the members of the class
//! being written, the classes, keywords and basic types.
//!
//! The text is usually half written while suggestions are needed, so it is
//! never parsed: only the compiler's lexer (which keeps going after
//! mistakes) and the class table of the last check that could read the
//! program are used.

use star_script::semantic::{BASIC_TYPE_NAMES, ClassId, ClassTable, Type};
use star_script::{Keyword, LexErrorKind, Token, TokenKind, scan};

const LINE_COMMENT: &str = "//";

/// What a suggested name is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CandidateKind {
    /// A variable, field or constant.
    Value,
    Method,
    /// A name written somewhere in the text.
    Name,
    Class,
    Keyword,
    BasicType,
}

/// A name that may complete the word at the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Candidate {
    pub(super) name: String,
    pub(super) kind: CandidateKind,
    /// A member's type, or a method's parameters and result.
    pub(super) signature: Option<String>,
    /// Whether it is a method that takes parameters.
    takes_parameters: bool,
}

impl Candidate {
    /// A name written in the text.
    #[cfg(test)]
    pub(super) fn named(name: &str) -> Self {
        plain(name.to_owned(), CandidateKind::Name)
    }

    /// What accepting it writes in place of the word, and where the cursor
    /// goes then, in characters from the word's start: a method gets its
    /// parentheses, with the cursor inside if it takes parameters.
    pub(super) fn insertion(&self) -> (String, usize) {
        let length = self.name.chars().count();
        match self.kind {
            CandidateKind::Method if self.takes_parameters => {
                (format!("{}()", self.name), length + 1)
            }
            CandidateKind::Method => (format!("{}()", self.name), length + 2),
            _ => (self.name.clone(), length),
        }
    }
}

/// The suggestions for the word at the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Completion {
    /// Where the word starts, as a character index.
    pub(super) start: usize,
    /// What is typed of it.
    pub(super) typed: String,
    /// The candidates, the most likely first.
    pub(super) candidates: Vec<Candidate>,
}

/// The suggestions for the word that ends at the character `cursor` of
/// `text`, with the `classes` of the program if they are known. `None` if
/// there is no word being written there, it is inside a comment or a
/// string, or nothing completes it.
pub(super) fn complete(
    text: &str,
    cursor: usize,
    classes: Option<&ClassTable>,
) -> Option<Completion> {
    let byte = text
        .char_indices()
        .nth(cursor)
        .map_or(text.len(), |(byte, _)| byte);
    let before = &text[..byte];
    if is_inside_comment_or_string(before) {
        return None;
    }
    let typed = trailing_word(before);
    if typed.starts_with(|character: char| character.is_ascii_digit()) {
        return None;
    }
    let word_start = before.len() - typed.len();
    let context = Context::of(&before[..word_start], classes);
    let mut candidates = match receiver_chain(&before[..word_start]) {
        Some(chain) => context.members_of_chain(&chain)?,
        None if typed.is_empty() => return None,
        None => context.names(text),
    };
    candidates.retain(|candidate| candidate.name.starts_with(typed) && candidate.name != typed);
    candidates.sort_by(|first, second| {
        (first.kind, first.name.len(), &first.name).cmp(&(
            second.kind,
            second.name.len(),
            &second.name,
        ))
    });
    candidates.dedup_by(|later, earlier| later.name == earlier.name);
    if candidates.is_empty() {
        return None;
    }
    Some(Completion {
        start: before[..word_start].chars().count(),
        typed: typed.to_owned(),
        candidates,
    })
}

/// Whether the end of `before` is inside a comment or a string.
fn is_inside_comment_or_string(before: &str) -> bool {
    let found = scan(before);
    let in_comment = found.comments.last().is_some_and(|span| {
        let comment = span.text(before);
        span.end() == before.len()
            && (comment.starts_with(LINE_COMMENT) || !comment.ends_with("*/"))
    });
    let in_string = found.errors.iter().any(|error| {
        error.kind == LexErrorKind::UnterminatedString && error.span.end() == before.len()
    });
    in_comment || in_string
}

/// The name characters at the end of `text`.
fn trailing_word(text: &str) -> &str {
    let start = text
        .char_indices()
        .rev()
        .take_while(|(_, character)| is_name_character(*character))
        .last()
        .map_or(text.len(), |(index, _)| index);
    &text[start..]
}

const fn is_name_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

/// The names of `a.b.` (or `a?.b?.`) at the end of `before`, if it ends
/// with a `.` after names; empty if what is before the `.` is not a name.
fn receiver_chain(before: &str) -> Option<Vec<&str>> {
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

/// What is known where the word is written.
struct Context<'text> {
    classes: Option<&'text ClassTable>,
    /// The tokens before the word.
    tokens: Vec<Token>,
    /// The class being written, if known.
    current: Option<ClassId>,
}

/// The class whose members a `.` lists, and which of them.
#[derive(Clone, Copy)]
struct Receiver {
    class: ClassId,
    /// Whether it is the class itself (`Rock.`), whose static members are
    /// listed, or one of its objects.
    statics: bool,
    /// Whether its private members are seen.
    private: bool,
}

impl<'text> Context<'text> {
    fn of(before: &str, classes: Option<&'text ClassTable>) -> Self {
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

    /// Every name that may be written where no `.` comes before.
    fn names(&self, text: &str) -> Vec<Candidate> {
        let mut candidates: Vec<Candidate> = scan(text)
            .tokens
            .into_iter()
            .filter_map(|token| match token.kind {
                TokenKind::Identifier(name) => Some(plain(name, CandidateKind::Name)),
                _ => None,
            })
            .collect();
        candidates.extend(
            Keyword::ALL
                .iter()
                .filter(|keyword| !keyword.is_reserved())
                .map(|keyword| plain(keyword.as_str().to_owned(), CandidateKind::Keyword)),
        );
        candidates.extend(
            BASIC_TYPE_NAMES
                .iter()
                .map(|name| plain((*name).to_owned(), CandidateKind::BasicType)),
        );
        if let Some(classes) = self.classes {
            candidates.extend(
                classes
                    .iter()
                    .map(|(_, class)| plain(class.name.clone(), CandidateKind::Class)),
            );
            if let Some(current) = self.current {
                let own = Receiver {
                    class: current,
                    statics: false,
                    private: true,
                };
                candidates.extend(members(classes, own, true));
            }
        }
        candidates
    }

    /// The members of what `chain` names, if its type is known.
    fn members_of_chain(&self, chain: &[&str]) -> Option<Vec<Candidate>> {
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
        Some(members(classes, receiver, false))
    }

    /// What the name at the start of a chain is.
    fn receiver_named(&self, classes: &ClassTable, name: &str) -> Option<Receiver> {
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
    fn declared_class_of(&self, classes: &ClassTable, name: &str) -> Option<ClassId> {
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

/// A candidate without a signature.
const fn plain(name: String, kind: CandidateKind) -> Candidate {
    Candidate {
        name,
        kind,
        signature: None,
        takes_parameters: false,
    }
}

/// The members `receiver` offers, from its class and its bases; with
/// `everything`, static and not (as the class's own code sees them).
fn members(classes: &ClassTable, receiver: Receiver, everything: bool) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    for id in classes.ancestors(receiver.class) {
        let class = classes.class(id);
        let wanted = |is_static: bool, is_private: bool| {
            (everything || is_static == receiver.statics) && (receiver.private || !is_private)
        };
        for field in class
            .fields
            .iter()
            .filter(|field| wanted(false, field.is_private))
        {
            candidates.push(Candidate {
                signature: Some(classes.type_name(&field.ty)),
                ..plain(field.name.clone(), CandidateKind::Value)
            });
        }
        for constant in class
            .constants
            .iter()
            .filter(|constant| wanted(true, constant.is_private))
        {
            candidates.push(Candidate {
                signature: Some(classes.type_name(&constant.ty)),
                ..plain(constant.name.clone(), CandidateKind::Value)
            });
        }
        for method in class
            .methods
            .iter()
            .filter(|method| wanted(method.is_static, method.is_private))
        {
            let parameters: Vec<String> = method
                .parameters
                .iter()
                .map(|parameter| {
                    format!("{}: {}", parameter.name, classes.type_name(&parameter.ty))
                })
                .collect();
            let result = match method.return_type {
                Type::Void => String::new(),
                ref ty => format!(" -> {}", classes.type_name(ty)),
            };
            candidates.push(Candidate {
                signature: Some(format!("({}){result}", parameters.join(", "))),
                takes_parameters: !method.parameters.is_empty(),
                ..plain(method.name.clone(), CandidateKind::Method)
            });
        }
    }
    candidates
}

/// The type of the member `name` of `class` (or its bases): a field's or
/// constant's type, or what a method gives.
fn member_type<'table>(
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
fn class_of(ty: &Type) -> Option<ClassId> {
    match ty {
        Type::Class(class) => Some(*class),
        Type::Nullable(inner) => class_of(inner),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use star_script::{GameSettings, SourceSet, analyze_program_with};

    use super::*;

    const PROGRAM: &str = "class Rock extends Node {\n  var size: int = 1\n  private var secret = 2\n  var friend: Rock? = null\n  static func make(size: int) -> Rock {\n    return Rock()\n  }\n  func roll() {}\n}\n";

    fn classes() -> ClassTable {
        let mut sources = SourceSet::new();
        sources.add("rock.star", PROGRAM);
        analyze_program_with(&sources, &GameSettings::default())
            .unwrap()
            .classes()
            .clone()
    }

    /// The names suggested for `text` with the cursor at its `|`.
    fn suggested(text: &str) -> Vec<String> {
        let cursor = text.find('|').unwrap();
        let plain = text.replace('|', "");
        let classes = classes();
        complete(&plain, plain[..cursor].chars().count(), Some(&classes))
            .map(|completion| {
                completion
                    .candidates
                    .into_iter()
                    .map(|candidate| candidate.name)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn inside_rock(code: &str) -> String {
        format!("class Rock extends Node {{\n  func test() {{\n    {code}\n  }}\n}}\n")
    }

    #[test]
    fn names_of_the_text_and_keywords_complete_words() {
        let names = suggested(&inside_rock("var counter = 0\n    cou|"));

        assert_eq!(names.first().map(String::as_str), Some("counter"));
        assert!(suggested("cl|").contains(&"class".to_owned()));
    }

    #[test]
    fn members_of_self_are_listed_after_a_dot() {
        let names = suggested(&inside_rock("self.|"));

        assert!(names.contains(&"size".to_owned()));
        assert!(names.contains(&"secret".to_owned()));
        assert!(names.contains(&"roll".to_owned()));
        // Inherited from `Node`.
        assert!(names.contains(&"add_child".to_owned()));
        assert!(!names.contains(&"make".to_owned()));
    }

    #[test]
    fn classes_list_their_static_members() {
        let names = suggested(&inside_rock("Rock.ma|"));

        assert_eq!(names, ["make"]);
    }

    #[test]
    fn variables_list_the_members_of_their_class() {
        let typed = suggested("class Other {\n  func f(rock: Rock) {\n    rock.si|\n  }\n}\n");
        let built =
            suggested("class Other {\n  func f() {\n    var rock = Rock()\n    rock.|\n  }\n}\n");

        assert_eq!(typed, ["size"]);
        assert!(built.contains(&"roll".to_owned()));
        // Private members are not seen from another class.
        assert!(!built.contains(&"secret".to_owned()));
    }

    #[test]
    fn chains_follow_the_types_of_fields() {
        let names = suggested(&inside_rock("self.friend?.ro|"));

        // `rotation` comes from `Node`.
        assert_eq!(names, ["rotation", "roll"]);
    }

    #[test]
    fn nothing_is_suggested_in_comments_strings_or_numbers() {
        assert_eq!(suggested(&inside_rock("// cou|")), Vec::<String>::new());
        assert_eq!(
            suggested(&inside_rock("var a = \"cl|")),
            Vec::<String>::new()
        );
        assert_eq!(suggested(&inside_rock("var a = 1|")), Vec::<String>::new());
    }

    #[test]
    fn methods_are_written_with_their_parentheses() {
        let make = Candidate {
            takes_parameters: true,
            ..plain("make".to_owned(), CandidateKind::Method)
        };
        let roll = plain("roll".to_owned(), CandidateKind::Method);

        assert_eq!(make.insertion(), ("make()".to_owned(), 5));
        assert_eq!(roll.insertion(), ("roll()".to_owned(), 6));
    }
}
