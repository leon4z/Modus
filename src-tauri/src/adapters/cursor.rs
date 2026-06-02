// Purpose: Cursor adapter for verified global Skills, MCP, and CLI config boundaries.

use crate::adapters::ToolAdapter;
use crate::platform::tool_adapters::declared::{
    directory_diagnostics, file_diagnostics, DeclaredCapability, DeclaredToolAdapter,
};
use crate::platform::tool_capabilities::{
    runtime_action_gates, ToolCapabilityAccess, ToolCapabilityAction, ToolCapabilityActionEvidence,
    ToolCapabilityFormat, ToolCapabilityKind, ToolCapabilityScope, ToolCapabilitySourceConfidence,
    ToolCapabilitySourceKind, ToolSourceDiagnosticState,
};
use std::path::{Path, PathBuf};

pub fn create(home: &Path) -> Box<dyn ToolAdapter> {
    Box::new(
        DeclaredToolAdapter::new(
            "cursor",
            "Cursor",
            "cursor",
            home.join(".cursor"),
            capabilities(home),
        )
        .with_skills_dir(home.join(".cursor/skills"))
        .with_generic_skills(true)
        .with_detection_paths(&[PathBuf::from("/Applications/Cursor.app")]),
    )
}

fn capabilities(home: &Path) -> Vec<DeclaredCapability> {
    vec![
        capability(
            "dedicated-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            home.join(".cursor/skills"),
            "Dedicated Skills",
            directory_diagnostics(),
            "Cursor Agent discovers Cursor-specific user Skills from ~/.cursor/skills.",
            runtime_action_gates(
                "Cursor Agent user Skills directory",
                "~/.cursor/skills",
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
            "Cursor Agent discovers shared personal Skills from ~/.agents/skills; Skill management owns writes.",
            runtime_action_gates(
                "Cursor Agent shared personal Skills source",
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
            "mcp-config",
            ToolCapabilityKind::Mcp,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            home.join(".cursor/mcp.json"),
            "MCP configuration",
            file_diagnostics(),
            "Cursor Agent user-scope MCP configuration is stored in ~/.cursor/mcp.json; this does not certify server lifecycle or connectivity.",
            runtime_action_gates(
                "Cursor Agent user MCP configuration",
                "~/.cursor/mcp.json",
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
            ToolCapabilityFormat::Json,
            home.join(".cursor/cli-config.json"),
            "Agent CLI config",
            file_diagnostics(),
            "Cursor Agent CLI configuration lives in ~/.cursor/cli-config.json. Cursor desktop GUI settings remain outside this ordinary config boundary.",
            runtime_action_gates(
                "Cursor Agent CLI configuration",
                "~/.cursor/cli-config.json",
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
        source_confidence: ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
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
        project_capabilities, ToolCapabilityAction as ProjectedAction, ToolCapabilityModule,
        ToolCapabilitySourceRole,
    };

    #[test]
    fn runtime_capabilities_are_adapter_owned_and_snapshot_free() {
        let adapter = create(Path::new("/Users/example"));
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
        assert!(!capabilities.iter().any(|capability| {
            capability.kind == ToolCapabilityKind::Rule
                || capability.source_path.contains(".cursor/rules")
                || capability.source_path.contains(".cursorrules")
                || capability.source_path.contains("AGENTS.md")
        }));

        let skill_projections =
            project_capabilities("cursor", ToolCapabilityModule::Skills, &capabilities);
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.allows(&ProjectedAction::Install)
                && projection.evidence.source_path == "/Users/example/.cursor/skills"
        }));
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
                && projection.allows(&ProjectedAction::Read)
                && !projection.allows(&ProjectedAction::Install)
        }));

        let mcp_projections =
            project_capabilities("cursor", ToolCapabilityModule::Mcp, &capabilities);
        assert!(mcp_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::McpGlobalConfig
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.cursor/mcp.json"
        }));

        let config_projections = project_capabilities(
            "cursor",
            ToolCapabilityModule::OrdinaryConfig,
            &capabilities,
        );
        assert!(config_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::OrdinaryConfigFile
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.cursor/cli-config.json"
        }));
    }

    #[test]
    fn read_mcp_servers_maps_cursor_user_mcp_servers() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".cursor");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("mcp.json"),
            r#"{
              "mcpServers": {
                "enabled-demo": { "command": "node" },
                "disabled-demo": { "command": "node", "disabled": true }
              }
            }"#,
        )
        .unwrap();
        let adapter = create(tmp.path());

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
