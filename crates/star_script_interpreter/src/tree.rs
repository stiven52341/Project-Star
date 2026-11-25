//! The interpreter's own tree of a program: what lowering builds and
//! running walks. Names are resolved, and conversions are written out, so
//! the two operands of an operation always hold values of the same type.

use star_script::semantic::BuiltinMethod;

use crate::value::Value;

/// A whole lowered program: what a [`crate::Program`] shares with the
/// coroutines it starts.
#[derive(Debug)]
pub(crate) struct Code {
    pub(crate) functions: Vec<Function>,
    pub(crate) constructors: Vec<Constructor>,
    pub(crate) classes: Vec<Class>,
    /// `static func main()`.
    pub(crate) main: FunctionId,
}

/// Position of a function in the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FunctionId(pub(crate) usize);

/// Position of a class of the program, which its objects carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClassRef(pub(crate) usize);

/// Position of a constructor in the program: one per class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConstructorId(pub(crate) usize);

/// What running needs to know about a class.
#[derive(Debug)]
pub(crate) struct Class {
    /// The function of each method that objects run their own version
    /// of, by its place in the table.
    pub(crate) methods: Vec<FunctionId>,
    /// The class itself and its bases: what `is` accepts.
    pub(crate) ancestors: Vec<ClassRef>,
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
    pub(crate) constructor: ConstructorId,
    pub(crate) arguments: Vec<Expression>,
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
        position: usize,
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
}

/// A piece of code that produces a value.
#[derive(Debug)]
pub(crate) enum Expression {
    /// A value known before running: a literal.
    Constant(Value),
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
        constructor: ConstructorId,
        arguments: Vec<Expression>,
    },
    /// The field at `position` of the object's row of fields; with `?.`,
    /// a `null` object gives `null`.
    Field {
        object: Box<Expression>,
        position: usize,
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
