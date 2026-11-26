// Coroutines in a console program: started, awaited, with values, through
// overrides, `?.` and `??`, and waiting until all of them finish.
class Counter {
    var name: string

    init(name: string) {
        self.name = name
    }

    async func count(times: int) -> int {
        for i in 0..times {
            Debug.print(name + " " + i.to_string())
            await Time.next_frame()
        }
        return times
    }

    async func greet() {
        Debug.print("hello from " + name)
    }
}

class Loud extends Counter {
    init() {
        super.init("loud")
    }

    override async func greet() {
        await super.greet()
        Debug.print("HELLO")
    }
}

class Main {
    static async func twice(value: int) -> int {
        await Time.wait(0.01)
        return value * 2
    }

    static func maybe(value: int) -> int? {
        if value > 0 { return value }
        return null
    }

    static async func main() {
        const a = Counter("a")
        const b = Counter("b")
        // Started: each runs until its first wait, and main goes on.
        a.count(2)
        b.count(2)
        Debug.print("both started")
        const total = await a.count(1) + await twice(5)
        Debug.print("total " + total.to_string())

        const counter: Counter = Loud()
        await counter.greet()
        const nobody: Counter? = null
        nobody?.greet()
        const somebody: Counter? = Counter("c")
        somebody?.greet()
        Debug.print((maybe(0) ?? await twice(4)).to_string())
        Time.wait(0.05)
        // The frames of a console program last real time: only its sign
        // is the same every run.
        Debug.print("delta " + (Time.delta() >= 0.0).to_string())
        Debug.print("main ends")
    }
}
