// Purpose: OpenClaw tool adapter for paths, rules, skills, and MCP listings.

use super::{
    adapter_can_write_path, discover_rule_sources_for_adapter, RuleSource, ToolAdapter,
    ToolCapability, ToolCapabilityKind,
};
use crate::platform::tool_adapters::declared::{directory_diagnostics, file_diagnostics};
use crate::platform::tool_adapters::{command_exists, ToolPresence};
use crate::platform::tool_capabilities::{
    runtime_action_gates, ToolCapabilityAccess, ToolCapabilityAction, ToolCapabilityActionEvidence,
    ToolCapabilityFormat, ToolCapabilityScope, ToolCapabilitySourceConfidence,
    ToolCapabilitySourceKind, ToolSourceDiagnosticState,
};
use crate::platform::tool_catalog::{registry, types::ToolDefinition};
use std::fs;
use std::path::{Path, PathBuf};

pub struct OpenClawAdapter {
    home: PathBuf,
}

impl OpenClawAdapter {
    pub fn new(home: PathBuf) -> Self {
        Self { home }
    }

    fn definition(&self) -> &'static ToolDefinition {
        registry::definition("openclaw").expect("missing OpenClaw catalog definition")
    }

    fn workspace_rule_dirs(&self) -> Vec<(String, PathBuf)> {
        let config_dir = self.config_dir();
        let mut dirs = vec![];
        if let Ok(entries) = fs::read_dir(&config_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let Some(name) = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                else {
                    continue;
                };
                if name == "workspace" || name.starts_with("workspace-") {
                    dirs.push((name, path));
                }
            }
        }
        dirs.sort_by(|a, b| a.0.cmp(&b.0));
        dirs
    }
}

fn openclaw_presence(app_detected: bool, cli_detected: bool) -> ToolPresence {
    ToolPresence::from_presence(app_detected, cli_detected)
}

