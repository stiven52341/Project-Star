//! Expressions, with the conversions between number types written out.

use std::sync::Arc;

use star_script::Span;
use star_script::ast::{BinaryOperator, Expr, ExprKind, Literal, Name, UnaryOperator};
use star_script::semantic::{
    BuiltinMethod, ClassId, ClassOrigin, Execution, Resolution, Type, is_debug_print,
};

use super::Member;
use super::body::Body;
use crate::error::LowerError;
use crate::tree::{
    self, Arithmetic, BodyMethod, BodyShape, ClassRef, Comparison, Coroutine, EngineCall,
    EngineMethod, Expression, MethodTarget, NodeMethod, NumberType, SoundMethod, TweenGoal,
};

/// The engine class with the game's clock.
const TIME_CLASS: &str = "Time";
/// The engine class that reads the game's input.
const INPUT_CLASS: &str = "Input";
/// The engine class of games.
const GAME_CLASS: &str = "Game";
/// The engine class of nodes.
const NODE_CLASS: &str = "Node";
/// The engine class of 3D nodes.
const NODE_3D_CLASS: &str = "Node3D";
/// The engine class that loads scenes and searches the game's tree.
const SCENE_CLASS: &str = "Scene";
/// The engine class of sprites.
const SPRITE_CLASS: &str = "Sprite";
/// The engine class of the nodes that play keyframe animations.
const ANIMATOR_CLASS: &str = "Animator";
/// The engine class of physics bodies.
const BODY_CLASS: &str = "Body";
/// The engine class of sounds.
const SOUND_CLASS: &str = "Sound";
/// The engine class with the volume of every sound.
const AUDIO_CLASS: &str = "Audio";
/// The engine class that prints and shows the frames per second.
const DEBUG_CLASS: &str = "Debug";

/// How a method is called on its object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Through {
    /// On an object (`self` included), with `?.` or not.
    Object { null_safe: bool },
    /// `super.method(…)`: the base's version, on `self`.
    Super,
}

