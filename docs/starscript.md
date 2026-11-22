# StarScript

Lenguaje de programación de juegos de ProjectStar. Oculta la complejidad de
Rust (lifetimes, préstamos, traits) detrás de un lenguaje **100 % orientado a
objetos**, con **tipado estático e inferencia**, inspirado en la ergonomía de
GDScript pero con bloques entre llaves.

Extensión de archivo: **`.star`**.

## Flujo de compilación

```
Blueprint (.blueprint) ──► StarScript (.star) ──► Rust (.rs) ──► código máquina
   editor visual             texto                 generado       cargo / rustc
```

- Cada etapa produce la entrada de la siguiente; ninguna se salta.
- Un juego puede mezclar las tres formas: blueprints, StarScript y **Rust puro**.
- El Rust generado es legible, pero es un artefacto: no se edita a mano.

## Principios

1. **Todo es un objeto**: el código vive en clases. No hay funciones sueltas.
   Los tipos básicos también tienen métodos (`3.abs()`, `"hola".length()`).
2. **Errores al compilar, no al jugar**: el tipado estático detecta los fallos
   antes de ejecutar. El compilador indica archivo, línea y columna.
3. **Nunca se ve Rust**: ni lifetimes, ni `&mut`, ni `Rc<RefCell<…>>`. Los
   mensajes de error hablan de StarScript, no del código generado.
4. **Rendimiento de Rust**: los tipos estáticos permiten generar Rust directo,
   sin un tipo dinámico (`Variant`) intermedio.

## Estado por fases

| Fase | Contenido | Estado |
|------|-----------|--------|
| 1 | Léxico (lexer) | **Definitivo** |
| 2 | Sintaxis (parser y AST) | **Definitivo** |
| 3 | Tipos, null safety y semántica | **Definitivo** |
| 4 | Generación de Rust | Borrador |
| 5 | API del motor | Pendiente |
| 6 | Integración en el editor | Pendiente |
| 7–8 | Blueprints | Borrador |

## 1. Léxico (definitivo)

### Archivo fuente

- Texto **UTF-8**. Se ignora una marca BOM inicial.
- Los finales de línea pueden ser `\n` o `\r\n`.

### Comentarios

```
// Comentario hasta el final de la línea.
/* Comentario de bloque, /* anidable */ y de varias líneas. */
```

### Saltos de línea

Un salto de línea **termina la sentencia**, así que el `;` es opcional (como
en Kotlin o Swift). `;` permite escribir varias sentencias en una línea.

- Varios saltos seguidos cuentan como uno.
- Dentro de `( … )` y `[ … ]` los saltos se ignoran, así que una llamada larga
  puede repartirse en varias líneas. Dentro de un `{ … }` vuelven a contar,
  aunque ese bloque esté entre paréntesis.

### Identificadores

`[A-Za-z_][A-Za-z0-9_]*`. Solo ASCII por ahora: `año` da un error claro. Así
cualquier nombre se traduce a un identificador válido de Rust.

Convención: `PascalCase` para clases, `snake_case` para variables y métodos,
`SCREAMING_SNAKE_CASE` para constantes.

### Palabras clave

| Uso | Palabras |
|-----|----------|
| Clases | `class` `extends` `init` `self` `super` `override` `private` `static` |
| Declaraciones | `var` `const` `func` |
| Control de flujo | `if` `else` `while` `for` `in` `break` `continue` `return` |
| Tipos | `as` `is` |
| Valores | `true` `false` `null` |
| Reservadas (futuro) | `interface` `implements` `abstract` `enum` `match` `signal` |

Las reservadas aún no hacen nada, pero no pueden usarse como nombres. Así, al
activarlas no se romperán programas existentes.

Los tipos básicos (`int`, `float`, `bool`, `string`) **no** son palabras
clave: son nombres de clases predefinidas.

### Literales

| Tipo | Ejemplos | Notas |
|------|----------|-------|
| Entero | `42` `1_000_000` `0xFF` `0b1010` | 64 bits con signo. `_` separa dígitos |
| Decimal | `3.14` `0.5` `1e3` `2.5e-3` | Necesita un dígito antes del punto: `.5` no vale |
| Texto | `"Hola"` `"línea\n"` `"\u{1F680}"` | Una sola línea. Escapes: `\n \t \r \\ \" \0 \u{…}` |
| Lógico | `true` `false` | |

`0..10` es un rango (`0`, `..`, `10`), no un decimal. Un número pegado a
letras (`10px`) es un error.

### Operadores y signos

| Categoría | Signos |
|-----------|--------|
| Aritméticos | `+` `-` `*` `/` `%` |
| Comparación | `==` `!=` `<` `<=` `>` `>=` |
| Lógicos | `&&` `\|\|` `!` |
| Asignación | `=` `+=` `-=` `*=` `/=` `%=` |
| Null safety | `?` (tipo anulable) `?.` (acceso seguro) `??` (valor por defecto) |
| Otros | `.` `..` `,` `:` `;` `->` `( )` `{ }` `[ ]` |

## 2. Sintaxis (definitiva)

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

