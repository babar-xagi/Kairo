//! Rust-native build engine foundation for Kairo.
//!
//! This crate owns build execution. Phase 3 begins with Android toolchain
//! discovery and project validation, then later phases will add dependency
//! resolution, Kotlin/Compose compilation, AAPT2, D8, APK packaging, signing,
//! install, and launch.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidBuildRequest {
    pub project_dir: PathBuf,
    pub workspace_root: PathBuf,
    pub build_plan: PathBuf,
    pub app_id: String,
    pub min_sdk: u32,
    pub target_sdk: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildReport {
    pub steps: Vec<BuildStepResult>,
    pub report_path: PathBuf,
}

impl BuildReport {
    pub fn is_complete(&self) -> bool {
        self.steps
            .iter()
            .all(|step| step.status == BuildStepStatus::Done)
    }

    pub fn blocking_message(&self) -> Option<String> {
        self.steps
            .iter()
            .find(|step| step.status == BuildStepStatus::Blocked)
            .map(|step| match step.details.as_slice() {
                [] => format!("{} is blocked", step.name),
                details => format!("{} is blocked: {}", step.name, details.join(" ")),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildStepResult {
    pub name: &'static str,
    pub status: BuildStepStatus,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildStepStatus {
    Done,
    Blocked,
    Planned,
}

impl BuildStepStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Blocked => "blocked",
            Self::Planned => "planned",
        }
    }
}

const ANDROID_STEPS: [&str; 10] = [
    "check-project",
    "resolve-compose-dependencies",
    "compile-kotlin",
    "compile-compose",
    "process-resources",
    "dex",
    "package-apk",
    "sign-debug",
    "install",
    "launch",
];

pub fn execute_android_build(request: &AndroidBuildRequest) -> Result<BuildReport, String> {
    let mut steps = Vec::new();

    let toolchain = match check_project(request) {
        Ok(toolchain) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[0],
                status: BuildStepStatus::Done,
                details: toolchain.details(),
            });
            toolchain
        }
        Err(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[0],
                status: BuildStepStatus::Blocked,
                details,
            });
            push_planned_steps(&mut steps, 1);
            return finish_report(request, steps);
        }
    };

    let resolution = match resolve_compose_dependencies(request, &toolchain) {
        Ok(resolution) if resolution.is_complete() => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[1],
                status: BuildStepStatus::Done,
                details: resolution.details(),
            });
            resolution
        }
        Ok(resolution) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[1],
                status: BuildStepStatus::Blocked,
                details: resolution.details(),
            });
            push_planned_steps(&mut steps, 2);
            return finish_report(request, steps);
        }
        Err(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[1],
                status: BuildStepStatus::Blocked,
                details,
            });
            push_planned_steps(&mut steps, 2);
            return finish_report(request, steps);
        }
    };

    match compile_kotlin(request, &toolchain, &resolution) {
        Ok(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[2],
                status: BuildStepStatus::Done,
                details,
            });
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[3],
                status: BuildStepStatus::Done,
                details: vec![
                    "Compose compiler plugin was passed directly to kotlinc.".to_string(),
                ],
            });
        }
        Err(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[2],
                status: BuildStepStatus::Blocked,
                details,
            });
            push_planned_steps(&mut steps, 3);
            return finish_report(request, steps);
        }
    }

    match process_resources(request, &toolchain, &resolution) {
        Ok(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[4],
                status: BuildStepStatus::Done,
                details,
            });
        }
        Err(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[4],
                status: BuildStepStatus::Blocked,
                details,
            });
            push_planned_steps(&mut steps, 5);
            return finish_report(request, steps);
        }
    }

    match dex(request, &toolchain, &resolution) {
        Ok(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[5],
                status: BuildStepStatus::Done,
                details,
            });
        }
        Err(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[5],
                status: BuildStepStatus::Blocked,
                details,
            });
            push_planned_steps(&mut steps, 6);
            return finish_report(request, steps);
        }
    }

    match package_apk(request) {
        Ok(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[6],
                status: BuildStepStatus::Done,
                details,
            });
        }
        Err(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[6],
                status: BuildStepStatus::Blocked,
                details,
            });
            push_planned_steps(&mut steps, 7);
            return finish_report(request, steps);
        }
    }

    match sign_debug(request, &toolchain) {
        Ok(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[7],
                status: BuildStepStatus::Done,
                details,
            });
        }
        Err(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[7],
                status: BuildStepStatus::Blocked,
                details,
            });
            push_planned_steps(&mut steps, 8);
            return finish_report(request, steps);
        }
    }

    match install_apk(request, &toolchain) {
        Ok(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[8],
                status: BuildStepStatus::Done,
                details,
            });
        }
        Err(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[8],
                status: BuildStepStatus::Blocked,
                details,
            });
            push_planned_steps(&mut steps, 9);
            return finish_report(request, steps);
        }
    }

    match launch_apk(request, &toolchain) {
        Ok(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[9],
                status: BuildStepStatus::Done,
                details,
            });
        }
        Err(details) => {
            steps.push(BuildStepResult {
                name: ANDROID_STEPS[9],
                status: BuildStepStatus::Blocked,
                details,
            });
            return finish_report(request, steps);
        }
    }

    finish_report(request, steps)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtifactRequirement {
    id: &'static str,
    group: &'static str,
    name: &'static str,
    version: &'static str,
    extension: ArtifactExtension,
}

impl ArtifactRequirement {
    fn coordinate(&self) -> String {
        format!("{}:{}:{}", self.group, self.name, self.version)
    }

    fn relative_artifact(&self) -> PathBuf {
        self.group
            .split('.')
            .fold(PathBuf::new(), |path, part| path.join(part))
            .join(self.name)
            .join(self.version)
            .join(format!(
                "{}-{}.{}",
                self.name,
                self.version,
                self.extension.as_str()
            ))
    }

