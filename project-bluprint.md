# Kairo: A Compose-First, Rust-Powered Android & Desktop Toolchain

## 1. Vision

Kairo is a modern application development toolchain for building native Android and desktop apps using Jetpack Compose directly, while removing the traditional Android project mess and replacing Gradle-style slow workflows with a Rust-powered build system.

The goal is not to replace Jetpack Compose.

The goal is to make Compose development faster, cleaner, and more beginner-friendly by providing:

* Direct Jetpack Compose UI development
* A Rust-based CLI and build tool
* Minimal project structure
* No visible Gradle files
* No XML layouts
* No Android project boilerplate
* Faster build and run experience
* Simple imports through `kairo.compose.*`
* Optional Rust native modules for performance-heavy logic

## 2. Core Philosophy

Kairo follows this philosophy:

> Write Compose. Build with Rust. Ship native apps without Android project mess.

Kairo should feel as simple as:

```bash
kairo new counter
cd counter
kairo run android
kairo run desktop
```

A normal Android project can contain many configuration files, Gradle files, XML resources, manifests, and nested folders.

A Kairo project should start with only:

```text
counter/
  kairo.toml
  src/
    main.kt
```

Optional Rust native code:

```text
counter/
  kairo.toml
  src/
    main.kt
    native.rs
```

## 3. Unique Selling Point

Kairo is not another UI framework.

Kairo is a Compose-first toolchain.

Its unique value is:

1. Compose directly, not XML.
2. Rust build tool, not Gradle-first workflow.
3. Tiny project structure.
4. One-import Compose developer experience.
5. Android and desktop first.
6. Optional Rust native performance layer.
7. Faster development loop.

## 4. Target Users

Kairo is designed for:

* Kotlin developers who like Jetpack Compose but hate Android project complexity
* Beginners who want to build Android apps without understanding Gradle first
* Rust developers who want to build mobile/desktop apps with Kotlin UI
* Indie developers who want fast build/run cycles
* Developers building small to medium apps quickly
* Future AI/mobile developers who want Kotlin UI + Rust native speed

## 5. What Kairo Is

Kairo is:

```text
Jetpack Compose UI
+ Rust CLI
+ Rust build system
+ simplified project layout
+ optional Rust native module support
+ Android/Desktop app packaging
```

## 6. What Kairo Is Not

Kairo is not:

* A new UI language
* An HTML/CSS framework
* A WebView wrapper
* A Flutter clone
* A replacement for Compose
* A toy DSL hiding Compose completely

Kairo keeps Compose as the main UI language.

## 7. Final Technical Direction

The final direction is:

```text
UI: Direct Jetpack Compose
Language: Kotlin
Build Tool: Rust
CLI: Rust
Android Target: Native Android app
Desktop Target: Compose Desktop / Compose Multiplatform
Optional Native Core: Rust
Project Style: Minimal
Gradle: Replaced gradually by Kairo Build Engine
```

## 8. Developer Experience Goal

Normal Compose code often requires multiple imports:

```kotlin
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.foundation.layout.*
```

Kairo should simplify this to:

```kotlin
import kairo.compose.*
```

Then the user writes:

```kotlin
import kairo.compose.*

fun main() = kairoApp("Counter") {
    var count by state(0)

    Column {
        Text("Count: $count")

        Button("+") {
            count++
        }
    }
}
```

This should compile to a native Android app using Jetpack Compose internally.

## 9. Why `kairo.compose.*` Instead of `androidx.compose.*`

Kotlin wildcard imports do not automatically import all subpackages.

So this:

```kotlin
import androidx.compose.*
```

will not automatically include:

```kotlin
androidx.compose.material3.*
androidx.compose.runtime.*
androidx.compose.foundation.layout.*
```

Therefore, Kairo should create its own facade package:

```kotlin
import kairo.compose.*
```

This package will expose the most commonly used Compose APIs, wrappers, helpers, and state utilities.

Advanced users can still import official Compose APIs when needed:

```kotlin
import kairo.compose.*
import androidx.compose.material3.NavigationBar
```

## 10. Example Kairo App

### File structure

```text
counter/
  kairo.toml
  src/
    main.kt
```

### `kairo.toml`

```toml
[app]
name = "Counter"
id = "com.babar.counter"
version = "0.1.0"

[targets]
android = true
desktop = true

[android]
min_sdk = 26
target_sdk = 36

[build]
engine = "kairo"
cache = true
parallel = true

[ui]
toolkit = "compose"
import_facade = "kairo.compose"
```

