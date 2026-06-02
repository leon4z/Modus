// Purpose: Pi Agent adapter entry and runtime capability boundary.

use crate::adapters::{
    adapter_can_write_path, discover_rule_sources_for_adapter, RuleSource, ToolAdapter,
    ToolCapability, ToolCapabilityAccess, ToolCapabilityAction, ToolCapabilityActionEvidence,
    ToolCapabilityFormat, ToolCapabilityKind, ToolCapabilityScope, ToolCapabilitySourceConfidence,
    ToolCapabilitySourceKind, ToolCapabilitySupportingSource, ToolCapabilitySupportingSourceRole,
    ToolSourceDiagnosticState,
};
use crate::platform::tool_adapters::declared::{directory_diagnostics, file_diagnostics};
use crate::platform::tool_adapters::{command_exists, is_executable_file};
use crate::platform::tool_capabilities::runtime_action_gates;
use crate::platform::tool_catalog::registry;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub struct PiAgentAdapter {
    home: PathBuf,
    config_dir: PathBuf,
}

impl PiAgentAdapter {
    fn new(home: PathBuf, config_dir: PathBuf) -> Self {
        Self { home, config_dir }
    }

    fn settings_path(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    fn mcp_adapter_configured(&self) -> bool {
        pi_mcp_adapter_configured(&self.settings_path())
    }
}

pub fn create(home: &Path) -> Box<dyn ToolAdapter> {
    let definition = registry::definition("pi-agent").expect("missing Pi Agent catalog definition");
    Box::new(PiAgentAdapter::new(
        home.to_path_buf(),
        definition.config_dir.resolve_path(home),
    ))
}

impl ToolAdapter for PiAgentAdapter {
    fn id(&self) -> &str {
        "pi-agent"
    }

    fn name(&self) -> &str {
        "Pi Agent"
    }

    fn icon(&self) -> &str {
        "pi-agent"
    }

    fn config_dir(&self) -> PathBuf {
        self.config_dir.clone()
    }

    fn detect(&self) -> bool {
        command_exists("pi")
            || ["/opt/homebrew/bin/pi", "/usr/local/bin/pi"]
                .iter()
                .any(|path| is_executable_file(Path::new(path)))
    }

    fn read_rules(&self) -> Result<Vec<RuleSource>, String> {
        discover_rule_sources_for_adapter(self.id(), &self.capabilities())
    }

