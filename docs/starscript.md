# StarScript

The programming language of ProjectStar games. It hides the complexity of
Rust (lifetimes, borrowing, traits) behind a **100% object-oriented**
language with **static typing and inference**, inspired by the ergonomics of
GDScript but with blocks in braces.

File extension: **`.star`**.

## Compilation flow

```
Blueprint (.blueprint) ──► StarScript (.star) ──► Rust (.rs) ──► machine code
   visual editor            text                  generated       cargo / rustc
```

- Each stage produces the input of the next one; none is skipped.
- A game can mix the three forms: blueprints, StarScript and **plain Rust**.
- The generated Rust is readable, but it is an artifact: it is not edited by hand.

## Principles

1. **Everything is an object**: code lives in classes. There are no free
   functions. Basic types have methods too (`3.abs()`, `"hello".length()`).
2. **Errors when compiling, not when playing**: static typing catches mistakes
   before running. The compiler reports file, line and column.
3. **Rust is never seen**: no lifetimes, no `&mut`, no `Rc<RefCell<…>>`.
   Error messages talk about StarScript, not about the generated code.
4. **Rust's performance**: static types allow generating direct Rust, without
   an intermediate dynamic type (`Variant`).

## Status by phase

| Phase | Content | Status |
|-------|---------|--------|
| 1 | Lexicon (lexer) | **Final** |
| 2 | Syntax (parser and AST) | **Final** |
| 3 | Types, null safety and semantics | **Final** |
| 4 | Rust generation | **Final** |
| 5 | Engine API | In progress: lifecycle (`Game`) done; input and nodes with engine phases 2 and 6 |
| 6 | Editor integration | Pending |
| 7–8 | Blueprints | Draft |

## 1. Lexicon (final)

### Source file

- **UTF-8** text. A leading BOM is ignored.
- Line endings may be `\n` or `\r\n`.

### Comments

```
// Comment until the end of the line.
/* Block comment, /* nestable */ and spanning several lines. */
```

### Line breaks

A line break **ends the statement**, so `;` is optional (as in Kotlin or
Swift). `;` allows writing several statements on one line.

- Several consecutive breaks count as one.
- Inside `( … )` and `[ … ]` line breaks are ignored, so a long call can be
  split across several lines. Inside a `{ … }` they count again, even if that
  block is inside parentheses.

### Identifiers

`[A-Za-z_][A-Za-z0-9_]*`. ASCII only for now: `año` gives a clear error. This
way any name translates to a valid Rust identifier.

Convention: `PascalCase` for classes, `snake_case` for variables and methods,
`SCREAMING_SNAKE_CASE` for constants.

### Keywords

| Use | Words |
|-----|-------|
| Classes | `class` `extends` `init` `self` `super` `override` `private` `static` |
| Declarations | `var` `const` `func` |
| Control flow | `if` `else` `while` `for` `in` `break` `continue` `return` |
| Types | `as` `is` |
| Values | `true` `false` `null` |
| Reserved (future) | `interface` `implements` `abstract` `enum` `match` `signal` |

Reserved words do nothing yet, but they cannot be used as names. That way,
enabling them later will not break existing programs.

The basic types (`int`, `float`, `bool`, `string`) are **not** keywords:
they are names of predefined classes.

### Literals

| Type | Examples | Notes |
|------|----------|-------|
| Integer | `42` `1_000_000` `0xFF` `0b1010` | 64-bit signed. `_` separates digits |
| Decimal | `3.14` `0.5` `1e3` `2.5e-3` | Needs a digit before the point: `.5` is not valid |
| Text | `"Hello"` `"line\n"` `"\u{1F680}"` | A single line. Escapes: `\n \t \r \\ \" \0 \u{…}` |
| Boolean | `true` `false` | |

`0..10` is a range (`0`, `..`, `10`), not a decimal. A number stuck to
letters (`10px`) is an error.

### Operators and signs

| Category | Signs |
|----------|-------|
| Arithmetic | `+` `-` `*` `/` `%` |
| Comparison | `==` `!=` `<` `<=` `>` `>=` |
| Logical | `&&` `\|\|` `!` |
| Assignment | `=` `+=` `-=` `*=` `/=` `%=` |
| Null safety | `?` (nullable type) `?.` (safe access) `??` (default value) |
| Others | `.` `..` `,` `:` `;` `->` `( )` `{ }` `[ ]` |

## 2. Syntax (final)

