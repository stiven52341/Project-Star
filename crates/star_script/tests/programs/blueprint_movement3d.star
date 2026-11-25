// Translated from a blueprint: change the blueprint, not this code.
class Movement3D extends Node3D {
    var speed = 3.0

    init() {
        super.init("Movement3D")
    }

    override func update(delta: float) {
        x = x + (Input.axis("left", "right") * (speed * delta))
        z = z + (Input.axis("up", "down") * (speed * delta))
    }
}

// Without a window no action is held, so the axes give 0.
class Main {
    static func main() {
        const level = Node3D("Level")
        const player = Movement3D()
        level.add_child(player)
        for frame in 0..4 {
            player.update(0.25)
        }
        Debug.print(player.name + " " + player.x.to_string() + " " + player.z.to_string() + " " + player.speed.to_string())
    }
}
