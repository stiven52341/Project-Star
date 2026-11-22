// Nodes in a game: sprites with their own code, children that move with
// their parent, and nodes that are added and removed while playing.
// `cargo run -p star_script --example run -- crates/star_script/examples/scripts/nodes/space.star`
// The images are in the `assets` folder next to this file.

// Turns on itself; its children turn around it.
class Orbit extends Node {
    var speed: float

    init(turn_speed: float) {
        super.init("Orbit")
        speed = turn_speed
    }

    override func update(delta: float) {
        rotation += speed * delta
    }
}

// Moved with the arrow keys, WASD or a gamepad; a moon goes around it.
class Ship extends Sprite {
    const SPEED = 0.9
    const LIMIT = 0.85

    init() {
        super.init("Ship", "sprites/star.png")
        width = 0.2
        height = 0.2
        layer = 1
    }

    override func start() {
        const orbit = Orbit(3.0)
        const moon = Sprite("Moon", "sprites/disc.png")
        moon.x = 0.18
        moon.width = 0.06
        moon.height = 0.06
        orbit.add_child(moon)
        add_child(orbit)
    }

    override func update(delta: float) {
        if Input.is_down("left") { x -= SPEED * delta }
        if Input.is_down("right") { x += SPEED * delta }
        if Input.is_down("up") { y += SPEED * delta }
        if Input.is_down("down") { y -= SPEED * delta }
        x = x.max(-LIMIT).min(LIMIT)
        y = y.max(-LIMIT).min(LIMIT)
    }
}

// Falls and spins until it leaves the screen, then removes itself.
class Rock extends Sprite {
    var speed: float

    init(start_x: float, fall_speed: float) {
        super.init("Rock", "sprites/disc.png")
        speed = fall_speed
        x = start_x
        y = 1.2
        width = 0.12
        height = 0.12
    }

    override func update(delta: float) {
        y -= speed * delta
        if y < -1.2 {
            remove()
        }
    }
}

class Space extends Game {
    const ROCK_EVERY = 0.6
    var until_next_rock = 0.0
    var rocks = 0

    override func start() {
        Debug.print("Muévete con las flechas, WASD o la cruceta; sal con Escape")
        add_child(Ship())
    }

    override func update(delta: float) {
        until_next_rock -= delta
        if until_next_rock <= 0.0 {
            until_next_rock = ROCK_EVERY
            rocks += 1
            // Spread them over the width, without a random number generator.
            const x = ((rocks * 37) % 17) as float / 8.5 - 1.0
            add_child(Rock(x, 0.4 + (rocks % 3) as float * 0.2))
        }
        if Input.just_pressed("quit") {
            quit()
        }
    }
}
