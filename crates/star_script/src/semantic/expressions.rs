//! Expressions: their types, and what names and members refer to.

use super::checker::{Checker, ContextKind, FreeCall, NameTarget};
use super::narrowing::is_null;
use super::natives::builtin_method;
use super::{ClassId, FoundMember, MemberRef, MethodInfo, Resolution, SemanticErrorKind, Type};
use crate::Span;
use crate::ast::{BinaryOperator, Expr, ExprKind, Literal, Name, TypeRef, UnaryOperator};

/// What is on the left of a `.`.
pub(super) enum Receiver {
    /// A class name, as in `Debug.print(…)`.
    Class(ClassId),
    /// A value of this type.
    Value(Type),
}

type Checked = (Type, Option<Resolution>);

impl Checker<'_> {
    /// Checks `expression` and records its type.
    ///
    /// `expected` is the type the surrounding code needs, if known. It lets
    /// literals adapt (`var speed: float = 2`, `var names: [string] = []`);
    /// whether the result actually fits is checked by the caller.
    pub(super) fn expression(&mut self, expression: &Expr, expected: Option<&Type>) -> Type {
        if self.context.kind == ContextKind::Constant && !is_constant_shape(expression) {
            self.error(SemanticErrorKind::NotConstant, expression.span);
            self.analysis.record(expression, Type::Error, None);
            return Type::Error;
        }
        let (ty, resolution) = self.expression_kind(expression, expected);
        self.analysis.record(expression, ty.clone(), resolution);
        ty
    }

    fn expression_kind(&mut self, expression: &Expr, expected: Option<&Type>) -> Checked {
        let span = expression.span;
        let ty = match &expression.kind {
            ExprKind::Literal(literal) => self.literal(literal, expected, false, span),
            ExprKind::Name(name) => return self.name_value(name, span),
            ExprKind::Member {
                object,
                name,
                null_safe,
            } => return self.member_value(object, name, *null_safe),
            ExprKind::SelfValue => {
                if self.require_self(span) {
                    Type::Class(self.context.class)
                } else {
                    Type::Error
                }
            }
            ExprKind::Super => {
                self.error(SemanticErrorKind::InvalidSuper, span);
                Type::Error
            }
            ExprKind::Unary {
                operator: UnaryOperator::Negate,
                operand,
            } if matches!(operand.kind, ExprKind::Literal(Literal::Integer(_))) => {
                self.negative_literal(operand, expected, span)
            }
            ExprKind::Unary { operator, operand } => self.unary(*operator, operand, expected),
            ExprKind::Binary {
                operator,
                left,
                right,
            } => self.binary(*operator, left, right, expected, span),
            ExprKind::TypeTest { value, type_ref } => self.type_test(value, type_ref),
            ExprKind::Cast { value, type_ref } => self.cast(value, type_ref, span),
            ExprKind::Call { callee, arguments } => self.call(callee, arguments, span),
            ExprKind::Index { object, index } => self.index(object, index),
            ExprKind::Array(elements) => self.array(elements, expected, span),
            ExprKind::Await(value) => self.await_value(value, span),
        };
        (ty, None)
    }

    /// `await call`: the value of an `async` method once it finishes.
    fn await_value(&mut self, value: &Expr, span: Span) -> Type {
        if !self.context.is_async {
            self.error(SemanticErrorKind::AwaitOutsideAsync, span);
        }
        self.context.free_call = Some(FreeCall {
            span: value.span,
            awaited: true,
        });
        let ty = self.expression(value, None);
        if !self.is_awaitable_call(value) {
            self.error(SemanticErrorKind::AwaitWithoutAsyncCall, value.span);
            return Type::Error;
        }
        if let ExprKind::Call { callee, .. } = &value.kind
            && let ExprKind::Member {
                null_safe: true, ..
            } = callee.kind
        {
            self.error(SemanticErrorKind::NullSafeAwait, value.span);
        }
        ty
    }

    /// Whether `value` calls an `async` method or a `parallel` function.
    /// Calls whose method is unknown count, so their mistake is reported
    /// once.
    fn is_awaitable_call(&self, value: &Expr) -> bool {
        let ExprKind::Call { callee, .. } = &value.kind else {
            return false;
        };
        match self.analysis.expression(callee) {
            Some(info) => match info.resolution {
                Some(Resolution::Method { class, index }) => self
                    .analysis
                    .classes
                    .class(class)
                    .methods
                    .get(index)
                    .is_some_and(MethodInfo::is_awaitable),
                _ => info.ty == Type::Error,
            },
            None => true,
        }
    }

    // --- Names and members ---

    fn name_value(&mut self, name: &str, span: Span) -> Checked {
        match self.resolve_name(name) {
            NameTarget::Local(id, ty) => (ty, Some(Resolution::Local(id))),
            NameTarget::Member(found) => {
                self.check_private(found, name, span);
                match found.member {
                    MemberRef::Field(index) => {
                        if !self.require_self(span) {
                            return (Type::Error, None);
                        }
                        self.field_value(found.class, index)
                    }
                    MemberRef::Constant(index) => self.constant_value(found.class, index, span),
                    MemberRef::Method(_) => {
                        self.error(SemanticErrorKind::NotAValue(name.to_owned()), span);
                        (Type::Error, None)
                    }
                }
            }
            NameTarget::Class(id) => {
                self.error(SemanticErrorKind::NotAValue(name.to_owned()), span);
                (Type::Error, Some(Resolution::Class(id)))
            }
            NameTarget::Unknown => {
                self.error(SemanticErrorKind::UnknownName(name.to_owned()), span);
                (Type::Error, None)
            }
        }
    }

    fn member_value(&mut self, object: &Expr, name: &Name, null_safe: bool) -> Checked {
        if matches!(object.kind, ExprKind::Super) {
            self.analysis.record(object, Type::Error, None);
            self.error(SemanticErrorKind::InvalidSuper, object.span);
            return (Type::Error, None);
        }
        match self.receiver(object) {
            Receiver::Class(class) => self.static_member(class, name),
            Receiver::Value(ty) => {
                let Some(receiver) = self.unwrap_receiver(&ty, null_safe, object) else {
                    return (Type::Error, None);
                };
                let (ty, resolution) = self.instance_member(&receiver, name);
                (wrap_null_safe(ty, null_safe), resolution)
            }
        }
    }

    fn static_member(&mut self, class: ClassId, name: &Name) -> Checked {
        let Some(found) = self.find_member_or_report(class, name) else {
            return (Type::Error, None);
        };
        self.check_private(found, &name.text, name.span);
        match found.member {
            MemberRef::Constant(index) => self.constant_value(found.class, index, name.span),
            MemberRef::Field(_) => {
                self.error(SemanticErrorKind::NotStatic(name.text.clone()), name.span);
                (Type::Error, None)
            }
            MemberRef::Method(_) => {
                self.error(SemanticErrorKind::NotAValue(name.text.clone()), name.span);
                (Type::Error, None)
            }
        }
    }

    fn instance_member(&mut self, receiver: &Type, name: &Name) -> Checked {
        let Type::Class(class) = receiver else {
            let kind = if builtin_method(receiver, &name.text).is_some() {
                SemanticErrorKind::NotAValue(name.text.clone())
            } else {
                SemanticErrorKind::UnknownMember {
                    owner: self.type_name(receiver),
                    name: name.text.clone(),
                }
            };
            self.error(kind, name.span);
            return (Type::Error, None);
        };
        let Some(found) = self.find_member_or_report(*class, name) else {
            return (Type::Error, None);
        };
        self.check_private(found, &name.text, name.span);
        match found.member {
            MemberRef::Field(index) => self.field_value(found.class, index),
            MemberRef::Constant(index) => self.constant_value(found.class, index, name.span),
            MemberRef::Method(_) => {
                self.error(SemanticErrorKind::NotAValue(name.text.clone()), name.span);
                (Type::Error, None)
            }
        }
    }

    pub(super) fn find_member_or_report(
        &mut self,
        class: ClassId,
        name: &Name,
    ) -> Option<FoundMember> {
        let found = self.analysis.classes.find_member(class, &name.text);
        if found.is_none() {
            let kind = SemanticErrorKind::UnknownMember {
                owner: self.analysis.classes.class(class).name.clone(),
                name: name.text.clone(),
            };
            self.error(kind, name.span);
        }
        found
    }

    fn field_value(&self, class: ClassId, index: usize) -> Checked {
        let ty = self
            .analysis
            .classes
            .class(class)
            .fields
            .get(index)
            .map_or(Type::Error, |field| field.ty.clone());
        (ty, Some(Resolution::Field { class, index }))
    }

    fn constant_value(&mut self, class: ClassId, index: usize, span: Span) -> Checked {
        let resolution = Some(Resolution::Constant { class, index });
        let Some(constant) = self.analysis.classes.class(class).constants.get(index) else {
            return (Type::Error, resolution);
        };
        if !constant.resolved {
            let kind = SemanticErrorKind::ConstantUsedBeforeDeclaration(constant.name.clone());
            self.error(kind, span);
            return (Type::Error, resolution);
        }
        (constant.ty.clone(), resolution)
    }

    /// Reports the use of a private member outside the class that declares it.
    pub(super) fn check_private(&mut self, found: FoundMember, name: &str, span: Span) {
        let class = self.analysis.classes.class(found.class);
        let is_private = match found.member {
            MemberRef::Field(index) => class.fields.get(index).is_some_and(|item| item.is_private),
            MemberRef::Constant(index) => class
                .constants
                .get(index)
                .is_some_and(|item| item.is_private),
            MemberRef::Method(index) => {
                class.methods.get(index).is_some_and(|item| item.is_private)
            }
        };
        if is_private && found.class != self.context.class {
            let kind = SemanticErrorKind::PrivateMember {
                name: name.to_owned(),
                owner: class.name.clone(),
            };
            self.error(kind, span);
        }
    }

    /// Checks what is left of a `.`: a class name or a value.
    pub(super) fn receiver(&mut self, object: &Expr) -> Receiver {
        if let ExprKind::Name(name) = &object.kind {
            if let NameTarget::Class(id) = self.resolve_name(name) {
                self.analysis
                    .record(object, Type::Class(id), Some(Resolution::Class(id)));
                return Receiver::Class(id);
            }
        }
        Receiver::Value(self.expression(object, None))
    }

    /// The type to look members up in, enforcing null safety: `.` needs a
    /// value that is never `null`, and `?.` one that may be.
    pub(super) fn unwrap_receiver(
        &mut self,
        receiver: &Type,
        null_safe: bool,
        object: &Expr,
    ) -> Option<Type> {
        if receiver.is_error() {
            return None;
        }
        let name = self.type_name(receiver);
        match (receiver.is_nullable(), null_safe) {
            (true, false) => {
                let kind = match self.mutable_local_name(object) {
                    Some(variable) => SemanticErrorKind::PossiblyNullVariable(variable),
                    None => SemanticErrorKind::PossiblyNull(name),
                };
                self.error(kind, object.span);
                None
            }
            (false, true) => {
                self.error(SemanticErrorKind::NeverNull(name), object.span);
                Some(receiver.clone())
            }
            _ => Some(receiver.non_null()),
        }
    }

    /// The name of the `var` that `expression` reads, if it is one.
    fn mutable_local_name(&self, expression: &Expr) -> Option<String> {
        let ExprKind::Name(name) = &expression.kind else {
            return None;
        };
        match self.analysis.expression(expression)?.resolution? {
            Resolution::Local(id) if self.is_mutable(id) => Some(name.clone()),
            _ => None,
        }
    }

    // --- Operators ---

    /// The type of `literal`; an integer must fit in it (`negated`: with
    /// a `-` before it, which lets `-32768` be a `short`).
    fn literal(
        &mut self,
        literal: &Literal,
        expected: Option<&Type>,
        negated: bool,
        span: Span,
    ) -> Type {
        let ty = literal_type(literal, expected);
        if let (Literal::Integer(value), Some((min, max))) = (literal, ty.integer_range()) {
            let value = if negated { -*value } else { *value };
            if !(min..=max).contains(&value) {
                let kind = SemanticErrorKind::IntegerOutOfRange {
                    value,
                    ty: self.type_name(&ty),
                    min,
                    max,
                };
                self.error(kind, span);
            }
        }
        ty
    }

    /// `-3`: an integer literal with `-`, whose value must fit in its type.
    fn negative_literal(&mut self, literal: &Expr, expected: Option<&Type>, span: Span) -> Type {
        let ExprKind::Literal(value) = &literal.kind else {
            return Type::Error;
        };
        let ty = self.literal(value, expected, true, span);
        self.analysis.record(literal, ty.clone(), None);
        ty
    }

    fn unary(&mut self, operator: UnaryOperator, operand: &Expr, expected: Option<&Type>) -> Type {
        let (expected, valid): (Option<&Type>, fn(&Type) -> bool) = match operator {
            UnaryOperator::Negate => (expected, Type::is_numeric),
            UnaryOperator::Not => (Some(&Type::Bool), |ty| *ty == Type::Bool),
        };
        let found = self.expression(operand, expected);
        if found.is_error() || valid(&found) {
            return if operator == UnaryOperator::Not {
                Type::Bool
            } else {
                found
            };
        }
        let kind = SemanticErrorKind::InvalidOperand {
            operator: operator.symbol(),
            operand: self.type_name(&found),
        };
        self.error(kind, operand.span);
        Type::Error
    }

    fn binary(
        &mut self,
        operator: BinaryOperator,
        left: &Expr,
        right: &Expr,
        expected: Option<&Type>,
        span: Span,
    ) -> Type {
        match operator {
            BinaryOperator::And | BinaryOperator::Or => self.logical(operator, left, right),
            BinaryOperator::Coalesce => self.coalesce(left, right),
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                self.equality(operator, left, right, span)
            }
            BinaryOperator::Range => {
                self.operands(left, right, None);
                self.error(SemanticErrorKind::RangeOutsideFor, span);
                Type::Error
            }
            BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => {
                let (left_type, right_type) = self.operands(left, right, None);
                let valid = left_type.common_number(&right_type).is_some();
                self.report_operands(operator, &left_type, &right_type, valid, span);
                Type::Bool
            }
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder => {
                let (left_type, right_type) = self.operands(left, right, expected);
                let joins_text = operator == BinaryOperator::Add
                    && left_type == Type::String
                    && right_type == Type::String;
                let result = if joins_text {
                    Some(Type::String)
                } else {
                    left_type.common_number(&right_type)
                };
                let valid = result.is_some();
                if self.report_operands(operator, &left_type, &right_type, valid, span) {
                    result.unwrap_or(Type::Error)
                } else {
                    Type::Error
                }
            }
        }
    }

    /// Reports invalid operands unless one side already has an error.
    /// Returns whether the operation is valid.
    fn report_operands(
        &mut self,
        operator: BinaryOperator,
        left: &Type,
        right: &Type,
        valid: bool,
        span: Span,
    ) -> bool {
        if left.is_error() || right.is_error() {
            return false;
        }
        if !valid {
            let kind = SemanticErrorKind::InvalidOperands {
                operator: operator.symbol(),
                left: self.type_name(left),
                right: self.type_name(right),
            };
            self.error(kind, span);
        }
        valid
    }

    /// Checks both operands so that a number literal adapts to the other
    /// side: in `speed * 2` and `2 * speed`, `2` is a `float` if `speed` is.
    /// The more adaptable side is checked last.
    pub(super) fn operands(
        &mut self,
        left: &Expr,
        right: &Expr,
        expected: Option<&Type>,
    ) -> (Type, Type) {
        if adaptability(left) > adaptability(right) {
            let right_type = self.expression(right, expected);
            let left_type = self.expression(left, Some(&right_type));
            (left_type, right_type)
        } else {
            let left_type = self.expression(left, expected);
            let right_type = self.expression(right, Some(&left_type));
            (left_type, right_type)
        }
    }

    /// `&&` and `||`. The right side knows what the left proved:
    /// in `x != null && x.alive`, `x` is not `null` on the right.
    fn logical(&mut self, operator: BinaryOperator, left: &Expr, right: &Expr) -> Type {
        let left_type = self.expression(left, Some(&Type::Bool));
        self.require_assignable(&left_type, &Type::Bool, left.span);
        let narrowed = self.facts(left, operator == BinaryOperator::And);
        let right_type = self.with_narrowing(narrowed, |checker| {
            checker.expression(right, Some(&Type::Bool))
        });
        self.require_assignable(&right_type, &Type::Bool, right.span);
        Type::Bool
    }

    /// `value ?? fallback`: the type without `?`, unless the fallback may be `null` too.
    fn coalesce(&mut self, left: &Expr, right: &Expr) -> Type {
        let left_type = self.expression(left, None);
        if left_type.is_error() {
            self.expression(right, None);
            return Type::Error;
        }
        if !left_type.is_nullable() {
            let kind = SemanticErrorKind::NeverNull(self.type_name(&left_type));
            self.error(kind, left.span);
        }
        let value = left_type.non_null();
        let fallback = self.expression(right, Some(&value));
        let classes = &self.analysis.classes;
        if classes.is_assignable(&fallback, &value) {
            value
        } else if classes.is_assignable(&fallback, &value.clone().nullable()) {
            value.nullable()
        } else {
            self.require_assignable(&fallback, &value, right.span);
            value
        }
    }

    fn equality(
        &mut self,
        operator: BinaryOperator,
        left: &Expr,
        right: &Expr,
        span: Span,
    ) -> Type {
        let (left_type, right_type) = self.operands(left, right, None);
        if left_type.is_error() || right_type.is_error() {
            return Type::Bool;
        }
        let compared_with_null = match (is_null(left), is_null(right)) {
            (false, true) => Some((&left_type, left.span)),
            (true, false) => Some((&right_type, right.span)),
            _ => None,
        };
        if let Some((compared, span)) = compared_with_null {
            if !compared.is_nullable() {
                let kind = SemanticErrorKind::NeverNull(self.type_name(compared));
                self.error(kind, span);
            }
            return Type::Bool;
        }
        let classes = &self.analysis.classes;
        let comparable = classes.is_assignable(&left_type, &right_type)
            || classes.is_assignable(&right_type, &left_type);
        self.report_operands(operator, &left_type, &right_type, comparable, span);
        Type::Bool
    }

    /// `value is Class`
    fn type_test(&mut self, value: &Expr, type_ref: &TypeRef) -> Type {
        let value_type = self.expression(value, None);
        let tested = self.resolve_type(type_ref);
        if value_type.is_error() || tested.is_error() {
            return Type::Bool;
        }
        let problem = if value_type.class().is_none() {
            Some((value_type, value.span))
        } else if !matches!(tested, Type::Class(_)) {
            Some((tested, type_ref.span))
        } else {
            None
        };
        if let Some((operand, span)) = problem {
            let kind = SemanticErrorKind::InvalidOperand {
                operator: "is",
                operand: self.type_name(&operand),
            };
            self.error(kind, span);
        }
        Type::Bool
    }

    /// `value as Type`: between `int` and `float`, or between related classes.
    /// Converting to a subclass may fail, so it gives a `?` type.
    fn cast(&mut self, value: &Expr, type_ref: &TypeRef, span: Span) -> Type {
        let value_type = self.expression(value, None);
        let target = self.resolve_type(type_ref);
        if value_type.is_error() || target.is_error() {
            return target;
        }
        if value_type.is_numeric() && target.is_numeric() || value_type == target {
            return target;
        }
        if let (Some(from), Some(to)) = (value_type.class(), target.class()) {
            let classes = &self.analysis.classes;
            if classes.is_subclass(from, to) {
                return if value_type.is_nullable() {
                    target.nullable()
                } else {
                    target
                };
            }
            if classes.is_subclass(to, from) {
                return target.nullable();
            }
        }
        let kind = SemanticErrorKind::InvalidCast {
            from: self.type_name(&value_type),
            to: self.type_name(&target),
        };
        self.error(kind, span);
        Type::Error
    }

    fn index(&mut self, object: &Expr, index: &Expr) -> Type {
        let list = self.expression(object, None);
        let index_type = self.expression(index, Some(&Type::Int));
        self.require_assignable(&index_type, &Type::Int, index.span);
        match list {
            Type::List(element) => *element,
            Type::Error => Type::Error,
            other => {
                let name = self.type_name(&other);
                let kind = if other.is_nullable() {
                    SemanticErrorKind::PossiblyNull(name)
                } else {
                    SemanticErrorKind::NotIndexable(name)
                };
                self.error(kind, object.span);
                Type::Error
            }
        }
    }

    /// `[a, b]`: the element type comes from the expected type or the elements.
    fn array(&mut self, elements: &[Expr], expected: Option<&Type>, span: Span) -> Type {
        if let Some(Type::List(element)) = expected.map(Type::non_null) {
            for item in elements {
                let found = self.expression(item, Some(&element));
                self.require_assignable(&found, &element, item.span);
            }
            return Type::List(element);
        }
        let found: Vec<Type> = elements
            .iter()
            .map(|item| self.expression(item, None))
            .collect();
        let Some(first) = found.iter().find(|ty| **ty != Type::Null).cloned() else {
            let kind = if elements.is_empty() {
                SemanticErrorKind::CannotInferEmptyList
            } else {
                SemanticErrorKind::CannotInferNull
            };
            self.error(kind, span);
            return Type::Error;
        };
        let element = if found.contains(&Type::Null) {
            first.nullable()
        } else {
            first
        };
        for (item, ty) in elements.iter().zip(&found) {
            self.require_assignable(ty, &element, item.span);
        }
        Type::list(element)
    }
}

