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
| 5 | Engine API | In progress: lifecycle (`Game`), input (`Input`), nodes (`Node`, `Sprite`) and colors (`Color`), the camera (`Camera`) and scenes (`Scene`, with script classes) done |
| 6 | Editor integration | In progress: “Compile” checks the project's scripts and scenes and lists the problems; building and running from the editor pending |
| 7–8 | Blueprints | Done: model, rules, files, translation to StarScript, “Compile” in the editor and the visual editor (graphs, class, new blueprints, mistakes on their nodes) |

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
| `Game` | fields `background: Color` (black), `camera: Camera?` (`null`); `start()`, `update(delta: float)`, `quit()`, `add_child(node: Node)` |
| `Node` | `init(name: string)`; fields `name: string`, `x`, `y`, `rotation`, `scale_x`, `scale_y: float`; `start()`, `update(delta: float)`, `add_child(child: Node)`, `remove()`, `parent() -> Node?`, `children() -> [Node]`, `find_child(name: string) -> Node?` |
| `Sprite` (extends `Node`) | `init(name: string, texture: string)`; fields `texture: string`, `width`, `height: float`, `layer: int`, `pixelated: bool`, `color: Color` (white: the image's own colors) |
| `Camera` (extends `Node`) | `init(name: string)`; field `view_height: float` (2.0) |
| `Color` | `init(r: float, g: float, b: float, a: float)`; fields `r`, `g`, `b`, `a: float` |
| `Scene` | `static load(path: string) -> Node?` |
| `Input` | `static is_down(action: string) -> bool`, `static just_pressed(action: string) -> bool`, `static just_released(action: string) -> bool` |

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

**Nodes** make up what the game shows. A `Node` is placed relative to its
parent; a `Sprite` is a node that shows an image of the project's `assets`
folder. Scripts extend them, as they extend `Game`:

```
class Ship extends Sprite {
    init() {
        super.init("Ship", "sprites/ship.png")
        width = 0.2
        height = 0.2
    }

    override func update(delta: float) {
        if Input.is_down("right") { x += 0.9 * delta }
    }
}

class Space extends Game {
    override func start() {
        add_child(Ship())
    }
}
```

- The game is the root of the tree: `add_child` of `Game` adds nodes at the
  top, and `add_child` of a node puts a child under it. A child moves,
  rotates and scales with its parent. Adding a node that already has a
  parent moves it; adding a node under itself or one of its children does
  nothing (and prints a warning).
- While a node is in the game's tree, the engine calls its `start()` once,
  at the start of the first frame it is there, and its `update(delta)`
  every frame, after the game's `update` and parents before children.
- `remove()` takes the node, with its children, out of the tree: they stop
  running and are no longer shown. The object can be added again later.
- `x`, `y`, `width` and `height` are in world units; `rotation` in radians,
  counterclockwise.
- The game is seen from its `camera`: the window is centered on where that
  `Camera` is in the world (so a camera that is the child of a node follows
  it) and shows `view_height` world units from bottom to top; the width
  follows the window's shape. The camera's rotation and scale are ignored.
  Without a camera, the window shows 2 units centered on `(0, 0)`.
- `texture` is an asset path such as `"sprites/hero.png"` (PNG or JPEG). An
  image that cannot be loaded is not shown; the engine warns once on the
  error output.
- A `Color` has red, green, blue and opacity (`a`) from `0.0` to `1.0`, as
  image editors show them. A sprite's `color` multiplies its image: white
  keeps it as it is, `Color(1.0, 1.0, 1.0, 0.5)` makes it half transparent.
  `background` fills the window behind everything. Colors are objects:
  sprites given the same `Color` change together when it changes.
- Fields of the engine classes are not redeclared by subclasses, and their
  own fields with a default value are assigned after `super.init(…)`.
- `Scene.load("levels/one.scene")` builds the nodes of a `.scene` file of
  the asset folder: a new tree, out of the game until it is added, whose
  root is returned. Nodes with an image become `Sprite` objects (with their
  size, layer, color and filter), the others `Node` objects; a sprite's
  `region` and `anchor` are not used yet. Each file is read once, so loading
  the same scene again (a bullet, an enemy) only builds new objects. If the
  scene cannot be loaded, `load` gives `null` and the reason is written once
  to the error output. Console programs have no asset folder: for them it
  is always `null`.
- A scene node can name the class it is built as, with `class = "Rock"` in
  its `[[node]]`. The class must extend `Node` (or `Sprite`, `Camera`) and
  its `init` must take no arguments: the object is created with `Rock()`,
  so it runs its own `start` and `update`, and then gets what the scene
  writes, which wins over what `init` set: its name always, its transform
  and image only if the file has them. `class = "Camera"` gives a plain
  camera. If a class does not exist, needs arguments, or is not a `Sprite`
  but the node shows an image, the whole scene is not loaded (`null`) and
  the reason, with the node's name, is written once.

```toml
[[node]]
name = "Big rock"
class = "Rock"
parent = 0
position = [1.0, 2.0]
```

```
const level = Scene.load("levels/one.scene")
if level != null {
    add_child(level)
}
```

**Input works with actions**, declared in the project, not in the scripts.
The game asks for an action by name, so the same code works with keyboard,
mouse, touch screens and gamepads:

```toml
# project.toml
[input.actions]
jump = ["Space", "MouseLeft", "Touch", "GamepadSouth"]
quit = ["Escape", "GamepadStart"]
```

```
override func update(delta: float) {
    if Input.just_pressed("jump") { jump() }
    if Input.just_pressed("quit") { quit() }
}
```

- An action is held (`is_down`) while any of its bindings is held;
  `just_pressed` and `just_released` are true only in the frame it changes.
- Binding names: keys by their position on a US keyboard (`"A"`, `"Space"`,
  `"ArrowLeft"`, `"Digit1"`, `"F1"`, `"ShiftLeft"`…), mouse buttons
  (`"MouseLeft"`, `"MouseRight"`, `"MouseMiddle"`…), `"Touch"` (any finger)
  and gamepad buttons by position (`"GamepadSouth"` is A on Xbox and ✕ on
  PlayStation; `"GamepadStart"`, `"GamepadDPadUp"`, `"GamepadLeftBumper"`…).
- **An action name written as text is checked when compiling**: a typo such
  as `Input.is_down("jupm")` is an error. A name computed while playing
  (`Input.is_down(player + "_jump")`) can only be checked then: an action
  that does not exist is never held.
- A misspelled binding name in `project.toml` stops the game when it starts,
  with an error that names it.

## 4. Translation to Rust (final)

To see the Rust of a program, or to compile and run it:

```bash
cargo run -p star_script --example run -- --rust file.star   # shows the Rust
cargo run -p star_script --example run -- file.star          # compiles and runs it
```

If the script's folder has a `project.toml`, the `run` example reads the
input actions from it (see `crates/star_script/examples/scripts/input/`).

**A program can have several files**: their classes form one program, so a
class of one file uses those of the others without importing them. Each file
is read on its own (an unclosed `{` never spills into the next one) and every
error names its file, line and column. The library takes them as a
`SourceSet` (`check_program_with`, `compile_program_with`); the editor gives
it every `.star` file of the project's `assets` folder.

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
Nodes are objects too: the engine keeps a node of its world in step with
each one in the game's tree (see `star_engine::script_nodes`).

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

## 5. Blueprints (in progress)

Node graphs (events, calls, variables, flow) saved in `.blueprint` files. The
editor translates them to StarScript, so everything a blueprint does can be
read, and debugged, as code. A blueprint is a class: it can inherit from
another class and other classes can use it. They live in the `star_blueprint`
crate.

**Model**: a blueprint has a class name, an optional base class (with the
constant values its `init` needs, `base_arguments`: a node gives its name,
a sprite also its image; the blueprint itself is created without
arguments), variables
(fields: a name, a type and/or a starting value, `private` or not) and
graphs. Each **graph** is a method: an event of the base class
(`override = true`, such as `update`) or one of its own, with its
parameters, the type it returns and its nodes and links.

**Pins**: nodes are joined by wires from an output pin to an input pin.
**Execution** pins say in which order things happen; **data** pins carry
values.

| Node (`kind`) | Inputs | Outputs |
|---------------|--------|---------|
| `entry` (one per graph) | — | execution `then`; one value per parameter |
| `literal` (`value`) | — | `value` |
| `get_variable` (`variable`) | — | `value` |
| `set_variable` (`variable`) | execution `in`, `value` | execution `then` |
| `call` (`target`, `method`, `arguments`, `returns_value`, `pure`) | execution `in` (not if pure), `target` (if it calls a method of a value), one per argument | execution `then` (not if pure), `result` (if it returns a value) |
| `operator` (`add`, `subtract`, `multiply`, `divide`, `remainder`, `equal`, `not_equal`, `less`, `less_or_equal`, `greater`, `greater_or_equal`, `and`, `or`, `not`, `negate`) | `left` and `right`, or `value` for `not` and `negate` | `result` |
| `branch` (an `if`) | execution `in`, `condition` | execution `true`, `false` |
| `return` | execution `in`, `value` (if the graph returns one) | — |

A call's `target` is `"self"` (a method of the blueprint), `"value"` (a
method of the object given to its `target` pin) or `{ class = "Input" }` (a
static method). A **pure** call (`Input.is_down`) only computes a value: it
has no execution pins and runs where its value is used, like literals,
variables and operators.

