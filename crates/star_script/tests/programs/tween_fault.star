// A tween with an ease that does not exist is a runtime error.
class Main {
    static async func main() {
        const ship = Node("Ship")
        Debug.print("before")
        await ship.tween("x", 1.0, 0.5, "bouncy")
        Debug.print("never")
    }
}
