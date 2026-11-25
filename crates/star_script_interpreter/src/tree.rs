//! The interpreter's own tree of a program: what lowering builds and
//! running walks. Names are resolved, and conversions are written out, so
//! the two operands of an operation always hold values of the same type.

use std::collections::HashMap;
use std::sync::Arc;

use star_script::semantic::BuiltinMethod;

use crate::value::Value;

/// A whole lowered program: what a [`crate::Program`] shares with the
/// coroutines it starts and the threads of its `parallel` functions, so
/// it holds no value that belongs to one thread (see [`Literal`]).
#[derive(Debug)]
pub(crate) struct Code {
    pub(crate) functions: Vec<Function>,
    pub(crate) constructors: Vec<Constructor>,
    pub(crate) classes: Vec<Class>,
    /// What starts the program.
    pub(crate) entry: Entry,
    /// What a scene can build its nodes as, by the class name it writes:
    /// every class of the program but the engine's node classes, which
    /// the runtime names itself.
    pub(crate) scene_classes: HashMap<String, SceneClass>,
}

/// A class, as the nodes of a scene see it.
#[derive(Debug)]
pub(crate) struct SceneClass {
    /// What creates its objects for a scene, with `init()`; or why a
    /// scene cannot (the analysis's reason, as compiled).
    pub(crate) creation: Result<ConstructorId, String>,
    /// Its `export` fields and its bases', which a scene can set.
    pub(crate) exported: Vec<ExportedField>,
}

/// An `export` field, which a scene sets by its name.
#[derive(Debug)]
pub(crate) struct ExportedField {
    pub(crate) name: String,
    /// Its place in the row of script fields.
    pub(crate) position: usize,
    pub(crate) kind: ExportKind,
}

/// The types an `export` field can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExportKind {
    Short,
    Int,
    BigInt,
    Float,
    Double,
    Bool,
    String,
    Color,
}

/// What starts a program.
#[derive(Debug)]
pub(crate) enum Entry {
    /// A console program: a call of `static func main()`, or its start as
    /// a coroutine if it is `async`.
    Main(Statement),
    /// A game: the constructor of the class that extends `Game`, whose
    /// object the engine plays.
    Game(ConstructorId),
    /// A game without a `Game` class: a plain `Game` with the project's
    /// main scene, at this path, as compiled.
    Scene(Arc<str>),
}

/// Position of a function in the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FunctionId(pub(crate) usize);

/// Position of a class of the program, which its objects carry: the
/// engine classes the interpreter knows first, in the order of
/// [`EngineClass`], and then the program's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClassRef(pub(crate) usize);

/// The engine classes whose objects the interpreter can make, with their
/// fixed places among the classes: the runtime makes some of their objects
/// (a game's colors) without knowing the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EngineClass {
    Color,
    Node,
    Camera2D,
    Game,
    Sprite,
    Light2D,
    Body,
    Sound,
    Node3D,
    Mesh,
    Light,
    Camera3D,
}

impl EngineClass {
    /// Every engine class the interpreter knows, in the order of their
    /// places.
    pub(crate) const ALL: [Self; 12] = [
        Self::Color,
        Self::Node,
        Self::Camera2D,
        Self::Game,
        Self::Sprite,
        Self::Light2D,
        Self::Body,
        Self::Sound,
        Self::Node3D,
        Self::Mesh,
        Self::Light,
        Self::Camera3D,
    ];

    /// Its place among the classes.
    pub(crate) const fn class(self) -> ClassRef {
        ClassRef(self as usize)
    }

    /// The name StarScript gives it.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Color => "Color",
            Self::Node => "Node",
            Self::Camera2D => "Camera2D",
            Self::Game => "Game",
            Self::Sprite => "Sprite",
            Self::Light2D => "Light2D",
            Self::Body => "Body",
            Self::Sound => "Sound",
            Self::Node3D => "Node3D",
            Self::Mesh => "Mesh",
            Self::Light => "Light",
            Self::Camera3D => "Camera3D",
        }
    }
}

/// Position of a constructor of a class of the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConstructorId(pub(crate) usize);