### `src/main.kt`

```kotlin
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
```

## 11. Optional Rust Native Module

Kairo should allow a Rust file for performance-heavy logic.

### File structure

```text
counter/
  kairo.toml
  src/
    main.kt
    native.rs
```

### `src/native.rs`

```rust
#[no_mangle]
pub extern "C" fn increment(x: i32) -> i32 {
    x + 1
}
```

### Kotlin usage

```kotlin
import kairo.compose.*

fun main() = kairoApp("Counter") {
    var count by state(0)

    Column {
        Text("Count: $count")

        Button("+") {
            count = Native.increment(count)
        }
    }
}
```

Kairo should generate the Kotlin native bridge automatically.

## 12. High-Level Architecture

```text
User Project
  kairo.toml
  src/main.kt
  src/native.rs optional
        |
        v
Kairo CLI
  project commands
  file watcher
  dev server
  build runner
        |
        v
Kairo Build Engine
  dependency resolver
  build graph
  incremental cache
  task scheduler
  Kotlin/Compose compiler orchestration
  Android packaging orchestration
  Rust native build orchestration
        |
        v
Generated Output
  APK for Android
  Desktop executable/package
```

## 13. Main Components

### 13.1 Kairo CLI

The CLI is written in Rust.

Commands:

```bash
kairo new app-name
kairo run android
kairo run desktop
kairo build android
kairo build desktop
kairo clean
kairo doctor
kairo add package-name
kairo test
```

The CLI handles:

* Project creation
* Build execution
* File watching
* Error reporting
* Android device detection
* Desktop run
* Native Rust module build
* Dependency management
* Cache control

### 13.2 Kairo Build Engine

The build engine is the heart of Kairo.

It should be written in Rust and designed to be faster than traditional Gradle workflows.

Responsibilities:

* Build graph creation
* Incremental rebuilds
* Parallel task execution
* File hashing
* Dependency locking
* Compiler orchestration
* Artifact caching
* Android APK packaging
* Desktop packaging
* Rust native library compilation

### 13.3 Kairo Compose Facade

Package:

```kotlin
kairo.compose.*
```

Purpose:

* Reduce imports
* Provide simple state helpers
* Provide commonly used Compose components
* Wrap common Material3/Foundation APIs
* Keep advanced Compose compatibility

Example:

```kotlin
import kairo.compose.*
```

Instead of:

```kotlin
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.foundation.layout.*
```

### 13.4 Kairo Android Backend

The Android backend handles:

* Android manifest generation
* SDK configuration
* Compose compiler integration
* Kotlin compilation
* Resource handling
* APK packaging
* Signing
* Debug/release modes
* Device install/run

### 13.5 Kairo Desktop Backend

The desktop backend handles:

* Compose Desktop support
* Windows build output
* Linux build output
* macOS build output
* Desktop packaging
* App metadata

### 13.6 Kairo Native Bridge

The native bridge connects Kotlin and Rust.

It handles:

* Rust compilation
* Shared library generation
* JNI or C ABI bridge
* Kotlin wrapper generation
* Type mapping
* Android ABI support

## 14. Build System Strategy

Kairo’s most ambitious goal is replacing Gradle slowly with a Rust-native build tool.

This should not be attempted as a complete Gradle clone from day one.

Instead, Kairo should start with a small focused build engine for simple Compose apps and then grow feature by feature.

## 15. Phase-by-Phase Roadmap

## Phase 0: Research and Design

Goal: Understand the Android build pipeline deeply.

Tasks:

* Study Kotlin compilation
* Study Compose compiler setup
* Study Android SDK tools
* Study APK packaging
* Study AAPT2
* Study D8/R8
* Study signing
* Study Compose Desktop
* Study Rust JNI integration

Output:

* Technical design document
* Build pipeline map
* Minimal command flow
* First project template

## Phase 1: CLI MVP

Goal: Create the first working Rust CLI.

Commands:

```bash
kairo new hello
kairo doctor
kairo clean
```

Features:

* Create minimal project
* Generate `kairo.toml`
* Generate `src/main.kt`
* Validate Android SDK installation
* Validate Kotlin/JDK setup
* Print clear errors

Success criteria:

```bash
kairo new hello
```

creates:

```text
hello/
  kairo.toml
  src/main.kt
```

