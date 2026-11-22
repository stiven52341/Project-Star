// A game in a window of the engine, which closes itself after a few seconds:
// `cargo run -p star_script --example run -- crates/star_script/examples/scripts/game.star`
class Countdown extends Game {
    const DURATION = 3

    var elapsed = 0.0
    var seconds_shown = 0

    override func start() {
        Debug.print("Cuenta atrás de " + DURATION.to_string() + " segundos")
    }

    override func update(delta: float) {
        elapsed += delta
        const seconds = elapsed as int
        if seconds > seconds_shown {
            seconds_shown = seconds
            Debug.print("Quedan " + (DURATION - seconds).to_string())
        }
        if seconds >= DURATION {
            quit()
        }
    }
}
