// Purpose: Claude Code tool adapter for paths, rules, skills, and MCP listings.

use super::{adapter_can_write_path, discover_rule_sources_for_adapter, RuleSource, ToolAdapter};
use crate::platform::tool_adapters::declared::{directory_diagnostics, file_diagnostics};
use crate::platform::tool_adapters::{command_exists, ToolPresence};
use crate::platform::tool_capabilities::{
    runtime_action_gates, ToolCapability, ToolCapabilityAccess, ToolCapabilityAction,
    ToolCapabilityActionEvidence, ToolCapabilityFormat, ToolCapabilityKind, ToolCapabilityScope,
    ToolCapabilitySourceConfidence, ToolCapabilitySourceKind, ToolSourceDiagnosticState,
};
use crate::platform::tool_catalog::{registry, types::ToolDefinition};
use std::fs;
use std::path::{Path, PathBuf};

pub struct ClaudeCodeAdapter {
    home: PathBuf,
}

impl ClaudeCodeAdapter {
    pub fn new(home: PathBuf) -> Self {
        Self { home }
    }

    fn definition(&self) -> &'static ToolDefinition {
        registry::definition("claude-code").expect("missing Claude Code catalog definition")
    }
}

fn claude_code_presence(cli_detected: bool) -> ToolPresence {
    ToolPresence::from_presence(false, cli_detected)
}