fn literal_type(literal: &Literal, expected: Option<&Type>) -> Type {
    match (literal, expected.map(Type::non_null)) {
        (Literal::Integer(_), Some(number)) if number.is_numeric() => number,
        (Literal::Integer(_), _) => Type::Int,
        (Literal::Float(_), Some(Type::Double)) => Type::Double,
        (literal, _) => other_literal_type(literal),
    }
}

fn other_literal_type(literal: &Literal) -> Type {
    match literal {
        Literal::Integer(_) => Type::Int,
        Literal::Float(_) => Type::Float,
        Literal::String(_) => Type::String,
        Literal::Bool(_) => Type::Bool,
        Literal::Null => Type::Null,
    }
}

/// How many number types a value may take: an integer literal (`3`, `-3`)
/// takes any, a decimal one `float` or `double`, anything else its own.
fn adaptability(expression: &Expr) -> u8 {
    match &expression.kind {
        ExprKind::Literal(Literal::Integer(_)) => 2,
        ExprKind::Literal(Literal::Float(_)) => 1,
        ExprKind::Unary {
            operator: UnaryOperator::Negate,
            operand,
        } => adaptability(operand),
        _ => 0,
    }
}

/// The kinds of expression a constant's value may contain.
const fn is_constant_shape(expression: &Expr) -> bool {
    matches!(
        expression.kind,
        ExprKind::Literal(_)
            | ExprKind::Name(_)
            | ExprKind::Member { .. }
            | ExprKind::Unary { .. }
            | ExprKind::Binary { .. }
            | ExprKind::Cast { .. }
    )
}

/// After `?.` the result may be `null`, unless there is no result at all.
pub(super) fn wrap_null_safe(ty: Type, null_safe: bool) -> Type {
    if null_safe { ty.nullable() } else { ty }
}
