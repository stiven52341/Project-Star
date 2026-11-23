//! Statements inside methods and constructors.

use super::checker::{Checker, ContextKind, NameTarget};
use super::{FoundMember, LocalId, MemberRef, Resolution, SemanticErrorKind, Type, flow};
use crate::Span;
use crate::ast::{
    AssignOperator, BinaryOperator, Block, ElseBranch, Expr, ExprKind, IfStatement, Name,
    Statement, StatementKind, VariableDecl, VariableKind,
};

impl Checker<'_> {
    /// Checks a block in a new scope that starts with `narrowed` knowledge.
    pub(super) fn block(&mut self, block: &Block, narrowed: Vec<(LocalId, Type)>) {
        self.push_scope(narrowed);
        for statement in &block.statements {
            self.statement(statement);
        }
        self.pop_scope();
    }

    fn statement(&mut self, statement: &Statement) {
        match &statement.kind {
            StatementKind::Variable(variable) => self.local_variable(variable),
            StatementKind::Assignment {
                target,
                operator,
                value,
            } => self.assignment(target, *operator, value, statement.span),
            StatementKind::Expression(expression) => {
                self.expression(expression, None);
                // `await f()` is a call too; `await` reports what is not.
                if !matches!(expression.kind, ExprKind::Call { .. } | ExprKind::Await(_)) {
                    self.error(SemanticErrorKind::UnusedValue, expression.span);
                }
            }
            StatementKind::If(if_statement) => self.if_statement(if_statement),
            StatementKind::While { condition, body } => {
                self.condition(condition);
                let narrowed = self.facts(condition, true);
                self.loop_body(body, narrowed);
            }
            StatementKind::For {
                variable,
                iterable,
                body,
            } => self.for_statement(variable, iterable, body),
            StatementKind::Return(value) => self.return_statement(value.as_ref(), statement.span),
            StatementKind::Break if self.context.loop_depth == 0 => {
                self.error(SemanticErrorKind::BreakOutsideLoop, statement.span);
            }
            StatementKind::Continue if self.context.loop_depth == 0 => {
                self.error(SemanticErrorKind::ContinueOutsideLoop, statement.span);
            }
            StatementKind::Break | StatementKind::Continue => {}
        }
    }

    fn local_variable(&mut self, variable: &VariableDecl) {
        let declared = variable
            .type_annotation
            .as_ref()
            .map(|type_ref| self.resolve_type(type_ref));
        let name = &variable.name;
        let ty = match (declared, &variable.initializer) {
            (declared, Some(value)) => self.initial_value(declared, value),
            (Some(declared), None) => {
                if !declared.has_default_value() {
                    self.error(
                        SemanticErrorKind::MissingValue(name.text.clone()),
                        name.span,
                    );
                }
                declared
            }
            (None, None) => {
                let kind = SemanticErrorKind::MissingTypeOrValue(name.text.clone());
                self.error(kind, name.span);
                Type::Error
            }
        };
        self.declare_local(name, ty, variable.kind == VariableKind::Var);
    }

    /// An `if`, narrowing types in each branch, and after it when one branch
    /// always leaves: after `if x == null { return }`, `x` is not `null`.
    fn if_statement(&mut self, if_statement: &IfStatement) {
        self.condition(&if_statement.condition);
        let when_true = self.facts(&if_statement.condition, true);
        let when_false = self.facts(&if_statement.condition, false);
        self.block(&if_statement.then_block, when_true.clone());
        match &if_statement.else_branch {
            Some(ElseBranch::Block(block)) => self.block(block, when_false.clone()),
            Some(ElseBranch::If(nested)) => {
                self.with_narrowing(when_false.clone(), |checker| checker.if_statement(nested));
            }
            None => {}
        }
        let then_exits = flow::always_exits(&if_statement.then_block);
        let else_exits = if_statement
            .else_branch
            .as_ref()
            .is_some_and(flow::branch_exits);
        if then_exits && !else_exits {
            self.narrow(when_false);
        } else if else_exits && !then_exits {
            self.narrow(when_true);
        }
    }

    fn condition(&mut self, condition: &Expr) {
        let found = self.expression(condition, Some(&Type::Bool));
        self.require_assignable(&found, &Type::Bool, condition.span);
    }

    fn loop_body(&mut self, body: &Block, narrowed: Vec<(LocalId, Type)>) {
        self.context.loop_depth += 1;
        self.block(body, narrowed);
        self.context.loop_depth -= 1;
    }

    fn for_statement(&mut self, variable: &Name, iterable: &Expr, body: &Block) {
        let element = self.iterable_element(iterable);
        self.push_scope(Vec::new());
        self.declare_local(variable, element, false);
        self.loop_body(body, Vec::new());
        self.pop_scope();
    }

    /// The type of each value a `for` goes through.
    fn iterable_element(&mut self, iterable: &Expr) -> Type {
        if let ExprKind::Binary {
            operator: BinaryOperator::Range,
            left,
            right,
        } = &iterable.kind
        {
            for bound in [left, right] {
                let found = self.expression(bound, Some(&Type::Int));
                self.require_assignable(&found, &Type::Int, bound.span);
            }
            self.analysis.record(iterable, Type::Range, None);
            return Type::Int;
        }
        match self.expression(iterable, None) {
            Type::List(element) => *element,
            Type::Error => Type::Error,
            other => {
                let kind = if other.is_nullable() {
                    SemanticErrorKind::PossiblyNull(self.type_name(&other))
                } else {
                    SemanticErrorKind::InvalidOperand {
                        operator: "for",
                        operand: self.type_name(&other),
                    }
                };
                self.error(kind, iterable.span);
                Type::Error
            }
        }
    }

    fn return_statement(&mut self, value: Option<&Expr>, span: Span) {
        if self.context.kind == ContextKind::Constructor && self.missing_initialization().is_some()
        {
            self.error(SemanticErrorKind::ReturnBeforeInitialization, span);
        }
        let expected = self.context.return_type.clone();
        match value {
            Some(value) if expected == Type::Void => {
                self.expression(value, None);
                self.error(SemanticErrorKind::UnexpectedReturnValue, value.span);
            }
            Some(value) => {
                let found = self.expression(value, Some(&expected));
                self.require_assignable(&found, &expected, value.span);
            }
            None if expected != Type::Void => {
                let kind = SemanticErrorKind::MissingReturnValue(self.type_name(&expected));
                self.error(kind, span);
            }
            None => {}
        }
    }

    // --- Assignment ---

    fn assignment(&mut self, target: &Expr, operator: AssignOperator, value: &Expr, span: Span) {
        let Some(target_type) = self.assignment_target(target) else {
            self.expression(value, None);
            return;
        };
        let found = self.expression(value, Some(&target_type));
        if operator == AssignOperator::Assign {
            self.require_assignable(&found, &target_type, value.span);
            return;
        }
        let valid = target_type.is_error()
            || found.is_error()
            || (found == target_type
                && (target_type.is_numeric()
                    || (operator == AssignOperator::Add && target_type == Type::String)));
        if !valid {
            let kind = SemanticErrorKind::InvalidOperands {
                operator: operator.symbol(),
                left: self.type_name(&target_type),
                right: self.type_name(&found),
            };
            self.error(kind, span);
        }
    }

    /// The declared type of what `target` names, if it can be assigned.
    fn assignment_target(&mut self, target: &Expr) -> Option<Type> {
        match &target.kind {
            ExprKind::Name(name) => match self.resolve_name(name) {
                NameTarget::Local(id, _) => {
                    let declared = self.declared_type(id);
                    self.analysis
                        .record(target, declared.clone(), Some(Resolution::Local(id)));
                    if !self.is_mutable(id) {
                        self.error(
                            SemanticErrorKind::AssignToImmutable(name.clone()),
                            target.span,
                        );
                    }
                    Some(declared)
                }
                NameTarget::Member(found) => self.assign_member_of_self(target, name, found),
                NameTarget::Class(_) => {
                    self.error(SemanticErrorKind::NotAValue(name.clone()), target.span);
                    None
                }
                NameTarget::Unknown => {
                    self.error(SemanticErrorKind::UnknownName(name.clone()), target.span);
                    None
                }
            },
            ExprKind::Member {
                object,
                name,
                null_safe,
            } => {
                if *null_safe {
                    self.expression(target, None);
                    self.error(SemanticErrorKind::AssignThroughSafeAccess, target.span);
                    return None;
                }
                if matches!(object.kind, ExprKind::SelfValue) {
                    return self.assign_through_self(target, object, &name.text);
                }
                let ty = self.expression(target, None);
                let resolution = self
                    .analysis
                    .expression(target)
                    .and_then(|info| info.resolution);
                match resolution {
                    Some(Resolution::Field { .. }) => Some(ty),
                    Some(Resolution::Constant { .. }) => {
                        let kind = SemanticErrorKind::AssignToImmutable(name.text.clone());
                        self.error(kind, target.span);
                        None
                    }
                    _ => None,
                }
            }
            _ => Some(self.expression(target, None)),
        }
    }

    /// `self.name = value`, which may be how `init` completes the object.
    fn assign_through_self(&mut self, target: &Expr, object: &Expr, name: &str) -> Option<Type> {
        let class = self.context.class;
        self.analysis.record(object, Type::Class(class), None);
        if let Some(found) = self.analysis.classes.find_member(class, name) {
            return self.assign_member_of_self(target, name, found);
        }
        let kind = SemanticErrorKind::UnknownMember {
            owner: self.analysis.classes.class(class).name.clone(),
            name: name.to_owned(),
        };
        self.error(kind, target.span);
        None
    }

    fn assign_member_of_self(
        &mut self,
        target: &Expr,
        name: &str,
        found: FoundMember,
    ) -> Option<Type> {
        self.check_private(found, name, target.span);
        match found.member {
            MemberRef::Field(index) => {
                let ty = self
                    .analysis
                    .classes
                    .class(found.class)
                    .fields
                    .get(index)
                    .map_or(Type::Error, |field| field.ty.clone());
                let resolution = Resolution::Field {
                    class: found.class,
                    index,
                };
                self.analysis.record(target, ty.clone(), Some(resolution));
                self.allow_field_assignment(found, index, target.span);
                Some(ty)
            }
            MemberRef::Constant(_) => {
                let kind = SemanticErrorKind::AssignToImmutable(name.to_owned());
                self.error(kind, target.span);
                None
            }
            MemberRef::Method(_) => {
                self.error(SemanticErrorKind::NotAValue(name.to_owned()), target.span);
                None
            }
        }
    }

    /// Fields of `self` can be assigned once the object is complete. Before
    /// that, `init` may only give pending fields their value, at its top level.
    fn allow_field_assignment(&mut self, found: FoundMember, index: usize, span: Span) {
        if self.context.kind != ContextKind::Constructor || self.missing_initialization().is_none()
        {
            self.require_self(span);
            return;
        }
        let at_top_level = self.at_top_level();
        let own_class = found.class == self.context.class;
        if let Some(initialization) = self.context.initialization.as_mut() {
            let pending = initialization
                .pending_fields
                .iter()
                .position(|i| *i == index);
            if let (true, true, Some(position)) = (at_top_level, own_class, pending) {
                initialization.pending_fields.remove(position);
                return;
            }
        }
        self.require_self(span);
    }
}
