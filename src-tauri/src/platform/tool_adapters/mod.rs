// Purpose: Shared adapter scaffolding that is not tied to a single tool entry.

pub mod declared;

use crate::platform::tool_capabilities::{
    capability_declared_source_path, capability_matches_eligible_global_rule_target,
    capability_matches_path, capability_projections, capability_source_is_directory,
    effective_capabilities, mcp_sources, rule_sources::can_write_rule_path_for_adapter,
};
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

pub use crate::platform::tool_capabilities::{
    get_file_modified, rule_sources::discover_rule_sources_for_adapter, RuleFormat, RuleSource,
    ToolCapability, ToolCapabilityAccess, ToolCapabilityActionEvidence, ToolCapabilityFormat,
    ToolCapabilityKind, ToolCapabilityScope, ToolCapabilitySourceConfidence,
    ToolCapabilitySourceKind, ToolCapabilitySupportingSource, ToolCapabilitySupportingSourceRole,
    ToolSourceDiagnosticState,
};

use crate::platform::tool_capabilities::skills::{McpServerInfo, SkillInfo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPresence {
    pub detected: bool,
    pub app_detected: bool,
    pub cli_detected: bool,
    pub label: String,
}

impl ToolPresence {
    pub fn from_presence(app_detected: bool, cli_detected: bool) -> Self {
        let label = match (app_detected, cli_detected) {
            (true, true) => "APP+CLI",
            (true, false) => "APP",
            (false, true) => "CLI",
            (false, false) => "",
        }
        .to_string();
        Self {
            detected: app_detected || cli_detected,
            app_detected,
            cli_detected,
            label,
        }
    }

    pub fn unlabeled(detected: bool) -> Self {
        Self {
            detected,
            app_detected: false,
            cli_detected: false,
            label: String::new(),
        }
    }
}

pub trait ToolAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn icon(&self) -> &str;
    fn config_dir(&self) -> PathBuf;
    fn primary_config_dir(&self) -> PathBuf {
        self.config_dir()
    }
    fn detect(&self) -> bool;
    fn presence(&self) -> ToolPresence {
        ToolPresence::unlabeled(self.detect())
    }
    fn read_rules(&self) -> Result<Vec<RuleSource>, String>;
    fn write_rule(&self, path: &str, content: &str) -> Result<(), String>;

    fn skills_dir(&self) -> Option<PathBuf> {
        None
    }

    fn read_skills(&self) -> Result<Vec<SkillInfo>, String> {
        match self.skills_dir() {
            Some(dir) => Ok(crate::platform::tool_capabilities::skills::scan_skills_dir(
                &dir,
                self.id(),
            )),
            None => Ok(vec![]),
        }
    }

    fn read_mcp_servers(&self) -> Result<Vec<McpServerInfo>, String> {
        let mut servers = vec![];
        for capability in self.capabilities() {
            if capability.kind != ToolCapabilityKind::Mcp
                || capability.scope == ToolCapabilityScope::Project
                || !capability.is_readable()
            {
                continue;
            }
            servers.extend(mcp_sources::parse_mcp_servers_for_capability(&capability)?);
        }
        Ok(servers)
    }

    /// Whether this tool auto-reads ~/.agents/skills/
    fn supports_generic_skills(&self) -> bool {
        false
    }

    /// Whether unmanaged external symlink to generic dir is allowed as a runtime state.
    fn allow_external_generic_symlink(&self) -> bool {
        false
    }

    /// Product capability declarations used to gate read/write/diagnostic behavior.
    fn capabilities(&self) -> Vec<ToolCapability> {
        let mut capabilities = vec![];
        if let Some(skills_dir) = self.skills_dir() {
            capabilities.push(ToolCapability {
                id: "dedicated-skills".to_string(),
                kind: ToolCapabilityKind::Skill,
                scope: ToolCapabilityScope::Tool,
                access: ToolCapabilityAccess::ReadOnly,
                format: ToolCapabilityFormat::SkillDirectory,
                source_path: skills_dir.to_string_lossy().to_string(),
                label: "Dedicated Skills".to_string(),
                diagnostics: vec![
                    ToolSourceDiagnosticState::Missing,
                    ToolSourceDiagnosticState::Loaded,
                ],
                source_confidence: ToolCapabilitySourceConfidence::Unknown,
                notes: "Default adapter Skill directory capability is read-only until adapter-specific implementations prove write support.".to_string(),
                source_kind: ToolCapabilitySourceKind::FeatureSource,
                primary_config_dir: None,
                supporting_sources: vec![],
                action_evidence: vec![],
            });
        }
        if self.supports_generic_skills() {
            capabilities.push(ToolCapability {
                id: "shared-skills".to_string(),
                kind: ToolCapabilityKind::Skill,
                scope: ToolCapabilityScope::Shared,
                access: ToolCapabilityAccess::ReadOnly,
                format: ToolCapabilityFormat::SkillDirectory,
                source_path: crate::platform::env::generic_skills_dir()
                    .to_string_lossy()
                    .to_string(),
                label: "Shared Skills".to_string(),
                diagnostics: vec![
                    ToolSourceDiagnosticState::Missing,
                    ToolSourceDiagnosticState::Loaded,
                ],
                source_confidence: ToolCapabilitySourceConfidence::Unknown,
                notes: "Shared Skill consumption only; management writes remain governed by Skill workflows.".to_string(),
                source_kind: ToolCapabilitySourceKind::FeatureSource,
                primary_config_dir: None,
                supporting_sources: vec![],
                action_evidence: vec![],
            });
        }
        capabilities
    }
}

