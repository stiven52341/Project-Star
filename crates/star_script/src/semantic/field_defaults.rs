//! The values fields start with when they are written as plain values:
//! what the editor's inspector shows for an `export` field until a scene
//! gives it another.

use crate::ast::{Expr, ExprKind, Literal, UnaryOperator};

use super::natives::COLOR_CLASS;

/// A field's first value, written as a plain value in its class.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldDefault {
    /// `true` or `false`.
    Bool(bool),
    /// A whole number.
    Int(i64),
    /// A number with decimals.
    Float(f64),
    /// A text.
    Text(String),
    /// `Color(r, g, b, a)` with numbers.
    Color([f64; 4]),
}

/// The value of `initializer` if it is a plain value: a literal, a negated
/// number, or a `Color` made of numbers. `None` for anything computed.
pub(super) fn plain_value(initializer: &Expr) -> Option<FieldDefault> {
    match &initializer.kind {
        ExprKind::Literal(Literal::Bool(truth)) => Some(FieldDefault::Bool(*truth)),
        ExprKind::Literal(Literal::Integer(number)) => Some(FieldDefault::Int(*number)),
        ExprKind::Literal(Literal::Float(number)) => Some(FieldDefault::Float(*number)),
        ExprKind::Literal(Literal::String(text)) => Some(FieldDefault::Text(text.clone())),
        ExprKind::Unary {
            operator: UnaryOperator::Negate,
            operand,
        } => match plain_value(operand)? {
            FieldDefault::Int(number) => number.checked_neg().map(FieldDefault::Int),
            FieldDefault::Float(number) => Some(FieldDefault::Float(-number)),
            _ => None,
        },
        ExprKind::Call { callee, arguments } => color(callee, arguments),
        _ => None,
    }
}

/// `Color(r, g, b, a)`, if every part is a number.
fn color(callee: &Expr, arguments: &[Expr]) -> Option<FieldDefault> {
    if !matches!(&callee.kind, ExprKind::Name(name) if name == COLOR_CLASS) {
        return None;
    }
    let parts: Vec<f64> = arguments
        .iter()
        .map(|argument| match plain_value(argument)? {
            FieldDefault::Float(number) => Some(number),
            #[allow(clippy::cast_precision_loss)] // The parts of a color are small.
            FieldDefault::Int(number) => Some(number as f64),
            _ => None,
        })
        .collect::<Option<_>>()?;
    Some(FieldDefault::Color(parts.try_into().ok()?))
}
