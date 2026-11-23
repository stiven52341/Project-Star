// A runtime error in a parallel function stops the program when its
// result is awaited.
class Work {
    parallel static func third(values: [int]) -> int {
        return values[2]
    }
}

class Main {
    static async func main() {
        Debug.print("before")
        Debug.print((await Work.third([1, 2])).to_string())
        Debug.print("never printed")
    }
}
