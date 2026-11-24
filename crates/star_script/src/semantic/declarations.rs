//! Passes over class declarations, in the order their results are needed:
//! classes, bases, member signatures, inheritance rules, constants, fields
//! and finally method bodies.

use super::InspectorHints;
use super::checker::{BodyContext, Checker, ContextKind, Initialization};
use super::field_defaults::plain_value;
use super::natives::{BASIC_TYPE_NAMES, NODE_CLASS, basic_type};
use super::{
    ClassId, ClassInfo, ClassOrigin, ConstantInfo, ConstructorInfo, Execution, FieldInfo,
    MemberRef, MethodInfo, ParameterInfo, SemanticErrorKind, Type, flow,
};
use crate::Span;
use crate::ast::{
    ClassDecl, ConstructorDecl, Expr, ExprKind, Member, MemberKind, MethodDecl, ModifierKind,
    Parameter, SourceFile, VariableDecl, VariableKind,
};
use crate::decorated::is_hidden_name;

impl Checker<'_> {
    pub(super) fn check_file(&mut self) {
        let file = self.file;
        let ids = self.declare_classes(file);
        let classes: Vec<(&ClassDecl, ClassId)> = declared(file, &ids).collect();
        self.resolve_bases(&classes);
        for (declaration, id) in &classes {
            self.collect_members(declaration, *id);
        }
        for (_, id) in &classes {
            self.check_inheritance(*id);
        }
        for (declaration, id) in &classes {
            self.resolve_constants(declaration, *id);
        }
        for (declaration, id) in &classes {
            self.resolve_fields(declaration, *id);
            self.check_decorators(declaration, *id);
        }
        for (declaration, id) in &classes {
            self.check_bodies(declaration, *id);
        }
        self.errors.sort_by_key(|error| error.span.start());
    }

    /// Registers every class by name; a repeated name gets `None`.
    fn declare_classes(&mut self, file: &SourceFile) -> Vec<Option<ClassId>> {
        file.classes
            .iter()
            .enumerate()
            .map(|(index, declaration)| {
                let name = &declaration.name.text;
                let taken = BASIC_TYPE_NAMES.contains(&name.as_str())
                    || self.analysis.classes.find(name).is_some();
                if taken {
                    let kind = SemanticErrorKind::DuplicateClass(name.clone());
                    self.error(kind, declaration.name.span);
                    return None;
                }
                let origin = ClassOrigin::Script {
                    declaration: index,
                    name: declaration.name.span,
                };
                Some(
                    self.analysis
                        .classes
                        .add(ClassInfo::new(name.clone(), origin)),
                )
            })
            .collect()
    }

    fn resolve_bases(&mut self, classes: &[(&ClassDecl, ClassId)]) {
        for (declaration, id) in classes {
            let Some(base) = &declaration.base else {
                continue;
            };
            if basic_type(&base.text).is_some() {
                self.error(
                    SemanticErrorKind::CannotExtend(base.text.clone()),
                    base.span,
                );
            } else if let Some(base_id) = self.analysis.classes.find(&base.text) {
                self.analysis.classes.class_mut(*id).base = Some(base_id);
            } else {
                self.error(SemanticErrorKind::UnknownType(base.text.clone()), base.span);
            }
        }
        for (declaration, id) in classes {
            let is_cyclic = self
                .analysis
                .classes
                .ancestors(*id)
                .skip(1)
                .any(|ancestor| ancestor == *id);
            if is_cyclic {
                let kind = SemanticErrorKind::CyclicInheritance(declaration.name.text.clone());
                self.error(kind, declaration.name.span);
                // Cutting the cycle keeps the rest of the analysis meaningful.
                self.analysis.classes.class_mut(*id).base = None;
            }
        }
    }

    // --- Member signatures ---

    fn collect_members(&mut self, declaration: &ClassDecl, id: ClassId) {
        for member in &declaration.members {
            match &member.kind {
                MemberKind::Variable(variable) if variable.kind == VariableKind::Var => {
                    self.collect_field(id, member, variable);
                }
                MemberKind::Variable(constant) => self.collect_constant(id, member, constant),
                MemberKind::Method(method) => self.collect_method(id, member, method),
                MemberKind::Constructor(constructor) => {
                    self.collect_constructor(id, member, constructor);
                }
            }
        }
    }

    /// Reports modifiers outside `allowed`; `member` describes what they are on.
    fn check_modifiers(&mut self, member: &Member, allowed: &[ModifierKind], what: &'static str) {
        for modifier in &member.modifiers {
            if !allowed.contains(&modifier.kind) {
                let kind = SemanticErrorKind::InvalidModifier {
                    modifier: modifier.kind,
                    member: what,
                };
                self.error(kind, modifier.span);
            }
        }
    }

    /// Whether `name` is free in the class; reports it if not.
    fn claim_name(&mut self, id: ClassId, name: &str, span: Span) -> bool {
        let taken = self.analysis.classes.class(id).own_member(name).is_some();
        if taken {
            self.error(SemanticErrorKind::DuplicateMember(name.to_owned()), span);
        }
        !taken
    }

    fn collect_field(&mut self, id: ClassId, member: &Member, field: &VariableDecl) {
        self.check_modifiers(
            member,
            &[ModifierKind::Private, ModifierKind::Export],
            "a field",
        );
        let is_private = member.has_modifier(ModifierKind::Private);
        let is_exported = member.has_modifier(ModifierKind::Export);
        if is_private && is_exported {
            let name = field.name.text.clone();
            self.error(SemanticErrorKind::PrivateExport(name), field.name.span);
        }
        if !self.claim_name(id, &field.name.text, field.name.span) {
            return;
        }
        // Without a written type, the type is inferred from the value later.
        let ty = field
            .type_annotation
            .as_ref()
            .map_or(Type::Error, |type_ref| self.resolve_type(type_ref));
        self.analysis.classes.class_mut(id).fields.push(FieldInfo {
            name: field.name.text.clone(),
            ty,
            is_private,
            is_exported,
            default: field.initializer.as_ref().and_then(plain_value),
            hints: InspectorHints::default(),
            has_initializer: field.initializer.is_some(),
            span: field.name.span,
        });
    }

    fn collect_constant(&mut self, id: ClassId, member: &Member, constant: &VariableDecl) {
        self.check_modifiers(member, &[ModifierKind::Private], "a constant");
        if !self.claim_name(id, &constant.name.text, constant.name.span) {
            return;
        }
        let ty = constant
            .type_annotation
            .as_ref()
            .map(|type_ref| self.resolve_type(type_ref));
        self.analysis
            .classes
            .class_mut(id)
            .constants
            .push(ConstantInfo {
                name: constant.name.text.clone(),
                resolved: ty.is_some(),
                ty: ty.unwrap_or(Type::Error),
                is_private: member.has_modifier(ModifierKind::Private),
                span: constant.name.span,
            });
    }

    fn collect_method(&mut self, id: ClassId, member: &Member, method: &MethodDecl) {
        let all = [
            ModifierKind::Private,
            ModifierKind::Static,
            ModifierKind::Override,
            ModifierKind::Async,
            ModifierKind::Parallel,
        ];
        self.check_modifiers(member, &all, "a method");
        let is_static = member.has_modifier(ModifierKind::Static);
        let is_private = member.has_modifier(ModifierKind::Private);
        let is_override = member.has_modifier(ModifierKind::Override);
        let execution = self.execution_of(member, is_static);
        if is_override && (is_static || is_private) {
            let what = if is_static {
                "a static method"
            } else {
                "a private method"
            };
            self.check_modifiers(member, &[ModifierKind::Private, ModifierKind::Static], what);
        }
        if !self.claim_name(id, &method.name.text, method.name.span) {
            return;
        }
        let parameters = self.parameter_infos(&method.parameters);
        let return_type = method
            .return_type
            .as_ref()
            .map_or(Type::Void, |type_ref| self.resolve_type(type_ref));
        if execution == Execution::Parallel {
            self.check_parallel_signature(method, &parameters, &return_type);
        }
        self.analysis
            .classes
            .class_mut(id)
            .methods
            .push(MethodInfo {
                name: method.name.text.clone(),
                parameters,
                return_type,
                is_static,
                is_private,
                is_override,
                execution,
                span: Some(method.name.span),
            });
    }

    /// How calls of the method run. `parallel` only goes with `static`,
    /// and never with `async`: it is awaited already.
    fn execution_of(&mut self, member: &Member, is_static: bool) -> Execution {
        if !member.has_modifier(ModifierKind::Parallel) {
            return if member.has_modifier(ModifierKind::Async) {
                Execution::Async
            } else {
                Execution::Direct
            };
        }
        if !is_static {
            self.check_modifiers(
                member,
                &[
                    ModifierKind::Private,
                    ModifierKind::Static,
                    ModifierKind::Override,
                    ModifierKind::Async,
                ],
                "a method that is not static",
            );
        }
        let without_async = [
            ModifierKind::Private,
            ModifierKind::Static,
            ModifierKind::Override,
            ModifierKind::Parallel,
        ];
        self.check_modifiers(member, &without_async, "a parallel function");
        Execution::Parallel
    }

    /// A `parallel` function only takes and gives plain values, which can
    /// be copied to its thread and back, and must give one: its result is
    /// its only effect.
    fn check_parallel_signature(
        &mut self,
        method: &MethodDecl,
        parameters: &[ParameterInfo],
        return_type: &Type,
    ) {
        for (parameter, info) in method.parameters.iter().zip(parameters) {
            if !info.ty.is_plain_value() {
                let kind = SemanticErrorKind::NotPlainValue(self.type_name(&info.ty));
                self.error(kind, parameter.type_annotation.span);
            }
        }
        match &method.return_type {
            None => {
                let kind = SemanticErrorKind::ParallelWithoutResult(method.name.text.clone());
                self.error(kind, method.name.span);
            }
            Some(type_ref) if !return_type.is_plain_value() => {
                let kind = SemanticErrorKind::NotPlainValue(self.type_name(return_type));
                self.error(kind, type_ref.span);
            }
            Some(_) => {}
        }
    }

    fn collect_constructor(&mut self, id: ClassId, member: &Member, constructor: &ConstructorDecl) {
        self.check_modifiers(member, &[ModifierKind::Private], "`init`");
        if self.analysis.classes.class(id).constructor.is_some() {
            self.error(SemanticErrorKind::DuplicateConstructor, member.span);
            return;
        }
        let parameters = self.parameter_infos(&constructor.parameters);
        self.analysis.classes.class_mut(id).constructor = Some(ConstructorInfo {
            parameters,
            is_private: member.has_modifier(ModifierKind::Private),
            span: member.span,
        });
    }

    fn parameter_infos(&mut self, parameters: &[Parameter]) -> Vec<ParameterInfo> {
        parameters
            .iter()
            .map(|parameter| ParameterInfo {
                name: parameter.name.text.clone(),
                ty: self.resolve_type(&parameter.type_annotation),
            })
            .collect()
    }

    // --- Inheritance ---

    /// Members cannot reuse names of base members, except methods that
    /// redefine a base method with `override` and the same signature.
    fn check_inheritance(&mut self, id: ClassId) {
        let classes = &self.analysis.classes;
        let class = classes.class(id);
        let base = class.base;
        let mut problems = Vec::new();
        let names = class
            .fields
            .iter()
            .map(|field| (&field.name, Some(field.span), None))
            .chain(
                class
                    .constants
                    .iter()
                    .map(|constant| (&constant.name, Some(constant.span), None)),
            )
            .chain(
                class
                    .methods
                    .iter()
                    .map(|method| (&method.name, method.span, Some(method))),
            );
        for (name, span, method) in names {
            let Some(span) = span else {
                continue;
            };
            let inherited = base.and_then(|base| classes.find_member(base, name));
            let problem = match (inherited, method) {
                (None, Some(method)) if method.is_override => {
                    Some(SemanticErrorKind::OverrideWithoutBase(name.clone()))
                }
                (None, _) => None,
                (Some(found), method) => {
                    let base_name = classes.class(found.class).name.clone();
                    let base_method = match found.member {
                        MemberRef::Method(index) => classes.class(found.class).methods.get(index),
                        MemberRef::Field(_) | MemberRef::Constant(_) => None,
                    };
                    override_problem(name, base_name, method, base_method)
                }
            };
            if let Some(problem) = problem {
                problems.push((problem, span));
            }
        }
        for (problem, span) in problems {
            self.error(problem, span);
        }
    }

    // --- Constants and fields ---

    fn resolve_constants(&mut self, declaration: &ClassDecl, id: ClassId) {
        for member in &declaration.members {
            let MemberKind::Variable(constant) = &member.kind else {
                continue;
            };
            let (Some(index), Some(value)) = (
                self.own_constant(id, constant),
                constant.initializer.as_ref(),
            ) else {
                continue;
            };
            self.context = BodyContext::new(id, ContextKind::Constant, Type::Void);
            let info = &self.analysis.classes.class(id).constants[index];
            let declared = info.resolved.then(|| info.ty.clone());
            let ty = self.initial_value(declared, value);
            let info = &mut self.analysis.classes.class_mut(id).constants[index];
            info.ty = ty;
            info.resolved = true;
        }
    }

    fn resolve_fields(&mut self, declaration: &ClassDecl, id: ClassId) {
        for member in &declaration.members {
            let MemberKind::Variable(field) = &member.kind else {
                continue;
            };
            let Some(index) = self.own_field(id, field) else {
                continue;
            };
            if is_hidden_name(&field.name.text) && !self.makes_a_decorator(field) {
                // Its decorator is reported as unknown where it is written.
                self.analysis.classes.class_mut(id).fields[index].ty = Type::Error;
                continue;
            }
            self.context = BodyContext::new(id, ContextKind::FieldInitializer, Type::Void);
            let declared = field
                .type_annotation
                .as_ref()
                .map(|_| self.analysis.classes.class(id).fields[index].ty.clone());
            let ty = match (declared, &field.initializer) {
                (declared, Some(value)) => self.initial_value(declared, value),
                (Some(declared), None) => declared,
                (None, None) => {
                    let name = field.name.text.clone();
                    self.error(SemanticErrorKind::MissingTypeOrValue(name), field.name.span);
                    Type::Error
                }
            };
            if member.has_modifier(ModifierKind::Export) {
                self.check_export(id, &field.name.text, &ty, field.name.span);
            }
            self.analysis.classes.class_mut(id).fields[index].ty = ty;
        }
    }

    /// Whether the hidden `field` of a decorated method makes an object of
    /// a decorator class.
    fn makes_a_decorator(&self, field: &VariableDecl) -> bool {
        let class = field
            .initializer
            .as_ref()
            .and_then(|value| match &value.kind {
                ExprKind::Call { callee, .. } => match &callee.kind {
                    ExprKind::Name(name) => Some(name),
                    _ => None,
                },
                _ => None,
            });
        class.is_some_and(|class| self.decorator_class(class).is_some())
    }

    /// An exported field belongs to a node, the only objects scenes make,
    /// and holds a value a scene can write.
    fn check_export(&mut self, id: ClassId, name: &str, ty: &Type, span: Span) {
        let classes = &self.analysis.classes;
        let is_node = classes
            .find(NODE_CLASS)
            .is_some_and(|node| classes.is_subclass(id, node));
        if !is_node {
            let class = classes.class(id).name.clone();
            self.error(SemanticErrorKind::ExportOutsideNode(class), span);
        } else if !classes.is_exportable(ty) && *ty != Type::Error {
            let kind = SemanticErrorKind::UnexportableType {
                field: name.to_owned(),
                ty: classes.type_name(ty),
            };
            self.error(kind, span);
        }
    }

    /// Checks `value` against the declared type, or infers the type from it.
    pub(super) fn initial_value(&mut self, declared: Option<Type>, value: &Expr) -> Type {
        if let Some(declared) = declared {
            let found = self.expression(value, Some(&declared));
            self.require_assignable(&found, &declared, value.span);
            return declared;
        }
        let found = self.expression(value, None);
        match found {
            Type::Null => {
                self.error(SemanticErrorKind::CannotInferNull, value.span);
                Type::Error
            }
            Type::Void => {
                self.error(SemanticErrorKind::NoValue, value.span);
                Type::Error
            }
            other => other,
        }
    }

    /// Index of the constant declared by `constant`, unless it was a duplicate.
    fn own_constant(&self, id: ClassId, constant: &VariableDecl) -> Option<usize> {
        let class = self.analysis.classes.class(id);
        match class.own_member(&constant.name.text) {
            Some(MemberRef::Constant(index))
                if class.constants.get(index)?.span == constant.name.span =>
            {
                Some(index)
            }
            _ => None,
        }
    }

    /// Index of the field declared by `field`, unless it was a duplicate.
    fn own_field(&self, id: ClassId, field: &VariableDecl) -> Option<usize> {
        let class = self.analysis.classes.class(id);
        match class.own_member(&field.name.text) {
            Some(MemberRef::Field(index)) if class.fields.get(index)?.span == field.name.span => {
                Some(index)
            }
            _ => None,
        }
    }

    // --- Bodies ---

    fn check_bodies(&mut self, declaration: &ClassDecl, id: ClassId) {
        let mut has_constructor = false;
        for member in &declaration.members {
            match &member.kind {
                MemberKind::Method(method) => self.check_method(id, method),
                MemberKind::Constructor(constructor) => {
                    let is_own = self
                        .analysis
                        .classes
                        .class(id)
                        .constructor
                        .as_ref()
                        .is_some_and(|info| info.span == member.span);
                    if is_own {
                        has_constructor = true;
                        self.check_constructor(id, constructor, member.span);
                    }
                }
                MemberKind::Variable(_) => {}
            }
        }
        if !has_constructor {
            self.check_implicit_constructor(declaration, id);
        }
    }

    fn check_method(&mut self, id: ClassId, method: &MethodDecl) {
        let class = self.analysis.classes.class(id);
        let Some(info) = class
            .methods
            .iter()
            .find(|info| info.span == Some(method.name.span))
            .cloned()
        else {
            return;
        };
        let kind = if info.is_static {
            ContextKind::Static
        } else {
            ContextKind::Instance
        };
        self.context = BodyContext::new(id, kind, info.return_type.clone());
        self.context.is_async = info.is_async();
        self.context.is_parallel = info.is_parallel();
        self.check_body(&method.parameters, &info.parameters, &method.body);
        if info.return_type != Type::Void && !flow::always_returns(&method.body) {
            let kind = SemanticErrorKind::MissingReturn(info.name.clone());
            self.error(kind, method.name.span);
        }
    }

    fn check_constructor(&mut self, id: ClassId, constructor: &ConstructorDecl, span: Span) {
        let parameters = self
            .analysis
            .classes
            .class(id)
            .constructor_parameters()
            .to_vec();
        self.context = BodyContext::new(id, ContextKind::Constructor, Type::Void);
        self.context.initialization = Some(Initialization {
            pending_fields: self.fields_needing_constructor(id),
            needs_super: self.base_needs_arguments(id),
            super_called: false,
        });
        self.check_body(&constructor.parameters, &parameters, &constructor.body);
        let Some(initialization) = self.context.initialization.take() else {
            return;
        };
        let class = self.analysis.classes.class(id);
        let mut problems: Vec<SemanticErrorKind> = initialization
            .pending_fields
            .iter()
            .filter_map(|index| class.fields.get(*index))
            .map(|field| SemanticErrorKind::FieldNotInitialized(field.name.clone()))
            .collect();
        if initialization.needs_super {
            problems.push(SemanticErrorKind::MissingSuperInit(self.base_name(id)));
        }
        for problem in problems {
            self.error(problem, span);
        }
    }

    /// A class without `init` can only be created if nothing needs one.
    fn check_implicit_constructor(&mut self, declaration: &ClassDecl, id: ClassId) {
        let class = self.analysis.classes.class(id);
        let mut problems: Vec<(SemanticErrorKind, Span)> = self
            .fields_needing_constructor(id)
            .iter()
            .filter_map(|index| class.fields.get(*index))
            .map(|field| {
                let kind = SemanticErrorKind::FieldNotInitialized(field.name.clone());
                (kind, field.span)
            })
            .collect();
        if self.base_needs_arguments(id) {
            let kind = SemanticErrorKind::MissingSuperInit(self.base_name(id));
            problems.push((kind, declaration.name.span));
        }
        for (problem, span) in problems {
            self.error(problem, span);
        }
    }

    /// Declares the parameters, then checks the body.
    fn check_body(
        &mut self,
        parameters: &[Parameter],
        infos: &[ParameterInfo],
        body: &crate::ast::Block,
    ) {
        self.push_scope(Vec::new());
        for (parameter, info) in parameters.iter().zip(infos) {
            self.declare_local(&parameter.name, info.ty.clone(), false);
        }
        self.block(body, Vec::new());
        self.pop_scope();
    }

    fn fields_needing_constructor(&self, id: ClassId) -> Vec<usize> {
        let class = self.analysis.classes.class(id);
        (0..class.fields.len())
            .filter(|index| {
                class
                    .fields
                    .get(*index)
                    .is_some_and(FieldInfo::needs_constructor)
            })
            .collect()
    }

    pub(super) fn base_needs_arguments(&self, id: ClassId) -> bool {
        let classes = &self.analysis.classes;
        classes
            .class(id)
            .base
            .is_some_and(|base| !classes.class(base).constructor_parameters().is_empty())
    }

    fn base_name(&self, id: ClassId) -> String {
        let classes = &self.analysis.classes;
        classes
            .class(id)
            .base
            .map(|base| classes.class(base).name.clone())
            .unwrap_or_default()
    }
}

