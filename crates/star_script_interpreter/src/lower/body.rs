//! The body of a function: its locals and statements.

use std::collections::HashMap;

use star_script::ast::{
    AssignOperator, BinaryOperator, Block, ElseBranch, Expr, ExprKind, IfStatement, MethodDecl,
    Name, Parameter, Statement as SyntaxStatement, StatementKind, VariableDecl,
};
use star_script::semantic::{Analysis, LocalId, Resolution, Type};

use super::FunctionTable;
use crate::error::LowerError;
use crate::tree::{Arithmetic, Expression, Function, Statement};
use crate::value::Value;

/// Lowers the body of one function.
pub(super) struct Body<'a> {
    pub(super) analysis: &'a Analysis,
    pub(super) functions: &'a FunctionTable,
    /// The slot of each local declared so far.
    slots: HashMap<LocalId, usize>,
    /// What `return` gives back.
    return_type: Type,
}

impl<'a> Body<'a> {
    pub(super) fn new(
        analysis: &'a Analysis,
        functions: &'a FunctionTable,
        return_type: Type,
    ) -> Self {
        Self {
            analysis,
            functions,
            slots: HashMap::new(),
            return_type,
        }
    }

    /// The function with these parameters and body.
    pub(super) fn function(mut self, declaration: &MethodDecl) -> Result<Function, LowerError> {
        for parameter in &declaration.parameters {
            self.parameter(parameter)?;
        }
        let body = self.block(&declaration.body)?;
        Ok(Function {
            slots: self.slots.len(),
            body,
        })
    }

    fn parameter(&mut self, parameter: &Parameter) -> Result<usize, LowerError> {
        self.declare(&parameter.name)
    }

    /// A new slot for the local declared with `name`.
    fn declare(&mut self, name: &Name) -> Result<usize, LowerError> {
        let local = self
            .analysis
            .declared_local(name)
            .ok_or_else(|| LowerError::internal("local without analysis", name.span))?;
        let slot = self.slots.len();
        self.slots.insert(local, slot);
        Ok(slot)
    }

    /// The slot of a local declared before.
    pub(super) fn slot(&self, local: LocalId, expression: &Expr) -> Result<usize, LowerError> {
        self.slots.get(&local).copied().ok_or_else(|| {
            LowerError::internal("local used before its declaration", expression.span)
        })
    }

    /// The declared type of a local.
    pub(super) fn local_type(
        &self,
        local: LocalId,
        expression: &Expr,
    ) -> Result<&Type, LowerError> {
        self.analysis
            .local(local)
            .map(|info| &info.ty)
            .ok_or_else(|| LowerError::internal("unknown local", expression.span))
    }

    fn block(&mut self, block: &Block) -> Result<Vec<Statement>, LowerError> {
        block
            .statements
            .iter()
            .map(|statement| self.statement(statement))
            .collect()
    }

    fn statement(&mut self, statement: &SyntaxStatement) -> Result<Statement, LowerError> {
        Ok(match &statement.kind {
            StatementKind::Variable(variable) => self.variable(variable)?,
            StatementKind::Assignment {
                target,
                operator,
                value,
            } => self.assignment(target, *operator, value)?,
            StatementKind::Expression(expression) => {
                Statement::Evaluate(self.expression(expression)?)
            }
            StatementKind::If(if_statement) => self.if_statement(if_statement)?,
            StatementKind::While { condition, body } => Statement::While {
                condition: self.expression(condition)?,
                body: self.block(body)?,
            },
            StatementKind::For {
                variable,
                iterable,
                body,
            } => self.for_statement(variable, iterable, body)?,
            StatementKind::Return(value) => Statement::Return(
                value
                    .as_ref()
                    .map(|value| self.value_as(value, &self.return_type))
                    .transpose()?,
            ),
            StatementKind::Break => Statement::Break,
            StatementKind::Continue => Statement::Continue,
        })
    }

