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
| 5 | Engine API | In progress: lifecycle (`Game`), input (`Input`: actions, analog strength and the pointer), nodes (`Node`, `Sprite`) and colors (`Color`), the cameras (`Camera2D`, `Camera3D`), scenes (`Scene`, with script classes), sounds (`Sound`, `Audio`), physics (`Body`) and 3D (`Node3D`, `Mesh`, `Light`, `Camera3D`) done |
| 6 | Editor integration | In progress: “Compile” checks the project's scripts and scenes and lists the problems; “Play” builds and runs the game; the editor has a code editor for `.star` files (colors, writing aids, mistakes while writing, suggestions) |
| 7–8 | Blueprints | Done: model, rules, files, translation to StarScript, “Compile” in the editor and the visual editor (graphs, class, new blueprints, mistakes on their nodes) |
| 9 | Concurrency and parallelism | Done: coroutines (`async`, `await`, `Time`) and `parallel` functions (section 7), shown by the code editor; example `examples/scripts/coroutines.star` |

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
| Classes | `class` `extends` `init` `self` `super` `override` `private` `static` `export` |
| Concurrency | `async` `await` `parallel` |
| Declarations | `var` `const` `func` |
| Control flow | `if` `else` `while` `for` `in` `break` `continue` `return` |
| Types | `as` `is` |
| Values | `true` `false` `null` |
| Reserved (future) | `interface` `implements` `abstract` `enum` `match` `signal` |

Reserved words do nothing yet, but they cannot be used as names. That way,
enabling them later will not break existing programs.

The basic types (`short`, `int`, `bigint`, `float`, `double`, `bool`,
`string`) are **not** keywords:
they are names of predefined classes.

### Literals

| Type | Examples | Notes |
|------|----------|-------|
| Integer | `42` `1_000_000` `0xFF` `0b1010` | An `int`, unless its place needs another number type (see Inference). `_` separates digits |
| Decimal | `3.14` `0.5` `1e3` `2.5e-3` | A `float`, or a `double` where one is needed. Needs a digit before the point: `.5` is not valid |
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
| Others | `.` `..` `,` `:` `;` `->` `( )` `{ }` `[ ]` `@` (decorator) |

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
  - Decorators: `@name` or `@name(arguments)` before a member, on its own
    line or the same one (see “Decorators”).
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
| `short` | `i16` (from -32768 to 32767) | `0` |
| `int` | `i32` (about ±2100 million): the usual integer | `0` |
| `bigint` | `i64` (about ±9.2·10¹⁸) | `0` |
| `float` | `f32` (same as the engine and the GPU): the usual decimal | `0.0` |
| `double` | `f64`, for when a `float` is not precise enough | `0.0` |
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
- An integer literal takes the number type its place needs:
  `var speed: float = 2`, `speed * 2` and `2 * speed` are valid, and so are
  `var small: short = 3` and `small + 1`. It must fit in that type:
  `var small: short = 40000` is an error, `-32768` is a valid `short`. A
  decimal literal is a `double` where one is needed (`var d: double = 0.1`).
- A number **converts by itself to a wider type that holds every one of
  its values exactly**: `short` → `int` → `bigint`, `float` → `double`,
  `short` → `float`, `int` → `double`. So `var total: bigint = lives`,
  `small + lives` (an `int`), `precise + speed` (a `double`) and
  `small < lives` are valid. It happens wherever a value is stored or
  passed (variables, fields, arguments, `return`, `T?`) and between the two
  sides of an operator, whose result is the wider type.
- Conversions that may lose values need `as`: `int` → `short`, `bigint` →
  `int`, `float` → `int`, and also `int` → `float` and `bigint` → `double`
  (a `float` holds integers exactly only up to 16777216). So
  `speed + lives` is an error; write `speed + lives as float`. The error
  suggests it.
- Lists do not convert: a `[short]` is not a `[int]`, because both names
  would be the same shared list.
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
- **`export var`**: a field of a node that scenes, and so the editor's
  inspector, can give a value: `export var speed = 2.5`. Only classes that
  extend `Node` have them; the field holds a number, a `bool`, a `string`
  or a `Color`, and cannot be `private`. The value written in the class is
  the one used when the scene gives none.

### Decorators

`@name(arguments)` before a member. The engine's own say how the editor's
inspector shows an `export` field, and only go on those; their arguments
are plain values (numbers, texts):

```
@header("Motion")
@range(0, 10, 0.5)
@tooltip("Units a second")
export var speed = 2.5

@multiline
export var notes = ""
```

| Decorator | On | Does |
|-----------|----|------|
| `@range(min, max)`, `@range(min, max, step)` | numbers | a slider between `min` and `max` (`min` below `max`), in steps of `step` (positive) if given |
| `@tooltip("text")` | any | the help shown over the field |
| `@header("title")` | any | a title over the field and those after it |
| `@multiline` | `string` | a box of several lines |

A name that is not one of these is an error, as is the same decorator
twice on one member, or one on a field of a type it does not fit.

**Decorators of the program** are classes that extend `Decorator`, an
engine class written in StarScript. They go on methods that return
nothing and are not `static`, `async` or `parallel`, and run around them:
`before(method)` first (if it gives `false`, the method does not run) and
`after(method)` when it ends. Their arguments are given to their `init`.

```
class Cooldown extends Decorator {
    var seconds: float
    var last = -1000.0

    init(seconds: float) {
        self.seconds = seconds
    }

    override func before(method: string) -> bool {
        if Time.now() - last < seconds {
            return false
        }
        last = Time.now()
        return true
    }
}

class Player extends Node {
    @Cooldown(0.5)
    func shoot() { … }
}
```

- Each object keeps its own decorator objects, made with it: a cooldown
  counts for each player apart.
- Several decorators nest, the first one outermost: `@Log @Once func f()`
  logs every call, also those `Once` stops.
- StarScript has no functions as values, so the compiler rewrites each
  decorated method before checking it: the code moves to a hidden method,
  and the method calls `before`, that code and `after` of each decorator,
  kept in hidden fields. Hidden names have `@`, so no program can name
  them, and the editor does not suggest them.
- **`static func`**: belongs to the class and is called as `Class.method()`;
  it has no `self`. `static var` fields do not exist yet.