/// What running needs to know about a class (an engine class's table is
/// empty: its methods are not functions of the program).
#[derive(Debug)]
pub(crate) struct Class {
    /// The function of each method that objects run their own version
    /// of, by its place in the table.
    pub(crate) methods: Vec<FunctionId>,
    /// The class itself and its bases: what `is` accepts.
    pub(crate) ancestors: Vec<ClassRef>,
    /// The methods of the engine its objects run a script's version of
    /// (written in the class or a base), such as the `start` the engine
    /// calls once. Overrides of the engine's methods are never `async`.
    pub(crate) overrides: Vec<(EngineMethod, FunctionId)>,
}

impl Class {
    /// The script's version of the engine's `method`, if one was written.
    pub(crate) fn override_of(&self, method: EngineMethod) -> Option<FunctionId> {
        self.overrides
            .iter()
            .find(|(overridden, _)| *overridden == method)
            .map(|(_, function)| *function)
    }
}

/// How objects of a class are built, in the two phases of the language.
#[derive(Debug)]
pub(crate) struct Constructor {
    /// The class of the objects it builds.
    pub(crate) class: ClassRef,
    /// How many locals it has: the parameters of `init` first.
    pub(crate) slots: usize,
    /// Before the object exists: the fields' values and the start of
    /// `init`, which put each field's value in its local.
    pub(crate) setup: Vec<Statement>,
    /// The local that holds the value of each field the class declares
    /// when the object is built, in order.
    pub(crate) fields: Vec<usize>,
    /// The constructor of the base, with its arguments (evaluated in this
    /// constructor's locals): it builds the object.
    pub(crate) base: Option<BaseConstructor>,
    /// The local that holds the object once it exists: `self`.
    pub(crate) self_slot: usize,
    /// The rest of `init`, with `self`.
    pub(crate) rest: Vec<Statement>,
}

/// The call of a base's constructor: `super.init(…)`, written or not.
#[derive(Debug)]
pub(crate) struct BaseConstructor {
    pub(crate) constructor: ConstructorRef,
    pub(crate) arguments: Vec<Expression>,
}

/// What builds an object.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ConstructorRef {
    /// The constructor of a class of the program.
    Script(ConstructorId),
    /// An engine class's: it gives the object the runtime's fields of the
    /// class, and builds it.
    Engine(EngineClass),
}

/// Where a field of an object is.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FieldRef {
    /// At this position of the object's row of script fields.
    Script(usize),
    /// Among the runtime's fields of its engine class.
    Engine(EngineField),
}

/// A field of an engine class, which the runtime keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EngineField {
    /// `r`, `g`, `b` and `a` of `Color`.
    Red,
    Green,
    Blue,
    Alpha,
    /// `name`, `x`, `y`, `rotation`, `scale_x` and `scale_y` of `Node`.
    Name,
    X,
    Y,
    Rotation,
    ScaleX,
    ScaleY,
    /// `tags` of `Node`: the interpreter's list, which the runtime reads.
    Tags,
    /// `body` of `Node`.
    Body,
    /// `view_height` and `main` of `Camera2D` (`main` of `Camera3D` too).
    ViewHeight,
    Main,
    /// `z`, `pitch`, `yaw` and `scale_z` of `Node3D`.
    Z,
    Pitch,
    Yaw,
    ScaleZ,
    /// `fov` of `Camera3D`.
    Fov,
    /// `metallic` and `roughness` of `Mesh` (its other fields are those of
    /// `Sprite` and `Body`).
    Metallic,
    Roughness,
    /// `texture`, `width`, `height`, `layer`, `pixelated` and `color` of
    /// `Sprite` (`color` of `Light2D` too, `width` and `height` of
    /// `Body`).
    Texture,
    Width,
    Height,
    Layer,
    Pixelated,
    Tint,
    /// `kind`, `intensity`, `range`, `angle` and `blend` of `Light2D`
    /// (`kind` of `Body` too).
    Kind,
    Intensity,
    Range,
    Angle,
    Blend,
    /// The fields of `Body` beyond `kind`, `width` and `height`.
    Shape,
    Radius,
    Depth,
    GravityScale,
    Friction,
    Bounciness,
    Density,
    VelocityX,
    VelocityY,
    VelocityZ,
    FixedRotation,
    Sensor,
    /// `volume` and `looping` of `Sound`.
    Volume,
    Looping,
    /// `background`, `camera`, `camera_3d`, `ambient` and `ambient_2d` of
    /// `Game`.
    Background,
    Camera,
    Camera3D,
    Ambient,
    Ambient2D,
}

