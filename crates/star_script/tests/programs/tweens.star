// Tweens in a console program: awaited until a value, by a value and to a
// color; with options (a delay, a way back, repeats); of an `export` field;
// one taking the place of another, and `stop_tweens`.
class Ship extends Node {
    export var speed = 1.0

    init() {
        super.init("Ship")
    }
}

class Main {
    static async func main() {
        const ship = Ship()
        await ship.tween("x", 4.0, 0.02, "ease_out")
        Debug.print("x " + ship.x.to_string())
        await ship.tween_by("x", -1.5, 0.02, "linear")
        Debug.print("x " + ship.x.to_string())

        const hull = Sprite("Hull", "hull.png")
        await hull.tween_color("color", Color(1.0, 0.0, 0.5, 1.0), 0.02, "ease_in")
        Debug.print("green " + hull.color.g.to_string() + ", blue " + hull.color.b.to_string())

        const options = TweenOptions()
        Debug.print("options " + options.delay.to_string() + " " + options.back.to_string() + " " + options.repeats.to_string())
        options.delay = 0.01
        options.back = true
        options.repeats = 2
        await ship.tween_with("y", 3.0, 0.01, "ease_in_out", options)
        Debug.print("y " + ship.y.to_string())

        await ship.tween("speed", 2.5, 0.02, "linear")
        Debug.print("speed " + ship.speed.to_string())

        // A new tween of `x` ends the first one where it was.
        ship.tween("x", 100.0, 10.0, "linear")
        await ship.tween("x", 0.0, 0.02, "step")
        Debug.print("x " + ship.x.to_string())

        ship.tween("rotation", 9.0, 10.0, "linear")
        ship.stop_tweens()
        await Time.next_frame()
        await Time.next_frame()
        Debug.print("stopped " + (ship.rotation < 9.0).to_string())
    }
}
