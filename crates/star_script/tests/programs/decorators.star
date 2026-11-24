// Decorators of the program: classes that extend `Decorator`, around
// methods that return nothing; each object keeps its own, and several nest.
class Once extends Decorator {
    var done = false

    override func before(method: string) -> bool {
        if done {
            return false
        }
        done = true
        return true
    }
}

class Log extends Decorator {
    var label: string

    init(label: string) {
        self.label = label
    }

    override func before(method: string) -> bool {
        Debug.print(label + " before " + method)
        return true
    }

    override func after(method: string) {
        Debug.print(label + " after " + method)
    }
}

class Cooldown extends Decorator {
    var seconds: float
    var last = -1000.0

    init(seconds: float) {
        self.seconds = seconds
    }

    override func before(method: string) -> bool {
        if Time.now() - last < seconds {
            return false
        }
        last = Time.now()
        return true
    }
}

class Door {
    var opened = 0

    @Log("door")
    @Once
    func open(force: int) {
        opened += force
        Debug.print("open " + force.to_string())
    }

    @Cooldown(1.0)
    func knock() {
        Debug.print("knock")
    }
}

class Main {
    static func main() {
        const door = Door()
        door.open(2)
        door.open(3)
        door.knock()
        door.knock()
        Debug.print(door.opened.to_string())
        const other = Door()
        other.open(5)
    }
}
