//! Mistakes in the meaning of the code: types, names, null safety, …

use std::error::Error;
use std::fmt;

use crate::Span;
use crate::ast::ModifierKind;

/// A semantic mistake and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    /// What is wrong.
    pub kind: SemanticErrorKind,
    /// The code that is wrong.
    pub span: Span,
}

impl SemanticError {
    pub(super) const fn new(kind: SemanticErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl fmt::Display for SemanticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(formatter)
    }
}

impl Error for SemanticError {}

/// Every kind of semantic mistake. Types are stored as StarScript writes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticErrorKind {
    // --- Names ---
    /// A type that does not exist.
    UnknownType(String),
    /// A variable, member or class that does not exist.
    UnknownName(String),
    /// A member that the type does not have.
    UnknownMember {
        /// The type searched.
        owner: String,
        /// The missing member.
        name: String,
    },
    /// Two classes with the same name, or a class named like a basic type.
    DuplicateClass(String),
    /// Two members of a class with the same name.
    DuplicateMember(String),
    /// A local variable named like another one still visible.
    DuplicateLocal(String),
    /// More than one `init` in a class.
    DuplicateConstructor,
    /// A private member used outside its class.
    PrivateMember {
        /// The member.
        name: String,
        /// Its class.
        owner: String,
    },
    /// A class or method used where a value is expected.
    NotAValue(String),
    /// An input action that the project does not declare.
    UnknownAction(String),

    // --- Classes and inheritance ---
    /// A class that inherits from itself.
    CyclicInheritance(String),
    /// A class that tries to extend a basic type.
    CannotExtend(String),
    /// A member with the name of a member of a base class.
    MemberShadowsBase {
        /// The repeated name.
        name: String,
        /// The base class that has it.
        base: String,
    },
    /// A redefined method without `override`.
    MissingOverride {
        /// The method.
        name: String,
        /// The base class that defines it.
        base: String,
    },
    /// `override` on a method no base class has.
    OverrideWithoutBase(String),
    /// An `override` with different parameters or return type.
    OverrideMismatch {
        /// The method.
        name: String,
        /// The base class that defines it.
        base: String,
    },
    /// An override that is `async` when its base method is not, or the
    /// other way round.
    AsyncOverrideMismatch {
        /// The method.
        name: String,
        /// The base class that defines it.
        base: String,
        /// Whether the base method is `async`.
        base_is_async: bool,
    },
    /// A modifier that does not apply to this kind of member.
    InvalidModifier {
        /// The modifier.
        modifier: ModifierKind,
        /// What it was written on: "a field", "a constant", ….
        member: &'static str,
    },

    // --- Types ---
    /// A value of the wrong type.
    TypeMismatch {
        /// What the code needed.
        expected: String,
        /// What it got.
        found: String,
    },
    /// An operator applied to values it does not work with.
    InvalidOperands {
        /// The operator.
        operator: &'static str,
        /// Type of the left value.
        left: String,
        /// Type of the right value.
        right: String,
    },
    /// A unary operator, `is` or iteration applied to a value it does not work with.
    InvalidOperand {
        /// The operator.
        operator: &'static str,
        /// Type of the value.
        operand: String,
    },
    /// An integer literal that does not fit in its type: `var s: short = 40000`.
    IntegerOutOfRange {
        /// The value written.
        value: i64,
        /// The type it should have.
        ty: String,
        /// The smallest value of the type.
        min: i64,
        /// The largest value of the type.
        max: i64,
    },
    /// An `as` that cannot convert between the two types.
    InvalidCast {
        /// Type of the value.
        from: String,
        /// Requested type.
        to: String,
    },
    /// `[]` without a type to give it.
    CannotInferEmptyList,
    /// `var x = null` without a type.
    CannotInferNull,
    /// A field without type and without value.
    MissingTypeOrValue(String),
    /// A decorator that is neither the engine's nor a class.
    UnknownDecorator(String),
    /// The same decorator twice on one member.
    DuplicateDecorator(String),
    /// One of the engine's decorators on something other than an `export`
    /// field.
    DecoratorNeedsExport(String),
    /// One of the engine's decorators on a field of a type it does not fit.
    DecoratorField {
        /// The decorator.
        decorator: String,
        /// What the field must hold: "a number", "a `string`".
        needs: &'static str,
    },
    /// A decorator given the wrong values.
    DecoratorArguments {
        /// The decorator.
        decorator: String,
        /// What it takes.
        expected: &'static str,
    },
    /// A field both `private` and `export`: scenes set it from outside.
    PrivateExport(String),
    /// An `export` field of a class that does not extend `Node`.
    ExportOutsideNode(String),
    /// An `export` field of a type scenes cannot write.
    UnexportableType {
        /// The field's name.
        field: String,
        /// Its type, as written.
        ty: String,
    },
    /// An object variable without an initial value.
    MissingValue(String),
    /// Something other than a call used as a statement.
    UnusedValue,
    /// The result of a method that returns nothing, used as a value.
    NoValue,
    /// `a..b` outside a `for`.
    RangeOutsideFor,
    /// `[i]` on a value that is not a list.
    NotIndexable(String),
    /// `(…)` on something that is not a method or class.
    NotCallable,
    /// A call with too many or too few arguments.
    WrongArgumentCount {
        /// Parameters of the method.
        expected: usize,
        /// Arguments given.
        found: usize,
    },

    // --- Null safety ---
    /// `.` on a value that may be `null`.
    PossiblyNull(String),
    /// `.` on a `var` that may be `null`; checking it does not help, since it could change.
    PossiblyNullVariable(String),
    /// `?.`, `??` or a `null` comparison on a value that can never be `null`.
    NeverNull(String),
    /// `a?.b = value`.
    AssignThroughSafeAccess,

    // --- Context ---
    /// `self`, a field or an instance method where there is no object:
    /// in a static method, a field initializer or a constant.
    SelfUnavailable,
    /// An instance member used through the class name.
    NotStatic(String),
    /// A static method called on an object instead of its class.
    StaticThroughObject(String),
    /// A constant whose value is not known at compile time.
    NotConstant,
    /// A constant used before the line that declares it.
    ConstantUsedBeforeDeclaration(String),
    /// `super` not followed by `.method(…)`.
    InvalidSuper,
    /// `await` outside an `async` method.
    AwaitOutsideAsync,
    /// `await` before something that is not a call of an `async` method.
    AwaitWithoutAsyncCall,
    /// `await object?.method()`: there may be nothing to wait for.
    NullSafeAwait,
    /// An `async` method called inside an expression without `await`.
    AsyncCallNotAwaited(String),
    /// A `parallel` function called without `await`.
    ParallelCallNotAwaited(String),
    /// Inside a `parallel` function, a method or class that is not another
    /// `parallel` function.
    NotParallel(String),
    /// A type of a `parallel` function that is not a plain value.
    NotPlainValue(String),
    /// A `parallel` function without `->`.
    ParallelWithoutResult(String),
    /// Assigning to a constant, parameter or loop variable.
    AssignToImmutable(String),

    // --- Control flow ---
    /// A method with `->` that can end without `return`.
    MissingReturn(String),
    /// `return value` in a method or `init` that returns nothing.
    UnexpectedReturnValue,
    /// `return` without value in a method that returns one.
    MissingReturnValue(String),
    /// `break` outside a loop.
    BreakOutsideLoop,
    /// `continue` outside a loop.
    ContinueOutsideLoop,

    // --- Construction ---
    /// An object field without value that some `init` does not assign.
    FieldNotInitialized(String),
    /// `self` used in `init` before every field has a value.
    SelfBeforeInitialization(String),
    /// A subclass that does not call `super.init(…)` when the base needs it.
    MissingSuperInit(String),
    /// `super.init(…)` outside the top level of an `init`, or twice.
    MisplacedSuperInit,
    /// `super.init(…)` before the subclass gave its own fields a value
    /// (the names of the fields still missing).
    SuperInitBeforeFields(String),
    /// `return` in `init` before every field has a value.
    ReturnBeforeInitialization,
}