impl Body<'_> {
    /// The interpreter's version of `expression`.
    pub(super) fn expression(&self, expression: &Expr) -> Result<Expression, LowerError> {
        let span = expression.span;
        match &expression.kind {
            ExprKind::Literal(literal) => Ok(Expression::Constant(literal_value(
                literal,
                self.type_of(expression)?,
                expression,
            )?)),
            ExprKind::Name(_) => self.name(expression),
            ExprKind::Unary { operator, operand } => {
                let operand = Box::new(self.expression(operand)?);
                Ok(match operator {
                    UnaryOperator::Negate => Expression::Negate(operand),
                    UnaryOperator::Not => Expression::Not(operand),
                })
            }
            ExprKind::Binary {
                operator,
                left,
                right,
            } => self.binary(expression, *operator, left, right),
            ExprKind::Cast { value, .. } => self.cast(expression, value),
            ExprKind::Call { callee, arguments } => self.call(callee, arguments),
            ExprKind::Member {
                object,
                name,
                null_safe,
            } => self.member(expression, object, name, *null_safe),
            ExprKind::Index { object, index } => Ok(Expression::Index {
                list: Box::new(self.expression(object)?),
                index: Box::new(self.expression(index)?),
            }),
            ExprKind::Array(elements) => self.array(expression, elements),
            ExprKind::SelfValue => self.self_value(expression),
            ExprKind::Super => Err(LowerError::internal("`super` used as a value", span)),
            ExprKind::TypeTest { value, type_ref } => Ok(Expression::IsInstance {
                value: Box::new(self.expression(value)?),
                class: self.class_named(&type_ref.to_string(), expression)?,
            }),
            ExprKind::Await(value) => match self.expression(value)? {
                Expression::Start(coroutine) => Ok(Expression::Await(coroutine)),
                _ => Err(LowerError::internal(
                    "`await` of a call that is not `async`",
                    span,
                )),
            },
        }
    }

    /// `expression` converted to `ty`, where analysis allows that.
    pub(super) fn value_as(&self, expression: &Expr, ty: &Type) -> Result<Expression, LowerError> {
        let value = self.expression(expression)?;
        coerce(value, self.type_of(expression)?, ty, expression)
    }

    /// Both sides of an operator, numbers converted to the type they meet
    /// in (`small < count`: `small` as an `int`).
    pub(super) fn common_operands(
        &self,
        left: &Expr,
        right: &Expr,
    ) -> Result<(Expression, Expression), LowerError> {
        let left_type = self.type_of(left)?;
        match left_type.common_number(self.type_of(right)?) {
            Some(common) => Ok((
                self.value_as(left, &common)?,
                self.value_as(right, &common)?,
            )),
            None => Ok((self.expression(left)?, self.expression(right)?)),
        }
    }

    /// What analysis says `expression` refers to.
    pub(super) fn resolution(&self, expression: &Expr) -> Result<Option<Resolution>, LowerError> {
        self.analysis
            .expression(expression)
            .map(|info| info.resolution)
            .ok_or_else(|| LowerError::internal("expression without analysis", expression.span))
    }

    /// The type analysis gave `expression`.
    pub(super) fn type_of(&self, expression: &Expr) -> Result<&Type, LowerError> {
        self.analysis
            .type_of(expression)
            .ok_or_else(|| LowerError::internal("expression without a type", expression.span))
    }

    fn name(&self, expression: &Expr) -> Result<Expression, LowerError> {
        match self.resolution(expression)? {
            // Read where a check proved more about it (`!= null`, `is`),
            // it is still the same value: values carry their class.
            Some(Resolution::Local(local)) => Ok(Expression::Local(self.slot(local, expression)?)),
            Some(Resolution::Constant { class, index }) => self.constant(class, index, expression),
            Some(Resolution::Field { class, index }) => Ok(Expression::Field {
                object: Box::new(self.self_value(expression)?),
                field: self.field_ref(class, index, expression)?,
                null_safe: false,
            }),
            _ => Err(LowerError::internal(
                "name that is not a value",
                expression.span,
            )),
        }
    }

    /// `object.name`: a field, or a constant through its class.
    fn member(
        &self,
        expression: &Expr,
        object: &Expr,
        name: &Name,
        null_safe: bool,
    ) -> Result<Expression, LowerError> {
        match self.resolution(expression)? {
            Some(Resolution::Constant { class, index }) => {
                let constant = self.constant(class, index, expression)?;
                if matches!(self.resolution(object)?, Some(Resolution::Class(_))) {
                    return Ok(constant);
                }
                // Through an object: it is still evaluated, for its effects.
                Ok(Expression::Then(
                    Box::new(self.expression(object)?),
                    Box::new(constant),
                ))
            }
            Some(Resolution::Field { class, index }) => Ok(Expression::Field {
                object: Box::new(self.expression(object)?),
                field: self.field_ref(class, index, expression)?,
                null_safe,
            }),
            _ => Err(LowerError::internal(
                "member that is not a value",
                name.span,
            )),
        }
    }

    /// The value of a constant: a call of the function that computes it.
    fn constant(
        &self,
        class: ClassId,
        index: usize,
        expression: &Expr,
    ) -> Result<Expression, LowerError> {
        let function = self
            .tables
            .functions
            .get(&Member::Constant(class, index))
            .copied()
            .ok_or_else(|| LowerError::internal("constant without a function", expression.span))?;
        Ok(Expression::Call {
            function,
            arguments: Vec::new(),
        })
    }

    fn binary(
        &self,
        expression: &Expr,
        operator: BinaryOperator,
        left: &Expr,
        right: &Expr,
    ) -> Result<Expression, LowerError> {
        let (left, right) = match operator {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder => {
                let ty = self.type_of(expression)?;
                (self.value_as(left, ty)?, self.value_as(right, ty)?)
            }
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                self.equality_operands(left, right)?
            }
            BinaryOperator::Coalesce => {
                // The value is already of the result's type, or the type
                // without `null`; the fallback is converted to it.
                let result = self.type_of(expression)?;
                (self.expression(left)?, self.value_as(right, result)?)
            }
            _ => self.common_operands(left, right)?,
        };
        operation(operator, left, right, expression)
    }

    /// Both sides of `==`, as the type of the side the other converts to.
    fn equality_operands(
        &self,
        left: &Expr,
        right: &Expr,
    ) -> Result<(Expression, Expression), LowerError> {
        let left_type = self.type_of(left)?;
        let right_type = self.type_of(right)?;
        let common = if self.analysis.classes().is_assignable(left_type, right_type) {
            right_type
        } else {
            left_type
        };
        Ok((self.value_as(left, common)?, self.value_as(right, common)?))
    }

    /// `value as Type`: between numbers, to a base class (nothing to do)
    /// or to a subclass (the object, or `null` if it is not one).
    fn cast(&self, expression: &Expr, value: &Expr) -> Result<Expression, LowerError> {
        let target = self.type_of(expression)?;
        let from = self.type_of(value)?;
        let lowered = self.expression(value)?;
        if from == target {
            return Ok(lowered);
        }
        if from.is_numeric() && target.is_numeric() {
            return convert(lowered, target, expression);
        }
        let (Some(from_class), Some(to_class)) = (from.class(), target.class()) else {
            return Err(LowerError::internal(
                "cast between unrelated types",
                expression.span,
            ));
        };
        if self.analysis.classes().is_subclass(from_class, to_class) {
            return Ok(lowered);
        }
        Ok(Expression::Downcast {
            value: Box::new(lowered),
            class: self.class_ref(to_class, expression)?,
        })
    }

    /// The class of the program called `name`, as `is` names it.
    fn class_named(&self, name: &str, expression: &Expr) -> Result<ClassRef, LowerError> {
        let class = self
            .analysis
            .classes()
            .find(name)
            .ok_or_else(|| LowerError::internal("unknown class", expression.span))?;
        self.class_ref(class, expression)
    }

    /// The position of `class`: a class of the program, or an engine
    /// class the interpreter knows.
    fn class_ref(&self, class: ClassId, expression: &Expr) -> Result<ClassRef, LowerError> {
        if let Some(position) = self.tables.classes.get(&class) {
            return Ok(*position);
        }
        if self.analysis.classes().class(class).origin == ClassOrigin::Native {
            return Err(LowerError::unsupported(
                "`is` and `as` with this engine class",
                expression.span,
            ));
        }
        Err(LowerError::internal(
            "class without a position",
            expression.span,
        ))
    }

    /// `[a, b, c]`, each element converted to the list's element type.
    fn array(&self, expression: &Expr, elements: &[Expr]) -> Result<Expression, LowerError> {
        let Type::List(element) = self.type_of(expression)? else {
            return Err(LowerError::internal(
                "list without a list type",
                expression.span,
            ));
        };
        let elements = elements
            .iter()
            .map(|item| self.value_as(item, element))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Expression::List(elements))
    }

    fn call(&self, callee: &Expr, arguments: &[Expr]) -> Result<Expression, LowerError> {
        match self.resolution(callee)? {
            Some(Resolution::Method { class, index }) => {
                let call = self.method_call(callee, class, index, arguments)?;
                let execution = self
                    .analysis
                    .classes()
                    .class(class)
                    .methods
                    .get(index)
                    .map(|method| method.execution);
                match execution {
                    Some(Execution::Async) => started(call, callee.span),
                    Some(Execution::Parallel) if !self.in_parallel => {
                        on_another_thread(call, callee.span)
                    }
                    _ => Ok(call),
                }
            }
            Some(Resolution::Builtin(method)) => self.builtin_call(callee, method, arguments),
            Some(Resolution::Constructor(class)) => self.construct(callee, class, arguments),
            _ => Err(LowerError::internal(
                "call of something that is not callable",
                callee.span,
            )),
        }
    }

    /// `Debug.print(message)`: its `string`, or its number, which running
    /// writes as the number's `to_string` does.
    fn print(&self, arguments: &[Expr], span: Span) -> Result<Expression, LowerError> {
        let [message] = arguments else {
            return Err(LowerError::internal("`print` without one message", span));
        };
        Ok(Expression::Print(Box::new(self.expression(message)?)))
    }

    fn method_call(
        &self,
        callee: &Expr,
        class: ClassId,
        index: usize,
        arguments: &[Expr],
    ) -> Result<Expression, LowerError> {
        let info = self.analysis.classes().class(class);
        let method = info
            .methods
            .get(index)
            .ok_or_else(|| LowerError::internal("unknown method", callee.span))?;
        if is_debug_print(&info.name, &method.name) {
            return self.print(arguments, callee.span);
        }
        let arguments = method
            .parameters
            .iter()
            .zip(arguments)
            .map(|(parameter, argument)| self.value_as(argument, &parameter.ty))
            .collect::<Result<Vec<_>, _>>()?;
        if info.origin == ClassOrigin::Native {
            return match (info.name.as_str(), method.name.as_str()) {
                (TIME_CLASS, "now") => Ok(Expression::Now),
                (TIME_CLASS, "delta") => Ok(Expression::Delta),
                (TIME_CLASS, "wait") => {
                    let seconds = arguments.into_iter().next().ok_or_else(|| {
                        LowerError::internal("`wait` without seconds", callee.span)
                    })?;
                    Ok(Expression::Start(Box::new(Coroutine::Wait(Box::new(
                        seconds,
                    )))))
                }
                (TIME_CLASS, "next_frame") => Ok(Expression::Start(Box::new(Coroutine::NextFrame))),
                (NODE_CLASS, name) if TweenGoal::named(name).is_some() => {
                    self.tween(callee, name, arguments)
                }
                (class, name) if method.is_static => {
                    let method = static_method(class, name).ok_or_else(|| {
                        LowerError::unsupported("this method of the engine", callee.span)
                    })?;
                    Ok(Expression::Engine {
                        method,
                        arguments,
                        call: EngineCall::Static,
                    })
                }
                (class, name) => self.engine_object_method(callee, class, name, arguments),
            };
        }
        let function = self
            .tables
            .functions
            .get(&Member::Method(class, index))
            .copied()
            .ok_or_else(|| LowerError::internal("method without a function", callee.span))?;
        if method.is_static {
            return Ok(Expression::Call {
                function,
                arguments,
            });
        }
        let (receiver, call) = self.receiver(callee)?;
        let null_safe = match call {
            Through::Super => {
                // The base's version, whatever the object overrides.
                return Ok(Expression::CallMethod {
                    target: MethodTarget::Direct(function),
                    receiver: Box::new(receiver),
                    arguments,
                    null_safe: false,
                });
            }
            Through::Object { null_safe } => null_safe,
        };
        // `private` methods are not overridden: they run their own code.
        let target = if method.is_private {
            MethodTarget::Direct(function)
        } else {
            let place = self
                .tables
                .overridable
                .get(&(class, index))
                .copied()
                .ok_or_else(|| LowerError::internal("method without a place", callee.span))?;
            MethodTarget::Overridable(place)
        };
        Ok(Expression::CallMethod {
            target,
            receiver: Box::new(receiver),
            arguments,
            null_safe,
        })
    }

    /// The object a method is called on, and how it is written: `self`
    /// for a bare name or `super`, else the object written.
    fn receiver(&self, callee: &Expr) -> Result<(Expression, Through), LowerError> {
        Ok(match &callee.kind {
            ExprKind::Name(_) => (
                self.self_value(callee)?,
                Through::Object { null_safe: false },
            ),
            ExprKind::Member { object, .. } if matches!(object.kind, ExprKind::Super) => {
                (self.self_value(callee)?, Through::Super)
            }
            ExprKind::Member {
                object, null_safe, ..
            } => (
                self.expression(object)?,
                Through::Object {
                    null_safe: *null_safe,
                },
            ),
            _ => {
                return Err(LowerError::internal(
                    "method called through an expression",
                    callee.span,
                ));
            }
        })
    }

    /// A tween method of `Node` (`name`), as a coroutine that starts there
    /// (or that `await` waits for).
    fn tween(
        &self,
        callee: &Expr,
        name: &str,
        arguments: Vec<Expression>,
    ) -> Result<Expression, LowerError> {
        let goal = TweenGoal::named(name)
            .ok_or_else(|| LowerError::internal("not a tween method", callee.span))?;
        let (receiver, through) = self.receiver(callee)?;
        let null_safe = match through {
            Through::Object { null_safe } => null_safe,
            Through::Super => false,
        };
        Ok(Expression::Start(Box::new(Coroutine::Tween {
            goal,
            receiver: Box::new(receiver),
            arguments,
            null_safe,
        })))
    }

    /// A method of an object of an engine class (`class`), with its
    /// receiver as the first argument: those of `Node`, `Game`, `Body`
    /// and `Sound`, and the engine's own `start` and `update`, which do
    /// nothing.
    fn engine_object_method(
        &self,
        callee: &Expr,
        class: &str,
        name: &str,
        arguments: Vec<Expression>,
    ) -> Result<Expression, LowerError> {
        let unsupported = || LowerError::unsupported("this method of the engine", callee.span);
        let method = object_method(class, name).ok_or_else(unsupported)?;
        let (receiver, through) = self.receiver(callee).map_err(|_| unsupported())?;
        let call = match through {
            Through::Object { null_safe } => EngineCall::Object { null_safe },
            Through::Super => EngineCall::Base,
        };
        Ok(Expression::Engine {
            method,
            arguments: std::iter::once(receiver).chain(arguments).collect(),
            call,
        })
    }

    /// `Class(arguments)`.
    fn construct(
        &self,
        callee: &Expr,
        class: ClassId,
        arguments: &[Expr],
    ) -> Result<Expression, LowerError> {
        let info = self.analysis.classes().class(class);
        let constructor = self.tables.constructor(class).ok_or_else(|| {
            if info.origin == ClassOrigin::Native {
                LowerError::unsupported("objects of this engine class", callee.span)
            } else {
                LowerError::internal("class without a constructor", callee.span)
            }
        })?;
        let arguments = info
            .constructor_parameters()
            .iter()
            .zip(arguments)
            .map(|(parameter, argument)| self.value_as(argument, &parameter.ty))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Expression::Construct {
            constructor,
            arguments,
        })
    }

    /// A method of a basic type or list, its argument converted to the
    /// type the method takes.
    fn builtin_call(
        &self,
        callee: &Expr,
        method: BuiltinMethod,
        arguments: &[Expr],
    ) -> Result<Expression, LowerError> {
        let ExprKind::Member {
            object, null_safe, ..
        } = &callee.kind
        else {
            return Err(LowerError::internal(
                "builtin method without a receiver",
                callee.span,
            ));
        };
        let receiver_type = self.type_of(object)?.non_null();
        let argument = match (arguments.first(), builtin_parameter(method, &receiver_type)) {
            (Some(argument), Some(parameter)) => {
                Some(Box::new(self.value_as(argument, &parameter)?))
            }
            _ => None,
        };
        Ok(Expression::Builtin {
            method,
            receiver: Box::new(self.expression(object)?),
            argument,
            null_safe: *null_safe,
        })
    }
}

