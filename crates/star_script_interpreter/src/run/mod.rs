//! Running: walks the interpreter's tree with values.

mod builtins;
mod numbers;

use std::io::Write;
use std::rc::Rc;

use star_script::semantic::BuiltinMethod;
use star_script_runtime::try_index;

use crate::Program;
use crate::error::RunError;
use crate::tree::{Arithmetic, ConstructorId, Expression, FunctionId, MethodTarget, Statement};
use crate::value::{Elements, Object, Value, read, write};

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
    program: &'a Program,
    output: &'a mut dyn Write,
}

/// The locals of the function that runs.
type Frame = [Value];

impl<'a> Machine<'a> {
    pub(crate) fn new(program: &'a Program, output: &'a mut dyn Write) -> Self {
        Self { program, output }
    }

    /// Runs `function` with `arguments`, already of its parameters' types,
    /// and gives what it returns ([`Value::Null`] if nothing).
    pub(crate) fn call(
        &mut self,
        function: FunctionId,
        mut arguments: Vec<Value>,
    ) -> Result<Value, RunError> {
        let program = self.program;
        let function = program
            .functions
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
            Statement::SetField {
                object,
                position,
                operator,
                value,
            } => {
                let object = self.evaluate(object, frame)?;
                let object = object_of(&object)?;
                let new_value = match operator {
                    None => self.evaluate(value, frame)?,
                    Some(operator) => {
                        let current = object.field(*position)?;
                        let value = self.evaluate(value, frame)?;
                        numbers::arithmetic(*operator, &current, &value)?
                    }
                };
                object.set_field(*position, new_value)?;
            }
            Statement::SetElement {
                list,
                index,
                operator,
                value,
            } => self.set_element(list, index, *operator, value, frame)?,
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
            Statement::ForEach { slot, list, body } => {
                let list = self.evaluate(list, frame)?;
                // A copy: changing the list inside the loop does not change
                // what the loop goes through.
                let elements = read(list_elements(&list)?)?.clone();
                for element in elements {
                    store(frame, *slot, element)?;
                    if let Some(flow) = leave_loop(self.block(body, frame)?) {
                        return Ok(flow);
                    }
                }
            }
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

    /// Builds an object: the fields' values and the start of `init` run
    /// before the object exists, then the rest of `init` with `self`.
    fn construct(
        &mut self,
        constructor: ConstructorId,
        mut frame: Vec<Value>,
    ) -> Result<Value, RunError> {
        let program = self.program;
        let constructor = program
            .constructors
            .get(constructor.0)
            .ok_or(RunError::Internal("unknown constructor"))?;
        frame.resize(constructor.slots, Value::Null);
        if !matches!(self.block(&constructor.setup, &mut frame)?, Flow::Next) {
            return Err(RunError::Internal("`init` left before its object exists"));
        }
        let fields = constructor
            .fields
            .iter()
            .map(|slot| {
                frame
                    .get(*slot)
                    .cloned()
                    .ok_or(RunError::Internal("field value outside its constructor"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let object = Value::Object(Rc::new(Object::new(constructor.class, fields)));
        store(&mut frame, constructor.self_slot, object.clone())?;
        // A `return` in `init` only ends it: the object is built anyway.
        self.block(&constructor.rest, &mut frame)?;
        Ok(object)
    }

    /// A method of the object `receiver`, with `self` as its first local.
    fn call_method(
        &mut self,
        target: MethodTarget,
        receiver: Value,
        arguments: &[Expression],
        frame: &Frame,
    ) -> Result<Value, RunError> {
        let function = match target {
            MethodTarget::Direct(function) => function,
            MethodTarget::Overridable(place) => {
                let class = object_of(&receiver)?.class;
                self.program
                    .classes
                    .get(class.0)
                    .and_then(|class| class.methods.get(place))
                    .copied()
                    .ok_or(RunError::Internal("method missing from its class's table"))?
            }
        };
        let mut values = Vec::with_capacity(arguments.len() + 1);
        values.push(receiver);
        for argument in arguments {
            values.push(self.evaluate(argument, frame)?);
        }
        self.call(function, values)
    }

    /// `list[index] = value` or `list[index] op= value`.
    fn set_element(
        &mut self,
        list: &Expression,
        index: &Expression,
        operator: Option<Arithmetic>,
        value: &Expression,
        frame: &Frame,
    ) -> Result<(), RunError> {
        let list = self.evaluate(list, frame)?;
        let elements = list_elements(&list)?;
        let index = self.evaluate(index, frame)?;
        let new_value = match operator {
            None => self.evaluate(value, frame)?,
            Some(operator) => {
                let current = element(elements, &index)?;
                let value = self.evaluate(value, frame)?;
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
                let (left, right) = self.evaluate_pair(left, right, frame)?;
                numbers::arithmetic(*operator, &left, &right)?
            }
            Expression::Compare {
                operator,
                left,
                right,
            } => {
                let (left, right) = self.evaluate_pair(left, right, frame)?;
                Value::Bool(numbers::compare(*operator, &left, &right)?)
            }
            Expression::Equal(left, right) => {
                let (left, right) = self.evaluate_pair(left, right, frame)?;
                Value::Bool(left.star_eq(&right)?)
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
                let arguments = self.evaluate_all(arguments, frame)?;
                self.call(*function, arguments)?
            }
            Expression::CallMethod {
                target,
                receiver,
                arguments,
                null_safe,
            } => {
                let receiver = self.evaluate(receiver, frame)?;
                if *null_safe && matches!(receiver, Value::Null) {
                    return Ok(Value::Null);
                }
                self.call_method(*target, receiver, arguments, frame)?
            }
            Expression::Construct {
                constructor,
                arguments,
            } => {
                let arguments = self.evaluate_all(arguments, frame)?;
                self.construct(*constructor, arguments)?
            }
            Expression::Field {
                object,
                position,
                null_safe,
            } => match self.evaluate(object, frame)? {
                Value::Null if *null_safe => Value::Null,
                object => object_of(&object)?.field(*position)?,
            },
            Expression::Print(message) => {
                let message = self.evaluate(message, frame)?.to_text()?;
                writeln!(self.output, "{message}").map_err(RunError::Output)?;
                Value::Null
            }
            Expression::List(elements) => Value::list(self.evaluate_all(elements, frame)?),
            Expression::Index { list, index } => {
                let list = self.evaluate(list, frame)?;
                element(list_elements(&list)?, &self.evaluate(index, frame)?)?
            }
            Expression::Coalesce(value, fallback) => match self.evaluate(value, frame)? {
                Value::Null => self.evaluate(fallback, frame)?,
                value => value,
            },
            Expression::Builtin {
                method,
                receiver,
                argument,
                null_safe,
            } => self.builtin(*method, receiver, argument.as_deref(), *null_safe, frame)?,
        })
    }

    /// A method of a basic type or list; with `?.`, `null` without
    /// running the argument.
    fn builtin(
        &mut self,
        method: BuiltinMethod,
        receiver: &Expression,
        argument: Option<&Expression>,
        null_safe: bool,
        frame: &Frame,
    ) -> Result<Value, RunError> {
        let receiver = self.evaluate(receiver, frame)?;
        if null_safe && matches!(receiver, Value::Null) {
            return Ok(Value::Null);
        }
        let argument = argument
            .map(|argument| self.evaluate(argument, frame))
            .transpose()?;
        builtins::call(method, &receiver, argument)
    }

    /// The values of `left` and then `right`.
    fn evaluate_pair(
        &mut self,
        left: &Expression,
        right: &Expression,
        frame: &Frame,
    ) -> Result<(Value, Value), RunError> {
        let left = self.evaluate(left, frame)?;
        Ok((left, self.evaluate(right, frame)?))
    }

    /// The values of `expressions`, in order.
    fn evaluate_all(
        &mut self,
        expressions: &[Expression],
        frame: &Frame,
    ) -> Result<Vec<Value>, RunError> {
        expressions
            .iter()
            .map(|expression| self.evaluate(expression, frame))
            .collect()
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

/// The object a value refers to.
fn object_of(value: &Value) -> Result<&Object, RunError> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(RunError::Internal(
            "member of something that is not an object",
        )),
    }
}

/// The elements of a list value.
fn list_elements(value: &Value) -> Result<&Elements, RunError> {
    match value {
        Value::List(elements) => Ok(elements),
        _ => Err(RunError::Internal("index of something that is not a list")),
    }
}

/// The position `index` names in a list of `length` elements; outside the
/// list it is the same runtime error as in the compiled game.
fn position(index: &Value, length: usize) -> Result<usize, RunError> {
    match index {
        Value::Int(index) => Ok(try_index(*index, length)?),
        _ => Err(RunError::Internal("index that is not an `int`")),
    }
}

/// `list[index]`.
fn element(elements: &Elements, index: &Value) -> Result<Value, RunError> {
    let elements = read(elements)?;
    let position = position(index, elements.len())?;
    elements
        .get(position)
        .cloned()
        .ok_or(RunError::Internal("checked index out of range"))
}

/// Puts `value` in the local `slot`.
fn store(frame: &mut Frame, slot: usize, value: Value) -> Result<(), RunError> {
    let local = frame
        .get_mut(slot)
        .ok_or(RunError::Internal("local outside its function"))?;
    *local = value;
    Ok(())
}
