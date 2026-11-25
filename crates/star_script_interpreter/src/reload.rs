//! Hot reload (decision 76): the game this thread plays goes on with a new
//! program. Its objects keep existing, with their engine part and their
//! place in the tree; each finds its class again by name, and its fields
//! by name and type. Nothing changes unless every object can.

use std::any::Any;
use std::sync::Arc;

use crate::Program;
use crate::error::{ReloadError, RunError};
use crate::play;
use crate::run::Machine;
use crate::tree::{ClassRef, Code, EngineClass, FieldLayout, Function};
use crate::value::{Object, Value};

/// What a reload did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reloaded {
    /// How many objects of the program's classes went on with the new code.
    pub objects: usize,
    /// How many coroutines ended: they ran the old code.
    pub coroutines_ended: usize,
}

/// Gives the game this thread plays ([`Program::new_game`]) the code of
/// `program`, between two frames. `start` does not run again.
///
/// # Errors
///
/// When the game's objects cannot go on with the new program
/// ([`ReloadError`]); the game then goes on with the code it had.
pub fn reload(program: &Program) -> Result<Reloaded, ReloadError> {
    let old = play::machine().ok_or(ReloadError::NotPlaying)?;
    if program.kind() != crate::ProgramKind::Game {
        return Err(ReloadError::NotAGame);
    }
    let new = old.with_code(Arc::clone(&program.code));
    let objects = old.living_objects()?;
    let plans = objects
        .iter()
        .map(|object| plan(old.code(), new.code(), &**object))
        .collect::<Result<Vec<_>, _>>()?;
    let rows = objects
        .iter()
        .zip(&plans)
        .map(|(object, plan)| plan.row(&new, &**object))
        .collect::<Result<Vec<_>, _>>()?;
    for ((object, plan), row) in objects.iter().zip(&plans).zip(rows) {
        object.reshape(plan.class, row)?;
    }
    let coroutines_ended = star_script_runtime::tasks::waiting();
    star_script_runtime::tasks::clear();
    play::replace_machine(new);
    Ok(Reloaded {
        objects: objects.len(),
        coroutines_ended,
    })
}

/// How an object goes on with the new program.
struct Plan<'a> {
    /// Its class in the new program.
    class: ClassRef,
    /// Where each field of the new class's row comes from.
    fields: Vec<Source<'a>>,
}

/// Where a field of an object reloaded comes from.
enum Source<'a> {
    /// The field it had at this place.
    Kept(usize),
    /// A field it did not have: this gives its value.
    Start(&'a Function),
}

impl Plan<'_> {
    /// The fields of `object` for the new program, which runs the values
    /// of the new ones.
    fn row(&self, new: &Machine, object: &Object<dyn Any>) -> Result<Vec<Value>, RunError> {
        let old = object.fields()?;
        self.fields
            .iter()
            .map(|source| match source {
                Source::Kept(place) => old
                    .get(*place)
                    .cloned()
                    .ok_or(RunError::Internal("field outside its object")),
                Source::Start(function) => new.run_function(function),
            })
            .collect()
    }
}

/// How `object`, of a class of `old`, goes on with `new`; why it cannot.
fn plan<'a>(old: &Code, new: &'a Code, object: &Object<dyn Any>) -> Result<Plan<'a>, ReloadError> {
    let class = object.class();
    let name = &old
        .classes
        .get(class.0)
        .ok_or(RunError::Internal("unknown class"))?
        .name;
    let found = new
        .classes
        .iter()
        .position(|candidate| candidate.name == *name)
        .map(ClassRef)
        .ok_or_else(|| ReloadError::ClassRemoved(name.to_string()))?;
    if engine_class(old, class) != engine_class(new, found) {
        return Err(ReloadError::EngineClassChanged(name.to_string()));
    }
    let before = row(old, class);
    let fields = row(new, found)
        .into_iter()
        .map(|field| {
            if let Some(place) = before
                .iter()
                .position(|old| old.name == field.name && old.ty == field.ty)
            {
                return Ok(Source::Kept(place));
            }
            field
                .start
                .as_ref()
                .map(Source::Start)
                .ok_or_else(|| ReloadError::FieldNeedsInit {
                    class: name.to_string(),
                    field: field.name.clone(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Plan {
        class: found,
        fields,
    })
}

/// The fields of the objects of `class`, as their rows lay them out: those
/// of the root class first.
fn row(code: &Code, class: ClassRef) -> Vec<&FieldLayout> {
    let Some(class) = code.classes.get(class.0) else {
        return Vec::new();
    };
    class
        .ancestors
        .iter()
        .rev()
        .filter_map(|ancestor| code.classes.get(ancestor.0))
        .flat_map(|ancestor| &ancestor.fields)
        .collect()
}

/// The engine class that `class` extends, if any: its objects' engine part.
fn engine_class(code: &Code, class: ClassRef) -> Option<ClassRef> {
    code.classes
        .get(class.0)?
        .ancestors
        .iter()
        .copied()
        .find(|ancestor| ancestor.0 < EngineClass::ALL.len())
}
