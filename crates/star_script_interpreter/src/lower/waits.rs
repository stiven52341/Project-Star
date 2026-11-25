//! Marks the code of `async` methods that contains an `await`, so the
//! runner of coroutines knows which parts may stop and go on in another
//! frame, and hands everything else to the usual runner (decision 68).

use crate::tree::{Coroutine, Expression, Literal, Statement};

/// Marks the statements of a body; whether any of them waits.
pub(super) fn mark_block(statements: &mut [Statement]) -> bool {
    statements
        .iter_mut()
        .fold(false, |waits, statement| mark_statement(statement) | waits)
}

/// Marks `statement` (and what is inside it); whether it waits.
fn mark_statement(statement: &mut Statement) -> bool {
    let waits = match statement {
        Statement::Store { value, .. }
        | Statement::Evaluate(value)
        | Statement::Return(Some(value)) => mark(value),
        Statement::SetElement {
            list, index, value, ..
        } => mark(list) | mark(index) | mark(value),
        Statement::SetField { object, value, .. } => mark(object) | mark(value),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => mark(condition) | mark_block(then_branch) | mark_block(else_branch),
        Statement::While { condition, body } => mark(condition) | mark_block(body),
        Statement::ForRange {
            start, end, body, ..
        } => mark(start) | mark(end) | mark_block(body),
        Statement::ForEach { list, body, .. } => mark(list) | mark_block(body),
        Statement::Waits(_) => return true,
        Statement::Return(None) | Statement::Break | Statement::Continue => false,
    };
    if waits {
        let inner = std::mem::replace(statement, Statement::Break);
        *statement = Statement::Waits(Box::new(inner));
    }
    waits
}

/// Marks `expression` (and what is inside it); whether it waits.
fn mark(expression: &mut Expression) -> bool {
    let waits = match expression {
        Expression::Constant(_) | Expression::Local(_) | Expression::Now => false,
        Expression::Convert { value, .. }
        | Expression::Negate(value)
        | Expression::Not(value)
        | Expression::Print(value)
        | Expression::IsInstance { value, .. }
        | Expression::Downcast { value, .. }
        | Expression::Field { object: value, .. } => mark(value),
        Expression::Arithmetic { left, right, .. }
        | Expression::Compare { left, right, .. }
        | Expression::Equal(left, right)
        | Expression::And(left, right)
        | Expression::Or(left, right)
        | Expression::Coalesce(left, right)
        | Expression::Index {
            list: left,
            index: right,
        } => mark(left) | mark(right),
        Expression::Call { arguments, .. }
        | Expression::Construct { arguments, .. }
        | Expression::List(arguments) => mark_all(arguments),
        Expression::CallMethod {
            receiver,
            arguments,
            ..
        } => mark(receiver) | mark_all(arguments),
        Expression::Builtin {
            receiver, argument, ..
        } => mark(receiver) | argument.as_deref_mut().is_some_and(mark),
        Expression::Start(coroutine) => mark_coroutine(coroutine),
        Expression::Await(coroutine) => {
            mark_coroutine(coroutine);
            return true;
        }
        Expression::Waits(_) => return true,
    };
    if waits {
        let inner = std::mem::replace(expression, Expression::Constant(Literal::Null));
        *expression = Expression::Waits(Box::new(inner));
    }
    waits
}

fn mark_all(expressions: &mut [Expression]) -> bool {
    expressions
        .iter_mut()
        .fold(false, |waits, expression| mark(expression) | waits)
}

/// Marks the receiver and arguments of a coroutine; whether one waits.
fn mark_coroutine(coroutine: &mut Coroutine) -> bool {
    match coroutine {
        Coroutine::Call { arguments, .. } | Coroutine::Parallel { arguments, .. } => {
            mark_all(arguments)
        }
        Coroutine::Method {
            receiver,
            arguments,
            ..
        } => mark(receiver) | mark_all(arguments),
        Coroutine::Wait(seconds) => mark(seconds),
        Coroutine::NextFrame => false,
    }
}
