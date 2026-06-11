import kairo.compose.*

fun main() = kairoApp("{{app_name}}") {
    var count by state(0)

    Column {
        Text("Hello Kairo")
        Text("Count: $count")

        Button("+") {
            count++
        }
    }
}

