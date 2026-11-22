//! What a condition proves about local variables, such as `x != null`.
//!
//! Only locals that cannot change are narrowed (`const`, parameters and loop
//! variables): a `var` or a field could become `null` again between the check
//! and the use, so narrowing them would not be safe.

use super::checker::Checker;
use super::{LocalId, Resolution, Type};
use crate::ast::{BinaryOperator, Expr, ExprKind, Literal, UnaryOperator};

impl Checker<'_> {
    /// Narrower types that hold while `condition` evaluates to `when`.
    ///
    /// Must be called after `condition` was checked, since it reads what the
    /// names in it resolved to.
    pub(super) fn facts(&self, condition: &Expr, when: bool) -> Vec<(LocalId, Type)> {
        match &condition.kind {
            ExprKind::Unary {
                operator: UnaryOperator::Not,
                operand,
            } => self.facts(operand, !when),
            ExprKind::Binary {
                operator: BinaryOperator::And,
                left,
                right,
            } if when => self.both_facts(left, right, true),
            ExprKind::Binary {
                operator: BinaryOperator::Or,
                left,
                right,
            } if !when => self.both_facts(left, right, false),
            ExprKind::Binary {
                operator: operator @ (BinaryOperator::Equal | BinaryOperator::NotEqual),
                left,
                right,
            } => {
                let proves_not_null = (*operator == BinaryOperator::NotEqual) == when;
                let compared = match (is_null(left), is_null(right)) {
                    (false, true) => left,
                    (true, false) => right,
                    _ => return Vec::new(),
                };
                if !proves_not_null {
                    return Vec::new();
                }
                self.immutable_local(compared)
                    .map(|(id, ty)| (id, ty.non_null()))
                    .into_iter()
                    .collect()
            }
            ExprKind::TypeTest { value, type_ref } if when => {
                let Ok(tested @ Type::Class(_)) = self.find_type(type_ref) else {
                    return Vec::new();
                };
                self.immutable_local(value)
                    .map(|(id, _)| (id, tested))
                    .into_iter()
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn both_facts(&self, left: &Expr, right: &Expr, when: bool) -> Vec<(LocalId, Type)> {
        let mut facts = self.facts(left, when);
        facts.extend(self.facts(right, when));
        facts
    }

    /// The local `expression` names and its current type, if it cannot change.
    fn immutable_local(&self, expression: &Expr) -> Option<(LocalId, Type)> {
        if !matches!(expression.kind, ExprKind::Name(_)) {
            return None;
        }
        let info = self.analysis.expression(expression)?;
        let Some(Resolution::Local(id)) = info.resolution else {
            return None;
        };
        (!self.is_mutable(id)).then(|| (id, info.ty.clone()))
    }
}

pub(super) const fn is_null(expression: &Expr) -> bool {
    matches!(expression.kind, ExprKind::Literal(Literal::Null))
}
