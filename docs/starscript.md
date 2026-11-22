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
| 4 | Generación de Rust | **Definitivo** |
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
| `[T]` (lista) | `List<T>` del runtime (compartida) | `[]` |
| `T?` (anulable) | `Option<T>` | `null` |
| Clases | referencia compartida `Rc` (sección 4) | — (necesitan valor) |

Las listas se escriben `[int]`, no `Array<int>`, porque `<` es también la
comparación: `x as int < 3` sería ambiguo.

Los objetos y las listas se **comparten por referencia**, como en GDScript:
tras `var a = b`, ambos nombres son la misma lista, y `a.push(1)` también se
ve en `b`. Los números, `bool` y `string` se copian.

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
  la versión de la clase base. Los constructores no se heredan: cada clase
  declara el suyo si lo necesita.
- **Primero los campos propios, después `super.init(…)`** (como en Swift): el
  `init` de la base puede llamar a métodos que la subclase redefine, y esos
  métodos ya encuentran todos los campos con valor.
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

### Valores en ejecución

- `==` compara números, `bool`, `string` y listas **por contenido** (las
  listas, elemento a elemento); los objetos, **por identidad** (si son el
  mismo objeto). `null` solo es igual a `null`.
- Los enteros **dan la vuelta** al desbordarse (`9223372036854775807 + 1` da
  el mínimo), igual en debug y en release.
- `a / b` entre enteros redondea hacia cero (`-7 / 2` es `-3`) y `a % b` toma
  el signo de `a` (`-7 % 3` es `-1`). `float as int` corta los decimales y
  satura en los extremos.
- `for x in lista` recorre los elementos que había al empezar: cambiar la
  lista dentro del bucle no altera el recorrido.
- `float.to_string()` siempre muestra que es un `float`: `2.0`, no `2`.
- **Errores en ejecución**: leer fuera de una lista (`lista[5]` con dos
  elementos) o dividir un entero entre cero **detiene el juego** con un
  mensaje claro (`StarScript runtime error: index 5 is out of range for a
  list of length 2`). No se pueden detectar al compilar, y seguir con un
  valor inventado escondería el fallo.

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

## 4. Traducción a Rust (definitiva)

Para ver el Rust de un programa, o compilarlo y ejecutarlo:

```bash
cargo run -p star_script --example run -- --rust archivo.star   # muestra el Rust
cargo run -p star_script --example run -- archivo.star          # lo compila y ejecuta
```

Hasta que exista la API del motor (fase 5), un programa empieza en el
`static func main()` de una de sus clases (solo una puede tenerlo).

**Resultado**: un archivo Rust, raíz de su propio crate, que depende de
`star_script_runtime` (listas compartidas, campos, igualdad y errores en
ejecución). No lleva avisos de licencia: pertenece al autor del juego.

**Objetos**: referencias compartidas con conteo de referencias (`Rc`): un
objeto se libera cuando nadie lo usa, y acceder a él nunca falla. Dos objetos
que se referencian entre sí no se liberan; se resolverá con referencias
débiles. Los nodos del motor (fase 5) serán handles del motor, aparte.

**Cada clase** `Player` genera un módulo `classes::Player` con:

| Elemento | Qué es |
|----------|--------|
| `Fields` | Los campos que declara la clase, cada uno en un `Field<T>` del runtime. |
| `HasFields` | Trait que da acceso a esos campos en objetos de cualquier subclase. |
| `Object` | Trait con los métodos redefinibles; el de una subclase extiende el de su base. |
| `Ref` | `Rc<dyn Object>`: referencia a un objeto de la clase o de una subclase. |
| `Methods` | El código de cada método y las constantes (como funciones). |
| `Instance` | Los objetos creados como esa clase: los `Fields` de cada clase de su jerarquía. |
| `init_`, `new` | Construcción en dos fases; `Player(…)` llama a `new`. |

- **Herencia**: un `Player::Ref` se convierte en `Character::Ref` sin coste
  (los traits se extienden). `is` y `as` usan el trait `AnyObject`, con una
  conversión por clase.
- **Métodos**: los redefinibles se llaman por su trait, así cada objeto
  ejecuta su versión; `super.metodo()` y los métodos `private` llaman
  directamente al código de su clase.
- **Construcción**: `init_` recibe una función que crea el objeto final. La
  subclase prepara sus campos (sin `self`), llama al `init_` de su base, y
  cuando la clase raíz tiene los suyos se crea el objeto entero; después
  cada `init` ejecuta el resto de su cuerpo, de la raíz hacia abajo.
- **Campos**: leer un campo copia su valor y escribirlo lo reemplaza, sin
  dejar préstamos abiertos: el borrow checker nunca falla en ejecución.
- **Null safety**: `T?` es `Option<T>`. Donde el compilador demostró que un
  valor no es `null`, el Rust lo extrae sin comprobar de nuevo.
- **Nombres**: se conservan tal cual. Los que chocan con Rust se adaptan
  (`type` → `r#type`, `crate` → `crate_`) y los terminados en `_` reciben
  otro, así nunca coinciden con los nombres internos del generador (`this_`,
  `object_`, …).
- El Rust generado se compila con `cargo` como un crate más. El
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
