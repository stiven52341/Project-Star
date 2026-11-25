// Translated from a blueprint: change the blueprint, not this code.
class Spinner3D extends Node3D {
    var speed = 1.5

    init() {
        super.init("Spinner3D")
    }

    override func update(delta: float) {
        yaw = yaw + (speed * delta)
    }
}

class Main {
    static func main() {
        const level = Node3D("Level")
        const spinner = Spinner3D()
        spinner.speed = 2.0
        level.add_child(spinner)
        for frame in 0..4 {
            spinner.update(0.25)
            Debug.print(spinner.yaw.to_string())
        }
    }
}
