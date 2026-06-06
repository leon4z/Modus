// Purpose: Hermes Agent adapter entry and runtime capability boundary.

use crate::adapters::ToolAdapter;
use crate::platform::tool_adapters::declared::{
    directory_diagnostics, file_diagnostics, DeclaredCapability, DeclaredToolAdapter,
};
use crate::platform::tool_capabilities::{
    runtime_action_gates, ToolCapabilityAccess, ToolCapabilityAction, ToolCapabilityFormat,
    ToolCapabilityKind, ToolCapabilityScope, ToolCapabilitySourceConfidence,
    ToolCapabilitySourceKind, ToolSourceDiagnosticState,
};
use crate::platform::tool_catalog::registry;
use std::path::{Path, PathBuf};

pub fn create(home: &Path) -> Box<dyn ToolAdapter> {
    let definition =
        registry::definition("hermes-agent").expect("missing Hermes Agent catalog definition");
    Box::new(
        DeclaredToolAdapter::new(
            definition.id,
            definition.name,
            definition.icon,
            definition.config_dir.resolve_path(home),
            capabilities(home),
        )
        .with_skills_dir(home.join(".hermes/skills"))
        .with_detection_paths(&[PathBuf::from("/Applications/Hermes.app")])
        .with_detection_commands(&["hermes"]),
    )
}

