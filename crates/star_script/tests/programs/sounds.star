// Outside a game no sound can be loaded: `Sound.load` gives null (with a
// warning on the error output), and the volume stays where it is.
class Main {
    static func main() {
        const jump = Sound.load("sounds/jump.wav")
        Debug.print((jump == null).to_string())
        jump?.play()
        Debug.print((jump?.is_playing() ?? false).to_string())
        Audio.set_volume(0.5)
        Debug.print(Audio.volume().to_string())
    }
}
