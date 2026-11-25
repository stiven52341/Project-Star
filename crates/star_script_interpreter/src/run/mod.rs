//! Running: walks the interpreter's tree with values.

mod builtins;
mod coroutines;
mod engine;
mod numbers;
mod parallel;
mod scenes;

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use star_script::semantic::BuiltinMethod;
use star_script_runtime::try_index;

use crate::error::RunError;
use crate::host::GameObject;
use crate::tree::{
    Arithmetic, Class, ClassRef, Code, Constructor, ConstructorId, ConstructorRef, EngineCall,
    EngineClass, EngineMethod, Entry, Expression, FieldRef, Function, FunctionId, MethodTarget,
    Statement,
};
use crate::value::{Elements, Object, Value, read, write};

use crate::{Output, ProgramKind};
pub(crate) use scenes::{create, set_exported};

/// What the engine calls on a node or the game.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Hook {
    /// `start`, once.
    Start,
    /// `update`, every frame, with the seconds since the last one.
    Update(f32),
}

/// What the usual runner says of code that only coroutines can run.
const WAIT_OUTSIDE_COROUTINE: &str = "`await` outside a coroutine";

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
/// coroutines it starts can keep a copy after the call that started them.
#[derive(Clone)]
pub(crate) struct Machine {
    code: Arc<Code>,
    output: Output,
    /// The first runtime error of a coroutine, shared by every copy: once
    /// there is one, the program stops and no coroutine goes on.
    failure: Rc<RefCell<Option<RunError>>>,
}

/// The locals of the function that runs.
type Frame = [Value];

impl Machine {
    pub(crate) fn new(code: Arc<Code>, output: Output) -> Self {
        Self {
            code,
            output,
            failure: Rc::default(),
        }
    }

    /// Runs the program: `main`, and then its coroutines until none is
    /// left, or until one fails.
    pub(crate) fn run(&self) -> Result<(), RunError> {
        let Entry::Main(main) = &self.code.entry else {
            return Err(RunError::WrongKind(ProgramKind::Game));
        };
        self.statement(main, &mut [])?;
        star_script_runtime::tasks::run_until_finished_or(|| self.has_failed());
        self.take_failure().map_or(Ok(()), Err)
    }

    /// Creates the game: the object of the class that extends `Game`.
    pub(crate) fn new_game(&self) -> Result<Rc<GameObject>, RunError> {
        let constructor = match &self.code.entry {
            Entry::Game { constructor, .. } => *constructor,
            Entry::Scene { path, .. } => return self.main_scene_game(path),
            Entry::Main(_) => return Err(RunError::WrongKind(ProgramKind::Console)),
        };
        match self.construct(ConstructorRef::Script(constructor), Vec::new())? {
            Value::Game(game) => Ok(game),
            _ => Err(RunError::Internal(
                "a game class whose objects are not games",
            )),
        }
    }

    /// The game of a program without a `Game` class: a plain `Game` with
    /// the main scene at `path`, as compiled.
    fn main_scene_game(&self, path: &str) -> Result<Rc<GameObject>, RunError> {
        let game = EngineClass::Game;
        let Value::Game(game) = engine::new_object(game, game.class(), Vec::new(), &[])? else {
            return Err(RunError::Internal("a `Game` that is not a game"));
        };
        let Value::Node(scene) = self.load_scene(path)? else {
            return Err(RunError::MainScene(path.to_owned()));
        };
        game.engine.add_child(scene);
        Ok(game)
    }

    /// Runs what `object` (a node or the game) does when the engine calls
    /// `hook`: the override a script wrote, if any.
    pub(crate) fn run_hook(&self, object: Value, hook: Hook) -> Result<(), RunError> {
        let (method, arguments) = match hook {
            Hook::Start => (EngineMethod::Start, vec![object]),
            Hook::Update(delta) => (EngineMethod::Update, vec![object, Value::Float(delta)]),
        };
        self.engine(method, EngineCall::Object { null_safe: false }, arguments)
            .map(drop)
    }

    /// Runs a method of the engine with `arguments` (the object first, for
    /// a method of objects): the script's version if it is called through
    /// an object whose class overrides it, else the engine's.
    fn engine(
        &self,
        method: EngineMethod,
        call: EngineCall,
        arguments: Vec<Value>,
    ) -> Result<Value, RunError> {
        if let (EngineMethod::LoadScene, [Value::String(path)]) = (method, arguments.as_slice()) {
            return self.load_scene(path);
        }
        if let EngineCall::Object { .. } = call
            && let Some(object) = arguments.first()
            && let Some(function) = self.class_of(object)?.override_of(method)
        {
            return self.call(function, arguments);
        }
        engine::call(method, &arguments)
    }

