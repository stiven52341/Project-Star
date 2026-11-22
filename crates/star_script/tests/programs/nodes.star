// Nodes without the engine: building a tree, moving and finding nodes.
class Player extends Sprite {
    var lives = 3

    init() {
        super.init("Player", "sprites/hero.png")
        width = 2.0
    }

    func describe() -> string {
        return name + " " + texture + " " + width.to_string() + " " + lives.to_string()
    }
}

class Main {
    static func main() {
        const level = Node("Level")
        const enemies = Node("Enemies")
        const player = Player()
        level.add_child(enemies)
        level.add_child(player)
        enemies.add_child(Sprite("Bat", "sprites/bat.png"))
        Debug.print(player.describe())
        Debug.print("children: " + level.children().length().to_string())

        const found = level.find_child("Player")
        if found is Player {
            Debug.print("found the player")
        }
        Debug.print((level.find_child("Nobody") == null).to_string())

        const parent = player.parent()
        if parent != null {
            Debug.print("parent: " + parent.name)
        }

        // Moving a node takes it from its old parent.
        enemies.add_child(player)
        Debug.print("level: " + level.children().length().to_string())
        Debug.print("enemies: " + enemies.children().length().to_string())

        // A node cannot go under its own children: nothing changes.
        player.add_child(level)
        Debug.print((level.parent() == null).to_string())

        player.remove()
        Debug.print((player.parent() == null).to_string())
        Debug.print("enemies: " + enemies.children().length().to_string())

        player.x += 1.5
        player.scale_x = 2.0
        Debug.print(player.x.to_string() + " " + player.scale_x.to_string() + " " + player.y.to_string())

        // Colors are objects: sprites that share one change together.
        const shade = Color(1.0, 0.5, 0.25, 1.0)
        player.color = shade
        const bat = Sprite("Bat", "sprites/bat.png")
        bat.color = shade
        shade.a = 0.5
        Debug.print(player.color.g.to_string() + " " + bat.color.a.to_string())
        Debug.print(Sprite("Rock", "rock.png").color.r.to_string())
    }
}
