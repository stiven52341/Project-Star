// An animator's state machine in a console program: its `states` field
// and its methods run, but without a game no `.states` file is read, so
// the machine is in no state and the values set warn on the error output.
class Main {
    static func main() {
        const animator = Animator("Animator")
        Debug.print("states '" + animator.states + "'")
        animator.states = "hero.states"
        Debug.print("states " + animator.states)
        animator.set_value("speed", 2.5)
        animator.set_flag("grounded", true)
        animator.trigger("jump")
        animator.play("run")
        Debug.print("state '" + animator.state() + "'")
    }
}
