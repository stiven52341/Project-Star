//! Expressions, with the conversions between number types written out.

use star_script::Span;
use star_script::ast::{BinaryOperator, Expr, ExprKind, Literal, Name, UnaryOperator};
use star_script::semantic::{BuiltinMethod, ClassId, ClassOrigin, MethodInfo, Resolution, Type};

use super::Member;
use super::body::Body;
use crate::error::LowerError;
use crate::tree::{
    Arithmetic, ClassRef, Comparison, Coroutine, Expression, MethodTarget, NumberType,
};
use crate::value::Value;

/// The engine class whose `print` writes a line.
const DEBUG_CLASS: &str = "Debug";
/// The engine class with the game's clock.
const TIME_CLASS: &str = "Time";

impl Body<'_> {
    /// The interpreter's version of `expression`.
    pub(super) fn expression(&self, expression: &Expr) -> Result<Expression, LowerError> {
        let span = expression.span;
        match &expression.kind {
            ExprKind::Literal(literal) => Ok(Expression::Constant(literal_value(
                literal,
                self.type_of(expression)?,
                expression,
            )?)),
            ExprKind::Name(_) => self.name(expression),
            ExprKind::Unary { operator, operand } => {
                let operand = Box::new(self.expression(operand)?);
                Ok(match operator {
                    UnaryOperator::Negate => Expression::Negate(operand),
                    UnaryOperator::Not => Expression::Not(operand),
                })
            }
            ExprKind::Binary {
                operator,
                left,
                right,
            } => self.binary(expression, *operator, left, right),
            ExprKind::Cast { value, .. } => self.cast(expression, value),
            ExprKind::Call { callee, arguments } => self.call(callee, arguments),
            ExprKind::Member {
                object,
                name,
                null_safe,
            } => self.member(expression, object, name, *null_safe),
            ExprKind::Index { object, index } => Ok(Expression::Index {
                list: Box::new(self.expression(object)?),
                index: Box::new(self.expression(index)?),
            }),
            ExprKind::Array(elements) => self.array(expression, elements),
            ExprKind::SelfValue => self.self_value(expression),
            ExprKind::Super => Err(LowerError::internal("`super` used as a value", span)),
            ExprKind::TypeTest { value, type_ref } => Ok(Expression::IsInstance {
                value: Box::new(self.expression(value)?),
                class: self.class_named(&type_ref.to_string(), expression)?,
            }),
            ExprKind::Await(value) => match self.expression(value)? {
                Expression::Start(coroutine) => Ok(Expression::Await(coroutine)),
                _ => Err(LowerError::internal(
                    "`await` of a call that is not `async`",
                    span,
                )),
            },
        }
    }

    /// `expression` converted to `ty`, where analysis allows that.
    pub(super) fn value_as(&self, expression: &Expr, ty: &Type) -> Result<Expression, LowerError> {
        let value = self.expression(expression)?;
        coerce(value, self.type_of(expression)?, ty, expression)
    }

    /// Both sides of an operator, numbers converted to the type they meet
    /// in (`small < count`: `small` as an `int`).
    pub(super) fn common_operands(
        &self,
        left: &Expr,
        right: &Expr,
    ) -> Result<(Expression, Expression), LowerError> {
        let left_type = self.type_of(left)?;
        match left_type.common_number(self.type_of(right)?) {
            Some(common) => Ok((
                self.value_as(left, &common)?,
                self.value_as(right, &common)?,
            )),
            None => Ok((self.expression(left)?, self.expression(right)?)),
        }
    }

    /// What analysis says `expression` refers to.
    pub(super) fn resolution(&self, expression: &Expr) -> Result<Option<Resolution>, LowerError> {
        self.analysis
            .expression(expression)
            .map(|info| info.resolution)
            .ok_or_else(|| LowerError::internal("expression without analysis", expression.span))
    }

    /// The type analysis gave `expression`.
    pub(super) fn type_of(&self, expression: &Expr) -> Result<&Type, LowerError> {
        self.analysis
            .type_of(expression)
            .ok_or_else(|| LowerError::internal("expression without a type", expression.span))
    }

    fn name(&self, expression: &Expr) -> Result<Expression, LowerError> {
        match self.resolution(expression)? {
            // Read where a check proved more about it (`!= null`, `is`),
            // it is still the same value: values carry their class.
            Some(Resolution::Local(local)) => Ok(Expression::Local(self.slot(local, expression)?)),
            Some(Resolution::Constant { class, index }) => self.constant(class, index, expression),
            Some(Resolution::Field { class, index }) => Ok(Expression::Field {
                object: Box::new(self.self_value(expression)?),
                position: self.field_position(class, index, expression)?,
                null_safe: false,
            }),
            _ => Err(LowerError::internal(
                "name that is not a value",
                expression.span,
            )),
        }
    }

    /// `object.name`: a field, or a constant through its class.
    fn member(
        &self,
        expression: &Expr,
        object: &Expr,
        name: &Name,
        null_safe: bool,
    ) -> Result<Expression, LowerError> {
        match self.resolution(expression)? {
            Some(Resolution::Constant { class, index }) => {
                if !matches!(self.resolution(object)?, Some(Resolution::Class(_))) {
                    return Err(LowerError::unsupported("objects", object.span));
                }
                self.constant(class, index, expression)
            }
            Some(Resolution::Field { class, index }) => Ok(Expression::Field {
                object: Box::new(self.expression(object)?),
                position: self.field_position(class, index, expression)?,
                null_safe,
            }),
            _ => Err(LowerError::internal(
                "member that is not a value",
                name.span,
            )),
        }
    }

    /// The value of a constant: a call of the function that computes it.
    fn constant(
        &self,
        class: ClassId,
        index: usize,
        expression: &Expr,
    ) -> Result<Expression, LowerError> {
        let function = self
            .tables
            .functions
            .get(&Member::Constant(class, index))
            .copied()
            .ok_or_else(|| LowerError::internal("constant without a function", expression.span))?;
        Ok(Expression::Call {
            function,
            arguments: Vec::new(),
        })
    }

    fn binary(
        &self,
        expression: &Expr,
        operator: BinaryOperator,
        left: &Expr,
        right: &Expr,
    ) -> Result<Expression, LowerError> {
        let (left, right) = match operator {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder => {
                let ty = self.type_of(expression)?;
                (self.value_as(left, ty)?, self.value_as(right, ty)?)
            }
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                self.equality_operands(left, right)?
            }
            BinaryOperator::Coalesce => {
                // The value is already of the result's type, or the type
                // without `null`; the fallback is converted to it.
                let result = self.type_of(expression)?;
                (self.expression(left)?, self.value_as(right, result)?)
            }
            _ => self.common_operands(left, right)?,
        };
        operation(operator, left, right, expression)
    }

    /// Both sides of `==`, as the type of the side the other converts to.
    fn equality_operands(
        &self,
        left: &Expr,
        right: &Expr,
    ) -> Result<(Expression, Expression), LowerError> {
        let left_type = self.type_of(left)?;
        let right_type = self.type_of(right)?;
        let common = if self.analysis.classes().is_assignable(left_type, right_type) {
            right_type
        } else {
            left_type
        };
        Ok((self.value_as(left, common)?, self.value_as(right, common)?))
    }

    /// `value as Type`: between numbers, to a base class (nothing to do)
    /// or to a subclass (the object, or `null` if it is not one).
    fn cast(&self, expression: &Expr, value: &Expr) -> Result<Expression, LowerError> {
        let target = self.type_of(expression)?;
        let from = self.type_of(value)?;
        let lowered = self.expression(value)?;
        if from == target {
            return Ok(lowered);
        }
        if from.is_numeric() && target.is_numeric() {
            return convert(lowered, target, expression);
        }
        let (Some(from_class), Some(to_class)) = (from.class(), target.class()) else {
            return Err(LowerError::internal(
                "cast between unrelated types",
                expression.span,
            ));
        };
        if self.analysis.classes().is_subclass(from_class, to_class) {
            return Ok(lowered);
        }
        Ok(Expression::Downcast {
            value: Box::new(lowered),
            class: self.class_ref(to_class, expression)?,
        })
    }

    /// The class of the program called `name`, as `is` names it.
    fn class_named(&self, name: &str, expression: &Expr) -> Result<ClassRef, LowerError> {
        let class = self
            .analysis
            .classes()
            .find(name)
            .ok_or_else(|| LowerError::internal("unknown class", expression.span))?;
        self.class_ref(class, expression)
    }

    /// The position of `class`, a class of the program; engine classes
    /// are refused.
    fn class_ref(&self, class: ClassId, expression: &Expr) -> Result<ClassRef, LowerError> {
        if self.analysis.classes().class(class).origin == ClassOrigin::Native {
            return Err(LowerError::unsupported(
                "`is` and `as` with engine classes",
                expression.span,
            ));
        }
        self.tables
            .classes
            .get(&class)
            .copied()
            .ok_or_else(|| LowerError::internal("class without a position", expression.span))
    }

    /// `[a, b, c]`, each element converted to the list's element type.
    fn array(&self, expression: &Expr, elements: &[Expr]) -> Result<Expression, LowerError> {
        let Type::List(element) = self.type_of(expression)? else {
            return Err(LowerError::internal(
                "list without a list type",
                expression.span,
            ));
        };
        let elements = elements
            .iter()
            .map(|item| self.value_as(item, element))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Expression::List(elements))
    }

    fn call(&self, callee: &Expr, arguments: &[Expr]) -> Result<Expression, LowerError> {
        match self.resolution(callee)? {
            Some(Resolution::Method { class, index }) => {
                let call = self.method_call(callee, class, index, arguments)?;
                let is_async = self
                    .analysis
                    .classes()
                    .class(class)
                    .methods
                    .get(index)
                    .is_some_and(MethodInfo::is_async);
                if is_async {
                    started(call, callee.span)
                } else {
                    Ok(call)
                }
            }
            Some(Resolution::Builtin(method)) => self.builtin_call(callee, method, arguments),
            Some(Resolution::Constructor(class)) => self.construct(callee, class, arguments),
            _ => Err(LowerError::internal(
                "call of something that is not callable",
                callee.span,
            )),
        }
    }

    fn method_call(
        &self,
        callee: &Expr,
        class: ClassId,
        index: usize,
        arguments: &[Expr],
    ) -> Result<Expression, LowerError> {
        let info = self.analysis.classes().class(class);
        let method = info
            .methods
            .get(index)
            .ok_or_else(|| LowerError::internal("unknown method", callee.span))?;
        let arguments = method
            .parameters
            .iter()
            .zip(arguments)
            .map(|(parameter, argument)| self.value_as(argument, &parameter.ty))
            .collect::<Result<Vec<_>, _>>()?;
        if info.origin == ClassOrigin::Native {
            return match (info.name.as_str(), method.name.as_str()) {
                (DEBUG_CLASS, "print") => {
                    let message = arguments.into_iter().next().ok_or_else(|| {
                        LowerError::internal("`print` without a message", callee.span)
                    })?;
                    Ok(Expression::Print(Box::new(message)))
                }
                (TIME_CLASS, "now") => Ok(Expression::Now),
                (TIME_CLASS, "wait") => {
                    let seconds = arguments.into_iter().next().ok_or_else(|| {
                        LowerError::internal("`wait` without seconds", callee.span)
                    })?;
                    Ok(Expression::Start(Box::new(Coroutine::Wait(Box::new(
                        seconds,
                    )))))
                }
                (TIME_CLASS, "next_frame") => Ok(Expression::Start(Box::new(Coroutine::NextFrame))),
                _ => Err(LowerError::unsupported(
                    "this method of the engine",
                    callee.span,
                )),
            };
        }
        let function = self
            .tables
            .functions
            .get(&Member::Method(class, index))
            .copied()
            .ok_or_else(|| LowerError::internal("method without a function", callee.span))?;
        if method.is_static {
            return Ok(Expression::Call {
                function,
                arguments,
            });
        }
        let (receiver, null_safe) = match &callee.kind {
            ExprKind::Name(_) => (self.self_value(callee)?, false),
            ExprKind::Member { object, .. } if matches!(object.kind, ExprKind::Super) => {
                // The base's version, whatever the object overrides.
                return Ok(Expression::CallMethod {
                    target: MethodTarget::Direct(function),
                    receiver: Box::new(self.self_value(callee)?),
                    arguments,
                    null_safe: false,
                });
            }
            ExprKind::Member {
                object, null_safe, ..
            } => (self.expression(object)?, *null_safe),
            _ => {
                return Err(LowerError::internal(
                    "method called through an expression",
                    callee.span,
                ));
            }
        };
        // `private` methods are not overridden: they run their own code.
        let target = if method.is_private {
            MethodTarget::Direct(function)
        } else {
            let place = self
                .tables
                .overridable
                .get(&(class, index))
                .copied()
                .ok_or_else(|| LowerError::internal("method without a place", callee.span))?;
            MethodTarget::Overridable(place)
        };
        Ok(Expression::CallMethod {
            target,
            receiver: Box::new(receiver),
            arguments,
            null_safe,
        })
    }

    /// `Class(arguments)`.
    fn construct(
        &self,
        callee: &Expr,
        class: ClassId,
        arguments: &[Expr],
    ) -> Result<Expression, LowerError> {
        let info = self.analysis.classes().class(class);
        if info.origin == ClassOrigin::Native {
            return Err(LowerError::unsupported(
                "objects of engine classes",
                callee.span,
            ));
        }
        let constructor = self
            .tables
            .constructor(class)
            .ok_or_else(|| LowerError::internal("class without a constructor", callee.span))?;
        let arguments = info
            .constructor_parameters()
            .iter()
            .zip(arguments)
            .map(|(parameter, argument)| self.value_as(argument, &parameter.ty))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Expression::Construct {
            constructor,
            arguments,
        })
    }

    /// A method of a basic type or list, its argument converted to the
    /// type the method takes.
    fn builtin_call(
        &self,
        callee: &Expr,
        method: BuiltinMethod,
        arguments: &[Expr],
    ) -> Result<Expression, LowerError> {
        let ExprKind::Member {
            object, null_safe, ..
        } = &callee.kind
        else {
            return Err(LowerError::internal(
                "builtin method without a receiver",
                callee.span,
            ));
        };
        let receiver_type = self.type_of(object)?.non_null();
        let argument = match (arguments.first(), builtin_parameter(method, &receiver_type)) {
            (Some(argument), Some(parameter)) => {
                Some(Box::new(self.value_as(argument, &parameter)?))
            }
            _ => None,
        };
        Ok(Expression::Builtin {
            method,
            receiver: Box::new(self.expression(object)?),
            argument,
            null_safe: *null_safe,
        })
    }
}

