# Kairo Android MVP Implementation and App Size Notes

## Short Summary

Kairo can now build and run the sample Android app without Gradle in the generated app workspace. The Android flow is Rust-native: Kairo prepares `.kairo/android`, resolves dependencies, invokes Kotlin and the Compose compiler plugin, processes resources with AAPT2, dexes with D8, packages/signs an APK, installs it with ADB, and launches it on the emulator.

The current `hello` debug APK is about 8.31 MB. Android may show about 22 MB after install because Compose and AndroidX bytecode expands and Android also accounts for installed/optimized runtime artifacts. For a debug Compose MVP, that is not surprising. It is still larger than the long-term target, and the next optimization phase should add release shrinking/minification and dependency pruning.

## What We Have Implemented

### 1. Project Structure

The repository now has a Rust workspace with separate crates:

- `crates/kairo-cli`: command line entry point.
- `crates/kairo-config`: `kairo.toml` parsing and project config.
- `crates/kairo-android`: generated Android workspace and source layout.
- `crates/kairo-build`: Rust-native build executor.
- `crates/kairo-desktop`: placeholder desktop backend.
- `crates/kairo-native`: placeholder native bridge.

It also has:

- `kotlin/kairo-compose`: first Kotlin Compose facade.
- `templates/app-basic`: minimal project template.
- `examples/counter` and `hello`: sample apps.
- `doc`: roadmap and phase notes.

### 2. CLI Commands

Implemented/used CLI flow:

```powershell
kairo new hello
kairo doctor
kairo clean
kairo run android
```

`kairo run android` is now the main Android executor command.

### 3. Gradle-Free Generated Android Workspace

Kairo generates:

```text
.kairo/
  android/
    kairo-build-plan.md
    kairo-build-plan.toml
    src/main/AndroidManifest.xml
    src/main/kotlin/
    cache/
    intermediates/
    outputs/
```

Important point: this workspace intentionally has no Gradle files. Kairo owns the Android build steps itself.

### 4. Toolchain Discovery

The Rust build engine discovers and reports:

- Android SDK
- Android Build Tools
- `android.jar`
- Java
- `javac`
- Kotlin compiler
- AAPT2
- D8
- APK signer
- ADB

The build report is written to:

```text
hello/.kairo/android/outputs/reports/android-build-report.txt
```

### 5. Rust-Native Dependency Resolution

Kairo now resolves a direct MVP dependency set for Kotlin, Compose, and AndroidX.

Sources:

- local Kotlin install for `kotlin-stdlib` and `compose-compiler-plugin`
- local Kairo cache under `.kairo/android/cache/artifacts`
- local Maven/Gradle-style caches when available
- network downloads from Google Maven and Maven Central

Important dependencies currently included:

- Kotlin stdlib
- Compose compiler plugin
- Compose runtime, saveable, UI, text, graphics, unit, geometry, util
- Compose foundation, animation core, material ripple, Material3
- AndroidX Activity and Activity Compose
- AndroidX Lifecycle JVM/Android variants
- AndroidX SavedState Android/Compose variants
- AndroidX Core, Annotation, Collection, CustomView pooling container
- Kotlinx coroutines core and Android
- AndroidX ProfileInstaller, Autofill, Emoji2, Tracing

This is still a direct-list resolver, not a full transitive Maven resolver. A proper POM-based resolver is a future phase.

### 6. Kotlin and Compose Compilation

Kairo invokes `kotlinc` directly and passes the Compose compiler plugin from the Kotlin install.

It compiles:

- generated `MainActivity.kt`
- copied Kairo facade sources
- user app source from `src/main.kt`

Compile report:

```text
hello/.kairo/android/outputs/reports/kotlin-compile.txt
```

### 7. AAPT2 Resource Processing

Kairo now handles both app resources and dependency AAR resources.

Implemented:

- compile app `res/`
- extract dependency AAR `res/`
- compile dependency resources with AAPT2
- link resources into `resources.ap_`
- generate `R.java`
- compile generated `R.java` with `javac`
- feed generated `R.class` files into D8

This fixed runtime crashes like missing:

```text
androidx.customview.poolingcontainer.R$id
```

### 8. D8 Dexing

Kairo builds:

- app classes jar
- generated R classes jar
- dependency jar/AAR class inputs
- final dex files

Current sample output has two dex files:

```text
classes.dex
classes2.dex
```

### 9. APK Packaging

Kairo packages the APK using Rust ZIP logic.

Important fix:

- `resources.arsc` is stored and aligned correctly for Android install/runtime expectations.

Output:

