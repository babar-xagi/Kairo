# Kairo Phase Plan

Source: `project-bluprint.md`

## Project Vision

Kairo is a Compose-first, Rust-powered toolchain for building native Android and desktop apps with a minimal project structure. The product goal is to let developers write direct Jetpack Compose UI, build and run through a Rust CLI, and avoid visible Android project boilerplate such as Gradle files, XML layouts, nested Android folders, and manual manifest setup.

Core promise:

```text
Write Compose. Build with Rust. Ship native apps without Android project mess.
```

Target developer flow:

```bash
kairo new counter
cd counter
kairo run android
kairo run desktop
```

Target starter project:

```text
counter/
  kairo.toml
  src/
    main.kt
```

Optional native project:

```text
counter/
  kairo.toml
  src/
    main.kt
    native.rs
```

## MVP Definition

The first useful MVP should prove that Kairo can hide Android project complexity while still running real Compose code. The MVP does not need to support every Android build feature on day one, but it should avoid Gradle as the build runner from the start. It must create a minimal project, expose a simple `kairo.compose.*` developer experience, build a basic Compose app, and run it on Android with clear diagnostics.

MVP acceptance:

- `kairo new hello` creates `kairo.toml` and `src/main.kt`.
- `kairo doctor` validates the local JDK, Kotlin, Android SDK, and required Android tools.
- A user can write a basic app with only `import kairo.compose.*`.
- `kairo run android` builds, installs, and launches a simple debug Android app.
- Generated Android files may exist internally, but they must not become part of the user-facing project layout.
- Error messages must explain missing tools, invalid config, compiler failures, and device issues in beginner-friendly language.

Post-MVP expansion:

- Add incremental Rust-native build orchestration.
- Add optional Rust native module support.
- Add desktop support through Compose Desktop.
- Add dependency management through `kairo.toml` and `kairo.lock`.
- Implement Android build steps as Rust-native tasks over time.
- Mature into production release, signing, packaging, testing, and CI support.

## Phase 0: Research and Technical Design

### Goal

Understand the Android, Kotlin, Compose, desktop, and Rust-native build paths deeply enough to avoid building the wrong abstraction.

### Scope

- Map the Android debug build pipeline for a minimal Compose app.
- Map Kotlin compiler and Compose compiler plugin requirements.
- Identify required Android SDK tools, including AAPT2, D8, APK signing, install, and launch commands.
- Map Compose Desktop requirements and packaging choices.
- Study Rust-to-Kotlin native interop paths for Android and desktop.
- Decide what Kairo can safely generate internally during early phases.

### Deliverables

- Technical design document for Kairo architecture.
- Build pipeline map for Android debug builds.
- Build pipeline map for desktop runs.
- Native bridge design note covering JNI or C ABI direction.
- First `kairo.toml` schema draft.
- First app template draft.
- Toolchain requirements list for `kairo doctor`.

### Success Criteria

- The team can explain every step from `src/main.kt` to installed Android app.
- The team can identify which steps are temporarily delegated to existing tools and which steps Kairo owns.
- The app template and config schema are clear enough to implement Phase 1.
- The design calls out hard risks before implementation starts.

### Dependencies

- Local Android SDK and JDK documentation.
- Compose compiler and Kotlin compiler documentation.
- Compose Desktop packaging documentation.
- Rust Android target and JNI documentation.

## Phase 1: Rust CLI MVP

### Goal

Create the first working Rust CLI with project creation and environment diagnostics.

### Scope

- Implement `kairo new <app-name>`.
- Implement `kairo doctor`.
- Implement `kairo clean`.
- Generate a minimal project structure.
- Generate `kairo.toml`.
- Generate `src/main.kt`.
- Validate local development requirements.
- Print clear errors for missing or unsupported tools.

### Deliverables

- Rust CLI crate.
- Config parser for `kairo.toml`.
- Basic app template.
- Doctor checks for JDK, Kotlin, Android SDK, Android platform tools, and required environment variables.
- Clean command that removes Kairo-generated build output only.

### Success Criteria

Running:

```bash
kairo new hello
```

creates:

```text
hello/
  kairo.toml
  src/
    main.kt
```

The generated `kairo.toml` includes:

