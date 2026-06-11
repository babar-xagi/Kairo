//! Android backend foundation for Kairo.
//!
//! Phase 3 starts with a hidden Android workspace under `.kairo/android`.
//! The workspace is owned by Kairo's Rust build engine, not Gradle.

use kairo_config::KairoConfig;
use std::fs;
use std::path::{Path, PathBuf};

const APP_KT: &str =
    include_str!("../../../kotlin/kairo-compose/src/commonMain/kotlin/kairo/compose/App.kt");
const LAYOUT_KT: &str =
    include_str!("../../../kotlin/kairo-compose/src/commonMain/kotlin/kairo/compose/Layout.kt");
const MATERIAL_KT: &str =
    include_str!("../../../kotlin/kairo-compose/src/commonMain/kotlin/kairo/compose/Material.kt");
const MODIFIERS_KT: &str = include_str!(
    "../../../kotlin/kairo-compose/src/commonMain/kotlin/kairo/compose/ModifierHelpers.kt"
);
const STATE_KT: &str =
    include_str!("../../../kotlin/kairo-compose/src/commonMain/kotlin/kairo/compose/State.kt");
const TYPES_KT: &str =
    include_str!("../../../kotlin/kairo-compose/src/commonMain/kotlin/kairo/compose/Types.kt");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidWorkspace {
    pub root: PathBuf,
    pub apk_path: PathBuf,
    pub app_id: String,
    pub build_plan: PathBuf,
}

pub fn generate_android_workspace(
    project_dir: &Path,
    config: &KairoConfig,
) -> Result<AndroidWorkspace, String> {
    if !config.android_enabled {
        return Err("android target is disabled in kairo.toml".to_string());
    }

    let user_main = project_dir.join("src").join("main.kt");
    if !user_main.is_file() {
        return Err(format!("missing app entrypoint `{}`", user_main.display()));
    }

    let generated_root = project_dir.join(".kairo").join("android");
    reset_generated_workspace(&generated_root)?;

    let main_root = generated_root.join("src").join("main");
    let kotlin_root = main_root.join("kotlin");
    let values_root = main_root.join("res").join("values");
    let facade_root = kotlin_root.join("kairo").join("compose");
    let android_package_root = package_dir(&kotlin_root, &config.app_id);
    let user_package = user_package(&fs::read_to_string(&user_main).map_err(|err| {
        format!(
            "failed to read app entrypoint `{}`: {err}",
            user_main.display()
        )
    })?);
    let user_source_root = package_dir(&kotlin_root, &user_package.name);

    fs::create_dir_all(&facade_root)
        .map_err(|err| format!("failed to create `{}`: {err}", facade_root.display()))?;
    fs::create_dir_all(&android_package_root).map_err(|err| {
        format!(
            "failed to create `{}`: {err}",
            android_package_root.display()
        )
    })?;
    fs::create_dir_all(&user_source_root)
        .map_err(|err| format!("failed to create `{}`: {err}", user_source_root.display()))?;
    fs::create_dir_all(&values_root)
        .map_err(|err| format!("failed to create `{}`: {err}", values_root.display()))?;

    write_file(
        &generated_root.join("kairo-build-plan.md"),
        &build_plan(config),
    )?;
    write_file(
        &generated_root.join("kairo-build-plan.toml"),
        &build_plan_toml(config),
    )?;
    write_file(&main_root.join("AndroidManifest.xml"), &manifest(config))?;
    write_file(&values_root.join("styles.xml"), &styles())?;
    write_file(
        &android_package_root.join("MainActivity.kt"),
        &main_activity(config, &user_package.name),
    )?;
    write_file(
        &user_source_root.join("Main.kt"),
        &normalized_user_source(&user_main, &user_package)?,
    )?;
    write_facade(&facade_root)?;
    write_file(&generated_root.join("README.md"), &generated_readme(config))?;

    Ok(AndroidWorkspace {
        apk_path: generated_root
            .join("outputs")
            .join("apk")
            .join("debug")
            .join(format!(
                "{}-debug.apk",
                config.app_name.to_ascii_lowercase()
            )),
        root: generated_root,
        app_id: config.app_id.clone(),
        build_plan: project_dir
            .join(".kairo")
            .join("android")
            .join("kairo-build-plan.toml"),
    })
}

fn reset_generated_workspace(root: &Path) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }

    for name in [
        "src",
        "outputs",
        "intermediates",
        "app",
        "settings.gradle.kts",
        "build.gradle.kts",
        "gradle.properties",
        "kairo-build-plan.md",
        "kairo-build-plan.toml",
        "README.md",
    ] {
        let path = root.join(name);
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .map_err(|err| format!("failed to remove `{}`: {err}", path.display()))?;
        } else if path.is_file() {
            fs::remove_file(&path)
                .map_err(|err| format!("failed to remove `{}`: {err}", path.display()))?;
        }
    }

    Ok(())
}

fn styles() -> String {
    r#"<resources>
    <style name="AppTheme" parent="android:style/Theme.Material.Light.NoActionBar">
        <item name="android:windowNoTitle">true</item>
        <item name="android:windowActionBar">false</item>
    </style>
</resources>
"#
    .to_string()
}

fn write_file(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create `{}`: {err}", parent.display()))?;
    }

    fs::write(path, contents).map_err(|err| format!("failed to write `{}`: {err}", path.display()))
}

fn write_facade(root: &Path) -> Result<(), String> {
    for (name, contents) in [
        ("App.kt", APP_KT),
        ("Layout.kt", LAYOUT_KT),
        ("Material.kt", MATERIAL_KT),
        ("ModifierHelpers.kt", MODIFIERS_KT),
        ("State.kt", STATE_KT),
        ("Types.kt", TYPES_KT),
    ] {
        write_file(&root.join(name), contents)?;
    }

    Ok(())
}

