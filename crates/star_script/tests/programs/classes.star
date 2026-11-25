// Objects of classes without a base: fields, init, methods, self,
// private methods, fields of other objects, ?. and equality by identity.
class Counter {
    var name: string
    var count = 0
    var step: short = 1
    var history: [int] = []
    var partner: Counter?

    init(name: string, step: short) {
        self.name = name
        Debug.print("created " + name)
        self.step = step
        remember()
    }

    func increase() -> int {
        count += step
        remember()
        return count
    }

    func link(other: Counter) {
        partner = other
        other.partner = self
    }

    func partner_name() -> string {
        return partner?.name ?? "nobody"
    }

    func total_with_partner() -> int {
        return count + (partner?.count ?? 0)
    }

    func same_as(other: Counter?) -> bool {
        return self == other
    }

    private func remember() {
        history.push(count)
    }

    func history_text() -> string {
        var text = ""
        for value in history {
            text += value.to_string() + " "
        }
        return text
    }
}

class Point {
    var x: float
    var y: float

    init(x: float, y: float) {
        self.x = x
        self.y = y
    }

    func moved(dx: float, dy: float) -> Point {
        return Point(x + dx, y + dy)
    }

    func text() -> string {
        return "(" + x.to_string() + ", " + y.to_string() + ")"
    }
}

class Empty {
    func hello() -> string {
        return "hello from an object without fields"
    }
}

class Main {
    static func main() {
        const first = Counter("first", 2)
        const second = Counter("second", 5)
        first.increase()
        first.increase()
        second.increase()
        Debug.print(first.count.to_string() + " " + second.count.to_string())
        Debug.print(first.partner_name())
        first.link(second)
        Debug.print(first.partner_name() + " " + second.partner_name())
        Debug.print(first.total_with_partner().to_string())
        first.count += 10
        const partner = second.partner
        if partner != null {
            partner.count -= 1
        }
        Debug.print(first.count.to_string())
        Debug.print(first.history_text())
        Debug.print(first.same_as(first).to_string() + " " + first.same_as(second).to_string() + " " + first.same_as(null).to_string())
        Debug.print((first == second.partner).to_string())
        var nobody: Counter? = null
        Debug.print((nobody?.increase() ?? -1).to_string())
        nobody = second
        Debug.print((nobody?.increase() ?? -1).to_string())
        const start = Point(1, 2.5)
        const end = start.moved(0.5, -1).moved(1, 1)
        Debug.print(start.text() + " " + end.text())
        end.x = 10
        Debug.print(end.text())
        const points = [start, end]
        points[1].y *= 2
        Debug.print(end.text() + " " + points.contains(start).to_string())
        Debug.print(Empty().hello())
    }
}