- **`is` and `as`**: `x is Player` checks the class. `x as Player` towards a
  subclass may fail, so it gives a `Player?`. Between number types, `as`
  converts.
- **Creating objects**: `Player(…)` with the arguments of its `init`. A
  `private init` can only be used inside the class.

### Rules for methods and statements

- A method with `->` must return a value on **every** path.
- Only calls can be statements: a lone `a + 1` is an error.
- Parameters, constants and the variable of a `for` cannot be reassigned.
- A local variable cannot have the name of another one still visible. It can
  have the name of a field: `self.speed = speed`.
- Ranges `a..b` only exist in `for`. Their ends are integers, and the loop
  variable has the type they meet in (`small..lives` gives `int`s; `int`
  if both are literals).

### Values at run time

- `==` compares numbers, `bool`, `string` and lists **by content** (lists,
  element by element); objects, **by identity** (whether they are the same
  object). `null` is only equal to `null`.
- Integers **wrap around** on overflow (`2147483647 + 1` gives the minimum
  `int`), the same in debug and release builds. `as` between integer types
  keeps the lowest bits: `70000 as short` is `4464`.
- List indices and `length()` are `int`.
- `a / b` between integers rounds towards zero (`-7 / 2` is `-3`) and `a % b`
  takes the sign of `a` (`-7 % 3` is `-1`). `float as int` (or `short`,
  `bigint`) drops the decimals and saturates at the extremes; `double as
  float` rounds to the nearest `float`.
- `for x in list` goes through the elements it had when it started: changing
  the list inside the loop does not change what it goes through.
- `float.to_string()` and `double.to_string()` always show a decimal:
  `2.0`, not `2`.
- **Runtime errors**: reading outside a list (`list[5]` with two elements),
  dividing an integer by zero or nesting more than 1000 calls (a function
  that calls itself without end; `async` methods and constructors are not
  counted) **stops the game** with a clear message (`StarScript runtime
  error: index 5 is out of range for a list of length 2`). They cannot be
  detected when compiling, and continuing with a made-up value would hide
  the bug.

### Predefined methods

| Type | Methods |
|------|---------|
| `short`, `int`, `bigint` | `abs()` `min(x)` `max(x)` `to_string()` |
| `float`, `double` | `abs()` `min(x)` `max(x)` `floor()` `ceil()` `round()` `sqrt()` `to_string()` |
| `bool` | `to_string()` |
| `string` | `length()` `is_empty()` `contains(s)` `to_upper()` `to_lower()` `to_string()` |
| `[T]` | `length()` `is_empty()` `contains(x)` `push(x)` `pop() -> T?` `clear()` |

There are no global functions: everything is a method of some class.

### Engine classes

The engine provides them; their names cannot be used for other classes.