fn title_case_words(value: &str) -> String {
    value
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn capability(
    id: &str,
    kind: ToolCapabilityKind,
    scope: ToolCapabilityScope,
    access: ToolCapabilityAccess,
    format: ToolCapabilityFormat,
    source_path: String,
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
        source_path,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::tool_capabilities::capability_projections::{
        project_capabilities, ToolCapabilityAction as ProjectedAction, ToolCapabilityModule,
        ToolCapabilitySourceRole,
    };

    #[test]
    fn openclaw_presence_labels_app_only_cli_only_and_app_plus_cli() {
        let app_only = openclaw_presence(true, false);
        assert!(app_only.detected);
        assert!(app_only.app_detected);
        assert!(!app_only.cli_detected);
        assert_eq!(app_only.label, "APP");

        let cli_only = openclaw_presence(false, true);
        assert!(cli_only.detected);
        assert!(!cli_only.app_detected);
        assert!(cli_only.cli_detected);
        assert_eq!(cli_only.label, "CLI");

        let both = openclaw_presence(true, true);
        assert!(both.detected);
        assert!(both.app_detected);
        assert!(both.cli_detected);
        assert_eq!(both.label, "APP+CLI");
    }

    #[test]
    fn declares_trusted_workspace_agents_as_writable() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".openclaw").join("workspace")).unwrap();
        let adapter = OpenClawAdapter::new(tmp.path().to_path_buf());

        let capability = adapter
            .capabilities()
            .into_iter()
            .find(|capability| capability.id == "trusted-workspace-agents")
            .expect("expected trusted workspace AGENTS capability");

        assert_eq!(capability.access, ToolCapabilityAccess::Writable);
        assert_eq!(capability.kind, ToolCapabilityKind::Rule);
        assert!(capability.source_path.ends_with("workspace/AGENTS.md"));
    }

    #[test]
    fn declares_workspace_rule_directories_as_writable() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".openclaw").join("workspace")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".openclaw").join("workspace-mac")).unwrap();
        let adapter = OpenClawAdapter::new(tmp.path().to_path_buf());
        let capabilities = adapter.capabilities();

        for id in ["workspace-rules", "workspace-mac-rules"] {
            let capability = capabilities
                .iter()
                .find(|capability| capability.id == id)
                .expect("expected OpenClaw workspace directory rule capability");
            assert_eq!(capability.access, ToolCapabilityAccess::Writable);
            assert_eq!(capability.kind, ToolCapabilityKind::Rule);
            assert_eq!(capability.format, ToolCapabilityFormat::Directory);
            assert_eq!(
                capability.source_confidence,
                ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior
            );
        }
    }

    #[test]
    fn declares_openclaw_config_as_writable() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".openclaw");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("openclaw.json"), "{}").unwrap();
        let adapter = OpenClawAdapter::new(tmp.path().to_path_buf());

        let capability = adapter
            .capabilities()
            .into_iter()
            .find(|capability| capability.id == "ordinary-config")
            .expect("expected OpenClaw ordinary config capability");

        assert_eq!(capability.access, ToolCapabilityAccess::Writable);
        assert_eq!(capability.kind, ToolCapabilityKind::OrdinaryConfig);
        assert!(capability.source_path.ends_with("openclaw.json"));
    }

    #[test]
    fn declares_openclaw_skills_as_writable() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = OpenClawAdapter::new(tmp.path().to_path_buf());

        let capability = adapter
            .capabilities()
            .into_iter()
            .find(|capability| capability.id == "dedicated-skills")
            .expect("expected OpenClaw dedicated skills capability");

        assert_eq!(capability.access, ToolCapabilityAccess::Writable);
        assert_eq!(capability.kind, ToolCapabilityKind::Skill);
        assert!(capability.source_path.ends_with(".openclaw/skills"));
    }

    #[test]
    fn runtime_capabilities_are_adapter_owned_and_snapshot_free() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".openclaw");
        std::fs::create_dir_all(config_dir.join("workspace")).unwrap();
        std::fs::create_dir_all(config_dir.join("workspace-mac")).unwrap();
        let adapter = OpenClawAdapter::new(tmp.path().to_path_buf());
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

        let workspace = config_dir.join("workspace").to_string_lossy().to_string();
        let workspace_mac = config_dir
            .join("workspace-mac")
            .to_string_lossy()
            .to_string();
        let agents = config_dir
            .join("workspace")
            .join("AGENTS.md")
            .to_string_lossy()
            .to_string();
        let skills = config_dir.join("skills").to_string_lossy().to_string();
        let config = config_dir
            .join("openclaw.json")
            .to_string_lossy()
            .to_string();

        let rule_projections =
            project_capabilities("openclaw", ToolCapabilityModule::Rules, &capabilities);
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleGlobalTarget
                && projection.allows(&ProjectedAction::Inject)
                && projection.evidence.source_path == agents
        }));
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleNativeFileSource
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == workspace
                && !projection.allows(&ProjectedAction::Inject)
        }));
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleNativeFileSource
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == workspace_mac
        }));

        let skill_projections =
            project_capabilities("openclaw", ToolCapabilityModule::Skills, &capabilities);
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.allows(&ProjectedAction::Install)
                && projection.evidence.source_path == skills
        }));
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
                && projection.allows(&ProjectedAction::Read)
                && !projection.allows(&ProjectedAction::Install)
        }));

        let mcp_projections =
            project_capabilities("openclaw", ToolCapabilityModule::Mcp, &capabilities);
        assert!(mcp_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::McpGlobalConfig
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == format!("{config}#mcp.servers")
        }));

        let config_projections = project_capabilities(
            "openclaw",
            ToolCapabilityModule::OrdinaryConfig,
            &capabilities,
        );
        assert!(config_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::OrdinaryConfigFile
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == config
        }));
    }

    #[test]
    fn hides_missing_workspace_rule_directories() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".openclaw").join("workspace")).unwrap();
        let adapter = OpenClawAdapter::new(tmp.path().to_path_buf());
        let capabilities = adapter.capabilities();

        assert!(capabilities
            .iter()
            .any(|capability| capability.id == "workspace-rules"));
        assert!(!capabilities
            .iter()
            .any(|capability| capability.id == "workspace-mac-rules"));
    }

    #[test]
    fn discovers_only_workspace_prefixed_rule_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".openclaw");
        std::fs::create_dir_all(config_dir.join("workspace/team")).unwrap();
        std::fs::create_dir_all(config_dir.join("workspace-mac")).unwrap();
        std::fs::create_dir_all(config_dir.join("other")).unwrap();
        std::fs::create_dir_all(config_dir.join("skills/demo")).unwrap();
        std::fs::write(config_dir.join("workspace/AGENTS.md"), "agents").unwrap();
        std::fs::write(config_dir.join("workspace/team/rules.md"), "team").unwrap();
        std::fs::write(config_dir.join("workspace-mac/MAC.md"), "mac").unwrap();
        std::fs::write(config_dir.join("other/OTHER.md"), "other").unwrap();
        std::fs::write(config_dir.join("skills/demo/SKILL.md"), "skill").unwrap();
        let adapter = OpenClawAdapter::new(tmp.path().to_path_buf());

        let rules = adapter.read_rules().unwrap();
        let paths: Vec<_> = rules.iter().map(|rule| rule.path.as_str()).collect();

        assert_eq!(rules.len(), 4);
        let agents_rule = rules
            .iter()
            .find(|rule| rule.path.ends_with("workspace/AGENTS.md"))
            .expect("expected workspace AGENTS.md rule");
        assert_eq!(agents_rule.group, "workspace");
        assert_eq!(agents_rule.label, "AGENTS.md");
        assert!(paths
            .iter()
            .any(|path| path.ends_with("workspace/AGENTS.md")));
        assert!(paths
            .iter()
            .any(|path| path.ends_with("workspace/team/rules.md")));
        assert!(paths.iter().any(|path| path.ends_with("workspace/team")));
        assert!(paths
            .iter()
            .any(|path| path.ends_with("workspace-mac/MAC.md")));
        assert!(!paths.iter().any(|path| path.contains("OTHER.md")));
        assert!(!paths.iter().any(|path| path.contains("SKILL.md")));
    }

    #[test]
    fn read_mcp_servers_maps_openclaw_mcp_servers_to_activation_state() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".openclaw");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("openclaw.json"),
            r#"{
              "mcp": {
                "servers": {
                  "enabled-demo": { "command": "node" },
                  "disabled-demo": { "command": "node", "disabled": true }
                }
              }
            }"#,
        )
        .unwrap();
        let adapter = OpenClawAdapter::new(tmp.path().to_path_buf());

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

