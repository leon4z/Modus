// Purpose: Trae Solo CN adapter entry and runtime capability boundary.

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
        registry::definition("trae-solo-cn").expect("missing Trae Solo CN catalog definition");
    Box::new(
        DeclaredToolAdapter::new(
            definition.id,
            definition.name,
            definition.icon,
            definition.config_dir.resolve_path(home),
            capabilities(home),
        )
        .with_skills_dir(home.join(".trae-cn/skills"))
        .with_detection_paths(&[PathBuf::from("/Applications/TRAE SOLO CN.app")]),
    )
}

fn capabilities(home: &Path) -> Vec<DeclaredCapability> {
    vec![
        capability(
            home,
            "primary-config-directory",
            ToolCapabilityKind::OrdinaryConfig,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Readable,
            ToolCapabilityFormat::Directory,
            home.join(".trae-cn"),
            "Primary configuration directory",
            directory_diagnostics(),
            "Trae Solo CN primary local data root. App support state, MCP, and Settings are declared separately.",
            ToolCapabilitySourceKind::PrimaryConfigDirectory,
            runtime_action_gates(
                "Trae Solo CN primary configuration directory",
                "~/.trae-cn",
                &[ToolCapabilityAction::View, ToolCapabilityAction::Read, ToolCapabilityAction::Diagnose],
            ),
        ),
        capability(
            home,
            "user-rules",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Directory,
            home.join(".trae-cn/user_rules"),
            "User Rules",
            directory_diagnostics(),
            "Trae Solo CN local user Rules persist under ~/.trae-cn/user_rules. This directory is a native rule source, not a default Global Rule injection target.",
            ToolCapabilitySourceKind::FeatureSource,
            runtime_action_gates(
                "Trae Solo CN user Rules directory",
                "~/.trae-cn/user_rules",
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
            home,
            "skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            home.join(".trae-cn/skills"),
            "Skills",
            directory_diagnostics(),
            "Trae Solo CN discovers user-level Skills from ~/.trae-cn/skills. Marketplace metadata is tool-owned state and not treated as a separate Modus write source.",
            ToolCapabilitySourceKind::FeatureSource,
            runtime_action_gates(
                "Trae Solo CN Skills directory",
                "~/.trae-cn/skills",
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
                    ToolCapabilityAction::Link,
                ],
            ),
        ),
        capability(
            home,
            "mcp-config",
            ToolCapabilityKind::Mcp,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            home.join("Library/Application Support/TRAE SOLO CN/User/mcp.json"),
            "MCP configuration",
            file_diagnostics(),
            "Trae Solo CN user-level MCP configuration is stored in the app support mcp.json file under mcpServers. Modus edits single entry fragments only and does not manage server lifecycle.",
            ToolCapabilitySourceKind::FeatureSource,
            runtime_action_gates(
                "Trae Solo CN MCP configuration",
                "~/Library/Application Support/TRAE SOLO CN/User/mcp.json",
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
            home,
            "ordinary-settings",
            ToolCapabilityKind::OrdinaryConfig,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            home.join("Library/Application Support/TRAE SOLO CN/User/settings.json"),
            "Settings",
            file_diagnostics(),
            "Trae Solo CN user settings file. Rules, Skills, MCP, caches, logs, and runtime state remain separate feature sources.",
            ToolCapabilitySourceKind::FeatureSource,
            runtime_action_gates(
                "Trae Solo CN settings",
                "~/Library/Application Support/TRAE SOLO CN/User/settings.json",
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
    home: &Path,
    id: &'static str,
    kind: ToolCapabilityKind,
    scope: ToolCapabilityScope,
    access: ToolCapabilityAccess,
    format: ToolCapabilityFormat,
    source_path: PathBuf,
    label: &'static str,
    diagnostics: Vec<ToolSourceDiagnosticState>,
    notes: &'static str,
    source_kind: ToolCapabilitySourceKind,
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
        source_kind,
        primary_config_dir: Some(home.join(".trae-cn").to_string_lossy().to_string()),
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

        assert_eq!(
            adapter.config_dir(),
            Path::new("/Users/example").join(".trae-cn")
        );
        assert_eq!(
            adapter.primary_config_dir(),
            Path::new("/Users/example").join(".trae-cn")
        );
        assert!(capabilities
            .iter()
            .filter(|capability| !capability.action_evidence.is_empty())
            .all(|capability| capability
                .action_evidence
                .iter()
                .all(|evidence| evidence.version.is_none() && evidence.verified_at.is_none())));

        let rule_projections =
            project_capabilities("trae-solo-cn", ToolCapabilityModule::Rules, &capabilities);
        assert!(rule_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::RuleNativeFileSource
                && projection.allows(&ProjectedAction::Create)
                && projection.allows(&ProjectedAction::Save)
                && projection.allows(&ProjectedAction::Delete)
                && !projection.allows(&ProjectedAction::Inject)
                && projection.evidence.source_path == "/Users/example/.trae-cn/user_rules"
        }));

        let skill_projections =
            project_capabilities("trae-solo-cn", ToolCapabilityModule::Skills, &capabilities);
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.allows(&ProjectedAction::Install)
                && projection.evidence.source_path == "/Users/example/.trae-cn/skills"
        }));
        assert!(!skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
                && projection.allows(&ProjectedAction::Read)
        }));

        let mcp_projections =
            project_capabilities("trae-solo-cn", ToolCapabilityModule::Mcp, &capabilities);
        assert!(mcp_projections.iter().any(|projection| {
            projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path
                    == "/Users/example/Library/Application Support/TRAE SOLO CN/User/mcp.json"
        }));

        let ordinary_projections = project_capabilities(
            "trae-solo-cn",
            ToolCapabilityModule::OrdinaryConfig,
            &capabilities,
        );
        assert!(ordinary_projections.iter().any(|projection| {
            projection.allows(&ProjectedAction::Save)
                && projection.evidence.source_path
                    == "/Users/example/Library/Application Support/TRAE SOLO CN/User/settings.json"
        }));
    }
}
