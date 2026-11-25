// Translated from a blueprint: change the blueprint, not this code.
class Tally extends Node {
    var count = 0
    var sum = 0

    init() {
        super.init("Tally")
    }

    func total(values: [int]) -> int {
        sum = 0
        for element_4 in values {
            sum = sum + element_4
        }
        return sum
    }

    func parity(value: int) -> string {
        if (value % 2) == 0 {
            return "even"
        } else {
            return "odd"
        }
    }

    override func update(delta: float) {
        count = count + 1
        Debug.print(parity(count))
        translate(1.0, delta * 2.0)
    }
}

class Main {
    static func main() {
        const level = Node("Level")
        const tally = Tally()
        level.add_child(tally)
        Debug.print(tally.total([1, 2, 3, 4]).to_string())
        Debug.print(tally.total([]).to_string())
        for frame in 0..3 {
            tally.update(0.25)
        }
        Debug.print(tally.count.to_string() + " " + tally.x.to_string() + " " + tally.y.to_string())
    }
}
