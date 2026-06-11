import kairo.compose.*

fun main() = kairoApp("Counter App") {
    var count by state(0)

    Column {
        Text("Hello Kairo")
        Text("Count: $count")

        Button("+") {
            count++
        }
    }
}
