//! Centralized runtime-shape and path routing for all filesystem locations.
//!
//! ALL app-owned paths in the codebase MUST go through this module.
//! Debug builds default to the development sandbox. Installed release builds
//! resolve to the build-time runtime shape and ignore local runtime overrides.

use serde::Serialize;
use std::path::PathBuf;
use std::sync::OnceLock;

const RUNTIME_ENV_VAR: &str = "MODUS_RUNTIME";
const LEGACY_ENV_VAR: &str = "MODUS_ENV";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeShape {
    DevelopmentSandbox,
    PreRelease,
    Release,
}

impl RuntimeShape {
    pub fn id(self) -> &'static str {
        match self {
            Self::DevelopmentSandbox => "development-sandbox",
            Self::PreRelease => "pre-release",
            Self::Release => "release",
        }
    }

    pub fn internal_identifier(self) -> &'static str {
        match self {
            Self::DevelopmentSandbox => "com.leon4z.modus.dev",
            Self::PreRelease => "com.leon4z.modus.pre-release",
            Self::Release => "com.leon4z.modus",
        }
    }

    pub fn update_channel(self) -> RuntimeUpdateChannel {
        match self {
            Self::DevelopmentSandbox => RuntimeUpdateChannel::Disabled,
            Self::PreRelease => RuntimeUpdateChannel::Test,
            Self::Release => RuntimeUpdateChannel::Stable,
        }
    }

    pub fn uses_sandbox_tools(self) -> bool {
        self == Self::DevelopmentSandbox
    }

    pub fn uses_real_tools(self) -> bool {
        !self.uses_sandbox_tools()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeUpdateChannel {
    Disabled,
    Test,
    Stable,
}

impl RuntimeUpdateChannel {
    pub fn id(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Test => "test",
            Self::Stable => "stable",
        }
    }

    pub fn can_check_for_updates(self) -> bool {
        self != Self::Disabled
    }
}

/// All resolved filesystem paths for the current environment.
pub struct EnvPaths {
    /// Active runtime shape.
    pub runtime_shape: RuntimeShape,
    /// App data root selected by the active runtime shape.
    pub data_dir: PathBuf,
    /// Generic skills directory selected by the active runtime shape.
    pub generic_skills_dir: PathBuf,
    /// Home directory.
    pub home: PathBuf,
    /// Whether this binary was compiled as a release build.
    pub release_build: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub shape: String,
    pub internal_identifier: String,
    pub update_channel: String,
    pub can_check_for_updates: bool,
    pub uses_sandbox_tools: bool,
    pub uses_real_tools: bool,
    pub app_data_dir: String,
    pub generic_skills_dir: String,
    pub release_build: bool,
}

static ENV_PATHS: OnceLock<EnvPaths> = OnceLock::new();

fn runtime_override(value: Option<&str>) -> Option<RuntimeShape> {
    let value = value?.trim().to_ascii_lowercase();
    match value.as_str() {
        "pre-release" | "pre_release" | "prerelease" | "preview" => Some(RuntimeShape::PreRelease),
        "development-sandbox" | "development_sandbox" | "dev" | "development" | "sandbox" => {
            Some(RuntimeShape::DevelopmentSandbox)
        }
        _ => None,
    }
}

fn legacy_override(value: Option<&str>) -> Option<RuntimeShape> {
    let value = value?.trim().to_ascii_lowercase();
    match value.as_str() {
        "dev" => Some(RuntimeShape::DevelopmentSandbox),
        _ => None,
    }
}

fn resolve_runtime_shape_for(
    debug_assertions: bool,
    runtime_value: Option<&str>,
    legacy_value: Option<&str>,
    build_runtime_value: Option<&str>,
) -> RuntimeShape {
    if !debug_assertions {
        return runtime_override(build_runtime_value).unwrap_or(RuntimeShape::Release);
    }

    runtime_override(runtime_value)
        .or_else(|| legacy_override(legacy_value))
        .unwrap_or(RuntimeShape::DevelopmentSandbox)
}