impl ToolAdapter for OpenClawAdapter {
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
        openclaw_presence(
            Path::new("/Applications/OpenClaw.app").exists(),
            command_exists("openclaw"),
        )
    }

    fn read_rules(&self) -> Result<Vec<RuleSource>, String> {
        discover_rule_sources_for_adapter(self.id(), &self.capabilities())
    }

    fn write_rule(&self, path: &str, content: &str) -> Result<(), String> {
        let target = PathBuf::from(shellexpand::tilde(path).to_string());
        if !adapter_can_write_path(self, ToolCapabilityKind::Rule, &target) {
            return Err("Rule writing is not enabled for this OpenClaw target".to_string());
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(target, content).map_err(|e| e.to_string())
    }

    fn skills_dir(&self) -> Option<PathBuf> {
        Some(self.home.join(".openclaw/skills"))
    }

    fn supports_generic_skills(&self) -> bool {
        true
    }

    fn allow_external_generic_symlink(&self) -> bool {
        true
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        let workspace_dirs = self.workspace_rule_dirs();
        let mut capabilities = vec![];
        for (name, path) in workspace_dirs {
            let display_name = title_case_words(&name.replace('-', " "));
            let source_label = format!("~/.openclaw/{name}");
            capabilities.push(capability(
                &format!("{}-rules", name),
                ToolCapabilityKind::Rule,
                ToolCapabilityScope::Global,
                ToolCapabilityAccess::Writable,
                ToolCapabilityFormat::Directory,
                path.to_string_lossy().to_string(),
                &format!("{display_name} rules"),
                directory_diagnostics(),
                "OpenClaw agent workspace rule directory discovered under ~/.openclaw/workspace*; each agent may have its own workspace with similar rule files.",
                runtime_action_gates(
                    "OpenClaw agent workspace rule directory",
                    &source_label,
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
            ));
        }
        let workspace_dir = capabilities.iter().find_map(|capability| {
            if capability.id == "workspace-rules" {
                Some(PathBuf::from(capability.source_path.clone()))
            } else {
                None
            }
        });
        if let Some(workspace_dir) = workspace_dir {
            capabilities.push(capability(
                "trusted-workspace-agents",
                ToolCapabilityKind::Rule,
                ToolCapabilityScope::Global,
                ToolCapabilityAccess::Writable,
                ToolCapabilityFormat::Markdown,
                workspace_dir
                    .join("AGENTS.md")
                    .to_string_lossy()
                    .to_string(),
                "Workspace AGENTS.md",
                file_diagnostics(),
                "Default OpenClaw agent workspace AGENTS.md instruction file.",
                runtime_action_gates(
                    "OpenClaw default workspace AGENTS.md instruction file",
                    "~/.openclaw/workspace/AGENTS.md",
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
            ));
        }

        let skills_dir = self.home.join(".openclaw/skills");
        capabilities.push(capability(
            "dedicated-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            skills_dir.to_string_lossy().to_string(),
            "Dedicated Skills",
            directory_diagnostics(),
            "OpenClaw discovers managed user-level Skills from ~/.openclaw/skills.",
            runtime_action_gates(
                "OpenClaw managed user Skills directory",
                "~/.openclaw/skills",
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
        ));
        capabilities.push(capability(
            "shared-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Shared,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::SkillDirectory,
            crate::platform::env::generic_skills_dir()
                .to_string_lossy()
                .to_string(),
            "Shared Skills",
            directory_diagnostics(),
            "OpenClaw discovers shared personal Skills from ~/.agents/skills; Skill management owns writes.",
            runtime_action_gates(
                "OpenClaw shared personal Skills source",
                "~/.agents/skills",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                    ToolCapabilityAction::Copy,
                ],
            ),
        ));
        let config = self.home.join(".openclaw/openclaw.json");
        capabilities.push(capability(
            "ordinary-config",
            ToolCapabilityKind::OrdinaryConfig,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            config.to_string_lossy().to_string(),
            "OpenClaw config",
            file_diagnostics(),
            "OpenClaw reads and validates the user-level openclaw.json file; Rules, Skills, and MCP remain separate capability rows.",
            runtime_action_gates(
                "OpenClaw user configuration",
                "~/.openclaw/openclaw.json",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                    ToolCapabilityAction::Edit,
                    ToolCapabilityAction::Save,
                ],
            ),
        ));
        capabilities.push(capability(
            "mcp-config",
            ToolCapabilityKind::Mcp,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            format!("{}#mcp.servers", config.to_string_lossy()),
            "MCP configuration",
            file_diagnostics(),
            "OpenClaw user-scope MCP configuration is stored under mcp.servers in openclaw.json; this does not certify server lifecycle or connectivity.",
            runtime_action_gates(
                "OpenClaw user MCP configuration",
                "~/.openclaw/openclaw.json#mcp.servers",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                    ToolCapabilityAction::Edit,
                    ToolCapabilityAction::Save,
                ],
            ),
        ));
        capabilities
    }
}