| Class | Members |
|-------|---------|
| `Debug` | `static print(message: string)`: writes a line to the output. It also takes a number of any type and writes it as its `to_string` does (`Debug.print(2.0)` writes `2.0`); anything else (a `bool`, a list, an object, a nullable number) is converted first with `.to_string()` if it has one. `static show_fps(shown: bool)`: shows or hides the frames-per-second counter in the corner of the game's window from the end of the frame (decision 85; F3 does too); a console program shows nothing. |
| `Game` | fields `background: Color` (black), `camera: Camera2D?` (`null`), `camera_3d: Camera3D?` (`null`), `ambient: Color` (black: no ambient light), `ambient_2d: Color` (white: sprites show their own colors); `start()`, `update(delta: float)`, `quit()`, `add_child(node: Node)` |
| `Node` | `init(name: string)`; fields `name: string`, `x`, `y`, `rotation`, `scale_x`, `scale_y: float`, `visible: bool` (`true`), `body: Body?` (`null`), `tags: [string]` (`[]`); `start()`, `update(delta: float)`, `add_child(child: Node)`, `remove()`, `parent() -> Node?`, `children() -> [Node]`, `find_child(name: string) -> Node?`, `has_tag(tag: string) -> bool`, `find_by_name(name: string) -> Node?`, `find_by_tag(tag: string) -> [Node]`, `global_x() -> float`, `global_y() -> float`, `global_rotation() -> float`, `set_global_position(x: float, y: float)`, `set_global_rotation(angle: float)`, `translate(x: float, y: float)`, `translate_global(x: float, y: float)`, `rotate(angle: float)`; tweens (see below): `async tween(property: string, to: float, seconds: float, ease: string)`, `async tween_by(property: string, by: float, seconds: float, ease: string)`, `async tween_color(property: string, to: Color, seconds: float, ease: string)`, `async tween_with(property: string, to: float, seconds: float, ease: string, options: TweenOptions)`, `stop_tweens()` |
| `Sprite` (extends `Node`) | `init(name: string, texture: string)`; fields `texture: string`, `width`, `height: float`, `layer: int`, `pixelated: bool`, `color: Color` (white: the image's own colors), `columns`, `rows: int` (1), `frame: int` (0): its sprite sheet; `frames: string` (`""`): its `.frames` file; `play(name: string)`, `stop()`, `is_playing() -> bool`, `animation() -> string`; the event `animation_finished(name: string)` |
| `Light2D` (extends `Node`) | `init(name: string, kind: string)`; fields `kind: string`, `color: Color` (white), `intensity` (1.0), `range` (3.0), `angle` (about 0.524 radians: 30°), `blend: float` (0.2) |
| `Animator` (extends `Node`) | `init(name: string)`; fields `animations: string` (`""`): its `.anim` file, and `states: string` (`""`): the `.states` file of its state machine; `play(name: string)`, `stop()`, `is_playing() -> bool`, `animation() -> string`; the event `animation_finished(name: string)`; its machine's `set_value(name: string, value: float)`, `set_flag(name: string, value: bool)`, `trigger(name: string)` and `state() -> string` |
| `Bone2D` (extends `Node`) | `init(name: string)`; field `length: float` (0.5): how far the bone reaches along its rotation. Bones are joints of cut-out characters: their sprites are their children and turn with them; they are not drawn while the game plays (decision 82). A scene's bone is a `[node.bone]` (`length = 0.5`) |
| `Camera2D` (extends `Node`) | `init(name: string)`; fields `view_height: float` (2.0), `main: bool` (`false`) |
| `Node3D` (extends `Node`) | `init(name: string)`; fields `z`, `pitch`, `yaw: float` (0.0), `scale_z: float` (1.0); `look_at(x: float, y: float, z: float)`, `global_z() -> float`, `set_global_position_3d(x: float, y: float, z: float)`, `translate_3d(x: float, y: float, z: float)`, `translate_global_3d(x: float, y: float, z: float)` |
| `Mesh` (extends `Node3D`) | `init(name: string, shape: string)`; fields `shape: string`, `width`, `height`, `depth` (1.0), `radius` (0.5), `color: Color` (white), `metallic` (0.0), `roughness: float` (0.5), `texture: string` (`""`: none), `pixelated: bool` (`false`) |
| `Model` (extends `Node3D`) | `init(name: string, file: string)`; field `file: string`: the `.glb` or `.gltf` of the assets it shows, every mesh and material of it, placed and turned as the node (decision 83). A file that cannot be read is a warning, once, and the node shows nothing. A scene's model is a `[node.model]` (`file = "chars/knight.glb"`) |
| `SoundPlayer` (extends `Node`) | `init(name: string, sound: string)`; fields `sound: string`: the sound file it plays, `volume: float` (1.0), `looping: bool` (`false`), `play_on_start: bool` (`true`); `play()`: plays its sound from the beginning (stopping the one it played), `stop()`, `is_playing() -> bool`. A player that plays on start plays the frame it starts, before its `start()`; removing it from the tree stops its sound (decision 87) |
| `Light` (extends `Node3D`) | `init(name: string, kind: string)`; fields `kind: string`, `color: Color` (white), `intensity` (1.0), `range` (10.0), `angle` (about 0.524 radians: 30°), `blend: float` (0.2) |
| `Camera3D` (extends `Node3D`) | `init(name: string)`; fields `fov: float` (about 1.047 radians: 60°), `main: bool` (`false`) |
| `Color` | `init(r: float, g: float, b: float, a: float)`; fields `r`, `g`, `b`, `a: float` |
| `Scene` | `static load(path: string) -> Node?`, `static find_by_name(name: string) -> Node?`, `static find_by_tag(tag: string) -> [Node]` |
| `Sound` | no `init`: `static load(path: string) -> Sound?`; fields `volume: float` (1.0), `looping: bool` (`false`); `play()`, `stop()`, `is_playing() -> bool` |
| `Audio` | `static volume() -> float`, `static set_volume(volume: float)` |
| `TweenOptions` | `init()`; fields `delay: float` (0.0), `back: bool` (`false`), `repeats: int` (1; `-1` repeats forever): what `tween_with` does besides going to its value once |
| `Time` | `static async wait(seconds: float)`, `static async next_frame()`: what coroutines wait for (section 7); `static now() -> float`: the seconds of the game so far (the sum of the frames' `delta`s, the clock `wait` counts with); `static delta() -> float`: the seconds of the frame going on, the same `delta` that `update` gets, for code away from `update` (0 before the first frame) |
| `Decorator` | `before(method: string) -> bool` (`true`), `after(method: string)`: what the program's decorators redefine (see “Decorators”) |
| `Body` | no `init`: `static circle(kind: string, radius: float) -> Body`, `static rectangle(kind: string, width: float, height: float) -> Body`, `static capsule(kind: string, height: float, radius: float) -> Body`, `static sphere(kind: string, radius: float) -> Body`, `static box(kind: string, width: float, height: float, depth: float) -> Body`, `static mesh(kind: string) -> Body`; fields `kind`, `shape: string`, `radius` (0.5), `width`, `height`, `depth` (1.0), `offset_x`, `offset_y`, `offset_z` (0.0), `gravity_scale` (1.0), `friction` (0.5), `bounciness` (0.0), `density` (1.0), `velocity_x`, `velocity_y`, `velocity_z: float` (0.0), `fixed_rotation`, `sensor: bool` (`false`); `apply_impulse(x: float, y: float)`, `apply_impulse_3d(x: float, y: float, z: float)`, `touching() -> [Node]`, `just_touched() -> [Node]`, `just_separated() -> [Node]` |
| `Input` | `static is_down(action: string) -> bool`, `static just_pressed(action: string) -> bool`, `static just_released(action: string) -> bool`, `static strength(action: string) -> float`, `static axis(negative: string, positive: string) -> float`, `static pointer_x() -> float`, `static pointer_y() -> float`, `static has_pointer() -> bool` |

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
- `visible = false` hides a node and every node under it: they are not
  drawn and their lights do not shine, but they stay in the tree, move,
  collide and run their code. Scenes save it (`visible = false` in a
  `[[node]]`).
- `x`, `y`, `width` and `height` are in world units; `rotation` in radians,
  counterclockwise.
- A node's `x`, `y`, `rotation` and scales are **local**: relative to its
  parent, which places it scaled, then turned, then moved, as the engine
  draws it. Its **global** place, in the world, comes from methods:
  `global_x()`, `global_y()` and `global_rotation()` read it;
  `set_global_position(x, y)` and `set_global_rotation(angle)` change the
  local values so the node ends up there. `translate(x, y)` moves it in its
  parent's space (adds to `x` and `y`), `translate_global(x, y)` in the
  world, and `rotate(angle)` turns it more. A `Node3D` adds `global_z()`,
  `set_global_position_3d`, `translate_3d` and `translate_global_3d`, and
  its 2D ones keep its depth in the world. 2D nodes are placed by their 2D
  ancestors and 3D nodes by their 3D ancestors, as the engine does; a
  parent scaled to nothing leaves its children where they are when they
  are placed globally. `global_rotation` adds the rotations of the node
  and its ancestors (for a `Node3D`, their rolls), which is exact unless a
  parent is scaled differently across and up.
- The game is seen from its `camera`: the window is centered on where that
  `Camera2D` is in the world (so a camera that is the child of a node
  follows it) and shows `view_height` world units from bottom to top; the
  width follows the window's shape. The camera's rotation and scale are
  ignored. Without a camera, the window shows 2 units centered on `(0, 0)`.
- A camera whose `main` is `true` becomes the game's `camera` (or
  `camera_3d`) when it enters the game's tree, just before its `start`:
  a scene can bring the camera it is seen from. A scene has at most one
  main camera. Cameras that enter later win; `main` changed afterwards
  does nothing until the camera enters the tree again. The game can still set `camera` itself at any time.
- Sprites are lit by the game's `ambient_2d`, white at first, so they show
  their own colors, and by its `Light2D` nodes, which add their color to
  the sprites they reach. Darken `ambient_2d` (a night, a cave) for the
  lights to be seen. A `Light2D` is `"point"` (from the node, in every
  direction) or `"spot"` (from the node, in a cone along its +x axis,
  turned with its `rotation`), with the `angle` and `blend` of a 3D spot;
  both fade smoothly to nothing at their `range`. At most 16 shine in a
  frame. `Scene.load` makes a node with `[node.light_2d]` (`kind`,
  `color` of three numbers, `intensity`, `range`, and the `angle` and
  `blend` of spots) a `Light2D`.
- Nodes are found by name or by tag. `find_child` looks only at a node's
  children; `node.find_by_name` and `node.find_by_tag` look at all its
  descendants (not the node itself), parents before their children and
  children in order: the first one named so, or every one with the tag.
  `Scene.find_by_name` and `Scene.find_by_tag` do the same in the whole
  game's tree, from any script; in a console program, which has no game,
  they find nothing. A node's `tags` is a list of words that groups of
  nodes share (`enemy`, `pickup`): `tags.push("enemy")`, `has_tag("enemy")`,
  or `tags = ["enemy"]` in its `[[node]]` of a scene.
- `texture` is an asset path such as `"sprites/hero.png"` (PNG or JPEG). An
  image that cannot be loaded is not shown; the engine warns once on the
  error output.
- A sprite sheet is an image cut into `columns` x `rows` equal cells; the
  sprite shows the one numbered `frame`, counted from the top-left cell,
  row by row (`columns = 4`, `rows = 2`, `frame = 5` shows the second cell
  of the second row). Fewer than one column or row count as one, and frames
  go round the sheet both ways, so `frame += 1` (or `-= 1`) loops over its
  cells. A scene's sprite keeps them in its `[node.sprite]` (`columns`,
  `rows`, `frame`), where they cut the sprite's `region`.