fn resolve_paths(home: PathBuf, runtime_shape: RuntimeShape, release_build: bool) -> EnvPaths {
    let (data_dir, generic_skills_dir) = match runtime_shape {
        RuntimeShape::DevelopmentSandbox => (
            home.join(".modus-dev"),
            home.join(".agents-dev").join("skills"),
        ),
        RuntimeShape::PreRelease => (
            home.join(".modus-pre-release"),
            home.join(".agents").join("skills"),
        ),
        RuntimeShape::Release => (home.join(".modus"), home.join(".agents").join("skills")),
    };
    EnvPaths {
        runtime_shape,
        data_dir,
        generic_skills_dir,
        home,
        release_build,
    }
}

/// Get the resolved paths for the current environment. Initialized once on first call.
pub fn paths() -> &'static EnvPaths {
    ENV_PATHS.get_or_init(|| {
        let home = dirs::home_dir().expect(
            "[env_config] FATAL: Cannot determine home directory. \
             This is required for all file operations. \
             Ensure $HOME is set in the environment.",
        );
        let debug_assertions = cfg!(debug_assertions);
        let runtime_shape = resolve_runtime_shape_for(
            debug_assertions,
            std::env::var(RUNTIME_ENV_VAR).ok().as_deref(),
            std::env::var(LEGACY_ENV_VAR).ok().as_deref(),
            option_env!("MODUS_RUNTIME"),
        );

        resolve_paths(home, runtime_shape, !debug_assertions)
    })
}

// ── Convenience accessors ──────────────────────────────────────────

/// App data root directory.
pub fn data_dir() -> PathBuf {
    paths().data_dir.clone()
}

/// `config.json` path.
pub fn config_path() -> PathBuf {
    paths().data_dir.join("config.json")
}

/// Default rules storage directory.
pub fn default_rules_dir() -> PathBuf {
    paths().data_dir.join("default-rules")
}

/// Migration backups directory.
pub fn backups_dir() -> PathBuf {
    paths().data_dir.join("backups")
}

/// Application logs directory.
pub fn logs_dir() -> PathBuf {
    paths().data_dir.join("logs")
}

/// Generic skills directory (`~/.agents/skills/` or dev equivalent).
pub fn generic_skills_dir() -> PathBuf {
    paths().generic_skills_dir.clone()
}

/// Active runtime shape.
pub fn runtime_shape() -> RuntimeShape {
    paths().runtime_shape
}

/// Runtime metadata for Settings and future startup update checks.
pub fn runtime_info() -> RuntimeInfo {
    let p = paths();
    let channel = p.runtime_shape.update_channel();
    RuntimeInfo {
        shape: p.runtime_shape.id().to_string(),
        internal_identifier: p.runtime_shape.internal_identifier().to_string(),
        update_channel: channel.id().to_string(),
        can_check_for_updates: channel.can_check_for_updates(),
        uses_sandbox_tools: p.runtime_shape.uses_sandbox_tools(),
        uses_real_tools: p.runtime_shape.uses_real_tools(),
        app_data_dir: p.data_dir.to_string_lossy().to_string(),
        generic_skills_dir: p.generic_skills_dir.to_string_lossy().to_string(),
        release_build: p.release_build,
    }
}

/// Home directory.
pub fn home_dir() -> PathBuf {
    paths().home.clone()
}

/// Whether running in the development sandbox.
pub fn is_development_sandbox() -> bool {
    runtime_shape() == RuntimeShape::DevelopmentSandbox
}

/// Whether running in the pre-release version.
pub fn is_pre_release() -> bool {
    runtime_shape() == RuntimeShape::PreRelease
}