/// `left operator right`, both already of the operation's type.
/// A call of an `async` method, as a coroutine that starts there (or that
/// `await` waits for).
fn started(call: Expression, span: Span) -> Result<Expression, LowerError> {
    let coroutine = match call {
        Expression::Call {
            function,
            arguments,
        } => Coroutine::Call {
            function,
            arguments,
        },
        Expression::CallMethod {
            target,
            receiver,
            arguments,
            null_safe,
        } => Coroutine::Method {
            target,
            receiver,
            arguments,
            null_safe,
        },
        Expression::Start(coroutine) => return Ok(Expression::Start(coroutine)),
        _ => return Err(LowerError::internal("`async` call of no method", span)),
    };
    Ok(Expression::Start(Box::new(coroutine)))
}

fn operation(
    operator: BinaryOperator,
    left: Expression,
    right: Expression,
    expression: &Expr,
) -> Result<Expression, LowerError> {
    let (left, right) = (Box::new(left), Box::new(right));
    Ok(match operator {
        BinaryOperator::Add => arithmetic(Arithmetic::Add, left, right),
        BinaryOperator::Subtract => arithmetic(Arithmetic::Subtract, left, right),
        BinaryOperator::Multiply => arithmetic(Arithmetic::Multiply, left, right),
        BinaryOperator::Divide => arithmetic(Arithmetic::Divide, left, right),
        BinaryOperator::Remainder => arithmetic(Arithmetic::Remainder, left, right),
        BinaryOperator::Less => compare(Comparison::Less, left, right),
        BinaryOperator::LessEqual => compare(Comparison::LessEqual, left, right),
        BinaryOperator::Greater => compare(Comparison::Greater, left, right),
        BinaryOperator::GreaterEqual => compare(Comparison::GreaterEqual, left, right),
        BinaryOperator::Equal => Expression::Equal(left, right),
        BinaryOperator::NotEqual => Expression::Not(Box::new(Expression::Equal(left, right))),
        BinaryOperator::And => Expression::And(left, right),
        BinaryOperator::Or => Expression::Or(left, right),
        BinaryOperator::Coalesce => Expression::Coalesce(left, right),
        BinaryOperator::Range => {
            return Err(LowerError::internal("range outside `for`", expression.span));
        }
    })
}

