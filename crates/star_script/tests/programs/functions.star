// Static functions, the five number types, conversions, logic and loops.
class Functions {
    static func main() {
        calls()
        numbers()
        conversions()
        logic()
        flow()
    }

    static func calls() {
        Debug.print(fibonacci(15).to_string())
        Debug.print(greeting("Ana", 3))
        Debug.print(sign(-4) + " " + sign(0) + " " + sign(9))
        Debug.print(average(3, 4).to_string())
        Debug.print(halve(9).to_string())
        stop_early(true)
        stop_early(false)
        var unset: int
        var silent: bool
        var empty: string
        Debug.print(unset.to_string() + " " + silent.to_string() + " [" + empty + "]")
    }

    static func fibonacci(n: int) -> int {
        if n < 2 {
            return n
        }
        return fibonacci(n - 1) + fibonacci(n - 2)
    }

    static func greeting(name: string, times: int) -> string {
        var text = ""
        for i in 0..times {
            if i > 0 {
                text += ", "
            }
            text += "Hola " + name
        }
        return text
    }

    static func sign(value: int) -> string {
        if value < 0 {
            return "negative"
        } else if value == 0 {
            return "zero"
        } else {
            return "positive"
        }
    }

    static func average(a: double, b: double) -> double {
        return (a + b) / 2
    }

    static func halve(value: short) -> short {
        return value / 2
    }

    static func stop_early(stop: bool) {
        if stop {
            return
        }
        Debug.print("not stopped")
    }

    static func numbers() {
        var small: short = 30000
        small += 30000
        Debug.print(small.to_string())
        Debug.print((-small).to_string())
        const lowest: short = -32768
        Debug.print((-lowest).to_string())
        var count = 1000000
        count *= 5000
        Debug.print(count.to_string())
        Debug.print((small + count).to_string())
        var big: bigint = 9223372036854775807
        big -= -1
        Debug.print(big.to_string())
        Debug.print((-17 / 5).to_string() + " " + (-17 % 5).to_string())
        var ratio: float = 0.1
        ratio += 0.2
        Debug.print(ratio.to_string())
        var precise: double = 0.1
        precise += 0.2
        Debug.print(precise.to_string())
        Debug.print((7.5 % 2).to_string())
        var zero = 0.0
        Debug.print((1.0 / zero).to_string() + " " + (-1.0 / zero).to_string())
        var level = 10
        level -= 3
        level /= 2
        level %= 2
        Debug.print(level.to_string())
    }

    static func conversions() {
        Debug.print((3.99 as int).to_string() + " " + (-3.99 as int).to_string())
        var huge: double = 10000000000.0
        Debug.print((huge as int).to_string())
        Debug.print((70000 as short).to_string() + " " + (40000 as short).to_string())
        var precise: double = 1.1
        Debug.print((precise as float).to_string())
        Debug.print((precise as float as double).to_string())
        var count = 7
        Debug.print((count as float / 2).to_string())
        var wide: bigint = count
        Debug.print((wide * 1000000000000).to_string())
        var small: short = 12
        var widened: double = small
        Debug.print(widened.to_string())
        Debug.print((count as double / 4).to_string())
    }

    static func logic() {
        Debug.print((1 < 2).to_string() + " " + (2 <= 1).to_string() + " " + (3 > 3).to_string() + " " + (3 >= 3).to_string())
        Debug.print((2 == 2).to_string() + " " + (2 != 2).to_string() + " " + ("a" == "a").to_string() + " " + (1.5 != 2.5).to_string())
        Debug.print((!true).to_string())
        if false && loud("and") {
            Debug.print("never")
        }
        if true || loud("or") {
            Debug.print("short circuit")
        }
        if true && loud("both") {
            Debug.print("both ran")
        }
        var small: short = 5
        var big: bigint = 5
        Debug.print((small == big).to_string() + " " + (small < big).to_string())
    }

    static func loud(text: string) -> bool {
        Debug.print(text)
        return true
    }

    static func flow() {
        var found = -1
        for i in 0..100 {
            if i * i > 50 {
                found = i
                break
            }
        }
        Debug.print(found.to_string())
        var odd = 0
        for i in 0..10 {
            if i % 2 == 0 {
                continue
            }
            odd += i
        }
        Debug.print(odd.to_string())
        var n = 27
        var steps = 0
        while n != 1 {
            if n % 2 == 0 {
                n /= 2
            } else {
                n = 3 * n + 1
            }
            steps += 1
        }
        Debug.print(steps.to_string())
        var table = ""
        for row in 1..4 {
            for column in 1..4 {
                if column > row {
                    break
                }
                table += (row * column).to_string() + " "
            }
        }
        Debug.print(table)
        var limit = 3
        for i in 0..limit {
            limit = 10
            Debug.print("lap " + i.to_string())
        }
        Debug.print("limit " + limit.to_string())
        var start: bigint = 5000000000
        for k in start..start + 2 {
            Debug.print(k.to_string())
        }
        Debug.print(first_multiple(7, 30).to_string())
    }

    static func first_multiple(of: int, after: int) -> int {
        for value in after..after + of {
            if value % of == 0 {
                return value
            }
        }
        return -1
    }
}