/// The static method `name` of the engine class `class` (`Input`,
/// `Scene`, `Body`, `Sound` or `Audio`).
pub(super) fn static_method(class: &str, name: &str) -> Option<EngineMethod> {
    Some(match (class, name) {
        (INPUT_CLASS, "is_down") => EngineMethod::IsDown,
        (INPUT_CLASS, "just_pressed") => EngineMethod::JustPressed,
        (INPUT_CLASS, "just_released") => EngineMethod::JustReleased,
        (INPUT_CLASS, "strength") => EngineMethod::Strength,
        (INPUT_CLASS, "axis") => EngineMethod::Axis,
        (INPUT_CLASS, "pointer_x") => EngineMethod::PointerX,
        (INPUT_CLASS, "pointer_y") => EngineMethod::PointerY,
        (INPUT_CLASS, "has_pointer") => EngineMethod::HasPointer,
        (SCENE_CLASS, "load") => EngineMethod::LoadScene,
        (SCENE_CLASS, "find_by_name") => EngineMethod::FindInGame,
        (SCENE_CLASS, "find_by_tag") => EngineMethod::FindTaggedInGame,
        (BODY_CLASS, "circle") => EngineMethod::NewBody(BodyShape::Circle),
        (BODY_CLASS, "rectangle") => EngineMethod::NewBody(BodyShape::Rectangle),
        (BODY_CLASS, "capsule") => EngineMethod::NewBody(BodyShape::Capsule),
        (BODY_CLASS, "sphere") => EngineMethod::NewBody(BodyShape::Sphere),
        (BODY_CLASS, "box") => EngineMethod::NewBody(BodyShape::Box),
        (BODY_CLASS, "mesh") => EngineMethod::NewBody(BodyShape::Mesh),
        (SOUND_CLASS, "load") => EngineMethod::LoadSound,
        (AUDIO_CLASS, "volume") => EngineMethod::Volume,
        (AUDIO_CLASS, "set_volume") => EngineMethod::SetVolume,
        (DEBUG_CLASS, "show_fps") => EngineMethod::ShowFps,
        _ => return None,
    })
}