/// A static method of the engine that reads the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EngineMethod {
    /// `Input.is_down(action)`.
    IsDown,
    /// `Input.just_pressed(action)`.
    JustPressed,
    /// `Input.just_released(action)`.
    JustReleased,
    /// `Input.strength(action)`.
    Strength,
    /// `Input.axis(negative, positive)`.
    Axis,
    /// `Input.pointer_x()`.
    PointerX,
    /// `Input.pointer_y()`.
    PointerY,
    /// `Input.has_pointer()`.
    HasPointer,
    /// `game.quit()`, with the game as its argument.
    Quit,
    /// `game.add_child(node)`, with the game and the node as arguments.
    AddChild,
    /// `start()` of a node or the game, with its receiver: the engine's
    /// does nothing.
    Start,
    /// `update(delta)` of a node or the game, with its receiver: the
    /// engine's does nothing.
    Update,
    /// A method of `Node`, with the node first.
    Node(NodeMethod),
    /// `Scene.load(path)`.
    LoadScene,
    /// `Scene.find_by_name(name)`, in the tree of the game being played.
    FindInGame,
    /// `Scene.find_by_tag(tag)`, in the tree of the game being played.
    FindTaggedInGame,
    /// `Body.circle`, `rectangle`, `capsule`, `sphere` or `box`.
    NewBody(BodyShape),
    /// A method of `Body`, with the body first.
    Body(BodyMethod),
    /// `Sound.load(path)`.
    LoadSound,
    /// A method of `Sound`, with the sound first.
    Sound(SoundMethod),
    /// `Audio.volume()`.
    Volume,
    /// `Audio.set_volume(volume)`.
    SetVolume,
}

/// The shape a static method of `Body` makes a body of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BodyShape {
    Circle,
    Rectangle,
    Capsule,
    Sphere,
    Box,
}

/// A method of `Body` that the runtime's fields of bodies run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BodyMethod {
    ApplyImpulse,
    ApplyImpulse3D,
    Touching,
    JustTouched,
    JustSeparated,
}

/// A method of `Sound` that the runtime's fields of sounds run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SoundMethod {
    Play,
    Stop,
    IsPlaying,
}

/// How a method of the engine is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EngineCall {
    /// A static method, such as `Input.is_down(action)`.
    Static,
    /// A method of an object (the first argument): an object whose class
    /// overrides it runs the script's version, as compiled. With `?.`, a
    /// `null` object gives `null` and the other arguments do not run.
    Object { null_safe: bool },
    /// `super.method(…)`: the engine's version, whatever `self` overrides.
    Base,
}

/// A method of `Node` (or `Node3D`) that the runtime's functions of nodes
/// run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeMethod {
    AddChild,
    Remove,
    Parent,
    Children,
    FindChild,
    HasTag,
    FindByName,
    FindByTag,
    GlobalX,
    GlobalY,
    GlobalRotation,
    SetGlobalPosition,
    SetGlobalRotation,
    Translate,
    TranslateGlobal,
    Rotate,
    LookAt,
    GlobalZ,
    SetGlobalPosition3D,
    Translate3D,
    TranslateGlobal3D,
}

impl NodeMethod {
    /// The method of `Node` called `name`, if the runtime runs it.
    pub(crate) fn named(name: &str) -> Option<Self> {
        Some(match name {
            "add_child" => Self::AddChild,
            "remove" => Self::Remove,
            "parent" => Self::Parent,
            "children" => Self::Children,
            "find_child" => Self::FindChild,
            "has_tag" => Self::HasTag,
            "find_by_name" => Self::FindByName,
            "find_by_tag" => Self::FindByTag,
            "global_x" => Self::GlobalX,
            "global_y" => Self::GlobalY,
            "global_rotation" => Self::GlobalRotation,
            "set_global_position" => Self::SetGlobalPosition,
            "set_global_rotation" => Self::SetGlobalRotation,
            "translate" => Self::Translate,
            "translate_global" => Self::TranslateGlobal,
            "rotate" => Self::Rotate,
            "look_at" => Self::LookAt,
            "global_z" => Self::GlobalZ,
            "set_global_position_3d" => Self::SetGlobalPosition3D,
            "translate_3d" => Self::Translate3D,
            "translate_global_3d" => Self::TranslateGlobal3D,
            _ => return None,
        })
    }
}