/// The declarations whose class was registered, with its identifier.
fn declared<'ast>(
    file: &'ast SourceFile,
    ids: &'ast [Option<ClassId>],
) -> impl Iterator<Item = (&'ast ClassDecl, ClassId)> {
    file.classes
        .iter()
        .zip(ids)
        .filter_map(|(declaration, id)| id.map(|id| (declaration, id)))
}

/// What is wrong with a member named like a member of a base class.
fn override_problem(
    name: &str,
    base: String,
    method: Option<&MethodInfo>,
    base_method: Option<&MethodInfo>,
) -> Option<SemanticErrorKind> {
    let name = name.to_owned();
    let (Some(method), Some(base_method)) = (method, base_method) else {
        return Some(SemanticErrorKind::MemberShadowsBase { name, base });
    };
    if method.is_static || base_method.is_static || base_method.is_private {
        Some(SemanticErrorKind::MemberShadowsBase { name, base })
    } else if !method.is_override {
        Some(SemanticErrorKind::MissingOverride { name, base })
    } else if !method.has_same_signature(base_method) {
        Some(SemanticErrorKind::OverrideMismatch { name, base })
    } else if method.execution != base_method.execution {
        Some(SemanticErrorKind::AsyncOverrideMismatch {
            name,
            base,
            base_is_async: base_method.is_async(),
        })
    } else {
        None
    }
}
