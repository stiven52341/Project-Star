// Translated from a blueprint: change the blueprint, not this code.
class Spinner2D extends Node {
    var speed = 1.5

    init() {
        super.init("Spinner2D")
    }

    override func update(delta: float) {
        rotation = rotation + (speed * delta)
    }
}

class Main {
    static func main() {
        const level = Node("Level")
        const spinner = Spinner2D()
        level.add_child(spinner)
        for frame in 0..4 {
            spinner.update(0.25)
            Debug.print(spinner.rotation.to_string())
        }
    }
}