**Rules** (`validate`):

- Names (class, variables, graphs, parameters, methods, arguments) are valid
  StarScript names, and are not repeated where they would clash.
- A variable has a type, a starting value or both.
- Every graph has exactly one `entry`.
- Links go from an output to an input of the same kind (execution or data),
  between nodes and pins that exist.
- **Each execution pin takes at most one wire**, so the order of a graph is
  a tree and reads as structured code. Paths that would join again are
  written as a function called from each one.
- Each data input has exactly one wire; a data output feeds any number.
- Every node with execution pins is reached from the `entry`.
- No value depends on itself.

Types are not checked by the rules: the StarScript compiler checks them in
the translated code.

**Files**: TOML with the version of their layout, written and read by
`Blueprint::to_text` and `Blueprint::from_text`:

```toml
format = 1
class = "Player"
extends = "Sprite"
base_arguments = ["Player", "sprites/player.png"]   # super.init(…)

[[variable]]
name = "speed"
default = 2.0

[[graph]]
name = "update"
override = true
parameters = [{ name = "delta", type = "float" }]

[[graph.node]]
id = 1
kind = "entry"

[[graph.node]]
id = 2
kind = "set_variable"
variable = "speed"
position = [240.0, 0.0]   # where the editor draws it

[[graph.node]]
id = 3
kind = "operator"
operator = "multiply"

[[graph.node]]
id = 4
kind = "get_variable"
variable = "speed"

[[graph.link]]
from = { node = 1, pin = "then" }
to = { node = 2, pin = "in" }

[[graph.link]]
from = { node = 4, pin = "value" }
to = { node = 3, pin = "left" }

[[graph.link]]
from = { node = 1, pin = "delta" }
to = { node = 3, pin = "right" }

[[graph.link]]
from = { node = 3, pin = "result" }
to = { node = 2, pin = "value" }
```

