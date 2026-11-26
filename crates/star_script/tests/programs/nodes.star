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
        const shown = player.visible
        player.visible = false
        Debug.print("visible: " + shown.to_string() + " " + player.visible.to_string())

        // Colors are objects: sprites that share one change together.
        const shade = Color(1.0, 0.5, 0.25, 1.0)
        player.color = shade
        const bat = Sprite("Bat", "sprites/bat.png")
        bat.color = shade
        shade.a = 0.5
        Debug.print(player.color.g.to_string() + " " + bat.color.a.to_string())
        Debug.print(Sprite("Rock", "rock.png").color.r.to_string())

        // A sprite sheet: the image cut into cells, one of them shown.
        const runner = Sprite("Runner", "sprites/run.png")
        Debug.print(runner.columns.to_string() + " " + runner.rows.to_string() + " " + runner.frame.to_string())
        runner.columns = 4
        runner.rows = 2
        runner.frame += 5
        Debug.print(runner.columns.to_string() + " " + runner.rows.to_string() + " " + runner.frame.to_string())

        // Animations: clips of a `.frames` file, moved on by the game's frames.
        runner.frames = "sprites/run.frames"
        Debug.print(runner.frames + " [" + runner.animation() + "] " + runner.is_playing().to_string())
        runner.play("run")
        Debug.print(runner.animation() + " " + runner.is_playing().to_string())
        runner.stop()
        Debug.print(runner.animation() + " " + runner.is_playing().to_string())
        player.play("idle")
        Debug.print(player.animation())

        // Keyframe animations: an animator plays those of an `.anim` file on
        // its parent and the nodes under it, moved on by the game's frames.
        const animator = Animator("Animator")
        player.add_child(animator)
        animator.animations = "sprites/player.anim"
        Debug.print(animator.animations + " [" + animator.animation() + "] " + animator.is_playing().to_string())
        animator.play("wave")
        Debug.print(animator.animation() + " " + animator.is_playing().to_string())
        animator.stop()
        Debug.print(animator.animation() + " " + animator.is_playing().to_string() + " " + (animator.parent() == player).to_string())
        animator.remove()

        // Scenes are read by the engine: a program without a game has none.
        Debug.print((Scene.load("levels/one.scene") == null).to_string())

        // Cameras are nodes too.
        const eye = Camera2D("Eye")
        player.add_child(eye)
        eye.view_height *= 2.0
        Debug.print(eye.name + " " + eye.view_height.to_string() + " " + player.children().length().to_string())
    }
}
