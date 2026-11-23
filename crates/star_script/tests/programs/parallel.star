// Parallel functions: work on other threads, with copies of plain values,
// awaited from coroutines; they call each other directly.
class Numbers {
    const SCALE = 10

    parallel static func total(values: [int]) -> int {
        var sum = 0
        for value in values {
            sum += value
        }
        return sum
    }

    parallel static func scaled(rows: [[int]]) -> [int?] {
        var result: [int?] = []
        for row in rows {
            result.push(total(row) * SCALE)
        }
        result.push(null)
        return result
    }

    parallel static func shout(text: string, times: int) -> string {
        var result = ""
        for i in 0..times {
            result = result + text.to_upper()
        }
        return result
    }

    parallel static func changed(values: [int]) -> [int] {
        values.push(99)
        return values
    }
}

class Main {
    static async func main() {
        Debug.print((await Numbers.total([1, 2, 3])).to_string())
        const rows = [[1, 2], [3]]
        const scaled = await Numbers.scaled(rows)
        Debug.print(scaled.length().to_string() + " " + (scaled[0] ?? 0).to_string() + " " + (scaled[1] ?? 0).to_string() + " " + (scaled[2] == null).to_string())
        Debug.print(await Numbers.shout("ab", 2))
        // The function changes its copy, not the caller's list.
        const mine = [1]
        const theirs = await Numbers.changed(mine)
        Debug.print(mine.length().to_string() + " " + theirs.length().to_string())
    }
}
