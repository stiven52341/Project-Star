// Translated from a blueprint: change the blueprint, not this code.
class Movement2D extends Node {
    var speed = 3.0

    init() {
        super.init("Movement2D")
    }

    override func update(delta: float) {
        x = x + (Input.axis("left", "right") * (speed * delta))
        y = y + (Input.axis("down", "up") * (speed * delta))
    }
}

// Without a window no action is held, so the axes give 0.
class Main {
    static func main() {
        const level = Node("Level")
        const player = Movement2D()
        level.add_child(player)
        for frame in 0..4 {
            player.update(0.25)
        }
        Debug.print(player.name + " " + player.x.to_string() + " " + player.y.to_string() + " " + player.speed.to_string())
    }
}
