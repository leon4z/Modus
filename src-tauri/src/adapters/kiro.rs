// Purpose: Kiro adapter entry and runtime capability boundary.

use crate::adapters::ToolAdapter;
use crate::platform::tool_adapters::declared::{
    directory_diagnostics, file_diagnostics, DeclaredCapability, DeclaredToolAdapter,
};
use crate::platform::tool_capabilities::{
    runtime_action_gates, ToolCapabilityAccess, ToolCapabilityAction, ToolCapabilityActionEvidence,
    ToolCapabilityFormat, ToolCapabilityKind, ToolCapabilityScope, ToolCapabilitySourceConfidence,
    ToolCapabilitySourceKind, ToolSourceDiagnosticState,
};
use crate::platform::tool_catalog::registry;
use std::path::{Path, PathBuf};

pub fn create(home: &Path) -> Box<dyn ToolAdapter> {
    let definition = registry::definition("kiro").expect("missing Kiro catalog definition");
    Box::new(
        DeclaredToolAdapter::new(
            definition.id,
            definition.name,
            definition.icon,
            definition.config_dir.resolve_path(home),
            capabilities(home),
        )
        .with_skills_dir(home.join(".kiro/skills"))
        .with_detection_paths(&[PathBuf::from("/Applications/Kiro.app")]),
    )
}

fn capabilities(home: &Path) -> Vec<DeclaredCapability> {
    vec![
        capability(
            "global-steering",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Directory,
            home.join(".kiro/steering"),
            "Global steering",
            directory_diagnostics(),
            "Kiro loads user-level global steering Markdown files from ~/.kiro/steering; Modus treats the directory as native rule files, not as a single Global Rule injection target.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "Kiro global steering directory",
                "~/.kiro/steering",
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
            "workspace-steering",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Project,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::Directory,
            PathBuf::from(".kiro/steering"),
            "Workspace steering",
            directory_diagnostics(),
            "Kiro workspace steering is project-scoped and outside default global/user integration authority.",
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
            "Kiro can consume project AGENTS.md as steering input; Modus does not write project sources.",
            ToolCapabilitySourceConfidence::OfficialDocs,
            vec![],
        ),
        capability(
            "agent-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            home.join(".kiro/skills"),
            "Agent Skills",
            directory_diagnostics(),
            "Kiro discovers user-level Skills from ~/.kiro/skills.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "Kiro user Skills directory",
                "~/.kiro/skills",
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
            ToolCapabilityAccess::Unsupported,
            ToolCapabilityFormat::Unknown,
            PathBuf::new(),
            "Shared Skills",
            vec![ToolSourceDiagnosticState::Unsupported],
            "Kiro does not automatically load shared Agent Skills from ~/.agents/skills in the verified installed environment.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            vec![],
        ),
        capability(
            "mcp-config",
            ToolCapabilityKind::Mcp,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            home.join(".kiro/settings/mcp.json"),
            "MCP configuration",
            file_diagnostics(),
            "Kiro Agent user-level MCP configuration is ~/.kiro/settings/mcp.json; the IDE shell --add-mcp path is a separate compatibility surface.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "Kiro Agent MCP configuration",
                "~/.kiro/settings/mcp.json",
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
            ToolCapabilityAccess::Unsupported,
            ToolCapabilityFormat::Unknown,
            PathBuf::new(),
            "Configuration",
            vec![ToolSourceDiagnosticState::Unsupported],
            "No single general user-level Kiro ordinary configuration file is verified for the installed Kiro Agent environment.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            vec![],
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
    action_evidence: Vec<ToolCapabilityActionEvidence>,
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
    fn presence_is_currently_app_only() {
        let adapter = create(Path::new("/Users/example"));
        let presence = adapter.presence();

        assert!(!presence.cli_detected);
        assert_ne!(presence.label, "CLI");
        assert_ne!(presence.label, "APP+CLI");
    }

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
            project_capabilities("kiro", ToolCapabilityModule::Rules, &capabilities);
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleNativeFileSource
                && projection.allows(&ProjectedAction::Create)
                && projection.allows(&ProjectedAction::Save)
                && !projection.allows(&ProjectedAction::Inject)
                && projection.evidence.source_path == "/Users/example/.kiro/steering"
        }));
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleProjectSource
                && projection.exclusion_reason == Some(ToolCapabilityExclusionReason::ProjectScoped)
                && projection.evidence.source_path == ".kiro/steering"
        }));

        let skill_projections =
            project_capabilities("kiro", ToolCapabilityModule::Skills, &capabilities);
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.allows(&ProjectedAction::Install)
                && projection.evidence.source_path == "/Users/example/.kiro/skills"
        }));
        let shared_skills = skill_projections
            .iter()
            .find(|projection| projection.evidence.id == "shared-skills")
            .expect("Kiro shared Skills evidence");
        assert_eq!(
            shared_skills.source_role,
            ToolCapabilitySourceRole::SkillNonActionableEvidence
        );
        assert_eq!(
            shared_skills.exclusion_reason,
            Some(ToolCapabilityExclusionReason::Unsupported)
        );

        let mcp_projections =
            project_capabilities("kiro", ToolCapabilityModule::Mcp, &capabilities);
        assert!(mcp_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::McpGlobalConfig
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.kiro/settings/mcp.json"
        }));

        let config_projections =
            project_capabilities("kiro", ToolCapabilityModule::OrdinaryConfig, &capabilities);
        let ordinary_config = config_projections
            .iter()
            .find(|projection| projection.evidence.id == "ordinary-config")
            .expect("Kiro ordinary config boundary");
        assert_eq!(
            ordinary_config.source_role,
            ToolCapabilitySourceRole::OrdinaryConfigNonActionableEvidence
        );
        assert_eq!(
            ordinary_config.exclusion_reason,
            Some(ToolCapabilityExclusionReason::Unsupported)
        );
        assert!(!ordinary_config.allows(&ProjectedAction::Save));
    }

    #[test]
    fn rule_discovery_reads_global_steering_but_not_skills() {
        let tmp = tempfile::tempdir().unwrap();
        let steering = tmp.path().join(".kiro/steering");
        let skills = tmp.path().join(".kiro/skills/demo");
        std::fs::create_dir_all(&steering).unwrap();
        std::fs::create_dir_all(&skills).unwrap();
        std::fs::write(steering.join("always.md"), "steering").unwrap();
        std::fs::write(skills.join("SKILL.md"), "skill").unwrap();
        let adapter = create(tmp.path());

        let rules = adapter.read_rules().unwrap();

        assert!(rules.iter().any(|rule| rule.label == "always.md"));
        assert!(!rules.iter().any(|rule| rule.path.contains("SKILL.md")));
    }

    #[test]
    fn reads_mcp_servers_from_agent_mcp_config() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join(".kiro/settings/mcp.json");
        std::fs::create_dir_all(config.parent().unwrap()).unwrap();
        std::fs::write(
            &config,
            r#"{"mcpServers":{"demo":{"command":"node","args":["server.js"],"env":{"API_KEY":"secret"}}}}"#,
        )
        .unwrap();
        let adapter = create(tmp.path());

        let servers = adapter.read_mcp_servers().unwrap();

        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "demo");
        assert_eq!(servers[0].command.as_deref(), Some("node"));
        assert_eq!(servers[0].args, vec!["server.js".to_string()]);
        assert_eq!(servers[0].env_keys, vec!["API_KEY".to_string()]);
        assert!(!format!("{servers:?}").contains("secret"));
    }
}