/// The method `name` that the engine class `class` declares for its
/// objects, if the interpreter runs it.
pub(super) fn object_method(class: &str, name: &str) -> Option<EngineMethod> {
    Some(match (class, name) {
        (GAME_CLASS, "quit") => EngineMethod::Quit,
        (GAME_CLASS, "add_child") => EngineMethod::AddChild,
        (GAME_CLASS | NODE_CLASS, "start") => EngineMethod::Start,
        (GAME_CLASS | NODE_CLASS, "update") => EngineMethod::Update,
        (NODE_CLASS | NODE_3D_CLASS, name) => EngineMethod::Node(NodeMethod::named(name)?),
        (SPRITE_CLASS, name) => EngineMethod::Node(NodeMethod::of_sprite(name)?),
        (ANIMATOR_CLASS, name) => EngineMethod::Node(NodeMethod::of_animator(name)?),
        (BODY_CLASS, "apply_impulse") => EngineMethod::Body(BodyMethod::ApplyImpulse),
        (BODY_CLASS, "apply_impulse_3d") => EngineMethod::Body(BodyMethod::ApplyImpulse3D),
        (BODY_CLASS, "touching") => EngineMethod::Body(BodyMethod::Touching),
        (BODY_CLASS, "just_touched") => EngineMethod::Body(BodyMethod::JustTouched),
        (BODY_CLASS, "just_separated") => EngineMethod::Body(BodyMethod::JustSeparated),
        (SOUND_CLASS, "play") => EngineMethod::Sound(SoundMethod::Play),
        (SOUND_CLASS, "stop") => EngineMethod::Sound(SoundMethod::Stop),
        (SOUND_CLASS, "is_playing") => EngineMethod::Sound(SoundMethod::IsPlaying),
        _ => return None,
    })
}