    fn url_path(&self) -> String {
        format!(
            "{}/{}/{}/{}-{}.{}",
            self.group.replace('.', "/"),
            self.name,
            self.version,
            self.name,
            self.version,
            self.extension.as_str()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArtifactExtension {
    Aar,
    Jar,
}

impl ArtifactExtension {
    fn as_str(self) -> &'static str {
        match self {
            Self::Aar => "aar",
            Self::Jar => "jar",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtifactResolution {
    requirement: ArtifactRequirement,
    path: Option<PathBuf>,
    source: ArtifactSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArtifactSource {
    Downloaded,
    KairoCache,
    KotlinInstall,
    LocalMavenCache,
    Missing,
}

impl ArtifactSource {
    fn label(self) -> &'static str {
        match self {
            Self::Downloaded => "downloaded",
            Self::KairoCache => "kairo-cache",
            Self::KotlinInstall => "kotlin-install",
            Self::LocalMavenCache => "local-maven-cache",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DependencyResolution {
    cache_root: PathBuf,
    manifest_path: PathBuf,
    artifacts: Vec<ArtifactResolution>,
}

impl DependencyResolution {
    fn is_complete(&self) -> bool {
        self.artifacts
            .iter()
            .all(|artifact| artifact.path.is_some())
    }

    fn details(&self) -> Vec<String> {
        let mut details = vec![
            format!("Kairo artifact cache: {}", self.cache_root.display()),
            format!("Artifact manifest: {}", self.manifest_path.display()),
        ];

        for artifact in &self.artifacts {
            match &artifact.path {
                Some(path) => details.push(format!(
                    "resolved {} ({}) from {} at {}",
                    artifact.requirement.id,
                    artifact.requirement.coordinate(),
                    artifact.source.label(),
                    path.display()
                )),
                None => details.push(format!(
                    "missing {} ({})",
                    artifact.requirement.id,
                    artifact.requirement.coordinate()
                )),
            }
        }

        if !self.is_complete() {
            details.push(
                "Some direct artifacts are missing. Check network access or repository/version availability."
                    .to_string(),
            );
        }

        details
    }
}

fn resolve_compose_dependencies(
    request: &AndroidBuildRequest,
    toolchain: &AndroidToolchain,
) -> Result<DependencyResolution, Vec<String>> {
    let cache_root = request.workspace_root.join("cache").join("artifacts");
    fs::create_dir_all(&cache_root).map_err(|err| {
        vec![format!(
            "failed to create artifact cache `{}`: {err}",
            cache_root.display()
        )]
    })?;

    let requirements = mvp_artifact_requirements();
    let artifacts = requirements
        .into_iter()
        .map(|requirement| {
            let (path, source) = resolve_artifact(&requirement, toolchain, &cache_root);
            ArtifactResolution {
                requirement,
                path,
                source,
            }
        })
        .collect::<Vec<_>>();

    let manifest_path = request.workspace_root.join("cache").join("artifacts.toml");
    write_artifact_manifest(&manifest_path, &artifacts)?;

    Ok(DependencyResolution {
        cache_root,
        manifest_path,
        artifacts,
    })
}

fn compile_kotlin(
    request: &AndroidBuildRequest,
    toolchain: &AndroidToolchain,
    resolution: &DependencyResolution,
) -> Result<Vec<String>, Vec<String>> {
    let source_root = request
        .workspace_root
        .join("src")
        .join("main")
        .join("kotlin");
    let classes_dir = request
        .workspace_root
        .join("intermediates")
        .join("kotlin")
        .join("classes");
    let report_path = request
        .workspace_root
        .join("outputs")
        .join("reports")
        .join("kotlin-compile.txt");

    recreate_dir(&classes_dir).map_err(|err| vec![err])?;

    let mut sources = Vec::new();
    collect_files_with_extension(&source_root, "kt", &mut sources).map_err(|err| {
        vec![format!(
            "failed to collect Kotlin sources under `{}`: {err}",
            source_root.display()
        )]
    })?;
    sources.sort();

    if sources.is_empty() {
        return Err(vec![format!(
            "no Kotlin sources found under `{}`",
            source_root.display()
        )]);
    }

    let compose_plugin = required_artifact_path(resolution, "compose-compiler-plugin")?;
    let mut classpath_entries = vec![toolchain.android_jar.clone()];
    let mut extracted_aars = 0usize;

    for artifact in &resolution.artifacts {
        let Some(path) = &artifact.path else {
            return Err(vec![format!(
                "cannot compile because {} is unresolved",
                artifact.requirement.coordinate()
            )]);
        };

        match artifact.requirement.extension {
            ArtifactExtension::Jar => {
                if artifact.requirement.id != "compose-compiler-plugin" {
                    classpath_entries.push(path.clone());
                }
            }
            ArtifactExtension::Aar => {
                if let Some(classes_jar) =
                    extract_aar_classes(request, artifact, path).map_err(|err| vec![err])?
                {
                    classpath_entries.push(classes_jar);
                    extracted_aars += 1;
                }
            }
        }
    }

    let classpath = env::join_paths(classpath_entries.iter()).map_err(|err| {
        vec![format!(
            "failed to assemble Kotlin classpath from resolved artifacts: {err}"
        )]
    })?;

    let argfile_path = request
        .workspace_root
        .join("intermediates")
        .join("kotlin")
        .join("kotlinc.args");
    write_kotlinc_argfile(
        &argfile_path,
        &classpath,
        &classes_dir,
        &compose_plugin,
        &sources,
    )
    .map_err(|err| vec![err])?;

    let args = vec![OsString::from(format!("@{}", argfile_path.display()))];

    let output = match run_program_os(&toolchain.kotlin, &args) {
        Ok(output) => output,
        Err(err) => {
            let details = vec![
                format!(
                    "failed to launch kotlinc at `{}`: {err}",
                    toolchain.kotlin.display()
                ),
                format!("Compiler report: {}", report_path.display()),
            ];
            let _ = write_process_report(
                &report_path,
                "Kairo Kotlin Compile Report",
                &toolchain.kotlin,
                &args,
                None,
            );
            return Err(details);
        }
    };

    write_process_report(
        &report_path,
        "Kairo Kotlin Compile Report",
        &toolchain.kotlin,
        &args,
        Some(&output),
    )
    .map_err(|err| vec![err])?;

    let mut details = vec![
        format!("Kotlin sources: {}", sources.len()),
        format!("Classpath entries: {}", classpath_entries.len()),
        format!("Extracted AAR classes jars: {extracted_aars}"),
        format!("Kotlin argfile: {}", argfile_path.display()),
        format!("Output classes: {}", classes_dir.display()),
        format!("Compiler report: {}", report_path.display()),
    ];

    if output.status.success() {
        return Ok(details);
    }

    details.push(format!(
        "kotlinc exited with {}",
        process_status_label(&output)
    ));
    details.extend(process_excerpt(&output, 8));

    Err(details)
}

fn process_resources(
    request: &AndroidBuildRequest,
    toolchain: &AndroidToolchain,
    resolution: &DependencyResolution,
) -> Result<Vec<String>, Vec<String>> {
    let main_root = request.workspace_root.join("src").join("main");
    let manifest = main_root.join("AndroidManifest.xml");
    let res_dir = main_root.join("res");
    let output_root = request.workspace_root.join("intermediates").join("aapt2");
    let compiled_resources = output_root.join("compiled-resources.zip");
    let linked_resources = output_root.join("resources.ap_");
    let r_java_dir = output_root.join("r-java");
    let r_classes_dir = output_root.join("r-classes");
    let compile_report = request
        .workspace_root
        .join("outputs")
        .join("reports")
        .join("aapt2-compile.txt");
    let link_report = request
        .workspace_root
        .join("outputs")
        .join("reports")
        .join("aapt2-link.txt");
    let javac_report = request
        .workspace_root
        .join("outputs")
        .join("reports")
        .join("javac-r.txt");

    recreate_dir(&output_root).map_err(|err| vec![err])?;
    fs::create_dir_all(&r_java_dir).map_err(|err| {
        vec![format!(
            "failed to create `{}`: {err}",
            r_java_dir.display()
        )]
    })?;
    fs::create_dir_all(&r_classes_dir).map_err(|err| {
        vec![format!(
            "failed to create `{}`: {err}",
            r_classes_dir.display()
        )]
    })?;

    let compile_args = vec![
        OsString::from("compile"),
        OsString::from("--dir"),
        res_dir.as_os_str().to_os_string(),
        OsString::from("-o"),
        compiled_resources.as_os_str().to_os_string(),
    ];
    let compile_output = run_program_os(&toolchain.aapt2, &compile_args).map_err(|err| {
        vec![
            format!(
                "failed to launch AAPT2 at `{}`: {err}",
                toolchain.aapt2.display()
            ),
            format!("AAPT2 compile report: {}", compile_report.display()),
        ]
    })?;
    write_process_report(
        &compile_report,
        "Kairo AAPT2 Compile Report",
        &toolchain.aapt2,
        &compile_args,
        Some(&compile_output),
    )
    .map_err(|err| vec![err])?;

    if !compile_output.status.success() {
        let mut details = vec![
            format!(
                "AAPT2 resource compile failed with {}",
                process_status_label(&compile_output)
            ),
            format!("AAPT2 compile report: {}", compile_report.display()),
        ];
        details.extend(process_excerpt(&compile_output, 8));
        return Err(details);
    }

    let aar_resources = compile_aar_resources(request, toolchain, resolution, &output_root)
        .map_err(|err| {
            let mut details = vec!["failed to compile dependency AAR resources".to_string()];
            details.extend(err);
            details
        })?;

    let mut extra_packages = aar_resources
        .iter()
        .map(|unit| unit.package_name.clone())
        .collect::<Vec<_>>();
    extra_packages.sort();
    extra_packages.dedup();

    let mut link_args = vec![
        OsString::from("link"),
        OsString::from("-I"),
        toolchain.android_jar.as_os_str().to_os_string(),
        OsString::from("--manifest"),
        manifest.as_os_str().to_os_string(),
        OsString::from("--java"),
        r_java_dir.as_os_str().to_os_string(),
        OsString::from("--min-sdk-version"),
        OsString::from(request.min_sdk.to_string()),
        OsString::from("--target-sdk-version"),
        OsString::from(request.target_sdk.to_string()),
    ];

    if !extra_packages.is_empty() {
        link_args.push(OsString::from("--extra-packages"));
        link_args.push(OsString::from(extra_packages.join(":")));
        link_args.push(OsString::from("--auto-add-overlay"));
    }

    for unit in &aar_resources {
        link_args.push(OsString::from("-R"));
        link_args.push(unit.compiled_resources.as_os_str().to_os_string());
    }

    link_args.push(OsString::from("-o"));
    link_args.push(linked_resources.as_os_str().to_os_string());
    link_args.push(compiled_resources.as_os_str().to_os_string());

    let link_output = run_program_os(&toolchain.aapt2, &link_args).map_err(|err| {
        vec![
            format!(
                "failed to launch AAPT2 at `{}`: {err}",
                toolchain.aapt2.display()
            ),
            format!("AAPT2 link report: {}", link_report.display()),
        ]
    })?;
    write_process_report(
        &link_report,
        "Kairo AAPT2 Link Report",
        &toolchain.aapt2,
        &link_args,
        Some(&link_output),
    )
    .map_err(|err| vec![err])?;

    if !link_output.status.success() {
        let mut details = vec![
            format!(
                "AAPT2 resource link failed with {}",
                process_status_label(&link_output)
            ),
            format!("AAPT2 link report: {}", link_report.display()),
        ];
        details.extend(process_excerpt(&link_output, 8));
        return Err(details);
    }

    let generated_r_sources = compile_generated_r_java(
        request,
        toolchain,
        &r_java_dir,
        &r_classes_dir,
        &javac_report,
    )?;

    Ok(vec![
        format!("Compiled resources: {}", compiled_resources.display()),
        format!("Compiled dependency AAR resources: {}", aar_resources.len()),
        format!("Linked resource package: {}", linked_resources.display()),
        format!("Generated R.java directory: {}", r_java_dir.display()),
        format!("Compiled R.java sources: {generated_r_sources}"),
        format!("Generated R classes directory: {}", r_classes_dir.display()),
        format!("AAPT2 compile report: {}", compile_report.display()),
        format!("AAPT2 link report: {}", link_report.display()),
        format!("R javac report: {}", javac_report.display()),
    ])
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AarResourceUnit {
    package_name: String,
    compiled_resources: PathBuf,
}

fn compile_aar_resources(
    request: &AndroidBuildRequest,
    toolchain: &AndroidToolchain,
    resolution: &DependencyResolution,
    output_root: &Path,
) -> Result<Vec<AarResourceUnit>, Vec<String>> {
    let mut units = Vec::new();

    for artifact in &resolution.artifacts {
        if artifact.requirement.extension != ArtifactExtension::Aar {
            continue;
        }

        let Some(path) = &artifact.path else {
            return Err(vec![format!(
                "cannot process resources because {} is unresolved",
                artifact.requirement.coordinate()
            )]);
        };

        let Some((package_name, res_dir)) =
            extract_aar_resources(request, artifact, path, output_root).map_err(|err| vec![err])?
        else {
            continue;
        };

        let compiled_resources = output_root
            .join("aar-compiled")
            .join(format!("{}.zip", artifact.requirement.id));
        if let Some(parent) = compiled_resources.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| vec![format!("failed to create `{}`: {err}", parent.display())])?;
        }

        let report_path = request
            .workspace_root
            .join("outputs")
            .join("reports")
            .join(format!("aapt2-compile-{}.txt", artifact.requirement.id));
        let compile_args = vec![
            OsString::from("compile"),
            OsString::from("--dir"),
            res_dir.as_os_str().to_os_string(),
            OsString::from("-o"),
            compiled_resources.as_os_str().to_os_string(),
        ];
        let output = run_program_os(&toolchain.aapt2, &compile_args).map_err(|err| {
            vec![
                format!(
                    "failed to launch AAPT2 at `{}` for {}: {err}",
                    toolchain.aapt2.display(),
                    artifact.requirement.coordinate()
                ),
                format!("AAPT2 dependency compile report: {}", report_path.display()),
            ]
        })?;
        write_process_report(
            &report_path,
            "Kairo AAPT2 Dependency Compile Report",
            &toolchain.aapt2,
            &compile_args,
            Some(&output),
        )
        .map_err(|err| vec![err])?;

        if !output.status.success() {
            let mut details = vec![
                format!(
                    "AAPT2 dependency resource compile failed for {} with {}",
                    artifact.requirement.coordinate(),
                    process_status_label(&output)
                ),
                format!("AAPT2 dependency compile report: {}", report_path.display()),
            ];
            details.extend(process_excerpt(&output, 8));
            return Err(details);
        }

        units.push(AarResourceUnit {
            package_name,
            compiled_resources,
        });
    }

    Ok(units)
}

fn extract_aar_resources(
    request: &AndroidBuildRequest,
    artifact: &ArtifactResolution,
    aar_path: &Path,
    output_root: &Path,
) -> Result<Option<(String, PathBuf)>, String> {
    let file = fs::File::open(aar_path)
        .map_err(|err| format!("failed to open AAR `{}`: {err}", aar_path.display()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|err| {
        format!(
            "failed to read AAR `{}` as a zip: {err}",
            aar_path.display()
        )
    })?;

    let manifest = read_zip_entry_to_string(&mut archive, "AndroidManifest.xml")?;
    let package_name = extract_manifest_package(&manifest).ok_or_else(|| {
        format!(
            "AAR `{}` for {} has resources but no manifest package",
            aar_path.display(),
            artifact.requirement.coordinate()
        )
    })?;

    let res_dir = output_root
        .join("aar-res")
        .join(artifact.requirement.id)
        .join("res");
    recreate_dir(&res_dir)?;

    let mut copied = 0usize;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|err| {
            format!(
                "failed to read entry {index} from AAR `{}`: {err}",
                aar_path.display()
            )
        })?;

        if entry.is_dir() {
            continue;
        }

        let Some(relative_path) = aar_res_relative_path(entry.name()) else {
            continue;
        };

        let destination = res_dir.join(relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create `{}`: {err}", parent.display()))?;
        }

        let mut output = fs::File::create(&destination)
            .map_err(|err| format!("failed to create `{}`: {err}", destination.display()))?;
        io::copy(&mut entry, &mut output).map_err(|err| {
            format!(
                "failed to extract `{}` from `{}`: {err}",
                entry.name(),
                aar_path.display()
            )
        })?;
        copied += 1;
    }

    if copied == 0 {
        let _ = fs::remove_dir_all(output_root.join("aar-res").join(artifact.requirement.id));
        return Ok(None);
    }

    let marker_path = output_root
        .join("aar-res")
        .join(artifact.requirement.id)
        .join("package.txt");
    fs::write(&marker_path, &package_name)
        .map_err(|err| format!("failed to write `{}`: {err}", marker_path.display()))?;

    let _ = request;
    Ok(Some((package_name, res_dir)))
}

fn read_zip_entry_to_string(
    archive: &mut zip::ZipArchive<fs::File>,
    name: &str,
) -> Result<String, String> {
    let mut entry = archive
        .by_name(name)
        .map_err(|err| format!("failed to read `{name}` from AAR: {err}"))?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|err| format!("failed to decode `{name}` from AAR as UTF-8: {err}"))?;
    Ok(text)
}

fn extract_manifest_package(manifest: &str) -> Option<String> {
    for marker in ["package=\"", "package='"] {
        let start = manifest.find(marker)? + marker.len();
        let quote = marker.chars().last()?;
        let rest = &manifest[start..];
        let end = rest.find(quote)?;
        let package = rest[..end].trim();
        if !package.is_empty() {
            return Some(package.to_string());
        }
    }

    None
}

fn aar_res_relative_path(name: &str) -> Option<PathBuf> {
    let relative = name.strip_prefix("res/")?;
    if relative.is_empty() || relative.ends_with('/') {
        return None;
    }

    let mut path = PathBuf::new();
    for part in relative.split('/') {
        if part.is_empty() || part == "." || part == ".." || part.contains('\\') {
            return None;
        }
        path.push(part);
    }

    Some(path)
}

fn compile_generated_r_java(
    request: &AndroidBuildRequest,
    toolchain: &AndroidToolchain,
    r_java_dir: &Path,
    r_classes_dir: &Path,
    report_path: &Path,
) -> Result<usize, Vec<String>> {
    recreate_dir(r_classes_dir).map_err(|err| vec![err])?;

    let mut sources = Vec::new();
    collect_files_with_extension(r_java_dir, "java", &mut sources).map_err(|err| {
        vec![format!(
            "failed to collect generated R.java sources under `{}`: {err}",
            r_java_dir.display()
        )]
    })?;
    sources.sort();

    if sources.is_empty() {
        write_text_report(
            report_path,
            "Kairo R Java Compile Report",
            "No generated R.java sources were emitted by AAPT2.",
        )
        .map_err(|err| vec![err])?;
        return Ok(0);
    }

    let mut args = vec![
        OsString::from("--release"),
        OsString::from("8"),
        OsString::from("-encoding"),
        OsString::from("UTF-8"),
        OsString::from("-classpath"),
        toolchain.android_jar.as_os_str().to_os_string(),
        OsString::from("-d"),
        r_classes_dir.as_os_str().to_os_string(),
    ];
    args.extend(
        sources
            .iter()
            .map(|source| source.as_os_str().to_os_string()),
    );

    let output = run_program_os(&toolchain.javac, &args).map_err(|err| {
        vec![
            format!(
                "failed to launch javac at `{}`: {err}",
                toolchain.javac.display()
            ),
            format!("R javac report: {}", report_path.display()),
        ]
    })?;
    write_process_report(
        report_path,
        "Kairo R Java Compile Report",
        &toolchain.javac,
        &args,
        Some(&output),
    )
    .map_err(|err| vec![err])?;

    if !output.status.success() {
        let mut details = vec![
            format!("R.java javac failed with {}", process_status_label(&output)),
            format!("R javac report: {}", report_path.display()),
        ];
        details.extend(process_excerpt(&output, 8));
        return Err(details);
    }

    let _ = request;
    Ok(sources.len())
}

fn dex(
    request: &AndroidBuildRequest,
    toolchain: &AndroidToolchain,
    resolution: &DependencyResolution,
) -> Result<Vec<String>, Vec<String>> {
    let classes_dir = request
        .workspace_root
        .join("intermediates")
        .join("kotlin")
        .join("classes");
    let app_classes_jar = request
        .workspace_root
        .join("intermediates")
        .join("kotlin")
        .join("app-classes.jar");
    let r_classes_dir = request
        .workspace_root
        .join("intermediates")
        .join("aapt2")
        .join("r-classes");
    let r_classes_jar = request
        .workspace_root
        .join("intermediates")
        .join("aapt2")
        .join("r-classes.jar");
    let dex_dir = request.workspace_root.join("intermediates").join("dex");
    let report_path = request
        .workspace_root
        .join("outputs")
        .join("reports")
        .join("d8.txt");

    zip_directory(&classes_dir, &app_classes_jar).map_err(|err| vec![err])?;
    recreate_dir(&dex_dir).map_err(|err| vec![err])?;

    let mut program_inputs = vec![app_classes_jar.clone()];
    let mut generated_r_classes = 0usize;
    if r_classes_dir.is_dir() {
        let mut class_files = Vec::new();
        collect_files_with_extension(&r_classes_dir, "class", &mut class_files).map_err(|err| {
            vec![format!(
                "failed to collect generated R classes under `{}`: {err}",
                r_classes_dir.display()
            )]
        })?;

        if !class_files.is_empty() {
            generated_r_classes = class_files.len();
            zip_directory(&r_classes_dir, &r_classes_jar).map_err(|err| vec![err])?;
            program_inputs.push(r_classes_jar.clone());
        }
    }

    let mut runtime_inputs = 0usize;
    for artifact in &resolution.artifacts {
        if artifact.requirement.id == "compose-compiler-plugin" {
            continue;
        }

        let Some(path) = &artifact.path else {
            return Err(vec![format!(
                "cannot dex because {} is unresolved",
                artifact.requirement.coordinate()
            )]);
        };

        match artifact.requirement.extension {
            ArtifactExtension::Jar => {
                program_inputs.push(path.clone());
                runtime_inputs += 1;
            }
            ArtifactExtension::Aar => {
                if let Some(classes_jar) =
                    extract_aar_classes(request, artifact, path).map_err(|err| vec![err])?
                {
                    program_inputs.push(classes_jar);
                    runtime_inputs += 1;
                }
            }
        }
    }

    let mut args = vec![
        OsString::from("--lib"),
        toolchain.android_jar.as_os_str().to_os_string(),
        OsString::from("--min-api"),
        OsString::from(request.min_sdk.to_string()),
        OsString::from("--output"),
        dex_dir.as_os_str().to_os_string(),
    ];
    args.extend(
        program_inputs
            .iter()
            .map(|input| input.as_os_str().to_os_string()),
    );

    let output = run_program_os(&toolchain.d8, &args).map_err(|err| {
        vec![
            format!("failed to launch D8 at `{}`: {err}", toolchain.d8.display()),
            format!("D8 report: {}", report_path.display()),
        ]
    })?;
    write_process_report(
        &report_path,
        "Kairo D8 Report",
        &toolchain.d8,
        &args,
        Some(&output),
    )
    .map_err(|err| vec![err])?;

    if !output.status.success() {
        let mut details = vec![
            format!("D8 failed with {}", process_status_label(&output)),
            format!("D8 report: {}", report_path.display()),
        ];
        details.extend(process_excerpt(&output, 8));
        return Err(details);
    }

    Ok(vec![
        format!("App classes jar: {}", app_classes_jar.display()),
        format!("Generated R classes: {generated_r_classes}"),
        format!("Program inputs: {}", program_inputs.len()),
        format!("Runtime library inputs: {runtime_inputs}"),
        format!("Dex output directory: {}", dex_dir.display()),
        format!("D8 report: {}", report_path.display()),
    ])
}

fn package_apk(request: &AndroidBuildRequest) -> Result<Vec<String>, Vec<String>> {
    let resources_package = request
        .workspace_root
        .join("intermediates")
        .join("aapt2")
        .join("resources.ap_");
    let dex_dir = request.workspace_root.join("intermediates").join("dex");
    let apk_dir = request
        .workspace_root
        .join("outputs")
        .join("apk")
        .join("debug");
    let unsigned_apk = apk_dir.join(format!("{}-unsigned.apk", request.app_id));

    if let Some(parent) = unsigned_apk.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| vec![format!("failed to create `{}`: {err}", parent.display())])?;
    }

    let mut dex_files = Vec::new();
    collect_files_with_extension(&dex_dir, "dex", &mut dex_files).map_err(|err| {
        vec![format!(
            "failed to collect dex files under `{}`: {err}",
            dex_dir.display()
        )]
    })?;
    dex_files.sort();

    if dex_files.is_empty() {
        return Err(vec![format!(
            "no dex files found under `{}`",
            dex_dir.display()
        )]);
    }

    create_unsigned_apk(&resources_package, &dex_files, &unsigned_apk).map_err(|err| vec![err])?;

    Ok(vec![
        format!("Resource package: {}", resources_package.display()),
        format!("Dex files: {}", dex_files.len()),
        format!("Unsigned APK: {}", unsigned_apk.display()),
    ])
}

fn sign_debug(
    request: &AndroidBuildRequest,
    toolchain: &AndroidToolchain,
) -> Result<Vec<String>, Vec<String>> {
    let unsigned_apk = request
        .workspace_root
        .join("outputs")
        .join("apk")
        .join("debug")
        .join(format!("{}-unsigned.apk", request.app_id));
    let signed_apk = request
        .workspace_root
        .join("outputs")
        .join("apk")
        .join("debug")
        .join(format!("{}-debug.apk", request.app_id));
    let signing_dir = request.workspace_root.join("cache").join("signing");
    let keystore = signing_dir.join("debug.keystore");
    let keytool_report = request
        .workspace_root
        .join("outputs")
        .join("reports")
        .join("keytool-debug-keystore.txt");
    let sign_report = request
        .workspace_root
        .join("outputs")
        .join("reports")
        .join("apksigner-sign.txt");
    let verify_report = request
        .workspace_root
        .join("outputs")
        .join("reports")
        .join("apksigner-verify.txt");

    fs::create_dir_all(&signing_dir).map_err(|err| {
        vec![format!(
            "failed to create `{}`: {err}",
            signing_dir.display()
        )]
    })?;

    let keytool = keytool_for_java(&toolchain.java);
    if !keystore.is_file() {
        if keytool != Path::new("keytool") && !keytool.is_file() {
            return Err(vec![format!(
                "keytool not found at `{}` for debug keystore creation",
                keytool.display()
            )]);
        }

        let keytool_args = vec![
            OsString::from("-genkeypair"),
            OsString::from("-keystore"),
            keystore.as_os_str().to_os_string(),
            OsString::from("-storepass"),
            OsString::from("android"),
            OsString::from("-keypass"),
            OsString::from("android"),
            OsString::from("-alias"),
            OsString::from("androiddebugkey"),
            OsString::from("-keyalg"),
            OsString::from("RSA"),
            OsString::from("-keysize"),
            OsString::from("2048"),
            OsString::from("-validity"),
            OsString::from("10000"),
            OsString::from("-dname"),
            OsString::from("CN=Android Debug,O=Kairo,C=US"),
            OsString::from("-storetype"),
            OsString::from("PKCS12"),
            OsString::from("-noprompt"),
        ];
        let output = run_program_os(&keytool, &keytool_args).map_err(|err| {
            vec![
                format!("failed to launch keytool at `{}`: {err}", keytool.display()),
                format!("Keytool report: {}", keytool_report.display()),
            ]
        })?;
        write_process_report(
            &keytool_report,
            "Kairo Debug Keystore Report",
            &keytool,
            &keytool_args,
            Some(&output),
        )
        .map_err(|err| vec![err])?;

        if !output.status.success() {
            let mut details = vec![
                format!("keytool failed with {}", process_status_label(&output)),
                format!("Keytool report: {}", keytool_report.display()),
            ];
            details.extend(process_excerpt(&output, 8));
            return Err(details);
        }
    }

    let _ = fs::remove_file(&signed_apk);
    let sign_args = vec![
        OsString::from("sign"),
        OsString::from("--ks"),
        keystore.as_os_str().to_os_string(),
        OsString::from("--ks-key-alias"),
        OsString::from("androiddebugkey"),
        OsString::from("--ks-pass"),
        OsString::from("pass:android"),
        OsString::from("--key-pass"),
        OsString::from("pass:android"),
        OsString::from("--out"),
        signed_apk.as_os_str().to_os_string(),
        unsigned_apk.as_os_str().to_os_string(),
    ];
    let sign_output = run_program_os(&toolchain.apksigner, &sign_args).map_err(|err| {
        vec![
            format!(
                "failed to launch apksigner at `{}`: {err}",
                toolchain.apksigner.display()
            ),
            format!("APK signer report: {}", sign_report.display()),
        ]
    })?;
    write_process_report(
        &sign_report,
        "Kairo APK Sign Report",
        &toolchain.apksigner,
        &sign_args,
        Some(&sign_output),
    )
    .map_err(|err| vec![err])?;

    if !sign_output.status.success() {
        let mut details = vec![
            format!(
                "apksigner sign failed with {}",
                process_status_label(&sign_output)
            ),
            format!("APK signer report: {}", sign_report.display()),
        ];
        details.extend(process_excerpt(&sign_output, 8));
        return Err(details);
    }

    let verify_args = vec![
        OsString::from("verify"),
        OsString::from("--verbose"),
        signed_apk.as_os_str().to_os_string(),
    ];
    let verify_output = run_program_os(&toolchain.apksigner, &verify_args).map_err(|err| {
        vec![
            format!(
                "failed to launch apksigner at `{}`: {err}",
                toolchain.apksigner.display()
            ),
            format!("APK verify report: {}", verify_report.display()),
        ]
    })?;
    write_process_report(
        &verify_report,
        "Kairo APK Verify Report",
        &toolchain.apksigner,
        &verify_args,
        Some(&verify_output),
    )
    .map_err(|err| vec![err])?;

    if !verify_output.status.success() {
        let mut details = vec![
            format!(
                "apksigner verify failed with {}",
                process_status_label(&verify_output)
            ),
            format!("APK verify report: {}", verify_report.display()),
        ];
        details.extend(process_excerpt(&verify_output, 8));
        return Err(details);
    }

    Ok(vec![
        format!("Debug keystore: {}", keystore.display()),
        format!("Unsigned APK: {}", unsigned_apk.display()),
        format!("Signed APK: {}", signed_apk.display()),
        format!("APK signer report: {}", sign_report.display()),
        format!("APK verify report: {}", verify_report.display()),
    ])
}

fn install_apk(
    request: &AndroidBuildRequest,
    toolchain: &AndroidToolchain,
) -> Result<Vec<String>, Vec<String>> {
    let signed_apk = request
        .workspace_root
        .join("outputs")
        .join("apk")
        .join("debug")
        .join(format!("{}-debug.apk", request.app_id));
    let report_path = request
        .workspace_root
        .join("outputs")
        .join("reports")
        .join("adb-install.txt");
    let uninstall_report_path = request
        .workspace_root
        .join("outputs")
        .join("reports")
        .join("adb-uninstall-incompatible.txt");
    let retry_report_path = request
        .workspace_root
        .join("outputs")
        .join("reports")
        .join("adb-install-retry.txt");

    let args = vec![
        OsString::from("install"),
        OsString::from("-r"),
        signed_apk.as_os_str().to_os_string(),
    ];
    let output = run_program_os(&toolchain.adb, &args).map_err(|err| {
        vec![
            format!(
                "failed to launch adb at `{}`: {err}",
                toolchain.adb.display()
            ),
            format!("ADB install report: {}", report_path.display()),
        ]
    })?;
    write_process_report(
        &report_path,
        "Kairo ADB Install Report",
        &toolchain.adb,
        &args,
        Some(&output),
    )
    .map_err(|err| vec![err])?;

    if !output.status.success()
        && process_output_text(&output).contains("INSTALL_FAILED_UPDATE_INCOMPATIBLE")
    {
        let uninstall_args = vec![OsString::from("uninstall"), OsString::from(&request.app_id)];
        let uninstall_output = run_program_os(&toolchain.adb, &uninstall_args).map_err(|err| {
            vec![
                format!(
                    "failed to launch adb at `{}` for uninstall retry: {err}",
                    toolchain.adb.display()
                ),
                format!("ADB uninstall report: {}", uninstall_report_path.display()),
            ]
        })?;
        write_process_report(
            &uninstall_report_path,
            "Kairo ADB Uninstall Incompatible Report",
            &toolchain.adb,
            &uninstall_args,
            Some(&uninstall_output),
        )
        .map_err(|err| vec![err])?;

        if !uninstall_output.status.success()
            && !process_output_text(&uninstall_output).contains("Unknown package")
        {
            let mut details = vec![
                "adb install found an incompatible existing signature, but uninstall failed"
                    .to_string(),
                format!("ADB install report: {}", report_path.display()),
                format!("ADB uninstall report: {}", uninstall_report_path.display()),
            ];
            details.extend(process_excerpt(&uninstall_output, 8));
            return Err(details);
        }

        let retry_output = run_program_os(&toolchain.adb, &args).map_err(|err| {
            vec![
                format!(
                    "failed to launch adb at `{}` for install retry: {err}",
                    toolchain.adb.display()
                ),
                format!("ADB install retry report: {}", retry_report_path.display()),
            ]
        })?;
        write_process_report(
            &retry_report_path,
            "Kairo ADB Install Retry Report",
            &toolchain.adb,
            &args,
            Some(&retry_output),
        )
        .map_err(|err| vec![err])?;

        if retry_output.status.success() {
            return Ok(vec![
                "Removed incompatible existing install before retry.".to_string(),
                format!("Installed APK: {}", signed_apk.display()),
                format!("ADB install report: {}", report_path.display()),
                format!("ADB uninstall report: {}", uninstall_report_path.display()),
                format!("ADB install retry report: {}", retry_report_path.display()),
            ]);
        }

        let mut details = vec![
            format!(
                "adb install retry failed with {}",
                process_status_label(&retry_output)
            ),
            format!("ADB install report: {}", report_path.display()),
            format!("ADB uninstall report: {}", uninstall_report_path.display()),
            format!("ADB install retry report: {}", retry_report_path.display()),
        ];
        details.extend(process_excerpt(&retry_output, 8));
        return Err(details);
    }

    if !output.status.success() {
        let mut details = vec![
            format!("adb install failed with {}", process_status_label(&output)),
            format!("ADB install report: {}", report_path.display()),
        ];
        details.extend(process_excerpt(&output, 8));
        return Err(details);
    }

    Ok(vec![
        format!("Installed APK: {}", signed_apk.display()),
        format!("ADB install report: {}", report_path.display()),
    ])
}

fn launch_apk(
    request: &AndroidBuildRequest,
    toolchain: &AndroidToolchain,
) -> Result<Vec<String>, Vec<String>> {
    let report_path = request
        .workspace_root
        .join("outputs")
        .join("reports")
        .join("adb-launch.txt");
    let component = format!("{}/.MainActivity", request.app_id);
    let args = vec![
        OsString::from("shell"),
        OsString::from("am"),
        OsString::from("start"),
        OsString::from("-n"),
        OsString::from(&component),
    ];
    let output = run_program_os(&toolchain.adb, &args).map_err(|err| {
        vec![
            format!(
                "failed to launch adb at `{}`: {err}",
                toolchain.adb.display()
            ),
            format!("ADB launch report: {}", report_path.display()),
        ]
    })?;
    write_process_report(
        &report_path,
        "Kairo ADB Launch Report",
        &toolchain.adb,
        &args,
        Some(&output),
    )
    .map_err(|err| vec![err])?;

    if !output.status.success() {
        let mut details = vec![
            format!("adb launch failed with {}", process_status_label(&output)),
            format!("ADB launch report: {}", report_path.display()),
        ];
        details.extend(process_excerpt(&output, 8));
        return Err(details);
    }

    Ok(vec![
        format!("Launched component: {component}"),
        format!("ADB launch report: {}", report_path.display()),
    ])
}

fn mvp_artifact_requirements() -> Vec<ArtifactRequirement> {
    vec![
        ArtifactRequirement {
            id: "kotlin-stdlib",
            group: "org.jetbrains.kotlin",
            name: "kotlin-stdlib",
            version: "2.4.0",
            extension: ArtifactExtension::Jar,
        },
        ArtifactRequirement {
            id: "compose-compiler-plugin",
            group: "org.jetbrains.kotlin",
            name: "compose-compiler-plugin",
            version: "2.4.0",
            extension: ArtifactExtension::Jar,
        },
        ArtifactRequirement {
            id: "androidx-activity-compose",
            group: "androidx.activity",
            name: "activity-compose",
            version: "1.11.0",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-activity",
            group: "androidx.activity",
            name: "activity",
            version: "1.11.0",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-annotation",
            group: "androidx.annotation",
            name: "annotation",
            version: "1.8.1",
            extension: ArtifactExtension::Jar,
        },
        ArtifactRequirement {
            id: "androidx-annotation-jvm",
            group: "androidx.annotation",
            name: "annotation-jvm",
            version: "1.9.1",
            extension: ArtifactExtension::Jar,
        },
        ArtifactRequirement {
            id: "androidx-annotation-experimental",
            group: "androidx.annotation",
            name: "annotation-experimental",
            version: "1.4.1",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "jspecify",
            group: "org.jspecify",
            name: "jspecify",
            version: "1.0.0",
            extension: ArtifactExtension::Jar,
        },
        ArtifactRequirement {
            id: "androidx-collection-jvm",
            group: "androidx.collection",
            name: "collection-jvm",
            version: "1.5.0",
            extension: ArtifactExtension::Jar,
        },
        ArtifactRequirement {
            id: "androidx-core",
            group: "androidx.core",
            name: "core",
            version: "1.15.0",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-core-ktx",
            group: "androidx.core",
            name: "core-ktx",
            version: "1.13.1",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-core-viewtree",
            group: "androidx.core",
            name: "core-viewtree",
            version: "1.0.0",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-customview-poolingcontainer",
            group: "androidx.customview",
            name: "customview-poolingcontainer",
            version: "1.0.0",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-arch-core-common",
            group: "androidx.arch.core",
            name: "core-common",
            version: "2.2.0",
            extension: ArtifactExtension::Jar,
        },
        ArtifactRequirement {
            id: "androidx-arch-core-runtime",
            group: "androidx.arch.core",
            name: "core-runtime",
            version: "2.2.0",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-lifecycle-common",
            group: "androidx.lifecycle",
            name: "lifecycle-common-jvm",
            version: "2.9.4",
            extension: ArtifactExtension::Jar,
        },
        ArtifactRequirement {
            id: "androidx-lifecycle-common-java8",
            group: "androidx.lifecycle",
            name: "lifecycle-common-java8",
            version: "2.9.4",
            extension: ArtifactExtension::Jar,
        },
        ArtifactRequirement {
            id: "androidx-lifecycle-runtime",
            group: "androidx.lifecycle",
            name: "lifecycle-runtime-android",
            version: "2.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-lifecycle-runtime-ktx",
            group: "androidx.lifecycle",
            name: "lifecycle-runtime-ktx-android",
            version: "2.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-lifecycle-runtime-compose",
            group: "androidx.lifecycle",
            name: "lifecycle-runtime-compose-android",
            version: "2.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-lifecycle-viewmodel",
            group: "androidx.lifecycle",
            name: "lifecycle-viewmodel-android",
            version: "2.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-lifecycle-viewmodel-ktx",
            group: "androidx.lifecycle",
            name: "lifecycle-viewmodel-ktx",
            version: "2.8.7",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-lifecycle-viewmodel-savedstate",
            group: "androidx.lifecycle",
            name: "lifecycle-viewmodel-savedstate-android",
            version: "2.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-savedstate",
            group: "androidx.savedstate",
            name: "savedstate-android",
            version: "1.3.3",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-savedstate-ktx",
            group: "androidx.savedstate",
            name: "savedstate-ktx",
            version: "1.3.3",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-savedstate-compose",
            group: "androidx.savedstate",
            name: "savedstate-compose-android",
            version: "1.3.3",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-tracing",
            group: "androidx.tracing",
            name: "tracing",
            version: "1.0.0",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-profileinstaller",
            group: "androidx.profileinstaller",
            name: "profileinstaller",
            version: "1.4.0",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-autofill",
            group: "androidx.autofill",
            name: "autofill",
            version: "1.0.0",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "androidx-emoji2",
            group: "androidx.emoji2",
            name: "emoji2",
            version: "1.2.0",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "kotlinx-coroutines-core",
            group: "org.jetbrains.kotlinx",
            name: "kotlinx-coroutines-core-jvm",
            version: "1.8.1",
            extension: ArtifactExtension::Jar,
        },
        ArtifactRequirement {
            id: "kotlinx-coroutines-android",
            group: "org.jetbrains.kotlinx",
            name: "kotlinx-coroutines-android",
            version: "1.8.1",
            extension: ArtifactExtension::Jar,
        },
        ArtifactRequirement {
            id: "compose-runtime",
            group: "androidx.compose.runtime",
            name: "runtime-android",
            version: "1.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "compose-runtime-annotation",
            group: "androidx.compose.runtime",
            name: "runtime-annotation-android",
            version: "1.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "compose-runtime-saveable",
            group: "androidx.compose.runtime",
            name: "runtime-saveable-android",
            version: "1.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "compose-foundation-layout",
            group: "androidx.compose.foundation",
            name: "foundation-layout-android",
            version: "1.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "compose-foundation",
            group: "androidx.compose.foundation",
            name: "foundation-android",
            version: "1.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "compose-animation-core",
            group: "androidx.compose.animation",
            name: "animation-core-android",
            version: "1.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "compose-material-ripple",
            group: "androidx.compose.material",
            name: "material-ripple-android",
            version: "1.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "compose-material3",
            group: "androidx.compose.material3",
            name: "material3-android",
            version: "1.4.0",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "compose-ui",
            group: "androidx.compose.ui",
            name: "ui-android",
            version: "1.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "compose-ui-util",
            group: "androidx.compose.ui",
            name: "ui-util-android",
            version: "1.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "compose-ui-graphics",
            group: "androidx.compose.ui",
            name: "ui-graphics-android",
            version: "1.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "compose-ui-text",
            group: "androidx.compose.ui",
            name: "ui-text-android",
            version: "1.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "compose-ui-unit",
            group: "androidx.compose.ui",
            name: "ui-unit-android",
            version: "1.9.4",
            extension: ArtifactExtension::Aar,
        },
        ArtifactRequirement {
            id: "compose-ui-geometry",
            group: "androidx.compose.ui",
            name: "ui-geometry-android",
            version: "1.9.4",
            extension: ArtifactExtension::Aar,
        },
    ]
}

fn resolve_artifact(
    requirement: &ArtifactRequirement,
    toolchain: &AndroidToolchain,
    cache_root: &Path,
) -> (Option<PathBuf>, ArtifactSource) {
    if let Some(kotlin_lib) = resolve_kotlin_lib(requirement, &toolchain.kotlin) {
        return (Some(kotlin_lib), ArtifactSource::KotlinInstall);
    }

    let cached = cache_root.join(requirement.relative_artifact());
    if cached.is_file() {
        return (Some(cached), ArtifactSource::KairoCache);
    }

    for cache in local_maven_caches() {
        if let Some(local) = find_in_local_cache(requirement, &cache) {
            if copy_if_missing(&local, &cached).is_ok() {
                return (Some(cached), ArtifactSource::LocalMavenCache);
            }
            return (Some(local), ArtifactSource::LocalMavenCache);
        }
    }

    match download_artifact(requirement, &cached) {
        Ok(()) => (Some(cached), ArtifactSource::Downloaded),
        Err(_) => (None, ArtifactSource::Missing),
    }
}

fn resolve_kotlin_lib(requirement: &ArtifactRequirement, kotlinc: &Path) -> Option<PathBuf> {
    let lib_dir = kotlin_lib_dir(kotlinc)?;
    let file_name = match requirement.id {
        "kotlin-stdlib" => "kotlin-stdlib.jar",
        "compose-compiler-plugin" => "compose-compiler-plugin.jar",
        _ => return None,
    };
    let jar = lib_dir.join(file_name);
    jar.is_file().then_some(jar)
}

fn kotlin_lib_dir(kotlinc: &Path) -> Option<PathBuf> {
    if kotlinc == Path::new("kotlinc") {
        #[cfg(windows)]
        {
            let lib = PathBuf::from(r"C:\kotlin\kotlinc\lib");
            return lib.is_dir().then_some(lib);
        }

        #[cfg(not(windows))]
        {
            return None;
        }
    }

    kotlinc.parent()?.parent().map(|path| path.join("lib"))
}

fn local_maven_caches() -> Vec<PathBuf> {
    let mut caches = Vec::new();

    if let Some(home) = home_dir() {
        caches.push(home.join(".m2").join("repository"));
        caches.push(
            home.join(".gradle")
                .join("caches")
                .join("modules-2")
                .join("files-2.1"),
        );
    }

    caches.into_iter().filter(|path| path.is_dir()).collect()
}

fn find_in_local_cache(requirement: &ArtifactRequirement, cache: &Path) -> Option<PathBuf> {
    let maven_path = cache.join(requirement.relative_artifact());
    if maven_path.is_file() {
        return Some(maven_path);
    }

    let gradle_dir = requirement
        .group
        .split('.')
        .fold(cache.to_path_buf(), |path, part| path.join(part))
        .join(requirement.name)
        .join(requirement.version);

    let file_name = format!(
        "{}-{}.{}",
        requirement.name,
        requirement.version,
        requirement.extension.as_str()
    );

    find_file_recursive(&gradle_dir, &file_name)
}

fn find_file_recursive(root: &Path, file_name: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(root).ok()?;

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() && path.file_name().is_some_and(|name| name == file_name) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_file_recursive(&path, file_name) {
                return Some(found);
            }
        }
    }

    None
}

fn download_artifact(requirement: &ArtifactRequirement, destination: &Path) -> Result<(), String> {
    if destination.is_file() {
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create `{}`: {err}", parent.display()))?;
    }

    let mut errors = Vec::new();
    for base in repository_bases(requirement) {
        let url = format!("{base}/{}", requirement.url_path());
        match download_url(&url, destination) {
            Ok(()) => return Ok(()),
            Err(err) => errors.push(format!("{url}: {err}")),
        }
    }

    let _ = fs::remove_file(destination);
    Err(errors.join("; "))
}

fn repository_bases(requirement: &ArtifactRequirement) -> Vec<&'static str> {
    if requirement.group.starts_with("androidx.") {
        vec!["https://dl.google.com/dl/android/maven2"]
    } else {
        vec![
            "https://repo1.maven.org/maven2",
            "https://dl.google.com/dl/android/maven2",
        ]
    }
}

