//! Statements, inside the body of a method, constructor or value.

use super::conversions::{RUNTIME, SOME, default_value, read_variable};
use super::expressions::arithmetic;
use super::names::ident;
use super::writer::Writer;
use super::{Context, Result, internal};
use crate::Span;
use crate::ast::{
    AssignOperator, BinaryOperator, Block, ElseBranch, Expr, ExprKind, IfStatement, Statement,
    StatementKind, VariableDecl, VariableKind,
};
use crate::semantic::{ClassId, Resolution, Type};

/// Where code is being generated, which decides what `self` and `return` are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BodyKind {
    /// An instance method: `this_` is the object.
    Method,
    /// A static method, a field's initial value or a constant: no object.
    Static,
    /// A `parallel` function: static, on a thread of its own, where other
    /// `parallel` functions are called directly.
    Parallel,
    /// `init` before the object exists: assigning a field of the class only
    /// keeps the value, to build the object with it.
    ConstructorSetup,
    /// `init` once the object exists: `return` gives the object back.
    Constructor,
}

/// Generates the code of one body.
pub(super) struct Body<'a> {
    pub(super) context: Context<'a>,
    /// The class the code belongs to.
    pub(super) class: ClassId,
    pub(super) kind: BodyKind,
    /// What `return` must give back.
    pub(super) return_type: Type,
}

