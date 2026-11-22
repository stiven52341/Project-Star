// Reading past the end of a list stops the program with a clear message.
class Main {
    static func main() {
        const numbers = [1, 2]
        Debug.print("before")
        Debug.print(numbers[5].to_string())
        Debug.print("never printed")
    }
}