fn download_url(url: &str, destination: &Path) -> Result<(), String> {
    let response = ureq::get(url)
        .call()
        .map_err(|err| format!("request failed: {err}"))?;

    let mut reader = response.into_reader();
    let mut file = fs::File::create(destination)
        .map_err(|err| format!("failed to create `{}`: {err}", destination.display()))?;

    io::copy(&mut reader, &mut file)
        .map_err(|err| format!("failed to write `{}`: {err}", destination.display()))?;

    Ok(())
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var_os("USERPROFILE").map(PathBuf::from)
    }

    #[cfg(not(windows))]
    {
        env::var_os("HOME").map(PathBuf::from)
    }
}

fn copy_if_missing(source: &Path, destination: &Path) -> Result<(), String> {
    if destination.is_file() {
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create `{}`: {err}", parent.display()))?;
    }

    fs::copy(source, destination).map_err(|err| {
        format!(
            "failed to copy `{}` to `{}`: {err}",
            source.display(),
            destination.display()
        )
    })?;

    Ok(())
}

fn write_artifact_manifest(
    path: &Path,
    artifacts: &[ArtifactResolution],
) -> Result<(), Vec<String>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            vec![format!(
                "failed to create artifact manifest directory `{}`: {err}",
                parent.display()
            )]
        })?;
    }

    let mut manifest = String::from("# Generated by Kairo\n\n");
    for artifact in artifacts {
        manifest.push_str("[[artifact]]\n");
        manifest.push_str(&format!("id = \"{}\"\n", artifact.requirement.id));
        manifest.push_str(&format!(
            "coordinate = \"{}\"\n",
            artifact.requirement.coordinate()
        ));
        manifest.push_str(&format!(
            "extension = \"{}\"\n",
            artifact.requirement.extension.as_str()
        ));
        manifest.push_str(&format!("source = \"{}\"\n", artifact.source.label()));
        match &artifact.path {
            Some(path) => {
                manifest.push_str("status = \"resolved\"\n");
                manifest.push_str(&format!("path = \"{}\"\n", escape_toml_path(path)));
            }
            None => {
                manifest.push_str("status = \"missing\"\n");
            }
        }
        manifest.push('\n');
    }

    fs::write(path, manifest)
        .map_err(|err| vec![format!("failed to write `{}`: {err}", path.display())])
}