- A sprite's animations are the clips of a `.frames` file of the assets,
  named in its `frames` field. The file names the image and its cells, and
  each clip's frames, speed (frames per second) and whether it loops:

  ```toml
  format = 1
  texture = "sprites/hero.png"
  columns = 8
  rows = 4

  [[clip]]
  name = "run"
  frames = [8, 9, 10, 11, 12, 13]
  speed = 12.0     # 10 if left out
  looping = true   # true if left out
  ```

  While a game runs, a sprite with `frames` shows the file's image and
  cells, and `play("run")` starts the clip from its first frame (playing
  the clip that already plays does not restart it). Every frame, right
  after the sprite's `update`, its `frame` moves to the clip's cell for the
  time it has played, so a clip played in `update` shows its first frame at
  once. A clip that loops starts over; one that does not stays on its last
  frame and `is_playing()` becomes `false`, and the engine calls the
  sprite's `animation_finished(name)` with the clip's name, in that same
  frame (override it to chain clips or react: `override func
  animation_finished(name: string) { play("idle") }`); clips that loop and
  `stop()` never call it. `stop()` keeps the frame shown.
  `animation()` is the clip playing, or the last one played. A scene's
  sprite names them in its `[node.sprite]` (`frames = "hero.frames"`,
  `animation = "idle"` to play from the start). A file that cannot be read,
  or a clip it does not have, is a warning on the error output, and the
  sprite stays as it is. A program without a game keeps the fields, but no
  clip moves.
- An `Animator` plays keyframe animations on its **parent** and the nodes
  under it: those of the `.anim` file of the assets named in its
  `animations` field. Each animation has a length in seconds, whether it
  loops, tracks that change a property of a node from key to key, and calls
  to methods:

  ```toml
  format = 1

  [[animation]]
  name = "attack"
  length = 0.6
  looping = false              # false if left out

  [[animation.track]]
  node = "Arm"                 # a path from the parent: "" (or left out) is
                               # the parent, "Arm/Hand" a child of a child
  property = "rotation"
  interpolation = "ease_out"   # step, linear (if left out), ease_in,
                               # ease_out or ease_in_out
  keys = [[0.0, 0.0], [0.3, 1.2], [0.6, 0.0]]   # [time, value]

  [[animation.call]]
  node = ""
  time = 0.3
  method = "hit"
  ```

  A track changes a field of the node's engine class (`x`, `y`,
  `rotation`, `scale_x`, `scale_y`, `visible`; a bone's `length`; `z`, `pitch`, `yaw`,
  `scale_z` of a `Node3D`; a sprite's `texture`, `width`, `height`,
  `layer`, `pixelated`, `color`, `columns`, `rows`, `frame`; a mesh's or a
  light's numbers and `color`; a camera's `view_height` or `fov`) or else
  an `export` field of its class. Keys take the values scenes give fields:
  `true`/`false`, whole numbers, decimals, texts and colors (`[r, g, b,
  a]`); all the keys of a track are of one kind, in order of time, within
  the length. Decimals and colors follow the interpolation between two
  keys; whole numbers, `bool`s and texts jump from one key's value to the
  next (so `frame` steps through cells). Before its first key a track has
  that key's value, and after its last, the last one's.
  While a game runs, `play("attack")` starts an animation from its start
  (playing the one that already plays does not restart it). Every frame,
  right after the animator's `update`, each property its tracks change
  gets its value for the time the animation has played. One that loops
  starts over; one that does not ends on its last values, `is_playing()`
  becomes `false` and the engine calls the animator's
  `animation_finished(name)`, as a sprite's. `stop()` leaves every
  property as it is.
  A call runs a method of its node in the frame the animation passes its
  time, after the tracks are applied (and before `animation_finished`):
  one of the methods of the node's script class that is public, not
  `static`, nor `async` or `parallel`, and takes no arguments; the
  object's own version runs, as with any call. A call at the start runs in
  the frame the animation is played; when a looping animation goes round,
  the calls past its end and those from its start run. A frame long enough
  to go round more than once runs each call once. A runtime error in the
  method stops the game as anywhere else. A scene's animator names its file in its
  `[node.animator]` (`animations = "knight.anim"`, `animation = "idle"` to
  play from the start). A file that cannot be read, an animation it does
  not have, a node that is not there or a value that does not fit its
  property is a warning on the error output (once), and the rest goes on.
