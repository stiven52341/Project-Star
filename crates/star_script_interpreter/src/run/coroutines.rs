//! Coroutines (decision 68): the code of `async` methods that can wait,
//! run as Rust futures on the scheduler of `star_script_runtime`, which
//! compiled coroutines use too, so both start, wait and go on in the same
//! order.
//!
//! Only the statements and expressions that lowering marked as waiting
//! (`Waits`, `Await`) run here; everything inside them that does not wait
//! goes back to the usual runner, without allocating futures.

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use star_script_runtime::{Task, tasks};

use super::{
    Flow, Frame, Machine, builtins, element, engine, leave_loop, list_elements, numbers, position,
    store,
};
use crate::error::RunError;
use crate::tree::{Arithmetic, Coroutine, Expression, FunctionId, Statement};
use crate::value::{Value, read, write};

/// A part of a coroutine that may wait, boxed: the runner calls itself, and
/// a future cannot contain itself.
type Pending<'s, T> = Pin<Box<dyn Future<Output = Result<T, RunError>> + 's>>;

/// A coroutine whose receiver and arguments were evaluated.
pub(super) enum Ready {
    /// An `async` method, with its arguments (the object first).
    Function(FunctionId, Vec<Value>),
    /// A `parallel` function, with its arguments.
    Parallel(FunctionId, Vec<Value>),
    /// `Time.wait(seconds)`.
    Wait(f32),
    /// `Time.next_frame()`.
    NextFrame,
}

impl Machine {
    /// Evaluates the receiver and arguments of a coroutine; `None` for a
    /// `?.` call on `null`, which starts nothing.
    pub(super) fn prepare(
        &self,
        coroutine: &Coroutine,
        frame: &Frame,
    ) -> Result<Option<Ready>, RunError> {
        Ok(Some(match coroutine {
            Coroutine::Call {
                function,
                arguments,
            } => Ready::Function(*function, self.evaluate_all(arguments, frame)?),
            Coroutine::Parallel {
                function,
                arguments,
            } => Ready::Parallel(*function, self.evaluate_all(arguments, frame)?),
            Coroutine::Method {
                target,
                receiver,
                arguments,
                null_safe,
            } => {
                let receiver = self.evaluate(receiver, frame)?;
                if *null_safe && matches!(receiver, Value::Null) {
                    return Ok(None);
                }
                let function = self.method_function(*target, &receiver)?;
                let mut values = Vec::with_capacity(arguments.len() + 1);
                values.push(receiver);
                values.extend(self.evaluate_all(arguments, frame)?);
                Ready::Function(function, values)
            }
            Coroutine::Wait(seconds) => Ready::Wait(seconds_of(&self.evaluate(seconds, frame)?)?),
            Coroutine::NextFrame => Ready::NextFrame,
        }))
    }

    /// [`Machine::prepare`] for a coroutine whose receiver or arguments wait.
    async fn prepare_waiting(
        &self,
        coroutine: &Coroutine,
        frame: &Frame,
    ) -> Result<Option<Ready>, RunError> {
        Ok(Some(match coroutine {
            Coroutine::Call {
                function,
                arguments,
            } => Ready::Function(*function, self.values(arguments, frame).await?),
            Coroutine::Parallel {
                function,
                arguments,
            } => Ready::Parallel(*function, self.values(arguments, frame).await?),
            Coroutine::Method {
                target,
                receiver,
                arguments,
                null_safe,
            } => {
                let receiver = self.value(receiver, frame).await?;
                if *null_safe && matches!(receiver, Value::Null) {
                    return Ok(None);
                }
                let function = self.method_function(*target, &receiver)?;
                let mut values = Vec::with_capacity(arguments.len() + 1);
                values.push(receiver);
                values.extend(self.values(arguments, frame).await?);
                Ready::Function(function, values)
            }
            Coroutine::Wait(seconds) => {
                Ready::Wait(seconds_of(&self.value(seconds, frame).await?)?)
            }
            Coroutine::NextFrame => Ready::NextFrame,
        }))
    }

