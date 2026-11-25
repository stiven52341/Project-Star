// Translated from a blueprint: change the blueprint, not this code.
class Mover2D extends Node {
    var velocity_x = 0.0
    var velocity_y = 1.0

    init() {
        super.init("Mover2D")
    }

    override func update(delta: float) {
        x = x + (velocity_x * delta)
        y = y + (velocity_y * delta)
    }
}

class Main {
    static func main() {
        const level = Node("Level")
        const bullet = Mover2D()
        bullet.velocity_x = -2.0
        level.add_child(bullet)
        for frame in 0..4 {
            bullet.update(0.25)
            Debug.print(bullet.x.to_string() + " " + bullet.y.to_string())
        }
    }
}
