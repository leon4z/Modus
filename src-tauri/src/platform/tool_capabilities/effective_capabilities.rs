// Purpose: Resolve certified adapter capability facts with user-configured overlays.

use super::{
    ToolCapability, ToolCapabilityAccess, ToolCapabilityAction, ToolCapabilityActionEvidence,
    ToolCapabilityFormat, ToolCapabilityKind, ToolCapabilityScope, ToolCapabilitySourceConfidence,
    ToolCapabilitySourceKind, ToolSourceDiagnosticState,
};
use crate::platform::config::{AppConfig, ToolCapabilityOverrides};

pub fn resolve_effective_capabilities(
    tool_id: &str,
    base_capabilities: &[ToolCapability],
    config: &AppConfig,
) -> Vec<ToolCapability> {
    let canonical_tool_id =
        crate::platform::tool_catalog::normalization::canonical_tool_id(tool_id);
    let overrides = config.tool_capability_overrides.get(&canonical_tool_id);
    let mut capabilities = base_capabilities.to_vec();

    if let Some(overrides) = overrides {
        apply_shared_skill_override(&mut capabilities, overrides);
        apply_rule_source_override(&mut capabilities, overrides);
        apply_mcp_config_override(&mut capabilities, overrides);
        apply_tool_config_override(&mut capabilities, overrides);
    }
    if let Some(skill_dir) = normalized_custom_tool_skill_dir(&config, &canonical_tool_id) {
        apply_tool_skill_directory_override(&mut capabilities, skill_dir);
    }

    capabilities
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RuleSourceOverride {
    Directory(String),
}

fn apply_rule_source_override(
    capabilities: &mut Vec<ToolCapability>,
    overrides: &ToolCapabilityOverrides,
) {
    if let Some(source) = normalized_rule_source_override(overrides) {
        match source {
            RuleSourceOverride::Directory(directory) => {
                capabilities.push(user_configured_rule_source_directory(directory));
            }
        }
    }

    if let Some(target) = overrides.normalized_global_rule_target() {
        capabilities.push(user_configured_global_rule_target(target));
    }
}

fn normalized_rule_source_override(
    overrides: &ToolCapabilityOverrides,
) -> Option<RuleSourceOverride> {
    let path = overrides.normalized_rule_source_path()?;
    match overrides
        .normalized_rule_source_type()
        .unwrap_or("directory")
    {
        "directory" => Some(RuleSourceOverride::Directory(path)),
        _ => None,
    }
}

fn normalized_custom_tool_skill_dir(config: &AppConfig, tool_id: &str) -> Option<String> {
    let path = config.tool_paths.get(tool_id)?.skills_dir.trim();
    if path.is_empty() {
        return None;
    }
    Some(shellexpand::tilde(path).to_string())
}

fn apply_tool_skill_directory_override(
    capabilities: &mut Vec<ToolCapability>,
    source_path: String,
) {
    let mut found = false;
    for capability in capabilities.iter_mut().filter(|capability| {
        capability.kind == ToolCapabilityKind::Skill
            && capability.scope == ToolCapabilityScope::Tool
            && capability.format == ToolCapabilityFormat::SkillDirectory
            && capability.supporting_sources.is_empty()
    }) {
        found = true;
        apply_tool_skill_directory_state(capability, &source_path);
    }
    if !found {
        capabilities.insert(0, user_configured_tool_skill_directory(source_path));
    }
}

fn apply_mcp_config_override(
    capabilities: &mut Vec<ToolCapability>,
    overrides: &ToolCapabilityOverrides,
) {
    if let Some(source_path) = overrides.normalized_mcp_config_path() {
        capabilities.push(user_configured_mcp_config(source_path));
    }
}

fn apply_tool_config_override(
    capabilities: &mut Vec<ToolCapability>,
    overrides: &ToolCapabilityOverrides,
) {
    if let Some(source_path) = overrides.normalized_tool_config_path() {
        capabilities.push(user_configured_tool_config(source_path));
    }
}

fn apply_tool_skill_directory_state(capability: &mut ToolCapability, source_path: &str) {
    capability.access = ToolCapabilityAccess::Writable;
    capability.format = ToolCapabilityFormat::SkillDirectory;
    capability.source_path = source_path.to_string();
    capability.diagnostics = directory_diagnostics();
    capability.source_confidence = ToolCapabilitySourceConfidence::UserConfigured;
    capability.notes = "Tool Skill directory configured by the user; writes still use Skill preview and confirmation.".to_string();
    capability.source_kind = ToolCapabilitySourceKind::FeatureSource;
    capability.action_evidence = user_configured_actions(&[
        ToolCapabilityAction::View,
        ToolCapabilityAction::Read,
        ToolCapabilityAction::Diagnose,
        ToolCapabilityAction::Install,
        ToolCapabilityAction::Link,
        ToolCapabilityAction::Uninstall,
        ToolCapabilityAction::Copy,
        ToolCapabilityAction::Save,
        ToolCapabilityAction::Delete,
    ]);
}

fn apply_shared_skill_override(
    capabilities: &mut Vec<ToolCapability>,
    overrides: &ToolCapabilityOverrides,
) {
    let Some(direct_read) = overrides.shared_skill_direct_read else {
        return;
    };
    let mut found = false;
    for capability in capabilities.iter_mut().filter(|capability| {
        capability.kind == ToolCapabilityKind::Skill
            && capability.scope == ToolCapabilityScope::Shared
    }) {
        found = true;
        apply_shared_skill_direct_read_state(capability, direct_read);
    }
    if !found {
        capabilities.push(user_configured_shared_skill_capability(direct_read));
    }
}

fn apply_shared_skill_direct_read_state(capability: &mut ToolCapability, direct_read: bool) {
    capability.source_confidence = ToolCapabilitySourceConfidence::UserConfigured;
    capability.notes = if direct_read {
        "Shared Skill direct-read support is enabled by user configuration.".to_string()
    } else {
        "Shared Skill direct-read support is disabled by user configuration.".to_string()
    };
    if direct_read {
        capability.access = ToolCapabilityAccess::ReadOnly;
        capability.format = ToolCapabilityFormat::SkillDirectory;
        capability.diagnostics = shared_skill_diagnostics();
        capability.action_evidence = user_configured_actions(&[
            ToolCapabilityAction::View,
            ToolCapabilityAction::Read,
            ToolCapabilityAction::Diagnose,
        ]);
    } else {
        capability.access = ToolCapabilityAccess::Unsupported;
        capability.diagnostics = vec![ToolSourceDiagnosticState::Unsupported];
        capability.action_evidence = user_configured_actions(&[ToolCapabilityAction::Diagnose]);
    }
}

fn user_configured_global_rule_target(source_path: String) -> ToolCapability {
    ToolCapability {
        id: "user-configured-global-rule-target".to_string(),
        kind: ToolCapabilityKind::Rule,
        scope: ToolCapabilityScope::Global,
        access: ToolCapabilityAccess::Writable,
        format: rule_format_for_path(&source_path),
        source_path,
        label: "Custom Global Rule target".to_string(),
        diagnostics: vec![
            ToolSourceDiagnosticState::Missing,
            ToolSourceDiagnosticState::Loaded,
            ToolSourceDiagnosticState::Unreadable,
        ],
        source_confidence: ToolCapabilitySourceConfidence::UserConfigured,
        notes: "Exact Global Rule target configured by the user; saving this setting does not create or edit the target file.".to_string(),
        source_kind: ToolCapabilitySourceKind::FeatureSource,
        primary_config_dir: None,
        supporting_sources: vec![],
        action_evidence: user_configured_actions(&[
            ToolCapabilityAction::Diagnose,
            ToolCapabilityAction::Inject,
        ]),
    }
}

fn user_configured_rule_source_directory(source_path: String) -> ToolCapability {
    ToolCapability {
        id: "user-configured-rule-source-directory".to_string(),
        kind: ToolCapabilityKind::Rule,
        scope: ToolCapabilityScope::Global,
        access: ToolCapabilityAccess::Writable,
        format: ToolCapabilityFormat::Directory,
        source_path,
        label: "Custom Rule directory".to_string(),
        diagnostics: directory_diagnostics(),
        source_confidence: ToolCapabilitySourceConfidence::UserConfigured,
        notes: "Rule directory configured by the user; writes still use Rules preview and confirmation.".to_string(),
        source_kind: ToolCapabilitySourceKind::FeatureSource,
        primary_config_dir: None,
        supporting_sources: vec![],
        action_evidence: user_configured_actions(&[
            ToolCapabilityAction::View,
            ToolCapabilityAction::Read,
            ToolCapabilityAction::Diagnose,
            ToolCapabilityAction::Create,
            ToolCapabilityAction::Edit,
            ToolCapabilityAction::Save,
            ToolCapabilityAction::Delete,
        ]),
    }
}

fn user_configured_mcp_config(source_path: String) -> ToolCapability {
    ToolCapability {
        id: "user-configured-mcp-config".to_string(),
        kind: ToolCapabilityKind::Mcp,
        scope: ToolCapabilityScope::Global,
        access: ToolCapabilityAccess::Writable,
        format: config_format_for_path(&source_path),
        source_path,
        label: "Custom MCP configuration".to_string(),
        diagnostics: config_file_diagnostics(),
        source_confidence: ToolCapabilitySourceConfidence::UserConfigured,
        notes: "MCP configuration path configured by the user; saves still use MCP validation and backup.".to_string(),
        source_kind: ToolCapabilitySourceKind::FeatureSource,
        primary_config_dir: None,
        supporting_sources: vec![],
        action_evidence: user_configured_actions(&[
            ToolCapabilityAction::View,
            ToolCapabilityAction::Read,
            ToolCapabilityAction::Diagnose,
            ToolCapabilityAction::Edit,
            ToolCapabilityAction::Save,
            ToolCapabilityAction::Sync,
        ]),
    }
}

fn user_configured_tool_config(source_path: String) -> ToolCapability {
    ToolCapability {
        id: "user-configured-tool-config".to_string(),
        kind: ToolCapabilityKind::OrdinaryConfig,
        scope: ToolCapabilityScope::Global,
        access: ToolCapabilityAccess::Writable,
        format: config_format_for_path(&source_path),
        source_path,
        label: "Custom tool configuration".to_string(),
        diagnostics: config_file_diagnostics(),
        source_confidence: ToolCapabilitySourceConfidence::UserConfigured,
        notes: "Tool configuration path configured by the user; saves still use config validation and backup.".to_string(),
        source_kind: ToolCapabilitySourceKind::FeatureSource,
        primary_config_dir: None,
        supporting_sources: vec![],
        action_evidence: user_configured_actions(&[
            ToolCapabilityAction::View,
            ToolCapabilityAction::Read,
            ToolCapabilityAction::Diagnose,
            ToolCapabilityAction::Edit,
            ToolCapabilityAction::Save,
            ToolCapabilityAction::Sync,
        ]),
    }
}

fn user_configured_tool_skill_directory(source_path: String) -> ToolCapability {
    let mut capability = ToolCapability {
        id: "user-configured-tool-skill-directory".to_string(),
        kind: ToolCapabilityKind::Skill,
        scope: ToolCapabilityScope::Tool,
        access: ToolCapabilityAccess::Writable,
        format: ToolCapabilityFormat::SkillDirectory,
        source_path,
        label: "Tool Skills".to_string(),
        diagnostics: directory_diagnostics(),
        source_confidence: ToolCapabilitySourceConfidence::UserConfigured,
        notes: String::new(),
        source_kind: ToolCapabilitySourceKind::FeatureSource,
        primary_config_dir: None,
        supporting_sources: vec![],
        action_evidence: vec![],
    };
    let source_path = capability.source_path.clone();
    apply_tool_skill_directory_state(&mut capability, &source_path);
    capability
}

fn user_configured_shared_skill_capability(direct_read: bool) -> ToolCapability {
    let mut capability = ToolCapability {
        id: "shared-skills".to_string(),
        kind: ToolCapabilityKind::Skill,
        scope: ToolCapabilityScope::Shared,
        access: ToolCapabilityAccess::ReadOnly,
        format: ToolCapabilityFormat::SkillDirectory,
        source_path: crate::platform::env::generic_skills_dir()
            .to_string_lossy()
            .to_string(),
        label: "Shared Skills".to_string(),
        diagnostics: shared_skill_diagnostics(),
        source_confidence: ToolCapabilitySourceConfidence::UserConfigured,
        notes: String::new(),
        source_kind: ToolCapabilitySourceKind::FeatureSource,
        primary_config_dir: None,
        supporting_sources: vec![],
        action_evidence: vec![],
    };
    apply_shared_skill_direct_read_state(&mut capability, direct_read);
    capability
}

fn user_configured_actions(actions: &[ToolCapabilityAction]) -> Vec<ToolCapabilityActionEvidence> {
    actions
        .iter()
        .map(|action| ToolCapabilityActionEvidence {
            action: action.clone(),
            supported: true,
            evidence: "User-configured capability override.".to_string(),
            variant: None,
            version: None,
            verified_at: None,
        })
        .collect()
}

fn shared_skill_diagnostics() -> Vec<ToolSourceDiagnosticState> {
    vec![
        ToolSourceDiagnosticState::Missing,
        ToolSourceDiagnosticState::Loaded,
        ToolSourceDiagnosticState::Unreadable,
    ]
}

fn config_file_diagnostics() -> Vec<ToolSourceDiagnosticState> {
    vec![
        ToolSourceDiagnosticState::Missing,
        ToolSourceDiagnosticState::Loaded,
        ToolSourceDiagnosticState::Unreadable,
        ToolSourceDiagnosticState::Malformed,
    ]
}

fn directory_diagnostics() -> Vec<ToolSourceDiagnosticState> {
    vec![
        ToolSourceDiagnosticState::Missing,
        ToolSourceDiagnosticState::Loaded,
        ToolSourceDiagnosticState::Unreadable,
    ]
}

fn rule_format_for_path(source_path: &str) -> ToolCapabilityFormat {
    if source_path
        .split('#')
        .next()
        .unwrap_or("")
        .trim()
        .ends_with(".mdc")
    {
        ToolCapabilityFormat::Mdc
    } else {
        ToolCapabilityFormat::Markdown
    }
}

fn config_format_for_path(source_path: &str) -> ToolCapabilityFormat {
    let source = source_path
        .split('#')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if source.ends_with(".jsonc") {
        ToolCapabilityFormat::Jsonc
    } else if source.ends_with(".toml") {
        ToolCapabilityFormat::Toml
    } else if source.ends_with(".yaml") || source.ends_with(".yml") {
        ToolCapabilityFormat::Yaml
    } else {
        ToolCapabilityFormat::Json
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::config::ToolPaths;
    use crate::platform::tool_capabilities::capability_projections::{
        project_capabilities, ToolCapabilityModule, ToolCapabilitySourceRole,
    };
    use crate::platform::tool_capabilities::{
        tool_capability, ToolCapabilitySupportingSource, ToolCapabilitySupportingSourceRole,
    };

    fn rule_capability() -> ToolCapability {
        tool_capability(
            "global-rule",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Markdown,
            "/tmp/base/AGENTS.md",
            "AGENTS.md",
            ToolCapabilitySourceConfidence::OfficialDocs,
            "certified rule target",
        )
    }

    fn shared_skill_capability(access: ToolCapabilityAccess) -> ToolCapability {
        tool_capability(
            "shared-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Shared,
            access,
            ToolCapabilityFormat::SkillDirectory,
            "/tmp/shared-skills",
            "Shared Skills",
            ToolCapabilitySourceConfidence::OfficialDocs,
            "certified shared Skill state",
        )
    }

    fn compound_skill_capability() -> ToolCapability {
        let mut capability = tool_capability(
            "compound-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Readable,
            ToolCapabilityFormat::SkillDirectory,
            "/tmp/product-managed-skills",
            "Compound Skills",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            "content plus metadata",
        );
        capability.supporting_sources = vec![ToolCapabilitySupportingSource {
            id: "skill-metadata".to_string(),
            role: ToolCapabilitySupportingSourceRole::Metadata,
            source_path: "/tmp/product-managed-skills/skill-config.json".to_string(),
            format: ToolCapabilityFormat::Json,
            required: true,
            diagnostics: vec![ToolSourceDiagnosticState::Loaded],
            notes: "test metadata".to_string(),
        }];
        capability
    }

    fn config_with_override(tool_id: &str, overrides: ToolCapabilityOverrides) -> AppConfig {
        let mut config = AppConfig::default();
        config
            .tool_capability_overrides
            .insert(tool_id.to_string(), overrides);
        crate::platform::tool_catalog::normalization::normalize_config(&mut config);
        config
    }

    #[test]
    fn certified_defaults_project_without_mutating_base_facts() {
        let base = vec![
            rule_capability(),
            shared_skill_capability(ToolCapabilityAccess::ReadOnly),
        ];
        let config = AppConfig::default();

        let effective = resolve_effective_capabilities("codex", &base, &config);

        assert_eq!(effective, base);
        let skill_projections =
            project_capabilities("codex", ToolCapabilityModule::Skills, &effective);
        assert!(skill_projections.iter().any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
                && projection.allows(&ToolCapabilityAction::View)
                && projection.evidence.source_confidence
                    == ToolCapabilitySourceConfidence::OfficialDocs
        }));
        assert_eq!(
            base[1].source_confidence,
            ToolCapabilitySourceConfidence::OfficialDocs
        );
    }

    #[test]
    fn user_configured_global_rule_target_projects_as_injectable() {
        let base = vec![shared_skill_capability(ToolCapabilityAccess::ReadOnly)];
        let config = config_with_override(
            "cursor",
            ToolCapabilityOverrides {
                custom_rule_source_type: None,
                custom_rule_source_path: None,
                custom_global_rule_target: Some("/tmp/cursor/CUSTOM.md".to_string()),
                shared_skill_direct_read: None,
                ..Default::default()
            },
        );

        let effective = resolve_effective_capabilities("cursor", &base, &config);
        let rule_projections =
            project_capabilities("cursor", ToolCapabilityModule::Rules, &effective);
        let custom_projection = rule_projections
            .iter()
            .find(|projection| projection.evidence.id == "user-configured-global-rule-target")
            .expect("expected user-configured rule target");

        assert_eq!(
            custom_projection.source_role,
            ToolCapabilitySourceRole::RuleGlobalTarget
        );
        assert_eq!(custom_projection.exclusion_reason, None);
        assert!(custom_projection.allows(&ToolCapabilityAction::Inject));
        assert!(!custom_projection.allows(&ToolCapabilityAction::Create));
        assert!(!custom_projection.allows(&ToolCapabilityAction::Edit));
        assert!(!custom_projection.allows(&ToolCapabilityAction::Save));
        assert!(!custom_projection.allows(&ToolCapabilityAction::View));
        assert_eq!(
            custom_projection.evidence.source_confidence,
            ToolCapabilitySourceConfidence::UserConfigured
        );
        assert!(!rule_projections
            .iter()
            .any(|projection| projection.source_role
                == ToolCapabilitySourceRole::RuleNativeFileSource));
        assert_eq!(base.len(), 1);
    }

    #[test]
    fn user_configured_directory_rule_source_and_global_target_project_independently() {
        let base = vec![rule_capability()];
        let config = config_with_override(
            "codex",
            ToolCapabilityOverrides {
                custom_rule_source_type: Some("directory".to_string()),
                custom_rule_source_path: Some("/tmp/codex/rules".to_string()),
                custom_global_rule_target: Some("/tmp/outside/AGENTS.md".to_string()),
                shared_skill_direct_read: None,
                ..Default::default()
            },
        );

        let effective = resolve_effective_capabilities("codex", &base, &config);
        let rule_projections =
            project_capabilities("codex", ToolCapabilityModule::Rules, &effective);

        let directory_projection = rule_projections
            .iter()
            .find(|projection| projection.evidence.id == "user-configured-rule-source-directory")
            .expect("expected user-configured directory rule source");
        assert_eq!(
            directory_projection.source_role,
            ToolCapabilitySourceRole::RuleNativeFileSource
        );
        assert_eq!(
            directory_projection.evidence.source_path,
            "/tmp/codex/rules"
        );
        assert!(directory_projection.allows(&ToolCapabilityAction::Create));
        assert!(!directory_projection.allows(&ToolCapabilityAction::Inject));
        assert!(effective.iter().any(|capability| {
            capability.id == "user-configured-global-rule-target"
                && capability.source_path == "/tmp/outside/AGENTS.md"
        }));
        assert!(effective.iter().any(|capability| {
            capability.id == "global-rule" && capability.source_path == "/tmp/base/AGENTS.md"
        }));
    }

    #[test]
    fn user_configured_directory_rule_source_does_not_constrain_global_target() {
        let config = config_with_override(
            "codex",
            ToolCapabilityOverrides {
                custom_rule_source_type: Some("directory".to_string()),
                custom_rule_source_path: Some("/tmp/codex/rules".to_string()),
                custom_global_rule_target: Some("/tmp/codex/rules-sibling/AGENTS.md".to_string()),
                shared_skill_direct_read: None,
                ..Default::default()
            },
        );

        let effective = resolve_effective_capabilities("codex", &[], &config);

        assert!(effective.iter().any(|capability| {
            capability.id == "user-configured-global-rule-target"
                && capability.source_path == "/tmp/codex/rules-sibling/AGENTS.md"
        }));
    }

    #[test]
    fn shared_skill_true_override_enables_readable_shared_projection() {
        let base = vec![shared_skill_capability(ToolCapabilityAccess::Unknown)];
        let config = config_with_override(
            "trae-cn",
            ToolCapabilityOverrides {
                custom_rule_source_type: None,
                custom_rule_source_path: None,
                custom_global_rule_target: None,
                shared_skill_direct_read: Some(true),
                ..Default::default()
            },
        );

        let effective = resolve_effective_capabilities("trae-cn", &base, &config);
        let skill_projections =
            project_capabilities("trae-cn", ToolCapabilityModule::Skills, &effective);
        let shared_projection = skill_projections
            .iter()
            .find(|projection| {
                projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
            })
            .expect("expected shared Skill projection");

        assert_eq!(shared_projection.exclusion_reason, None);
        assert!(shared_projection.allows(&ToolCapabilityAction::View));
        assert_eq!(
            shared_projection.evidence.source_confidence,
            ToolCapabilitySourceConfidence::UserConfigured
        );
        assert_eq!(base[0].access, ToolCapabilityAccess::Unknown);
    }

    #[test]
    fn user_configured_tool_skill_directory_enables_deployable_projection() {
        let mut config = AppConfig::default();
        config.tool_paths.insert(
            "claude-code".to_string(),
            ToolPaths {
                config_dir: String::new(),
                skills_dir: "/tmp/custom-claude-skills".to_string(),
            },
        );

        let effective = resolve_effective_capabilities("claude-code", &[], &config);
        let skill_projections =
            project_capabilities("claude-code", ToolCapabilityModule::Skills, &effective);
        let projection = skill_projections
            .iter()
            .find(|projection| {
                projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
            })
            .expect("expected user configured tool Skill directory");

        assert!(projection.allows(&ToolCapabilityAction::Install));
        assert!(projection.allows(&ToolCapabilityAction::Link));
        assert!(projection.allows(&ToolCapabilityAction::Uninstall));
        assert_eq!(
            projection.evidence.source_confidence,
            ToolCapabilitySourceConfidence::UserConfigured
        );
        assert_eq!(projection.evidence.source_path, "/tmp/custom-claude-skills");
    }

    #[test]
    fn user_configured_tool_skill_directory_does_not_mutate_compound_source() {
        let mut config = AppConfig::default();
        config.tool_paths.insert(
            "trae-solo-cn".to_string(),
            ToolPaths {
                config_dir: String::new(),
                skills_dir: "/tmp/custom-trae-solo-cn-skills".to_string(),
            },
        );
        let base = vec![compound_skill_capability()];

        let effective = resolve_effective_capabilities("trae-solo-cn", &base, &config);
        let compound = effective
            .iter()
            .find(|capability| capability.id == "compound-skills")
            .expect("expected original compound source");
        assert_eq!(compound.access, ToolCapabilityAccess::Readable);
        assert_eq!(compound.source_path, "/tmp/product-managed-skills");
        assert!(!compound.supporting_sources.is_empty());

        let skill_projections =
            project_capabilities("trae-solo-cn", ToolCapabilityModule::Skills, &effective);
        let user_projection = skill_projections
            .iter()
            .find(|projection| projection.evidence.id == "user-configured-tool-skill-directory")
            .expect("expected independent user configured tool Skill directory");
        assert_eq!(
            user_projection.source_role,
            ToolCapabilitySourceRole::SkillToolDirectory
        );
        assert!(user_projection.allows(&ToolCapabilityAction::Install));
        assert_eq!(
            user_projection.evidence.source_path,
            "/tmp/custom-trae-solo-cn-skills"
        );
    }

    #[test]
    fn shared_skill_false_override_closes_certified_shared_projection() {
        let base = vec![shared_skill_capability(ToolCapabilityAccess::ReadOnly)];
        let config = config_with_override(
            "codex",
            ToolCapabilityOverrides {
                custom_rule_source_type: None,
                custom_rule_source_path: None,
                custom_global_rule_target: None,
                shared_skill_direct_read: Some(false),
                ..Default::default()
            },
        );

        let effective = resolve_effective_capabilities("codex", &base, &config);
        let skill_projections =
            project_capabilities("codex", ToolCapabilityModule::Skills, &effective);
        let shared_projection = skill_projections
            .iter()
            .find(|projection| {
                projection.source_role == ToolCapabilitySourceRole::SkillNonActionableEvidence
            })
            .expect("expected disabled shared Skill projection");

        assert_eq!(
            shared_projection.evidence.access,
            ToolCapabilityAccess::Unsupported
        );
        assert!(!shared_projection.allows(&ToolCapabilityAction::View));
        assert_eq!(
            shared_projection.evidence.source_confidence,
            ToolCapabilitySourceConfidence::UserConfigured
        );
        assert_eq!(base[0].access, ToolCapabilityAccess::ReadOnly);
    }

    #[test]
    fn empty_override_returns_to_certified_projection() {
        let base = vec![shared_skill_capability(ToolCapabilityAccess::ReadOnly)];
        let config = config_with_override(
            "codex",
            ToolCapabilityOverrides {
                custom_rule_source_type: None,
                custom_rule_source_path: None,
                custom_global_rule_target: Some("  ".to_string()),
                shared_skill_direct_read: None,
                ..Default::default()
            },
        );

        let effective = resolve_effective_capabilities("codex", &base, &config);

        assert_eq!(effective, base);
        assert!(config.tool_capability_overrides.is_empty());
    }

    #[test]
    fn legacy_alias_override_is_resolved_by_canonical_tool_id() {
        let base = vec![shared_skill_capability(ToolCapabilityAccess::ReadOnly)];
        let config = config_with_override(
            "claude_code",
            ToolCapabilityOverrides {
                custom_rule_source_type: None,
                custom_rule_source_path: None,
                custom_global_rule_target: Some("/tmp/claude/CUSTOM.md".to_string()),
                shared_skill_direct_read: Some(false),
                ..Default::default()
            },
        );

        let effective = resolve_effective_capabilities("claude_code", &base, &config);

        assert!(config.tool_capability_overrides.contains_key("claude-code"));
        assert!(effective
            .iter()
            .any(|capability| capability.id == "user-configured-global-rule-target"));
        assert!(effective.iter().any(|capability| {
            capability.kind == ToolCapabilityKind::Skill
                && capability.scope == ToolCapabilityScope::Shared
                && capability.access == ToolCapabilityAccess::Unsupported
        }));
    }

    #[test]
    fn user_configured_mcp_and_tool_config_paths_project_as_writable_sources() {
        let config = config_with_override(
            "codex",
            ToolCapabilityOverrides {
                custom_mcp_config_path: Some("/tmp/custom/mcp.json".to_string()),
                custom_tool_config_path: Some("/tmp/custom/settings.toml".to_string()),
                ..Default::default()
            },
        );

        let effective = resolve_effective_capabilities("codex", &[], &config);
        let mcp_projections = project_capabilities("codex", ToolCapabilityModule::Mcp, &effective);
        let mcp_projection = mcp_projections
            .iter()
            .find(|projection| projection.evidence.id == "user-configured-mcp-config")
            .expect("expected custom MCP config projection");
        assert_eq!(
            mcp_projection.evidence.source_confidence,
            ToolCapabilitySourceConfidence::UserConfigured
        );
        assert!(mcp_projection.allows(&ToolCapabilityAction::Save));

        let config_projections =
            project_capabilities("codex", ToolCapabilityModule::OrdinaryConfig, &effective);
        let config_projection = config_projections
            .iter()
            .find(|projection| projection.evidence.id == "user-configured-tool-config")
            .expect("expected custom tool config projection");
        assert_eq!(
            config_projection.evidence.source_confidence,
            ToolCapabilitySourceConfidence::UserConfigured
        );
        assert!(config_projection.allows(&ToolCapabilityAction::Save));
    }
}
