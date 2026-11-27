// Bodies made of the triangles of their meshes: a cube stretched into a
// wide floor, which holds a ball that falls far beyond the cube's own
// size, and a crate that falls on its convex hull.
class Ball extends Mesh {
    init() {
        super.init("Ball", "sphere")
        x = 3.0
        y = 2.0
        body = Body.sphere("dynamic", 0.5)
    }
}

class Crate extends Mesh {
    init() {
        super.init("Crate", "cube")
        x = -2.0
        y = 4.0
        body = Body.mesh("dynamic")
    }
}

class MeshGame extends Game {
    var frames = 0
    var ball = Ball()
    var crate = Crate()
    var landed = 0

    override func start() {
        const floor = Mesh("Floor", "cube")
        floor.scale_x = 10.0
        floor.scale_z = 10.0
        floor.body = Body.mesh("static")
        add_child(floor)
        add_child(ball)
        add_child(crate)
        const eye = Camera3D("Eye")
        eye.y = 3.0
        eye.z = 10.0
        eye.look_at(0.0, 0.0, 0.0)
        camera_3d = eye
        add_child(eye)
    }

    override func update(delta: float) {
        frames += 1
        report(ball)
        report(crate)
        if landed == 2 {
            quit()
        }
        if frames == 600 {
            Debug.print("never landed")
            quit()
        }
    }

    func report(falling: Mesh) {
        const solid = falling.body
        if solid != null {
            for other in solid.just_touched() {
                Debug.print(falling.name + " landed on " + other.name + " " + (falling.y > 0.0).to_string())
                landed += 1
            }
        }
    }
}