fn escape_toml_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

fn required_artifact_path(
    resolution: &DependencyResolution,
    artifact_id: &str,
) -> Result<PathBuf, Vec<String>> {
    resolution
        .artifacts
        .iter()
        .find(|artifact| artifact.requirement.id == artifact_id)
        .and_then(|artifact| artifact.path.clone())
        .ok_or_else(|| {
            vec![format!(
                "required artifact `{artifact_id}` was not resolved"
            )]
        })
}

fn extract_aar_classes(
    request: &AndroidBuildRequest,
    artifact: &ArtifactResolution,
    aar_path: &Path,
) -> Result<Option<PathBuf>, String> {
    let destination = request
        .workspace_root
        .join("intermediates")
        .join("aar-classes")
        .join(artifact.requirement.id)
        .join("classes.jar");

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create `{}`: {err}", parent.display()))?;
    }

    let file = fs::File::open(aar_path)
        .map_err(|err| format!("failed to open AAR `{}`: {err}", aar_path.display()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|err| {
        format!(
            "failed to read AAR `{}` as a zip: {err}",
            aar_path.display()
        )
    })?;
    let mut classes = match archive.by_name("classes.jar") {
        Ok(classes) => classes,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(err) => {
            return Err(format!(
                "failed to read classes.jar from AAR `{}` for {}: {err}",
                aar_path.display(),
                artifact.requirement.coordinate()
            ));
        }
    };
    let mut output = fs::File::create(&destination)
        .map_err(|err| format!("failed to create `{}`: {err}", destination.display()))?;

    io::copy(&mut classes, &mut output).map_err(|err| {
        format!(
            "failed to extract classes.jar from `{}` to `{}`: {err}",
            aar_path.display(),
            destination.display()
        )
    })?;

    Ok(Some(destination))
}

