use kairo_android::generate_android_workspace;
use kairo_build::{AndroidBuildRequest, execute_android_build};
use kairo_config::{KairoConfig, display_name_from_project_name, validate_project_name};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<u8, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    match args.first().map(String::as_str) {
        None | Some("-h") | Some("--help") => {
            print_help();
            Ok(0)
        }
        Some("-V") | Some("--version") => {
            println!("kairo {}", env!("CARGO_PKG_VERSION"));
            Ok(0)
        }
        Some("new") => {
            let name = args
                .get(1)
                .ok_or_else(|| "missing app name. Usage: kairo new <app-name>".to_string())?;
            command_new(name)?;
            Ok(0)
        }
        Some("doctor") => Ok(command_doctor()),
        Some("clean") => {
            command_clean()?;
            Ok(0)
        }
        Some("run") => {
            let target = args
                .get(1)
                .ok_or_else(|| "missing run target. Usage: kairo run android".to_string())?;

            match target.as_str() {
                "android" => {
                    command_run_android(&args[2..])?;
                    Ok(0)
                }
                other => Err(format!(
                    "unknown run target `{other}`. Supported target: android"
                )),
            }
        }
        Some(command) => Err(format!("unknown command `{command}`. Run `kairo --help`.")),
    }
}

fn print_help() {
    println!(
        "\
Kairo {}

Usage:
  kairo new <app-name>   Create a minimal Kairo project
  kairo run android      Prepare and run the Android target
  kairo doctor           Check local development tools
  kairo clean            Remove Kairo-generated output

Options:
  -h, --help             Show this help
  -V, --version          Show the CLI version",
        env!("CARGO_PKG_VERSION")
    );
}

fn command_new(name: &str) -> Result<(), String> {
    validate_project_name(name)?;

    let project_dir = env::current_dir()
        .map_err(|err| format!("failed to read current directory: {err}"))?
        .join(name);

    if project_dir.exists() {
        return Err(format!(
            "cannot create `{}` because it already exists",
            project_dir.display()
        ));
    }

    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir).map_err(|err| {
        format!(
            "failed to create project directory `{}`: {err}",
            project_dir.display()
        )
    })?;

    let config = KairoConfig::new(name);
    write_file(&project_dir.join("kairo.toml"), &config.to_toml())?;
    write_file(&src_dir.join("main.kt"), &starter_main_kt(name))?;

    println!("Created Kairo project `{name}`");
    println!("Next steps:");
    println!("  cd {name}");
    println!("  kairo doctor");
    println!("  kairo run android");

    Ok(())
}

fn command_run_android(options: &[String]) -> Result<(), String> {
    if let Some(option) = options.first() {
        return Err(format!(
            "unknown option `{option}`. Usage: kairo run android"
        ));
    }

    let project_dir =
        env::current_dir().map_err(|err| format!("failed to read current directory: {err}"))?;
    let config_path = project_dir.join("kairo.toml");

    if !config_path.is_file() {
        return Err(format!(
            "missing `{}`. Run this command from a Kairo project root.",
            config_path.display()
        ));
    }

    let config = KairoConfig::from_file(&config_path)?;
    let workspace = generate_android_workspace(&project_dir, &config)?;

    println!(
        "Prepared Rust-native Android workspace at `{}`",
        workspace.root.display()
    );
    println!("Build plan: `{}`", workspace.build_plan.display());

    let report = execute_android_build(&AndroidBuildRequest {
        project_dir,
        workspace_root: workspace.root,
        build_plan: workspace.build_plan,
        app_id: workspace.app_id,
        min_sdk: config.min_sdk,
        target_sdk: config.target_sdk,
    })?;

    for (index, step) in report.steps.iter().enumerate() {
        println!(
            "[{}/{}] {} ... {}",
            index + 1,
            report.steps.len(),
            step.name,
            step.status.label()
        );

        for detail in &step.details {
            println!("      {detail}");
        }
    }

    println!("Build report: `{}`", report.report_path.display());

    if report.is_complete() {
        Ok(())
    } else {
        Err(report.blocking_message().unwrap_or_else(|| {
            "Android build is incomplete; see the generated build report.".to_string()
        }))
    }
}

