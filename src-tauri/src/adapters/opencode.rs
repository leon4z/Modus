// Purpose: OpenCode adapter entry and runtime capability boundary.

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
    let definition = registry::definition("opencode").expect("missing OpenCode catalog definition");
    Box::new(
        DeclaredToolAdapter::new(
            definition.id,
            definition.name,
            definition.icon,
            definition.config_dir.resolve_path(home),
            capabilities(home),
        )
        .with_skills_dir(home.join(".config/opencode/skills"))
        .with_generic_skills(true)
        .with_detection_paths(&[PathBuf::from("/Applications/OpenCode.app")])
        .with_detection_commands(&["opencode"]),
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
            home.join(".config/opencode/AGENTS.md"),
            "AGENTS.md",
            file_diagnostics(),
            "OpenCode global AGENTS.md was verified locally through opencode run.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "OpenCode global AGENTS.md",
                "~/.config/opencode/AGENTS.md",
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
            "instructions-config",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::Jsonc,
            home.join(".config/opencode/opencode.jsonc#instructions"),
            "Instructions",
            file_diagnostics(),
            "OpenCode instructions can reference custom files and globs; Modus treats the config fragment as read-only source evidence, not a fixed rule directory.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
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
            "OpenCode supports project AGENTS.md as rule context.",
            ToolCapabilitySourceConfidence::OfficialDocs,
            vec![],
        ),
        capability(
            "agent-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            home.join(".config/opencode/skills"),
            "Agent Skills",
            directory_diagnostics(),
            "OpenCode global Skills under ~/.config/opencode/skills were verified locally through debug skill and opencode run.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "OpenCode global Skills directory",
                "~/.config/opencode/skills",
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
            "OpenCode can consume shared Agent Skills from ~/.agents/skills; Skill management owns shared writes.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "OpenCode shared Skills directory",
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
            ToolCapabilityFormat::Jsonc,
            home.join(".config/opencode/opencode.jsonc#mcp"),
            "MCP configuration",
            file_diagnostics(),
            "OpenCode MCP config fragment in the user config was verified locally with a no-secret stdio server probe.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "OpenCode MCP configuration",
                "~/.config/opencode/opencode.jsonc#mcp",
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
            ToolCapabilityFormat::Jsonc,
            home.join(".config/opencode/opencode.jsonc"),
            "Config",
            file_diagnostics(),
            "OpenCode user configuration file; local install uses opencode.jsonc and probes confirmed runtime behavior.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "OpenCode user configuration",
                "~/.config/opencode/opencode.jsonc",
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
            project_capabilities("opencode", ToolCapabilityModule::Rules, &capabilities);
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleGlobalTarget
                && projection.allows(&ProjectedAction::Inject)
                && projection.evidence.source_path == "/Users/example/.config/opencode/AGENTS.md"
        }));

        let instructions_projection = rule_projections
            .iter()
            .find(|projection| projection.evidence.id == "instructions-config")
            .expect("OpenCode instructions projection");
        assert_eq!(
            instructions_projection.source_role,
            ToolCapabilitySourceRole::RuleNonActionableEvidence
        );
        assert_eq!(
            instructions_projection.exclusion_reason,
            Some(ToolCapabilityExclusionReason::UnsupportedFormat)
        );
        for action in [
            ProjectedAction::Create,
            ProjectedAction::Edit,
            ProjectedAction::Save,
            ProjectedAction::Inject,
        ] {
            assert!(!instructions_projection.allows(&action));
        }

        let skill_projections =
            project_capabilities("opencode", ToolCapabilityModule::Skills, &capabilities);
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.allows(&ProjectedAction::Install)
        }));
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
                && projection.allows(&ProjectedAction::View)
        }));

        let mcp_projections =
            project_capabilities("opencode", ToolCapabilityModule::Mcp, &capabilities);
        assert!(mcp_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::McpGlobalConfig
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path
                    == "/Users/example/.config/opencode/opencode.jsonc#mcp"
        }));

        let settings_projections = project_capabilities(
            "opencode",
            ToolCapabilityModule::OrdinaryConfig,
            &capabilities,
        );
        assert!(settings_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::OrdinaryConfigFile
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path
                    == "/Users/example/.config/opencode/opencode.jsonc"
        }));
    }
}