fn collect_files_with_extension(
    root: &Path,
    extension: &str,
    files: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let mut entries = fs::read_dir(root)?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_extension(&path, extension, files)?;
        } else if path
            .extension()
            .and_then(|candidate| candidate.to_str())
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(extension))
        {
            files.push(path);
        }
    }

    Ok(())
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries = fs::read_dir(root)?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }

    Ok(())
}

fn zip_directory(source_dir: &Path, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create `{}`: {err}", parent.display()))?;
    }

    let mut files = Vec::new();
    collect_files(source_dir, &mut files).map_err(|err| {
        format!(
            "failed to collect files for jar from `{}`: {err}",
            source_dir.display()
        )
    })?;

    if files.is_empty() {
        return Err(format!(
            "cannot create jar because `{}` is empty",
            source_dir.display()
        ));
    }

    let file = fs::File::create(destination)
        .map_err(|err| format!("failed to create `{}`: {err}", destination.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for path in files {
        let entry_name = path
            .strip_prefix(source_dir)
            .map_err(|err| {
                format!(
                    "failed to make `{}` relative to `{}`: {err}",
                    path.display(),
                    source_dir.display()
                )
            })?
            .display()
            .to_string()
            .replace('\\', "/");
        zip.start_file(entry_name, options)
            .map_err(|err| format!("failed to add file to `{}`: {err}", destination.display()))?;

        let mut input = fs::File::open(&path)
            .map_err(|err| format!("failed to open `{}`: {err}", path.display()))?;
        io::copy(&mut input, &mut zip)
            .map_err(|err| format!("failed to write `{}`: {err}", destination.display()))?;
    }

    zip.finish()
        .map_err(|err| format!("failed to finish `{}`: {err}", destination.display()))?;

    Ok(())
}

fn create_unsigned_apk(
    resource_package: &Path,
    dex_files: &[PathBuf],
    destination: &Path,
) -> Result<(), String> {
    let input = fs::File::open(resource_package).map_err(|err| {
        format!(
            "failed to open resource package `{}`: {err}",
            resource_package.display()
        )
    })?;
    let mut resource_zip = zip::ZipArchive::new(input).map_err(|err| {
        format!(
            "failed to read resource package `{}` as a zip: {err}",
            resource_package.display()
        )
    })?;

    let output = fs::File::create(destination)
        .map_err(|err| format!("failed to create `{}`: {err}", destination.display()))?;
    let mut apk = zip::ZipWriter::new(output);
    let deflated = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let stored_aligned = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .with_alignment(4);

    for index in 0..resource_zip.len() {
        let mut entry = resource_zip.by_index(index).map_err(|err| {
            format!(
                "failed to read entry {index} from `{}`: {err}",
                resource_package.display()
            )
        })?;
        let name = entry.name().replace('\\', "/");

        if name == "classes.dex" || name.starts_with("classes") && name.ends_with(".dex") {
            continue;
        }

        if entry.is_dir() {
            apk.add_directory(name, stored).map_err(|err| {
                format!(
                    "failed to add directory to `{}`: {err}",
                    destination.display()
                )
            })?;
        } else {
            let options = if name == "resources.arsc" {
                stored_aligned
            } else {
                deflated
            };
            apk.start_file(name, options).map_err(|err| {
                format!(
                    "failed to add resource to `{}`: {err}",
                    destination.display()
                )
            })?;
            io::copy(&mut entry, &mut apk)
                .map_err(|err| format!("failed to write `{}`: {err}", destination.display()))?;
        }
    }

    for (index, dex) in dex_files.iter().enumerate() {
        let entry_name = if index == 0 {
            "classes.dex".to_string()
        } else {
            format!("classes{}.dex", index + 1)
        };
        apk.start_file(entry_name, deflated)
            .map_err(|err| format!("failed to add dex to `{}`: {err}", destination.display()))?;
        let mut input = fs::File::open(dex)
            .map_err(|err| format!("failed to open dex file `{}`: {err}", dex.display()))?;
        io::copy(&mut input, &mut apk).map_err(|err| {
            format!(
                "failed to write dex into `{}`: {err}",
                destination.display()
            )
        })?;
    }

    apk.finish()
        .map_err(|err| format!("failed to finish `{}`: {err}", destination.display()))?;

    Ok(())
}

fn recreate_dir(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|err| format!("failed to remove `{}`: {err}", path.display()))?;
    } else if path.is_file() {
        fs::remove_file(path)
            .map_err(|err| format!("failed to remove `{}`: {err}", path.display()))?;
    }

    fs::create_dir_all(path).map_err(|err| format!("failed to create `{}`: {err}", path.display()))
}