fn write_file(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|err| format!("failed to write `{}`: {err}", path.display()))
}

fn starter_main_kt(project_name: &str) -> String {
    let app_name = display_name_from_project_name(project_name);
    format!(
        "\
import kairo.compose.*

fun main() = kairoApp(\"{app_name}\") {{
    var count by state(0)

    Column {{
        Text(\"Hello Kairo\")
        Text(\"Count: $count\")

        Button(\"+\") {{
            count++
        }}
    }}
}}
"
    )
}

fn command_clean() -> Result<(), String> {
    let current_dir =
        env::current_dir().map_err(|err| format!("failed to read current directory: {err}"))?;
    let output_dir = current_dir.join(".kairo");

    if !output_dir.exists() {
        println!("Nothing to clean. `.kairo` does not exist.");
        return Ok(());
    }

    ensure_child_path(&current_dir, &output_dir)?;
    fs::remove_dir_all(&output_dir).map_err(|err| {
        format!(
            "failed to remove generated output `{}`: {err}",
            output_dir.display()
        )
    })?;

    println!("Removed generated output `{}`", output_dir.display());
    Ok(())
}

fn ensure_child_path(parent: &Path, child: &Path) -> Result<(), String> {
    let parent = parent
        .canonicalize()
        .map_err(|err| format!("failed to resolve `{}`: {err}", parent.display()))?;
    let child_parent = child
        .parent()
        .ok_or_else(|| format!("failed to resolve parent of `{}`", child.display()))?
        .canonicalize()
        .map_err(|err| format!("failed to resolve `{}`: {err}", child.display()))?;

    if child_parent != parent {
        return Err(format!(
            "refusing to clean `{}` because it is outside `{}`",
            child.display(),
            parent.display()
        ));
    }

    Ok(())
}

fn command_doctor() -> u8 {
    let mut report = DoctorReport::default();

    report.add(command_check(
        "Rust compiler",
        "rustc",
        &["--version"],
        "Install Rust from https://rustup.rs/",
    ));
    report.add(command_check(
        "Cargo",
        "cargo",
        &["--version"],
        "Install Rust from https://rustup.rs/",
    ));
    report.add(command_check_with_fallback(
        "Java",
        "java",
        &["-version"],
        &java_candidates(),
        "Install JDK 21 and add its bin directory to PATH.",
    ));
    report.add(command_check_with_fallback(
        "Kotlin compiler",
        "kotlinc",
        &["-version"],
        &kotlin_candidates(),
        "Install the Kotlin command-line compiler and add kotlinc/bin to PATH.",
    ));

    let android_sdk = find_android_sdk();
    match &android_sdk {
        Some(path) => {
            report.add(CheckResult {
                name: "Android SDK".to_string(),
                status: CheckStatus::Ok,
                detail: if env::var_os("ANDROID_HOME").is_some()
                    || env::var_os("ANDROID_SDK_ROOT").is_some()
                {
                    format!("found at {}", path.display())
                } else {
                    format!("found at default location {}", path.display())
                },
                hint: None,
            });

            report.add(sdk_file_check(
                "ADB",
                path,
                &["platform-tools", executable_name("adb")],
                "Install Android SDK Platform-Tools.",
            ));
            report.add(sdk_file_check(
                "Android emulator",
                path,
                &["emulator", executable_name("emulator")],
                "Install Android Emulator from Android Studio SDK Tools.",
            ));
            report.add(sdk_file_check(
                "sdkmanager",
                path,
                &[
                    "cmdline-tools",
                    "latest",
                    "bin",
                    executable_name("sdkmanager"),
                ],
                "Install Android SDK Command-line Tools (latest).",
            ));
            report.add(sdk_dir_check(
                "Android platforms",
                &path.join("platforms"),
                "Install at least one Android SDK platform.",
            ));
            report.add(sdk_dir_check(
                "Android build tools",
                &path.join("build-tools"),
                "Install Android SDK Build-Tools.",
            ));
        }
        None => report.add(CheckResult {
            name: "Android SDK".to_string(),
            status: CheckStatus::Fail,
            detail: "not found".to_string(),
            hint: Some(
                "Install Android Studio, then set ANDROID_HOME to the Android SDK path.".into(),
            ),
        }),
    }

    println!();
    report.print_summary();

    if report.failures == 0 { 0 } else { 1 }
}

