// A constant read through an object: the object is still evaluated, for
// its effects, before the constant is given; in a coroutine too.
class Crate {
    const SIZE = 4

    var label: string

    init(label: string) {
        self.label = label
    }
}

class Main {
    static func make(label: string) -> Crate {
        Debug.print("made " + label)
        return Crate(label)
    }

    static async func later(label: string) -> Crate {
        await Time.next_frame()
        return make(label)
    }

    static async func main() {
        Debug.print(make("a").SIZE.to_string())
        const total = make("b").SIZE + Crate.SIZE
        Debug.print(total.to_string())
        const size = (await later("c")).SIZE
        Debug.print(size.to_string())
    }
}
