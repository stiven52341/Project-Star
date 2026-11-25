//! Lowering: from the syntax tree and its analysis to the interpreter's
//! tree, with names resolved and conversions written out.

mod body;
mod constructor;
mod expressions;
mod waits;

use std::collections::HashMap;
use std::sync::Arc;

use star_script::Span;
use star_script::ast::{ClassDecl, Expr, MemberKind, MethodDecl, SourceFile, VariableKind};
use star_script::semantic::{
    Analysis, ClassId, ClassInfo, ClassOrigin, MemberRef, MethodInfo, Type,
};

use crate::error::{LowerError, LowerErrorKind};
use crate::tree::{
    Class, ClassRef, Code, ConstructorId, ConstructorRef, Coroutine, EngineClass, EngineMethod,
    Entry, ExportKind, ExportedField, Expression, Function, FunctionId, SceneClass, Statement,
};
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
    /// The position of each class of the program, the engine classes the
    /// interpreter knows included.
    classes: HashMap<ClassId, ClassRef>,
    /// The constructor of each class of the program.
    constructors: HashMap<ClassId, ConstructorId>,
    /// The engine classes the interpreter can make objects of.
    engine: HashMap<ClassId, EngineClass>,
    /// The place of each overridable method (class, index) in the table
    /// of its class.
    overridable: HashMap<(ClassId, usize), usize>,
    /// The table of each class, in the order of the classes.
    methods: Vec<Vec<FunctionId>>,
    /// Each class and its bases, in the order of the classes.
    ancestors: Vec<Vec<ClassRef>>,
}

impl Tables {
    /// What builds the objects of `class`.
    fn constructor(&self, class: ClassId) -> Option<ConstructorRef> {
        self.constructors
            .get(&class)
            .copied()
            .map(ConstructorRef::Script)
            .or_else(|| self.engine.get(&class).copied().map(ConstructorRef::Engine))
    }
}

