//! The type of the name under the pointer, written as code: `turns: int`
//! for a variable, `Rock.roll(speed: float) -> bool` for a member,
//! `class Rock extends Node` for a class.
//!
//! What names mean is the `scope` module's, as for “Go to definition”.
//! Members and classes come from the class table of the last check that
//! could read the program; the types of variables (even inferred ones),
//! from the locals that check found, by where their declarations are.

use std::ops::Range;

use star_script::semantic::{ClassId, ClassTable, LocalInfo, MemberRef, Type, builtin_method};

use super::completion::{builtin_signature, is_inside_comment_or_string, method_signature};
use super::scope::{Chained, Known, Scope, name_around};

/// A name of the text and what its type is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NameType {
    /// The characters of the name.
    pub(super) range: Range<usize>,
    /// Its type, written as code.
    pub(super) description: String,
}

/// The type of the name around the character `cursor` of `text`, from what
/// the last check `knows`. `None` outside names, in comments and strings,
/// and where the type is unknown.
pub(super) fn name_type_at(text: &str, cursor: usize, knows: Known<'_>) -> Option<NameType> {
    let bytes = name_around(text, cursor)?;
    let before = &text[..bytes.start];
    if is_inside_comment_or_string(before) {
        return None;
    }
    let name = &text[bytes.clone()];
    let description = describe(before, name, knows.classes?, knows)?;
    let start = before.chars().count();
    Some(NameType {
        range: start..start + name.chars().count(),
        description,
    })
}

/// What `name`, written after `before`, is.
fn describe(before: &str, name: &str, classes: &ClassTable, knows: Known<'_>) -> Option<String> {
    let scope = Scope::of(before, knows);
    if scope.is_after_dot() {
        return match scope.chained()? {
            Chained::Class(receiver) => member_description(classes, receiver.class, name),
            Chained::Basic(ty) => builtin_description(classes, &ty, name),
        };
    }
    // The name of a declaration.
    if let Some(local) = scope.local_at(before.len(), name) {
        return Some(local_description(classes, local));
    }
    if scope.declaration_of(name, true).is_some() {
        // A variable of the function hides any member with its name.
        let ty = scope.variable_type(name)?;
        return Some(format!("{name}: {}", classes.type_name(&ty)));
    }
    let class_of_self = match name {
        "self" => Some(scope.current),
        "super" => Some(
            scope
                .current
                .and_then(|current| classes.class(current).base),
        ),
        _ => None,
    };
    if let Some(class) = class_of_self {
        return Some(class_description(classes, class?));
    }
    scope
        .current
        .and_then(|current| member_description(classes, current, name))
        .or_else(|| {
            let class = classes.find(name)?;
            Some(class_description(classes, class))
        })
}

/// `string.length() -> int`: the method `name` of the language that values
/// of type `ty` have.
fn builtin_description(classes: &ClassTable, ty: &Type, name: &str) -> Option<String> {
    let method = builtin_method(ty, name)?;
    Some(format!(
        "{}.{name}{}",
        classes.type_name(ty),
        builtin_signature(classes, &method)
    ))
}

/// `turns: int`.
fn local_description(classes: &ClassTable, local: &LocalInfo) -> String {
    format!("{}: {}", local.name, classes.type_name(&local.ty))
}

/// `Rock.size: int` or `Rock.roll(speed: float) -> bool`, from the class
/// that declares the member `name` of `class`.
fn member_description(classes: &ClassTable, class: ClassId, name: &str) -> Option<String> {
    let found = classes.find_member(class, name)?;
    let declaring = classes.class(found.class);
    let owner = &declaring.name;
    let description = match found.member {
        MemberRef::Field(index) => {
            let field = declaring.fields.get(index)?;
            format!("{owner}.{name}: {}", classes.type_name(&field.ty))
        }
        MemberRef::Constant(index) => {
            let constant = declaring.constants.get(index)?;
            format!("{owner}.{name}: {}", classes.type_name(&constant.ty))
        }
        MemberRef::Method(index) => {
            let method = declaring.methods.get(index)?;
            format!("{owner}.{name}{}", method_signature(classes, method))
        }
    };
    Some(description)
}

/// `class Rock extends Node`.
fn class_description(classes: &ClassTable, class: ClassId) -> String {
    let info = classes.class(class);
    match info.base {
        Some(base) => format!("class {} extends {}", info.name, classes.class(base).name),
        None => format!("class {}", info.name),
    }
}

