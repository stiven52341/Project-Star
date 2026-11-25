//! Running: walks the interpreter's tree with values.

mod builtins;
mod numbers;

use std::rc::Rc;

use star_script::semantic::BuiltinMethod;
use star_script_runtime::try_index;

use crate::Output;
use crate::error::RunError;
use crate::tree::{
    Arithmetic, ClassRef, Code, Constructor, ConstructorId, Expression, FunctionId, MethodTarget,
    Statement,
};
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

/// Runs the functions of a program. It owns what it uses, so the
/// coroutines it starts can keep it after the call that started them.
pub(crate) struct Machine {
    code: Rc<Code>,
    output: Output,
}

/// The locals of the function that runs.
type Frame = [Value];

impl Machine {
    pub(crate) const fn new(code: Rc<Code>, output: Output) -> Self {
        Self { code, output }
    }

    /// Runs `function` with `arguments`, already of its parameters' types,
    /// and gives what it returns ([`Value::Null`] if nothing).
    pub(crate) fn call(
        &self,
        function: FunctionId,
        mut arguments: Vec<Value>,
    ) -> Result<Value, RunError> {
        let function = self
            .code
            .functions
            .get(function.0)
            .ok_or(RunError::Internal("unknown function"))?;
        arguments.resize(function.slots, Value::Null);
        match self.block(&function.body, &mut arguments)? {
            Flow::Return(value) => Ok(value),
            Flow::Next | Flow::Break | Flow::Continue => Ok(Value::Null),
        }
    }

    fn block(&self, statements: &[Statement], frame: &mut Frame) -> Result<Flow, RunError> {
        for statement in statements {
            let flow = self.statement(statement, frame)?;
            if !matches!(flow, Flow::Next) {
                return Ok(flow);
            }
        }
        Ok(Flow::Next)
    }

    fn statement(&self, statement: &Statement, frame: &mut Frame) -> Result<Flow, RunError> {
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
        &self,
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

    /// Builds an object of the constructor's class.
    fn construct(
        &self,
        constructor: ConstructorId,
        arguments: Vec<Value>,
    ) -> Result<Value, RunError> {
        let class = self.constructor(constructor)?.class;
        let object = self.build(constructor, arguments, class, &mut Vec::new())?;
        Ok(Value::Object(object))
    }

    /// Runs a constructor for an object of `class`: its fields' values and
    /// the start of its `init` before the object exists; then its base's
    /// constructor (which builds the object), or, in the root class, the
    /// object itself, with the fields of its own class first and then
    /// `subclass_fields` from the closest subclass down; and then the rest
    /// of `init` with `self`. So `init`s finish from the root down.
    fn build(
        &self,
        constructor: ConstructorId,
        mut frame: Vec<Value>,
        class: ClassRef,
        subclass_fields: &mut Vec<Vec<Value>>,
    ) -> Result<Rc<Object>, RunError> {
        let constructor = self.constructor(constructor)?;
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
        let object = if let Some(base) = &constructor.base {
            let arguments = self.evaluate_all(&base.arguments, &frame)?;
            subclass_fields.push(fields);
            self.build(base.constructor, arguments, class, subclass_fields)?
        } else {
            let mut row = fields;
            for fields in subclass_fields.drain(..).rev() {
                row.extend(fields);
            }
            Rc::new(Object::new(class, row))
        };
        store(
            &mut frame,
            constructor.self_slot,
            Value::Object(Rc::clone(&object)),
        )?;
        // A `return` in `init` only ends it: the object is built anyway.
        self.block(&constructor.rest, &mut frame)?;
        Ok(object)
    }

    fn constructor(&self, constructor: ConstructorId) -> Result<&Constructor, RunError> {
        self.code
            .constructors
            .get(constructor.0)
            .ok_or(RunError::Internal("unknown constructor"))
    }

    /// The field at `position` of an object; with `?.`, `null` for no
    /// object.
    fn field(
        &self,
        object: &Expression,
        position: usize,
        null_safe: bool,
        frame: &Frame,
    ) -> Result<Value, RunError> {
        match self.evaluate(object, frame)? {
            Value::Null if null_safe => Ok(Value::Null),
            object => object_of(&object)?.field(position),
        }
    }

    /// Whether `value` is an object of `class` or of a subclass.
    fn is_instance(&self, value: &Value, class: ClassRef) -> bool {
        let Value::Object(object) = value else {
            return false;
        };
        self.code
            .classes
            .get(object.class.0)
            .is_some_and(|own| own.ancestors.contains(&class))
    }

    /// `Debug.print`: a line of the program's output.
    fn print(&self, message: &str) -> Result<(), RunError> {
        let mut output = self
            .output
            .try_borrow_mut()
            .map_err(|_| RunError::Internal("output already in use"))?;
        writeln!(output, "{message}").map_err(RunError::Output)
    }

    /// A method of the object `receiver`, with `self` as its first local.
    fn call_method(
        &self,
        target: MethodTarget,
        receiver: Value,
        arguments: &[Expression],
        frame: &Frame,
    ) -> Result<Value, RunError> {
        let function = match target {
            MethodTarget::Direct(function) => function,
            MethodTarget::Overridable(place) => {
                let class = object_of(&receiver)?.class;
                self.code
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
        &self,
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

    fn condition(&self, condition: &Expression, frame: &Frame) -> Result<bool, RunError> {
        match self.evaluate(condition, frame)? {
            Value::Bool(value) => Ok(value),
            _ => Err(RunError::Internal("condition that is not a `bool`")),
        }
    }

    fn evaluate(&self, expression: &Expression, frame: &Frame) -> Result<Value, RunError> {
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
            } => self.field(object, *position, *null_safe, frame)?,
            Expression::IsInstance { value, class } => {
                let value = self.evaluate(value, frame)?;
                Value::Bool(self.is_instance(&value, *class))
            }
            Expression::Downcast { value, class } => {
                let value = self.evaluate(value, frame)?;
                let is_instance = self.is_instance(&value, *class);
                if is_instance { value } else { Value::Null }
            }
            Expression::Now => Value::Float(star_script_runtime::tasks::now()),
            Expression::Print(message) => {
                let message = self.evaluate(message, frame)?.to_text()?;
                self.print(&message)?;
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
        &self,
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
        &self,
        left: &Expression,
        right: &Expression,
        frame: &Frame,
    ) -> Result<(Value, Value), RunError> {
        let left = self.evaluate(left, frame)?;
        Ok((left, self.evaluate(right, frame)?))
    }

    /// The values of `expressions`, in order.
    fn evaluate_all(
        &self,
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
