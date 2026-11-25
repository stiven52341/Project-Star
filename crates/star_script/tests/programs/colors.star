// Engine objects without a game: colors and cameras, a class that extends
// one, and `is` and `as` with engine classes.
class Palette {
    var main = Color(0.0, 0.0, 0.0, 1.0)
    var colors = [Color(1.0, 1.0, 1.0, 1.0), Color(0.0, 0.5, 0.0, 1.0)]
}

class Eye extends Camera2D {
    var zoom = 2.0

    init() {
        super.init("Eye")
        view_height = 10.0
    }
}

class Main {
    static func main() {
        const red = Color(1.0, 0.0, 0.0, 1.0)
        red.g += 0.5
        Debug.print(red.r.to_string() + " " + red.g.to_string() + " " + red.b.to_string() + " " + red.a.to_string())
        // Objects are shared: a change through one name is seen through all.
        const same = red
        same.b = 0.25
        Debug.print(red.b.to_string() + " " + (same == red).to_string() + " " + (Color(1.0, 0.5, 0.25, 1.0) == red).to_string())

        // Colors kept in objects of the program.
        const palette = Palette()
        palette.main.r = 0.75
        Debug.print(palette.main.r.to_string() + " " + palette.colors.length().to_string() + " " + palette.colors[1].g.to_string())

        const camera = Camera2D("View")
        Debug.print(camera.name + " " + camera.view_height.to_string() + " " + camera.main.to_string())
        camera.x = 3.0
        camera.x *= 2.0
        camera.main = true
        camera.name = "Main view"
        Debug.print(camera.name + " " + camera.x.to_string() + " " + camera.main.to_string())

        const eye = Eye()
        const node: Node = eye
        Debug.print((node is Camera2D).to_string() + " " + (node is Eye).to_string() + " " + (camera is Eye).to_string())
        Debug.print(eye.name + " " + eye.view_height.to_string() + " " + eye.zoom.to_string() + " " + eye.scale_x.to_string() + " " + eye.rotation.to_string() + " " + eye.y.to_string())
        const seen = node as Camera2D
        Debug.print((seen?.view_height ?? 0.0).to_string())
        const plain = Node("Plain")
        Debug.print(((plain as Camera2D) == null).to_string() + " " + plain.name)
    }
}
