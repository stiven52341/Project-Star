// A character that moves along the X axis.
class Character {
    var x: float = 0.0
    var name: string

    init(name: string) {
        self.name = name
    }

    func move(direction: float, delta: float) {
        x += direction * speed() * delta
    }

    func speed() -> float {
        return 100.0
    }
}

// The player: faster than other characters, and loses lives when hit.
class Player extends Character {
    const MAX_LIVES = 3

    var boost: float = 2
    var target: Enemy?
    var inventory: [string] = []
    private var lives = MAX_LIVES

    init(name: string) {
        super.init(name)
        inventory.push("espada")
    }

    override func speed() -> float {
        return super.speed() * boost
    }

    func hit() {
        lives -= 1
        if lives <= 0 {
            Debug.print(name + " ha caído")
        }
    }

    func is_alive() -> bool {
        return lives > 0
    }

    func attack() {
        const enemy = target
        if enemy == null { return }
        enemy.hit_by(self)
    }

    func target_name() -> string {
        return target?.name ?? "nadie"
    }

    /* Counts down from the given number,
       skipping the even ones. */
    func countdown(from: int) {
        for i in 0..from {
            if i % 2 == 0 { continue }
            Debug.print("Quedan " + (from - i).to_string())
        }
    }
}

class Enemy extends Character {
    var health = 10

    init() {
        super.init("Enemigo")
    }

    func hit_by(player: Player) {
        health -= 1
        if health == 0 { player.target = null }
    }

    func touch(other: Character) {
        if other is Player && other.is_alive() {
            other.hit()
        }
    }
}
