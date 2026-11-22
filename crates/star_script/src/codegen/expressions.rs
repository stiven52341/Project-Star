//! Expressions: each becomes a Rust expression that yields an owned value.

use super::conversions::{NONE, RC, RUNTIME, read_variable};
use super::names::ident;
use super::statements::Body;
use super::{Result, internal};
use crate::Span;
use crate::ast::{BinaryOperator, Expr, ExprKind, Literal, Name, UnaryOperator};
use crate::semantic::{ClassId, LocalId, Resolution, Type};

impl Body<'_> {
    /// The Rust code of `expression`, a value of the type analysis recorded.
    pub(super) fn expression(&self, expression: &Expr) -> Result<String> {
        let span = expression.span;
        match &expression.kind {
            ExprKind::Literal(literal) => {
                Ok(literal_code(literal, self.context.type_of(expression)?))
            }
            ExprKind::Name(name) => self.name(expression, name),
            ExprKind::SelfValue => Ok(self_reference()),
            ExprKind::Super => Err(internal("`super` used as a value", span)),
            ExprKind::Unary { operator, operand } => self.unary(*operator, operand),
            ExprKind::Binary {
                operator,
                left,
                right,
            } => self.binary(expression, *operator, left, right),
            ExprKind::TypeTest { value, .. } => self.type_test(expression, value),
            ExprKind::Cast { value, .. } => self.cast(expression, value),
            ExprKind::Call { callee, arguments } => self.call(callee, arguments),
            ExprKind::Member {
                object,
                name,
                null_safe,
            } => self.member(expression, object, name, *null_safe),
            ExprKind::Index { object, index } => Ok(format!(
                "({}).get({})",
                self.expression(object)?,
                self.expression(index)?
            )),
            ExprKind::Array(elements) => self.array(expression, elements),
        }
    }

    /// `expression` converted to `ty`, where analysis allows that.
    pub(super) fn value_as(&self, expression: &Expr, ty: &Type) -> Result<String> {
        let code = self.expression(expression)?;
        let found = self.context.type_of(expression)?;
        self.context.coerce(code, found, ty, expression.span)
    }

    /// The fields that `class` declares, of `self`.
    pub(super) fn own_fields(&self, class: ClassId) -> String {
        format!(
            "{}::HasFields::fields(&**this_)",
            self.context.class_path(class)
        )
    }

    /// The fields that `class` declares, of the object `object`.
    pub(super) fn fields_of(&self, class: ClassId, object: &str) -> String {
        format!(
            "{}::HasFields::fields(&*({object}))",
            self.context.class_path(class)
        )
    }

    /// The Rust name of a field.
    pub(super) fn field_name(&self, class: ClassId, index: usize, span: Span) -> Result<String> {
        self.context
            .analysis
            .classes()
            .class(class)
            .fields
            .get(index)
            .map(|field| ident(&field.name))
            .ok_or_else(|| internal("unknown field", span))
    }

    /// The value of a constant: `crate::classes::Player::Methods::MAX_LIVES()`.
    pub(super) fn constant(&self, class: ClassId, index: usize, span: Span) -> Result<String> {
        let constant = self
            .context
            .analysis
            .classes()
            .class(class)
            .constants
            .get(index)
            .ok_or_else(|| internal("unknown constant", span))?;
        Ok(format!(
            "{}::Methods::{}()",
            self.context.class_path(class),
            ident(&constant.name)
        ))
    }

    // --- Names and members ---

    fn name(&self, expression: &Expr, name: &str) -> Result<String> {
        let span = expression.span;
        match self.context.resolution(expression)? {
            Resolution::Local(id) => self.local(id, name, self.context.type_of(expression)?, span),
            Resolution::Field { class, index } => Ok(format!(
                "{}.{}.get()",
                self.own_fields(class),
                self.field_name(class, index, span)?
            )),
            Resolution::Constant { class, index } => self.constant(class, index, span),
            _ => Err(internal("name that is not a value", span)),
        }
    }

    /// Reads a local, narrowed to what analysis proved about it here.
    fn local(&self, id: LocalId, name: &str, known: &Type, span: Span) -> Result<String> {
        let declared = &self
            .context
            .analysis
            .local(id)
            .ok_or_else(|| internal("unknown local", span))?
            .ty;
        let code = read_variable(&ident(name), declared);
        self.context.narrow(code, declared, known, span)
    }

    fn member(
        &self,
        expression: &Expr,
        object: &Expr,
        name: &Name,
        null_safe: bool,
    ) -> Result<String> {
        let span = name.span;
        let resolution = self.context.resolution(expression)?;
        if let Resolution::Constant { class, index } = resolution {
            let constant = self.constant(class, index, span)?;
            return Ok(match self.context.resolution(object) {
                Ok(Resolution::Class(_)) => constant,
                // Through an object: it is still evaluated, for its effects.
                _ => format!("{{ let _ = {}; {constant} }}", self.expression(object)?),
            });
        }
        let Resolution::Field { class, index } = resolution else {
            return Err(internal("member that is not a value", span));
        };
        let field = self.field_name(class, index, span)?;
        let object_code = self.expression(object)?;
        if !null_safe {
            return Ok(format!(
                "{}.{field}.get()",
                self.fields_of(class, &object_code)
            ));
        }
        let read = format!("{}.{field}.get()", self.fields_of(class, "value_"));
        let field_type = &self.context.analysis.classes().class(class).fields[index].ty;
        let adapter = if field_type.is_nullable() {
            "and_then"
        } else {
            "map"
        };
        Ok(format!("({object_code}).{adapter}(|value_| {read})"))
    }

    // --- Operators ---

    fn unary(&self, operator: UnaryOperator, operand: &Expr) -> Result<String> {
        let code = self.expression(operand)?;
        Ok(match (operator, self.context.type_of(operand)?) {
            (UnaryOperator::Negate, Type::Int) => format!("({code}).wrapping_neg()"),
            (UnaryOperator::Negate, _) => format!("(-({code}))"),
            (UnaryOperator::Not, _) => format!("(!({code}))"),
        })
    }

    fn binary(
        &self,
        expression: &Expr,
        operator: BinaryOperator,
        left: &Expr,
        right: &Expr,
    ) -> Result<String> {
        match operator {
            BinaryOperator::Coalesce => self.coalesce(expression, left, right),
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                let equal = self.equality(left, right)?;
                Ok(if operator == BinaryOperator::Equal {
                    equal
                } else {
                    format!("(!{equal})")
                })
            }
            BinaryOperator::Range => Err(internal("range outside `for`", expression.span)),
            BinaryOperator::And
            | BinaryOperator::Or
            | BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => Ok(format!(
                "(({}) {} ({}))",
                self.expression(left)?,
                operator.symbol(),
                self.expression(right)?
            )),
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder => {
                let ty = self.context.type_of(expression)?;
                let left = self.expression(left)?;
                let right = self.expression(right)?;
                arithmetic(operator, ty, &left, &right, expression.span)
            }
        }
    }

    /// `value ?? fallback`
    fn coalesce(&self, expression: &Expr, left: &Expr, right: &Expr) -> Result<String> {
        let result = self.context.type_of(expression)?;
        let value = self.expression(left)?;
        let fallback = self.value_as(right, result)?;
        let adapter = if result.is_nullable() {
            "or_else"
        } else {
            "unwrap_or_else"
        };
        Ok(format!("({value}).{adapter}(|| {fallback})"))
    }

    /// `left == right`: both sides as the same type, then compared as
    /// StarScript does (values by content, objects by identity).
    fn equality(&self, left: &Expr, right: &Expr) -> Result<String> {
        let null_check = |other: &Expr| -> Result<String> {
            Ok(format!("({}).is_none()", self.expression(other)?))
        };
        match (is_null(left), is_null(right)) {
            (true, true) => return Ok("true".to_owned()),
            (false, true) => return null_check(left),
            (true, false) => return null_check(right),
            (false, false) => {}
        }
        let left_type = self.context.type_of(left)?;
        let right_type = self.context.type_of(right)?;
        let common = if self
            .context
            .analysis
            .classes()
            .is_assignable(left_type, right_type)
        {
            right_type
        } else {
            left_type
        };
        Ok(format!(
            "{RUNTIME}::StarEq::star_eq(&({}), &({}))",
            self.value_as(left, common)?,
            self.value_as(right, common)?
        ))
    }

    /// `value is Class`
    fn type_test(&self, expression: &Expr, value: &Expr) -> Result<String> {
        let ExprKind::TypeTest { type_ref, .. } = &expression.kind else {
            return Err(internal("type test expected", expression.span));
        };
        let tested = self
            .context
            .analysis
            .classes()
            .find(&type_ref.to_string())
            .ok_or_else(|| internal("unknown tested class", type_ref.span))?;
        let code = self.expression(value)?;
        let cast = self.context.cast_function(tested);
        Ok(if self.context.type_of(value)?.is_nullable() {
            format!("({code}).is_some_and(|value_| {cast}(value_).is_some())")
        } else {
            format!("{cast}({code}).is_some()")
        })
    }

    /// `value as Type`
    fn cast(&self, expression: &Expr, value: &Expr) -> Result<String> {
        let span = expression.span;
        let target = self.context.type_of(expression)?;
        let from = self.context.type_of(value)?;
        let code = self.expression(value)?;
        match (from, target) {
            _ if from == target => Ok(code),
            (Type::Int, Type::Float) => Ok(format!("(({code}) as f32)")),
            // Rust's `as` saturates: too large values give the largest `int`.
            (Type::Float, Type::Int) => Ok(format!("(({code}) as i64)")),
            _ => {
                let (Some(from_class), Some(to_class)) = (from.class(), target.class()) else {
                    return Err(internal("cast between unrelated types", span));
                };
                if self
                    .context
                    .analysis
                    .classes()
                    .is_subclass(from_class, to_class)
                {
                    self.context.coerce(code, from, target, span)
                } else {
                    Ok(self.context.downcast(&code, from, to_class))
                }
            }
        }
    }

    /// `[a, b, c]`
    fn array(&self, expression: &Expr, elements: &[Expr]) -> Result<String> {
        let Type::List(element) = self.context.type_of(expression)? else {
            return Err(internal("list without a list type", expression.span));
        };
        let items = elements
            .iter()
            .map(|item| self.value_as(item, element))
            .collect::<Result<Vec<_>>>()?;
        Ok(format!(
            "{RUNTIME}::List::from_vec(::std::vec![{}])",
            items.join(", ")
        ))
    }
}

