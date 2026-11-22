// Outside a game window nobody points anywhere: the pointer stays at the
// origin.
class Main {
    static func main() {
        Debug.print(Input.has_pointer().to_string())
        Debug.print(Input.pointer_x().to_string() + " " + Input.pointer_y().to_string())
    }
}
