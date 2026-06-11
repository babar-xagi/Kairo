# Phase 2: Compose Facade MVP

Phase 2 creates the first source-level `kairo.compose.*` facade. The goal is not to hide Jetpack Compose. The goal is to make the first app import simple while still allowing advanced users to import official Compose APIs.

## Current API

```kotlin
import kairo.compose.*

fun main() = kairoApp("Hello") {
    var count by state(0)

    Column {
        Text("Hello Kairo")
        Text("Count: $count")

        Button("+") {
            count++
        }
    }
}
```

Implemented source files:

- `App.kt`: `kairoApp` and registered app definition
- `State.kt`: `state`, `stateOf`, `listState`, delegation operators
- `Layout.kt`: `Column`, `Row`, `Box`, `Spacer`
- `Material.kt`: `Text`, `Button`, `TextField`
- `ModifierHelpers.kt`: simple spacing and sizing helpers
- `Types.kt`: common Compose type aliases

## Boundaries

This phase adds source code and API shape only. It does not yet add Compose compiler wiring, Android Activity generation, or desktop launcher.

Phase 3 should consume this facade through the internal generated Android build path.

## Success Criteria

- Kairo app templates use only `import kairo.compose.*`.
- The facade contains the common controls promised in the blueprint.
- Official Compose imports remain compatible because the facade uses normal Kotlin declarations, not a custom language or DSL.
- Rust workspace checks continue to pass after the Kotlin source is added.