- An animator whose `animations` names a 3D model (`.glb`, `.gltf`)
  plays that model's own animations on its parent `Model` (decision 83):
  they are played as an `.anim`'s are (`play`, `animation()`,
  `is_playing()`, a `.states` machine with its fades and blends), loop,
  and change no property; the engine bends the model's skeleton with the
  animations the animator shows, at their times and weights. A `Model`
  is posed by the first animator under it that plays a model's file; one
  without it stands at rest.
- An animator whose `states` names a `.states` file runs a **state
  machine** that chooses its animations (decision 81):

  ```toml
  format = 1
  start = "idle"

  [[value]]
  name = "speed"
  kind = "number"              # number, flag or trigger

  [[state]]
  name = "idle"
  animation = "idle"           # an animation of the animator's `.anim`
  position = [0.0, 0.0]        # where the editor draws it

  [[state]]
  name = "move"
  blend = "speed"              # a number value that blends animations
  points = [{ animation = "walk", at = 1.0 }, { animation = "run", at = 4.0 }]

  [[transition]]
  from = "idle"                # left out: from any other state
  to = "move"
  fade = 0.2                   # seconds; 0 (if left out) cuts
  at_end = false               # wait for the state to have played once
  conditions = [{ value = "speed", above = 0.1 }]
  # also `below = 1.0` for numbers, `is = true` for flags, and only
  # `value` for a trigger
  ```

  Numbers start at zero and flags at `false`; scripts set them with
  `set_value` and `set_flag`, and `trigger(name)` sets a trigger for the
  frame it is set in. Every frame, right after the animator's `update`,
  the machine takes the first transition, in the file's order, that
  leaves its state (or any other, without `from`) and whose conditions all
  hold, waiting, with `at_end`, until the state's last frame of a round was
  shown; then it gives the properties the values of its state's
  animations. A state that blends plays the two animations around its
  value (the first or the last one alone outside them), mixed by how near
  the value is to each, and keeps them in step. While a transition fades,
  the state left keeps playing, and its values give way to the new one's:
  decimals and colors blend, and whole numbers, `bool`s and texts jump
  halfway; a property only one of them changes keeps its value. The calls
  of the animation of the state that weighs the most run, and when it is
  one that does not loop and ends, `animation_finished(name)` runs once
  with its name. `state()` is the state the machine is in (empty before
  its first frame), and `animation()` that animation. `play(name)` goes to
  the state `name` at once, and `stop()` holds the machine until a state
  is played. A scene's animator names the file in its `[node.animator]`
  (`states = "knight.states"`). A file that cannot be read, a value it
  does not have or of another kind, and a state or animation that is not
  there are warnings on the error output (once).
- A **tween** takes one property of a node, named as a track names it
  (above), from where it is to a value over some seconds, with an ease
  (decision 80):

  ```
  await self.tween("x", 5.0, 0.5, "ease_out")      // until x is 5
  self.tween_by("y", 2.0, 0.3, "linear")           // y + 2, without waiting
  self.tween_color("color", Color(1.0, 0.0, 0.0, 1.0), 0.2, "linear")

  const options = TweenOptions()
  options.delay = 0.5      // waits half a second first
  options.back = true      // then comes back, taking as long again
  options.repeats = 3      // there and back three times; -1: forever
  self.tween_with("x", 5.0, 0.5, "ease_in_out", options)
  self.stop_tweens()
  ```

  The ease is one of the words of `.anim` interpolations: `step`,
  `linear`, `ease_in`, `ease_out` or `ease_in_out`. Tweens are `async`
  methods: `await` waits until the tween ends, and a call written alone
  starts it as a coroutine and goes on (section 7). A tween reads where
  its property is when it starts, after its delay, and changes it once per
  frame, with the coroutines, after the nodes' `update`; it ends right on
  its value (or back where it started, with `back`). `tween` and
  `tween_with` go to a decimal, `tween_by` adds a decimal to where the
  property starts, and `tween_color` goes to a color: the property must
  hold a decimal (a field of the engine classes or an `export` field), or
  a color for `tween_color`. A new tween of a property of a node takes the
  place of the one it had: that one ends where it was, and its `await`
  finishes. A tween also ends when its node leaves the game's tree, and
  `stop_tweens()` ends every tween of the node. An unknown ease, a
  property the node does not have or that holds another kind of value,
  negative seconds or delay, and `repeats` below 1 other than `-1` are
  runtime errors.
- A `Color` has red, green, blue and opacity (`a`) from `0.0` to `1.0`, as
  image editors show them. A sprite's `color` multiplies its image: white
  keeps it as it is, `Color(1.0, 1.0, 1.0, 0.5)` makes it half transparent.
  `background` fills the window behind everything. Colors are objects:
  sprites given the same `Color` change together when it changes.
- Fields of the engine classes are not redeclared by subclasses, and their
  own fields with a default value are assigned after `super.init(…)`.
- A program in which no class extends `Game` (and none has `static func
  main`) plays the project's main scene (`main_scene` in `project.toml`),
  if it has one, as a plain `Game` with that scene added to its tree: the
  scene's nodes run their classes' code and it is seen from its main
  camera. Programs compiled outside the editor do not have one.
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
  its `[[node]]`. The class must extend `Node` (or `Sprite`, `Camera2D`) and
  its `init` must take no arguments: the object is created with `Rock()`,
  so it runs its own `start` and `update`, and then gets what the scene
  writes, which wins over what `init` set: its name always, its transform
  and image only if the file has them. A node with a
  `[node.camera]` table (`view_height`, `main`) is a `Camera2D`, or
  the class it names, which must extend it; `class = "Camera2D"` alone
  gives a plain camera. If a class does not exist, needs arguments, or is not a `Sprite`
  but the node shows an image, the whole scene is not loaded (`null`) and
  the reason, with the node's name, is written once.
- A node with a class can give its `export` fields values, in a
  `[node.fields]` table: `speed = 2.5`, `lives = 3`, `alive = true`,
  `title = "Boss"`, `tint = [1.0, 0.0, 0.0, 1.0]` (a `Color`). They are
  set after `init`, so they win over the values of the class; a field
  of decimals also takes a whole number. A value for a field that is not
  `export`, or that does not fit its type, stops the scene as an unknown
  class does, and “Compile” in the editor reports it first.

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