    /// Starts a coroutine: it runs at once until its first wait. If it
    /// fails there, the error stops the code that started it, as in the
    /// compiled game.
    pub(super) fn start(&self, ready: Ready) -> Result<(), RunError> {
        match ready {
            Ready::Wait(seconds) => tasks::start(tasks::wait(seconds)),
            Ready::NextFrame => tasks::start(tasks::next_frame()),
            // The analysis only lets `parallel` functions be awaited.
            Ready::Parallel(..) => return Err(PARALLEL_NOT_AWAITED),
            Ready::Function(function, arguments) => {
                let machine = self.clone();
                let coroutine = async move {
                    if let Err(error) = machine.call_waiting(function, arguments).await {
                        machine.fail(error);
                    }
                };
                // After a failure the program stops: no coroutine goes on.
                let failure = Rc::clone(&self.failure);
                let alive = move || failure.try_borrow().is_ok_and(|failure| failure.is_none());
                tasks::start(Task::owned(coroutine, alive));
            }
        }
        self.take_failure().map_or(Ok(()), Err)
    }

    /// A call of an `async` method written alone: it starts there.
    pub(super) fn start_call(&self, coroutine: &Coroutine, frame: &Frame) -> Result<(), RunError> {
        match self.prepare(coroutine, frame)? {
            Some(ready) => self.start(ready),
            None => Ok(()),
        }
    }

    /// Keeps the first error of a coroutine.
    fn fail(&self, error: RunError) {
        if let Ok(mut failure) = self.failure.try_borrow_mut() {
            failure.get_or_insert(error);
        }
    }

    /// Whether a coroutine failed.
    pub(super) fn has_failed(&self) -> bool {
        self.failure
            .try_borrow()
            .is_ok_and(|failure| failure.is_some())
    }

    /// The error of the coroutine that failed, if one did.
    pub(super) fn take_failure(&self) -> Option<RunError> {
        self.failure.try_borrow_mut().ok()?.take()
    }

    /// Runs an `async` method as part of the coroutine that calls it, and
    /// gives what it returns.
    async fn call_waiting(
        &self,
        function: FunctionId,
        mut arguments: Vec<Value>,
    ) -> Result<Value, RunError> {
        let function = self.function(function)?;
        arguments.resize(function.slots, Value::Null);
        match self.block_waiting(&function.body, &mut arguments).await? {
            Flow::Return(value) => Ok(value),
            Flow::Next | Flow::Break | Flow::Continue => Ok(Value::Null),
        }
    }

    async fn block_waiting(
        &self,
        statements: &[Statement],
        frame: &mut Frame,
    ) -> Result<Flow, RunError> {
        for statement in statements {
            let flow = match statement {
                Statement::Waits(statement) => self.statement_waiting(statement, frame).await?,
                statement => self.statement(statement, frame)?,
            };
            if !matches!(flow, Flow::Next) {
                return Ok(flow);
            }
        }
        Ok(Flow::Next)
    }

