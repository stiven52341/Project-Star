//! Lowering: from the syntax tree and its analysis to the interpreter's
//! tree, with names resolved and conversions written out.

mod body;
mod constructor;
mod expressions;

use std::collections::HashMap;

use star_script::Span;
use star_script::ast::{ClassDecl, Expr, MemberKind, MethodDecl, SourceFile, VariableKind};
use star_script::semantic::{
    Analysis, ClassId, ClassInfo, ClassOrigin, Execution, MemberRef, MethodInfo, Type,
};

use crate::error::{LowerError, LowerErrorKind};
use crate::tree::{Class, ClassRef, Code, ConstructorId, Function, FunctionId, Statement};
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

/// Where each part of the program went, for the code that refers to it.
#[derive(Debug, Default)]
struct Tables {
    /// The function each method and constant became.
    functions: HashMap<Member, FunctionId>,
    /// The position of each class of the program; its constructor has
    /// the same position.
    classes: HashMap<ClassId, ClassRef>,
    /// The place of each overridable method (class, index) in the table
    /// of its class.
    overridable: HashMap<(ClassId, usize), usize>,
    /// The table of each class, in the order of the classes.
    methods: Vec<Vec<FunctionId>>,
    /// Each class and its bases, in the order of the classes.
    ancestors: Vec<Vec<ClassRef>>,
}

impl Tables {
    /// The constructor of `class`.
    fn constructor(&self, class: ClassId) -> Option<ConstructorId> {
        self.classes.get(&class).map(|class| ConstructorId(class.0))
    }
}

/// A member of the program, before its code is lowered.
enum Pending<'a> {
    Method {
        class: ClassId,
        declaration: &'a MethodDecl,
        info: &'a MethodInfo,
    },
    Constant {
        class: ClassId,
        value: &'a Expr,
        ty: &'a Type,
    },
}

/// A class of the program, before its constructor is lowered.
struct ScriptClass<'a> {
    id: ClassId,
    info: &'a ClassInfo,
    declaration: &'a ClassDecl,
    /// Its methods that objects run their own version of: (index of the
    /// method, its function).
    overridable: Vec<(usize, FunctionId)>,
}

