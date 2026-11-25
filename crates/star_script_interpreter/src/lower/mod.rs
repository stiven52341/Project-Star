//! Lowering: from the syntax tree and its analysis to the interpreter's
//! tree, with names resolved and conversions written out.

mod body;
mod expressions;

use std::collections::HashMap;

use star_script::Span;
use star_script::ast::{ClassDecl, MemberKind, MethodDecl, SourceFile};
use star_script::semantic::{Analysis, ClassId, ClassOrigin, Execution, MethodInfo, Type};

use crate::Program;
use crate::error::{LowerError, LowerErrorKind};
use crate::tree::{Function, FunctionId};
use body::Body;

/// Where each method of the program went: `(class, index of the method)`
/// to its function.
type FunctionTable = HashMap<(ClassId, usize), FunctionId>;

/// A method of the program, before its body is lowered.
struct Pending<'a> {
    declaration: &'a MethodDecl,
    info: &'a MethodInfo,
}

/// Lowers the whole program.
pub(crate) fn program(syntax: &SourceFile, analysis: &Analysis) -> Result<Program, LowerError> {
    let mut table = FunctionTable::new();
    let mut pending = Vec::new();
    let mut main = None;
    for (id, class) in analysis.classes().iter() {
        let ClassOrigin::Script { declaration, name } = class.origin else {
            continue;
        };
        let declaration = syntax
            .classes
            .get(declaration)
            .ok_or_else(|| LowerError::internal("class without declaration", name))?;
        // Methods of objects only run on objects, and lowering refuses
        // the code that would create one, so they are left out.
        let static_methods = class
            .methods
            .iter()
            .enumerate()
            .filter(|(_, info)| info.is_static);
        for (index, info) in static_methods {
            let method = method_declaration(declaration, info)?;
            check_method(info, method.name.span)?;
            let function = FunctionId(pending.len());
            if is_entry_point(info) {
                if main.is_some() {
                    return Err(LowerError::new(
                        LowerErrorKind::SeveralEntryPoints,
                        method.name.span,
                    ));
                }
                main = Some(function);
            }
            table.insert((id, index), function);
            pending.push(Pending {
                declaration: method,
                info,
            });
        }
    }
    let main =
        main.ok_or_else(|| LowerError::new(LowerErrorKind::NoEntryPoint, Span::new(0, 0)))?;
    let functions = pending
        .iter()
        .map(|method| {
            Body::new(analysis, &table, method.info.return_type.clone())
                .function(method.declaration)
        })
        .collect::<Result<Vec<Function>, LowerError>>()?;
    Ok(Program { functions, main })
}

/// Refuses the methods the interpreter cannot run yet.
fn check_method(info: &MethodInfo, span: Span) -> Result<(), LowerError> {
    match info.execution {
        Execution::Direct => Ok(()),
        Execution::Async => Err(LowerError::unsupported("`async` methods", span)),
        Execution::Parallel => Err(LowerError::unsupported("`parallel` functions", span)),
    }
}

/// The declaration of the method `info` describes.
fn method_declaration<'a>(
    declaration: &'a ClassDecl,
    info: &MethodInfo,
) -> Result<&'a MethodDecl, LowerError> {
    declaration
        .members
        .iter()
        .find_map(|member| match &member.kind {
            MemberKind::Method(method) if Some(method.name.span) == info.span => Some(method),
            _ => None,
        })
        .ok_or_else(|| LowerError::internal("method without declaration", declaration.name.span))
}

/// `static func main()`, with no parameters and no result.
fn is_entry_point(method: &MethodInfo) -> bool {
    method.name == "main"
        && method.is_static
        && method.parameters.is_empty()
        && method.return_type == Type::Void
}
