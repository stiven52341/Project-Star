//! State shared by every part of semantic analysis.

use super::natives::basic_type;
use super::{
    Analysis, ClassId, FoundMember, LocalId, LocalInfo, SemanticError, SemanticErrorKind, Type,
};
use crate::Span;
use crate::ast::{Name, SourceFile, TypeRef, TypeRefKind};

/// The checker: walks the syntax tree, filling an [`Analysis`] and errors.
pub(super) struct Checker<'ast> {
    pub(super) file: &'ast SourceFile,
    pub(super) analysis: Analysis,
    pub(super) errors: Vec<SemanticError>,
    pub(super) context: BodyContext,
}

/// What kind of code is being checked, which decides whether `self` exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ContextKind {
    /// An instance method: `self` is the object.
    Instance,
    /// A static method: there is no object.
    Static,
    /// An `init`: `self` exists once every field has a value.
    Constructor,
    /// The value of a field: there is no object yet.
    FieldInitializer,
    /// The value of a constant: only constant expressions.
    Constant,
}

/// The method, constructor or initializer being checked.
pub(super) struct BodyContext {
    pub(super) class: ClassId,
    pub(super) kind: ContextKind,
    pub(super) return_type: Type,
    pub(super) loop_depth: usize,
    pub(super) scopes: Vec<Scope>,
    /// Number of scopes at the top level of the body (parameters + body block).
    pub(super) top_level_depth: usize,
    pub(super) initialization: Option<Initialization>,
    /// Whether the body is an `async` method, where `await` can be used.
    pub(super) is_async: bool,
    /// The call that may be of an `async` method: the one right after
    /// `await`, or a statement of its own, which starts a coroutine.
    pub(super) free_call: Option<Span>,
}

impl BodyContext {
    pub(super) const fn new(class: ClassId, kind: ContextKind, return_type: Type) -> Self {
        Self {
            class,
            kind,
            return_type,
            loop_depth: 0,
            scopes: Vec::new(),
            top_level_depth: 2,
            initialization: None,
            is_async: false,
            free_call: None,
        }
    }
}

/// Locals declared in a block, and what is known about locals there.
#[derive(Default)]
pub(super) struct Scope {
    locals: Vec<(String, LocalId)>,
    narrowed: Vec<(LocalId, Type)>,
}

/// Progress of an `init`: the object is complete when nothing is pending.
pub(super) struct Initialization {
    /// Fields of the class that still need a value, by index.
    pub(super) pending_fields: Vec<usize>,
    /// Whether `super.init(…)` must still be called.
    pub(super) needs_super: bool,
    /// Whether `super.init(…)` was called.
    pub(super) super_called: bool,
}

/// What a name refers to, before it is recorded.
pub(super) enum NameTarget {
    Local(LocalId, Type),
    Member(FoundMember),
    Class(ClassId),
    Unknown,
}

impl<'ast> Checker<'ast> {
    pub(super) fn new(file: &'ast SourceFile, analysis: Analysis) -> Self {
        // A placeholder: every body sets its own context before being checked.
        let context = BodyContext::new(ClassId::new(0), ContextKind::Constant, Type::Void);
        Self {
            file,
            analysis,
            errors: Vec::new(),
            context,
        }
    }

    pub(super) fn error(&mut self, kind: SemanticErrorKind, span: Span) {
        self.errors.push(SemanticError::new(kind, span));
    }

    pub(super) fn type_name(&self, ty: &Type) -> String {
        self.analysis.classes.type_name(ty)
    }

    /// Reports a mismatch unless `found` fits where `expected` is needed.
    pub(super) fn require_assignable(&mut self, found: &Type, expected: &Type, span: Span) {
        if !self.analysis.classes.is_assignable(found, expected) {
            let kind = SemanticErrorKind::TypeMismatch {
                expected: self.type_name(expected),
                found: self.type_name(found),
            };
            self.error(kind, span);
        }
    }

    /// The type written as `type_ref`, reporting unknown names.
    pub(super) fn resolve_type(&mut self, type_ref: &TypeRef) -> Type {
        match self.find_type(type_ref) {
            Ok(ty) => ty,
            Err(name) => {
                self.error(SemanticErrorKind::UnknownType(name), type_ref.span);
                Type::Error
            }
        }
    }

    /// The type written as `type_ref`, or the first unknown name in it.
    pub(super) fn find_type(&self, type_ref: &TypeRef) -> Result<Type, String> {
        match &type_ref.kind {
            TypeRefKind::Named(name) => basic_type(name)
                .or_else(|| self.analysis.classes.find(name).map(Type::Class))
                .ok_or_else(|| name.clone()),
            TypeRefKind::List(element) => self.find_type(element).map(Type::list),
            TypeRefKind::Nullable(inner) => self.find_type(inner).map(Type::nullable),
        }
    }

    // --- Scopes and locals ---

    pub(super) fn push_scope(&mut self, narrowed: Vec<(LocalId, Type)>) {
        self.context.scopes.push(Scope {
            locals: Vec::new(),
            narrowed,
        });
    }