- **Archivo**: una o más clases.
- **Clase**: `class Nombre [extends Base] { miembros }`. Herencia simple.
- **Miembros**:
  - `var` (campo mutable) y `const` (constante de clase).
  - `func` (método); el tipo de retorno va tras `->` y se omite si no devuelve nada.
  - `init` (constructor).
  - Modificadores: `private`, `static` y `override` (obligatorio al redefinir
    un método de la base).
- **Sentencias**:
  - `var`/`const` locales.
  - Asignación (`=`, `+=`, …).
  - `if`/`else if`/`else`, `while`, `for x in 0..10` / `for x in lista`.
  - `break`, `continue`, `return`, y expresiones (llamadas).
- **La asignación es una sentencia, no una expresión**: `if x = 3` es un error,
  no un fallo silencioso.
- **`self` es implícito**: `speed` equivale a `self.speed` salvo que una
  variable local lo oculte.
- **Crear objetos**: `Player(250.0)` llama al constructor; no hay `new`.
- **Listas**: `[1, 2, 3]` y `lista[i]`. Su tipo se define en la fase 3.
- **Comas finales** permitidas en parámetros, argumentos y listas: `f(a, b,)`.

Reglas de líneas y llaves:

- Las sentencias de una misma línea se separan con `;`: `a(); b()`. Sin él,
  `a() b()` es un error.
- `if`, `while` y `for` terminan en su bloque y no necesitan separador.
- La `{` puede ir en la misma línea o en la siguiente (estilo Allman), y
  `else` puede empezar en la línea siguiente a la `}`.
- Un operador al **final** de una línea continúa la expresión en la
  siguiente: `total = a +` ⏎ `b`. Al principio de la línea no, porque la
  línea anterior ya terminó la sentencia.
- Las comparaciones, `is` y los rangos **no se encadenan**: `0 < x < 10` es
  un error; se escribe `0 < x && x < 10`. Operadores de niveles distintos sí
  se combinan: `a < b == c`.
- El anidamiento tiene un límite de 128 niveles, para que un archivo absurdo
  dé un error y no agote la pila.
- Qué modificadores admite cada miembro (p. ej., `override` en un campo) se
  comprueba en la fase 3; el parser solo rechaza los repetidos.

Para ver cómo entiende el parser un archivo:

```bash
cargo run -p star_script --example parse -- archivo.star
```

Muestra el árbol con las expresiones totalmente entre paréntesis, p. ej.
`(+= (. position x) (* speed delta))`.

Precedencia de operadores, de menor a mayor:

| Nivel | Operadores |
|-------|------------|
| 1 | `\|\|` |
| 2 | `&&` |
| 3 | `==` `!=` |
| 4 | `<` `<=` `>` `>=` `is` |
| 5 | `??` (agrupa a la derecha: `a ?? b ?? c` = `a ?? (b ?? c)`) |
| 6 | `..` |
| 7 | `+` `-` |
| 8 | `*` `/` `%` |
| 9 | `as` |
| 10 | unarios `-` `!` |
| 11 | llamada `f()`, miembro `a.b` / `a?.b`, índice `a[i]` |

## 3. Tipos y semántica (definitiva)

Para ver qué tipos deduce el compilador, o sus errores:

```bash
cargo run -p star_script --example check -- archivo.star
```

### Tipos

| StarScript | Rust | Valor por defecto |
|------------|------|-------------------|
| `int` | `i64` | `0` |
| `float` | `f32` (igual que el motor y la GPU) | `0.0` |
| `bool` | `bool` | `false` |
| `string` | `String` | `""` |
| `[T]` (lista) | `Vec<T>` | `[]` |
| `T?` (anulable) | `Option<T>` | `null` |
| Clases | handle del motor (sección 4) | — (necesitan valor) |

Las listas se escriben `[int]`, no `Array<int>`, porque `<` es también la
comparación: `x as int < 3` sería ambiguo.

### Inferencia

- `var x = 3` deduce `int`; `var x = [1, 2]` deduce `[int]`; `var e = [a, null]`
  deduce `[Enemy?]`.
- `null` solo y `[]` vacío no dicen su tipo: hay que escribirlo
  (`var target: Enemy? = null`, `var names: [string] = []`).
- Un literal entero se convierte en `float` donde hace falta uno:
  `var speed: float = 2`, `speed * 2` y `2 * speed` son válidos.
- Fuera de eso, `int` y `float` **no se mezclan**: `speed + lives` es un
  error; se escribe `speed + lives as float`. El error lo sugiere.
- `+` une textos: `"Vidas: " + lives.to_string()`.

### Null safety

Un programa StarScript **nunca falla por un `null` inesperado**: el compilador
lo impide.

- `Enemy` nunca es `null`. Solo `Enemy?` puede serlo, y solo a él se le puede
  asignar `null`.
- Con un `Enemy?` no se puede usar `.`. Hay tres alternativas:
  - `target?.hit()`: si `target` es `null`, no hace nada. `target?.health`
    es un `int?`.
  - `target?.name ?? "nadie"`: el valor, o el de la derecha si es `null`.
  - Comprobarlo primero; dentro del `if`, el compilador sabe que no es `null`:

    ```
    const enemy = target
    if enemy == null { return }
    enemy.hit()          // válido: aquí enemy es Enemy
    ```