**Sounds** are files of the asset folder (WAV, OGG Vorbis, MP3 or FLAC),
loaded with `Sound.load`:

```
class Shooter extends Game {
    var shot = Sound.load("sounds/shot.wav")
    var music = Sound.load("music/theme.ogg")

    override func start() {
        if music != null {
            music.looping = true
            music.volume = 0.6
            music.play()
        }
    }

    override func update(delta: float) {
        if Input.just_pressed("shoot") { shot?.play() }
    }
}
```

- `Sound.load` gives `null` if the file cannot be loaded, and the reason is
  written once to the error output. Each file is read once: loading it
  again (another enemy's shot) gives a new `Sound`, with its own `volume`
  and `looping`, that shares the samples. Console programs have no asset
  folder: for them it is always `null`.
- `play()` starts the sound once more, so quick shots overlap. `volume` (1
  as recorded, 0 silent, above 1 louder) and `looping` are read when
  `play()` is called: a sound already playing keeps how it started (stop it
  and play it again to change it). `stop()` stops every playing that this
  `Sound` started; `is_playing()` says whether one still sounds.
- `Audio.set_volume(0.5)` scales every sound at once, also those playing;
  `Audio.volume()` reads it (1 at the start).
- Sounds load from the moment the game object is created (fields included),
  but are only heard from `start` on. Without an audio device the game runs
  silent: `play` is accepted and `is_playing()` is always `false`.
- `Sound` cannot be extended, and has no `init`: `Sound()` is an error.

**Physics**: a node with a `Body` in its `body` field falls, collides and
is pushed. Bodies are made by their shape, with how they move as a word:
`"dynamic"` (gravity, collisions, impulses), `"static"` (never moves:
floors, walls) or `"kinematic"` (moves only at its velocity, and pushes
dynamic bodies: platforms, doors).

```
class Ball extends Sprite {
    init() {
        super.init("Ball", "sprites/ball.png")
        const solid = Body.circle("dynamic", 0.5)
        solid.bounciness = 0.6
        body = solid
    }
}

class Hero extends Sprite {
    init() {
        super.init("Hero", "sprites/hero.png")
        const solid = Body.capsule("dynamic", 2.0, 0.5)
        solid.fixed_rotation = true
        body = solid
    }

    override func update(delta: float) {
        const solid = body
        if solid == null { return }
        solid.velocity_x = Input.axis("left", "right") * 4.0
        if Input.just_pressed("jump") { solid.apply_impulse(0.0, 5.0) }
        for other in solid.just_touched() {
            if other.name == "Coin" { other.remove() }
        }
    }
}
```

- The engine simulates the bodies after the scripts' frame, and before the
  next one writes where it left them into `x`, `y` and `rotation`, and how
  fast they move into `velocity_x` and `velocity_y`. Setting `x` or `y`
  moves the body there at once; setting the velocity makes it move at that
  velocity from then on. `apply_impulse` pushes a dynamic body once (mass
  times velocity: a jump, a hit), when the frame ends.
- `kind` and `shape` can be changed, like the sizes and the surface
  (`friction`, `bounciness`, `density`, `gravity_scale`, `fixed_rotation`,
  `sensor`): the engine then replaces the body. Only the sizes of the
  shape are used: `radius` for circles, `width` and `height` for
  rectangles, `height` (half circles included) and `radius` for capsules.
  A body on a `Sprite` whose `texture` is one of the images the engine
  brings, `"@square"` or `"@circle"` (white, shown in the sprite's
  `color`; decision 86), always takes that shape and the sprite's
  `width` and `height`: a rectangle, or a circle as wide as the larger
  of them, so a floor is `Sprite("Floor", "@square")` with any body,
  stretched by its size or its scale. `visible = false` hides it and
  keeps its body.
  `offset_x`, `offset_y` and `offset_z` (2D bodies leave `z` out) place
  the shape's center away from the node's origin, in the node's space: a
  character whose origin is at its feet keeps its capsule above them.
  Shapes are stretched by the node's scale in the world (decision 84):
  rectangles and boxes along each axis, circles and spheres by the
  largest factor, capsules with their height by `y` and their radius by
  the others; a scale changed while playing remakes the shape.
- A body that cannot be simulated (an unknown kind or shape, a size that is
  not positive, a negative friction) is left out, and the reason is written
  once to the error output; the node goes on without a body.
- `touching()` gives the nodes whose bodies touch this one now,
  `just_touched()` those that started touching it during the last frame and
  `just_separated()` those that stopped. A **sensor** (`sensor = true`)
  does not collide: it only notices what overlaps it (coins, doors, areas
  that hurt). The lists are new each time, and only name nodes of the
  game's tree.
- `body = null` takes the body away. A `Body` belongs to one node: given
  to two, both are simulated, and it shows what happened to the last one.
- Nodes of scenes keep their `[node.body]`. Console programs have no
  physics: their bodies never move or touch anything.
- `Body` cannot be extended, and has no `init`: `Body()` is an error.

**3D**: a `Node3D` is a node placed in 3D. It is a `Node`, so it keeps
`x`, `y`, `scale_x`, `scale_y`, its `body` and the whole tree (`add_child`,
`children`, …; 2D and 3D nodes can hang from each other), and adds `z`,
`pitch` (around x: up and down), `yaw` (around y: left and right) and
`scale_z`; its `rotation` is the roll, around z. A `Mesh` shows a shape, a
`Light` lights the meshes, and the game is seen from its `camera_3d`:

```
class Crate extends Mesh {
    init() {
        super.init("Crate", "cube")
        y = 3.0
        yaw = 0.5
        roughness = 0.8
        texture = "textures/crate.png"
        body = Body.box("dynamic", 1.0, 1.0, 1.0)
    }
}

class World3D extends Game {
    override func start() {
        const eye = Camera3D("Eye")
        eye.y = 4.0
        eye.z = 10.0
        eye.look_at(0.0, 0.0, 0.0)
        camera_3d = eye
        add_child(eye)
        const sun = Light("Sun", "directional")
        sun.pitch = -1.0
        add_child(sun)
        ambient = Color(0.2, 0.2, 0.25, 1.0)
        add_child(Crate())
    }
}
```

- Angles are radians, applied roll first, then pitch, then yaw: a camera
  turns left and right, then looks up and down. A 3D node looks along its
  -z axis (where cameras look and directional lights shine), so with no
  rotation it looks towards -z. `look_at(x, y, z)` sets the pitch and yaw
  that make it look at a point given in its parent's space, as its own
  `x`, `y` and `z` are; the roll is kept.
- A `Mesh`'s `shape` is `"cube"` (`width`, `height`, `depth`), `"sphere"`
  (`radius`) or `"plane"` (flat and facing up: `width` and `depth`); only
  the sizes of its shape are used. Its surface is physically based:
  `color`, `metallic` (0 plastic, wood…; 1 metal) and `roughness` (0
  polished, 1 matte); `texture` is an image of the asset folder multiplied
  by the color. A mesh of an unknown shape or a size that is not positive
  is not shown, and the reason is written once to the error output.
- A `Light` is `"directional"` (the sun: from far away, along the node's
  -z, wherever the node is), `"point"` (a lamp: from the node, in every
  direction, reaching `range` world units) or `"spot"` (a flashlight: from
  the node, in a cone along its -z, reaching `range`). A spot's `angle` goes
  from its axis to the edge of its cone, more than 0 and less than a right
  angle (others are not shown, and the reason is written once); `blend` is
  how much of the cone, from its edge inwards, fades into the dark, from 0
  (a sharp edge) to 1. `intensity` 1 lights a white
  matte surface facing it fully. At most 16 lights shine in a frame.
  `ambient` lights everything a little, from no direction.
- With a `camera_3d` the game is drawn in perspective from where that
  camera is in the world (so it follows its parents), looking along its
  -z; `fov` is how wide it sees, from bottom to top, in radians. Its roll
  is ignored. `camera_3d` wins over `camera`; with a 3D camera,
  `Input.pointer_x` and `pointer_y` are not known (`has_pointer()` is
  `false`).
- The `body` of a `Node3D` is a body of the 3D physics: `Body.sphere`,
  `Body.box` (with `depth`), `Body.capsule` (vertical) or `Body.mesh`, and
  it moves in `z`, `pitch` and `yaw` too, with `velocity_z` and
  `apply_impulse_3d`. A 2D shape on a 3D node (or a 3D one on a 2D node)
  is left out, and the reason is written once.
- `Body.mesh` (shape `"mesh"`, decision 84) collides with the triangles of
  what its node shows: the shape of a `Mesh`, or the file of a `Model` (a
  model with a skeleton as it is at rest). Static and kinematic bodies
  collide with every triangle, for floors, walls and whole levels;
  dynamic ones with the convex hull around them, which has a volume to
  fall and roll with (a flat mesh such as a plane cannot be dynamic). The
  triangles are taken when the body is added, stretched by the node's
  scale; the body waits for them, so a node that shows nothing never gets
  one (warned once).
- `Scene.load` builds 3D nodes too: a node with `[node.transform_3d]` is a
  `Node3D`, with `[node.mesh]` a `Mesh` and with `[node.light]` a `Light`
  (or the class it names, which must extend them); `[node.body_3d]` gives
  its body. A node with `[node.camera_3d]` (`fov` in radians, `main`)
  is a `Camera3D`, as `class = "Camera3D"` is. The divisions of a sphere
  are not kept.

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
  gamepad buttons by position (`"GamepadSouth"` is A on Xbox and ✕ on
  PlayStation; `"GamepadStart"`, `"GamepadDPadUp"`, `"GamepadLeftBumper"`…)
  and the directions of the sticks (`"GamepadLeftStickLeft"`,
  `"GamepadLeftStickRight"`, `"GamepadLeftStickUp"`, `"GamepadLeftStickDown"`
  and the same with `RightStick`).
- **Analog input**: a stick direction holds its action once the stick leans
  halfway. `Input.strength("right")` says how strongly an action is held,
  from `0.0` to `1.0`: keys and buttons give `1.0` while held, triggers
  (`"GamepadLeftTrigger"`, `"GamepadRightTrigger"`) how far they are
  pressed, and stick directions how far the stick leans, even before it
  holds the action. `Input.axis("left", "right")` is the strength of
  `right` minus that of `left`, from `-1.0` to `1.0`: one line moves with
  keys and sticks alike, softly with a stick. Both names are checked when
  compiling.

```toml
left = ["A", "ArrowLeft", "GamepadLeftStickLeft"]
right = ["D", "ArrowRight", "GamepadLeftStickRight"]
```

```
x += Input.axis("left", "right") * speed * delta
```
- **An action name written as text is checked when compiling**: a typo such
  as `Input.is_down("jupm")` is an error. A name computed while playing
  (`Input.is_down(player + "_jump")`) can only be checked then: an action
  that does not exist is never held.
- A misspelled binding name in `project.toml` stops the game when it starts,
  with an error that names it.
- **Where the player points**: `Input.pointer_x()` and `Input.pointer_y()`
  are the point of the world under the mouse cursor or a finger, in the
  same units as `x` and `y` of nodes, seen from the game's `camera`. A
  finger on the screen (or one that leaves it in this frame) wins over the
  mouse. `Input.has_pointer()` is false while there is no finger and the
  cursor is outside the window; the position is then where the player last
  pointed (`(0, 0)` if they never did). Pointing is not pressing: a click or
  a tap is an action bound to `"MouseLeft"` or `"Touch"`.

```
override func update(delta: float) {
    if Input.just_pressed("shoot") && Input.has_pointer() {
        shoot_at(Input.pointer_x(), Input.pointer_y())
    }
}
```

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
(fields: a name, a type and/or a starting value, `private` or not, and
`export` or not, with the options of the inspector in `[variable.inspector]`:
`range = [0.0, 10.0]`, `tooltip`, `header` and `multiline`, written as
decorators) and
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
| `get_variable` (`variable`, `of_value`) | `target` (if `of_value`) | `value` |
| `set_variable` (`variable`, `of_value`) | execution `in`, `target` (if `of_value`), `value` | execution `then` |
| `call` (`target`, `method`, `arguments`, `returns_value`, `pure`) | execution `in` (not if pure), `target` (if it calls a method of a value), one per argument | execution `then` (not if pure), `result` (if it returns a value) |
| `operator` (`add`, `subtract`, `multiply`, `divide`, `remainder`, `equal`, `not_equal`, `less`, `less_or_equal`, `greater`, `greater_or_equal`, `and`, `or`, `not`, `negate`) | `left` and `right`, or `value` for `not` and `negate` | `result` |
| `branch` (an `if`) | execution `in`, `condition` | execution `true`, `false` |
| `for_each` (a `for` loop) | execution `in`, `list` | execution `body`, `element`, execution `done` |
| `return` | execution `in`, `value` (if the graph returns one) | — |

A variable node reads or sets a field of the blueprint (or of its base),
or, with `of_value = true`, the field of the object given to its `target`
pin: `enemy.x`. Files without `of_value` use the blueprint's own field.

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
  follows it, as the flow is a tree. A `for_each` is `for element_N in
  list { … }` with the path of its `body` inside, and goes on with the path
  of `done`; its `element` only exists inside the body.
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
selected node, or the class and the graph shown. “Create › Blueprint”, in
the right-click menu of “Assets”, creates one. Saving rewrites the file from the model, so comments written by hand
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

## 7. Concurrency and parallelism (final)

Objects, lists and nodes live on the game's thread (they are `Rc`, see
section 4), so StarScript has two separate tools:

- **Coroutines** (`async` and `await`): code that waits across frames, on
  the game's thread. It can use everything: objects, nodes, the input.
- **Parallel functions** (`parallel`): heavy work on the other cores of the
  processor. It only sees values (numbers, `bool`, `string` and lists of
  them), copied in and out, so two threads never share anything.

### Coroutines

```
class Lamp extends Sprite {
    async func blink(times: int) {
        for i in 0..times {
            color = Color(1.0, 1.0, 1.0, 0.0)   // see-through
            await Time.wait(0.25)
            color = Color(1.0, 1.0, 1.0, 1.0)
            await Time.wait(0.25)
        }
    }

    override func start() {
        blink(3)            // starts it; start() goes on at once
    }
}
```

- `async` is a modifier of methods (also `static` ones), like `private`.
  `init` cannot be `async`. An `override` is `async` exactly when the method
  it redefines is. The engine's methods (`start`, `update`…) are not, so
  they start coroutines instead of waiting.
- `await call` waits until an `async` method finishes and gives its value:
  `var path = await find_path()`. It can only be written inside an `async`
  method, and only before a call of something that waits. Not through
  `?.`: `await lamp?.blink()` might have nothing to wait for, so the
  object is checked first (`lamp?.blink()` alone, which starts it, is
  valid).
