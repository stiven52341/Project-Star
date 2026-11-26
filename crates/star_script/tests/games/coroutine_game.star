// A node whose coroutine prints every frame until it leaves the tree, and
// a coroutine of the game that waits for a frame; every frame checks that
// `Time.delta()` is the `delta` of `update`.
class Blinker extends Node {
    init() {
        super.init("Blinker")
    }

    async func blink() {
        while true {
            Debug.print("blink")
            await Time.next_frame()
        }
    }

    override func start() {
        blink()
    }
}

class CoroutineGame extends Game {
    var frames = 0
    var blinker = Blinker()

    override func start() {
        add_child(blinker)
        later()
    }

    async func later() {
        await Time.next_frame()
        Debug.print("later")
    }

    override func update(delta: float) {
        frames += 1
        Debug.print("frame " + frames.to_string() + " " + (Time.delta() == delta).to_string())
        if frames == 3 {
            blinker.remove()
        }
        if frames == 5 {
            quit()
        }
    }
}
