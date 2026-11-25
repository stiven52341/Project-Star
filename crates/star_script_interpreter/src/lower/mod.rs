//! Lowering: from the syntax tree and its analysis to the interpreter's
//! tree, with names resolved and conversions written out.

mod body;
mod expressions;

use std::collections::HashMap;

use star_script::Span;
use star_script::ast::{ClassDecl, Expr, MemberKind, MethodDecl, SourceFile, VariableKind};
use star_script::semantic::{Analysis, ClassId, ClassOrigin, Execution, MethodInfo, Type};

use crate::Program;
use crate::error::{LowerError, LowerErrorKind};
use crate::tree::{Function, FunctionId, Statement};
use body::Body;

/// A member of a class that lowering turns into a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Member {
    /// The method at this index of the class's methods.
    Method(ClassId, usize),
    /// The constant at this index of the class's constants: a function
    /// without parameters that computes its value, as compiled.
    Constant(ClassId, usize),
}

/// The function each member of the program became.
type FunctionTable = HashMap<Member, FunctionId>;

/// A member of the program, before its code is lowered.
enum Pending<'a> {
    Method {
        declaration: &'a MethodDecl,
        info: &'a MethodInfo,
    },
    Constant {
        value: &'a Expr,
        ty: &'a Type,
    },
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
            table.insert(Member::Method(id, index), function);
            pending.push(Pending::Method {
                declaration: method,
                info,
            });
        }
        for (index, constant) in class.constants.iter().enumerate() {
            let value = constant_value(declaration, constant.span)?;
            table.insert(Member::Constant(id, index), FunctionId(pending.len()));
            pending.push(Pending::Constant {
                value,
                ty: &constant.ty,
            });
        }
    }
    let main =
        main.ok_or_else(|| LowerError::new(LowerErrorKind::NoEntryPoint, Span::new(0, 0)))?;
    let functions = pending
        .iter()
        .map(|member| match member {
            Pending::Method { declaration, info } => {
                Body::new(analysis, &table, info.return_type.clone()).function(declaration)
            }
            Pending::Constant { value, ty } => {
                let value = Body::new(analysis, &table, (*ty).clone()).value_as(value, ty)?;
                Ok(Function {
                    slots: 0,
                    body: vec![Statement::Return(Some(value))],
                })
            }
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

/// The value of the constant whose name is written at `name`.
fn constant_value(declaration: &ClassDecl, name: Span) -> Result<&Expr, LowerError> {
    declaration
        .members
        .iter()
        .find_map(|member| match &member.kind {
            MemberKind::Variable(variable)
                if variable.kind == VariableKind::Const && variable.name.span == name =>
            {
                variable.initializer.as_ref()
            }
            _ => None,
        })
        .ok_or_else(|| LowerError::internal("constant without value", name))
}

/// `static func main()`, with no parameters and no result.
fn is_entry_point(method: &MethodInfo) -> bool {
    method.name == "main"
        && method.is_static
        && method.parameters.is_empty()
        && method.return_type == Type::Void
}