    fn write_rule(&self, path: &str, content: &str) -> Result<(), String> {
        let target = PathBuf::from(shellexpand::tilde(path).to_string());
        if !adapter_can_write_path(self, ToolCapabilityKind::Rule, &target) {
            return Err("Rule writing is not enabled for this Pi Agent target".to_string());
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(target, content).map_err(|error| error.to_string())
    }

    fn skills_dir(&self) -> Option<PathBuf> {
        Some(self.config_dir.join("skills"))
    }

    fn supports_generic_skills(&self) -> bool {
        true
    }

    fn allow_external_generic_symlink(&self) -> bool {
        true
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        capabilities(&self.home, &self.config_dir, self.mcp_adapter_configured())
    }
}

fn capabilities(
    home: &Path,
    config_dir: &Path,
    mcp_adapter_configured: bool,
) -> Vec<ToolCapability> {
    let settings_path = config_dir.join("settings.json");
    let mut capabilities = vec![
        capability(
            "global-rule",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Markdown,
            config_dir.join("AGENTS.md"),
            "AGENTS.md",
            file_diagnostics(),
            "Pi Agent loads ~/.pi/agent/AGENTS.md as user-level global instruction context.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            ToolCapabilitySourceKind::FeatureSource,
            None,
            vec![],
            runtime_action_gates(
                "Pi Agent global instructions",
                "~/.pi/agent/AGENTS.md",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                    ToolCapabilityAction::Create,
                    ToolCapabilityAction::Edit,
                    ToolCapabilityAction::Save,
                    ToolCapabilityAction::Inject,
                ],
            ),
        ),
        capability(
            "rule-directory",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Unsupported,
            ToolCapabilityFormat::Unknown,
            PathBuf::new(),
            "Rule Directory",
            vec![ToolSourceDiagnosticState::Unsupported],
            "No global/user-level Pi Agent rule directory is certified; Modus must not invent ~/.pi/agent/rules.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            ToolCapabilitySourceKind::FeatureSource,
            None,
            vec![],
            vec![],
        ),
        capability(
            "shared-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Shared,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::SkillDirectory,
            crate::platform::env::generic_skills_dir(),
            "Shared Skills",
            directory_diagnostics(),
            "Pi Agent discovers shared personal Skills from ~/.agents/skills; Skill management owns shared writes.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            ToolCapabilitySourceKind::FeatureSource,
            None,
            vec![],
            runtime_action_gates(
                "Pi Agent shared Skills directory",
                "~/.agents/skills",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                ],
            ),
        ),
        capability(
            "agent-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            config_dir.join("skills"),
            "Agent Skills",
            directory_diagnostics(),
            "Pi Agent discovers Pi-owned user Skills from ~/.pi/agent/skills.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            ToolCapabilitySourceKind::FeatureSource,
            None,
            vec![],
            runtime_action_gates(
                "Pi Agent user Skills directory",
                "~/.pi/agent/skills",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                    ToolCapabilityAction::Edit,
                    ToolCapabilityAction::Save,
                    ToolCapabilityAction::Install,
                    ToolCapabilityAction::Copy,
                    ToolCapabilityAction::Uninstall,
                    ToolCapabilityAction::Delete,
                    ToolCapabilityAction::Link,
                ],
            ),
        ),
    ];

    if mcp_adapter_configured {
        capabilities.extend([
            mcp_config_capability(
                "shared-mcp-config",
                home.join(".config/mcp/mcp.json"),
                "Shared MCP configuration",
                "Pi Agent can read shared MCP servers from ~/.config/mcp/mcp.json when pi-mcp-adapter is installed.",
                &settings_path,
            ),
            mcp_config_capability(
                "pi-mcp-config",
                config_dir.join("mcp.json"),
                "Pi MCP configuration",
                "Pi Agent can read Pi-specific MCP servers from ~/.pi/agent/mcp.json when pi-mcp-adapter is installed.",
                &settings_path,
            ),
        ]);
    } else {
        capabilities.push(capability(
            "mcp-adapter",
            ToolCapabilityKind::Mcp,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Unsupported,
            ToolCapabilityFormat::Unknown,
            PathBuf::new(),
            "pi-mcp-adapter",
            vec![ToolSourceDiagnosticState::Unsupported],
            "Install pi-mcp-adapter in Pi Agent to enable MCP configuration sources.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            ToolCapabilitySourceKind::FeatureSource,
            None,
            vec![settings_supporting_source(&settings_path)],
            vec![],
        ));
    }

    capabilities.push(capability(
        "ordinary-settings",
        ToolCapabilityKind::OrdinaryConfig,
        ToolCapabilityScope::Global,
        ToolCapabilityAccess::Readable,
        ToolCapabilityFormat::Json,
        settings_path,
        "Settings",
        file_diagnostics(),
        "Pi Agent user settings live in ~/.pi/agent/settings.json; Modus reads this as ordinary configuration evidence.",
        ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
        ToolCapabilitySourceKind::FeatureSource,
        None,
        vec![],
        runtime_action_gates(
            "Pi Agent settings",
            "~/.pi/agent/settings.json",
            &[
                ToolCapabilityAction::View,
                ToolCapabilityAction::Read,
                ToolCapabilityAction::Diagnose,
            ],
        ),
    ));

    capabilities
}

fn mcp_config_capability(
    id: &'static str,
    source_path: PathBuf,
    label: &'static str,
    notes: &'static str,
    settings_path: &Path,
) -> ToolCapability {
    capability(
        id,
        ToolCapabilityKind::Mcp,
        ToolCapabilityScope::Global,
        ToolCapabilityAccess::ReadOnly,
        ToolCapabilityFormat::Json,
        source_path,
        label,
        file_diagnostics(),
        notes,
        ToolCapabilitySourceConfidence::OfficialDocs,
        ToolCapabilitySourceKind::FeatureSource,
        None,
        vec![settings_supporting_source(settings_path)],
        runtime_action_gates(
            label,
            "pi-mcp-adapter MCP configuration",
            &[
                ToolCapabilityAction::View,
                ToolCapabilityAction::Read,
                ToolCapabilityAction::Diagnose,
            ],
        ),
    )
}

fn settings_supporting_source(settings_path: &Path) -> ToolCapabilitySupportingSource {
    ToolCapabilitySupportingSource {
        id: "pi-agent-settings".to_string(),
        role: ToolCapabilitySupportingSourceRole::Settings,
        source_path: settings_path.to_string_lossy().to_string(),
        format: ToolCapabilityFormat::Json,
        required: true,
        diagnostics: file_diagnostics(),
        notes: "The pi-mcp-adapter package declaration gates Pi Agent MCP support.".to_string(),
    }
}

fn capability(
    id: &'static str,
    kind: ToolCapabilityKind,
    scope: ToolCapabilityScope,
    access: ToolCapabilityAccess,
    format: ToolCapabilityFormat,
    source_path: PathBuf,
    label: &'static str,
    diagnostics: Vec<ToolSourceDiagnosticState>,
    notes: &'static str,
    source_confidence: ToolCapabilitySourceConfidence,
    source_kind: ToolCapabilitySourceKind,
    primary_config_dir: Option<String>,
    supporting_sources: Vec<ToolCapabilitySupportingSource>,
    action_evidence: Vec<ToolCapabilityActionEvidence>,
) -> ToolCapability {
    ToolCapability {
        id: id.to_string(),
        kind,
        scope,
        access,
        format,
        source_path: source_path.to_string_lossy().to_string(),
        label: label.to_string(),
        diagnostics,
        source_confidence,
        notes: notes.to_string(),
        source_kind,
        primary_config_dir,
        supporting_sources,
        action_evidence,
    }
}