pub(crate) fn command_exists(command: &str) -> bool {
    command_exists_in_paths(command, command_search_paths())
}

fn command_exists_in_paths<I>(command: &str, paths: I) -> bool
where
    I: IntoIterator<Item = PathBuf>,
{
    paths
        .into_iter()
        .any(|path| is_executable_file(&path.join(command)))
}

fn command_search_paths() -> impl Iterator<Item = PathBuf> {
    unique_command_paths(path_env_command_dirs().chain(fallback_command_dirs()))
}

fn path_env_command_dirs() -> impl Iterator<Item = PathBuf> {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
}

fn fallback_command_dirs() -> impl Iterator<Item = PathBuf> {
    let home = dirs::home_dir();
    [
        Some(PathBuf::from("/opt/homebrew/bin")),
        Some(PathBuf::from("/usr/local/bin")),
        Some(PathBuf::from("/opt/local/bin")),
        Some(PathBuf::from("/usr/bin")),
        Some(PathBuf::from("/bin")),
        home.as_ref().map(|path| path.join(".local/bin")),
        home.as_ref().map(|path| path.join(".cargo/bin")),
    ]
    .into_iter()
    .flatten()
}

fn unique_command_paths<I>(paths: I) -> impl Iterator<Item = PathBuf>
where
    I: IntoIterator<Item = PathBuf>,
{
    let mut seen = HashSet::<OsString>::new();
    paths.into_iter().filter(move |path| {
        let key = path.as_os_str().to_os_string();
        seen.insert(key)
    })
}

#[cfg(unix)]
pub(crate) fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
pub(crate) fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn command_search_paths_keeps_path_env_before_fallbacks_and_dedupes() {
        let path_env = std::env::join_paths([
            PathBuf::from("/tmp/modus-command-a"),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/tmp/modus-command-a"),
        ])
        .unwrap();

        let paths: Vec<_> =
            unique_command_paths(std::env::split_paths(&path_env).chain(fallback_command_dirs()))
                .collect();

        assert_eq!(paths[0], PathBuf::from("/tmp/modus-command-a"));
        assert_eq!(paths[1], PathBuf::from("/opt/homebrew/bin"));
        assert_eq!(
            paths
                .iter()
                .filter(|path| path.as_path() == Path::new("/opt/homebrew/bin"))
                .count(),
            1
        );
        assert!(paths.iter().any(|path| path == Path::new("/usr/local/bin")));
    }

    #[test]
    fn command_exists_checks_executable_files_in_candidate_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let command_path = tmp.path().join("modus-test-command");
        fs::write(&command_path, "").unwrap();
        let mut permissions = fs::metadata(&command_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&command_path, permissions).unwrap();

        assert!(command_exists_in_paths(
            "modus-test-command",
            unique_command_paths([tmp.path().to_path_buf()])
        ));
    }
}

pub fn adapter_has_default_global_rule_file_target(adapter: &dyn ToolAdapter) -> bool {
    capability_projections::project_capabilities(
        adapter.id(),
        capability_projections::ToolCapabilityModule::Rules,
        &adapter.capabilities(),
    )
    .iter()
    .any(|projection| {
        projection.allows(&capability_projections::ToolCapabilityAction::Inject)
            && capability_declared_source_path(&projection.evidence)
                .map(|source| !capability_source_is_directory(&projection.evidence, &source))
                .unwrap_or(false)
    })
}

pub fn adapter_can_manage_global_rule_target(adapter: &dyn ToolAdapter, path: &Path) -> bool {
    let config = crate::platform::config::load_config();
    let capabilities = effective_capabilities::resolve_effective_capabilities(
        adapter.id(),
        &adapter.capabilities(),
        &config,
    );
    capability_projections::project_capabilities(
        adapter.id(),
        capability_projections::ToolCapabilityModule::Rules,
        &capabilities,
    )
    .iter()
    .filter(|projection| projection.allows(&capability_projections::ToolCapabilityAction::Inject))
    .any(|projection| {
        capability_matches_eligible_global_rule_target(adapter.id(), &projection.evidence, path)
    })
}

pub fn adapter_can_write_path(
    adapter: &dyn ToolAdapter,
    kind: ToolCapabilityKind,
    path: &Path,
) -> bool {
    let config = crate::platform::config::load_config();
    let capabilities = effective_capabilities::resolve_effective_capabilities(
        adapter.id(),
        &adapter.capabilities(),
        &config,
    );
    if kind == ToolCapabilityKind::Rule {
        return can_write_rule_path_for_adapter(adapter.id(), &capabilities, path);
    }
    capabilities.iter().any(|capability| {
        capability.kind == kind
            && capability.is_writable()
            && capability_matches_path(capability, path)
    })
}
