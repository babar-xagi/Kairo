# Phase 3: Android Run MVP

Phase 3 starts the path from a minimal Kairo project to a native Android app.

## Current Slice

Implemented command:

```powershell
kairo run android
```

This command reads:

```text
kairo.toml
src/main.kt
```

and generates:

```text
.kairo/android/
  kairo-build-plan.md
  kairo-build-plan.toml
  src/main/AndroidManifest.xml
  src/main/res/values/styles.xml
  src/main/kotlin/<app id>/MainActivity.kt
  src/main/kotlin/kairo/compose/*.kt
  src/main/kotlin/kairo/user/Main.kt
```

## Behavior

- User project files stay clean.
- Generated Android files live under `.kairo/android`.
- No Gradle files are generated.
- The build plan is owned by Kairo's Rust build engine.
- `src/main.kt` is copied into the generated workspace.
- If the user source has no package, Kairo wraps it in `package kairo.user`.
- The generated `MainActivity` calls the user `main()` function, then renders the registered `kairoApp`.
- The Phase 2 `kairo.compose.*` facade source is embedded into the generated Android workspace.

## Current Limitation

`kairo run android` currently prepares the Rust-native Android workspace, runs the first executor task, and writes:

```text
.kairo/android/outputs/reports/android-build-report.txt
```

Implemented executor task:

- `check-project`: validates project files and discovers Java, Kotlin, Android SDK, `android.jar`, AAPT2, D8, APK signer, and ADB.

Implemented executor task:

- `resolve-compose-dependencies`: creates `.kairo/android/cache/artifacts.toml`, resolves Kotlin stdlib and the Compose compiler plugin from the local Kotlin install, checks local Maven-style caches, and downloads the MVP AndroidX/Compose AARs into Kairo's artifact cache.

Current blocker:

- `compile-kotlin`: invoke `kotlinc` directly with the generated sources, `android.jar`, resolved Kotlin jars, extracted AAR class jars, and Compose compiler plugin.

## Next Work

- Extract `classes.jar` from resolved AAR artifacts.
- Invoke the Kotlin compiler and Compose compiler plugin directly.
- Run AAPT2 resource compile/link directly.
- Run D8 directly.
- Package and sign a debug APK directly.
- Install with `adb install -r`.
- Launch with `adb shell am start`.
