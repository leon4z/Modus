// Purpose: Windsurf adapter entry and runtime capability boundary.

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
    let definition = registry::definition("windsurf").expect("missing Windsurf catalog definition");
    Box::new(
        DeclaredToolAdapter::new(
            definition.id,
            definition.name,
            definition.icon,
            definition.config_dir.resolve_path(home),
            capabilities(home),
        )
        .with_skills_dir(home.join(".codeium/windsurf/skills"))
        .with_generic_skills(true)
        .with_detection_paths(&[PathBuf::from("/Applications/Windsurf.app")]),
    )
}

fn capabilities(home: &Path) -> Vec<DeclaredCapability> {
    vec![
        capability(
            "global-rules",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::Markdown,
            home.join(".codeium/windsurf/memories/global_rules.md"),
            "Global rules",
            file_diagnostics(),
            "Windsurf loads the user-level global rules file as native read-only rule evidence; writable injection was not verified.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "Windsurf global rules",
                "~/.codeium/windsurf/memories/global_rules.md",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                ],
            ),
        ),
        capability(
            "skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            home.join(".codeium/windsurf/skills"),
            "Skills",
            directory_diagnostics(),
            "Windsurf discovers user-level Skills from ~/.codeium/windsurf/skills. Workspace .windsurf/skills remains project-scoped and outside Community global scope.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "Windsurf global user Skills",
                "~/.codeium/windsurf/skills",
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
            "Windsurf can consume shared Agent Skills from ~/.agents/skills; Skill management owns shared writes.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "Windsurf shared Skills directory",
                "~/.agents/skills",
                &[
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                ],
            ),
        ),
        capability(
            "mcp-config",
            ToolCapabilityKind::Mcp,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            home.join(".codeium/windsurf/mcp_config.json"),
            "MCP configuration",
            file_diagnostics(),
            "Windsurf user-level MCP configuration is ~/.codeium/windsurf/mcp_config.json. Modus edits single entry fragments only and does not manage MCP lifecycle.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "Windsurf MCP configuration",
                "~/.codeium/windsurf/mcp_config.json",
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
            "No general user-level Windsurf settings file comparable to Claude settings.json or Codex config.toml is verified.",
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
            project_capabilities("windsurf", ToolCapabilityModule::Rules, &capabilities);
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleNativeFileSource
                && projection.allows(&ProjectedAction::Read)
                && !projection.allows(&ProjectedAction::Create)
                && !projection.allows(&ProjectedAction::Inject)
                && projection.evidence.source_path
                    == "/Users/example/.codeium/windsurf/memories/global_rules.md"
        }));

        let skill_projections =
            project_capabilities("windsurf", ToolCapabilityModule::Skills, &capabilities);
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.allows(&ProjectedAction::Install)
                && projection.evidence.source_path == "/Users/example/.codeium/windsurf/skills"
        }));
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
                && projection.allows(&ProjectedAction::View)
        }));

        let mcp_projections =
            project_capabilities("windsurf", ToolCapabilityModule::Mcp, &capabilities);
        assert!(mcp_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::McpGlobalConfig
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path
                    == "/Users/example/.codeium/windsurf/mcp_config.json"
        }));

        let config_projections = project_capabilities(
            "windsurf",
            ToolCapabilityModule::OrdinaryConfig,
            &capabilities,
        );
        let ordinary_config = config_projections
            .iter()
            .find(|projection| projection.evidence.id == "ordinary-config")
            .expect("Windsurf ordinary config boundary");
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
}
