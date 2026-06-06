// Purpose: Codex tool adapter for paths, rules, skills, and MCP listings.

use super::{
    adapter_can_write_path, discover_rule_sources_for_adapter, RuleSource, ToolAdapter,
    ToolCapability, ToolCapabilityKind,
};
use crate::platform::tool_adapters::declared::{directory_diagnostics, file_diagnostics};
use crate::platform::tool_adapters::{command_exists, ToolPresence};
use crate::platform::tool_capabilities::{
    runtime_action_gates, ToolCapabilityAccess, ToolCapabilityAction, ToolCapabilityFormat,
    ToolCapabilityScope, ToolCapabilitySourceConfidence, ToolCapabilitySourceKind,
    ToolSourceDiagnosticState,
};
use crate::platform::tool_catalog::{registry, types::ToolDefinition};
use std::fs;
use std::path::{Path, PathBuf};

pub struct CodexAdapter {
    home: PathBuf,
}

impl CodexAdapter {
    pub fn new(home: PathBuf) -> Self {
        Self { home }
    }

    fn definition(&self) -> &'static ToolDefinition {
        registry::definition("codex").expect("missing Codex catalog definition")
    }
}

fn codex_presence(app_detected: bool, cli_detected: bool) -> ToolPresence {
    ToolPresence::from_presence(app_detected, cli_detected)
}

fn capabilities(home: &Path) -> Vec<ToolCapability> {
    vec![
        capability(
            "global-rule",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Markdown,
            home.join(".codex/AGENTS.md"),
            "AGENTS.md",
            file_diagnostics(),
            "Codex loads the user-level AGENTS.md instruction file from the active Codex home.",
            runtime_action_gates(
                "Codex user AGENTS.md instruction file",
                "~/.codex/AGENTS.md",
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
            "dedicated-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            home.join(".codex/skills"),
            "Dedicated Skills",
            directory_diagnostics(),
            "Codex discovers personal Skills from the active Codex home Skills directory.",
            runtime_action_gates(
                "Codex personal Skills directory",
                "~/.codex/skills",
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
                ],
            ),
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
            "Codex discovers personal shared Skills from ~/.agents/skills; Skill management owns writes.",
            runtime_action_gates(
                "Codex shared Skills source",
                "~/.agents/skills",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                    ToolCapabilityAction::Copy,
                ],
            ),
        ),
        capability(
            "ordinary-config",
            ToolCapabilityKind::OrdinaryConfig,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Toml,
            home.join(".codex/config.toml"),
            "Config",
            file_diagnostics(),
            "Codex reads and validates the user-level config.toml file; Rules, Skills, and MCP remain separate capability rows.",
            runtime_action_gates(
                "Codex user configuration",
                "~/.codex/config.toml",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                    ToolCapabilityAction::Edit,
                    ToolCapabilityAction::Save,
                ],
            ),
        ),
        capability(
            "mcp-config",
            ToolCapabilityKind::Mcp,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Toml,
            home.join(".codex/config.toml"),
            "MCP configuration",
            file_diagnostics(),
            "Codex user-scope MCP configuration is stored under mcp_servers tables in config.toml; this does not certify server lifecycle or connectivity.",
            runtime_action_gates(
                "Codex user MCP configuration",
                "~/.codex/config.toml",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                    ToolCapabilityAction::Edit,
                    ToolCapabilityAction::Save,
                ],
            ),
        ),
    ]
}

fn capability(
    id: &str,
    kind: ToolCapabilityKind,
    scope: ToolCapabilityScope,
    access: ToolCapabilityAccess,
    format: ToolCapabilityFormat,
    source_path: PathBuf,
    label: &str,
    diagnostics: Vec<ToolSourceDiagnosticState>,
    notes: &str,
    action_evidence: Vec<crate::platform::tool_capabilities::ToolCapabilityActionEvidence>,
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
        source_confidence: ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
        notes: notes.to_string(),
        source_kind: ToolCapabilitySourceKind::FeatureSource,
        primary_config_dir: None,
        supporting_sources: vec![],
        action_evidence,
    }
}

impl ToolAdapter for CodexAdapter {
    fn id(&self) -> &str {
        self.definition().id
    }
    fn name(&self) -> &str {
        self.definition().name
    }
    fn icon(&self) -> &str {
        self.definition().icon
    }

    fn config_dir(&self) -> PathBuf {
        self.definition().config_dir.resolve_path(&self.home)
    }

    fn detect(&self) -> bool {
        self.presence().detected
    }

    fn presence(&self) -> ToolPresence {
        codex_presence(
            Path::new("/Applications/Codex.app").exists(),
            command_exists("codex"),
        )
    }

    fn read_rules(&self) -> Result<Vec<RuleSource>, String> {
        discover_rule_sources_for_adapter(self.id(), &self.capabilities())
    }