- **An `async` method called without `await`** starts a **coroutine**: it
  runs at once until its first `await`, and the caller goes on. That call
  can only be a statement on its own (its value, if any, is lost).
- What waits, from the engine: `Time.wait(seconds: float)` (time of the
  game, the sum of the `delta`s) and `Time.next_frame()`.
- **When they continue**: once per frame, after every `update` of the
  frame, each coroutine whose wait is over goes on until its next `await`.
  A wait of `0.5` continues in the first frame at least `0.5` seconds later.
- **Coroutines of nodes end with them**: a coroutine started by calling a
  method of a node (`blink(3)` above, or `lamp.blink(3)`) ends, without
  continuing, if that node is not in the game's tree when its wait is over.
  The rest (static methods, objects that are not nodes) run until they
  finish.
- In console programs (`static func main`, which can be `async` too), the
  coroutines that are still waiting when `main` returns continue until all
  of them finish, and `Time.wait` waits for real. There is no game tree
  there, so the coroutines of nodes never end early.
- **In Rust**, an `async` method returns a runtime `Task<T>`: a boxed
  future (`async move`) that owns its object and arguments; `await` is
  Rust's `.await`. A call written alone goes to `tasks::start`, and the
  game calls `tasks::frame_started(delta)` before the `update`s and
  `tasks::resume()` after them. The tasks of a node carry a check of
  `Node::in_tree_`. `?.` and `??` are translated as `match` rather than
  closures, so an `await` may appear inside them.

### Parallel functions

```
class Paths {
    parallel static func find(tiles: [int], from: int, to: int) -> [int] {
        // only values here: no objects, no nodes, no engine classes
    }
}

class Walker extends Node {
    async func walk() {
        var route = await Paths.find(map, 0, 99)   // the game keeps playing
    }
}
```

- `parallel` only goes with `static func`: such a function has no `self`.
- Its parameters and its result can only be **values**: numbers,
  `bool`, `string`, lists of values and nullable values. Never classes.
  It must have a result (`->`): that is all it does. It cannot be `async`:
  it is awaited already.
- Inside it, only its parameters, its local variables, the constants of
  classes, the methods of values (`abs`, `push`…) and other `parallel`
  functions (which run directly on the same thread) can be used: no
  objects are created, and no other method is called, not even of the
  engine (`Debug.print`).
- It is called with `await`, from an `async` method: the call runs on
  another thread and the coroutine continues in the first frame after the
  result is ready. Calling it without `await` is an error, since its result
  is its only effect.
- **Copies**: the arguments are copied when it starts and the result when it
  ends, so changing a list it received does not change the caller's.
- A runtime error inside it (an index out of range…) stops the game, as
  anywhere else, when its result is awaited.
- If the coroutine that waits for it ends first, the work finishes anyway
  and its result is dropped: a thread cannot be stopped halfway safely.
- **In Rust**, a `parallel` function is a plain static function. Awaited
  from a coroutine, the call becomes `parallel::run(arguments, work)` of
  the runtime: the arguments travel as copies (the `Plain` trait: lists as
  vectors), `work` is a function that receives them, calls it and sends
  the result back, on one of a few threads started the first time (one
  fewer than the cores, at least one). The coroutine checks each frame
  whether the result arrived. On those threads `fault` only stops the
  work, and the game's thread reports it when the result is awaited.

Blueprints have neither yet.

The code editor shows `async` and `parallel` before the methods that run
so, in its suggestions (`async (seconds: float)`) and in the type of the
name under the pointer (`async Time.wait(seconds: float)`).

Example: `cargo run -p star_script --example run --
crates/star_script/examples/scripts/coroutines.star` counts primes in a
`parallel` function while a node's coroutine keeps talking, and that
coroutine ends when the node leaves the tree.