## Phase 2: Compose Facade MVP

Goal: Create `kairo.compose.*`.

Features:

* `kairoApp`
* `state`
* `Text`
* `Button`
* `Column`
* `Row`
* `Box`
* `Spacer`
* Basic padding helpers
* Basic alignment helpers

Example:

```kotlin
import kairo.compose.*

fun main() = kairoApp("Hello") {
    Column {
        Text("Hello Kairo")

        Button("Click") {
            println("Clicked")
        }
    }
}
```

Success criteria:

* User writes only `import kairo.compose.*`
* Basic Compose app compiles
* Official Compose can still be imported when needed

## Phase 3: Android Run MVP

Goal: Run a basic Compose app on Android using Kairo.

Commands:

```bash
kairo run android
```

Features:

* Generate necessary Android project files internally
* Compile Kotlin/Compose code
* Package debug APK
* Install on connected emulator/device
* Launch app

Important:

At this stage, Kairo may still use Android SDK tools and possibly temporary generated build files internally, but the user should not see project mess.

Success criteria:

```bash
kairo run android
```

builds and launches a simple Counter app.

## Phase 4: Rust Build Engine Core

Goal: Start replacing build orchestration with Rust-native tasks.

Features:

* File hashing
* Build graph
* Incremental cache
* Parallel task scheduler
* Task logs
* Artifact directory
* Source change detection
* Skip unchanged tasks

Success criteria:

* Second build is much faster than first build
* Only changed files trigger rebuilds
* Clear build output

Example output:

```text
[1/5] Checking project config
[2/5] Kotlin unchanged, skipped
[3/5] Resources unchanged, skipped
[4/5] Packaging APK
[5/5] Installing app
Done in 1.2s
```

## Phase 5: Rust Native Module Support

Goal: Allow Rust code inside Kairo apps.

Features:

* Detect `src/native.rs`
* Compile Rust to shared library
* Generate Kotlin bridge
* Support Android ABIs
* Support desktop native library
* Rebuild Rust only when Rust code changes

Example:

```rust
#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

Kotlin:

```kotlin
val result = Native.add(10, 20)
```

Success criteria:

* Kotlin can call Rust function
* Rust builds only when Rust source changes

## Phase 6: Desktop Target

Goal: Support desktop apps.

Command:

```bash
kairo run desktop
```

Features:

* Compose Desktop integration
* Window creation
* App icon
* Desktop run
* Windows/Linux/macOS packaging later

Success criteria:

Same `main.kt` can run on desktop.

## Phase 7: Dependency System

Goal: Add dependency management without exposing Gradle.

Command:

```bash
kairo add ktor-client
kairo add compose-icons
```

Files:

```text
kairo.toml
kairo.lock
```

Features:

* Dependency aliases
* Version locking
* Maven dependency resolution
* Local cache
* Conflict detection

Example:

```toml
[dependencies]
ktor-client = "latest"
compose-icons = "latest"
```

Success criteria:

* User can add libraries without editing Gradle
* Build remains reproducible through `kairo.lock`

## Phase 8: Android Build Backend Expansion

Goal: Reduce Gradle dependency further.

Features:

* AAPT2 integration
* Manifest generation
* Resource processing
* D8 integration
* APK signing
* Debug/release variants
* ProGuard/R8 later
* AAB support later

Success criteria:

* Simple apps build without Gradle
* Kairo handles APK creation directly for basic cases

## Phase 9: Performance Optimization

Goal: Make Kairo builds significantly faster.

Focus areas:

* Persistent build daemon
* Hot reload-like development loop
* Kotlin compile cache
* Compose compiler caching
* Parallel tasks
* Rust native cache
* Dependency prefetching
* File watcher
* Device install optimization

Target:

* Small apps should feel dramatically faster than traditional Android Gradle builds
* Large apps should still be stable and predictable

## Phase 10: Production Readiness

Goal: Make Kairo usable for real apps.

Features:

* Release APK
* Signing configuration
* App icons
* Permissions
* Assets
* Resources
* Navigation helpers
* Testing
* CI support
* Crash logs
* Build profiles
* Documentation
* Plugin system

Success criteria:

* A developer can build and release a real Android app using Kairo

## 16. Suggested Internal Repository Structure

```text
kairo/
  crates/
    kairo-cli/
    kairo-build/
    kairo-android/
    kairo-desktop/
    kairo-native/
    kairo-config/
    kairo-cache/
  kotlin/
    kairo-compose/
    kairo-runtime/
    kairo-android-runtime/
    kairo-desktop-runtime/
  templates/
    app-basic/
    app-rust-native/
  examples/
    counter/
    todo/
    calculator/
  docs/
    blueprint.md
    build-system.md
    compose-facade.md
    android-backend.md
