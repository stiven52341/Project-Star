// Input in a game: the actions are declared in the `project.toml` of this folder.
// `cargo run -p star_script --example run -- crates/star_script/examples/scripts/input/jumper.star`
class Jumper extends Game {
    var jumps = 0

    override func start() {
        Debug.print("Salta con Espacio, el clic, un toque o el botón A; sal con Escape")
    }

    override func update(delta: float) {
        if Input.just_pressed("jump") {
            jumps += 1
            Debug.print("¡Salto " + jumps.to_string() + "!")
        }
        if Input.just_released("jump") {
            Debug.print("Aterrizaje")
        }
        if Input.just_pressed("quit") {
            quit()
        }
    }
}