fn package_dir(root: &Path, package_name: &str) -> PathBuf {
    package_name
        .split('.')
        .filter(|part| !part.is_empty())
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UserPackage {
    name: String,
    had_package: bool,
}

fn user_package(source: &str) -> UserPackage {
    let package = source.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix("package ")
            .map(str::trim)
            .filter(|package| !package.is_empty())
            .map(str::to_string)
    });

    match package {
        Some(name) => UserPackage {
            name,
            had_package: true,
        },
        None => UserPackage {
            name: "kairo.user".to_string(),
            had_package: false,
        },
    }
}

fn normalized_user_source(path: &Path, package: &UserPackage) -> Result<String, String> {
    let source = fs::read_to_string(path)
        .map_err(|err| format!("failed to read app entrypoint `{}`: {err}", path.display()))?;

    if package.had_package {
        Ok(source)
    } else {
        Ok(format!("package {}\n\n{source}", package.name))
    }
}

fn build_plan(config: &KairoConfig) -> String {
    format!(
        r#"# Kairo Rust-Native Android Build Plan

App: {app_name}
Application ID: {app_id}
Min SDK: {min_sdk}
Target SDK: {target_sdk}

This workspace is generated for Kairo's Rust build engine. It intentionally
does not contain Gradle files.

Planned Rust-owned tasks:

1. check-project
   Validate kairo.toml, src/main.kt, Android SDK, JDK, Kotlin compiler, and device state.

2. resolve-compose-dependencies
   Resolve Compose, AndroidX Activity, Kotlin, and compiler plugin artifacts without Gradle.

3. compile-kotlin
   Invoke the Kotlin compiler directly over generated source and the Kairo facade.

4. compile-compose
   Invoke the Compose compiler plugin directly through the Kotlin compiler.

5. process-resources
   Use AAPT2 from the Android SDK to compile/link resources and AndroidManifest.xml.

6. dex
   Use D8 from Android Build Tools to convert JVM bytecode to dex.

7. package-apk
   Package manifest, resources, dex, and assets into an APK.

8. sign-debug
   Sign the APK with a Kairo-managed debug key.

9. install
   Use adb install -r.

10. launch
    Use adb shell am start.
"#,
        app_name = config.app_name,
        app_id = config.app_id,
        min_sdk = config.min_sdk,
        target_sdk = config.target_sdk,
    )
}

fn build_plan_toml(config: &KairoConfig) -> String {
    format!(
        r#"[app]
name = "{app_name}"
id = "{app_id}"
version = "{version}"

[android]
min_sdk = {min_sdk}
target_sdk = {target_sdk}

[[task]]
name = "check-project"
status = "planned"

[[task]]
name = "resolve-compose-dependencies"
status = "planned"

[[task]]
name = "compile-kotlin"
status = "planned"

[[task]]
name = "compile-compose"
status = "planned"

[[task]]
name = "process-resources"
tool = "aapt2"
status = "planned"

[[task]]
name = "dex"
tool = "d8"
status = "planned"

[[task]]
name = "package-apk"
status = "planned"

[[task]]
name = "sign-debug"
tool = "apksigner"
status = "planned"

[[task]]
name = "install"
tool = "adb"
status = "planned"

[[task]]
name = "launch"
tool = "adb"
status = "planned"
"#,
        app_name = config.app_name,
        app_id = config.app_id,
        version = config.version,
        min_sdk = config.min_sdk,
        target_sdk = config.target_sdk,
    )
}

fn manifest(config: &KairoConfig) -> String {
    format!(
        r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{app_id}">
    <application
        android:allowBackup="true"
        android:label="{app_name}"
        android:supportsRtl="true"
        android:theme="@style/AppTheme">
        <activity
            android:name=".{activity_name}"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#,
        app_name = config.app_name,
        app_id = config.app_id,
        activity_name = "MainActivity",
    )
}

fn main_activity(config: &KairoConfig, user_package: &str) -> String {
    format!(
        r#"package {app_id}

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import kairo.compose.Text
import kairo.compose.currentKairoApp
import {user_package}.main

class MainActivity : ComponentActivity() {{
    override fun onCreate(savedInstanceState: Bundle?) {{
        super.onCreate(savedInstanceState)

        main()
        val app = currentKairoApp()

        setContent {{
            if (app == null) {{
                Text("No Kairo app registered")
            }} else {{
                app.content()
            }}
        }}
    }}
}}
"#,
        app_id = config.app_id,
        user_package = user_package,
    )
}

fn generated_readme(config: &KairoConfig) -> String {
    format!(
        r#"# Generated Android Workspace

This directory is generated by Kairo from the project root.

App: {app_name}
Application ID: {app_id}

Do not edit files here by hand. Edit `kairo.toml` and `src/main.kt`, then run Kairo again.

Current Phase 3 command:

```powershell
kairo run android
```

This workspace intentionally does not contain Gradle files. Kairo's Rust build
engine will execute the tasks described in `kairo-build-plan.toml`.
"#,
        app_name = config.app_name,
        app_id = config.app_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_default_user_package_when_missing() {
        let package = user_package("import kairo.compose.*\n\nfun main() = Unit\n");

        assert_eq!(package.name, "kairo.user");
        assert!(!package.had_package);
    }

    #[test]
    fn preserves_existing_user_package() {
        let package = user_package("package com.example.app\n\nfun main() = Unit\n");

        assert_eq!(package.name, "com.example.app");
        assert!(package.had_package);
    }
}
