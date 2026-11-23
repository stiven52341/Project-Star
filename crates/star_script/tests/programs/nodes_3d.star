// 3D nodes without the engine: they are made, placed, turned and given
// bodies, but nothing draws or simulates them.
class Crate extends Mesh {
    init() {
        super.init("Crate", "cube")
        y = 3.0
        roughness = 0.8
        body = Body.box("dynamic", 1.0, 2.0, 3.0)
    }
}

class Main {
    static func main() {
        const crate_mesh = Crate()
        Debug.print(crate_mesh.shape + " " + crate_mesh.width.to_string() + " " + crate_mesh.radius.to_string() + " " + crate_mesh.roughness.to_string())
        Debug.print(crate_mesh.z.to_string() + " " + crate_mesh.scale_z.to_string() + " " + crate_mesh.texture.is_empty().to_string())
        const solid = crate_mesh.body
        if solid != null {
            Debug.print(solid.shape + " " + solid.depth.to_string() + " " + solid.velocity_z.to_string())
            solid.apply_impulse_3d(0.0, 0.0, 1.0)
        }
        const ball = Body.sphere("static", 0.25)
        Debug.print(ball.shape + " " + ball.radius.to_string())

        // Looking from the front at the origin, then from the side upwards.
        const eye = Camera3D("Eye")
        eye.z = 5.0
        eye.look_at(0.0, 0.0, 0.0)
        Debug.print((eye.pitch.abs() < 0.001).to_string() + " " + (eye.yaw.abs() < 0.001).to_string() + " " + ((eye.fov - 1.0472).abs() < 0.001).to_string())
        eye.x = 5.0
        eye.z = 0.0
        eye.look_at(0.0, 5.0, 0.0)
        Debug.print(((eye.pitch - 0.7854).abs() < 0.001).to_string() + " " + ((eye.yaw - 1.5708).abs() < 0.001).to_string())

        const sun = Light("Sun", "directional")
        Debug.print(sun.kind + " " + sun.intensity.to_string() + " " + sun.range.to_string())

        const pivot = Node3D("Pivot")
        pivot.add_child(crate_mesh)
        pivot.add_child(eye)
        Debug.print(pivot.children().length().to_string() + " " + (crate_mesh.parent()?.name ?? ""))
        Debug.print((crate_mesh is Node3D).to_string() + " " + (eye is Mesh).to_string() + " " + (pivot is Node).to_string())
    }
}
