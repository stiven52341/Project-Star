// A skinned model whose animator plays an animation of the model's own
// file: the clips of a `.glb` are played as those of an `.anim` (they
// loop), and the engine poses the model's skeleton with them. The game
// prints what does not depend on the time of each frame.
class Hero extends Model {
    init() {
        super.init("Hero", "levels/wave.glb")
    }
}

class ModelGame extends Game {
    var animator: Animator? = null
    var frames_played = 0

    override func start() {
        const hero = Hero()
        const waving = Animator("Animator")
        waving.animations = "levels/wave.glb"
        hero.add_child(waving)
        add_child(hero)
        waving.play("wave")
        animator = waving
        Debug.print("playing " + waving.animation())
    }

    override func update(delta: float) {
        frames_played += 1
        const waving = animator
        if waving == null {
            quit()
        } else if frames_played == 30 {
            Debug.print("still " + waving.animation() + " " + waving.is_playing().to_string())
            quit()
        }
    }
}
