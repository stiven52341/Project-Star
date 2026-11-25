// Translated from a blueprint: change the blueprint, not this code.
class Lifetime extends Node {
    var seconds = 3.0

    init() {
        super.init("Lifetime")
    }

    override func update(delta: float) {
        seconds = seconds - delta
        if seconds <= 0.0 {
            remove()
        }
    }
}

// The node leaves its parent once its seconds run out.
class Main {
    static func main() {
        const level = Node("Level")
        const spark = Lifetime()
        spark.seconds = 0.5
        level.add_child(spark)
        for frame in 0..3 {
            spark.update(0.25)
            Debug.print(spark.seconds.to_string() + " " + level.children().length().to_string())
        }
    }
}
