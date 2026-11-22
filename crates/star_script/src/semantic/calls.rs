//! Calls: methods, constructors, builtin methods and `super`.

use super::checker::{Checker, ContextKind, NameTarget};
use super::expressions::{Receiver, wrap_null_safe};
use super::natives::{INPUT_CLASS, builtin_method};
use super::{ClassId, ClassOrigin, MemberRef, MethodInfo, Resolution, SemanticErrorKind, Type};
use crate::Span;
use crate::ast::{Expr, ExprKind, Literal, Name};

/// Name of the constructor in `super.init(…)`.
const CONSTRUCTOR_NAME: &str = "init";

impl Checker<'_> {
    /// `callee(arguments)`, returning the type of the result.
    pub(super) fn call(&mut self, callee: &Expr, arguments: &[Expr], span: Span) -> Type {
        match &callee.kind {
            ExprKind::Name(name) => self.call_by_name(callee, name, arguments, span),
            ExprKind::Member {
                object,
                name,
                null_safe,
            } if matches!(object.kind, ExprKind::Super) => {
                if *null_safe {
                    self.error(SemanticErrorKind::InvalidSuper, object.span);
                }
                self.call_super(callee, object, name, arguments, span)
            }
            ExprKind::Member {
                object,
                name,
                null_safe,
            } => self.call_member(callee, object, name, *null_safe, arguments, span),
            _ => {
                self.expression(callee, None);
                self.not_callable(callee, arguments)
            }
        }
    }

    /// `method(…)` on `self`, or `Class(…)` to create an object.
    fn call_by_name(&mut self, callee: &Expr, name: &str, arguments: &[Expr], span: Span) -> Type {
        match self.resolve_name(name) {
            NameTarget::Member(found) => {
                let MemberRef::Method(index) = found.member else {
                    self.expression(callee, None);
                    return self.not_callable(callee, arguments);
                };
                self.check_private(found, name, callee.span);
                let Some(method) = self.method_info(found.class, index) else {
                    return self.unchecked_call(callee, arguments);
                };
                if !method.is_static && !self.require_self(callee.span) {
                    return self.unchecked_call(callee, arguments);
                }
                self.method_call(callee, found.class, index, &method, false, arguments, span)
            }
            NameTarget::Class(class) => self.construct(callee, class, arguments, span),
            NameTarget::Local(..) => {
                self.expression(callee, None);
                self.not_callable(callee, arguments)
            }
            NameTarget::Unknown => {
                self.error(SemanticErrorKind::UnknownName(name.to_owned()), callee.span);
                self.unchecked_call(callee, arguments)
            }
        }
    }

    fn construct(&mut self, callee: &Expr, class: ClassId, arguments: &[Expr], span: Span) -> Type {
        let info = self.analysis.classes.class(class);
        let is_private = info.constructor.as_ref().is_some_and(|c| c.is_private);
        let parameters: Vec<Type> = info
            .constructor_parameters()
            .iter()
            .map(|parameter| parameter.ty.clone())
            .collect();
        if is_private && class != self.context.class {
            let kind = SemanticErrorKind::PrivateMember {
                name: CONSTRUCTOR_NAME.to_owned(),
                owner: info.name.clone(),
            };
            self.error(kind, callee.span);
        }
        let ty = Type::Class(class);
        self.analysis
            .record(callee, ty.clone(), Some(Resolution::Constructor(class)));
        self.arguments(&parameters, arguments, span);
        ty
    }

    /// `object.method(…)`, `object?.method(…)` or `Class.method(…)`.
    fn call_member(
        &mut self,
        callee: &Expr,
        object: &Expr,
        name: &Name,
        null_safe: bool,
        arguments: &[Expr],
        span: Span,
    ) -> Type {
        let receiver = match self.receiver(object) {
            Receiver::Class(class) => {
                return self.call_static(callee, class, name, arguments, span);
            }
            Receiver::Value(ty) => ty,
        };
        let Some(receiver) = self.unwrap_receiver(&receiver, null_safe, object) else {
            return self.unchecked_call(callee, arguments);
        };
        let Type::Class(class) = receiver else {
            return self.call_builtin(callee, &receiver, name, null_safe, arguments, span);
        };
        let Some(found) = self.find_member_or_report(class, name) else {
            return self.unchecked_call(callee, arguments);
        };
        let MemberRef::Method(index) = found.member else {
            return self.not_callable(callee, arguments);
        };
        self.check_private(found, &name.text, name.span);
        let Some(method) = self.method_info(found.class, index) else {
            return self.unchecked_call(callee, arguments);
        };
        if method.is_static {
            let kind = SemanticErrorKind::StaticThroughObject(name.text.clone());
            self.error(kind, name.span);
        }
        self.method_call(
            callee,
            found.class,
            index,
            &method,
            null_safe,
            arguments,
            span,
        )
    }

    fn call_static(
        &mut self,
        callee: &Expr,
        class: ClassId,
        name: &Name,
        arguments: &[Expr],
        span: Span,
    ) -> Type {
        let Some(found) = self.find_member_or_report(class, name) else {
            return self.unchecked_call(callee, arguments);
        };
        let MemberRef::Method(index) = found.member else {
            return self.not_callable(callee, arguments);
        };
        self.check_private(found, &name.text, name.span);
        let Some(method) = self.method_info(found.class, index) else {
            return self.unchecked_call(callee, arguments);
        };
        if !method.is_static {
            self.error(SemanticErrorKind::NotStatic(name.text.clone()), name.span);
            return self.unchecked_call(callee, arguments);
        }
        if self.is_input_class(found.class) {
            self.check_action_names(arguments);
        }
        self.method_call(callee, found.class, index, &method, false, arguments, span)
    }

    fn is_input_class(&self, class: ClassId) -> bool {
        let info = self.analysis.classes.class(class);
        info.origin == ClassOrigin::Native && info.name == INPUT_CLASS
    }

    /// Action names written as text are checked against the project's
    /// actions, so a typo is a compile error. Names computed while playing
    /// can only be checked then: an unknown action is never held.
    fn check_action_names(&mut self, arguments: &[Expr]) {
        for argument in arguments {
            if let ExprKind::Literal(Literal::String(action)) = &argument.kind
                && !self.analysis.settings.has_action(action)
            {
                let kind = SemanticErrorKind::UnknownAction(action.clone());
                self.error(kind, argument.span);
            }
        }
    }

    fn call_builtin(
        &mut self,
        callee: &Expr,
        receiver: &Type,
        name: &Name,
        null_safe: bool,
        arguments: &[Expr],
        span: Span,
    ) -> Type {
        let Some(signature) = builtin_method(receiver, &name.text) else {
            let kind = SemanticErrorKind::UnknownMember {
                owner: self.type_name(receiver),
                name: name.text.clone(),
            };
            self.error(kind, name.span);
            return self.unchecked_call(callee, arguments);
        };
        let result = wrap_null_safe(signature.return_type, null_safe);
        let resolution = Resolution::Builtin(signature.method);
        self.analysis
            .record(callee, result.clone(), Some(resolution));
        self.arguments(&signature.parameters, arguments, span);
        result
    }

    /// `super.method(…)` runs the base version; `super.init(…)` the base constructor.
    fn call_super(
        &mut self,
        callee: &Expr,
        object: &Expr,
        name: &Name,
        arguments: &[Expr],
        span: Span,
    ) -> Type {
        let Some(base) = self.analysis.classes.class(self.context.class).base else {
            self.analysis.record(object, Type::Error, None);
            self.error(SemanticErrorKind::InvalidSuper, object.span);
            return self.unchecked_call(callee, arguments);
        };
        self.analysis.record(object, Type::Class(base), None);
        if name.text == CONSTRUCTOR_NAME {
            return self.call_super_constructor(callee, base, arguments, span);
        }
        if !self.require_self(object.span) {
            return self.unchecked_call(callee, arguments);
        }
        let Some(found) = self.find_member_or_report(base, name) else {
            return self.unchecked_call(callee, arguments);
        };
        let MemberRef::Method(index) = found.member else {
            return self.not_callable(callee, arguments);
        };
        self.check_private(found, &name.text, name.span);
        let Some(method) = self.method_info(found.class, index) else {
            return self.unchecked_call(callee, arguments);
        };
        if method.is_static {
            let kind = SemanticErrorKind::StaticThroughObject(name.text.clone());
            self.error(kind, name.span);
        }
        self.method_call(callee, found.class, index, &method, false, arguments, span)
    }

    fn call_super_constructor(
        &mut self,
        callee: &Expr,
        base: ClassId,
        arguments: &[Expr],
        span: Span,
    ) -> Type {
        let base_info = self.analysis.classes.class(base);
        if base_info.constructor.is_none() {
            let kind = SemanticErrorKind::UnknownMember {
                owner: base_info.name.clone(),
                name: CONSTRUCTOR_NAME.to_owned(),
            };
            self.error(kind, callee.span);
            return self.unchecked_call(callee, arguments);
        }
        let parameters: Vec<Type> = base_info
            .constructor_parameters()
            .iter()
            .map(|parameter| parameter.ty.clone())
            .collect();
        let well_placed = self.context.kind == ContextKind::Constructor
            && self.at_top_level()
            && self
                .context
                .initialization
                .as_ref()
                .is_some_and(|initialization| !initialization.super_called);
        if !well_placed {
            self.error(SemanticErrorKind::MisplacedSuperInit, callee.span);
        } else if let Some(missing) = self.missing_own_fields() {
            self.error(
                SemanticErrorKind::SuperInitBeforeFields(missing),
                callee.span,
            );
        }
        self.analysis
            .record(callee, Type::Void, Some(Resolution::SuperConstructor(base)));
        // The arguments are checked first: they cannot use `self` yet.
        self.arguments(&parameters, arguments, span);
        if let (true, Some(initialization)) = (well_placed, self.context.initialization.as_mut()) {
            initialization.needs_super = false;
            initialization.super_called = true;
        }
        Type::Void
    }

    // --- Shared ---

    #[allow(clippy::too_many_arguments)]
    fn method_call(
        &mut self,
        callee: &Expr,
        class: ClassId,
        index: usize,
        method: &MethodInfo,
        null_safe: bool,
        arguments: &[Expr],
        span: Span,
    ) -> Type {
        let result = if method.return_type == Type::Void {
            Type::Void
        } else {
            wrap_null_safe(method.return_type.clone(), null_safe)
        };
        let resolution = Resolution::Method { class, index };
        self.analysis
            .record(callee, result.clone(), Some(resolution));
        let parameters: Vec<Type> = method
            .parameters
            .iter()
            .map(|parameter| parameter.ty.clone())
            .collect();
        self.arguments(&parameters, arguments, span);
        result
    }

    fn method_info(&self, class: ClassId, index: usize) -> Option<MethodInfo> {
        self.analysis
            .classes
            .class(class)
            .methods
            .get(index)
            .cloned()
    }

    /// Checks each argument against its parameter.
    fn arguments(&mut self, parameters: &[Type], arguments: &[Expr], span: Span) {
        if parameters.len() != arguments.len() {
            let kind = SemanticErrorKind::WrongArgumentCount {
                expected: parameters.len(),
                found: arguments.len(),
            };
            self.error(kind, span);
        }
        for (position, argument) in arguments.iter().enumerate() {
            let parameter = parameters.get(position);
            let found = self.expression(argument, parameter);
            if let Some(parameter) = parameter {
                self.require_assignable(&found, parameter, argument.span);
            }
        }
    }

    fn not_callable(&mut self, callee: &Expr, arguments: &[Expr]) -> Type {
        self.error(SemanticErrorKind::NotCallable, callee.span);
        self.unchecked_call(callee, arguments)
    }

    /// Records the callee as an error and still checks the arguments, so
    /// mistakes inside them are reported too.
    fn unchecked_call(&mut self, callee: &Expr, arguments: &[Expr]) -> Type {
        if self.analysis.expression(callee).is_none() {
            self.analysis.record(callee, Type::Error, None);
        }
        for argument in arguments {
            self.expression(argument, None);
        }
        Type::Error
    }
}
