// Translated from a blueprint: change the blueprint, not this code.
class Mover3D extends Node3D {
    var velocity_x = 0.0
    var velocity_y = 0.0
    var velocity_z = -1.0

    init() {
        super.init("Mover3D")
    }

    override func update(delta: float) {
        x = x + (velocity_x * delta)
        y = y + (velocity_y * delta)
        z = z + (velocity_z * delta)
    }
}

class Main {
    static func main() {
        const level = Node3D("Level")
        const rocket = Mover3D()
        rocket.velocity_y = 4.0
        level.add_child(rocket)
        for frame in 0..4 {
            rocket.update(0.25)
            Debug.print(rocket.x.to_string() + " " + rocket.y.to_string() + " " + rocket.z.to_string())
        }
    }
}