/// `self`, as a new reference.
pub(super) fn self_reference() -> String {
    format!("{RC}::clone(this_)")
}

/// `left operator right` for numbers, or `+` for texts. Integers wrap around
/// on overflow, so debug and release builds give the same results.
pub(super) fn arithmetic(
    operator: BinaryOperator,
    ty: &Type,
    left: &str,
    right: &str,
    span: Span,
) -> Result<String> {
    Ok(match (ty, operator) {
        (Type::Int, BinaryOperator::Add) => format!("({left}).wrapping_add({right})"),
        (Type::Int, BinaryOperator::Subtract) => format!("({left}).wrapping_sub({right})"),
        (Type::Int, BinaryOperator::Multiply) => format!("({left}).wrapping_mul({right})"),
        (Type::Int, BinaryOperator::Divide) => format!("{RUNTIME}::int_divide({left}, {right})"),
        (Type::Int, BinaryOperator::Remainder) => {
            format!("{RUNTIME}::int_remainder({left}, {right})")
        }
        (Type::String, BinaryOperator::Add) => format!("{RUNTIME}::concat({left}, &({right}))"),
        (Type::Float, _) => format!("(({left}) {} ({right}))", operator.symbol()),
        _ => return Err(internal("arithmetic on a non-numeric type", span)),
    })
}

fn literal_code(literal: &Literal, ty: &Type) -> String {
    match literal {
        // An integer literal where a `float` is needed: `var speed: float = 2`.
        #[allow(clippy::cast_precision_loss)] // Literals that large are not exact floats anyway.
        Literal::Integer(value) if *ty == Type::Float => float_literal(*value as f64),
        Literal::Integer(value) => format!("{value}_i64"),
        Literal::Float(value) => float_literal(*value),
        Literal::String(text) => format!("::std::string::String::from({text:?})"),
        Literal::Bool(value) => value.to_string(),
        Literal::Null => NONE.to_owned(),
    }
}

/// A `float` literal. Values too large for a `float` become infinity,
/// as when converting them at run time.
#[allow(clippy::cast_possible_truncation)] // `float` is `f32` by design.
fn float_literal(value: f64) -> String {
    let value = value as f32;
    if value.is_finite() {
        format!("{value:?}_f32")
    } else if value > 0.0 {
        "f32::INFINITY".to_owned()
    } else {
        "f32::NEG_INFINITY".to_owned()
    }
}

const fn is_null(expression: &Expr) -> bool {
    matches!(expression.kind, ExprKind::Literal(Literal::Null))
}