    /// What running needs to know about the class of `object`.
    fn class_of(&self, object: &Value) -> Result<&Class, RunError> {
        let class = object_of(object)?.class;
        self.code
            .classes
            .get(class.0)
            .ok_or(RunError::Internal("unknown class"))
    }

    /// Runs `function` with `arguments`, already of its parameters' types,
    /// and gives what it returns ([`Value::Null`] if nothing).
    pub(crate) fn call(
        &self,
        function: FunctionId,
        mut arguments: Vec<Value>,
    ) -> Result<Value, RunError> {
        let function = self.function(function)?;
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
                field,
                operator,
                value,
            } => {
                let object = self.evaluate(object, frame)?;
                let new_value = match operator {
                    None => self.evaluate(value, frame)?,
                    Some(operator) => {
                        let current = engine::read_field(&object, *field)?;
                        let value = self.evaluate(value, frame)?;
                        numbers::arithmetic(*operator, &current, &value)?
                    }
                };
                engine::write_field(&object, *field, new_value)?;
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
            Statement::Waits(_) => return Err(RunError::Internal(WAIT_OUTSIDE_COROUTINE)),
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
        constructor: ConstructorRef,
        arguments: Vec<Value>,
    ) -> Result<Value, RunError> {
        match constructor {
            ConstructorRef::Script(constructor) => {
                let class = self.constructor(constructor)?.class;
                self.build(constructor, arguments, class, &mut Vec::new())
            }
            ConstructorRef::Engine(engine) => {
                engine::new_object(engine, engine.class(), Vec::new(), &arguments)
            }
        }
    }

    /// Runs a constructor for an object of `class`: its fields' values and
    /// the start of its `init` before the object exists; then its base's
    /// constructor (which builds the object), or, in the root class, the
    /// object itself, with the fields of its own class first and then
    /// `subclass_fields` from the closest subclass down (and the runtime's
    /// fields, if the root extends an engine class); and then the rest of
    /// `init` with `self`. So `init`s finish from the root down.
    fn build(
        &self,
        constructor: ConstructorId,
        mut frame: Vec<Value>,
        class: ClassRef,
        subclass_fields: &mut Vec<Vec<Value>>,
    ) -> Result<Value, RunError> {
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
        let base = match &constructor.base {
            Some(base) => Some((
                base.constructor,
                self.evaluate_all(&base.arguments, &frame)?,
            )),
            None => None,
        };
        let object = if let Some((ConstructorRef::Script(base), arguments)) = base {
            subclass_fields.push(fields);
            self.build(base, arguments, class, subclass_fields)?
        } else {
            let mut row = fields;
            for fields in subclass_fields.drain(..).rev() {
                row.extend(fields);
            }
            match base {
                Some((ConstructorRef::Engine(engine), arguments)) => {
                    engine::new_object(engine, class, row, &arguments)?
                }
                _ => Value::Object(Rc::new(Object::new(class, row, ()))),
            }
        };
        store(&mut frame, constructor.self_slot, object.clone())?;
        // A `return` in `init` only ends it: the object is built anyway.
        self.block(&constructor.rest, &mut frame)?;
        Ok(object)
    }

    fn function(&self, function: FunctionId) -> Result<&Function, RunError> {
        self.code
            .functions
            .get(function.0)
            .ok_or(RunError::Internal("unknown function"))
    }

    fn constructor(&self, constructor: ConstructorId) -> Result<&Constructor, RunError> {
        self.code
            .constructors
            .get(constructor.0)
            .ok_or(RunError::Internal("unknown constructor"))
    }

    /// A field of an object; with `?.`, `null` for no object.
    fn field(
        &self,
        object: &Expression,
        field: FieldRef,
        null_safe: bool,
        frame: &Frame,
    ) -> Result<Value, RunError> {
        match self.evaluate(object, frame)? {
            Value::Null if null_safe => Ok(Value::Null),
            object => engine::read_field(&object, field),
        }
    }