/// A call of a `parallel` function from a coroutine, as work that runs on
/// another thread (and that `await` waits for).
fn on_another_thread(call: Expression, span: Span) -> Result<Expression, LowerError> {
    let Expression::Call {
        function,
        arguments,
    } = call
    else {
        return Err(LowerError::internal("`parallel` call of no function", span));
    };
    Ok(Expression::Start(Box::new(Coroutine::Parallel {
        function,
        arguments,
    })))
}

/// A call of an `async` method, as a coroutine that starts there (or that
/// `await` waits for).
fn started(call: Expression, span: Span) -> Result<Expression, LowerError> {
    let coroutine = match call {
        Expression::Call {
            function,
            arguments,
        } => Coroutine::Call {
            function,
            arguments,
        },
        Expression::CallMethod {
            target,
            receiver,
            arguments,
            null_safe,
        } => Coroutine::Method {
            target,
            receiver,
            arguments,
            null_safe,
        },
        Expression::Start(coroutine) => return Ok(Expression::Start(coroutine)),
        _ => return Err(LowerError::internal("`async` call of no method", span)),
    };
    Ok(Expression::Start(Box::new(coroutine)))
}

/// `left operator right`, both already of the operation's type.
fn operation(
    operator: BinaryOperator,
    left: Expression,
    right: Expression,
    expression: &Expr,
) -> Result<Expression, LowerError> {
    let (left, right) = (Box::new(left), Box::new(right));
    Ok(match operator {
        BinaryOperator::Add => arithmetic(Arithmetic::Add, left, right),
        BinaryOperator::Subtract => arithmetic(Arithmetic::Subtract, left, right),
        BinaryOperator::Multiply => arithmetic(Arithmetic::Multiply, left, right),
        BinaryOperator::Divide => arithmetic(Arithmetic::Divide, left, right),
        BinaryOperator::Remainder => arithmetic(Arithmetic::Remainder, left, right),
        BinaryOperator::Less => compare(Comparison::Less, left, right),
        BinaryOperator::LessEqual => compare(Comparison::LessEqual, left, right),
        BinaryOperator::Greater => compare(Comparison::Greater, left, right),
        BinaryOperator::GreaterEqual => compare(Comparison::GreaterEqual, left, right),
        BinaryOperator::Equal => Expression::Equal(left, right),
        BinaryOperator::NotEqual => Expression::Not(Box::new(Expression::Equal(left, right))),
        BinaryOperator::And => Expression::And(left, right),
        BinaryOperator::Or => Expression::Or(left, right),
        BinaryOperator::Coalesce => Expression::Coalesce(left, right),
        BinaryOperator::Range => {
            return Err(LowerError::internal("range outside `for`", expression.span));
        }
    })
}

