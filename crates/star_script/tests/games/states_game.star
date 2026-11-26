// A scene whose animator runs a state machine: it starts at rest, a
// trigger does not attack until a flag says the guard is ready, the attack
// calls a method and ends, and the machine goes back to rest by itself.
// The game prints what does not depend on the time of each frame.
class Guard extends Sprite {
    init() {
        super.init("Guard", "levels/none.png")
    }

    func strike() {
        Debug.print("strike")
    }
}

class GuardAnimator extends Animator {
    var finished = false

    init() {
        super.init("Animator")
    }

    override func start() {
        Debug.print("states " + states + ", starting in '" + state() + "'")
    }

    override func animation_finished(name: string) {
        var rotation = -1.0
        const owner = parent()
        if owner != null {
            const arm = owner.find_child("Arm")
            if arm != null {
                rotation = arm.rotation
            }
        }
        Debug.print("finished " + name + " in " + state() + " at " + rotation.to_string())
        finished = true
    }
}

class StatesGame extends Game {
    var animator: GuardAnimator? = null
    var frames_played = 0

    override func start() {
        const guard = Scene.load("levels/guard.scene")
        if guard != null {
            add_child(guard)
            const found = guard.find_child("Animator")
            if found != null {
                animator = found as GuardAnimator
            }
        }
    }

    override func update(delta: float) {
        frames_played += 1
        const guard = animator
        if guard == null {
            Debug.print("no animator")
            quit()
        } else if frames_played == 2 {
            Debug.print("state " + guard.state())
            guard.trigger("attack")
        } else if frames_played == 3 {
            Debug.print("state " + guard.state() + ", not ready")
            guard.set_flag("ready", true)
            guard.trigger("attack")
        } else if frames_played == 4 {
            Debug.print("state " + guard.state() + ", playing " + guard.animation())
        } else if guard.finished && guard.state() == "idle" {
            Debug.print("back to " + guard.state())
            quit()
        } else if frames_played == 600 {
            Debug.print("the machine never went back")
            quit()
        }
    }
}