/// Legacy scenario-runner guard alias for the development sandbox.
pub fn is_dev() -> bool {
    is_development_sandbox()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_consistent() {
        let p = paths();
        assert!(p.data_dir.to_string_lossy().contains("modus"));
        assert!(p.generic_skills_dir.to_string_lossy().contains("skills"));
        assert_eq!(config_path(), p.data_dir.join("config.json"));
        assert_eq!(default_rules_dir(), p.data_dir.join("default-rules"));
        assert_eq!(backups_dir(), p.data_dir.join("backups"));
        assert_eq!(logs_dir(), p.data_dir.join("logs"));
    }

    #[test]
    fn log_paths_are_separated_between_dev_and_prod() {
        let home = PathBuf::from("/tmp/modus-home");
        let prod = resolve_paths(home.clone(), RuntimeShape::Release, true);
        let dev = resolve_paths(home, RuntimeShape::DevelopmentSandbox, false);

        assert_eq!(
            prod.data_dir.join("logs"),
            PathBuf::from("/tmp/modus-home/.modus/logs")
        );
        assert_eq!(
            dev.data_dir.join("logs"),
            PathBuf::from("/tmp/modus-home/.modus-dev/logs")
        );
        assert_ne!(prod.data_dir.join("logs"), dev.data_dir.join("logs"));
    }

    #[test]
    fn debug_build_defaults_to_development_sandbox() {
        assert_eq!(
            resolve_runtime_shape_for(true, None, None, None),
            RuntimeShape::DevelopmentSandbox
        );
    }

    #[test]
    fn debug_build_can_request_pre_release() {
        assert_eq!(
            resolve_runtime_shape_for(true, Some("pre-release"), None, None),
            RuntimeShape::PreRelease
        );
        assert_eq!(
            resolve_runtime_shape_for(true, Some("pre_release"), Some("dev"), None),
            RuntimeShape::PreRelease
        );
    }

    #[test]
    fn release_build_ignores_runtime_overrides_without_build_runtime() {
        assert_eq!(
            resolve_runtime_shape_for(false, Some("pre-release"), Some("dev"), None),
            RuntimeShape::Release
        );
        assert_eq!(
            resolve_runtime_shape_for(false, Some("development-sandbox"), None, None),
            RuntimeShape::Release
        );
    }

    #[test]
    fn release_build_can_pin_pre_release_at_build_time() {
        assert_eq!(
            resolve_runtime_shape_for(false, None, None, Some("pre-release")),
            RuntimeShape::PreRelease
        );
        assert_eq!(
            resolve_runtime_shape_for(false, Some("development-sandbox"), None, Some("pre-release")),
            RuntimeShape::PreRelease
        );
    }

    #[test]
    fn invalid_override_falls_back_to_development_sandbox_in_debug() {
        assert_eq!(
            resolve_runtime_shape_for(true, Some("release"), Some("prod"), None),
            RuntimeShape::DevelopmentSandbox
        );
    }

    #[test]
    fn runtime_paths_are_distinct_across_all_shapes() {
        let home = PathBuf::from("/tmp/modus-home");
        let dev = resolve_paths(home.clone(), RuntimeShape::DevelopmentSandbox, false);
        let pre_release = resolve_paths(home.clone(), RuntimeShape::PreRelease, false);
        let release = resolve_paths(home, RuntimeShape::Release, true);

        assert_eq!(dev.data_dir, PathBuf::from("/tmp/modus-home/.modus-dev"));
        assert_eq!(
            pre_release.data_dir,
            PathBuf::from("/tmp/modus-home/.modus-pre-release")
        );
        assert_eq!(release.data_dir, PathBuf::from("/tmp/modus-home/.modus"));
        assert_ne!(dev.data_dir, pre_release.data_dir);
        assert_ne!(pre_release.data_dir, release.data_dir);
        assert_eq!(
            dev.generic_skills_dir,
            PathBuf::from("/tmp/modus-home/.agents-dev/skills")
        );
        assert_eq!(
            pre_release.generic_skills_dir,
            PathBuf::from("/tmp/modus-home/.agents/skills")
        );
        assert_eq!(
            release.generic_skills_dir,
            PathBuf::from("/tmp/modus-home/.agents/skills")
        );
    }

    #[test]
    fn update_channels_follow_runtime_shape() {
        assert_eq!(
            RuntimeShape::DevelopmentSandbox.update_channel(),
            RuntimeUpdateChannel::Disabled
        );
        assert_eq!(
            RuntimeShape::PreRelease.update_channel(),
            RuntimeUpdateChannel::Test
        );
        assert_eq!(
            RuntimeShape::Release.update_channel(),
            RuntimeUpdateChannel::Stable
        );
        assert!(!RuntimeShape::DevelopmentSandbox
            .update_channel()
            .can_check_for_updates());
        assert!(RuntimeShape::PreRelease
            .update_channel()
            .can_check_for_updates());
    }
}
