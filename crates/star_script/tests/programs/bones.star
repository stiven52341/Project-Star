// Bones of a cut-out character in a console program: their length, the
// sprites that hang from them turning with them, and a tween of the
// length, as an animation track would change it.
class Main {
    static async func main() {
        const arm = Bone2D("Arm")
        Debug.print("length " + arm.length.to_string())
        arm.length = 2.0
        Debug.print("length " + arm.length.to_string())
        const hand = Bone2D("Hand")
        hand.x = arm.length
        arm.add_child(hand)
        const sword = Sprite("Sword", "sword.png")
        sword.x = 1.0
        hand.add_child(sword)

        // A quarter turn of the arm takes the hand and the sword up.
        arm.rotation = 1.5707964
        const up = sword.global_y()
        const across = sword.global_x()
        Debug.print("sword up " + (up > 2.99 && up < 3.01 && across > -0.01 && across < 0.01).to_string())

        await arm.tween("length", 3.0, 0.02, "linear")
        Debug.print("length " + arm.length.to_string())
    }
}
