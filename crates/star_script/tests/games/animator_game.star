// A scene whose animator plays an animation of an `.anim` file from the
// start: it turns and tints a child, counts an `export` field up and hides
// its parent. Animations move on with the time of each frame, so the game
// only prints where the animation ends.
class Knight extends Sprite {
    export var power = 0

    init() {
        super.init("Knight", "levels/none.png")
    }
}

class KnightAnimator extends Animator {
    var done = false

    init() {
        super.init("Animator")
    }

    override func start() {
        Debug.print("playing " + animation() + " " + is_playing().to_string())
    }

    override func animation_finished(name: string) {
        Debug.print("finished " + name + " " + is_playing().to_string())
        const owner = parent()
        if owner != null {
            const knight = owner as Knight
            if knight != null {
                Debug.print(knight.power.to_string() + " " + knight.visible.to_string())
            }
            const found = owner.find_child("Arm")
            if found != null {
                const arm = found as Sprite
                if arm != null {
                    Debug.print(arm.rotation.to_string() + " " + arm.color.g.to_string())
                }
            }
        }
        done = true
    }
}

class AnimatorGame extends Game {
    var animator: KnightAnimator? = null
    var frames_played = 0

    override func start() {
        const knight = Scene.load("levels/knight.scene")
        if knight != null {
            add_child(knight)
            const found = knight.find_child("Animator")
            if found != null {
                animator = found as KnightAnimator
            }
        }
    }

    override func update(delta: float) {
        frames_played += 1
        const playing = animator
        if playing != null && playing.done {
            quit()
        } else if frames_played == 600 {
            Debug.print("the animation never ended")
            quit()
        }
    }
}