/// Lowers the whole program.
pub(crate) fn program(syntax: &SourceFile, analysis: &Analysis) -> Result<Code, LowerError> {
    let mut tables = Tables::default();
    let mut pending = Vec::new();
    let mut classes = Vec::new();
    let mut main = None;
    for (id, info) in analysis.classes().iter() {
        let ClassOrigin::Script { declaration, name } = info.origin else {
            continue;
        };
        let declaration = syntax
            .classes
            .get(declaration)
            .ok_or_else(|| LowerError::internal("class without declaration", name))?;
        let extends_engine = info
            .base
            .is_some_and(|base| analysis.classes().class(base).origin == ClassOrigin::Native);
        if extends_engine {
            return Err(LowerError::unsupported(
                "classes that extend engine classes",
                name,
            ));
        }
        tables.classes.insert(id, ClassRef(classes.len()));
        let mut overridable = Vec::new();
        for (index, method) in info.methods.iter().enumerate() {
            let declaration = method_declaration(declaration, method)?;
            check_method(method, declaration.name.span)?;
            let function = FunctionId(pending.len());
            if is_entry_point(method) {
                if main.is_some() {
                    return Err(LowerError::new(
                        LowerErrorKind::SeveralEntryPoints,
                        declaration.name.span,
                    ));
                }
                main = Some(function);
            }
            if !method.is_static && !method.is_private {
                overridable.push((index, function));
            }
            tables.functions.insert(Member::Method(id, index), function);
            pending.push(Pending::Method {
                class: id,
                declaration,
                info: method,
            });
        }
        for (index, constant) in info.constants.iter().enumerate() {
            let value = constant_value(declaration, constant.span)?;
            tables
                .functions
                .insert(Member::Constant(id, index), FunctionId(pending.len()));
            pending.push(Pending::Constant {
                class: id,
                value,
                ty: &constant.ty,
            });
        }
        classes.push(ScriptClass {
            id,
            info,
            declaration,
            overridable,
        });
    }
    method_tables(analysis, &mut tables, &classes)?;
    let main =
        main.ok_or_else(|| LowerError::new(LowerErrorKind::NoEntryPoint, Span::new(0, 0)))?;
    let functions = pending
        .iter()
        .map(|member| function(analysis, &tables, member))
        .collect::<Result<Vec<_>, _>>()?;
    let constructors = classes
        .iter()
        .enumerate()
        .map(|(position, class)| {
            Body::new(analysis, &tables, class.id, Type::Void).constructor(
                ClassRef(position),
                class.info,
                class.declaration,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let classes = tables
        .methods
        .into_iter()
        .zip(tables.ancestors)
        .map(|(methods, ancestors)| Class { methods, ancestors })
        .collect();
    Ok(Code {
        functions,
        constructors,
        classes,
        main,
    })
}

/// The table of overridable methods of every class: a subclass starts
/// from its base's table, puts its overrides in the places of the methods
/// they replace and adds its new methods at the end. Bases are filled in
/// first, wherever they are written.
fn method_tables(
    analysis: &Analysis,
    tables: &mut Tables,
    classes: &[ScriptClass<'_>],
) -> Result<(), LowerError> {
    let table_classes = analysis.classes();
    let depth = |class: &ScriptClass<'_>| table_classes.ancestors(class.id).count();
    let mut order: Vec<&ScriptClass<'_>> = classes.iter().collect();
    order.sort_by_key(|class| depth(class));
    let mut methods: Vec<Option<Vec<FunctionId>>> = vec![None; classes.len()];
    for class in order {
        let span = class.declaration.name.span;
        let base = class.info.base;
        let mut table = match base.and_then(|base| tables.classes.get(&base)) {
            Some(base) => methods
                .get(base.0)
                .cloned()
                .flatten()
                .ok_or_else(|| LowerError::internal("base without a table", span))?,
            None => Vec::new(),
        };
        for (index, function) in &class.overridable {
            let method = class
                .info
                .methods
                .get(*index)
                .ok_or_else(|| LowerError::internal("unknown method", span))?;
            let overridden = base
                .filter(|_| method.is_override)
                .and_then(|base| overridden_place(analysis, tables, base, &method.name));
            let place = if let Some(place) = overridden {
                let slot = table
                    .get_mut(place)
                    .ok_or_else(|| LowerError::internal("place outside the table", span))?;
                *slot = *function;
                place
            } else {
                table.push(*function);
                table.len() - 1
            };
            tables.overridable.insert((class.id, *index), place);
        }
        let position = tables
            .classes
            .get(&class.id)
            .ok_or_else(|| LowerError::internal("class without a position", span))?;
        if let Some(entry) = methods.get_mut(position.0) {
            *entry = Some(table);
        }
    }
    tables.methods = methods.into_iter().map(Option::unwrap_or_default).collect();
    tables.ancestors = classes
        .iter()
        .map(|class| {
            table_classes
                .ancestors(class.id)
                .filter_map(|id| tables.classes.get(&id).copied())
                .collect()
        })
        .collect();
    Ok(())
}

/// The place in the tables of the method `name` that `base` (or one of
/// its bases) has, which an override takes.
fn overridden_place(
    analysis: &Analysis,
    tables: &Tables,
    base: ClassId,
    name: &str,
) -> Option<usize> {
    let found = analysis.classes().find_member(base, name)?;
    let MemberRef::Method(index) = found.member else {
        return None;
    };
    tables.overridable.get(&(found.class, index)).copied()
}

/// Lowers a method or a constant.
fn function(
    analysis: &Analysis,
    tables: &Tables,
    member: &Pending<'_>,
) -> Result<Function, LowerError> {
    match member {
        Pending::Method {
            class,
            declaration,
            info,
        } => Body::new(analysis, tables, *class, info.return_type.clone())
            .function(declaration, info.is_static),
        Pending::Constant { class, value, ty } => {
            let value = Body::new(analysis, tables, *class, (*ty).clone()).value_as(value, ty)?;
            Ok(Function {
                slots: 0,
                body: vec![Statement::Return(Some(value))],
            })
        }
    }
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