impl fmt::Display for SemanticErrorKind {
    // One arm per kind of error: splitting it would only scatter the messages.
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownType(name) => write!(formatter, "unknown type `{name}`"),
            Self::UnknownName(name) => write!(formatter, "`{name}` is not declared"),
            Self::UnknownMember { owner, name } => {
                write!(formatter, "`{owner}` has no member called `{name}`")
            }
            Self::UnknownAction(name) => write!(
                formatter,
                "there is no input action called \"{name}\"; actions are declared in \
                 `[input.actions]` of `project.toml`"
            ),
            Self::DuplicateClass(name) => write!(formatter, "class `{name}` already exists"),
            Self::DuplicateMember(name) => {
                write!(formatter, "`{name}` is already declared in this class")
            }
            Self::DuplicateLocal(name) => {
                write!(formatter, "a variable called `{name}` is already declared here")
            }
            Self::DuplicateConstructor => formatter.write_str("a class can only have one `init`"),
            Self::PrivateMember { name, owner } => {
                write!(formatter, "`{name}` is private to class `{owner}`")
            }
            Self::NotAValue(name) => write!(
                formatter,
                "`{name}` is not a value; call it with `{name}(…)`"
            ),
            Self::CyclicInheritance(name) => {
                write!(formatter, "class `{name}` inherits from itself")
            }
            Self::CannotExtend(name) => write!(formatter, "classes cannot extend `{name}`"),
            Self::MemberShadowsBase { name, base } => write!(
                formatter,
                "`{name}` is already declared in base class `{base}`"
            ),
            Self::MissingOverride { name, base } => write!(
                formatter,
                "`{name}` redefines a method of `{base}`; mark it with `override`"
            ),
            Self::OverrideWithoutBase(name) => write!(
                formatter,
                "`{name}` is marked `override`, but no base class has that method"
            ),
            Self::OverrideMismatch { name, base } => write!(
                formatter,
                "`{name}` must take the same parameters and return the same type as in `{base}`"
            ),
            Self::AsyncOverrideMismatch {
                name,
                base,
                base_is_async: true,
            } => write!(formatter, "`{name}` must be `async`, as in `{base}`"),
            Self::AsyncOverrideMismatch { name, base, .. } => write!(
                formatter,
                "`{name}` cannot be `async`: it is not in `{base}`"
            ),
            Self::InvalidModifier { modifier, member } => {
                write!(formatter, "`{modifier}` cannot be used on {member}")
            }
            Self::TypeMismatch { expected, found } => {
                write!(formatter, "expected `{expected}`, found `{found}`")?;
                if is_number_name(expected) && is_number_name(found) {
                    write!(formatter, "; convert it with `as {expected}`")?;
                }
                Ok(())
            }
            Self::IntegerOutOfRange {
                value,
                ty,
                min,
                max,
            } => write!(
                formatter,
                "{value} does not fit in a `{ty}`, which goes from {min} to {max}"
            ),
            Self::InvalidOperands {
                operator,
                left,
                right,
            } => write!(
                formatter,
                "`{operator}` cannot be used with `{left}` and `{right}`"
            ),
            Self::InvalidOperand { operator, operand } => {
                write!(formatter, "`{operator}` cannot be used with `{operand}`")
            }
            Self::InvalidCast { from, to } => {
                write!(formatter, "`{from}` cannot be converted to `{to}`")
            }
            Self::CannotInferEmptyList => formatter
                .write_str("the type of an empty list is unknown; declare it: `var list: [int] = []`"),
            Self::CannotInferNull => formatter.write_str(
                "the type of `null` alone is unknown; declare it: `var target: Enemy? = null`",
            ),
            Self::MissingTypeOrValue(name) => {
                write!(formatter, "`{name}` needs a type or a value")
            }
            Self::UnknownDecorator(name) => write!(
                formatter,
                "there is no decorator `@{name}`: the engine's are `@range`, `@tooltip`, `@header` and `@multiline`"
            ),
            Self::DuplicateDecorator(name) => {
                write!(formatter, "`@{name}` is written twice on the same member")
            }
            Self::DecoratorNeedsExport(name) => {
                write!(formatter, "`@{name}` only goes on `export` fields")
            }
            Self::DecoratorField { decorator, needs } => {
                write!(formatter, "`@{decorator}` only goes on fields that hold {needs}")
            }
            Self::DecoratorArguments {
                decorator,
                expected,
            } => write!(formatter, "`@{decorator}` takes {expected}"),
            Self::PrivateExport(name) => write!(
                formatter,
                "`{name}` cannot be both `private` and `export`: scenes set it from outside"
            ),
            Self::ExportOutsideNode(class) => write!(
                formatter,
                "only nodes have `export` fields, and `{class}` does not extend `Node`"
            ),
            Self::UnexportableType { field, ty } => write!(
                formatter,
                "`{field}` cannot be `export`: its type `{ty}` is not a number, `bool`, `string` or `Color`"
            ),
            Self::MissingValue(name) => write!(
                formatter,
                "`{name}` needs an initial value (objects cannot be empty; use a `?` type to allow `null`)"
            ),
            Self::NoValue => formatter.write_str("this call returns nothing, so it has no value"),
            Self::UnusedValue => {
                formatter.write_str("this value is not used; only calls can be statements")
            }
            Self::RangeOutsideFor => {
                formatter.write_str("ranges `a..b` can only be used in `for`")
            }
            Self::NotIndexable(found) => write!(formatter, "`{found}` is not a list"),
            Self::NotCallable => formatter.write_str("only methods and classes can be called"),
            Self::WrongArgumentCount { expected, found } => {
                write!(formatter, "expected {expected} arguments, found {found}")
            }
            Self::PossiblyNull(found) => write!(
                formatter,
                "this value is `{found}` and may be `null`; use `?.`, `??` or check it with `!= null` first"
            ),
            Self::PossiblyNullVariable(name) => write!(
                formatter,
                "`{name}` may be `null`; a `var` can change after being checked, so copy it into a `const` and check that, or use `?.` or `??`"
            ),
            Self::NeverNull(found) => {
                write!(formatter, "this value is `{found}`, so it is never `null`")
            }
            Self::AssignThroughSafeAccess => {
                formatter.write_str("cannot assign through `?.`; check the object first")
            }
            Self::SelfUnavailable => formatter.write_str(
                "there is no object here: static methods, field values and constants cannot use `self` or fields",
            ),
            Self::NotStatic(name) => write!(
                formatter,
                "`{name}` belongs to each object; use it on an object, not on the class"
            ),
            Self::StaticThroughObject(name) => write!(
                formatter,
                "`{name}` is static; call it on the class, not on an object"
            ),
            Self::NotConstant => formatter.write_str(
                "constants can only use literals, operators and other constants",
            ),
            Self::ConstantUsedBeforeDeclaration(name) => {
                write!(formatter, "constant `{name}` is used before it is declared")
            }
            Self::InvalidSuper => {
                formatter.write_str("`super` can only be used to call a base method: `super.name()`")
            }
            Self::AwaitOutsideAsync => formatter.write_str(
                "`await` can only be used inside an `async` method",
            ),
            Self::AwaitWithoutAsyncCall => {
                formatter.write_str("only calls of `async` methods can be awaited")
            }
            Self::NullSafeAwait => formatter.write_str(
                "`await` cannot wait through `?.`: check that the object is not `null` first",
            ),
            Self::ParallelCallNotAwaited(name) => write!(
                formatter,
                "`{name}` runs on another thread: write `await {name}(…)`, inside an `async` method"
            ),
            Self::NotParallel(name) => write!(
                formatter,
                "a `parallel` function can only call methods of values and other `parallel` \
                 functions: `{name}` is not one"
            ),
            Self::NotPlainValue(ty) => write!(
                formatter,
                "a `parallel` function only takes and gives values (numbers, `bool`, `string` \
                 and lists of them), not `{ty}`"
            ),
            Self::ParallelWithoutResult(name) => write!(
                formatter,
                "`{name}` must return a value: the result of a `parallel` function is all it does"
            ),
            Self::AsyncCallNotAwaited(name) => write!(
                formatter,
                "`{name}` is `async`: write `await {name}(…)` to wait for it, \
                 or call it on a line of its own to start it"
            ),
            Self::AssignToImmutable(name) => write!(formatter, "`{name}` cannot be changed"),
            Self::MissingReturn(name) => write!(
                formatter,
                "`{name}` must return a value in every case"
            ),
            Self::UnexpectedReturnValue => formatter.write_str("this method returns nothing"),
            Self::MissingReturnValue(expected) => {
                write!(formatter, "this method must return a `{expected}`")
            }
            Self::BreakOutsideLoop => formatter.write_str("`break` can only be used in a loop"),
            Self::ContinueOutsideLoop => {
                formatter.write_str("`continue` can only be used in a loop")
            }
            Self::FieldNotInitialized(name) => write!(
                formatter,
                "field `{name}` has no value; give it one or assign it at the start of `init`"
            ),
            Self::SelfBeforeInitialization(missing) => write!(
                formatter,
                "`self` cannot be used until the object is complete (missing: {missing})"
            ),
            Self::MissingSuperInit(base) => {
                write!(formatter, "`init` must call `super.init(…)` of `{base}`")
            }
            Self::MisplacedSuperInit => formatter.write_str(
                "`super.init(…)` must be called once, directly in the body of `init`",
            ),
            Self::SuperInitBeforeFields(missing) => write!(
                formatter,
                "`super.init(…)` must come after giving a value to {missing}, \
                 because the base `init` may already use the whole object"
            ),
            Self::ReturnBeforeInitialization => {
                formatter.write_str("`init` cannot return before every field has a value")
            }
        }
    }
}

/// Whether `name` is a numeric type, which `as` converts to any other.
fn is_number_name(name: &str) -> bool {
    matches!(name, "short" | "int" | "bigint" | "float" | "double")
}
