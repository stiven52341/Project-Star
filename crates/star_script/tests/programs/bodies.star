// Bodies without the engine: they are made, changed and pushed, but nothing
// simulates them, so they never move or touch anything.
class Ball extends Sprite {
    init() {
        super.init("Ball", "sprites/ball.png")
        body = Body.circle("dynamic", 0.25)
    }
}

class Main {
    static func main() {
        const ball = Ball()
        const solid = ball.body
        if solid != null {
            Debug.print(solid.kind + " " + solid.shape + " " + solid.radius.to_string())
            Debug.print(solid.friction.to_string() + " " + solid.density.to_string() + " " + solid.sensor.to_string())
            solid.bounciness = 0.75
            solid.velocity_x = 3.0
            solid.apply_impulse(0.0, 2.0)
            Debug.print(solid.bounciness.to_string() + " " + solid.velocity_x.to_string())
            Debug.print(solid.touching().length().to_string() + " " + solid.just_touched().is_empty().to_string())
        }

        const floor = Node("Floor")
        floor.body = Body.rectangle("static", 10.0, 1.0)
        Debug.print((floor.body?.width ?? 0.0).to_string())
        const hero = Node("Hero")
        hero.body = Body.capsule("kinematic", 2.0, 0.5)
        Debug.print((hero.body?.height ?? 0.0).to_string() + " " + (hero.body?.shape ?? ""))

        // A node without a body is not a body.
        hero.body = null
        Debug.print((hero.body == null).to_string())
    }
}