/// Which function a call of a method of an object runs.
#[derive(Debug, Clone, Copy)]
pub(crate) enum MethodTarget {
    /// The version of the object's class: the function at this place of
    /// its class's table.
    Overridable(usize),
    /// Always this function: `private` methods.
    Direct(FunctionId),
}

/// A static function, a method of objects (whose first local is `self`),
/// or the value of a constant.
#[derive(Debug)]
pub(crate) struct Function {
    /// How many locals it has: its parameters first, in order.
    pub(crate) slots: usize,
    /// Its statements.
    pub(crate) body: Vec<Statement>,
}

/// One step of a function.
#[derive(Debug)]
pub(crate) enum Statement {
    /// Puts a value in a local: a declaration or an assignment.
    Store { slot: usize, value: Expression },
    /// Evaluates an expression for its effects, such as a call.
    Evaluate(Expression),
    /// `list[index] = value`, or `list[index] op= value` with the
    /// operator: the list, the index, the element (for `op=`) and then the
    /// value are evaluated, in that order.
    SetElement {
        list: Expression,
        index: Expression,
        operator: Option<Arithmetic>,
        value: Expression,
    },
    /// `object.field = value`, or `object.field op= value` with the
    /// operator: the object, the field (for `op=`) and then the value are
    /// evaluated, in that order.
    SetField {
        object: Expression,
        field: FieldRef,
        operator: Option<Arithmetic>,
        value: Expression,
    },
    /// `if`; `else if` is an `if` alone in the else branch.
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Vec<Statement>,
    },
    /// `while`.
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    /// `for slot in start..end`, both ends integers of the same type,
    /// evaluated once before the first repetition.
    ForRange {
        slot: usize,
        start: Expression,
        end: Expression,
        body: Vec<Statement>,
    },
    /// `for slot in list`: goes through the elements the list has when
    /// the loop starts.
    ForEach {
        slot: usize,
        list: Expression,
        body: Vec<Statement>,
    },
    /// `return`, with its value already converted to the function's type.
    Return(Option<Expression>),
    /// `break`.
    Break,
    /// `continue`.
    Continue,
    /// A statement with an `await` inside, which only the runner of
    /// coroutines can run: it may stop there and go on in another frame.
    Waits(Box<Statement>),
}

