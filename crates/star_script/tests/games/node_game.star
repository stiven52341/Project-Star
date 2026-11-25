class Ship extends Sprite {
    init() {
        super.init("Ship", "sprites/ship.png")
        color = Color(1.0, 0.5, 0.5, 1.0)
    }

    override func start() {
        Debug.print("ship start")
    }

    override func update(delta: float) {
        x += 1.0
        Debug.print("ship x " + x.to_string())
    }
}

class Drone extends Node {
    export var speed = 1.0
    export var label = "none"
    export var tint = Color(1.0, 1.0, 1.0, 1.0)

    init() {
        super.init("Drone")
    }

    override func start() {
        Debug.print("drone start " + name + " " + x.to_string() + " " + speed.to_string() + " " + label + " " + tint.r.to_string())
    }
}

class NodeGame extends Game {
    var frames = 0

    override func start() {
        background = Color(0.0, 0.0, 0.1, 1.0)
        ambient_2d = Color(0.2, 0.2, 0.3, 1.0)
        add_child(Light2D("Lamp", "point"))
        const fleet = Scene.load("levels/fleet.scene")
        if fleet != null {
            add_child(fleet)
            Debug.print(fleet.name + " " + fleet.children().length().to_string())
            Debug.print(Scene.find_by_tag("ship").length().to_string() + " " + (Scene.find_by_name("Wingman") != null).to_string())
            const leader = fleet.find_child("Leader")
            if leader is Sprite {
                Debug.print(leader.texture + " " + leader.x.to_string() + " " + leader.color.g.to_string())
            }
            const torch = fleet.find_child("Torch")
            if torch is Light2D {
                Debug.print(torch.kind + " " + torch.range.to_string() + " " + torch.angle.to_string())
            }
        }
        Debug.print((Scene.load("levels/missing.scene") == null).to_string())
        Debug.print((Scene.load("levels/misspelled.scene") == null).to_string())
        const ship = Ship()
        add_child(ship)
        // Not `current`: the scene's camera is the one the game is seen from.
        const eye = Camera2D("Eye")
        ship.add_child(eye)
    }

    override func update(delta: float) {
        frames += 1
        Debug.print("game frame " + frames.to_string())
        const view = camera
        if view != null {
            Debug.print("camera " + view.name + " " + view.view_height.to_string())
        }
        if frames == 3 {
            quit()
        }
    }
}