```text
hello/.kairo/android/outputs/apk/debug/com.example.hello-debug.apk
```

### 10. Debug Signing

Kairo creates and reuses a debug keystore at:

```text
hello/.kairo/android/cache/signing/debug.keystore
```

Then it signs and verifies the APK using `apksigner`.

### 11. Install and Launch

Kairo installs the APK with ADB and launches:

```text
com.example.hello/.MainActivity
```

It also handles reinstall signature mismatch by uninstalling and retrying when needed.

### 12. Verified Runtime Result

Verified locally:

```powershell
kairo run android
adb shell pidof com.example.hello
```

The app launched successfully, Android logged:

```text
Displayed com.example.hello/.MainActivity
```

and the process stayed alive after launch.

## App Size Analysis

### Current Debug APK Size

Measured file:

```text
hello/.kairo/android/outputs/apk/debug/com.example.hello-debug.apk
```

Current size:

```text
8,712,079 bytes
about 8.31 MB
```

### APK Internal Breakdown

Approximate APK contents:

| Group | Files | Compressed | Raw |
| --- | ---: | ---: | ---: |
| dex | 2 | 7.83 MB | 22.73 MB |
| resources | 40 | 0.46 MB | 0.47 MB |
| signature | 3 | ~0.01 MB | ~0.01 MB |

The main size is bytecode, not images or app resources.

Largest files:

- `classes.dex`: about 5.59 MB compressed, 16.12 MB raw
- `classes2.dex`: about 2.24 MB compressed, 6.61 MB raw
- `resources.arsc`: about 0.45 MB

### Why Android Shows About 22 MB Installed

Android Settings often shows installed footprint, not only APK download size.

Installed size can include:

- base APK
- optimized dex/oat/vdex runtime artifacts
- extracted or indexed app code metadata
- app data/cache
- OS-specific accounting overhead

Because the raw dex content is about 22.73 MB, seeing about 22 MB installed for this debug Compose app is believable.

### Is 22 MB Too Much?

For a tiny hand-written Android app without Compose, yes, 22 MB installed would be too high.

For the current Kairo debug Compose MVP, it is expected because we include:

- Compose runtime/UI/Foundation/Material3
- AndroidX Activity/Lifecycle/SavedState/Core
- Kotlin stdlib and coroutines
- debug-style unshrunk bytecode
- no R8 shrinking/minification yet

So the current size is acceptable for MVP validation, but not acceptable as the final production target.

## Reasonable Size Targets

### Current Debug MVP

Reasonable right now:

- APK: 8-12 MB
- Installed size: 18-30 MB

Current result:

- APK: about 8.31 MB
- Installed size observed: about 22 MB

This is inside the expected debug Compose range.

### Future Release Target

After release optimization:

- APK: 4-8 MB for a tiny Compose app
- Installed size: 10-18 MB

More aggressive target:

- APK: 2-5 MB if Kairo avoids Material3 for minimal apps and prunes unused dependencies

Non-Compose/native-only tiny apps can be much smaller, but that is a different runtime model.

## Size Optimization Plan

### 1. Add Release Build Mode

Add:

```powershell
kairo build android --release
kairo run android --release
```

Release mode should use stricter packaging and optimization defaults.

### 2. Add R8 Shrinking and Minification

Current D8 only dexes. It does not shrink unused classes.

R8 should:

- remove unused Compose/AndroidX classes
- minify class names
- optimize bytecode
- reduce dex size

This is the biggest next size win.

### 3. Add Resource Shrinking

After code shrink is wired, Kairo should remove unused resources from dependency AARs.

This is smaller than the dex win, but still useful.

### 4. Dependency Pruning

Current direct dependency list is intentionally broad to make the MVP run.

Later Kairo should:

- parse POM files
- resolve real transitive dependencies
- avoid duplicate or unused artifacts
- include Material3 only when the app uses Material controls
- allow a lightweight UI profile for very small apps

### 5. Better Facade Profiles

Possible profiles:

- `kairo-ui-min`: text/layout/basic state only
- `kairo-ui-material`: Material controls
- `kairo-ui-full`: larger Compose feature set

The current app imports `kairo.compose.*`, and the facade includes Material helpers, so Material3 is pulled in.

### 6. Release Signing and Final Packaging

Debug signing is fine for development. Production mode should support release signing and deterministic optimized output.

## Bottom Line

The app running successfully is a big milestone: Kairo now owns a real Gradle-free Android build pipeline.

The 22 MB installed size is not a blocker yet. It is mostly unshrunk Compose/AndroidX/Kotlin bytecode. The correct next size-focused work is R8 shrink/minify plus dependency pruning.
