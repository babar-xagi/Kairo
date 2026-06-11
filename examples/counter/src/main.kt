import kairo.compose.*

fun main() = kairoApp("Counter") {
    var count by state(0)

    Column {
        Text("Counter App")
        Text("Count: $count")

        Button("+") {
            count++
        }

        Button("-") {
            count--
        }
    }
}

