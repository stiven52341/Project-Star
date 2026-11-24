//! Third compiler stage: checks the meaning of a syntax tree.
//!
//! Resolves every name, infers and checks every type, enforces null safety
//! and the rules of classes, inheritance and construction. The resulting
//! [`Analysis`] tells later stages the type of each expression and what each
//! name refers to.

mod analysis;
mod calls;
mod checker;
mod classes;
mod declarations;
mod error;
mod expressions;
mod flow;
mod narrowing;
mod natives;
mod scene_classes;
mod statements;
mod types;

pub use analysis::{Analysis, ExprInfo, LocalId, LocalInfo, Resolution};
pub use classes::{
    ClassId, ClassInfo, ClassOrigin, ClassTable, ConstantInfo, ConstructorInfo, Execution,
    FieldInfo, FoundMember, MemberRef, MethodInfo, ParameterInfo,
};
pub use error::{SemanticError, SemanticErrorKind};
pub use natives::{
    AUDIO_CLASS, BASIC_TYPE_NAMES, BODY_CLASS, BuiltinMethod, BuiltinSignature, CAMERA_3D_CLASS,
    CAMERA_CLASS, COLOR_CLASS, GAME_CLASS, INPUT_CLASS, LIGHT_CLASS, MESH_CLASS, NODE_3D_CLASS,
    NODE_CLASS, SCENE_CLASS, SOUND_CLASS, SPRITE_CLASS, basic_type, builtin_method,
    builtin_methods,
};
pub use scene_classes::SceneClassError;
pub use types::Type;

use crate::GameSettings;
use crate::ast::SourceFile;
use checker::Checker;

/// Checks the meaning of `file`.
///
/// # Errors
///
/// Returns every semantic mistake found, in source order.
pub fn check(file: &SourceFile) -> Result<Analysis, Vec<SemanticError>> {
    check_with(file, &GameSettings::default())
}

/// Checks the meaning of `file`, a script of a game with `settings`.
///
/// # Errors
///
/// Returns every semantic mistake found, in source order.
pub fn check_with(
    file: &SourceFile,
    settings: &GameSettings,
) -> Result<Analysis, Vec<SemanticError>> {
    let (analysis, errors) = analyze_with(file, settings);
    if errors.is_empty() {
        Ok(analysis)
    } else {
        Err(errors)
    }
}

/// The classes the engine provides, with their members, as every program
/// sees them before its own: what tools list, such as the ready-made
/// functions of the blueprint editor.
#[must_use]
pub fn native_classes() -> ClassTable {
    let mut classes = ClassTable::default();
    natives::add_native_classes(&mut classes);
    classes
}

/// Checks the meaning of `file` as [`check_with`] does, but keeps what the
/// analysis learned even when it finds mistakes (the classes and their
/// members, and the types of the code without mistakes): what tools that
/// help write code, such as suggestions, need.
#[must_use]
pub fn analyze_with(file: &SourceFile, settings: &GameSettings) -> (Analysis, Vec<SemanticError>) {
    let mut analysis = Analysis {
        settings: settings.clone(),
        ..Analysis::default()
    };
    natives::add_native_classes(&mut analysis.classes);
    let mut checker = Checker::new(file, analysis);
    checker.check_file();
    (checker.analysis, checker.errors)
}

#[cfg(test)]
mod tests;