```toml
[app]
name = "Hello"
id = "com.example.hello"
version = "0.1.0"

[targets]
android = true
desktop = false

[build]
engine = "kairo"
cache = true
parallel = true

[ui]
toolkit = "compose"
import_facade = "kairo.compose"
```

`kairo doctor` reports pass, warn, or fail states without panics.

### Dependencies

- Phase 0 config schema draft.
- Phase 0 toolchain requirements.
- Rust argument parsing and TOML parsing libraries.

## Phase 2: Compose Facade MVP

### Goal

Create the first version of `kairo.compose.*` so users can write simple Compose apps with one Kairo import.

### Scope

- Provide a Kotlin package named `kairo.compose`.
- Re-export or wrap common Compose runtime, foundation, and Material3 APIs.
- Provide `kairoApp`.
- Provide `state`.
- Provide basic layout and UI helpers.
- Keep official Compose APIs available for advanced imports.

### Deliverables

- Kotlin facade module.
- Runtime entrypoint for simple Kairo apps.
- Wrappers or aliases for:
  - `Text`
  - `Button`
  - `Column`
  - `Row`
  - `Box`
  - `Spacer`
  - basic state helpers
  - basic padding helpers
  - basic alignment helpers
- Example counter app using only `import kairo.compose.*`.
- Compatibility note for importing official Compose APIs alongside Kairo APIs.

### Success Criteria

This app compiles in the supported build path:

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

Advanced users can still write:

```kotlin
import kairo.compose.*
import androidx.compose.material3.NavigationBar
```

### Dependencies

- Kotlin module layout from Phase 0.
- Compose runtime, foundation, and Material3 dependencies.
- Temporary internal build path that can compile the facade.

## Phase 3: Android Run MVP

### Goal

Make `kairo run android` build, install, and launch a basic Compose app on an emulator or connected Android device.

### Scope

- Read `kairo.toml`.
- Generate required Android project files internally.
- Compile Kotlin and Compose code.
- Include the `kairo.compose` facade.
- Package a debug APK.
- Detect connected Android devices or emulators.
- Install the APK.
- Launch the main activity.
- Keep generated Android files out of the user-facing project structure.

### Deliverables

- `kairo run android` command.
- Internal generated Android workspace layout.
- Debug manifest generation.
- Debug APK packaging path.
- Device detection and selection.
- Install and launch flow.
- Counter app example.
- Error messages for no devices, SDK mismatch, compiler failure, install failure, and launch failure.

### Success Criteria

Running:

```bash
kairo run android
```

from a generated counter app builds and launches a native Android Compose app.

The user project remains:

```text
counter/
  kairo.toml
  src/
    main.kt
```

### Dependencies

- Phase 1 CLI and config parser.
- Phase 2 Compose facade.
- Android SDK and platform tools.
- Temporary generated build files or existing Android tooling, hidden from the user project.

## Phase 4: Rust Build Engine Core

### Goal

Start replacing ad hoc orchestration with a Rust-native build engine that can track inputs, skip unchanged work, and run tasks consistently.

### Scope

- Build graph representation.
- Task scheduler.
- File hashing.
- Input and output tracking.
- Incremental cache.
- Artifact directory conventions.
- Parallel task execution.
- Structured task logs.
- Source change detection.

### Deliverables

- `kairo-build` crate.
- Build graph API.
- Task abstraction for check, compile, package, install, and run steps.
- File hash cache.
- Artifact cache metadata.
- Incremental rebuild logic.
- Log format like:

```text
[1/5] Checking project config
[2/5] Kotlin unchanged, skipped
[3/5] Resources unchanged, skipped
[4/5] Packaging APK
[5/5] Installing app
Done in 1.2s
```

### Success Criteria

- A second build of an unchanged app skips unchanged tasks.
- Editing `src/main.kt` rebuilds only the affected Kotlin and packaging steps.
- Generated artifacts stay in Kairo-owned output directories.
- Logs clearly show executed, skipped, and failed tasks.

### Dependencies

- Phase 3 Android run path.
- Stable artifact directory policy.
- Stable task input and output definitions.

## Phase 5: Rust Native Module Support

### Goal

Allow Kairo apps to include Rust code in `src/native.rs` and call exported Rust functions from Kotlin.

### Scope