/// A piece of code that produces a value.
#[derive(Debug)]
pub(crate) enum Expression {
    /// A value known before running: a literal.
    Constant(Literal),
    /// The value of a local.
    Local(usize),
    /// A number converted to another number type, as Rust's `as` does;
    /// `null` stays `null` (an `int?` read as a `bigint?`).
    Convert {
        value: Box<Expression>,
        to: NumberType,
    },
    /// `-value`; integers wrap around.
    Negate(Box<Expression>),
    /// `!value`.
    Not(Box<Expression>),
    /// `left operator right` between numbers of one type, or `+` of texts.
    Arithmetic {
        operator: Arithmetic,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// `<`, `<=`, `>` or `>=` between values of one type.
    Compare {
        operator: Comparison,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// `left == right`, as StarScript compares values.
    Equal(Box<Expression>, Box<Expression>),
    /// `left && right`: the right side only runs if the left one holds.
    And(Box<Expression>, Box<Expression>),
    /// `left || right`: the right side only runs if the left one fails.
    Or(Box<Expression>, Box<Expression>),
    /// Runs the first expression for its effects and gives the second:
    /// a constant read through an object, which still evaluates the object.
    Then(Box<Expression>, Box<Expression>),
    /// A call of a static function, with its arguments already converted
    /// to the types of its parameters.
    Call {
        function: FunctionId,
        arguments: Vec<Expression>,
    },
    /// A call of a method of an object, with its arguments already
    /// converted; with `?.`, a `null` object gives `null` and the
    /// arguments do not run.
    CallMethod {
        target: MethodTarget,
        receiver: Box<Expression>,
        arguments: Vec<Expression>,
        null_safe: bool,
    },
    /// `Class(arguments)`: a new object.
    Construct {
        constructor: ConstructorRef,
        arguments: Vec<Expression>,
    },
    /// A field of the object; with `?.`, a `null` object gives `null`.
    Field {
        object: Box<Expression>,
        field: FieldRef,
        null_safe: bool,
    },
    /// `value is Class`: whether the value is an object of the class or
    /// of a subclass (`null` is not).
    IsInstance {
        value: Box<Expression>,
        class: ClassRef,
    },
    /// `value as Subclass`: the object if it is one, else `null`.
    Downcast {
        value: Box<Expression>,
        class: ClassRef,
    },
    /// `Time.now()`: the seconds of the game so far, from the clock of
    /// `star_script_runtime` that the compiled game reads too.
    Now,
    /// `Debug.print(text)`.
    Print(Box<Expression>),
    /// A method of the engine, such as `Input.is_down(action)`, with its
    /// arguments (the object first, for a method of objects).
    Engine {
        method: EngineMethod,
        arguments: Vec<Expression>,
        call: EngineCall,
    },
    /// `[a, b, c]`: a new list.
    List(Vec<Expression>),
    /// `list[index]`.
    Index {
        list: Box<Expression>,
        index: Box<Expression>,
    },
    /// `value ?? fallback`: the fallback only runs if the value is `null`.
    Coalesce(Box<Expression>, Box<Expression>),
    /// A method of a basic type or list, such as `abs()` or `push(value)`;
    /// with `?.`, a `null` receiver gives `null` and the argument does not
    /// run.
    Builtin {
        method: BuiltinMethod,
        receiver: Box<Expression>,
        argument: Option<Box<Expression>>,
        null_safe: bool,
    },
    /// A call of an `async` method written alone: it starts as a
    /// coroutine, which runs until its first wait, and gives nothing.
    Start(Box<Coroutine>),
    /// `await call`: runs the coroutine as part of the one that waits for
    /// it, and gives its value.
    Await(Box<Coroutine>),
    /// An expression with an `await` inside (but not an `await` itself),
    /// which only the runner of coroutines can evaluate.
    Waits(Box<Expression>),
}

/// A value written in the program. Unlike a [`Value`], it can never be a
/// list or an object, which belong to the thread that runs them: the tree
/// is shared between threads.
#[derive(Debug, Clone)]
pub(crate) enum Literal {
    Null,
    Short(i16),
    Int(i32),
    BigInt(i64),
    Float(f32),
    Double(f64),
    Bool(bool),
    String(Arc<str>),
}

impl Literal {
    /// The value it stands for: a text shares the literal's.
    pub(crate) fn value(&self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Short(value) => Value::Short(*value),
            Self::Int(value) => Value::Int(*value),
            Self::BigInt(value) => Value::BigInt(*value),
            Self::Float(value) => Value::Float(*value),
            Self::Double(value) => Value::Double(*value),
            Self::Bool(value) => Value::Bool(*value),
            Self::String(text) => Value::String(Arc::clone(text)),
        }
    }
}

/// A call of an `async` method or a `parallel` function, before it is
/// started or awaited.
#[derive(Debug)]
pub(crate) enum Coroutine {
    /// A static `async` function, with its arguments already converted.
    Call {
        function: FunctionId,
        arguments: Vec<Expression>,
    },
    /// An `async` method of an object; with `?.`, a `null` object starts
    /// nothing and the arguments do not run.
    Method {
        target: MethodTarget,
        receiver: Box<Expression>,
        arguments: Vec<Expression>,
        null_safe: bool,
    },
    /// A `parallel` function, with its arguments already converted: it
    /// runs on another thread, with copies of them.
    Parallel {
        function: FunctionId,
        arguments: Vec<Expression>,
    },
    /// `Time.wait(seconds)`.
    Wait(Box<Expression>),
    /// `Time.next_frame()`.
    NextFrame,
}

/// The number types of StarScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumberType {
    Short,
    Int,
    BigInt,
    Float,
    Double,
}

/// The arithmetic operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arithmetic {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

/// The order comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Comparison {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