fn write_kotlinc_argfile(
    path: &Path,
    classpath: &OsString,
    classes_dir: &Path,
    compose_plugin: &Path,
    sources: &[PathBuf],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create `{}`: {err}", parent.display()))?;
    }

    let classpath = classpath.to_string_lossy().replace('\\', "/");
    let plugin = format!("-Xplugin={}", path_for_kotlinc(compose_plugin));

    let mut args = String::new();
    args.push_str("-classpath\n");
    args.push_str(&quote_kotlinc_arg(&classpath));
    args.push('\n');
    args.push_str("-d\n");
    args.push_str(&quote_kotlinc_arg(&path_for_kotlinc(classes_dir)));
    args.push('\n');
    args.push_str("-jvm-target\n17\n");
    args.push_str(&quote_kotlinc_arg(&plugin));
    args.push('\n');

    for source in sources {
        args.push_str(&quote_kotlinc_arg(&path_for_kotlinc(source)));
        args.push('\n');
    }

    fs::write(path, args).map_err(|err| format!("failed to write `{}`: {err}", path.display()))
}

fn path_for_kotlinc(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn quote_kotlinc_arg(value: &str) -> String {
    if value
        .chars()
        .any(|character| character.is_whitespace() || matches!(character, '=' | ';' | ','))
    {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

fn write_process_report(
    path: &Path,
    title: &str,
    program: &Path,
    args: &[OsString],
    output: Option<&Output>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create `{}`: {err}", parent.display()))?;
    }

    let mut report = format!("# {title}\n\n");
    report.push_str("## Command\n\n");
    report.push_str("```text\n");
    report.push_str(&format_process_command(program, args));
    report.push_str("\n```\n\n");

    match output {
        Some(output) => {
            report.push_str("## Exit\n\n");
            report.push_str(&format!("{}\n\n", process_status_label(output)));
            report.push_str("## Stdout\n\n");
            report.push_str("```text\n");
            report.push_str(&String::from_utf8_lossy(&output.stdout));
            report.push_str("\n```\n\n");
            report.push_str("## Stderr\n\n");
            report.push_str("```text\n");
            report.push_str(&String::from_utf8_lossy(&output.stderr));
            report.push_str("\n```\n");
        }
        None => {
            report.push_str("## Exit\n\nkotlinc was not launched.\n");
        }
    }

    fs::write(path, report).map_err(|err| format!("failed to write `{}`: {err}", path.display()))
}

fn write_text_report(path: &Path, title: &str, body: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create `{}`: {err}", parent.display()))?;
    }

    fs::write(path, format!("# {title}\n\n{body}\n"))
        .map_err(|err| format!("failed to write `{}`: {err}", path.display()))
}

fn format_process_command(program: &Path, args: &[OsString]) -> String {
    let mut parts = vec![program.display().to_string()];
    parts.extend(args.iter().map(|arg| arg.to_string_lossy().into_owned()));
    parts.join(" ")
}

fn process_status_label(output: &Output) -> String {
    output
        .status
        .code()
        .map(|code| format!("exit code {code}"))
        .unwrap_or_else(|| "terminated by signal".to_string())
}

fn process_excerpt(output: &Output, limit: usize) -> Vec<String> {
    let combined = process_output_text(output);
    let excerpt = combined
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(limit)
        .map(|line| format!("compiler: {line}"))
        .collect::<Vec<_>>();

    if excerpt.is_empty() {
        vec!["compiler produced no diagnostic output".to_string()]
    } else {
        excerpt
    }
}

fn process_output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn push_planned_steps(steps: &mut Vec<BuildStepResult>, start_index: usize) {
    for name in ANDROID_STEPS.iter().skip(start_index) {
        steps.push(BuildStepResult {
            name,
            status: BuildStepStatus::Planned,
            details: Vec::new(),
        });
    }
}

fn finish_report(
    request: &AndroidBuildRequest,
    steps: Vec<BuildStepResult>,
) -> Result<BuildReport, String> {
    let report_path = request
        .workspace_root
        .join("outputs")
        .join("reports")
        .join("android-build-report.txt");

    write_report(&report_path, &steps)?;

    Ok(BuildReport { steps, report_path })
}

fn write_report(path: &Path, steps: &[BuildStepResult]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create `{}`: {err}", parent.display()))?;
    }

    let mut output = String::from("# Kairo Android Build Report\n\n");
    for (index, step) in steps.iter().enumerate() {
        output.push_str(&format!(
            "{}. {} - {}\n",
            index + 1,
            step.name,
            step.status.label()
        ));

        for detail in &step.details {
            output.push_str(&format!("   - {detail}\n"));
        }
    }

    fs::write(path, output).map_err(|err| format!("failed to write `{}`: {err}", path.display()))
}