```
// A player that moves right and loses lives when hit.
class Player extends Sprite {
    const MAX_LIVES = 3

    var speed: float = 200.0
    private var lives = MAX_LIVES

    init(speed: float) {
        self.speed = speed
    }

    override func update(delta: float) {
        if Input.is_pressed("right") {
            position.x += speed * delta
        }
    }

    func hit() {
        lives -= 1
        if lives <= 0 { destroy() }
    }

    func is_alive() -> bool {
        return lives > 0
    }
}
```

- **File**: one or more classes.
- **Class**: `class Name [extends Base] { members }`. Single inheritance.
- **Members**:
  - `var` (mutable field) and `const` (class constant).
  - `func` (method); the return type goes after `->` and is omitted if it
    returns nothing.
  - `init` (constructor).
  - Modifiers: `private`, `static` and `override` (required when redefining
    a method of the base).
- **Statements**:
  - Local `var`/`const`.
  - Assignment (`=`, `+=`, …).
  - `if`/`else if`/`else`, `while`, `for x in 0..10` / `for x in list`.
  - `break`, `continue`, `return`, and expressions (calls).
- **Assignment is a statement, not an expression**: `if x = 3` is an error,
  not a silent bug.
- **`self` is implicit**: `speed` means `self.speed` unless a local variable
  hides it.
- **Creating objects**: `Player(250.0)` calls the constructor; there is no `new`.
- **Lists**: `[1, 2, 3]` and `list[i]`. Their type is defined in phase 3.
- **Trailing commas** are allowed in parameters, arguments and lists: `f(a, b,)`.

Rules for lines and braces:

- Statements on the same line are separated with `;`: `a(); b()`. Without it,
  `a() b()` is an error.
- `if`, `while` and `for` end with their block and need no separator.
- The `{` may go on the same line or on the next one (Allman style), and
  `else` may start on the line after the `}`.
- An operator at the **end** of a line continues the expression on the next
  one: `total = a +` ⏎ `b`. At the start of the line it does not, because the
  previous line already ended the statement.
- Comparisons, `is` and ranges **do not chain**: `0 < x < 10` is an error;
  write `0 < x && x < 10`. Operators of different levels do combine:
  `a < b == c`.
- Nesting is limited to 128 levels, so an absurd file gives an error instead
  of exhausting the stack.
- Which modifiers each member accepts (e.g. `override` on a field) is checked
  in phase 3; the parser only rejects repeated ones.

To see how the parser understands a file:

```bash
cargo run -p star_script --example parse -- file.star
```

It shows the tree with expressions fully parenthesized, e.g.
`(+= (. position x) (* speed delta))`.

Operator precedence, from lowest to highest:

| Level | Operators |
|-------|-----------|
| 1 | `\|\|` |
| 2 | `&&` |
| 3 | `==` `!=` |
| 4 | `<` `<=` `>` `>=` `is` |
| 5 | `??` (groups to the right: `a ?? b ?? c` = `a ?? (b ?? c)`) |
| 6 | `..` |
| 7 | `+` `-` |
| 8 | `*` `/` `%` |
| 9 | `as` |
| 10 | unary `-` `!` |
| 11 | call `f()`, member `a.b` / `a?.b`, index `a[i]` |

## 3. Types and semantics (final)

To see which types the compiler infers, or its errors:

```bash
cargo run -p star_script --example check -- file.star
```

### Types

| StarScript | Rust | Default value |
|------------|------|---------------|
| `int` | `i64` | `0` |
| `float` | `f32` (same as the engine and the GPU) | `0.0` |
| `bool` | `bool` | `false` |
| `string` | `String` | `""` |
| `[T]` (list) | runtime `List<T>` (shared) | `[]` |
| `T?` (nullable) | `Option<T>` | `null` |
| Classes | shared `Rc` reference (section 4) | — (they need a value) |

Lists are written `[int]`, not `Array<int>`, because `<` is also the
comparison: `x as int < 3` would be ambiguous.

Objects and lists are **shared by reference**, as in GDScript: after
`var a = b`, both names are the same list, and `a.push(1)` is also seen
through `b`. Numbers, `bool` and `string` are copied.

### Inference

- `var x = 3` infers `int`; `var x = [1, 2]` infers `[int]`;
  `var e = [a, null]` infers `[Enemy?]`.
- `null` alone and an empty `[]` do not tell their type: it must be written
  (`var target: Enemy? = null`, `var names: [string] = []`).
- An integer literal becomes a `float` where one is needed:
  `var speed: float = 2`, `speed * 2` and `2 * speed` are valid.
- Apart from that, `int` and `float` **do not mix**: `speed + lives` is an
  error; write `speed + lives as float`. The error suggests it.
