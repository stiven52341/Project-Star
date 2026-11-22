// Classes: inheritance, overriding, construction in two phases, is and as.
class Weapon {
    var damage: int

    init(damage: int) {
        self.damage = damage
    }
}

class Character {
    var name: string
    var health = 100

    init(name: string) {
        self.name = name
        // Runs the version of the subclass, whose fields already exist.
        Debug.print("new " + describe())
    }

    func describe() -> string {
        return "character " + name
    }

    func take(amount: int) {
        health -= amount
        if health < 0 { health = 0 }
    }
}

class Player extends Character {
    var weapon: Weapon
    var friends: [Character] = []
    private var score = 0

    init(name: string, damage: int) {
        weapon = Weapon(damage)
        super.init(name)
        add_score(10)
    }

    override func describe() -> string {
        return "player " + name + " with " + weapon.damage.to_string()
    }

    private func add_score(points: int) {
        score += points
    }

    func attack(target: Character) {
        target.take(weapon.damage)
        add_score(1)
        Debug.print(name + " hits " + target.name + ": " + target.health.to_string())
    }

    func score_text() -> string {
        return "score " + score.to_string()
    }
}

class Animal {
    var sound = "..."

    init() {
        Debug.print("animal ready")
    }

    func speak() -> string {
        return sound
    }
}

class Dog extends Animal {
    init() {
        sound = "guau"
    }

    override func speak() -> string {
        return super.speak() + "!"
    }
}

class Puppy extends Dog {
    override func speak() -> string {
        return "small " + super.speak()
    }
}

class Game {
    static func main() {
        const hero = Player("Ana", 30)
        const orc = Character("Orc")
        hero.attack(orc)
        hero.attack(orc)
        Debug.print(hero.score_text())
        hero.friends.push(orc)
        Debug.print(hero.friends.length().to_string())

        const someone: Character = hero
        Debug.print(someone.describe())
        Debug.print((someone is Player).to_string())
        Debug.print((orc is Player).to_string())
        const as_player = someone as Player
        Debug.print(as_player?.score_text() ?? "not a player")
        const orc_as_player = orc as Player
        Debug.print(orc_as_player?.score_text() ?? "not a player")
        if someone is Player {
            Debug.print("narrowed: " + someone.score_text())
        }
        Debug.print((someone == hero).to_string())
        Debug.print((orc == hero).to_string())
        hero.weapon.damage = 99
        orc.take(500)
        Debug.print(orc.health.to_string())
        Debug.print(hero.describe())
        animals()
    }

    static func animals() {
        const puppy = Puppy()
        Debug.print(puppy.speak())
        var maybe_dog: Dog? = null
        Debug.print(maybe_dog?.speak() ?? "no dog")
        maybe_dog = puppy
        Debug.print(maybe_dog?.speak() ?? "no dog")
    }
}
