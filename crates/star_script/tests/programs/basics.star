// Values, operators, control flow, shared lists and null safety.
class Basics {
    const GREETING = "Hola"
    const LIMIT = 2 * 3

    static func main() {
        Debug.print(GREETING + ", StarScript")
        Debug.print(LIMIT.to_string())
        arithmetic()
        integers()
        loops()
        lists()
        nulls()
    }

    static func arithmetic() {
        var a = 7
        a += 3
        Debug.print((a / 3).to_string())
        Debug.print((-7 % 3).to_string())
        var speed: float = 2
        speed *= 1.5
        Debug.print(speed.to_string())
        Debug.print((10 as float / 4).to_string())
        Debug.print((2.7 as int).to_string())
        Debug.print((-5).abs().to_string())
        var ten = 10.0
        Debug.print(ten.sqrt().floor().to_string())
        const big: bigint = 9223372036854775807
        Debug.print((big + 1).to_string())
        Debug.print("abc".to_upper() + " " + "abc".length().to_string())
        var text = "Star"
        text += "Script"
        Debug.print(text + " " + text.contains("Scr").to_string())
    }

    static func integers() {
        var small: short = 32767
        small += 1
        Debug.print(small.to_string())
        const lowest: short = -32768
        Debug.print(lowest.to_string())
        var count = 2147483647
        count += 1
        Debug.print(count.to_string())
        const wide = count as bigint * 2
        Debug.print(wide.to_string())
        Debug.print((70000 as short).to_string())
        var steps: short = 0
        for step in steps..3 {
            steps += step
        }
        Debug.print(steps.to_string())
        var total: bigint = count
        total += steps
        Debug.print((total * 2).to_string())
        Debug.print((steps < count).to_string())
        var precise: double = 0.1
        precise += 0.2
        Debug.print(precise.to_string())
        Debug.print((precise + steps).to_string())
        var third: float = 1.5
        precise = third
        Debug.print((precise / 3).to_string())
        var sizes: [bigint] = [steps, count]
        Debug.print((sizes[0] == steps).to_string())
    }

    static func loops() {
        var total = 0
        for i in 0..5 {
            if i == 3 { continue }
            total += i
        }
        Debug.print("total " + total.to_string())
        var n = 10
        while n > 0 {
            n -= 4
            if n < 3 { break }
        }
        Debug.print("n " + n.to_string())
        if total > 10 {
            Debug.print("big")
        } else if total > 5 {
            Debug.print("medium")
        } else {
            Debug.print("small")
        }
    }

    static func lists() {
        var names = ["Ana", "Luis"]
        const alias = names
        alias.push("Eva")
        Debug.print(names.length().to_string())
        names[0] = "Ada"
        Debug.print(alias[0])
        var numbers: [int] = []
        for value in [3, 4] {
            numbers.push(value * 10)
        }
        numbers[1] += 5
        Debug.print(numbers[0].to_string() + " " + numbers[1].to_string())
        Debug.print(numbers.contains(45).to_string())
        Debug.print((numbers == [30, 45]).to_string())
        const last = numbers.pop()
        Debug.print((last ?? 0).to_string())
    }

    static func nulls() {
        var maybe: int? = null
        Debug.print((maybe ?? -1).to_string())
        maybe = 5
        const known = maybe
        if known != null {
            Debug.print((known + 1).to_string())
        }
        var words: [string?] = ["a", null]
        const second = words[1]
        Debug.print(second ?? "nada")
        Debug.print((second == null).to_string())
    }
}
