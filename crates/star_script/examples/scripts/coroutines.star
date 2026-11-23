// Coroutines and parallel functions in a game of the engine, which closes
// itself once its work is done:
// `cargo run -p star_script --example run -- crates/star_script/examples/scripts/coroutines.star`
class Primes {
    // Heavy work: it runs on another thread while the game keeps playing.
    parallel static func count_below(limit: int) -> int {
        var count = 0
        for n in 2..limit {
            if is_prime(n) {
                count += 1
            }
        }
        return count
    }

    // Called directly from another parallel function, on the same thread.
    parallel static func is_prime(n: int) -> bool {
        var divisor = 2
        while divisor * divisor <= n {
            if n % divisor == 0 {
                return false
            }
            divisor += 1
        }
        return true
    }
}

class Messenger extends Node {
    init() {
        super.init("Messenger")
    }

    // A coroutine of a node: it ends by itself when the node leaves the tree.
    async func talk() {
        var said = 0
        while true {
            said += 1
            Debug.print("Mensajero: sigo aquí (" + said.to_string() + ")")
            await Time.wait(0.25)
        }
    }

    override func start() {
        talk()
    }
}

class Showcase extends Game {
    const FADE_FRAMES = 60

    var messenger = Messenger()

    override func start() {
        add_child(messenger)
        // Both start at once; start() goes on without waiting for them.
        fade_in()
        count_primes()
    }

    // The background brightens a little every frame.
    async func fade_in() {
        for step in 0..FADE_FRAMES {
            const light = step as float / FADE_FRAMES as float
            background = Color(0.0, light * 0.3, light * 0.5, 1.0)
            await Time.next_frame()
        }
    }

    async func count_primes() {
        Debug.print("Contando primos en otro hilo...")
        const count = await Primes.count_below(1000000)
        Debug.print("Hay " + count.to_string() + " primos por debajo de un millón")
        messenger.remove()
        Debug.print("El mensajero salió del árbol: su corrutina terminó con él")
        await Time.wait(1.0)
        quit()
    }
}