fn check_project(request: &AndroidBuildRequest) -> Result<AndroidToolchain, Vec<String>> {
    let mut failures = Vec::new();

    require_file(&request.project_dir.join("kairo.toml"), &mut failures);
    require_file(
        &request.project_dir.join("src").join("main.kt"),
        &mut failures,
    );
    require_file(&request.build_plan, &mut failures);
    require_file(
        &request
            .workspace_root
            .join("src")
            .join("main")
            .join("AndroidManifest.xml"),
        &mut failures,
    );
    require_file(
        &package_dir(
            &request
                .workspace_root
                .join("src")
                .join("main")
                .join("kotlin"),
            &request.app_id,
        )
        .join("MainActivity.kt"),
        &mut failures,
    );

    let sdk = find_android_sdk().ok_or_else(|| {
        vec![
            "Android SDK not found.".to_string(),
            "Set ANDROID_HOME or ANDROID_SDK_ROOT, or install Android Studio SDK.".to_string(),
        ]
    })?;

    let java = find_java().ok_or_else(|| {
        vec![
            "Java executable not found.".to_string(),
            "Install JDK 21 or set JAVA_HOME.".to_string(),
        ]
    })?;

    let javac = find_javac(&java).ok_or_else(|| {
        vec![
            "javac executable not found.".to_string(),
            "Install a full JDK, not only a JRE, or set JAVA_HOME to a JDK.".to_string(),
        ]
    })?;

    let kotlin = find_kotlinc().ok_or_else(|| {
        vec![
            "Kotlin compiler not found.".to_string(),
            "Install Kotlin compiler or set KOTLIN_HOME.".to_string(),
        ]
    })?;

    let android_jar = find_android_jar(&sdk, request.target_sdk).ok_or_else(|| {
        vec![format!(
            "android.jar not found for target SDK {} under `{}`.",
            request.target_sdk,
            sdk.display()
        )]
    })?;

    let build_tools = latest_build_tools_dir(&sdk).ok_or_else(|| {
        vec![format!(
            "Android Build Tools not found under `{}`.",
            sdk.join("build-tools").display()
        )]
    })?;

    let aapt2 = require_tool(
        &build_tools,
        executable_name("aapt2"),
        "AAPT2",
        &mut failures,
    );
    let d8 = require_tool(&build_tools, executable_name("d8"), "D8", &mut failures);
    let apksigner = require_tool(
        &build_tools,
        executable_name("apksigner"),
        "APK signer",
        &mut failures,
    );
    let adb = require_tool(
        &sdk.join("platform-tools"),
        executable_name("adb"),
        "ADB",
        &mut failures,
    );

    if !failures.is_empty() {
        return Err(failures);
    }

    let java_version = version_line(&java, &["-version"]);
    let javac_version = version_line(&javac, &["-version"]);
    let kotlin_version = version_line(&kotlin, &["-version"]);
    let aapt2_version = version_line(&aapt2, &["version"]);
    let d8_version = version_line(&d8, &["--version"]);
    let adb_version = version_line(&adb, &["version"]);

    Ok(AndroidToolchain {
        sdk,
        java,
        javac,
        kotlin,
        android_jar,
        build_tools,
        aapt2,
        d8,
        apksigner,
        adb,
        java_version,
        javac_version,
        kotlin_version,
        aapt2_version,
        d8_version,
        adb_version,
    })
}

