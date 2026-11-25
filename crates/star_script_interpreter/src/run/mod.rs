//! Running: walks the interpreter's tree with values.

mod numbers;

use std::io::Write;

use crate::error::RunError;
use crate::tree::{Expression, Function, FunctionId, Statement};
use crate::value::Value;

/// What a statement leaves the code around it to do.
enum Flow {
    /// Go on with the next statement.
    Next,
    /// Leave the innermost loop.
    Break,
    /// Start the next repetition of the innermost loop.
    Continue,
    /// Leave the function with this value.
    Return(Value),
}

/// Runs the functions of a program.
pub(crate) struct Machine<'a> {
    functions: &'a [Function],
    output: &'a mut dyn Write,
}

/// The locals of the function that runs.
type Frame = [Value];

impl<'a> Machine<'a> {
    pub(crate) fn new(functions: &'a [Function], output: &'a mut dyn Write) -> Self {
        Self { functions, output }
    }

    /// Runs `function` with `arguments`, already of its parameters' types,
    /// and gives what it returns ([`Value::Null`] if nothing).
    pub(crate) fn call(
        &mut self,
        function: FunctionId,
        mut arguments: Vec<Value>,
    ) -> Result<Value, RunError> {
        let functions = self.functions;
        let function = functions
            .get(function.0)
            .ok_or(RunError::Internal("unknown function"))?;
        arguments.resize(function.slots, Value::Null);
        match self.block(&function.body, &mut arguments)? {
            Flow::Return(value) => Ok(value),
            Flow::Next | Flow::Break | Flow::Continue => Ok(Value::Null),
        }
    }

    fn block(&mut self, statements: &[Statement], frame: &mut Frame) -> Result<Flow, RunError> {
        for statement in statements {
            let flow = self.statement(statement, frame)?;
            if !matches!(flow, Flow::Next) {
                return Ok(flow);
            }
        }
        Ok(Flow::Next)
    }

    fn statement(&mut self, statement: &Statement, frame: &mut Frame) -> Result<Flow, RunError> {
        match statement {
            Statement::Store { slot, value } => {
                let value = self.evaluate(value, frame)?;
                store(frame, *slot, value)?;
            }
            Statement::Evaluate(expression) => {
                self.evaluate(expression, frame)?;
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let branch = if self.condition(condition, frame)? {
                    then_branch
                } else {
                    else_branch
                };
                return self.block(branch, frame);
            }
            Statement::While { condition, body } => {
                while self.condition(condition, frame)? {
                    if let Some(flow) = leave_loop(self.block(body, frame)?) {
                        return Ok(flow);
                    }
                }
            }
            Statement::ForRange {
                slot,
                start,
                end,
                body,
            } => return self.for_range(*slot, start, end, body, frame),
            Statement::Return(value) => {
                let value = match value {
                    Some(value) => self.evaluate(value, frame)?,
                    None => Value::Null,
                };
                return Ok(Flow::Return(value));
            }
            Statement::Break => return Ok(Flow::Break),
            Statement::Continue => return Ok(Flow::Continue),
        }
        Ok(Flow::Next)
    }

    /// `for slot in start..end`: the ends are read once, as a Rust range.
    fn for_range(
        &mut self,
        slot: usize,
        start: &Expression,
        end: &Expression,
        body: &[Statement],
        frame: &mut Frame,
    ) -> Result<Flow, RunError> {
        let start = self.evaluate(start, frame)?;
        let end = numbers::integer(&self.evaluate(end, frame)?)?;
        let mut current = numbers::integer(&start)?;
        while current < end {
            store(frame, slot, numbers::same_integer(&start, current)?)?;
            if let Some(flow) = leave_loop(self.block(body, frame)?) {
                return Ok(flow);
            }
            current += 1;
        }
        Ok(Flow::Next)
    }

    fn condition(&mut self, condition: &Expression, frame: &Frame) -> Result<bool, RunError> {
        match self.evaluate(condition, frame)? {
            Value::Bool(value) => Ok(value),
            _ => Err(RunError::Internal("condition that is not a `bool`")),
        }
    }

    fn evaluate(&mut self, expression: &Expression, frame: &Frame) -> Result<Value, RunError> {
        Ok(match expression {
            Expression::Constant(value) => value.clone(),
            Expression::Local(slot) => frame
                .get(*slot)
                .cloned()
                .ok_or(RunError::Internal("local outside its function"))?,
            Expression::Convert { value, to } => {
                numbers::convert(&self.evaluate(value, frame)?, *to)?
            }
            Expression::Negate(value) => numbers::negate(&self.evaluate(value, frame)?)?,
            Expression::Not(value) => Value::Bool(!self.condition(value, frame)?),
            Expression::Arithmetic {
                operator,
                left,
                right,
            } => {
                let left = self.evaluate(left, frame)?;
                let right = self.evaluate(right, frame)?;
                numbers::arithmetic(*operator, &left, &right)?
            }
            Expression::Compare {
                operator,
                left,
                right,
            } => {
                let left = self.evaluate(left, frame)?;
                let right = self.evaluate(right, frame)?;
                Value::Bool(numbers::compare(*operator, &left, &right)?)
            }
            Expression::Equal(left, right) => {
                let left = self.evaluate(left, frame)?;
                Value::Bool(left.star_eq(&self.evaluate(right, frame)?))
            }
            Expression::And(left, right) => {
                Value::Bool(self.condition(left, frame)? && self.condition(right, frame)?)
            }
            Expression::Or(left, right) => {
                Value::Bool(self.condition(left, frame)? || self.condition(right, frame)?)
            }
            Expression::Call {
                function,
                arguments,
            } => {
                let arguments = arguments
                    .iter()
                    .map(|argument| self.evaluate(argument, frame))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call(*function, arguments)?
            }
            Expression::Print(message) => {
                let message = self.evaluate(message, frame)?.to_text();
                writeln!(self.output, "{message}").map_err(RunError::Output)?;
                Value::Null
            }
            Expression::ToText(value) => Value::String(self.evaluate(value, frame)?.to_text()),
        })
    }
}

/// The flow that leaves a loop after a repetition ended with `flow`, or
/// `None` to go on with the next repetition.
fn leave_loop(flow: Flow) -> Option<Flow> {
    match flow {
        Flow::Next | Flow::Continue => None,
        Flow::Break => Some(Flow::Next),
        Flow::Return(_) => Some(flow),
    }
}

/// Puts `value` in the local `slot`.
fn store(frame: &mut Frame, slot: usize, value: Value) -> Result<(), RunError> {
    let local = frame
        .get_mut(slot)
        .ok_or(RunError::Internal("local outside its function"))?;
    *local = value;
    Ok(())
}