#[cfg(test)]
mod tests {
    use star_script::{GameSettings, SourceSet, analyze_program_with};

    use super::*;
    use crate::project_check::ScriptLocals;

    const ROCK: &str = "class Rock extends Node {\n  var size: int = 1\n  func roll(speed: float) -> bool {\n    var turns = 2\n    turns = size\n    return speed > 0.0 // turns\n  }\n}\n";
    const GAME: &str = "class Demo {\n  func play(rock: Rock) {\n    rock.roll(1.0)\n    var other = Rock()\n  }\n}\n";

    /// The classes of both files, and the locals of `rock.star`, which is
    /// first, so its places are the program's.
    fn program() -> (ClassTable, ScriptLocals) {
        let mut sources = SourceSet::new();
        sources.add("rock.star", ROCK);
        sources.add("game.star", GAME);
        let analysis = analyze_program_with(&sources, &GameSettings::default()).unwrap();
        let locals = analysis
            .declared_locals()
            .filter(|(span, _)| span.start() < ROCK.len())
            .map(|(span, local)| (span.start(), local.clone()))
            .collect();
        (analysis.classes().clone(), locals)
    }

    fn knows<'known>(classes: &'known ClassTable, locals: &'known ScriptLocals) -> Known<'known> {
        Known {
            classes: Some(classes),
            locals: Some(locals),
        }
    }

    /// The type of the name at the first `|` of `marked`, as `name_type_at`
    /// gives it with what the program knows.
    fn type_at(marked: &str) -> Option<String> {
        let cursor = marked.find('|').unwrap();
        let text = marked.replace('|', "");
        let (classes, locals) = program();
        let found = name_type_at(
            &text,
            text[..cursor].chars().count(),
            knows(&classes, &locals),
        )?;
        Some(found.description)
    }

    #[test]
    fn variables_show_their_inferred_type() {
        let marked = ROCK.replace("turns = size", "tu|rns = size");

        assert_eq!(type_at(&marked).as_deref(), Some("turns: int"));
    }

    #[test]
    fn declarations_and_parameters_show_their_type() {
        let declared = ROCK.replace("var turns = 2", "var tur|ns = 2");
        let parameter = ROCK.replace("speed > 0.0", "spe|ed > 0.0");

        assert_eq!(type_at(&declared).as_deref(), Some("turns: int"));
        assert_eq!(type_at(&parameter).as_deref(), Some("speed: float"));
    }

    #[test]
    fn members_show_their_class_and_type() {
        let field = ROCK.replace("turns = size", "turns = si|ze");
        let method = GAME.replace("rock.roll", "rock.ro|ll");

        assert_eq!(type_at(&field).as_deref(), Some("Rock.size: int"));
        assert_eq!(
            type_at(&method).as_deref(),
            Some("Rock.roll(speed: float) -> bool")
        );
    }

    #[test]
    fn methods_of_the_language_show_their_type() {
        let marked = ROCK.replace("turns = size", "turns = size.ma|x(3)");

        assert_eq!(type_at(&marked).as_deref(), Some("int.max(int) -> int"));
    }

    #[test]
    fn classes_show_their_base() {
        let marked = GAME.replace("var other = Rock()", "var other = Ro|ck()");

        assert_eq!(type_at(&marked).as_deref(), Some("class Rock extends Node"));
    }

    #[test]
    fn variables_unknown_to_the_check_show_their_written_class() {
        // `game.star`'s locals are not given: its text is newer than the
        // check.
        let marked = GAME.replace("rock.roll", "ro|ck.roll");

        assert_eq!(type_at(&marked).as_deref(), Some("rock: Rock"));
    }

    #[test]
    fn the_range_is_the_whole_name() {
        let (classes, locals) = program();
        let cursor = ROCK.find("size\n").unwrap() + 2;

        let found = name_type_at(ROCK, cursor, knows(&classes, &locals)).unwrap();

        assert_eq!(found.range, cursor - 2..cursor + 2);
    }

    #[test]
    fn comments_numbers_and_unknown_names_show_nothing() {
        let comment = ROCK.replace("// turns", "// tu|rns");
        let number = ROCK.replace("var turns = 2", "var turns = |2");

        assert_eq!(type_at(&comment), None);
        assert_eq!(type_at(&number), None);
        assert_eq!(type_at("class Rock { func f() { mis|sing } }"), None);
    }
}
