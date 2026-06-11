use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KairoConfig {
    pub app_name: String,
    pub app_id: String,
    pub version: String,
    pub android_enabled: bool,
    pub desktop_enabled: bool,
    pub min_sdk: u32,
    pub target_sdk: u32,
}

impl KairoConfig {
    pub fn new(project_name: &str) -> Self {
        Self {
            app_name: display_name_from_project_name(project_name),
            app_id: app_id_from_project_name(project_name),
            version: "0.1.0".to_string(),
            android_enabled: true,
            desktop_enabled: false,
            min_sdk: 26,
            target_sdk: 36,
        }
    }

    pub fn to_toml(&self) -> String {
        format!(
            "\
[app]
name = \"{}\"
id = \"{}\"
version = \"{}\"

[targets]
android = {}
desktop = {}

[android]
min_sdk = {}
target_sdk = {}

[build]
engine = \"kairo\"
cache = true
parallel = true

[ui]
toolkit = \"compose\"
import_facade = \"kairo.compose\"
",
            self.app_name,
            self.app_id,
            self.version,
            self.android_enabled,
            self.desktop_enabled,
            self.min_sdk,
            self.target_sdk
        )
    }

    pub fn from_file(path: &Path) -> Result<Self, String> {
        let contents = fs::read_to_string(path)
            .map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
        Self::from_toml_str(&contents)
    }

    pub fn from_toml_str(contents: &str) -> Result<Self, String> {
        let mut config = KairoConfig::new("app");
        let mut section = String::new();

        for raw_line in contents.lines() {
            let line = raw_line.split('#').next().unwrap_or_default().trim();

            if line.is_empty() {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                section = line
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .trim()
                    .to_string();
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            let key = key.trim();
            let value = value.trim();

            match (section.as_str(), key) {
                ("app", "name") => config.app_name = parse_string(value),
                ("app", "id") => config.app_id = parse_string(value),
                ("app", "version") => config.version = parse_string(value),
                ("targets", "android") => config.android_enabled = parse_bool(value)?,
                ("targets", "desktop") => config.desktop_enabled = parse_bool(value)?,
                ("android", "min_sdk") => config.min_sdk = parse_u32(value, "android.min_sdk")?,
                ("android", "target_sdk") => {
                    config.target_sdk = parse_u32(value, "android.target_sdk")?
                }
                _ => {}
            }
        }

        if config.app_name.trim().is_empty() {
            return Err("kairo.toml [app].name cannot be empty".to_string());
        }

        if config.app_id.trim().is_empty() {
            return Err("kairo.toml [app].id cannot be empty".to_string());
        }

        Ok(config)
    }
}

fn parse_string(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("expected boolean value, found `{other}`")),
    }
}

fn parse_u32(value: &str, key: &str) -> Result<u32, String> {
    value
        .trim()
        .parse()
        .map_err(|err| format!("invalid `{key}` value `{value}`: {err}"))
}

pub fn validate_project_name(input: &str) -> Result<(), String> {
    let name = input.trim();

    if name.is_empty() {
        return Err("project name cannot be empty".to_string());
    }

    if name == "." || name == ".." {
        return Err("project name cannot be `.` or `..`".to_string());
    }

    if name.starts_with('-') {
        return Err("project name cannot start with `-`".to_string());
    }

    if name.contains('/') || name.contains('\\') {
        return Err("project name must not contain path separators".to_string());
    }

    let valid = name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');

    if !valid {
        return Err(
            "project name may only contain ASCII letters, numbers, hyphens, and underscores"
                .to_string(),
        );
    }

    Ok(())
}

pub fn display_name_from_project_name(project_name: &str) -> String {
    project_name
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(capitalize_ascii)
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn app_id_from_project_name(project_name: &str) -> String {
    let suffix = project_name
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(".");

    format!("com.example.{suffix}")
}

fn capitalize_ascii(input: &str) -> String {
    let mut chars = input.chars();
    match chars.next() {
        Some(first) => {
            let mut output = first.to_ascii_uppercase().to_string();
            output.push_str(chars.as_str());
            output
        }
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_default_config() {
        let config = KairoConfig::new("hello-world");

        assert_eq!(config.app_name, "Hello World");
        assert_eq!(config.app_id, "com.example.hello.world");
        assert!(config.android_enabled);
        assert!(!config.desktop_enabled);
    }

    #[test]
    fn rejects_unsafe_project_names() {
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("../hello").is_err());
        assert!(validate_project_name("-hello").is_err());
        assert!(validate_project_name("hello world").is_err());
        assert!(validate_project_name("hello").is_ok());
        assert!(validate_project_name("hello-world").is_ok());
    }

    #[test]
    fn reads_config_from_toml() {
        let config = KairoConfig::from_toml_str(
            r#"
[app]
name = "Counter"
id = "com.example.counter"
version = "0.2.0"

[targets]
android = true
desktop = true

[android]
min_sdk = 28
target_sdk = 36
"#,
        )
        .unwrap();

        assert_eq!(config.app_name, "Counter");
        assert_eq!(config.app_id, "com.example.counter");
        assert_eq!(config.version, "0.2.0");
        assert!(config.android_enabled);
        assert!(config.desktop_enabled);
        assert_eq!(config.min_sdk, 28);
        assert_eq!(config.target_sdk, 36);
    }
}