fn pi_mcp_adapter_configured(settings_path: &Path) -> bool {
    let Ok(content) = fs::read_to_string(settings_path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(&content) else {
        return false;
    };
    let Some(packages) = value
        .get("packages")
        .and_then(|packages| packages.as_array())
    else {
        return false;
    };
    packages
        .iter()
        .filter_map(package_source)
        .any(|source| source.contains("pi-mcp-adapter"))
}

fn package_source(package: &Value) -> Option<&str> {
    package
        .as_str()
        .or_else(|| package.get("source").and_then(|source| source.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::tool_capabilities::capability_projections::{
        project_capabilities, ToolCapabilityAction as ProjectedAction,
        ToolCapabilityExclusionReason, ToolCapabilityModule, ToolCapabilitySourceRole,
    };

    fn write_settings(home: &Path, content: &str) {
        let settings = home.join(".pi/agent/settings.json");
        fs::create_dir_all(settings.parent().unwrap()).unwrap();
        fs::write(settings, content).unwrap();
    }

    #[test]
    fn runtime_capabilities_without_mcp_adapter_keep_mcp_unavailable() {
        let tmp = tempfile::tempdir().unwrap();
        write_settings(tmp.path(), r#"{"packages":[]}"#);

        let adapter = create(tmp.path());
        let capabilities = adapter.capabilities();

        assert!(capabilities
            .iter()
            .filter(|capability| !capability.action_evidence.is_empty())
            .all(|capability| capability
                .action_evidence
                .iter()
                .all(|evidence| evidence.version.is_none() && evidence.verified_at.is_none())));

        let rule_projections =
            project_capabilities("pi-agent", ToolCapabilityModule::Rules, &capabilities);
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleGlobalTarget
                && projection.allows(&ProjectedAction::Inject)
                && projection
                    .evidence
                    .source_path
                    .ends_with(".pi/agent/AGENTS.md")
        }));
        let rule_directory = rule_projections
            .iter()
            .find(|projection| projection.evidence.id == "rule-directory")
            .expect("Pi Agent Rule Directory evidence");
        assert_eq!(
            rule_directory.source_role,
            ToolCapabilitySourceRole::RuleNonActionableEvidence
        );
        assert_eq!(
            rule_directory.exclusion_reason,
            Some(ToolCapabilityExclusionReason::Unsupported)
        );

        let skill_projections =
            project_capabilities("pi-agent", ToolCapabilityModule::Skills, &capabilities);
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.allows(&ProjectedAction::Install)
                && projection
                    .evidence
                    .source_path
                    .ends_with(".pi/agent/skills")
        }));
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
                && projection.allows(&ProjectedAction::View)
        }));

        let mcp_projections =
            project_capabilities("pi-agent", ToolCapabilityModule::Mcp, &capabilities);
        let mcp = mcp_projections
            .iter()
            .find(|projection| projection.evidence.id == "mcp-adapter")
            .expect("Pi Agent MCP adapter evidence");
        assert_eq!(
            mcp.source_role,
            ToolCapabilitySourceRole::McpNonActionableEvidence
        );
        assert_eq!(
            mcp.exclusion_reason,
            Some(ToolCapabilityExclusionReason::Unsupported)
        );
        assert!(mcp.evidence.notes.contains("pi-mcp-adapter"));
    }

    #[test]
    fn runtime_capabilities_with_mcp_adapter_expose_two_global_sources() {
        let tmp = tempfile::tempdir().unwrap();
        write_settings(
            tmp.path(),
            r#"{"packages":[{"source":"npm:pi-mcp-adapter"}]}"#,
        );

        let adapter = create(tmp.path());
        let capabilities = adapter.capabilities();
        let mcp_projections =
            project_capabilities("pi-agent", ToolCapabilityModule::Mcp, &capabilities);

        assert!(mcp_projections.iter().any(|projection| {
            projection.evidence.id == "shared-mcp-config"
                && projection.source_role == ToolCapabilitySourceRole::McpGlobalConfig
                && projection.allows(&ProjectedAction::View)
                && !projection.allows(&ProjectedAction::Save)
                && projection
                    .evidence
                    .source_path
                    .ends_with(".config/mcp/mcp.json")
        }));
        assert!(mcp_projections.iter().any(|projection| {
            projection.evidence.id == "pi-mcp-config"
                && projection.source_role == ToolCapabilitySourceRole::McpGlobalConfig
                && projection.allows(&ProjectedAction::View)
                && !projection.allows(&ProjectedAction::Save)
                && projection
                    .evidence
                    .source_path
                    .ends_with(".pi/agent/mcp.json")
        }));
        assert!(!mcp_projections
            .iter()
            .any(|projection| projection.evidence.id == "mcp-adapter"));
    }
}