fn arithmetic(operator: Arithmetic, left: Box<Expression>, right: Box<Expression>) -> Expression {
    Expression::Arithmetic {
        operator,
        left,
        right,
    }
}

fn compare(operator: Comparison, left: Box<Expression>, right: Box<Expression>) -> Expression {
    Expression::Compare {
        operator,
        left,
        right,
    }
}

/// `value`, of type `from`, as a value of type `to`. Analysis already
/// checked that it can be; only conversions between number types change
/// the value, since a nullable value is the value itself or `null`.
fn coerce(
    value: Expression,
    from: &Type,
    to: &Type,
    expression: &Expr,
) -> Result<Expression, LowerError> {
    match (from, to) {
        _ if from == to => Ok(value),
        (Type::Null, Type::Nullable(_)) => Ok(value),
        (Type::Nullable(from), Type::Nullable(to)) => coerce(value, from, to, expression),
        (from, Type::Nullable(to)) => coerce(value, from, to, expression),
        (from, to) if from.widens_to(to) => convert(value, to, expression),
        // An object of a subclass used as one of its base: objects carry
        // their class, so nothing changes.
        (Type::Class(_), Type::Class(_)) => Ok(value),
        _ => Err(LowerError::internal(
            "conversion between unrelated types",
            expression.span,
        )),
    }
}

