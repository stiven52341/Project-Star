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
mod statements;
mod types;

pub use analysis::{Analysis, ExprInfo, LocalId, LocalInfo, Resolution};
pub use classes::{
    ClassId, ClassInfo, ClassOrigin, ClassTable, ConstantInfo, ConstructorInfo, FieldInfo,
    FoundMember, MemberRef, MethodInfo, ParameterInfo,
};
pub use error::{SemanticError, SemanticErrorKind};
pub use natives::BuiltinMethod;
pub use types::Type;

use crate::ast::SourceFile;
use checker::Checker;

/// Checks the meaning of `file`.
///
/// # Errors
///
/// Returns every semantic mistake found, in source order.
pub fn check(file: &SourceFile) -> Result<Analysis, Vec<SemanticError>> {
    let mut analysis = Analysis::default();
    natives::add_native_classes(&mut analysis.classes);
    let mut checker = Checker::new(file, analysis);
    checker.check_file();
    if checker.errors.is_empty() {
        Ok(checker.analysis)
    } else {
        Err(checker.errors)
    }
}

#[cfg(test)]
mod tests;