    /// Whether `value` is an object of `class` or of a subclass.
    fn is_instance(&self, value: &Value, class: ClassRef) -> bool {
        let Some(object) = value.object() else {
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

    /// `value as Subclass`: the object if it is one, else `null`.
    fn downcast(&self, value: Value, class: ClassRef) -> Value {
        if self.is_instance(&value, class) {
            value
        } else {
            Value::Null
        }
    }

    /// A method of the object `receiver`, with `self` as its first local;
    /// with `?.`, `null` for no object, without running the arguments.
    fn call_method(
        &self,
        target: MethodTarget,
        receiver: &Expression,
        arguments: &[Expression],
        null_safe: bool,
        frame: &Frame,
    ) -> Result<Value, RunError> {
        let receiver = self.evaluate(receiver, frame)?;
        if null_safe && matches!(receiver, Value::Null) {
            return Ok(Value::Null);
        }
        let function = self.method_function(target, &receiver)?;
        let mut values = Vec::with_capacity(arguments.len() + 1);
        values.push(receiver);
        for argument in arguments {
            values.push(self.evaluate(argument, frame)?);
        }
        self.call(function, values)
    }

    /// The function a method of `receiver` runs: its own version of it, if
    /// it can be overridden.
    fn method_function(
        &self,
        target: MethodTarget,
        receiver: &Value,
    ) -> Result<FunctionId, RunError> {
        match target {
            MethodTarget::Direct(function) => Ok(function),
            MethodTarget::Overridable(place) => {
                let class = object_of(receiver)?.class;
                self.code
                    .classes
                    .get(class.0)
                    .and_then(|class| class.methods.get(place))
                    .copied()
                    .ok_or(RunError::Internal("method missing from its class's table"))
            }
        }
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
            Expression::Constant(literal) => literal.value(),
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
            } => self.call_method(*target, receiver, arguments, *null_safe, frame)?,
            Expression::Construct {
                constructor,
                arguments,
            } => self.construct(*constructor, self.evaluate_all(arguments, frame)?)?,
            Expression::Field {
                object,
                field,
                null_safe,
            } => self.field(object, *field, *null_safe, frame)?,
            Expression::IsInstance { value, class } => {
                Value::Bool(self.is_instance(&self.evaluate(value, frame)?, *class))
            }
            Expression::Downcast { value, class } => {
                self.downcast(self.evaluate(value, frame)?, *class)
            }
            Expression::Then(first, value) => self.evaluate_pair(first, value, frame)?.1,
            Expression::Now => Value::Float(star_script_runtime::tasks::now()),
            Expression::Print(message) => {
                self.print(&self.evaluate(message, frame)?.to_text()?)?;
                Value::Null
            }
            Expression::Engine {
                method,
                arguments,
                call,
            } => self.engine_call(*method, arguments, *call, frame)?,
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
            Expression::Start(coroutine) => {
                self.start_call(coroutine, frame)?;
                Value::Null
            }
            Expression::Await(_) | Expression::Waits(_) => {
                return Err(RunError::Internal(WAIT_OUTSIDE_COROUTINE));
            }
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

    /// A call of a method of the engine; `null` for a `?.` call on `null`.
    fn engine_call(
        &self,
        method: EngineMethod,
        arguments: &[Expression],
        call: EngineCall,
        frame: &Frame,
    ) -> Result<Value, RunError> {
        match self.engine_arguments(arguments, call, frame)? {
            Some(arguments) => self.engine(method, call, arguments),
            None => Ok(Value::Null),
        }
    }

    /// The arguments of a method of the engine, in order; `None` for a
    /// `?.` call on `null`, whose other arguments do not run.
    fn engine_arguments(
        &self,
        arguments: &[Expression],
        call: EngineCall,
        frame: &Frame,
    ) -> Result<Option<Vec<Value>>, RunError> {
        if let EngineCall::Object { null_safe: true } = call
            && let Some((object, rest)) = arguments.split_first()
        {
            let object = self.evaluate(object, frame)?;
            if matches!(object, Value::Null) {
                return Ok(None);
            }
            let mut values = Vec::with_capacity(arguments.len());
            values.push(object);
            values.extend(self.evaluate_all(rest, frame)?);
            return Ok(Some(values));
        }
        self.evaluate_all(arguments, frame).map(Some)
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
fn object_of(value: &Value) -> Result<&Object<dyn Any>, RunError> {
    value.object().ok_or(RunError::Internal(
        "member of something that is not an object",
    ))
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