fn arithmetic(operator: Arithmetic, left: Box<Expression>, right: Box<Expression>) -> Expression {
    Expression::Arithmetic {
        operator,
        left,
        right,
    }
}

fn compare(operator: Comparison, left: Box<Expression>, right: Box<Expression>) -> Expression {
    Expression::Compare {
        operator,
        left,
        right,
    }
}

/// `value`, of type `from`, as a value of type `to`. Analysis already
/// checked that it can be; only conversions between number types change
/// the value, since a nullable value is the value itself or `null`.
fn coerce(
    value: Expression,
    from: &Type,
    to: &Type,
    expression: &Expr,
) -> Result<Expression, LowerError> {
    match (from, to) {
        _ if from == to => Ok(value),
        (Type::Null, Type::Nullable(_)) => Ok(value),
        (Type::Nullable(from), Type::Nullable(to)) => coerce(value, from, to, expression),
        (from, Type::Nullable(to)) => coerce(value, from, to, expression),
        (from, to) if from.widens_to(to) => convert(value, to, expression),
        // An object of a subclass used as one of its base: objects carry
        // their class, so nothing changes.
        (Type::Class(_), Type::Class(_)) => Ok(value),
        _ => Err(LowerError::internal(
            "conversion between unrelated types",
            expression.span,
        )),
    }
}