**Translation** (`translate`): each blueprint becomes one StarScript class,
written to be read. The example above becomes:

```
// Translated from a blueprint: change the blueprint, not this code.
class Player extends Sprite {
    var speed = 2.0

    init() {
        super.init("Player", "sprites/player.png")
    }

    override func update(delta: float) {
        speed = speed * delta
    }
}
```

- Statements follow the execution wires from the `entry`; a `branch` is an
  `if`/`else` (or `if !condition` when only `false` is wired) and nothing
  follows it, as the flow is a tree.
- Values are written where they are used: literals, variables, operators and
  pure calls. Operators inside other expressions get parentheses.
- The result of a call that runs in order is kept in a constant named after
  its method and node (`const roll_3 = roll()`), and only exists after it,
  in its own path of a branch: using it elsewhere is an error.
- Members are written without `self.` unless a parameter hides them.
- Each piece of the code remembers its node, variable or graph
  (`Translation::origin`), so the mistakes the StarScript compiler finds in
  it are placed back on the blueprint.

In the editor, a double click on a `.blueprint` of “Assets” opens it in a
tab: its graphs are drawn as nodes and wires, built with the mouse (drag
between pins, right button to add nodes), and the inspector edits the
selected node, or the class and the graph shown. “New blueprint…” creates
one. Saving rewrites the file from the model, so comments written by hand
are not kept.

The editor's “Compile” checks every `.blueprint` of `assets/` together with
the `.star` files, as one program: scripts can use the classes of blueprints
and the other way round. A mistake in a blueprint is shown as
`assets/rock.blueprint: graph `update`, node 3: …`, and the node is marked
in red in the blueprint editor.

## 6. Plain Rust

A game can write classes in Rust by implementing the engine's traits, and
StarScript can use them like any other class. The exact mechanism will be
defined in phase 5.
