// A small program: `cargo run -p star_script --example run`.
class Greeter {
    var name: string
    var greetings = 0

    init(name: string) {
        self.name = name
    }

    func greet() {
        greetings += 1
        Debug.print("Hola, " + name + " (" + greetings.to_string() + ")")
    }
}

class LoudGreeter extends Greeter {
    init(name: string) {
        super.init(name)
    }

    override func greet() {
        super.greet()
        Debug.print("¡" + name.to_upper() + "!")
    }
}

class Main {
    static func main() {
        const greeters: [Greeter] = [Greeter("Ana"), LoudGreeter("Luis")]
        for greeter in greeters {
            greeter.greet()
        }
        var first: Greeter? = greeters.pop()
        Debug.print("Último: " + (first?.name ?? "nadie"))
    }
}
