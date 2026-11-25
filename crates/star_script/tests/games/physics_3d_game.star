// A ball that falls in 3D onto a floor of a scene, seen from a 3D camera
// and lit by the sun.
class Ball extends Mesh {
    init() {
        super.init("Ball", "sphere")
        color = Color(0.5, 0.7, 1.0, 1.0)
        body = Body.sphere("dynamic", 0.5)
    }
}

class Physics3dGame extends Game {
    var frames = 0
    var ball = Ball()

    override func start() {
        const floor = Scene.load("levels/floor_3d.scene")
        if floor != null {
            add_child(floor)
            const solid = floor.body
            if solid != null {
                Debug.print(solid.shape + " " + solid.depth.to_string() + " " + (floor is Mesh).to_string())
            }
            const spot = floor.find_child("Spot")
            if spot is Light {
                Debug.print(spot.kind + " " + spot.angle.to_string() + " " + spot.blend.to_string())
            }
        }
        const eye = Camera3D("Eye")
        eye.y = 2.0
        eye.z = 8.0
        eye.look_at(0.0, 0.0, 0.0)
        camera_3d = eye
        add_child(eye)
        const sun = Light("Sun", "directional")
        sun.pitch = -1.0
        add_child(sun)
        ambient = Color(0.2, 0.2, 0.2, 1.0)
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