- `+` joins texts: `"Lives: " + lives.to_string()`.

### Null safety

A StarScript program **never fails because of an unexpected `null`**: the
compiler prevents it.

- `Enemy` is never `null`. Only `Enemy?` can be, and only it can be assigned
  `null`.
- `.` cannot be used on an `Enemy?`. There are three alternatives:
  - `target?.hit()`: if `target` is `null`, it does nothing. `target?.health`
    is an `int?`.
  - `target?.name ?? "nobody"`: the value, or the one on the right if it is
    `null`.
  - Checking it first; inside the `if`, the compiler knows it is not `null`:

    ```
    const enemy = target
    if enemy == null { return }
    enemy.hit()          // valid: here enemy is an Enemy
    ```
- Narrowing works with `!= null`, `== null` followed by
  `return`/`break`/`continue`, `&&`, `||`, `!` and `is`. It only applies to
  what **cannot change**: local constants (`const`), parameters and `for`
  variables. A `var` or a field could become `null` again between the check
  and the use, so they are first copied into a `const`. The error message
  explains it.
- There is no "trust me" operator (`!!`): it would open the door to failures
  while playing.
- Using `?.`, `??` or `== null` on something that is never `null` is an
  error: that code is unnecessary and usually hides a confusion.

### Classes and objects

- **Fields**: `var name: Type = value`. Without a type, it is inferred from
  the value; without a value, the type's default value is used. Fields of a
  non-nullable class type have no default value: either they have `= value`,
  or `init` assigns them.
- **Safe construction**: inside `init`, `self` (and therefore the fields and
  methods) cannot be used until the object is complete, that is, until the
  pending fields have been assigned and `super.init(…)` has been called if
  needed. Those assignments must be directly in the body of `init`, not
  inside an `if`. That way a half-built object never exists.
- **Inheritance**: if the `init` of the base class has parameters, the `init`
  of the subclass must call `super.init(…)` once. `super.method()` calls the
  base class version. Constructors are not inherited: each class declares its
  own if it needs one.
- **Own fields first, then `super.init(…)`** (as in Swift): the base `init`
  may call methods that the subclass overrides, and those methods already
  find every field with a value.
- **Overriding methods**: requires `override` and the same parameters and
  return type. Fields and constants cannot repeat names of the base.
- **Values of fields and constants**: they cannot use `self` or other fields.
  Constants only accept literals, operators, `as` and other constants
  declared before.
- **`private`**: only used inside its class (other objects of the same class
  included).
- **`static func`**: belongs to the class and is called as `Class.method()`;
  it has no `self`. `static var` fields do not exist yet.
- **`is` and `as`**: `x is Player` checks the class. `x as Player` towards a
  subclass may fail, so it gives a `Player?`. Between `int` and `float`, `as`
  converts.
- **Creating objects**: `Player(…)` with the arguments of its `init`. A
  `private init` can only be used inside the class.

### Rules for methods and statements

- A method with `->` must return a value on **every** path.
- Only calls can be statements: a lone `a + 1` is an error.
- Parameters, constants and the variable of a `for` cannot be reassigned.
- A local variable cannot have the name of another one still visible. It can
  have the name of a field: `self.speed = speed`.
- Ranges `a..b` only exist in `for`.

### Values at run time

- `==` compares numbers, `bool`, `string` and lists **by content** (lists,
  element by element); objects, **by identity** (whether they are the same
  object). `null` is only equal to `null`.
- Integers **wrap around** on overflow (`9223372036854775807 + 1` gives the
  minimum), the same in debug and release builds.
- `a / b` between integers rounds towards zero (`-7 / 2` is `-3`) and `a % b`
  takes the sign of `a` (`-7 % 3` is `-1`). `float as int` drops the decimals
  and saturates at the extremes.
- `for x in list` goes through the elements it had when it started: changing
  the list inside the loop does not change what it goes through.
- `float.to_string()` always shows that it is a `float`: `2.0`, not `2`.
- **Runtime errors**: reading outside a list (`list[5]` with two elements) or
  dividing an integer by zero **stops the game** with a clear message
  (`StarScript runtime error: index 5 is out of range for a list of length
  2`). They cannot be detected when compiling, and continuing with a made-up
  value would hide the bug.

### Predefined methods

