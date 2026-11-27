// Sound players in a console program: their sound, volume, looping, play
// on start, group and reach, changed by the script. Without a game nothing is heard,
// so they never count as playing.
class Music extends SoundPlayer {
    init() {
        super.init("Music", "music/theme.ogg")
        looping = true
        volume = 0.5
    }
}

class Main {
    static func main() {
        const music = Music()
        Debug.print(music.sound + " " + music.volume.to_string() + " " + music.looping.to_string() + " " + music.play_on_start.to_string())
        music.play()
        Debug.print("playing " + music.is_playing().to_string())
        music.stop()
        const hit = SoundPlayer("Hit", "sounds/hit.wav")
        hit.play_on_start = false
        hit.sound = "sounds/punch.wav"
        music.add_child(hit)
        Debug.print(hit.sound + " " + hit.play_on_start.to_string() + " in " + (hit.parent()?.name ?? "nothing"))
        hit.group = "effects"
        Audio.set_group_volume("effects", 0.5)
        // Without a game the groups are not kept: every one is at full volume.
        Debug.print(music.group + "|" + hit.group + " " + Audio.group_volume("effects").to_string())
        hit.spatial = true
        hit.far = 8.0
        Debug.print(music.spatial.to_string() + " " + hit.spatial.to_string() + " " + hit.near.to_string() + " " + hit.far.to_string())
    }
}
