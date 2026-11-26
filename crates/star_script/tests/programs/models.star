// A 3D model in a console program: the file it shows, changed by the
// script, and the place it has as any `Node3D`. Without a game nothing is
// read or drawn.
class Knight extends Model {
    init() {
        super.init("Knight", "chars/knight.glb")
    }
}

class Main {
    static func main() {
        const knight = Knight()
        Debug.print("file " + knight.file)
        knight.file = "chars/guard.gltf"
        knight.z = 2.0
        knight.yaw = 0.5
        Debug.print("file " + knight.file + " at " + knight.z.to_string() + ", turned " + knight.yaw.to_string())
        const statue = Model("Statue", "statue.glb")
        knight.add_child(statue)
        Debug.print("statue in " + (statue.parent()?.name ?? "nothing"))
    }
}