fn command_check(name: &str, program: &str, args: &[&str], hint: &str) -> CheckResult {
    match run_tool(program, args) {
        Ok(output) if output.status.success() => CheckResult {
            name: name.to_string(),
            status: CheckStatus::Ok,
            detail: first_output_line(&output.stdout, &output.stderr),
            hint: None,
        },
        Ok(output) => CheckResult {
            name: name.to_string(),
            status: CheckStatus::Fail,
            detail: first_output_line(&output.stdout, &output.stderr),
            hint: Some(hint.to_string()),
        },
        Err(err) => CheckResult {
            name: name.to_string(),
            status: CheckStatus::Fail,
            detail: err.to_string(),
            hint: Some(hint.to_string()),
        },
    }
}

fn command_check_with_fallback(
    name: &str,
    program: &str,
    args: &[&str],
    fallback_paths: &[PathBuf],
    hint: &str,
) -> CheckResult {
    let path_result = command_check(name, program, args, hint);

    if path_result.status == CheckStatus::Ok {
        return path_result;
    }

    for path in fallback_paths {
        if !path.is_file() {
            continue;
        }

        match run_path_tool(path, args) {
            Ok(output) if output.status.success() => {
                return CheckResult {
                    name: name.to_string(),
                    status: CheckStatus::Ok,
                    detail: format!(
                        "{} ({})",
                        first_output_line(&output.stdout, &output.stderr),
                        path.display()
                    ),
                    hint: None,
                };
            }
            Ok(_) | Err(_) => {}
        }
    }

    path_result
}

fn run_path_tool(path: &Path, args: &[&str]) -> io::Result<Output> {
    if cfg!(windows) {
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();

        if extension.eq_ignore_ascii_case("bat") || extension.eq_ignore_ascii_case("cmd") {
            let mut shell_args = vec!["/C".to_string(), path.to_string_lossy().into_owned()];
            shell_args.extend(args.iter().map(|arg| arg.to_string()));

            let mut command = Command::new("cmd");
            command.args(shell_args);
            apply_discovered_java_home(&mut command);
            return command.output();
        }
    }

    let mut command = Command::new(path);
    command.args(args);
    apply_discovered_java_home(&mut command);
    command.output()
}

fn run_tool(program: &str, args: &[&str]) -> io::Result<Output> {
    match Command::new(program).args(args).output() {
        Ok(output) => Ok(output),
        Err(err) if cfg!(windows) && err.kind() == io::ErrorKind::NotFound => {
            let mut shell_args = vec!["/C", program];
            shell_args.extend(args);
            Command::new("cmd").args(shell_args).output()
        }
        Err(err) => Err(err),
    }
}

fn java_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(java_home) = env::var_os("JAVA_HOME") {
        candidates.push(
            PathBuf::from(java_home)
                .join("bin")
                .join(executable_name("java")),
        );
    }

    #[cfg(windows)]
    {
        candidates.extend(versioned_tool_candidates(
            Path::new(r"C:\Program Files\Eclipse Adoptium"),
            &["bin", executable_name("java")],
        ));
        candidates.extend(versioned_tool_candidates(
            Path::new(r"C:\Program Files\Java"),
            &["bin", executable_name("java")],
        ));
    }

    candidates
}

fn discovered_java_home() -> Option<PathBuf> {
    if let Some(java_home) = env::var_os("JAVA_HOME").map(PathBuf::from) {
        if java_home.exists() {
            return Some(java_home);
        }
    }

    java_candidates().into_iter().find_map(|java| {
        let home = java.parent()?.parent()?.to_path_buf();
        home.exists().then_some(home)
    })
}

fn apply_discovered_java_home(command: &mut Command) {
    let Some(java_home) = discovered_java_home() else {
        return;
    };

    command.env("JAVA_HOME", &java_home);

    let current_path = env::var_os("PATH").unwrap_or_default();
    let paths = std::iter::once(java_home.join("bin")).chain(env::split_paths(&current_path));

    if let Ok(path) = env::join_paths(paths) {
        command.env("PATH", path);
    }
}

