// Global and local positions, without the engine.
class Main {
    // A number to two decimals, so tiny rounding errors do not show.
    static func show(value: float) -> string {
        return ((value * 100.0).round() / 100.0).to_string()
    }

    static func main() {
        // A ship turned a quarter turn and scaled twice: its child, one unit
        // to its right, is two units above it in the world.
        const ship = Node("Ship")
        ship.x = 10.0
        ship.rotation = 1.5707964
        ship.scale_x = 2.0
        ship.scale_y = 2.0
        const gun = Node("Gun")
        gun.x = 1.0
        ship.add_child(gun)
        Debug.print(show(gun.global_x()) + " " + show(gun.global_y()))
        Debug.print(show(gun.global_rotation()))

        // Placed in the world, it gets the local place that puts it there.
        gun.set_global_position(10.0, -4.0)
        Debug.print(show(gun.x) + " " + show(gun.y))
        gun.translate_global(0.0, 2.0)
        Debug.print(show(gun.x) + " " + show(gun.y))
        gun.translate(1.0, 0.0)
        Debug.print(show(gun.x) + " " + show(gun.y))
        gun.set_global_rotation(0.0)
        Debug.print(show(gun.rotation))
        gun.rotate(0.5)
        Debug.print(show(gun.global_rotation()))

        // In 3D: a base turned a quarter turn around y, its arm one unit
        // ahead (-z) is one unit to its left (-x) in the world.
        const base = Node3D("Base")
        base.x = 5.0
        base.yaw = 1.5707964
        const arm = Node3D("Arm")
        arm.z = -1.0
        base.add_child(arm)
        Debug.print(show(arm.global_x()) + " " + show(arm.global_y()) + " " + show(arm.global_z()))
        arm.set_global_position_3d(5.0, 1.0, -2.0)
        Debug.print(show(arm.x) + " " + show(arm.y) + " " + show(arm.z))
        arm.translate_global_3d(-1.0, 0.0, 0.0)
        Debug.print(show(arm.global_x()) + " " + show(arm.z))
    }
}
