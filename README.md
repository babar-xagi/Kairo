# Kairo

Kairo is a Compose-first, Rust-powered toolchain for building native Android and desktop apps with a minimal project structure.

Current implementation status: Phase 2 facade scaffold.

## Current Commands

```bash
cargo run -p kairo-cli -- new hello
cargo run -p kairo-cli -- doctor
cargo run -p kairo-cli -- clean
```

The installed binary is named `kairo`, so once installed the intended commands are:

```bash
kairo new hello
kairo doctor
kairo clean
```

## Repository Layout

```text
kairo/
  crates/
    kairo-cli/
    kairo-config/
    kairo-build/
    kairo-android/
    kairo-desktop/
    kairo-native/
  kotlin/
    kairo-compose/
    kairo-runtime/
  templates/
    app-basic/
  examples/
    counter/
  doc/
    kairo-phases-plan.md
```

## Phase 1 Goal

Phase 1 focuses on a usable Rust CLI foundation:

- `kairo new <name>` creates a minimal Kairo project.
- `kairo doctor` checks Rust, Java, Kotlin, and Android SDK tooling.
- `kairo clean` removes Kairo-generated output.

## Phase 2 Goal

Phase 2 adds the first Kotlin source for the `kairo.compose.*` facade:

- `kairoApp`
- `state`
- `listState`
- basic layout wrappers
- basic Material controls
- small modifier helpers

The facade source currently lives in `kotlin/kairo-compose`. Full Kotlin/Compose compilation will be wired through the internal Android backend in Phase 3.

## Phase 3 Status

Phase 3 has started with Android workspace generation:

```bash
cd hello
kairo run android
```

This creates:

```text
.kairo/
  android/
    kairo-build-plan.md
    kairo-build-plan.toml
    src/main/AndroidManifest.xml
    src/main/kotlin/
```

The normal command is also available:

```bash
kairo run android
```

It intentionally does not generate Gradle files. The Rust-native executor now performs the first real task, `check-project`, which validates the generated workspace and discovers Java, Kotlin, Android SDK, `android.jar`, AAPT2, D8, APK signer, and ADB.

`resolve-compose-dependencies` now creates `.kairo/android/cache/artifacts.toml`, resolves Kotlin stdlib and the Compose compiler plugin from the local Kotlin install, and downloads the MVP AndroidX/Compose AARs into Kairo's artifact cache.

The current blocker is `compile-kotlin`: Kairo must invoke the Kotlin compiler and Compose compiler plugin directly using the resolved artifacts.
