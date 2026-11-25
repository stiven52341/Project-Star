// A main for the crate's example script, `examples/scripts/player.star`:
// both the compiled end-to-end test and the interpreter's conformance test
// run the script followed by this, expecting `player_main.out`.
class Main {
    static func main() {
        const p = Player("Ana")
        p.countdown(4)
        Debug.print(p.target_name())
    }
}
