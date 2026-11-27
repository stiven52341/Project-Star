// Sprites of the shapes the engine brings, whose bodies take their shape:
// a hidden floor of a stretched square, whose body is still there, and a
// ball of a circle that falls far from the middle and lands on it.
class Ball extends Sprite {
    init() {
        super.init("Ball", "@circle")
        x = 4.0
        y = 3.0
        color = Color(1.0, 0.5, 0.0, 1.0)
        // Its circle is 1 across: the body follows it, not this radius.
        body = Body.circle("dynamic", 0.1)
    }
}

class SquareGame extends Game {
    var frames = 0
    var ball = Ball()

    override func start() {
        const floor = Sprite("Floor", "@square")
        floor.width = 12.0
        floor.body = Body.rectangle("static", 1.0, 1.0)
        floor.visible = false
        add_child(floor)
        add_child(ball)
    }

    override func update(delta: float) {
        frames += 1
        const solid = ball.body
        if solid != null {
            for other in solid.just_touched() {
                // Its middle half a circle above the floor's top, at 0.5.
                Debug.print("landed on " + other.name + " " + (ball.y > 0.9).to_string())
                quit()
            }
        }
        if frames == 600 {
            Debug.print("never landed")
            quit()
        }
    }
}