impl Body<'_> {
    /// The statements of `block`, without braces.
    pub(super) fn statements(&self, out: &mut Writer, statements: &[Statement]) -> Result<()> {
        statements
            .iter()
            .try_for_each(|statement| self.statement(out, statement))
    }

    fn block(&self, out: &mut Writer, block: &Block) -> Result<()> {
        self.statements(out, &block.statements)
    }

    fn statement(&self, out: &mut Writer, statement: &Statement) -> Result<()> {
        match &statement.kind {
            StatementKind::Variable(variable) => self.variable(out, variable),
            StatementKind::Assignment {
                target,
                operator,
                value,
            } => self.assignment(out, target, *operator, value),
            StatementKind::Expression(expression) => self.expression_statement(out, expression),
            StatementKind::If(if_statement) => {
                let condition = self.expression(&if_statement.condition)?;
                out.open(&format!("if {condition} {{"));
                self.if_rest(out, if_statement)
            }
            StatementKind::While { condition, body } => {
                out.open(&format!("while {} {{", self.expression(condition)?));
                self.block(out, body)?;
                out.close("}");
                Ok(())
            }
            StatementKind::For {
                variable,
                iterable,
                body,
            } => {
                let values = self.iterable(iterable)?;
                out.open(&format!("for {} in {values} {{", ident(&variable.text)));
                self.block(out, body)?;
                out.close("}");
                Ok(())
            }
            StatementKind::Return(value) => self.return_statement(out, value.as_ref()),
            StatementKind::Break => {
                out.line("break;");
                Ok(())
            }
            StatementKind::Continue => {
                out.line("continue;");
                Ok(())
            }
        }
    }

    /// A call alone: run, or started as a coroutine if it is `async`.
    fn expression_statement(&self, out: &mut Writer, expression: &Expr) -> Result<()> {
        let code = self.expression(expression)?;
        let ExprKind::Call { callee, .. } = &expression.kind else {
            out.line(&format!("{code};"));
            return Ok(());
        };
        if !self.context.calls_async_method(callee)? {
            out.line(&format!("{code};"));
            return Ok(());
        }
        let start = format!("{RUNTIME}::tasks::start");
        if matches!(
            callee.kind,
            ExprKind::Member {
                null_safe: true,
                ..
            }
        ) {
            out.line(&format!(
                "if let {SOME}(task_) = {code} {{ {start}(task_); }}"
            ));
        } else {
            out.line(&format!("{start}({code});"));
        }
        Ok(())
    }

    fn variable(&self, out: &mut Writer, variable: &VariableDecl) -> Result<()> {
        let span = variable.name.span;
        let analysis = self.context.analysis;
        let local = analysis
            .declared_local(&variable.name)
            .and_then(|id| analysis.local(id))
            .ok_or_else(|| internal("variable without analysis", span))?;
        let ty = self.context.rust_type(&local.ty, span)?;
        let value = match &variable.initializer {
            Some(value) => self.value_as(value, &local.ty)?,
            None => default_value(&local.ty, span)?,
        };
        let binding = match variable.kind {
            VariableKind::Var => "let mut",
            VariableKind::Const => "let",
        };
        out.line(&format!(
            "{binding} {}: {ty} = {value};",
            ident(&variable.name.text)
        ));
        Ok(())
    }

    /// Everything of an `if` after its condition.
    fn if_rest(&self, out: &mut Writer, if_statement: &IfStatement) -> Result<()> {
        self.block(out, &if_statement.then_block)?;
        match &if_statement.else_branch {
            None => out.close("}"),
            Some(ElseBranch::Block(block)) => {
                out.middle("} else {");
                self.block(out, block)?;
                out.close("}");
            }
            Some(ElseBranch::If(nested)) => {
                let condition = self.expression(&nested.condition)?;
                out.middle(&format!("}} else if {condition} {{"));
                self.if_rest(out, nested)?;
            }
        }
        Ok(())
    }

    /// What a `for` goes through: a range of integers, or a copy of the
    /// list's elements taken when the loop starts.
    fn iterable(&self, iterable: &Expr) -> Result<String> {
        if let ExprKind::Binary {
            operator: BinaryOperator::Range,
            left,
            right,
        } = &iterable.kind
        {
            return Ok(format!(
                "({})..({})",
                self.expression(left)?,
                self.expression(right)?
            ));
        }
        Ok(format!("({}).snapshot()", self.expression(iterable)?))
    }

    fn return_statement(&self, out: &mut Writer, value: Option<&Expr>) -> Result<()> {
        let line = match (self.kind, value) {
            (BodyKind::Constructor, _) => "return object_;".to_owned(),
            (_, Some(value)) => format!("return {};", self.value_as(value, &self.return_type)?),
            (_, None) => "return;".to_owned(),
        };
        out.line(&line);
        Ok(())
    }

    // --- Assignment ---

    fn assignment(
        &self,
        out: &mut Writer,
        target: &Expr,
        operator: AssignOperator,
        value: &Expr,
    ) -> Result<()> {
        let span = target.span;
        match &target.kind {
            ExprKind::Name(name) => match self.context.resolution(target)? {
                Resolution::Local(id) => {
                    let declared = &self
                        .context
                        .analysis
                        .local(id)
                        .ok_or_else(|| internal("unknown local", span))?
                        .ty;
                    let variable = ident(name);
                    let current = read_variable(&variable, declared);
                    let new_value = self.new_value(operator, declared, &current, value)?;
                    out.line(&format!("{variable} = {new_value};"));
                    Ok(())
                }
                Resolution::Field { class, index } => {
                    self.field_assignment(out, None, class, index, operator, value, span)
                }
                _ => Err(internal(
                    "assignment to something that is not a variable",
                    span,
                )),
            },
            ExprKind::Member { object, .. } => {
                let Resolution::Field { class, index } = self.context.resolution(target)? else {
                    return Err(internal(
                        "assignment to something that is not a field",
                        span,
                    ));
                };
                let object = (!matches!(object.kind, ExprKind::SelfValue)).then_some(&**object);
                self.field_assignment(out, object, class, index, operator, value, span)
            }
            ExprKind::Index { object, index } => {
                self.element_assignment(out, target, object, index, operator, value)
            }
            _ => Err(internal("assignment to an expression", span)),
        }
    }

    /// The value `target operator value` stores: `value` itself for `=`.
    fn new_value(
        &self,
        operator: AssignOperator,
        ty: &Type,
        current: &str,
        value: &Expr,
    ) -> Result<String> {
        match binary_operator(operator) {
            None => self.value_as(value, ty),
            Some(binary) => arithmetic(binary, ty, current, &self.expression(value)?, value.span),
        }
    }

    /// `field = value` on `self` (no `object`) or on `object.field = value`.
    #[allow(clippy::too_many_arguments)]
    fn field_assignment(
        &self,
        out: &mut Writer,
        object: Option<&Expr>,
        class: ClassId,
        index: usize,
        operator: AssignOperator,
        value: &Expr,
        span: Span,
    ) -> Result<()> {
        let field_type = &self
            .context
            .analysis
            .classes()
            .class(class)
            .fields
            .get(index)
            .ok_or_else(|| internal("unknown field", span))?
            .ty;
        if self.kind == BodyKind::ConstructorSetup && object.is_none() && class == self.class {
            // The object does not exist yet: keep the value to build it.
            let ty = self.context.rust_type(field_type, span)?;
            let value = self.value_as(value, field_type)?;
            out.line(&format!("let {}: {ty} = {value};", setup_field(index)));
            return Ok(());
        }
        let field = self.field_name(class, index, span)?;
        out.open("{");
        let holder = match object {
            Some(object) => {
                out.line(&format!("let target_ = {};", self.expression(object)?));
                self.fields_of(class, "target_")
            }
            None => self.own_fields(class),
        };
        out.line(&format!("let field_ = &{holder}.{field};"));
        let new_value = self.new_value(operator, field_type, "field_.get()", value)?;
        out.line(&format!("field_.set({new_value});"));
        out.close("}");
        Ok(())
    }

    /// `list[index] = value`
    fn element_assignment(
        &self,
        out: &mut Writer,
        target: &Expr,
        list: &Expr,
        index: &Expr,
        operator: AssignOperator,
        value: &Expr,
    ) -> Result<()> {
        let element = self.context.type_of(target)?;
        out.open("{");
        out.line(&format!("let list_ = {};", self.expression(list)?));
        out.line(&format!("let index_ = {};", self.expression(index)?));
        let new_value = self.new_value(operator, element, "list_.get(index_)", value)?;
        out.line(&format!("list_.set(index_, {new_value});"));
        out.close("}");
        Ok(())
    }
}

/// Name of the variable that keeps a field's value while `init` prepares
/// the object.
pub(super) fn setup_field(index: usize) -> String {
    format!("field{index}_")
}

/// The operator of `a op= b`, or `None` for plain `=`.
const fn binary_operator(operator: AssignOperator) -> Option<BinaryOperator> {
    match operator {
        AssignOperator::Assign => None,
        AssignOperator::Add => Some(BinaryOperator::Add),
        AssignOperator::Subtract => Some(BinaryOperator::Subtract),
        AssignOperator::Multiply => Some(BinaryOperator::Multiply),
        AssignOperator::Divide => Some(BinaryOperator::Divide),
        AssignOperator::Remainder => Some(BinaryOperator::Remainder),
    }
}
