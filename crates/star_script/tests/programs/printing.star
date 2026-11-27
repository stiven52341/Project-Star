// `Debug.print` writes texts, and numbers of every type as their
// `to_string` writes them. `Debug.show_fps` asks for the frames per second,
// which a console program does not show.
class Main {
    const LIVES = 3

    static func half(value: float) -> float {
        return value / 2.0
    }

    static func main() {
        Debug.show_fps(true)
        Debug.print("text")
        Debug.print(42)
        Debug.print(-7 as short)
        Debug.print((3000000 as bigint) * 3000)
        Debug.print(2.0)
        Debug.print(half(5.0))
        Debug.print(half(5.0) as double)
        Debug.print(LIVES * 2 + 1)
        const speed = 1.5
        Debug.print(speed)
        Debug.print("speed " + speed.to_string())
    }
}
