//! The interpreter's own tree of a program: what lowering builds and
//! running walks. Names are resolved, and conversions are written out, so
//! the two operands of an operation always hold values of the same type.

use star_script::semantic::BuiltinMethod;

use crate::value::Value;

/// Position of a function in the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FunctionId(pub(crate) usize);

/// A static function, or the value of a constant.
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
