//! The type of the name under the pointer, written as code: `turns: int`
//! for a variable, `Rock.roll(speed: float) -> bool` for a member,
//! `class Rock extends Node` for a class.
//!
//! What names mean is the `scope` module's, as for “Go to definition”.
//! Members and classes come from the class table of the last check that
//! could read the program; the types of variables (even inferred ones),
//! from the locals that check found, by where their declarations are.

use std::ops::Range;

use star_script::semantic::{ClassId, ClassTable, LocalInfo, MemberRef};

use super::completion::{is_inside_comment_or_string, method_signature};
use super::scope::{Scope, name_around, receiver_chain};
use crate::project_check::ScriptLocals;

/// A name of the text and what its type is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NameType {
    /// The characters of the name.
    pub(super) range: Range<usize>,
    /// Its type, written as code.
    pub(super) description: String,
}

/// The type of the name around the character `cursor` of `text`, from the
/// program's `classes` and the `locals` of this script found by the last
/// check. `None` outside names, in comments and strings, and where the
/// type is unknown.
pub(super) fn name_type_at(
    text: &str,
    cursor: usize,
    classes: Option<&ClassTable>,
    locals: Option<&ScriptLocals>,
) -> Option<NameType> {
    let bytes = name_around(text, cursor)?;
    let before = &text[..bytes.start];
    if is_inside_comment_or_string(before) {
        return None;
    }
    let name = &text[bytes.clone()];
    let description = describe(before, name, classes?, locals)?;
    let start = before.chars().count();
    Some(NameType {
        range: start..start + name.chars().count(),
        description,
    })
}

/// What `name`, written after `before`, is.
fn describe(
    before: &str,
    name: &str,
    classes: &ClassTable,
    locals: Option<&ScriptLocals>,
) -> Option<String> {
    let scope = Scope::of(before, Some(classes));
    if let Some(chain) = receiver_chain(before) {
        let receiver = scope.receiver_of_chain(&chain)?;
        return member_description(classes, receiver.class, name);
    }
    let local_at = |byte: usize| {
        locals
            .and_then(|locals| locals.get(&byte))
            .filter(|local| local.name == name)
    };
    // The name of a declaration.
    if let Some(local) = local_at(before.len()) {
        return Some(local_description(classes, local));
    }
    if let Some(declaration) = scope.declaration_of(name, true) {
        // A variable of the function hides any member with its name.
        return local_at(declaration)
            .map(|local| local_description(classes, local))
            .or_else(|| {
                let class = scope.declared_class_of(classes, name)?;
                Some(format!("{name}: {}", classes.class(class).name))
            });
    }
    if matches!(name, "self" | "super") {
        let receiver = scope.receiver_named(classes, name)?;
        return Some(class_description(classes, receiver.class));
    }
    scope
        .current
        .and_then(|current| member_description(classes, current, name))
        .or_else(|| {
            let class = classes.find(name)?;
            Some(class_description(classes, class))
        })
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

    /// The type of the name at the first `|` of `marked`, as `name_type_at`
    /// gives it with what the program knows.
    fn type_at(marked: &str) -> Option<String> {
        let cursor = marked.find('|').unwrap();
        let text = marked.replace('|', "");
        let (classes, locals) = program();
        let found = name_type_at(
            &text,
            text[..cursor].chars().count(),
            Some(&classes),
            Some(&locals),
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

        let found = name_type_at(ROCK, cursor, Some(&classes), Some(&locals)).unwrap();

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