- Detect `src/native.rs`.
- Compile Rust to shared libraries.
- Generate Kotlin bridge code.
- Load native libraries at runtime.
- Support Android ABI builds.
- Support desktop native library builds when desktop support lands.
- Rebuild native code only when Rust inputs change.

### Deliverables

- `kairo-native` crate.
- Rust source detection.
- Rust build task integration.
- Native artifact layout.
- Kotlin `Native` wrapper generation.
- Basic type mapping for integers, floats, booleans, and strings if feasible.
- Example native counter or math app.

### Success Criteria

This Rust function:

```rust
#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

can be called from Kotlin:

```kotlin
val result = Native.add(10, 20)
```

Rust recompiles only when Rust source or native build configuration changes.

### Dependencies

- Phase 4 build graph and cache.
- Rust toolchain installation.
- Android Rust targets.
- Native bridge design from Phase 0.

## Phase 6: Desktop Target

### Goal

Support running the same Kairo Compose app on desktop.

### Scope

- Implement `kairo run desktop`.
- Integrate Compose Desktop.
- Create desktop window entrypoint.
- Reuse `kairo.compose.*` where possible.
- Add desktop-specific runtime module only where needed.
- Run on Windows first, then Linux and macOS.
- Prepare for desktop packaging in later work.

### Deliverables

- `kairo-desktop` crate or backend module.
- Desktop runtime module.
- Desktop run command.
- Desktop artifact layout.
- Shared counter example that runs on Android and desktop.
- Basic app metadata support such as name and icon placeholder.

### Success Criteria

The same `src/main.kt` can run with:

```bash
kairo run android
kairo run desktop
```

No user-facing desktop project boilerplate is required.

### Dependencies

- Phase 2 Compose facade.
- Phase 4 build graph.
- Compose Desktop dependencies.
- Desktop runtime design.

## Phase 7: Dependency System

### Goal

Allow users to add libraries without editing generated build files.

### Scope

- Implement `kairo add <package-name>`.
- Add dependency declarations to `kairo.toml`.
- Generate and maintain `kairo.lock`.
- Support curated dependency aliases.
- Resolve Maven dependencies.
- Cache downloaded dependencies.
- Detect basic version conflicts.
- Keep builds reproducible through lockfiles.

### Deliverables

- Dependency resolver module.
- `kairo add` command.
- Alias registry for common dependencies.
- `kairo.lock` format.
- Local dependency cache.
- Lockfile update flow.
- Documentation for direct coordinates and aliases.

### Success Criteria

Running:

```bash
kairo add ktor-client
kairo add compose-icons
```

updates:

```toml
[dependencies]
ktor-client = "latest"
compose-icons = "latest"
```

and writes a reproducible `kairo.lock`.

Builds use locked versions by default.

### Dependencies

- Phase 1 config parser.
- Phase 4 build engine.
- Maven metadata and artifact resolution design.
- Cache directory policy.

## Phase 8: Android Build Backend Expansion

### Goal

Build Android apps through Kairo-owned Rust tasks rather than Gradle.

### Scope

- AAPT2 integration.
- Manifest generation.
- Resource processing.
- D8 integration.
- Debug APK signing.
- Release APK signing groundwork.
- Debug and release build variants.
- Asset and resource directory support.
- AAB and R8 support later, after APK basics are stable.

### Deliverables

- `kairo-android` backend module.
- Manifest generator.
- Resource processor task.
- DEX task.
- APK packaging task.
- Signing task.
- Debug and release profile model.
- Android backend tests for minimal apps.

### Success Criteria

- Simple Android Compose apps build without Gradle.
- Kairo directly owns basic APK creation steps for supported cases.
- The backend can explain each Android build step in logs.
- Unsupported Android features fail with clear messages.

### Dependencies

- Phase 3 Android run MVP.
- Phase 4 build engine.
- Phase 7 dependency resolution for Android libraries.
- Deep Android packaging research from Phase 0.

## Phase 9: Performance Optimization

### Goal

Make Kairo development loops meaningfully faster and more predictable than traditional project-heavy Android workflows for small and medium apps.

### Scope

- Persistent build daemon.
- File watcher.
- Faster no-op builds.
- Kotlin compile cache strategy.
- Compose compiler cache strategy where possible.
- Rust native cache.
- Dependency prefetching.
- Parallel task tuning.
- Device install optimization.
- Better profiling and timing logs.

### Deliverables

- Build daemon prototype.
- Watch mode.
- Timing report output.
- Cache hit and miss reporting.
- Optimized install path for unchanged APK parts where feasible.
- Performance benchmark examples.
- Regression checks for build time.

### Success Criteria

- No-op builds are near-instant for small apps.
- Small edits trigger only necessary work.
- Logs make slow tasks easy to identify.
- Benchmark results show measurable improvement over the Phase 3 path.

### Dependencies

- Phase 4 build graph and cache.
- Phase 8 Android backend ownership.
- Stable artifact and dependency cache policy.

## Phase 10: Production Readiness

### Goal

Make Kairo suitable for real applications, release workflows, CI, and long-term maintainability.

### Scope

- Release APK builds.
- Signing configuration.
- App icons.
- Permissions.
- Assets and resources.
- Navigation helpers.
- Testing command.
- CI support.
- Crash log helpers.
- Build profiles.
- Documentation.
- Plugin system groundwork.
- Production examples.

### Deliverables

- `kairo build android --release`.
- Signing config support.
- App icon and resource support.
- Permission configuration in `kairo.toml`.
- `kairo test`.
- CI setup guide.
- Production docs.
- Examples for todo, calculator, and native module apps.
- Plugin architecture proposal.

### Success Criteria

- A developer can build and release a real Android app using Kairo.
- Release builds are reproducible.
- Docs cover installation, project creation, Android run, desktop run, dependencies, native modules, release signing, and troubleshooting.
- CI can run checks and builds without interactive setup.

### Dependencies

- Phase 8 Android backend expansion.
- Phase 7 dependency system.
- Phase 9 performance work.
- Stable user-facing config and command set.

## Suggested Execution Order

1. Complete Phase 0 before committing to low-level build architecture.
2. Build Phase 1 so users can create projects and validate their environment.
3. Build Phase 2 in parallel with the early Android build path if the module boundaries are clear.
4. Finish Phase 3 as the first true product proof: a Kairo project runs on Android.
5. Add Phase 4 to make builds incremental and create the foundation for future backend work.
6. Add Phase 5 once the build graph can isolate Rust inputs and outputs.
7. Add Phase 6 after the facade and build graph can support multiple targets.
8. Add Phase 7 before broad production usage because real apps need dependencies.
9. Expand Phase 8 gradually, replacing generated or delegated build steps one at a time.
10. Run Phase 9 continuously after Phase 4, with larger optimization pushes after Phase 8.
11. Treat Phase 10 as the release hardening track once Android, desktop, dependencies, and caching are stable.

## Risk Notes

### Gradle Replacement

Replacing Gradle's Android orchestration is the hardest long-term goal. Kairo should not attempt a full Gradle clone. The safer strategy is to start with an explicit Rust-owned build plan, then implement individual Android build tasks one by one after each step is understood and tested.

### Compose Compiler Integration

Compose depends on Kotlin compiler behavior and version compatibility. Kairo must pin compatible versions, validate them through `kairo doctor`, and provide clear errors when Kotlin, Compose, or compiler plugin versions do not match.

### Android Packaging

APK and AAB packaging involve resources, manifests, DEX files, assets, native libraries, signing, variants, and SDK metadata. Kairo should support a small debug APK path first, then add resources, signing, release variants, R8, and AAB support in controlled stages.

### Dependency Resolution

Maven dependency resolution includes transitive dependencies, exclusions, conflicts, metadata, and lockfiles. Kairo should begin with curated aliases and direct coordinates, then add robust conflict handling after the basic lockfile flow works.

### One-Import Facade Maintenance

`kairo.compose.*` gives Kairo a clean developer experience but creates maintenance work as Compose evolves. The facade should stay intentionally small at first and allow official Compose imports for advanced APIs.

### Performance Claims

Kairo can target faster small-app loops, especially through Rust startup, caching, and task scheduling. It should avoid promising universal speedups until benchmarks exist across realistic projects.

## Implementation Guardrails

- Keep the user project structure minimal.
- Keep generated Android and desktop files internal.
- Prefer clear failures over hidden magic.
- Add one backend capability at a time.
- Keep the facade compatible with official Compose imports.
- Make every generated artifact reproducible from `kairo.toml`, source files, and lockfiles.
- Treat `kairo doctor` as a core product feature, not an afterthought.