fn require_file(path: &Path, failures: &mut Vec<String>) {
    if !path.is_file() {
        failures.push(format!("missing required file `{}`", path.display()));
    }
}

fn require_tool(
    dir: &Path,
    executable: &'static str,
    label: &str,
    failures: &mut Vec<String>,
) -> PathBuf {
    let path = dir.join(executable);
    if !path.is_file() {
        failures.push(format!("{label} not found at `{}`", path.display()));
    }
    path
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AndroidToolchain {
    sdk: PathBuf,
    java: PathBuf,
    javac: PathBuf,
    kotlin: PathBuf,
    android_jar: PathBuf,
    build_tools: PathBuf,
    aapt2: PathBuf,
    d8: PathBuf,
    apksigner: PathBuf,
    adb: PathBuf,
    java_version: String,
    javac_version: String,
    kotlin_version: String,
    aapt2_version: String,
    d8_version: String,
    adb_version: String,
}

impl AndroidToolchain {
    fn details(&self) -> Vec<String> {
        vec![
            format!("Android SDK: {}", self.sdk.display()),
            format!("Build Tools: {}", self.build_tools.display()),
            format!("android.jar: {}", self.android_jar.display()),
            format!("Java: {} ({})", self.java_version, self.java.display()),
            format!("Javac: {} ({})", self.javac_version, self.javac.display()),
            format!(
                "Kotlin: {} ({})",
                self.kotlin_version,
                self.kotlin.display()
            ),
            format!("AAPT2: {} ({})", self.aapt2_version, self.aapt2.display()),
            format!("D8: {} ({})", self.d8_version, self.d8.display()),
            format!("APK signer: {}", self.apksigner.display()),
            format!("ADB: {} ({})", self.adb_version, self.adb.display()),
        ]
    }
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

fn find_java() -> Option<PathBuf> {
    if let Some(java_home) = env::var_os("JAVA_HOME") {
        let java = PathBuf::from(java_home)
            .join("bin")
            .join(executable_name("java"));
        if java.is_file() {
            return Some(java);
        }
    }

    if command_exists("java", &["-version"]) {
        return Some(PathBuf::from("java"));
    }

    #[cfg(windows)]
    {
        for base in [
            Path::new(r"C:\Program Files\Eclipse Adoptium"),
            Path::new(r"C:\Program Files\Java"),
        ] {
            for candidate in versioned_tool_candidates(base, &["bin", executable_name("java")]) {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

fn find_javac(java: &Path) -> Option<PathBuf> {
    if let Some(bin) = java.parent() {
        let javac = bin.join(executable_name("javac"));
        if javac.is_file() {
            return Some(javac);
        }
    }

    if let Some(java_home) = discover_java_home() {
        let javac = java_home.join("bin").join(executable_name("javac"));
        if javac.is_file() {
            return Some(javac);
        }
    }

    if command_exists("javac", &["-version"]) {
        return Some(PathBuf::from("javac"));
    }

    None
}

fn find_kotlinc() -> Option<PathBuf> {
    if let Some(kotlin_home) = env::var_os("KOTLIN_HOME") {
        let kotlin = PathBuf::from(kotlin_home)
            .join("bin")
            .join(executable_name("kotlinc"));
        if kotlin.is_file() {
            return Some(kotlin);
        }
    }

    if command_exists("kotlinc", &["-version"]) {
        return Some(PathBuf::from("kotlinc"));
    }

    #[cfg(windows)]
    {
        let kotlin = PathBuf::from(r"C:\kotlin\kotlinc\bin").join(executable_name("kotlinc"));
        if kotlin.is_file() {
            return Some(kotlin);
        }
    }

    None
}

fn find_android_jar(sdk: &Path, target_sdk: u32) -> Option<PathBuf> {
    let platforms = sdk.join("platforms");
    let exact = platforms
        .join(format!("android-{target_sdk}"))
        .join("android.jar");
    if exact.is_file() {
        return Some(exact);
    }

    let prefix = format!("android-{target_sdk}");
    let matching = fs::read_dir(&platforms)
        .ok()?
        .filter_map(Result::ok)
        .find_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(&prefix) {
                let jar = entry.path().join("android.jar");
                jar.is_file().then_some(jar)
            } else {
                None
            }
        });

    matching.or_else(|| latest_child_file(&platforms, "android.jar"))
}

fn latest_build_tools_dir(sdk: &Path) -> Option<PathBuf> {
    latest_child_dir(&sdk.join("build-tools"))
}

fn latest_child_dir(path: &Path) -> Option<PathBuf> {
    let mut entries = fs::read_dir(path)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();
    entries.pop()
}

fn latest_child_file(path: &Path, file_name: &str) -> Option<PathBuf> {
    let mut entries = fs::read_dir(path)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path().join(file_name))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    entries.sort();
    entries.pop()
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

fn package_dir(root: &Path, package_name: &str) -> PathBuf {
    package_name
        .split('.')
        .filter(|part| !part.is_empty())
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

fn command_exists(program: &str, args: &[&str]) -> bool {
    run_program(Path::new(program), args)
        .ok()
        .is_some_and(|output| output.status.success())
}

fn version_line(program: &Path, args: &[&str]) -> String {
    match run_program(program, args) {
        Ok(output) => first_output_line(&output.stdout, &output.stderr),
        Err(err) => format!("version unavailable: {err}"),
    }
}

fn keytool_for_java(java: &Path) -> PathBuf {
    if java == Path::new("java") {
        return PathBuf::from("keytool");
    }

    java.parent()
        .map(|bin| bin.join(executable_name("keytool")))
        .unwrap_or_else(|| PathBuf::from(executable_name("keytool")))
}

fn run_program(program: &Path, args: &[&str]) -> io::Result<Output> {
    if cfg!(windows) {
        let extension = program
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();

        if extension.eq_ignore_ascii_case("bat") || extension.eq_ignore_ascii_case("cmd") {
            let mut shell_args = vec!["/C".to_string(), program.to_string_lossy().into_owned()];
            shell_args.extend(args.iter().map(|arg| arg.to_string()));

            let mut command = Command::new("cmd");
            command.args(shell_args);
            apply_java_env(&mut command);
            return command.output();
        }
    }

    let mut command = Command::new(program);
    command.args(args);
    apply_java_env(&mut command);
    command.output()
}

fn run_program_os(program: &Path, args: &[OsString]) -> io::Result<Output> {
    if cfg!(windows) {
        let extension = program
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();

        if extension.eq_ignore_ascii_case("bat") || extension.eq_ignore_ascii_case("cmd") {
            let mut shell_args = vec![OsString::from("/C"), program.as_os_str().to_os_string()];
            shell_args.extend(args.iter().cloned());

            let mut command = Command::new("cmd");
            command.args(shell_args);
            apply_java_env(&mut command);
            return command.output();
        }
    }

    let mut command = Command::new(program);
    command.args(args);
    apply_java_env(&mut command);
    command.output()
}

fn apply_java_env(command: &mut Command) {
    let Some(java_home) = discover_java_home() else {
        return;
    };

    command.env("JAVA_HOME", &java_home);

    let current_path = env::var_os("PATH").unwrap_or_default();
    let paths = std::iter::once(java_home.join("bin")).chain(env::split_paths(&current_path));

    if let Ok(path) = env::join_paths(paths) {
        command.env("PATH", path);
    }
}

fn discover_java_home() -> Option<PathBuf> {
    if let Some(java_home) = env::var_os("JAVA_HOME").map(PathBuf::from) {
        if java_home.exists() {
            return Some(java_home);
        }
    }

    #[cfg(windows)]
    {
        for base in [
            Path::new(r"C:\Program Files\Eclipse Adoptium"),
            Path::new(r"C:\Program Files\Java"),
        ] {
            for java in versioned_tool_candidates(base, &["bin", executable_name("java")]) {
                if let Some(home) = java.parent().and_then(Path::parent) {
                    if home.exists() {
                        return Some(home.to_path_buf());
                    }
                }
            }
        }
    }

    None
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

fn executable_name(name: &'static str) -> &'static str {
    match (cfg!(windows), name) {
        (true, "aapt2") => "aapt2.exe",
        (true, "adb") => "adb.exe",
        (true, "apksigner") => "apksigner.bat",
        (true, "d8") => "d8.bat",
        (true, "java") => "java.exe",
        (true, "javac") => "javac.exe",
        (true, "keytool") => "keytool.exe",
        (true, "kotlinc") => "kotlinc.bat",
        (_, "aapt2") => "aapt2",
        (_, "adb") => "adb",
        (_, "apksigner") => "apksigner",
        (_, "d8") => "d8",
        (_, "java") => "java",
        (_, "javac") => "javac",
        (_, "keytool") => "keytool",
        (_, "kotlinc") => "kotlinc",
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_labels_are_stable() {
        assert_eq!(BuildStepStatus::Done.label(), "done");
        assert_eq!(BuildStepStatus::Blocked.label(), "blocked");
        assert_eq!(BuildStepStatus::Planned.label(), "planned");
    }

    #[test]
    fn package_dir_uses_package_segments() {
        let path = package_dir(Path::new("root"), "com.example.app");

        assert_eq!(
            path,
            PathBuf::from("root")
                .join("com")
                .join("example")
                .join("app")
        );
    }

    #[test]
    fn report_blocking_message_uses_first_blocked_step() {
        let report = BuildReport {
            report_path: PathBuf::from("report.txt"),
            steps: vec![BuildStepResult {
                name: "resolve-compose-dependencies",
                status: BuildStepStatus::Blocked,
                details: vec!["resolver missing".to_string()],
            }],
        };

        assert_eq!(
            report.blocking_message(),
            Some("resolve-compose-dependencies is blocked: resolver missing".to_string())
        );
    }
}
