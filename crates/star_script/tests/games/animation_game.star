// A sprite that plays the clips of a `.frames` file: one that loops, then
// one that does not, whose end the sprite hears of. Clips move on with the
// time of each frame, so the game only prints what does not depend on it.
class Hero extends Sprite {
    var updates = 0
    var done = false

    init() {
        super.init("Hero", "levels/none.png")
        frames = "levels/hero.frames"
        play("wave")
    }

    override func update(delta: float) {
        updates += 1
        if updates == 2 {
            Debug.print(texture + " " + columns.to_string() + "x" + rows.to_string())
            Debug.print(animation() + " " + is_playing().to_string())
            play("hit")
        }
    }

    override func animation_finished(name: string) {
        Debug.print("finished " + name + " on " + frame.to_string() + " " + is_playing().to_string())
        done = true
    }
}

class AnimationGame extends Game {
    var hero = Hero()
    var frames_played = 0

    override func start() {
        add_child(hero)
    }

    override func update(delta: float) {
        frames_played += 1
        if hero.done {
            quit()
        } else if frames_played == 600 {
            Debug.print("the clip never ended")
            quit()
        }
    }
}