fn kotlin_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(kotlin_home) = env::var_os("KOTLIN_HOME") {
        candidates.push(
            PathBuf::from(kotlin_home)
                .join("bin")
                .join(executable_name("kotlinc")),
        );
    }

    #[cfg(windows)]
    {
        candidates.push(PathBuf::from(r"C:\kotlin\kotlinc\bin").join(executable_name("kotlinc")));
    }

    candidates
}

fn versioned_tool_candidates(base: &Path, suffix: &[&str]) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(base) else {
        return Vec::new();
    };

    entries
        .filter_map(Result::ok)
        .map(|entry| {
            suffix
                .iter()
                .fold(entry.path(), |path, part| path.join(part))
        })
        .collect()
}

fn first_output_line(stdout: &[u8], stderr: &[u8]) -> String {
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr)
    );

    combined
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("command completed")
        .to_string()
}

fn find_android_sdk() -> Option<PathBuf> {
    env::var_os("ANDROID_HOME")
        .or_else(|| env::var_os("ANDROID_SDK_ROOT"))
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .or_else(default_android_sdk_path)
}

fn default_android_sdk_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("Android").join("Sdk"))
            .filter(|path| path.exists())
    }

    #[cfg(not(windows))]
    {
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join("Android").join("Sdk"))
            .filter(|path| path.exists())
    }
}

fn sdk_file_check(name: &str, sdk: &Path, parts: &[&str], hint: &str) -> CheckResult {
    let path = parts
        .iter()
        .fold(sdk.to_path_buf(), |path, part| path.join(part));
    if path.is_file() {
        CheckResult {
            name: name.to_string(),
            status: CheckStatus::Ok,
            detail: format!("found at {}", path.display()),
            hint: None,
        }
    } else {
        CheckResult {
            name: name.to_string(),
            status: CheckStatus::Fail,
            detail: format!("missing {}", path.display()),
            hint: Some(hint.to_string()),
        }
    }
}

fn sdk_dir_check(name: &str, path: &Path, hint: &str) -> CheckResult {
    match fs::read_dir(path) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                CheckResult {
                    name: name.to_string(),
                    status: CheckStatus::Ok,
                    detail: format!("found entries in {}", path.display()),
                    hint: None,
                }
            } else {
                CheckResult {
                    name: name.to_string(),
                    status: CheckStatus::Fail,
                    detail: format!("no entries in {}", path.display()),
                    hint: Some(hint.to_string()),
                }
            }
        }
        Err(err) => CheckResult {
            name: name.to_string(),
            status: CheckStatus::Fail,
            detail: format!("{} ({err})", path.display()),
            hint: Some(hint.to_string()),
        },
    }
}

fn executable_name(name: &'static str) -> &'static str {
    match (cfg!(windows), name) {
        (true, "adb") => "adb.exe",
        (true, "emulator") => "emulator.exe",
        (true, "java") => "java.exe",
        (true, "kotlinc") => "kotlinc.bat",
        (true, "sdkmanager") => "sdkmanager.bat",
        (_, "adb") => "adb",
        (_, "emulator") => "emulator",
        (_, "java") => "java",
        (_, "kotlinc") => "kotlinc",
        (_, "sdkmanager") => "sdkmanager",
        _ => name,
    }
}

#[derive(Default)]
struct DoctorReport {
    failures: usize,
}

impl DoctorReport {
    fn add(&mut self, result: CheckResult) {
        match result.status {
            CheckStatus::Ok => println!("[ok]   {}: {}", result.name, result.detail),
            CheckStatus::Fail => {
                self.failures += 1;
                println!("[fail] {}: {}", result.name, result.detail);
            }
        }

        if let Some(hint) = result.hint {
            println!("       hint: {hint}");
        }
    }

    fn print_summary(&self) {
        if self.failures == 0 {
            println!("Doctor summary: all checks passed.");
        } else {
            println!("Doctor summary: {} failure(s).", self.failures);
        }
    }
}

struct CheckResult {
    name: String,
    status: CheckStatus,
    detail: String,
    hint: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CheckStatus {
    Ok,
    Fail,
}