fn capabilities(home: &Path) -> Vec<ToolCapability> {
    vec![
        capability(
            "global-rule",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Markdown,
            home.join(".claude/CLAUDE.md"),
            "CLAUDE.md",
            file_diagnostics(),
            "Claude Code loads the user-level CLAUDE.md memory file as instructions.",
            runtime_action_gates(
                "Claude Code user memory file",
                "~/.claude/CLAUDE.md",
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
            "global-rules-directory",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Directory,
            home.join(".claude/rules"),
            "Rules directory",
            directory_diagnostics(),
            "Claude Code loads user-level markdown rule files from ~/.claude/rules.",
            runtime_action_gates(
                "Claude Code user rules directory",
                "~/.claude/rules",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                    ToolCapabilityAction::Create,
                    ToolCapabilityAction::Edit,
                    ToolCapabilityAction::Save,
                    ToolCapabilityAction::Delete,
                ],
            ),
        ),
        capability(
            "dedicated-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            home.join(".claude/skills"),
            "Dedicated Skills",
            directory_diagnostics(),
            "Claude Code discovers personal Skills from ~/.claude/skills; shared ~/.agents/skills direct-read was not verified.",
            runtime_action_gates(
                "Claude Code personal Skills directory",
                "~/.claude/skills",
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
            "ordinary-settings",
            ToolCapabilityKind::OrdinaryConfig,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            home.join(".claude/settings.json"),
            "Settings",
            file_diagnostics(),
            "Claude Code user settings live at ~/.claude/settings.json; Rules, Skills, and MCP remain separate capability rows.",
            runtime_action_gates(
                "Claude Code user settings",
                "~/.claude/settings.json",
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
            ToolCapabilityFormat::Json,
            home.join(".claude.json"),
            "MCP configuration",
            file_diagnostics(),
            "Claude Code user-scope MCP configuration is stored in ~/.claude.json; this does not certify MCP server lifecycle or connectivity.",
            runtime_action_gates(
                "Claude Code user MCP configuration",
                "~/.claude.json",
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
        source_confidence: ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
        notes: notes.to_string(),
        source_kind: ToolCapabilitySourceKind::FeatureSource,
        primary_config_dir: None,
        supporting_sources: vec![],
        action_evidence,
    }
}

impl ToolAdapter for ClaudeCodeAdapter {
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
        claude_code_presence(command_exists("claude"))
    }

    fn read_rules(&self) -> Result<Vec<RuleSource>, String> {
        discover_rule_sources_for_adapter(self.id(), &self.capabilities())
    }

    fn write_rule(&self, path: &str, content: &str) -> Result<(), String> {
        let target = PathBuf::from(shellexpand::tilde(path).to_string());
        if !adapter_can_write_path(self, ToolCapabilityKind::Rule, &target) {
            return Err("Rule writing is not enabled for this Claude Code target".to_string());
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(target, content).map_err(|e| e.to_string())
    }

    fn skills_dir(&self) -> Option<PathBuf> {
        Some(self.home.join(".claude/skills"))
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
    fn claude_app_without_claude_cli_does_not_detect_claude_code() {
        let presence = claude_code_presence(false);

        assert!(!presence.detected);
        assert!(!presence.app_detected);
        assert!(!presence.cli_detected);
        assert_eq!(presence.label, "");
    }

    #[test]
    fn claude_code_presence_is_cli_only() {
        let presence = claude_code_presence(true);

        assert!(presence.detected);
        assert!(!presence.app_detected);
        assert!(presence.cli_detected);
        assert_eq!(presence.label, "CLI");
    }

    #[test]
    fn claude_rule_discovery_uses_memory_file_and_rules_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(config_dir.join("rules/team")).unwrap();
        std::fs::create_dir_all(config_dir.join("skills/demo")).unwrap();
        std::fs::write(config_dir.join("CLAUDE.md"), "memory").unwrap();
        std::fs::write(config_dir.join("rules/team/frontend.md"), "team").unwrap();
        std::fs::write(config_dir.join("OTHER.md"), "other").unwrap();
        std::fs::write(config_dir.join("skills/demo/SKILL.md"), "skill").unwrap();
        let adapter = ClaudeCodeAdapter::new(tmp.path().to_path_buf());

        let rules = adapter.read_rules().unwrap();
        let labels: Vec<_> = rules.iter().map(|rule| rule.label.as_str()).collect();

        assert_eq!(labels, vec!["CLAUDE.md", "frontend.md", "team"]);
        assert!(rules.iter().any(|rule| rule.group == "team"));
        assert!(!rules.iter().any(|rule| rule.path.contains("SKILL.md")));
        assert!(!rules.iter().any(|rule| rule.path.contains("OTHER.md")));
    }

    #[test]
    fn runtime_capabilities_are_adapter_owned_and_snapshot_free() {
        let adapter = ClaudeCodeAdapter::new(Path::new("/Users/example").to_path_buf());
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

        let rule_projections =
            project_capabilities("claude-code", ToolCapabilityModule::Rules, &capabilities);
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleGlobalTarget
                && projection.allows(&ProjectedAction::Inject)
                && projection.evidence.source_path == "/Users/example/.claude/CLAUDE.md"
        }));
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleNativeFileSource
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.claude/rules"
        }));

        let skill_projections =
            project_capabilities("claude-code", ToolCapabilityModule::Skills, &capabilities);
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.allows(&ProjectedAction::Install)
                && projection.evidence.source_path == "/Users/example/.claude/skills"
        }));
        assert!(!skill_projections.iter().any(
            |projection| projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
        ));

        let mcp_projections =
            project_capabilities("claude-code", ToolCapabilityModule::Mcp, &capabilities);
        assert!(mcp_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::McpGlobalConfig
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.claude.json"
        }));

        let settings_projections = project_capabilities(
            "claude-code",
            ToolCapabilityModule::OrdinaryConfig,
            &capabilities,
        );
        assert!(settings_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::OrdinaryConfigFile
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.claude/settings.json"
        }));
    }

    #[test]
    fn read_mcp_servers_maps_disabled_flag_to_activation_state() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(".claude.json"),
            r#"{
              "mcpServers": {
                "enabled-demo": { "command": "node" },
                "disabled-demo": { "command": "node", "disabled": true }
              }
            }"#,
        )
        .unwrap();
        let adapter = ClaudeCodeAdapter::new(tmp.path().to_path_buf());

        let servers = adapter.read_mcp_servers().unwrap();
        let enabled = servers
            .iter()
            .find(|server| server.name == "enabled-demo")
            .unwrap();
        let disabled = servers
            .iter()
            .find(|server| server.name == "disabled-demo")
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
