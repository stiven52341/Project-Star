//! Suggestions while writing a script: the names that may complete the
//! word at the cursor. After a `.`, the members of what is before it (its
//! type found from what the last check knows and the declarations in the
//! text), or the methods of the language for basic types and lists;
//! elsewhere, the names of the text, the members of the class
//! being written, the classes, keywords and basic types.
//! What names mean there is the `scope` module's.

use star_script::decorated::is_hidden_name;
use star_script::semantic::{
    BASIC_TYPE_NAMES, BuiltinSignature, ClassTable, Execution, MethodInfo, Type, builtin_methods,
};
use star_script::{Keyword, LexErrorKind, TokenKind, scan};

use super::scope::{Chained, Known, Receiver, Scope, trailing_word};

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
/// `text`, with what the last check `knows`. `None` if
/// there is no word being written there, it is inside a comment or a
/// string, or nothing completes it.
pub(super) fn complete(text: &str, cursor: usize, knows: Known<'_>) -> Option<Completion> {
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
    let scope = Scope::of(&before[..word_start], knows);
    let mut candidates = if scope.is_after_dot() {
        let classes = knows.classes?;
        match scope.chained()? {
            Chained::Class(receiver) => members(classes, receiver, false),
            Chained::Basic(ty) => builtins(classes, &ty),
        }
    } else if typed.is_empty() {
        return None;
    } else {
        names(&scope, text)
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
pub(super) fn is_inside_comment_or_string(before: &str) -> bool {
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

/// Every name that may be written where no `.` comes before.
fn names(scope: &Scope<'_>, text: &str) -> Vec<Candidate> {
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
    if let Some(classes) = scope.classes {
        candidates.extend(
            classes
                .iter()
                .map(|(_, class)| plain(class.name.clone(), CandidateKind::Class)),
        );
        if let Some(current) = scope.current {
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
        // The members the compiler made for decorated methods are not
        // written by anyone, so they are not offered.
        for field in class
            .fields
            .iter()
            .filter(|field| wanted(false, field.is_private) && !is_hidden_name(&field.name))
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
        for method in class.methods.iter().filter(|method| {
            wanted(method.is_static, method.is_private) && !is_hidden_name(&method.name)
        }) {
            candidates.push(Candidate {
                signature: Some(format!(
                    "{}{}",
                    execution_word(method),
                    method_signature(classes, method)
                )),
                takes_parameters: !method.parameters.is_empty(),
                ..plain(method.name.clone(), CandidateKind::Method)
            });
        }
    }
    candidates
}

/// The methods of the language that values of type `ty` have.
fn builtins(classes: &ClassTable, ty: &Type) -> Vec<Candidate> {
    builtin_methods(ty)
        .map(|signature| Candidate {
            signature: Some(builtin_signature(classes, &signature)),
            takes_parameters: !signature.parameters.is_empty(),
            ..plain(signature.method.name().to_owned(), CandidateKind::Method)
        })
        .collect()
}

/// The parameters of `method` and what it gives: `(speed: float) -> bool`.
pub(super) fn method_signature(classes: &ClassTable, method: &MethodInfo) -> String {
    let parameters = method
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, classes.type_name(&parameter.ty)));
    signature(classes, parameters, &method.return_type)
}

/// `async ` or `parallel ` before what is said of a method that runs so,
/// as it is written before its `func`; nothing for the others.
pub(super) const fn execution_word(method: &MethodInfo) -> &'static str {
    match method.execution {
        Execution::Direct => "",
        Execution::Async => "async ",
        Execution::Parallel => "parallel ",
    }
}

/// The parameter types of a method of the language and what it gives:
/// `(int) -> int`.
pub(super) fn builtin_signature(classes: &ClassTable, method: &BuiltinSignature) -> String {
    let parameters = method.parameters.iter().map(|ty| classes.type_name(ty));
    signature(classes, parameters, &method.return_type)
}

/// `(parameters) -> result`, without the result if there is none.
fn signature(
    classes: &ClassTable,
    parameters: impl Iterator<Item = String>,
    result: &Type,
) -> String {
    let parameters: Vec<String> = parameters.collect();
    let result = match result {
        Type::Void => String::new(),
        ty => format!(" -> {}", classes.type_name(ty)),
    };
    format!("({}){result}", parameters.join(", "))
}

#[cfg(test)]
mod tests {
    use star_script::{GameSettings, SourceSet, analyze_program_with};

    use super::*;

    const PROGRAM: &str = "class Rock extends Node {\n  var size: int = 1\n  private var secret = 2\n  var friend: Rock? = null\n  static func make(size: int) -> Rock {\n    return Rock()\n  }\n  func roll() {}\n  @Once\n  func knock() {}\n}\nclass Once extends Decorator {}\n";

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
        let knows = Known {
            classes: Some(&classes),
            locals: None,
        };
        complete(&plain, plain[..cursor].chars().count(), knows)
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
    fn members_made_for_decorators_are_not_offered() {
        let names = suggested(&inside_rock("self.|"));

        assert!(names.contains(&"knock".to_owned()));
        assert!(names.iter().all(|name| !name.contains('@')), "{names:?}");
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

        // `rotation` and `rotate` come from `Node`.
        assert_eq!(names, ["rotation", "roll", "rotate"]);
    }

    #[test]
    fn chains_follow_what_calls_give() {
        let static_call = suggested(&inside_rock("Rock.make(1).si|"));
        let constructor = suggested(&inside_rock("Rock().fri|"));

        assert_eq!(static_call, ["size"]);
        assert_eq!(constructor, ["friend"]);
    }

    #[test]
    fn basic_values_list_the_methods_of_the_language() {
        let text = suggested(&inside_rock("\"rock\".to_|"));
        let number = suggested(&inside_rock("self.size.|"));

        assert_eq!(text, ["to_lower", "to_upper", "to_string"]);
        assert_eq!(number, ["abs", "max", "min", "to_string"]);
    }

    #[test]
    fn lists_list_their_methods_and_their_elements_their_members() {
        let list = suggested(&inside_rock("var rocks: [Rock] = []\n    rocks.p|"));
        let element = suggested(&inside_rock("var rocks: [Rock] = []\n    rocks[0].si|"));

        assert_eq!(list, ["pop", "push"]);
        assert_eq!(element, ["size"]);
    }

    #[test]
    fn inferred_types_come_from_the_check() {
        let marked = inside_rock("var word = \"a\".to_upper()\n    word.is_|");
        let cursor = marked.find('|').unwrap();
        let text = marked.replace('|', "");
        let mut sources = SourceSet::new();
        sources.add("rock.star", text.as_str());
        let analysis = analyze_program_with(&sources, &GameSettings::default()).unwrap();
        let locals = analysis
            .declared_locals()
            .map(|(span, local)| (span.start(), local.clone()))
            .collect();
        let knows = Known {
            classes: Some(analysis.classes()),
            locals: Some(&locals),
        };

        let completion = complete(&text, cursor, knows).unwrap();

        assert_eq!(completion.candidates[0].name, "is_empty");
        assert_eq!(
            completion.candidates[0].signature.as_deref(),
            Some("() -> bool")
        );
    }

    #[test]
    fn methods_that_wait_or_run_in_parallel_say_so() {
        let classes = classes();
        let knows = Known {
            classes: Some(&classes),
            locals: None,
        };
        let text = inside_rock("Time.wa");

        let completion = complete(&text, text.find("wa").unwrap() + 2, knows).unwrap();

        assert_eq!(completion.candidates[0].name, "wait");
        assert_eq!(
            completion.candidates[0].signature.as_deref(),
            Some("async (seconds: float)")
        );
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