fn capabilities(home: &Path) -> Vec<DeclaredCapability> {
    vec![
        capability(
            "global-rule",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Markdown,
            home.join(".hermes/SOUL.md"),
            "SOUL.md",
            file_diagnostics(),
            "Hermes Agent loads ~/.hermes/SOUL.md as user-level global identity and instruction context.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "Hermes Agent SOUL.md",
                "~/.hermes/SOUL.md",
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
            "No global/user-level Hermes Rule Directory is certified; project context files stay project-scoped and out of default integration authority.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            vec![],
        ),
        capability(
            "project-hermes-md",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Project,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::Markdown,
            PathBuf::from("HERMES.md"),
            "HERMES.md",
            file_diagnostics(),
            "Hermes Agent can read project HERMES.md context; Modus does not use project files as global integration authority.",
            ToolCapabilitySourceConfidence::OfficialDocs,
            vec![],
        ),
        capability(
            "project-agents-md",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Project,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::Markdown,
            PathBuf::from("AGENTS.md"),
            "AGENTS.md",
            file_diagnostics(),
            "Hermes Agent can read project AGENTS.md context; Modus does not write project sources.",
            ToolCapabilitySourceConfidence::OfficialDocs,
            vec![],
        ),
        capability(
            "agent-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            home.join(".hermes/skills"),
            "Skills",
            directory_diagnostics(),
            "Hermes Agent discovers and uses user-level Skills from ~/.hermes/skills.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "Hermes Agent Skills directory",
                "~/.hermes/skills",
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
            ToolCapabilityAccess::Unknown,
            ToolCapabilityFormat::SkillDirectory,
            crate::platform::env::generic_skills_dir(),
            "Shared Skills",
            directory_diagnostics(),
            "Hermes Agent can consume shared personal Skills from ~/.agents/skills when that directory is listed in skills.external_dirs; Skill management owns shared writes.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            vec![],
        ),
        capability(
            "mcp-config",
            ToolCapabilityKind::Mcp,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Yaml,
            home.join(".hermes/config.yaml#mcp_servers"),
            "MCP configuration",
            file_diagnostics(),
            "Hermes Agent reads and manages MCP servers from ~/.hermes/config.yaml#mcp_servers.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "Hermes Agent MCP configuration",
                "~/.hermes/config.yaml#mcp_servers",
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
            "ordinary-config",
            ToolCapabilityKind::OrdinaryConfig,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Yaml,
            home.join(".hermes/config.yaml"),
            "Config",
            file_diagnostics(),
            "Hermes Agent primary user configuration lives in ~/.hermes/config.yaml.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "Hermes Agent configuration",
                "~/.hermes/config.yaml",
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
    action_evidence: Vec<crate::platform::tool_capabilities::ToolCapabilityActionEvidence>,
) -> DeclaredCapability {
    DeclaredCapability {
        id,
        kind,
        scope,
        access,
        format,
        source_path: source_path.to_string_lossy().to_string(),
        label,
        diagnostics,
        source_confidence,
        notes,
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
        project_capabilities, ToolCapabilityAction as ProjectedAction,
        ToolCapabilityExclusionReason, ToolCapabilityModule, ToolCapabilitySourceRole,
    };

    #[test]
    fn runtime_capabilities_are_adapter_owned_and_snapshot_free() {
        let adapter = create(Path::new("/Users/example"));
        let capabilities = adapter.capabilities();

        assert!(capabilities
            .iter()
            .filter(|capability| !capability.action_evidence.is_empty())
            .all(|capability| capability
                .action_evidence
                .iter()
                .all(|evidence| evidence.version.is_none() && evidence.verified_at.is_none())));

        let rule_projections =
            project_capabilities("hermes-agent", ToolCapabilityModule::Rules, &capabilities);
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleGlobalTarget
                && projection.allows(&ProjectedAction::Inject)
                && projection.evidence.source_path == "/Users/example/.hermes/SOUL.md"
        }));
        let rule_directory = rule_projections
            .iter()
            .find(|projection| projection.evidence.id == "rule-directory")
            .expect("Hermes Rule Directory evidence");
        assert_eq!(
            rule_directory.source_role,
            ToolCapabilitySourceRole::RuleNonActionableEvidence
        );
        assert_eq!(
            rule_directory.exclusion_reason,
            Some(ToolCapabilityExclusionReason::Unsupported)
        );
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleProjectSource
                && projection.exclusion_reason == Some(ToolCapabilityExclusionReason::ProjectScoped)
                && projection.evidence.source_path == "HERMES.md"
        }));

        let skill_projections =
            project_capabilities("hermes-agent", ToolCapabilityModule::Skills, &capabilities);
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.allows(&ProjectedAction::Install)
                && projection.evidence.source_path == "/Users/example/.hermes/skills"
        }));
        let shared_skills = skill_projections
            .iter()
            .find(|projection| projection.evidence.id == "shared-skills")
            .expect("Hermes shared Skills evidence");
        assert_eq!(
            shared_skills.source_role,
            ToolCapabilitySourceRole::SkillNonActionableEvidence
        );
        assert_eq!(
            shared_skills.exclusion_reason,
            Some(ToolCapabilityExclusionReason::Unknown)
        );
        assert!(shared_skills.allows(&ProjectedAction::Diagnose));
        assert!(!shared_skills.allows(&ProjectedAction::View));

        let mcp_projections =
            project_capabilities("hermes-agent", ToolCapabilityModule::Mcp, &capabilities);
        assert!(mcp_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::McpGlobalConfig
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path
                    == "/Users/example/.hermes/config.yaml#mcp_servers"
        }));

        let config_projections = project_capabilities(
            "hermes-agent",
            ToolCapabilityModule::OrdinaryConfig,
            &capabilities,
        );
        assert!(config_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::OrdinaryConfigFile
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.hermes/config.yaml"
        }));
    }

    #[test]
    fn reads_yaml_mcp_servers_from_config_fragment() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".hermes");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.yaml"),
            r#"
mcp_servers:
  enabled-demo:
    command: node
    args:
      - server.js
  disabled-demo:
    command: python
    enabled: false
"#,
        )
        .unwrap();
        let adapter = create(tmp.path());

        let servers = adapter.read_mcp_servers().unwrap();
        let enabled = servers
            .iter()
            .find(|server| server.name == "enabled-demo")
            .expect("enabled server");
        let disabled = servers
            .iter()
            .find(|server| server.name == "disabled-demo")
            .expect("disabled server");

        assert_eq!(enabled.command.as_deref(), Some("node"));
        assert_eq!(enabled.args, vec!["server.js".to_string()]);
        assert!(enabled.enabled);
        assert!(!disabled.enabled);
    }
}