/// The engine classes that classes of the program can extend for now.
const EXTENDABLE: [EngineClass; 5] = [
    EngineClass::Node,
    EngineClass::Camera2D,
    EngineClass::Game,
    EngineClass::Sprite,
    EngineClass::Light2D,
];

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
    let mut order = engine_classes(analysis, &mut tables)?;
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
        check_base(analysis, &tables, info, name)?;
        tables.classes.insert(id, ClassRef(order.len()));
        tables.constructors.insert(id, ConstructorId(classes.len()));
        order.push(id);
        let mut overridable = Vec::new();
        for (index, method) in info.methods.iter().enumerate() {
            let declaration = method_declaration(declaration, method)?;
            let function = FunctionId(pending.len());
            if is_entry_point(method) {
                if main.is_some() {
                    return Err(LowerError::new(
                        LowerErrorKind::SeveralEntryPoints,
                        declaration.name.span,
                    ));
                }
                main = Some((entry_point(function, method), declaration.name.span));
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
    method_tables(analysis, &mut tables, &classes, &order)?;
    let entry = entry(analysis, &tables, &classes, main)?;
    let functions = pending
        .iter()
        .map(|member| function(analysis, &tables, member))
        .collect::<Result<Vec<_>, _>>()?;
    let constructors = classes
        .iter()
        .map(|class| {
            let position = tables.classes.get(&class.id).copied().ok_or_else(|| {
                LowerError::internal("class without a position", class.declaration.name.span)
            })?;
            Body::new(analysis, &tables, class.id, Type::Void).constructor(
                position,
                class.info,
                class.declaration,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let classes = order
        .iter()
        .zip(tables.methods.iter().zip(&tables.ancestors))
        .map(|(id, (methods, ancestors))| Class {
            methods: methods.clone(),
            ancestors: ancestors.clone(),
            overrides: engine_overrides(analysis, &tables, *id),
        })
        .collect();
    Ok(Code {
        functions,
        constructors,
        classes,
        entry,
        scene_classes: scene_classes(analysis, &tables)?,
    })
}

/// What a scene can build its nodes as: every class but the engine's node
/// classes (which the runtime creates by their own names), with the
/// analysis's reason when a scene cannot use it, as compiled.
fn scene_classes(
    analysis: &Analysis,
    tables: &Tables,
) -> Result<HashMap<String, SceneClass>, LowerError> {
    let table = analysis.classes();
    let mut scene_classes = HashMap::new();
    for (id, info) in table.iter() {
        let check = table.check_scene_class(id);
        if info.origin == ClassOrigin::Native && check.is_ok() {
            continue;
        }
        let missing = || LowerError::internal("class without a place", Span::new(0, 0));
        let creation = match check {
            Ok(()) => Ok(tables.constructors.get(&id).copied().ok_or_else(missing)?),
            Err(reason) => Err(reason.to_string()),
        };
        let class = SceneClass {
            creation,
            exported: exported_fields(analysis, id),
        };
        scene_classes.insert(info.name.clone(), class);
    }
    Ok(scene_classes)
}

/// The `export` fields of `class` and its bases.
fn exported_fields(analysis: &Analysis, class: ClassId) -> Vec<ExportedField> {
    let table = analysis.classes();
    table
        .ancestors(class)
        .flat_map(|owner| {
            table
                .class(owner)
                .fields
                .iter()
                .enumerate()
                .filter(|(_, field)| field.is_exported)
                .filter_map(move |(index, field)| {
                    Some(ExportedField {
                        name: field.name.clone(),
                        position: body::script_field(analysis, owner, index),
                        kind: export_kind(analysis, &field.ty)?,
                    })
                })
        })
        .collect()
}

/// How an `export` field of type `ty` takes a scene's value; the analysis
/// lets no other type be exported.
fn export_kind(analysis: &Analysis, ty: &Type) -> Option<ExportKind> {
    Some(match ty {
        Type::Short => ExportKind::Short,
        Type::Int => ExportKind::Int,
        Type::BigInt => ExportKind::BigInt,
        Type::Float => ExportKind::Float,
        Type::Double => ExportKind::Double,
        Type::Bool => ExportKind::Bool,
        Type::String => ExportKind::String,
        Type::Class(class)
            if analysis.classes().class(*class).name == EngineClass::Color.name() =>
        {
            ExportKind::Color
        }
        _ => return None,
    })
}

/// What starts the program: `main`, or the class that extends `Game`
/// without being extended, as compiled; never both, nor two of them.
fn entry(
    analysis: &Analysis,
    tables: &Tables,
    classes: &[ScriptClass<'_>],
    main: Option<(Statement, Span)>,
) -> Result<Entry, LowerError> {
    let table = analysis.classes();
    let game = table
        .find(EngineClass::Game.name())
        .ok_or_else(|| LowerError::internal("engine class missing", Span::new(0, 0)))?;
    let mut games = classes.iter().filter(|class| {
        table.is_subclass(class.id, game)
            && !classes
                .iter()
                .any(|other| other.info.base == Some(class.id))
    });
    let first = games.next();
    let several = |span| Err(LowerError::new(LowerErrorKind::SeveralEntryPoints, span));
    if let Some(other) = games.next() {
        return several(other.declaration.name.span);
    }
    match (main, first) {
        (Some(_), Some(game)) => several(game.declaration.name.span),
        (Some((main, _)), None) => Ok(Entry::Main(main)),
        (None, Some(game)) => {
            let span = game.declaration.name.span;
            if !game.info.constructor_parameters().is_empty() {
                return Err(LowerError::new(LowerErrorKind::GameNeedsArguments, span));
            }
            let constructor = tables
                .constructors
                .get(&game.id)
                .copied()
                .ok_or_else(|| LowerError::internal("game without a constructor", span))?;
            Ok(Entry::Game(constructor))
        }
        (None, None) => match &analysis.settings().main_scene {
            Some(scene) => Ok(Entry::Scene(Arc::from(scene.path.as_str()))),
            None => Err(LowerError::new(
                LowerErrorKind::NoEntryPoint,
                Span::new(0, 0),
            )),
        },
    }
}

/// The methods of the engine that objects of `class` run a script's
/// version of: those its engine class declares that a script class of its
/// hierarchy overrides (none for an engine class).
fn engine_overrides(
    analysis: &Analysis,
    tables: &Tables,
    class: ClassId,
) -> Vec<(EngineMethod, FunctionId)> {
    let table = analysis.classes();
    let mut overrides: Vec<(EngineMethod, FunctionId)> = Vec::new();
    for ancestor in table.ancestors(class) {
        let info = table.class(ancestor);
        if info.origin != ClassOrigin::Native {
            continue;
        }
        for method in &info.methods {
            let Some(engine) = expressions::object_method(&info.name, &method.name) else {
                continue;
            };
            if overrides.iter().any(|(known, _)| *known == engine) {
                continue;
            }
            if let Some(function) = written_override(analysis, tables, class, &method.name) {
                overrides.push((engine, function));
            }
        }
    }
    overrides
}

/// The method `name` of objects of `class` that a script wrote, in the
/// class or its closest base that has one: what the engine runs instead of
/// its own.
fn written_override(
    analysis: &Analysis,
    tables: &Tables,
    class: ClassId,
    name: &str,
) -> Option<FunctionId> {
    let table = analysis.classes();
    table.ancestors(class).find_map(|ancestor| {
        let info = table.class(ancestor);
        if info.origin == ClassOrigin::Native {
            return None;
        }
        let index = info
            .methods
            .iter()
            .position(|method| method.name == name && !method.is_static)?;
        tables
            .functions
            .get(&Member::Method(ancestor, index))
            .copied()
    })
}

/// Refuses classes that extend an engine class the interpreter cannot
/// extend yet.
fn check_base(
    analysis: &Analysis,
    tables: &Tables,
    info: &ClassInfo,
    name: Span,
) -> Result<(), LowerError> {
    let refused = info.base.is_some_and(|base| {
        analysis.classes().class(base).origin == ClassOrigin::Native
            && !tables
                .engine
                .get(&base)
                .is_some_and(|base| EXTENDABLE.contains(base))
    });
    if refused {
        return Err(LowerError::unsupported(
            "classes that extend this engine class",
            name,
        ));
    }
    Ok(())
}

/// Gives the engine classes the interpreter knows their fixed places
/// (see [`EngineClass`]); the classes in the order of their places.
fn engine_classes(analysis: &Analysis, tables: &mut Tables) -> Result<Vec<ClassId>, LowerError> {
    EngineClass::ALL
        .iter()
        .map(|engine| {
            let id = analysis
                .classes()
                .find(engine.name())
                .ok_or_else(|| LowerError::internal("engine class missing", Span::new(0, 0)))?;
            tables.classes.insert(id, engine.class());
            tables.engine.insert(id, *engine);
            Ok(id)
        })
        .collect()
}

/// The table of overridable methods of every class: a subclass starts
/// from its base's table, puts its overrides in the places of the methods
/// they replace and adds its new methods at the end. Bases are filled in
/// first, wherever they are written; engine classes have empty tables.
/// `order` has every class in the order of their places.
fn method_tables(
    analysis: &Analysis,
    tables: &mut Tables,
    classes: &[ScriptClass<'_>],
    order: &[ClassId],
) -> Result<(), LowerError> {
    let table_classes = analysis.classes();
    let depth = |class: &ScriptClass<'_>| table_classes.ancestors(class.id).count();
    let mut by_depth: Vec<&ScriptClass<'_>> = classes.iter().collect();
    by_depth.sort_by_key(|class| depth(class));
    let mut methods: Vec<Option<Vec<FunctionId>>> = order
        .iter()
        .map(|id| tables.engine.contains_key(id).then(Vec::new))
        .collect();
    for class in by_depth {
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
    tables.ancestors = order
        .iter()
        .map(|class| {
            table_classes
                .ancestors(*class)
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
        } => {
            let mut body = Body::new(analysis, tables, *class, info.return_type.clone());
            body.in_parallel = info.is_parallel();
            let mut function = body.function(declaration, info.is_static)?;
            if info.is_async() {
                waits::mark_block(&mut function.body);
            }
            Ok(function)
        }
        Pending::Constant { class, value, ty } => {
            let value = Body::new(analysis, tables, *class, (*ty).clone()).value_as(value, ty)?;
            Ok(Function {
                slots: 0,
                body: vec![Statement::Return(Some(value))],
            })
        }
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

/// What starts the program: a call of `main`, or its start as a coroutine
/// if it is `async`, as compiled.
fn entry_point(function: FunctionId, method: &MethodInfo) -> Statement {
    let arguments = Vec::new();
    Statement::Evaluate(if method.is_async() {
        Expression::Start(Box::new(Coroutine::Call {
            function,
            arguments,
        }))
    } else {
        Expression::Call {
            function,
            arguments,
        }
    })
}

/// `static func main()`, with no parameters and no result.
fn is_entry_point(method: &MethodInfo) -> bool {
    method.name == "main"
        && method.is_static
        && method.parameters.is_empty()
        && method.return_type == Type::Void
}
