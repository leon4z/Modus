// Purpose: GitHub Copilot adapter entry and runtime capability boundary.

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
        registry::definition("github-copilot").expect("missing GitHub Copilot catalog definition");
    Box::new(
        DeclaredToolAdapter::new(
            definition.id,
            definition.name,
            definition.icon,
            definition.config_dir.resolve_path(home),
            capabilities(home),
        )
        .with_skills_dir(home.join(".copilot/skills"))
        .with_generic_skills(true)
        .with_detection_commands(&["copilot"]),
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
            home.join(".copilot/copilot-instructions.md"),
            "Copilot instructions",
            file_diagnostics(),
            "GitHub Copilot CLI consumes the user-level ~/.copilot/copilot-instructions.md instruction file.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "GitHub Copilot global instructions",
                "~/.copilot/copilot-instructions.md",
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
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::InstructionsMarkdown,
            home.join(".copilot/instructions"),
            "Instruction files",
            directory_diagnostics(),
            "GitHub Copilot CLI consumes user-level .instructions.md files from ~/.copilot/instructions.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "GitHub Copilot instruction directory",
                "~/.copilot/instructions",
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
            "repository-instructions",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Project,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::Markdown,
            PathBuf::from(".github/copilot-instructions.md"),
            "Repository instructions",
            file_diagnostics(),
            "GitHub Copilot repository instructions are official project-scoped context and are outside the current global/user management scope.",
            ToolCapabilitySourceConfidence::OfficialDocs,
            vec![],
        ),
        capability(
            "path-instructions",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Project,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::Directory,
            PathBuf::from(".github/instructions"),
            "Path instructions",
            directory_diagnostics(),
            "GitHub Copilot path-specific repository instructions are project-scoped context and are outside the current global/user management scope.",
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
            "GitHub Copilot can consume project AGENTS.md; Modus does not write project sources.",
            ToolCapabilitySourceConfidence::OfficialDocs,
            vec![],
        ),
        capability(
            "prompt-files",
            ToolCapabilityKind::ProjectAsset,
            ToolCapabilityScope::Project,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::Directory,
            PathBuf::from(".github/prompts"),
            "Prompt files",
            directory_diagnostics(),
            "GitHub Copilot prompt files are official project assets, not Skills.",
            ToolCapabilitySourceConfidence::OfficialDocs,
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
            "GitHub Copilot CLI discovers shared Agent Skills from ~/.agents/skills; Skill management owns shared writes.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "GitHub Copilot shared Skills directory",
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
            home.join(".copilot/skills"),
            "Agent Skills",
            directory_diagnostics(),
            "GitHub Copilot CLI discovers Copilot-specific user Skills from ~/.copilot/skills.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "GitHub Copilot user Skills directory",
                "~/.copilot/skills",
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
            "mcp-config",
            ToolCapabilityKind::Mcp,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            home.join(".copilot/mcp-config.json"),
            "MCP configuration",
            file_diagnostics(),
            "GitHub Copilot CLI user-level MCP configuration is ~/.copilot/mcp-config.json.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "GitHub Copilot MCP configuration",
                "~/.copilot/mcp-config.json",
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
            home.join(".copilot/settings.json"),
            "Settings",
            file_diagnostics(),
            "GitHub Copilot CLI reads user-editable settings from ~/.copilot/settings.json; config.json is tool-managed state.",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            runtime_action_gates(
                "GitHub Copilot settings",
                "~/.copilot/settings.json",
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
        project_capabilities, ToolCapabilityAction as ProjectedAction, ToolCapabilityModule,
        ToolCapabilitySourceRole,
    };

    #[test]
    fn copilot_rule_discovery_reads_global_file_and_instruction_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".copilot");
        std::fs::create_dir_all(config_dir.join("instructions/team")).unwrap();
        std::fs::write(config_dir.join("copilot-instructions.md"), "global").unwrap();
        std::fs::write(
            config_dir.join("instructions/team/frontend.instructions.md"),
            "team",
        )
        .unwrap();
        std::fs::write(config_dir.join("instructions/README.md"), "ignored").unwrap();
        std::fs::write(config_dir.join("instructions/ignored.txt"), "ignored").unwrap();
        std::fs::create_dir_all(config_dir.join("skills/demo")).unwrap();
        std::fs::write(config_dir.join("skills/demo/SKILL.md"), "skill").unwrap();
        let adapter = create(tmp.path());

        let rules = adapter.read_rules().unwrap();
        let labels: Vec<_> = rules.iter().map(|rule| rule.label.as_str()).collect();

        assert_eq!(
            labels,
            vec!["Copilot instructions", "frontend.instructions.md", "team"]
        );
        assert!(rules.iter().any(|rule| rule.group == "team"));
        assert!(!rules.iter().any(|rule| rule.path.contains("README.md")));
        assert!(!rules.iter().any(|rule| rule.path.contains("ignored.txt")));
        assert!(!rules.iter().any(|rule| rule.path.contains("SKILL.md")));
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
            project_capabilities("github-copilot", ToolCapabilityModule::Rules, &capabilities);
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleGlobalTarget
                && projection.allows(&ProjectedAction::Inject)
                && projection.evidence.source_path
                    == "/Users/example/.copilot/copilot-instructions.md"
        }));
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleNativeFileSource
                && projection.allows(&ProjectedAction::Create)
                && !projection.allows(&ProjectedAction::Inject)
                && projection.evidence.source_path == "/Users/example/.copilot/instructions"
        }));
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleProjectSource
                && projection.evidence.source_path == ".github/copilot-instructions.md"
        }));

        let skill_projections = project_capabilities(
            "github-copilot",
            ToolCapabilityModule::Skills,
            &capabilities,
        );
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.allows(&ProjectedAction::Install)
                && projection.evidence.source_path == "/Users/example/.copilot/skills"
        }));
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
                && projection.allows(&ProjectedAction::View)
        }));

        let mcp_projections =
            project_capabilities("github-copilot", ToolCapabilityModule::Mcp, &capabilities);
        assert!(mcp_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::McpGlobalConfig
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.copilot/mcp-config.json"
        }));

        let settings_projections = project_capabilities(
            "github-copilot",
            ToolCapabilityModule::OrdinaryConfig,
            &capabilities,
        );
        assert!(settings_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::OrdinaryConfigFile
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.copilot/settings.json"
        }));
    }
}
