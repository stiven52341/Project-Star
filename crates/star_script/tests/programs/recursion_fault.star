// A method that calls itself without end stops the program with a clear
// message instead of overflowing the stack; deep calls that do end run.
class Star {
    var turns = 0

    func start() {
        turns += 1
        start()
    }
}

class Main {
    static func down(steps: int) -> int {
        if steps == 0 { return 0 }
        return down(steps - 1) + 1
    }

    static func main() {
        Debug.print(down(900).to_string())
        Star().start()
        Debug.print("never printed")
    }
}