    fn variable(&mut self, variable: &VariableDecl) -> Result<Statement, LowerError> {
        let span = variable.name.span;
        let local = self
            .analysis
            .declared_local(&variable.name)
            .and_then(|id| self.analysis.local(id))
            .ok_or_else(|| LowerError::internal("variable without analysis", span))?;
        // Its value is lowered before the local exists: it cannot read it.
        let value = match &variable.initializer {
            Some(value) => self.value_as(value, &local.ty)?,
            None => default_value(&local.ty)
                .ok_or_else(|| LowerError::internal("type without a default value", span))?,
        };
        let slot = self.declare(&variable.name)?;
        Ok(Statement::Store { slot, value })
    }

    fn assignment(
        &self,
        target: &Expr,
        operator: AssignOperator,
        value: &Expr,
    ) -> Result<Statement, LowerError> {
        if let ExprKind::Index { object, index } = &target.kind {
            return Ok(Statement::SetElement {
                list: self.expression(object)?,
                index: self.expression(index)?,
                operator: compound_operator(operator),
                value: self.value_as(value, self.type_of(target)?)?,
            });
        }
        let Some(Resolution::Local(local)) = self.resolution(target)? else {
            return Err(LowerError::unsupported("fields", target.span));
        };
        let slot = self.slot(local, target)?;
        let ty = self.local_type(local, target)?.clone();
        let value = self.value_as(value, &ty)?;
        let value = match compound_operator(operator) {
            None => value,
            Some(operator) => Expression::Arithmetic {
                operator,
                left: Box::new(Expression::Local(slot)),
                right: Box::new(value),
            },
        };
        Ok(Statement::Store { slot, value })
    }

    fn if_statement(&mut self, if_statement: &IfStatement) -> Result<Statement, LowerError> {
        let else_branch = match &if_statement.else_branch {
            None => Vec::new(),
            Some(ElseBranch::Block(block)) => self.block(block)?,
            Some(ElseBranch::If(nested)) => vec![self.if_statement(nested)?],
        };
        Ok(Statement::If {
            condition: self.expression(&if_statement.condition)?,
            then_branch: self.block(&if_statement.then_block)?,
            else_branch,
        })
    }

    fn for_statement(
        &mut self,
        variable: &Name,
        iterable: &Expr,
        body: &Block,
    ) -> Result<Statement, LowerError> {
        let ExprKind::Binary {
            operator: BinaryOperator::Range,
            left,
            right,
        } = &iterable.kind
        else {
            let list = self.expression(iterable)?;
            let slot = self.declare(variable)?;
            return Ok(Statement::ForEach {
                slot,
                list,
                body: self.block(body)?,
            });
        };
        let (start, end) = self.common_operands(left, right)?;
        let slot = self.declare(variable)?;
        Ok(Statement::ForRange {
            slot,
            start,
            end,
            body: self.block(body)?,
        })
    }
}

/// The value a variable of `ty` starts with when none is given: an
/// expression, so each run of the declaration gets a list of its own.
fn default_value(ty: &Type) -> Option<Expression> {
    let value = match ty {
        Type::Short => Value::Short(0),
        Type::Int => Value::Int(0),
        Type::BigInt => Value::BigInt(0),
        Type::Float => Value::Float(0.0),
        Type::Double => Value::Double(0.0),
        Type::Bool => Value::Bool(false),
        Type::String => Value::text(""),
        Type::Nullable(_) => Value::Null,
        Type::List(_) => return Some(Expression::List(Vec::new())),
        _ => return None,
    };
    Some(Expression::Constant(value))
}

/// The operator of `a op= b`, or `None` for plain `=`.
const fn compound_operator(operator: AssignOperator) -> Option<Arithmetic> {
    match operator {
        AssignOperator::Assign => None,
        AssignOperator::Add => Some(Arithmetic::Add),
        AssignOperator::Subtract => Some(Arithmetic::Subtract),
        AssignOperator::Multiply => Some(Arithmetic::Multiply),
        AssignOperator::Divide => Some(Arithmetic::Divide),
        AssignOperator::Remainder => Some(Arithmetic::Remainder),
    }
}
