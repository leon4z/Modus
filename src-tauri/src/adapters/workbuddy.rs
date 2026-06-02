// Purpose: WorkBuddy desktop adapter entry and runtime capability boundary.

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
        registry::definition("workbuddy").expect("missing WorkBuddy catalog definition");
    Box::new(
        DeclaredToolAdapter::new(
            definition.id,
            definition.name,
            definition.icon,
            definition.config_dir.resolve_path(home),
            capabilities(home),
        )
        .with_skills_dir(home.join(".workbuddy/skills"))
        .with_detection_paths(&[PathBuf::from("/Applications/WorkBuddy.app")]),
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
            home.join(".workbuddy/SOUL.md"),
            "SOUL.md",
            file_diagnostics(),
            "WorkBuddy desktop loads ~/.workbuddy/SOUL.md as the global rule file for the certified desktop scope.",
            runtime_action_gates(
                "WorkBuddy global SOUL.md rule",
                "~/.workbuddy/SOUL.md",
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
            "identity-rule",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::Markdown,
            home.join(".workbuddy/IDENTITY.md"),
            "IDENTITY.md",
            file_diagnostics(),
            "WorkBuddy desktop recognizes IDENTITY.md as one of its fixed user-level rule/persona files.",
            runtime_action_gates(
                "WorkBuddy IDENTITY.md rule file",
                "~/.workbuddy/IDENTITY.md",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                ],
            ),
        ),
        capability(
            "user-rule",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::Markdown,
            home.join(".workbuddy/USER.md"),
            "USER.md",
            file_diagnostics(),
            "WorkBuddy desktop recognizes USER.md as one of its fixed user-level rule/persona files.",
            runtime_action_gates(
                "WorkBuddy USER.md rule file",
                "~/.workbuddy/USER.md",
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
            home.join(".workbuddy/skills"),
            "Skills",
            directory_diagnostics(),
            "WorkBuddy desktop Skills page and local state certify ~/.workbuddy/skills as the tool-specific Skill directory.",
            runtime_action_gates(
                "WorkBuddy Skills directory",
                "~/.workbuddy/skills",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                    ToolCapabilityAction::Install,
                    ToolCapabilityAction::Copy,
                    ToolCapabilityAction::Uninstall,
                    ToolCapabilityAction::Delete,
                ],
            ),
        ),
        capability(
            "mcp-config",
            ToolCapabilityKind::Mcp,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            home.join(".workbuddy/mcp.json#mcpServers"),
            "Connector MCP configuration",
            file_diagnostics(),
            "WorkBuddy connector service watches mcp.json; generated .mcp.json proxy output is supporting state, not the editable source.",
            runtime_action_gates(
                "WorkBuddy MCP configuration",
                "~/.workbuddy/mcp.json",
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
            "ordinary-settings",
            ToolCapabilityKind::OrdinaryConfig,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            home.join(".workbuddy/settings.json"),
            "Settings",
            file_diagnostics(),
            "WorkBuddy desktop ordinary settings file; Skills, MCP, memory, connectors, marketplace data, and runtime state stay with their owning surfaces.",
            runtime_action_gates(
                "WorkBuddy settings",
                "~/.workbuddy/settings.json",
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
            .filter(|capability| !capability.action_evidence.is_empty())
            .all(|capability| capability
                .action_evidence
                .iter()
                .all(|evidence| evidence.version.is_none() && evidence.verified_at.is_none())));

        let rule_projections =
            project_capabilities("workbuddy", ToolCapabilityModule::Rules, &capabilities);
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleGlobalTarget
                && projection.allows(&ProjectedAction::Inject)
                && projection.evidence.source_path == "/Users/example/.workbuddy/SOUL.md"
        }));

        let skill_projections =
            project_capabilities("workbuddy", ToolCapabilityModule::Skills, &capabilities);
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.allows(&ProjectedAction::Install)
        }));
        assert!(!skill_projections.iter().any(
            |projection| projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
        ));

        let mcp_projections =
            project_capabilities("workbuddy", ToolCapabilityModule::Mcp, &capabilities);
        assert!(mcp_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::McpGlobalConfig
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path
                    == "/Users/example/.workbuddy/mcp.json#mcpServers"
        }));

        let settings_projections = project_capabilities(
            "workbuddy",
            ToolCapabilityModule::OrdinaryConfig,
            &capabilities,
        );
        assert!(settings_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::OrdinaryConfigFile
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.workbuddy/settings.json"
        }));
    }
}
