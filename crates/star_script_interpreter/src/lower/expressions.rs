//! Expressions, with the conversions between number types written out.

use star_script::ast::{BinaryOperator, Expr, ExprKind, Literal, UnaryOperator};
use star_script::semantic::{BuiltinMethod, ClassId, ClassOrigin, Resolution, Type};

use super::body::Body;
use crate::error::LowerError;
use crate::tree::{Arithmetic, Comparison, Expression, NumberType};
use crate::value::Value;

/// The engine class whose `print` writes a line.
const DEBUG_CLASS: &str = "Debug";

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
            ExprKind::SelfValue | ExprKind::Super => Err(LowerError::unsupported("objects", span)),
            ExprKind::TypeTest { .. } => Err(LowerError::unsupported("`is`", span)),
            ExprKind::Member { .. } => Err(LowerError::unsupported("fields and constants", span)),
            ExprKind::Index { .. } | ExprKind::Array(_) => {
                Err(LowerError::unsupported("lists", span))
            }
            ExprKind::Await(_) => Err(LowerError::unsupported("`await`", span)),
        }
    }

    /// `expression` converted to `ty`, where analysis allows that.
    pub(super) fn value_as(&self, expression: &Expr, ty: &Type) -> Result<Expression, LowerError> {
        let value = self.expression(expression)?;
        let found = self.type_of(expression)?;
        if found == ty {
            return Ok(value);
        }
        if found.widens_to(ty) {
            return convert(value, ty, expression);
        }
        Err(LowerError::unsupported(
            "`null` and nullable types",
            expression.span,
        ))
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

    fn type_of(&self, expression: &Expr) -> Result<&Type, LowerError> {
        self.analysis
            .type_of(expression)
            .ok_or_else(|| LowerError::internal("expression without a type", expression.span))
    }

    fn name(&self, expression: &Expr) -> Result<Expression, LowerError> {
        match self.resolution(expression)? {
            Some(Resolution::Local(local)) => {
                if self.local_type(local, expression)? != self.type_of(expression)? {
                    return Err(LowerError::unsupported(
                        "narrowed types (`null` checks and `is`)",
                        expression.span,
                    ));
                }
                Ok(Expression::Local(self.slot(local, expression)?))
            }
            Some(Resolution::Field { .. }) => {
                Err(LowerError::unsupported("fields", expression.span))
            }
            Some(Resolution::Constant { .. }) => {
                Err(LowerError::unsupported("constants", expression.span))
            }
            _ => Err(LowerError::internal(
                "name that is not a value",
                expression.span,
            )),
        }
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

    /// `value as Type` between numbers.
    fn cast(&self, expression: &Expr, value: &Expr) -> Result<Expression, LowerError> {
        let target = self.type_of(expression)?;
        let from = self.type_of(value)?;
        let lowered = self.expression(value)?;
        if from == target {
            return Ok(lowered);
        }
        if !from.is_numeric() || !target.is_numeric() {
            return Err(LowerError::unsupported(
                "`as` between classes",
                expression.span,
            ));
        }
        convert(lowered, target, expression)
    }

    fn call(&self, callee: &Expr, arguments: &[Expr]) -> Result<Expression, LowerError> {
        match self.resolution(callee)? {
            Some(Resolution::Method { class, index }) => {
                self.method_call(callee, class, index, arguments)
            }
            Some(Resolution::Builtin(method)) => self.builtin_call(callee, method),
            Some(Resolution::Constructor(_)) => {
                Err(LowerError::unsupported("objects", callee.span))
            }
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
            return match (
                info.name.as_str(),
                method.name.as_str(),
                arguments.as_slice(),
            ) {
                (DEBUG_CLASS, "print", [_]) => Ok(Expression::Print(Box::new(
                    arguments.into_iter().next().ok_or_else(|| {
                        LowerError::internal("`print` without a message", callee.span)
                    })?,
                ))),
                _ => Err(LowerError::unsupported(
                    "this method of the engine",
                    callee.span,
                )),
            };
        }
        let function = self
            .functions
            .get(&(class, index))
            .copied()
            .ok_or_else(|| LowerError::internal("method without a function", callee.span))?;
        Ok(Expression::Call {
            function,
            arguments,
        })
    }

    fn builtin_call(&self, callee: &Expr, method: BuiltinMethod) -> Result<Expression, LowerError> {
        let ExprKind::Member {
            object, null_safe, ..
        } = &callee.kind
        else {
            return Err(LowerError::internal(
                "builtin method without a receiver",
                callee.span,
            ));
        };
        if *null_safe {
            return Err(LowerError::unsupported("`?.`", callee.span));
        }
        match method {
            BuiltinMethod::ToString => Ok(Expression::ToText(Box::new(self.expression(object)?))),
            _ => Err(LowerError::unsupported(
                "this method of basic types",
                callee.span,
            )),
        }
    }
}

/// `left operator right`, both already of the operation's type.
pub(super) fn operation(
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
        BinaryOperator::Coalesce => {
            return Err(LowerError::unsupported("`??`", expression.span));
        }
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

/// The value of a literal of type `ty`. Integers written where a decimal is
/// needed become one; integers of a smaller type keep their lowest bits,
/// so `-32768` negates `32768` as a `short` to the same value the compiled
/// game writes whole.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn literal_value(literal: &Literal, ty: &Type, expression: &Expr) -> Result<Value, LowerError> {
    Ok(match (literal, ty) {
        (Literal::Integer(value), Type::Short) => Value::Short(*value as i16),
        (Literal::Integer(value), Type::Int) => Value::Int(*value as i32),
        (Literal::Integer(value), Type::BigInt) => Value::BigInt(*value),
        (Literal::Integer(value), Type::Float) => Value::Float(*value as f64 as f32),
        (Literal::Integer(value), Type::Double) => Value::Double(*value as f64),
        (Literal::Float(value), Type::Float) => Value::Float(*value as f32),
        (Literal::Float(value), Type::Double) => Value::Double(*value),
        (Literal::String(text), _) => Value::text(text),
        (Literal::Bool(value), _) => Value::Bool(*value),
        (Literal::Null, _) => {
            return Err(LowerError::unsupported("`null`", expression.span));
        }
        _ => {
            return Err(LowerError::internal(
                "literal of an unexpected type",
                expression.span,
            ));
        }
    })
}