```

## 17. Kairo Commands

### Create project

```bash
kairo new counter
```

### Run Android

```bash
kairo run android
```

### Run Desktop

```bash
kairo run desktop
```

### Build release APK

```bash
kairo build android --release
```

### Clean build cache

```bash
kairo clean
```

### Diagnose environment

```bash
kairo doctor
```

### Add dependency

```bash
kairo add package-name
```

## 18. First MVP App Example

```kotlin
import kairo.compose.*

fun main() = kairoApp("Todo") {
    var task by state("")
    val tasks = listState<String>()

    Column {
        Text("My Tasks")

        TextField(
            value = task,
            placeholder = "Enter task",
            onChange = { task = it }
        )

        Button("Add") {
            if (task.isNotBlank()) {
                tasks.add(task)
                task = ""
            }
        }

        tasks.forEach {
            Text(it)
        }
    }
}
```

## 19. Branding

Recommended name:

# Kairo

Tagline:

> Compose apps. Rust speed. Zero project mess.

Alternative taglines:

* Write Compose. Build faster.
* Native apps without Android build pain.
* Kotlin UI. Rust build power.
* Jetpack Compose made clean.
* Android development without Gradle chaos.

## 20. Pros

### 20.1 Very clean developer experience

Users do not need to understand Android project structure at the start.

### 20.2 Direct Compose

No custom HTML syntax. No XML. No WebView.

### 20.3 Rust-powered speed

Rust can provide fast CLI startup, efficient file watching, caching, hashing, and parallel task execution.

### 20.4 Minimal project structure

Simple apps can start with only:

```text
kairo.toml
src/main.kt
```

### 20.5 One-import experience

Users can write:

```kotlin
import kairo.compose.*
```

### 20.6 Optional Rust native code

Apps can use Rust for performance-heavy modules.

### 20.7 Strong positioning

Kairo has a clear identity:

```text
Compose-first development with Rust-powered builds.
```

## 21. Cons and Risks

### 21.1 Replacing Gradle is very hard

Gradle and Android Gradle Plugin handle many complex Android tasks. Replacing them fully requires deep Android build knowledge.

### 21.2 Compose compiler integration is difficult

Compose depends on Kotlin compiler/plugin behavior. Kairo must handle this correctly.

### 21.3 Dependency resolution is complex

Maven dependency resolution, transitive dependencies, conflicts, and version locking are hard to implement.

### 21.4 Android packaging is complex

APK/AAB building involves resources, DEX, manifest, signing, assets, native libraries, and variants.

### 21.5 One-import facade needs maintenance

`kairo.compose.*` must be updated as Compose evolves.

### 21.6 10x faster is not guaranteed everywhere

Kairo can aim for much faster small-app development loops, but large production apps may not always be 10x faster.

## 22. Practical Final Strategy

The best strategy is:

```text
Do not build everything at once.
Start with a small working Compose app.
Make Kairo run it.
Then replace build pieces one by one.
```

Recommended order:

1. Rust CLI
2. Minimal project structure
3. `kairo.compose.*`
4. Android run
5. Build cache
6. Rust native support
7. Desktop support
8. Dependency management
9. Gradle replacement features
10. Production release support

## 23. Final Product Definition

Kairo is a Compose-first, Rust-powered application toolchain that allows developers to build native Android and desktop apps with minimal project structure, faster development loops, simplified imports, and optional Rust native performance modules.

Its final goal is:

```text
kairo new app
kairo run android
```

and the user only writes:

```kotlin
import kairo.compose.*

fun main() = kairoApp("Hello") {
    Text("Hello Kairo")
}
```

## 24. Final Decision

Kairo should use:

```text
Direct Jetpack Compose
Rust CLI
Rust build engine
kairo.compose.* facade
Minimal project layout
Android + desktop first
Optional Rust native modules
Gradle replacement step by step
```

The mission is simple:

> Make Jetpack Compose development as simple as Flutter’s CLI experience, but with Kotlin UI, Rust build speed, and no Android project mess.
