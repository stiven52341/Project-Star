// Finding nodes by name and by tag, without the engine.
class Bat extends Sprite {
    init() {
        super.init("Bat", "sprites/bat.png")
        tags.push("enemy")
    }
}

class Main {
    static func main() {
        const level = Node("Level")
        const cave = Node("Cave")
        const orc = Node("Orc")
        orc.tags.push("enemy")
        orc.tags.push("loud")
        level.add_child(cave)
        cave.add_child(Bat())
        cave.add_child(orc)
        level.add_child(Light2D("Torch", "point"))

        // Grandchildren too, unlike `find_child`.
        Debug.print((level.find_child("Orc") == null).to_string())
        const found = level.find_by_name("Orc")
        if found != null {
            Debug.print("found " + found.name + " " + found.has_tag("loud").to_string())
        }
        Debug.print((level.find_by_name("Nobody") == null).to_string())
        // Only descendants: not the node itself.
        Debug.print((cave.find_by_name("Cave") == null).to_string())

        var names = ""
        for enemy in level.find_by_tag("enemy") {
            names += enemy.name + " "
        }
        Debug.print(names)
        Debug.print(cave.find_by_tag("loud").length().to_string())

        // A console program has no game, so nothing is in it.
        Debug.print((Scene.find_by_name("Orc") == null).to_string())
        Debug.print(Scene.find_by_tag("enemy").length().to_string())
    }
}