/// `value` converted to the number type `ty`.
fn convert(value: Expression, ty: &Type, expression: &Expr) -> Result<Expression, LowerError> {
    let to = number_type(ty).ok_or_else(|| {
        LowerError::internal("conversion to a type that is not a number", expression.span)
    })?;
    Ok(Expression::Convert {
        value: Box::new(value),
        to,
    })
}

/// The number type of `ty`, if it is one.
const fn number_type(ty: &Type) -> Option<NumberType> {
    match ty {
        Type::Short => Some(NumberType::Short),
        Type::Int => Some(NumberType::Int),
        Type::BigInt => Some(NumberType::BigInt),
        Type::Float => Some(NumberType::Float),
        Type::Double => Some(NumberType::Double),
        _ => None,
    }
}

/// The type of the argument of a builtin method, if it takes one.
fn builtin_parameter(method: BuiltinMethod, receiver: &Type) -> Option<Type> {
    match (method, receiver) {
        (BuiltinMethod::Min | BuiltinMethod::Max, _) => Some(receiver.clone()),
        (BuiltinMethod::Contains | BuiltinMethod::Push, Type::List(element)) => {
            Some((**element).clone())
        }
        (BuiltinMethod::Contains, Type::String) => Some(Type::String),
        _ => None,
    }
}

/// The value of a literal of type `ty`. Integers written where a decimal is
/// needed become one; integers of a smaller type keep their lowest bits,
/// so `-32768` negates `32768` as a `short` to the same value the compiled
/// game writes whole.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn literal_value(literal: &Literal, ty: &Type, expression: &Expr) -> Result<Value, LowerError> {
    Ok(match (literal, ty.non_null()) {
        (Literal::Integer(value), Type::Short) => Value::Short(*value as i16),
        (Literal::Integer(value), Type::Int) => Value::Int(*value as i32),
        (Literal::Integer(value), Type::BigInt) => Value::BigInt(*value),
        (Literal::Integer(value), Type::Float) => Value::Float(*value as f64 as f32),
        (Literal::Integer(value), Type::Double) => Value::Double(*value as f64),
        (Literal::Float(value), Type::Float) => Value::Float(*value as f32),
        (Literal::Float(value), Type::Double) => Value::Double(*value),
        (Literal::String(text), _) => Value::text(text),
        (Literal::Bool(value), _) => Value::Bool(*value),
        (Literal::Null, _) => Value::Null,
        _ => {
            return Err(LowerError::internal(
                "literal of an unexpected type",
                expression.span,
            ));
        }
    })
}
