// Nodes in a game: sprites with their own code, children that move with
// their parent, nodes that are added and removed while playing, colors, a
// camera that follows the ship and a sky loaded from a scene file.
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
    const LIMIT_X = 1.8
    const LIMIT_Y = 0.9

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
        moon.color = Color(0.6, 0.8, 1.0, 1.0)
        orbit.add_child(moon)
        add_child(orbit)
    }

    override func update(delta: float) {
        if Input.is_down("left") { x -= SPEED * delta }
        if Input.is_down("right") { x += SPEED * delta }
        if Input.is_down("up") { y += SPEED * delta }
        if Input.is_down("down") { y -= SPEED * delta }
        x = x.max(-LIMIT_X).min(LIMIT_X)
        y = y.max(-LIMIT_Y).min(LIMIT_Y)
    }
}

// Falls and spins until it leaves the screen, then removes itself.
class Rock extends Sprite {
    var speed: float

    init(start_x: float, fall_speed: float) {
        super.init("Rock", "sprites/disc.png")
        speed = fall_speed
        x = start_x
        y = 1.4
        width = 0.12
        height = 0.12
        // Faster rocks are redder.
        color = Color(1.0, 1.2 - fall_speed, 0.3, 1.0)
    }

    override func update(delta: float) {
        y -= speed * delta
        if y < -1.4 {
            remove()
        }
    }
}

// Shows a wider world than the window: it glides after the ship from side
// to side, without following it up and down.
class FollowCamera extends Camera {
    const CATCH_UP = 3.0
    var ship: Node

    init(target: Node) {
        ship = target
        super.init("Camera")
        view_height = 2.4
    }

    override func update(delta: float) {
        x += (ship.x - x) * (CATCH_UP * delta).min(1.0)
    }
}

class Space extends Game {
    const ROCK_EVERY = 0.6
    var until_next_rock = 0.0
    var rocks = 0

    override func start() {
        Debug.print("Muévete con las flechas, WASD o la cruceta; sal con Escape")
        background = Color(0.02, 0.02, 0.08, 1.0)
        const sky = Scene.load("scenes/sky.scene")
        if sky != null {
            add_child(sky)
        }
        const ship = Ship()
        add_child(ship)
        const eye = FollowCamera(ship)
        add_child(eye)
        camera = eye
    }

    override func update(delta: float) {
        until_next_rock -= delta
        if until_next_rock <= 0.0 {
            until_next_rock = ROCK_EVERY
            rocks += 1
            // Spread them over the width, without a random number generator.
            const x = ((rocks * 37) % 17) as float / 4.0 - 2.0
            add_child(Rock(x, 0.4 + (rocks % 3) as float * 0.2))
        }
        if Input.just_pressed("quit") {
            quit()
        }
    }
}