/// `value` converted to the number type `ty`.
fn convert(value: Expression, ty: &Type, expression: &Expr) -> Result<Expression, LowerError> {
    let to = number_type(ty).ok_or_else(|| {
        LowerError::internal("conversion to a type that is not a number", expression.span)
    })?;
    Ok(Expression::Convert {
        value: Box::new(value),
        to,
    })
}

/// The number type of `ty`, if it is one.
const fn number_type(ty: &Type) -> Option<NumberType> {
    match ty {
        Type::Short => Some(NumberType::Short),
        Type::Int => Some(NumberType::Int),
        Type::BigInt => Some(NumberType::BigInt),
        Type::Float => Some(NumberType::Float),
        Type::Double => Some(NumberType::Double),
        _ => None,
    }
}

/// The type of the argument of a builtin method, if it takes one.
fn builtin_parameter(method: BuiltinMethod, receiver: &Type) -> Option<Type> {
    match (method, receiver) {
        (BuiltinMethod::Min | BuiltinMethod::Max, _) => Some(receiver.clone()),
        (BuiltinMethod::Contains | BuiltinMethod::Push, Type::List(element)) => {
            Some((**element).clone())
        }
        (BuiltinMethod::Contains, Type::String) => Some(Type::String),
        _ => None,
    }
}

/// The value of a literal of type `ty`. Integers written where a decimal is
/// needed become one; integers of a smaller type keep their lowest bits,
/// so `-32768` negates `32768` as a `short` to the same value the compiled
/// game writes whole.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn literal_value(
    literal: &Literal,
    ty: &Type,
    expression: &Expr,
) -> Result<tree::Literal, LowerError> {
    Ok(match (literal, ty.non_null()) {
        (Literal::Integer(value), Type::Short) => tree::Literal::Short(*value as i16),
        (Literal::Integer(value), Type::Int) => tree::Literal::Int(*value as i32),
        (Literal::Integer(value), Type::BigInt) => tree::Literal::BigInt(*value),
        (Literal::Integer(value), Type::Float) => tree::Literal::Float(*value as f64 as f32),
        (Literal::Integer(value), Type::Double) => tree::Literal::Double(*value as f64),
        (Literal::Float(value), Type::Float) => tree::Literal::Float(*value as f32),
        (Literal::Float(value), Type::Double) => tree::Literal::Double(*value),
        (Literal::String(text), _) => tree::Literal::String(Arc::from(text.as_str())),
        (Literal::Bool(value), _) => tree::Literal::Bool(*value),
        (Literal::Null, _) => tree::Literal::Null,
        _ => {
            return Err(LowerError::internal(
                "literal of an unexpected type",
                expression.span,
            ));
        }
    })
}
