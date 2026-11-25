// Counts its frames and closes itself after the third one. Nobody presses
// `jump` while the test runs.
class CountingGame extends Game {
    var frames = 0
    // The test's game has no sounds: it stays null, with a warning.
    var beep = Sound.load("sounds/beep.wav")

    override func start() {
        Debug.print("start")
    }

    override func update(delta: float) {
        frames += 1
        Debug.print("frame " + frames.to_string() + " " + (delta >= 0.0).to_string())
        if Input.is_down("jump") || Input.just_pressed("jump") || Input.strength("jump") > 0.0 || Input.axis("jump", "jump") != 0.0 {
            Debug.print("jump")
        }
        // Where the mouse is depends on the desktop: only read it.
        if Input.has_pointer() && Input.pointer_x() + Input.pointer_y() > 1000000.0 {
            Debug.print("far away")
        }
        beep?.play()
        Audio.set_volume(Audio.volume())
        if frames == 3 {
            quit()
        }
    }
}
