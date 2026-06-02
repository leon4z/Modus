// Purpose: CodeBuddy adapter entry and runtime capability boundary.

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
        registry::definition("codebuddy").expect("missing CodeBuddy catalog definition");
    Box::new(
        DeclaredToolAdapter::new(
            definition.id,
            definition.name,
            definition.icon,
            definition.config_dir.resolve_path(home),
            capabilities(home),
        )
        .with_skills_dir(home.join(".codebuddy/skills"))
        .with_generic_skills(true)
        .with_detection_commands(&["codebuddy", "cbc"]),
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
            home.join(".codebuddy/CODEBUDDY.md"),
            "CODEBUDDY.md",
            file_diagnostics(),
            "CodeBuddy Code loads the user-level CODEBUDDY.md memory file as global instruction context.",
            runtime_action_gates(
                "CodeBuddy user memory file",
                "~/.codebuddy/CODEBUDDY.md",
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
            "user-rules-directory",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Mdc,
            home.join(".codebuddy/rules"),
            "User Rules",
            directory_diagnostics(),
            "CodeBuddy GUI creates user-level .mdc rule files under ~/.codebuddy/rules, and local runtime verification confirmed consumption.",
            runtime_action_gates(
                "CodeBuddy user Rules directory",
                "~/.codebuddy/rules",
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
            "agent-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            home.join(".codebuddy/skills"),
            "Skills",
            directory_diagnostics(),
            "CodeBuddy GUI discovers user-level Skills under ~/.codebuddy/skills.",
            runtime_action_gates(
                "CodeBuddy user Skills directory",
                "~/.codebuddy/skills",
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
            "CodeBuddy GUI discovers shared Agent Skills from ~/.agents/skills; Skill management owns shared writes.",
            runtime_action_gates(
                "CodeBuddy shared Skills directory",
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
            home.join(".codebuddy/mcp.json"),
            "MCP configuration",
            file_diagnostics(),
            "CodeBuddy GUI watches the user-level MCP configuration at ~/.codebuddy/mcp.json.",
            runtime_action_gates(
                "CodeBuddy MCP configuration",
                "~/.codebuddy/mcp.json",
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
            home.join(".codebuddy/settings.json"),
            "Settings",
            file_diagnostics(),
            "CodeBuddy GUI reloads the user settings file at ~/.codebuddy/settings.json.",
            runtime_action_gates(
                "CodeBuddy settings",
                "~/.codebuddy/settings.json",
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
    fn codebuddy_rule_discovery_uses_memory_file_and_mdc_rules_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join(".codebuddy");
        std::fs::create_dir_all(config_dir.join("rules/team")).unwrap();
        std::fs::create_dir_all(config_dir.join("skills/demo")).unwrap();
        std::fs::write(config_dir.join("CODEBUDDY.md"), "memory").unwrap();
        std::fs::write(config_dir.join("rules/team/frontend.mdc"), "team").unwrap();
        std::fs::write(config_dir.join("rules/team/ignored.md"), "ignored").unwrap();
        std::fs::write(config_dir.join("skills/demo/SKILL.md"), "skill").unwrap();
        let adapter = create(tmp.path());

        let rules = adapter.read_rules().unwrap();
        let labels: Vec<_> = rules.iter().map(|rule| rule.label.as_str()).collect();

        assert_eq!(labels, vec!["CODEBUDDY.md", "frontend.mdc", "team"]);
        assert!(rules.iter().any(|rule| rule.group == "team"));
        assert!(!rules.iter().any(|rule| rule.path.contains("ignored.md")));
        assert!(!rules.iter().any(|rule| rule.path.contains("SKILL.md")));
    }

    #[test]
    fn runtime_capabilities_are_adapter_owned_and_snapshot_free() {
        let adapter = create(Path::new("/Users/example"));
        let capabilities = adapter.capabilities();

        let global_rule = capabilities
            .iter()
            .find(|capability| capability.id == "global-rule")
            .expect("global rule capability");
        assert_eq!(
            global_rule.source_path,
            "/Users/example/.codebuddy/CODEBUDDY.md"
        );
        let rule_projections =
            project_capabilities("codebuddy", ToolCapabilityModule::Rules, &capabilities);
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleGlobalTarget
                && projection.allows(&ProjectedAction::Inject)
                && projection.evidence.source_path == "/Users/example/.codebuddy/CODEBUDDY.md"
        }));
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleNativeFileSource
                && projection.allows(&ProjectedAction::Create)
                && !projection.allows(&ProjectedAction::Inject)
                && projection.evidence.source_path == "/Users/example/.codebuddy/rules"
        }));

        let tool_skills = capabilities
            .iter()
            .find(|capability| capability.id == "agent-skills")
            .expect("tool Skills capability");
        assert_eq!(tool_skills.source_path, "/Users/example/.codebuddy/skills");
        assert!(tool_skills
            .action_evidence
            .iter()
            .all(|evidence| evidence.version.is_none() && evidence.verified_at.is_none()));

        let projections =
            project_capabilities("codebuddy", ToolCapabilityModule::Skills, &capabilities);
        assert!(projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.allows(&ProjectedAction::Install)
        }));
        assert!(projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
                && projection.allows(&ProjectedAction::View)
        }));

        let mcp_projections =
            project_capabilities("codebuddy", ToolCapabilityModule::Mcp, &capabilities);
        assert!(mcp_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::McpGlobalConfig
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.codebuddy/mcp.json"
        }));

        let settings_projections = project_capabilities(
            "codebuddy",
            ToolCapabilityModule::OrdinaryConfig,
            &capabilities,
        );
        assert!(settings_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::OrdinaryConfigFile
                && projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path == "/Users/example/.codebuddy/settings.json"
        }));
    }
}
