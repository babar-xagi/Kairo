# Kairo

Kairo is a Kotlin + Rust toolchain experiment for building Compose-first native apps without Gradle in the app workspace. The current Android path is driven by Rust: it resolves artifacts, invokes Kotlin and the Compose compiler plugin, runs AAPT2 and D8, packages/signs an APK, installs it with ADB, and launches it on an emulator.

## Current Commands

```bash
kairo new hello
kairo doctor
kairo clean
```

Android run from a Kairo project:

```bash
cd hello
kairo run android
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

## Android Status

`kairo run android` creates a hidden generated workspace under `.kairo/android` and intentionally does not generate Gradle files.

Implemented Rust-native Android steps:

- toolchain discovery for Java, javac, Kotlin, Android SDK, AAPT2, D8, APK signer, and ADB
- local/downloaded artifact resolution into `.kairo/android/cache/artifacts`
- direct Kotlin and Compose compiler invocation
- AAPT2 resource compile/link for app resources and dependency AAR resources
- generated dependency `R.java` compilation
- D8 dexing
- Rust ZIP-based APK packaging
- debug signing
- install and launch through ADB

## Kotlin Facade

The first `kairo.compose.*` facade lives in `kotlin/kairo-compose` and includes:

- `kairoApp`
- `state`
- `listState`
- layout wrappers
- basic Material controls
- modifier helpers
