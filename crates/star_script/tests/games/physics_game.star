// A ball that falls on a floor loaded from a scene, and closes the game
// when it lands.
class Ball extends Node {
    init() {
        super.init("Ball")
        body = Body.circle("dynamic", 0.5)
    }
}

class PhysicsGame extends Game {
    var frames = 0
    var ball = Ball()

    override func start() {
        const floor = Scene.load("levels/floor.scene")
        if floor != null {
            add_child(floor)
            const solid = floor.body
            if solid != null {
                Debug.print(solid.kind + " " + solid.shape + " " + solid.width.to_string())
            }
        }
        add_child(ball)
    }

    override func update(delta: float) {
        frames += 1
        const solid = ball.body
        if solid != null {
            for other in solid.just_touched() {
                Debug.print("landed on " + other.name + " " + (ball.y < 0.0).to_string())
                quit()
            }
        }
        if frames == 600 {
            Debug.print("never landed")
            quit()
        }
    }
}