    fn write_rule(&self, path: &str, content: &str) -> Result<(), String> {
        let target = PathBuf::from(shellexpand::tilde(path).to_string());
        if !adapter_can_write_path(self, ToolCapabilityKind::Rule, &target) {
            return Err("Rule writing is not enabled for this Codex target".to_string());
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(target, content).map_err(|e| e.to_string())
    }

    fn skills_dir(&self) -> Option<PathBuf> {
        Some(self.home.join(".codex/skills"))
    }

    fn supports_generic_skills(&self) -> bool {
        true
    }

    fn allow_external_generic_symlink(&self) -> bool {
        true
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        capabilities(&self.home)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::tool_capabilities::capability_projections::{
        project_capabilities, ToolCapabilityAction as ProjectedAction, ToolCapabilityModule,
        ToolCapabilitySourceRole,
    };

    #[test]
    fn codex_presence_labels_app_only_cli_only_and_app_plus_cli() {
        let app_only = codex_presence(true, false);
        assert!(app_only.detected);
        assert!(app_only.app_detected);
        assert!(!app_only.cli_detected);
        assert_eq!(app_only.label, "APP");

        let cli_only = codex_presence(false, true);
        assert!(cli_only.detected);
        assert!(!cli_only.app_detected);
        assert!(cli_only.cli_detected);
        assert_eq!(cli_only.label, "CLI");

        let both = codex_presence(true, true);
        assert!(both.detected);
        assert!(both.app_detected);
        assert!(both.cli_detected);
        assert_eq!(both.label, "APP+CLI");
    }

    #[test]
    fn codex_rule_discovery_uses_fixed_agents_file_only() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".codex");
        std::fs::create_dir_all(config_dir.join("rules/team")).unwrap();
        std::fs::create_dir_all(config_dir.join("skills/demo")).unwrap();
        std::fs::write(config_dir.join("AGENTS.md"), "agents").unwrap();
        std::fs::write(config_dir.join("rules/team/policy.rules"), "rules").unwrap();
        std::fs::write(config_dir.join("OTHER.md"), "other").unwrap();
        std::fs::write(config_dir.join("skills/demo/SKILL.md"), "skill").unwrap();
        let adapter = CodexAdapter::new(tmp.path().to_path_buf());

        let rules = adapter.read_rules().unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].label, "AGENTS.md");
        assert_eq!(rules[0].content, "agents");
    }

    #[test]
    fn runtime_capabilities_are_adapter_owned_and_skip_execpolicy_rules() {
        let adapter = CodexAdapter::new(Path::new("/Users/example").to_path_buf());
        let capabilities = adapter.capabilities();

        assert!(capabilities
            .iter()
            .all(|capability| capability.source_confidence
                == ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior));
        assert!(capabilities
            .iter()
            .filter(|capability| !capability.action_evidence.is_empty())
            .all(|capability| capability
                .action_evidence
                .iter()
                .all(|evidence| evidence.version.is_none() && evidence.verified_at.is_none())));
        assert!(!capabilities
            .iter()
            .any(|capability| capability.source_path.contains(".codex/rules")));

        let rule_projections =
            project_capabilities("codex", ToolCapabilityModule::Rules, &capabilities);
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleGlobalTarget
                && projection.allows(&ProjectedAction::Inject)
                && projection.evidence.source_path == "/Users/example/.codex/AGENTS.md"
        }));
        assert!(!rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleNativeFileSource
        }));

        let skill_projections =
            project_capabilities("codex", ToolCapabilityModule::Skills, &capabilities);
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.allows(&ProjectedAction::Install)
                && projection.evidence.source_path == "/Users/example/.codex/skills"
        }));
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
                && projection.allows(&ProjectedAction::Read)
                && !projection.allows(&ProjectedAction::Install)
        }));

        let mcp_projections =
            project_capabilities("codex", ToolCapabilityModule::Mcp, &capabilities);
        assert!(mcp_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::McpGlobalConfig
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.codex/config.toml"
        }));

        let settings_projections =
            project_capabilities("codex", ToolCapabilityModule::OrdinaryConfig, &capabilities);
        assert!(settings_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::OrdinaryConfigFile
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.codex/config.toml"
        }));
    }

    #[test]
    fn read_mcp_servers_maps_enabled_flag_to_activation_state() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".codex");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.toml"),
            r#"
[mcp_servers.enabled_demo]
command = "node"

[mcp_servers.disabled_demo]
command = "node"
enabled = false
"#,
        )
        .unwrap();
        let adapter = CodexAdapter::new(tmp.path().to_path_buf());

        let servers = adapter.read_mcp_servers().unwrap();
        let enabled = servers
            .iter()
            .find(|server| server.name == "enabled_demo")
            .unwrap();
        let disabled = servers
            .iter()
            .find(|server| server.name == "disabled_demo")
            .unwrap();

        assert!(enabled.enabled);
        assert_eq!(
            enabled.activation_state,
            crate::adapters::skills::McpServerActivationState::Enabled
        );
        assert!(!disabled.enabled);
        assert_eq!(
            disabled.activation_state,
            crate::adapters::skills::McpServerActivationState::Disabled
        );
    }
}
