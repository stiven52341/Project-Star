//! “Go to definition”: where the name at the cursor is declared. A
//! variable of the function being written is found in the text; members
//! and classes, in the class table of the last check that could read the
//! program, whose places are offsets of the whole program.

use star_script::Span;
use star_script::semantic::{ClassId, ClassOrigin, ClassTable, MemberRef};

use super::scope::{Scope, is_name_character, receiver_chain, trailing_word};

/// Where a name is declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Definition {
    /// In the same text, at this character index.
    InText(usize),
    /// At this place of the program (any of its files).
    InProgram(Span),
}

/// Where the name around the character `cursor` of `text` is declared,
/// with the program's `classes` if they are known. `None` if there is no
/// name there, or its declaration is unknown or in the engine.
pub(super) fn definition_at(
    text: &str,
    cursor: usize,
    classes: Option<&ClassTable>,
) -> Option<Definition> {
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
    if name.is_empty() || name.starts_with(|character: char| character.is_ascii_digit()) {
        return None;
    }
    let before = &text[..start];
    let scope = Scope::of(before, classes);
    let in_text = |byte: usize| Definition::InText(text[..byte].chars().count());
    if let Some(chain) = receiver_chain(before) {
        let receiver = scope.receiver_of_chain(&chain)?;
        return member_definition(classes?, receiver.class, name);
    }
    if let Some(byte) = scope.declaration_of(name, true) {
        return Some(in_text(byte));
    }
    let declared = classes.and_then(|classes| {
        scope
            .current
            .and_then(|current| member_definition(classes, current, name))
            .or_else(|| class_definition(classes, name))
    });
    declared.or_else(|| scope.declaration_of(name, false).map(in_text))
}

/// Where the member `name` of `class` (or of its closest base that has
/// one) is declared, if a script declares it.
fn member_definition(classes: &ClassTable, class: ClassId, name: &str) -> Option<Definition> {
    let found = classes.find_member(class, name)?;
    let declaring = classes.class(found.class);
    if declaring.origin == ClassOrigin::Native {
        return None;
    }
    let span = match found.member {
        MemberRef::Field(index) => declaring.fields.get(index)?.span,
        MemberRef::Constant(index) => declaring.constants.get(index)?.span,
        MemberRef::Method(index) => declaring.methods.get(index)?.span?,
    };
    Some(Definition::InProgram(span))
}

/// Where the class `name` is declared, if a script declares it.
fn class_definition(classes: &ClassTable, name: &str) -> Option<Definition> {
    let class = classes.class(classes.find(name)?);
    match class.origin {
        ClassOrigin::Script { name, .. } => Some(Definition::InProgram(name)),
        ClassOrigin::Native => None,
    }
}

#[cfg(test)]
mod tests {
    use star_script::{GameSettings, SourceSet, analyze_program_with};

    use super::*;

    const ROCK: &str = "class Rock extends Node {\n  var size: int = 1\n  func roll(speed: float) {\n    var turns = 2\n    turns = size\n  }\n}\n";
    const GAME: &str = "class Demo {\n  func play(rock: Rock) {\n    rock.roll(1.0)\n    var other = Rock()\n  }\n}\n";

    /// The program of both files, `rock.star` first.
    fn classes() -> ClassTable {
        let mut sources = SourceSet::new();
        sources.add("rock.star", ROCK);
        sources.add("game.star", GAME);
        analyze_program_with(&sources, &GameSettings::default())
            .unwrap()
            .classes()
            .clone()
    }

    /// The text declaring the name at the first `|` of `marked` (which is
    /// `text` with the cursor), found in `text` or in `program`.
    fn declared(marked: &str, program: &str) -> Option<String> {
        let cursor = marked.find('|').unwrap();
        let text = marked.replace('|', "");
        let classes = classes();
        let found = definition_at(&text, text[..cursor].chars().count(), Some(&classes))?;
        let (source, start) = match found {
            Definition::InText(index) => (text.as_str(), text.char_indices().nth(index)?.0),
            Definition::InProgram(span) => (program, span.start()),
        };
        let rest = &source[start..];
        let end = rest
            .find(|character: char| !is_name_character(character))
            .unwrap_or(rest.len());
        Some(format!("{}@{start}", &rest[..end]))
    }

    fn offset_in_rock(text: &str) -> String {
        format!(
            "{}@{}",
            text.split(|c: char| !is_name_character(c)).next().unwrap(),
            ROCK.find(text).unwrap()
        )
    }

    #[test]
    fn variables_go_to_their_declaration_in_the_function() {
        let marked = ROCK.replace("turns = size", "tu|rns = size");

        let found = declared(&marked, ROCK);

        assert_eq!(
            found,
            Some(format!("turns@{}", ROCK.find("turns = 2").unwrap()))
        );
    }

    #[test]
    fn parameters_are_declarations() {
        let marked = ROCK.replace("var turns = 2", "var turns = spe|ed");

        assert_eq!(
            declared(&marked, ROCK)
                .as_deref()
                .map(|found| found.starts_with("speed@")),
            Some(true)
        );
    }

    #[test]
    fn fields_of_the_class_go_to_their_declaration() {
        let marked = ROCK.replace("turns = size", "turns = si|ze");

        assert_eq!(declared(&marked, ROCK), Some(offset_in_rock("size: int")));
    }

    #[test]
    fn members_after_a_dot_go_to_their_class() {
        let marked = GAME.replace("rock.roll", "rock.ro|ll");

        assert_eq!(declared(&marked, ROCK), Some(offset_in_rock("roll(speed")));
    }

    #[test]
    fn classes_go_to_their_declaration() {
        let marked = GAME.replace("var other = Rock()", "var other = Ro|ck()");

        assert_eq!(
            declared(&marked, ROCK),
            Some(offset_in_rock("Rock extends"))
        );
    }

    #[test]
    fn engine_names_have_nowhere_to_go() {
        let marked = ROCK.replace("extends Node", "extends No|de");

        assert_eq!(declared(&marked, ROCK), None);
    }
}