    /// A statement with an `await` inside.
    fn statement_waiting<'s>(
        &'s self,
        statement: &'s Statement,
        frame: &'s mut Frame,
    ) -> Pending<'s, Flow> {
        Box::pin(async move {
            match statement {
                Statement::Store { slot, value } => {
                    let value = self.value(value, frame).await?;
                    store(frame, *slot, value)?;
                }
                Statement::Evaluate(expression) => {
                    self.value(expression, frame).await?;
                }
                Statement::SetElement {
                    list,
                    index,
                    operator,
                    value,
                } => {
                    self.set_element_waiting(list, index, *operator, value, frame)
                        .await?;
                }
                Statement::SetField {
                    object,
                    field,
                    operator,
                    value,
                } => {
                    let object = self.value(object, frame).await?;
                    let new_value = match operator {
                        None => self.value(value, frame).await?,
                        Some(operator) => {
                            let current = engine::read_field(&object, *field)?;
                            let value = self.value(value, frame).await?;
                            numbers::arithmetic(*operator, &current, &value)?
                        }
                    };
                    engine::write_field(&object, *field, new_value)?;
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    let branch = if self.condition_waiting(condition, frame).await? {
                        then_branch
                    } else {
                        else_branch
                    };
                    return self.block_waiting(branch, frame).await;
                }
                Statement::While { condition, body } => {
                    while self.condition_waiting(condition, frame).await? {
                        if let Some(flow) = leave_loop(self.block_waiting(body, frame).await?) {
                            return Ok(flow);
                        }
                    }
                }
                Statement::ForRange {
                    slot,
                    start,
                    end,
                    body,
                } => {
                    let start = self.value(start, frame).await?;
                    let end = numbers::integer(&self.value(end, frame).await?)?;
                    let mut current = numbers::integer(&start)?;
                    while current < end {
                        store(frame, *slot, numbers::same_integer(&start, current)?)?;
                        if let Some(flow) = leave_loop(self.block_waiting(body, frame).await?) {
                            return Ok(flow);
                        }
                        current += 1;
                    }
                }
                Statement::ForEach { slot, list, body } => {
                    let list = self.value(list, frame).await?;
                    let elements = read(list_elements(&list)?)?.clone();
                    for element in elements {
                        store(frame, *slot, element)?;
                        if let Some(flow) = leave_loop(self.block_waiting(body, frame).await?) {
                            return Ok(flow);
                        }
                    }
                }
                Statement::Return(value) => {
                    let value = match value {
                        Some(value) => self.value(value, frame).await?,
                        None => Value::Null,
                    };
                    return Ok(Flow::Return(value));
                }
                Statement::Break => return Ok(Flow::Break),
                Statement::Continue => return Ok(Flow::Continue),
                Statement::Waits(statement) => {
                    return self.statement_waiting(statement, frame).await;
                }
            }
            Ok(Flow::Next)
        })
    }

    /// `list[index] = value` or `list[index] op= value`, in the order of
    /// the usual runner.
    async fn set_element_waiting(
        &self,
        list: &Expression,
        index: &Expression,
        operator: Option<Arithmetic>,
        value: &Expression,
        frame: &Frame,
    ) -> Result<(), RunError> {
        let list = self.value(list, frame).await?;
        let elements = list_elements(&list)?;
        let index = self.value(index, frame).await?;
        let new_value = match operator {
            None => self.value(value, frame).await?,
            Some(operator) => {
                let current = element(elements, &index)?;
                let value = self.value(value, frame).await?;
                numbers::arithmetic(operator, &current, &value)?
            }
        };
        let mut elements = write(elements)?;
        let position = position(&index, elements.len())?;
        let slot = elements
            .get_mut(position)
            .ok_or(RunError::Internal("checked index out of range"))?;
        *slot = new_value;
        Ok(())
    }

    /// The value of any expression inside a coroutine: what does not wait
    /// goes to the usual runner.
    async fn value(&self, expression: &Expression, frame: &Frame) -> Result<Value, RunError> {
        match expression {
            Expression::Waits(inner) => self.evaluate_waiting(inner, frame).await,
            Expression::Await(_) => self.evaluate_waiting(expression, frame).await,
            expression => self.evaluate(expression, frame),
        }
    }

    async fn condition_waiting(
        &self,
        condition: &Expression,
        frame: &Frame,
    ) -> Result<bool, RunError> {
        match self.value(condition, frame).await? {
            Value::Bool(value) => Ok(value),
            _ => Err(RunError::Internal("condition that is not a `bool`")),
        }
    }

    /// The values of `expressions`, in order.
    async fn values(
        &self,
        expressions: &[Expression],
        frame: &Frame,
    ) -> Result<Vec<Value>, RunError> {
        let mut values = Vec::with_capacity(expressions.len());
        for expression in expressions {
            values.push(self.value(expression, frame).await?);
        }
        Ok(values)
    }

    /// An expression with an `await` inside, or an `await`: as the usual
    /// runner evaluates it, with its parts in the same order.
    fn evaluate_waiting<'s>(
        &'s self,
        expression: &'s Expression,
        frame: &'s Frame,
    ) -> Pending<'s, Value> {
        Box::pin(async move {
            Ok(match expression {
                Expression::Await(coroutine) => self.await_coroutine(coroutine, frame).await?,
                Expression::Waits(inner) => self.evaluate_waiting(inner, frame).await?,
                Expression::Start(coroutine) => {
                    if let Some(ready) = self.prepare_waiting(coroutine, frame).await? {
                        self.start(ready)?;
                    }
                    Value::Null
                }
                Expression::Convert { value, to } => {
                    numbers::convert(&self.value(value, frame).await?, *to)?
                }
                Expression::Negate(value) => numbers::negate(&self.value(value, frame).await?)?,
                Expression::Not(value) => Value::Bool(!self.condition_waiting(value, frame).await?),
                Expression::Arithmetic {
                    operator,
                    left,
                    right,
                } => {
                    let left = self.value(left, frame).await?;
                    let right = self.value(right, frame).await?;
                    numbers::arithmetic(*operator, &left, &right)?
                }
                Expression::Compare {
                    operator,
                    left,
                    right,
                } => {
                    let left = self.value(left, frame).await?;
                    let right = self.value(right, frame).await?;
                    Value::Bool(numbers::compare(*operator, &left, &right)?)
                }
                Expression::Equal(left, right) => {
                    let left = self.value(left, frame).await?;
                    let right = self.value(right, frame).await?;
                    Value::Bool(left.star_eq(&right)?)
                }
                Expression::And(left, right) => Value::Bool(
                    self.condition_waiting(left, frame).await?
                        && self.condition_waiting(right, frame).await?,
                ),
                Expression::Or(left, right) => Value::Bool(
                    self.condition_waiting(left, frame).await?
                        || self.condition_waiting(right, frame).await?,
                ),
                Expression::Coalesce(value, fallback) => match self.value(value, frame).await? {
                    Value::Null => self.value(fallback, frame).await?,
                    value => value,
                },
                expression => self.evaluate_parts_waiting(expression, frame).await?,
            })
        })
    }

    /// The rest of [`Machine::evaluate_waiting`]: calls, objects and lists.
    async fn evaluate_parts_waiting(
        &self,
        expression: &Expression,
        frame: &Frame,
    ) -> Result<Value, RunError> {
        Ok(match expression {
            Expression::Call {
                function,
                arguments,
            } => {
                let arguments = self.values(arguments, frame).await?;
                self.call(*function, arguments)?
            }
            Expression::CallMethod {
                target,
                receiver,
                arguments,
                null_safe,
            } => {
                let receiver = self.value(receiver, frame).await?;
                if *null_safe && matches!(receiver, Value::Null) {
                    return Ok(Value::Null);
                }
                let function = self.method_function(*target, &receiver)?;
                let mut values = Vec::with_capacity(arguments.len() + 1);
                values.push(receiver);
                values.extend(self.values(arguments, frame).await?);
                self.call(function, values)?
            }
            Expression::Construct {
                constructor,
                arguments,
            } => {
                let arguments = self.values(arguments, frame).await?;
                self.construct(*constructor, arguments)?
            }
            Expression::Field {
                object,
                field,
                null_safe,
            } => match self.value(object, frame).await? {
                Value::Null if *null_safe => Value::Null,
                object => engine::read_field(&object, *field)?,
            },
            Expression::Engine { method, arguments } => {
                engine::call(*method, &self.values(arguments, frame).await?)?
            }
            Expression::IsInstance { value, class } => {
                let value = self.value(value, frame).await?;
                Value::Bool(self.is_instance(&value, *class))
            }
            Expression::Downcast { value, class } => {
                self.downcast(self.value(value, frame).await?, *class)
            }
            Expression::Print(message) => {
                let message = self.value(message, frame).await?.to_text()?;
                self.print(&message)?;
                Value::Null
            }
            Expression::List(elements) => Value::list(self.values(elements, frame).await?),
            Expression::Index { list, index } => {
                let list = self.value(list, frame).await?;
                let index = self.value(index, frame).await?;
                element(list_elements(&list)?, &index)?
            }
            Expression::Builtin {
                method,
                receiver,
                argument,
                null_safe,
            } => {
                let receiver = self.value(receiver, frame).await?;
                if *null_safe && matches!(receiver, Value::Null) {
                    return Ok(Value::Null);
                }
                let argument = match argument {
                    Some(argument) => Some(self.value(argument, frame).await?),
                    None => None,
                };
                builtins::call(*method, &receiver, argument)?
            }
            expression => self.evaluate(expression, frame)?,
        })
    }

    /// `await call`: the coroutine runs as part of this one, and its value
    /// comes back when it finishes.
    async fn await_coroutine(
        &self,
        coroutine: &Coroutine,
        frame: &Frame,
    ) -> Result<Value, RunError> {
        match self.prepare_waiting(coroutine, frame).await? {
            Some(Ready::Function(function, arguments)) => {
                self.call_waiting(function, arguments).await
            }
            Some(Ready::Parallel(function, arguments)) => {
                self.run_parallel(function, &arguments).await
            }
            Some(Ready::Wait(seconds)) => {
                tasks::wait(seconds).await;
                Ok(Value::Null)
            }
            Some(Ready::NextFrame) => {
                tasks::next_frame().await;
                Ok(Value::Null)
            }
            None => Err(RunError::Internal("`await` through `?.`")),
        }
    }
}

/// A `parallel` function called without `await`, which the analysis
/// refuses.
const PARALLEL_NOT_AWAITED: RunError = RunError::Internal("a `parallel` call not awaited");

/// The seconds of `Time.wait`.
fn seconds_of(value: &Value) -> Result<f32, RunError> {
    match value {
        Value::Float(seconds) => Ok(*seconds),
        _ => Err(RunError::Internal("seconds that are not a `float`")),
    }
}
