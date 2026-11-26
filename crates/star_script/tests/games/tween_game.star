// Tweens in a game: one awaited until its node arrives, and another whose
// node leaves the tree, which ends it where it was.
class TweenGame extends Game {
    var mover = Node("Mover")
    var leaving = Node("Leaving")

    override func start() {
        add_child(mover)
        add_child(leaving)
        leaving.tween("x", 10.0, 100.0, "linear")
        slide()
    }

    async func slide() {
        await mover.tween("x", 2.0, 0.05, "ease_out")
        Debug.print("arrived " + mover.x.to_string())
        leaving.remove()
        await Time.next_frame()
        const left = leaving.x
        await Time.next_frame()
        await Time.next_frame()
        Debug.print("left where it was " + (leaving.x == left).to_string())
        quit()
    }
}