    pub(super) fn pop_scope(&mut self) {
        self.context.scopes.pop();
    }

    /// Runs `check` with extra knowledge, such as `x` not being `null`.
    pub(super) fn with_narrowing<T>(
        &mut self,
        narrowed: Vec<(LocalId, Type)>,
        check: impl FnOnce(&mut Self) -> T,
    ) -> T {
        self.push_scope(narrowed);
        let result = check(self);
        self.pop_scope();
        result
    }

    /// Adds knowledge to the current scope, from here to its end.
    pub(super) fn narrow(&mut self, narrowed: Vec<(LocalId, Type)>) {
        if let Some(scope) = self.context.scopes.last_mut() {
            scope.narrowed.extend(narrowed);
        }
    }

    /// Declares a local in the current scope. A name still visible from an
    /// enclosing scope cannot be reused, so no variable hides another.
    pub(super) fn declare_local(&mut self, name: &Name, ty: Type, is_mutable: bool) -> LocalId {
        if self.find_local(&name.text).is_some() {
            self.error(
                SemanticErrorKind::DuplicateLocal(name.text.clone()),
                name.span,
            );
        }
        let local = LocalInfo {
            name: name.text.clone(),
            ty,
            is_mutable,
        };
        let id = self.analysis.add_local(name, local);
        if let Some(scope) = self.context.scopes.last_mut() {
            scope.locals.push((name.text.clone(), id));
        }
        id
    }

    fn find_local(&self, name: &str) -> Option<LocalId> {
        self.context.scopes.iter().rev().find_map(|scope| {
            scope
                .locals
                .iter()
                .rev()
                .find(|(local_name, _)| local_name == name)
                .map(|(_, id)| *id)
        })
    }

    /// The type of a local here: the narrowest known, else the declared one.
    fn local_type(&self, id: LocalId) -> Type {
        let narrowed = self.context.scopes.iter().rev().find_map(|scope| {
            scope
                .narrowed
                .iter()
                .rev()
                .find(|(local, _)| *local == id)
                .map(|(_, ty)| ty.clone())
        });
        narrowed.unwrap_or_else(|| self.declared_type(id))
    }

    pub(super) fn declared_type(&self, id: LocalId) -> Type {
        self.analysis
            .local(id)
            .map_or(Type::Error, |local| local.ty.clone())
    }

    pub(super) fn is_mutable(&self, id: LocalId) -> bool {
        self.analysis
            .local(id)
            .is_some_and(|local| local.is_mutable)
    }

    /// What `name` refers to here: a local, then a member of the class or
    /// its bases, then a class.
    pub(super) fn resolve_name(&self, name: &str) -> NameTarget {
        if let Some(id) = self.find_local(name) {
            return NameTarget::Local(id, self.local_type(id));
        }
        if let Some(found) = self.analysis.classes.find_member(self.context.class, name) {
            return NameTarget::Member(found);
        }
        self.analysis
            .classes
            .find(name)
            .map_or(NameTarget::Unknown, NameTarget::Class)
    }

    // --- Access to the current object ---

    /// Reports why `self` cannot be used here, if it cannot.
    pub(super) fn require_self(&mut self, span: Span) -> bool {
        let problem = match self.context.kind {
            ContextKind::Instance => None,
            ContextKind::Static | ContextKind::FieldInitializer | ContextKind::Constant => {
                Some(SemanticErrorKind::SelfUnavailable)
            }
            ContextKind::Constructor => self
                .missing_initialization()
                .map(SemanticErrorKind::SelfBeforeInitialization),
        };
        match problem {
            Some(kind) => {
                self.error(kind, span);
                false
            }
            None => true,
        }
    }

    /// What `init` must still do before `self` is complete, if anything.
    pub(super) fn missing_initialization(&self) -> Option<String> {
        let initialization = self.context.initialization.as_ref()?;
        let mut missing = self.pending_field_names();
        if initialization.needs_super {
            missing.push("`super.init(…)`".to_owned());
        }
        if missing.is_empty() {
            None
        } else {
            Some(missing.join(", "))
        }
    }

    /// The fields of its own class that `init` must still assign, if any.
    pub(super) fn missing_own_fields(&self) -> Option<String> {
        let missing = self.pending_field_names();
        (!missing.is_empty()).then(|| missing.join(", "))
    }

    fn pending_field_names(&self) -> Vec<String> {
        let Some(initialization) = self.context.initialization.as_ref() else {
            return Vec::new();
        };
        let class = self.analysis.classes.class(self.context.class);
        initialization
            .pending_fields
            .iter()
            .filter_map(|index| class.fields.get(*index))
            .map(|field| format!("`{}`", field.name))
            .collect()
    }

    /// Whether the current statement is directly in the body, not nested.
    pub(super) fn at_top_level(&self) -> bool {
        self.context.scopes.len() == self.context.top_level_depth
    }
}
