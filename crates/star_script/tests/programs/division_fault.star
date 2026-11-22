// Dividing an integer by zero stops the program with a clear message.
class Main {
    static func main() {
        var zero = 0
        Debug.print((10 / zero).to_string())
    }
}
