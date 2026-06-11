# kairo-compose

Kotlin source for the first `kairo.compose.*` facade.

This module is intentionally small. It gives Kairo apps one beginner-friendly import while keeping official Compose APIs available for advanced users.

## MVP API

```kotlin
import kairo.compose.*

fun main() = kairoApp("Counter") {
    var count by state(0)

    Column {
        Text("Counter")
        Text("Count: $count")

        Button("+") {
            count++
        }
    }
}
```

Current facade surface:

- `kairoApp`
- `state`
- `listState`
- `Text`
- `Button`
- `TextField`
- `Column`
- `Row`
- `Box`
- `Spacer`
- common Compose type aliases
- small modifier helpers such as `pad`, `fillWidth`, `widthDp`, and `heightDp`

Advanced Compose imports are still allowed:

```kotlin
import kairo.compose.*
import androidx.compose.material3.NavigationBar
```

## Compile Note

This folder contains source-level facade code. It is not wired to the Rust-native Compose compiler path yet because Phase 3 will define the internal Android build engine that consumes it.