- El estrechamiento funciona con `!= null`, `== null` seguido de
  `return`/`break`/`continue`, `&&`, `||`, `!` e `is`. Solo se aplica a lo
  que **no puede cambiar**: constantes locales (`const`), parámetros y
  variables de `for`. Un `var` o un campo podrían volver a ser `null` entre
  la comprobación y el uso, así que se copian antes en un `const`. El
  mensaje de error lo explica.
- No existe un operador «confía en mí» (`!!`): abriría la puerta a fallos en
  ejecución.
- Usar `?.`, `??` o `== null` con algo que nunca es `null` es un error: ese
  código sobra y suele esconder una confusión.

### Clases y objetos

- **Campos**: `var nombre: Tipo = valor`. Si falta el tipo se deduce del
  valor; si falta el valor se usa el valor por defecto del tipo. Los campos
  de tipo clase no anulable no tienen valor por defecto: o tienen `= valor`,
  o `init` los asigna.
- **Construcción segura**: dentro de `init`, `self` (y por tanto los campos y
  métodos) no se puede usar hasta que el objeto está completo, es decir,
  hasta que se han asignado los campos pendientes y se ha llamado a
  `super.init(…)` si hace falta. Esas asignaciones deben estar directamente
  en el cuerpo de `init`, no dentro de un `if`. Así nunca existe un objeto a
  medio construir.
- **Herencia**: si el `init` de la clase base tiene parámetros, el `init` de
  la subclase debe llamar a `super.init(…)` una vez. `super.metodo()` llama a
  la versión de la clase base.
- **Redefinir métodos**: requiere `override` y los mismos parámetros y tipo de
  retorno. Los campos y constantes no pueden repetir nombres de la base.
- **Valores de campos y constantes**: no pueden usar `self` ni otros campos.
  Las constantes solo admiten literales, operadores, `as` y otras constantes
  declaradas antes.
- **`private`**: solo se usa dentro de su clase (otros objetos de la misma
  clase incluidos).
- **`static func`**: pertenece a la clase y se llama como `Clase.metodo()`; no
  tiene `self`. Los campos `static var` aún no existen.
- **`is` y `as`**: `x is Player` comprueba la clase. `x as Player` hacia una
  subclase puede fallar, así que da un `Player?`. Entre `int` y `float`, `as`
  convierte.
- **Crear objetos**: `Player(…)` con los argumentos de su `init`. Un
  `private init` solo se puede usar dentro de la clase.

### Reglas de métodos y sentencias

- Un método con `->` debe devolver un valor en **todos** los caminos.
- Solo las llamadas pueden ser sentencias: `a + 1` suelto es un error.
- Los parámetros, las constantes y la variable de un `for` no se pueden
  reasignar.
- Una variable local no puede llamarse como otra todavía visible. Sí puede
  llamarse como un campo: `self.speed = speed`.
- Los rangos `a..b` solo existen en `for`.

### Métodos predefinidos

| Tipo | Métodos |
|------|---------|
| `int` | `abs()` `min(x)` `max(x)` `to_string()` |
| `float` | `abs()` `min(x)` `max(x)` `floor()` `ceil()` `round()` `sqrt()` `to_string()` |
| `bool` | `to_string()` |
| `string` | `length()` `is_empty()` `contains(s)` `to_upper()` `to_lower()` `to_string()` |
| `[T]` | `length()` `is_empty()` `contains(x)` `push(x)` `pop() -> T?` `clear()` |

Clase nativa `Debug`: `Debug.print(mensaje: string)`. No hay funciones
globales: todo es un método de alguna clase.

## 4. Traducción a Rust (borrador)

- Cada clase se traduce a un `struct` de Rust con sus campos. La herencia se
  traduce a **composición**: el struct contiene a su base. Los métodos
  redefinibles se despachan mediante traits.
- Los objetos se referencian con **handles** que gestiona el motor (índices
  con generación), no con referencias de Rust. Así el usuario puede guardar
  referencias a otros objetos sin chocar con el borrow checker. Usar un objeto
  destruido da un error claro, no un fallo de memoria.
- Los nombres que choquen con palabras clave de Rust se generan como
  identificadores crudos (`r#type`).
- El Rust generado se compila con `cargo` como un crate más del proyecto. El
  desarrollador necesita la toolchain de Rust instalada.

## 5. Blueprints (borrador)

Grafos de nodos (eventos, llamadas, variables, flujo) guardados en
`.blueprint`. El editor los traduce a StarScript, así que todo lo que hace un
blueprint puede leerse, y depurarse, como código. Un blueprint es una clase:
puede heredar de otra clase y otras clases pueden usarlo.

## 6. Rust puro

Un juego puede escribir clases en Rust implementando los traits del motor, y
StarScript puede usarlas como cualquier otra clase. El mecanismo concreto se
definirá en la fase 5.