| Type | Methods |
|------|---------|
| `int` | `abs()` `min(x)` `max(x)` `to_string()` |
| `float` | `abs()` `min(x)` `max(x)` `floor()` `ceil()` `round()` `sqrt()` `to_string()` |
| `bool` | `to_string()` |
| `string` | `length()` `is_empty()` `contains(s)` `to_upper()` `to_lower()` `to_string()` |
| `[T]` | `length()` `is_empty()` `contains(x)` `push(x)` `pop() -> T?` `clear()` |

There are no global functions: everything is a method of some class.

### Engine classes

The engine provides them; their names cannot be used for other classes.

| Class | Members |
|-------|---------|
| `Debug` | `static print(message: string)`: writes a line to the output. |
| `Game` | `start()`, `update(delta: float)`, `quit()` |

**A game is a class that extends `Game`**:

```
class MyGame extends Game {
    var time = 0.0

    override func start() {
        Debug.print("Here we go!")
    }

    override func update(delta: float) {
        time += delta
        if time > 10.0 { quit() }
    }
}
```

- The engine creates the object (its `init` cannot have parameters), opens
  the window, calls `start()` once and `update(delta)` every frame; `delta`
  is the number of seconds since the previous frame.
- `quit()` closes the game at the end of the frame. If it is not called, the
  game goes on until the user closes the window.
- `start` and `update` do nothing unless they are overridden.
- Input and nodes will come with phases 2 (input) and 6 (scenes) of the
  engine: each piece of the engine is born with its StarScript API.

## 4. Translation to Rust (final)

To see the Rust of a program, or to compile and run it:

```bash
cargo run -p star_script --example run -- --rust file.star   # shows the Rust
cargo run -p star_script --example run -- file.star          # compiles and runs it
```

A program starts in **one single** place:

- **A game**: the class that extends `Game` and that no other class extends
  (so `class MyGame extends BaseGame` runs `MyGame`). It opens in a window of
  the engine, titled with the class name, and the generated crate also
  depends on `star_engine`.
- **A console program**: the class with `static func main()`. It only depends
  on `star_script_runtime`; tests and tools use it.

Without either of them, the generated code can only be used as a library.

**Result**: a Rust file, the root of its own crate, that depends on
`star_script_runtime` (shared lists, fields, equality and runtime errors). It
carries no license notices: it belongs to the game's author.

**Objects**: shared references with reference counting (`Rc`): an object is
released when nobody uses it, and accessing it never fails. Two objects that
reference each other are not released; weak references will solve that.
Engine nodes (phase 5) will be separate engine handles.

**Each class** `Player` generates a module `classes::Player` with:

| Item | What it is |
|------|------------|
| `Fields` | The fields the class declares, each one in a runtime `Field<T>`. |
| `HasFields` | Trait that gives access to those fields in objects of any subclass. |
| `Object` | Trait with the overridable methods; a subclass's extends its base's. |
| `Ref` | `Rc<dyn Object>`: a reference to an object of the class or of a subclass. |
| `Methods` | The code of each method, and the constants (as functions). |
| `Instance` | The objects created as that class: the `Fields` of each class of its hierarchy. |
| `init_`, `new` | Two-phase construction; `Player(…)` calls `new`. |

- **Inheritance**: a `Player::Ref` becomes a `Character::Ref` at no cost (the
  traits extend each other). `is` and `as` use the `AnyObject` trait, with one
  conversion per class.
- **Methods**: overridable ones are called through their trait, so each
  object runs its own version; `super.method()` and `private` methods call
  the code of their class directly.
- **Construction**: `init_` receives a function that creates the final
  object. The subclass prepares its fields (without `self`) and calls the
  `init_` of its base; when the root class has its own, the whole object is
  created; then each `init` runs the rest of its body, from the root down.
- **Fields**: reading a field copies its value and writing it replaces it,
  without leaving borrows open: the borrow checker never fails at run time.
- **Null safety**: `T?` is `Option<T>`. Where the compiler proved that a value
  is not `null`, the Rust code takes it out without checking again.
- **Names**: they are kept as written. Those that clash with Rust are adapted
  (`type` → `r#type`, `crate` → `crate_`) and those ending in `_` get another
  one, so they never match the generator's internal names (`this_`,
  `object_`, …).
- The generated Rust is built with `cargo` like any other crate. The
  developer needs the Rust toolchain installed.

## 5. Blueprints (draft)

Node graphs (events, calls, variables, flow) saved in `.blueprint` files. The
editor translates them to StarScript, so everything a blueprint does can be
read, and debugged, as code. A blueprint is a class: it can inherit from
another class and other classes can use it.

## 6. Plain Rust

A game can write classes in Rust by implementing the engine's traits, and
StarScript can use them like any other class. The exact mechanism will be
defined in phase 5.
