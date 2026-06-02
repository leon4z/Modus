// Purpose: Shared module projections that turn tool capability facts into product action gates.

use super::{
    capability_declared_source_path, capability_is_eligible_global_rule_target,
    capability_source_is_directory, ToolCapability, ToolCapabilityAccess, ToolCapabilityFormat,
    ToolCapabilityKind, ToolCapabilityScope, ToolCapabilitySourceKind,
};
use serde::{Deserialize, Serialize};

pub(crate) use super::ToolCapabilityAction;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapabilityModule {
    Rules,
    Skills,
    Mcp,
    OrdinaryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapabilitySourceRole {
    RuleGlobalTarget,
    RuleNativeFileSource,
    RuleProjectSource,
    RuleNonActionableEvidence,
    SkillToolDirectory,
    SkillCompoundSource,
    SkillSharedSource,
    SkillNonActionableEvidence,
    McpGlobalConfig,
    McpProjectConfig,
    McpNonActionableEvidence,
    OrdinaryConfigFile,
    OrdinaryConfigNonActionableEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapabilityExclusionReason {
    WrongKind,
    ProjectScoped,
    ReadOnly,
    Unsupported,
    Unknown,
    MissingPath,
    UnsupportedFormat,
    UntrustedSource,
    ActionNotVerified,
    CompoundSourceRequiresVerifiedAction,
    OutOfScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCapabilityProjection {
    pub module: ToolCapabilityModule,
    pub source_role: ToolCapabilitySourceRole,
    pub actions: Vec<ToolCapabilityAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusion_reason: Option<ToolCapabilityExclusionReason>,
    pub evidence: ToolCapability,
}

impl ToolCapabilityProjection {
    pub fn allows(&self, action: &ToolCapabilityAction) -> bool {
        self.actions.iter().any(|candidate| candidate == action)
    }
}

pub fn project_capability(
    adapter_id: &str,
    module: ToolCapabilityModule,
    capability: &ToolCapability,
) -> ToolCapabilityProjection {
    match module {
        ToolCapabilityModule::Rules => project_rule_capability(adapter_id, capability),
        ToolCapabilityModule::Skills => project_skill_capability(capability),
        ToolCapabilityModule::Mcp => project_mcp_capability(capability),
        ToolCapabilityModule::OrdinaryConfig => project_ordinary_config_capability(capability),
    }
}

pub fn project_capabilities(
    adapter_id: &str,
    module: ToolCapabilityModule,
    capabilities: &[ToolCapability],
) -> Vec<ToolCapabilityProjection> {
    capabilities
        .iter()
        .map(|capability| project_capability(adapter_id, module.clone(), capability))
        .filter(|projection| {
            projection.exclusion_reason != Some(ToolCapabilityExclusionReason::WrongKind)
        })
        .collect()
}

fn evidence_actions(capability: &ToolCapability) -> Vec<ToolCapabilityAction> {
    if let Some(actions) = explicit_supported_actions(capability) {
        return actions;
    }
    if capability.is_readable() {
        vec![
            ToolCapabilityAction::View,
            ToolCapabilityAction::Read,
            ToolCapabilityAction::Diagnose,
        ]
    } else if capability.access != ToolCapabilityAccess::Unsupported {
        vec![ToolCapabilityAction::Diagnose]
    } else {
        vec![]
    }
}

fn rule_source_actions(capability: &ToolCapability) -> Vec<ToolCapabilityAction> {
    if let Some(actions) = explicit_supported_actions(capability) {
        return actions;
    }
    let mut actions = evidence_actions(capability);
    if capability.is_writable() {
        for action in [
            ToolCapabilityAction::Create,
            ToolCapabilityAction::Edit,
            ToolCapabilityAction::Save,
            ToolCapabilityAction::Delete,
        ] {
            if !actions.contains(&action) {
                actions.push(action);
            }
        }
    }
    actions
}

fn rule_global_target_actions(capability: &ToolCapability) -> Vec<ToolCapabilityAction> {
    let mut actions = rule_source_actions(capability);
    if !actions.contains(&ToolCapabilityAction::Inject) {
        actions.push(ToolCapabilityAction::Inject);
    }
    actions
}

fn rule_native_source_actions(capability: &ToolCapability) -> Vec<ToolCapabilityAction> {
    rule_source_actions(capability)
        .into_iter()
        .filter(|action| action != &ToolCapabilityAction::Inject)
        .collect()
}

fn explicit_supported_actions(capability: &ToolCapability) -> Option<Vec<ToolCapabilityAction>> {
    if !capability.has_action_evidence() {
        return None;
    }
    let mut actions = vec![];
    for evidence in capability
        .action_evidence
        .iter()
        .filter(|evidence| evidence.supported)
    {
        push_action_with_read_aliases(&mut actions, evidence.action.clone());
    }
    Some(actions)
}

fn push_action_with_read_aliases(
    actions: &mut Vec<ToolCapabilityAction>,
    action: ToolCapabilityAction,
) {
    if action == ToolCapabilityAction::View || action == ToolCapabilityAction::Read {
        for alias in [ToolCapabilityAction::View, ToolCapabilityAction::Read] {
            if !actions.contains(&alias) {
                actions.push(alias);
            }
        }
        return;
    }
    if !actions.contains(&action) {
        actions.push(action);
    }
}

fn has_verified_action(capability: &ToolCapability, action: &ToolCapabilityAction) -> bool {
    if capability.has_action_evidence() {
        capability.has_supported_action(action)
            || ((*action == ToolCapabilityAction::View || *action == ToolCapabilityAction::Read)
                && (capability.has_supported_action(&ToolCapabilityAction::View)
                    || capability.has_supported_action(&ToolCapabilityAction::Read)))
    } else {
        false
    }
}

fn has_any_verified_managed_skill_action(capability: &ToolCapability) -> bool {
    [
        ToolCapabilityAction::Install,
        ToolCapabilityAction::Copy,
        ToolCapabilityAction::Uninstall,
        ToolCapabilityAction::Delete,
        ToolCapabilityAction::Update,
        ToolCapabilityAction::Repair,
        ToolCapabilityAction::Sync,
    ]
    .iter()
    .any(|action| has_verified_action(capability, action))
}

fn skill_tool_directory_actions(
    capability: &ToolCapability,
    source_role: &ToolCapabilitySourceRole,
) -> Vec<ToolCapabilityAction> {
    let mut actions = explicit_supported_actions(capability).unwrap_or_else(|| {
        vec![
            ToolCapabilityAction::View,
            ToolCapabilityAction::Read,
            ToolCapabilityAction::Diagnose,
            ToolCapabilityAction::Install,
            ToolCapabilityAction::Copy,
            ToolCapabilityAction::Uninstall,
            ToolCapabilityAction::Delete,
            ToolCapabilityAction::Update,
            ToolCapabilityAction::Repair,
            ToolCapabilityAction::Link,
            ToolCapabilityAction::Sync,
        ]
    });

    if *source_role == ToolCapabilitySourceRole::SkillToolDirectory && capability.is_writable() {
        for action in [
            ToolCapabilityAction::Install,
            ToolCapabilityAction::Link,
            ToolCapabilityAction::Uninstall,
        ] {
            push_action_with_read_aliases(&mut actions, action);
        }
    }

    actions
}

fn wrong_kind_projection(
    module: ToolCapabilityModule,
    source_role: ToolCapabilitySourceRole,
    capability: &ToolCapability,
) -> ToolCapabilityProjection {
    ToolCapabilityProjection {
        module,
        source_role,
        actions: vec![],
        exclusion_reason: Some(ToolCapabilityExclusionReason::WrongKind),
        evidence: capability.clone(),
    }
}

fn non_actionable_projection(
    module: ToolCapabilityModule,
    source_role: ToolCapabilitySourceRole,
    capability: &ToolCapability,
    reason: ToolCapabilityExclusionReason,
) -> ToolCapabilityProjection {
    ToolCapabilityProjection {
        module,
        source_role,
        actions: evidence_actions(capability),
        exclusion_reason: Some(reason),
        evidence: capability.clone(),
    }
}

fn project_rule_capability(
    adapter_id: &str,
    capability: &ToolCapability,
) -> ToolCapabilityProjection {
    if capability.kind != ToolCapabilityKind::Rule {
        return wrong_kind_projection(
            ToolCapabilityModule::Rules,
            ToolCapabilitySourceRole::RuleNonActionableEvidence,
            capability,
        );
    }
    if capability.access == ToolCapabilityAccess::Unsupported {
        return non_actionable_projection(
            ToolCapabilityModule::Rules,
            ToolCapabilitySourceRole::RuleNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::Unsupported,
        );
    }
    if capability.access == ToolCapabilityAccess::Unknown {
        return non_actionable_projection(
            ToolCapabilityModule::Rules,
            ToolCapabilitySourceRole::RuleNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::Unknown,
        );
    }
    if capability.scope == ToolCapabilityScope::Project {
        return non_actionable_projection(
            ToolCapabilityModule::Rules,
            ToolCapabilitySourceRole::RuleProjectSource,
            capability,
            ToolCapabilityExclusionReason::ProjectScoped,
        );
    }
    if !matches!(
        capability.format,
        ToolCapabilityFormat::Markdown
            | ToolCapabilityFormat::InstructionsMarkdown
            | ToolCapabilityFormat::Mdc
            | ToolCapabilityFormat::Directory
    ) {
        return non_actionable_projection(
            ToolCapabilityModule::Rules,
            ToolCapabilitySourceRole::RuleNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::UnsupportedFormat,
        );
    }
    let Some(source) = capability_declared_source_path(capability) else {
        return non_actionable_projection(
            ToolCapabilityModule::Rules,
            ToolCapabilitySourceRole::RuleNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::MissingPath,
        );
    };
    let actions = rule_source_actions(capability);
    if capability.has_action_evidence() {
        if !actions.iter().any(|action| {
            matches!(
                action,
                ToolCapabilityAction::Create
                    | ToolCapabilityAction::Edit
                    | ToolCapabilityAction::Save
                    | ToolCapabilityAction::Delete
                    | ToolCapabilityAction::Inject
            )
        }) {
            return ToolCapabilityProjection {
                module: ToolCapabilityModule::Rules,
                source_role: ToolCapabilitySourceRole::RuleNativeFileSource,
                actions,
                exclusion_reason: Some(ToolCapabilityExclusionReason::ActionNotVerified),
                evidence: capability.clone(),
            };
        }
        if !has_verified_action(capability, &ToolCapabilityAction::Inject) {
            return ToolCapabilityProjection {
                module: ToolCapabilityModule::Rules,
                source_role: ToolCapabilitySourceRole::RuleNativeFileSource,
                actions,
                exclusion_reason: None,
                evidence: capability.clone(),
            };
        }
    }
    if capability_is_eligible_global_rule_target(adapter_id, capability)
        && (!capability.has_action_evidence()
            || has_verified_action(capability, &ToolCapabilityAction::Inject))
    {
        return ToolCapabilityProjection {
            module: ToolCapabilityModule::Rules,
            source_role: ToolCapabilitySourceRole::RuleGlobalTarget,
            actions: rule_global_target_actions(capability),
            exclusion_reason: None,
            evidence: capability.clone(),
        };
    }
    ToolCapabilityProjection {
        module: ToolCapabilityModule::Rules,
        source_role: ToolCapabilitySourceRole::RuleNativeFileSource,
        actions: rule_native_source_actions(capability),
        exclusion_reason: if capability_source_is_directory(capability, &source)
            || capability.is_readable()
        {
            None
        } else {
            Some(ToolCapabilityExclusionReason::ReadOnly)
        },
        evidence: capability.clone(),
    }
}

fn project_skill_capability(capability: &ToolCapability) -> ToolCapabilityProjection {
    if capability.kind != ToolCapabilityKind::Skill {
        return wrong_kind_projection(
            ToolCapabilityModule::Skills,
            ToolCapabilitySourceRole::SkillNonActionableEvidence,
            capability,
        );
    }
    if capability.access == ToolCapabilityAccess::Unsupported {
        return non_actionable_projection(
            ToolCapabilityModule::Skills,
            ToolCapabilitySourceRole::SkillNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::Unsupported,
        );
    }
    if capability.access == ToolCapabilityAccess::Unknown {
        return non_actionable_projection(
            ToolCapabilityModule::Skills,
            ToolCapabilitySourceRole::SkillNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::Unknown,
        );
    }
    if capability.scope == ToolCapabilityScope::Project {
        return non_actionable_projection(
            ToolCapabilityModule::Skills,
            ToolCapabilitySourceRole::SkillNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::ProjectScoped,
        );
    }
    if capability.format != ToolCapabilityFormat::SkillDirectory {
        return non_actionable_projection(
            ToolCapabilityModule::Skills,
            ToolCapabilitySourceRole::SkillNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::UnsupportedFormat,
        );
    }
    if capability_declared_source_path(capability).is_none() {
        return non_actionable_projection(
            ToolCapabilityModule::Skills,
            ToolCapabilitySourceRole::SkillNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::MissingPath,
        );
    }
    if capability.scope == ToolCapabilityScope::Shared {
        return ToolCapabilityProjection {
            module: ToolCapabilityModule::Skills,
            source_role: ToolCapabilitySourceRole::SkillSharedSource,
            actions: evidence_actions(capability),
            exclusion_reason: None,
            evidence: capability.clone(),
        };
    }
    let role = if capability.has_compound_supporting_sources() {
        ToolCapabilitySourceRole::SkillCompoundSource
    } else {
        ToolCapabilitySourceRole::SkillToolDirectory
    };
    if capability.scope == ToolCapabilityScope::Tool
        && capability.has_compound_supporting_sources()
        && !has_any_verified_managed_skill_action(capability)
    {
        return ToolCapabilityProjection {
            module: ToolCapabilityModule::Skills,
            source_role: role,
            actions: evidence_actions(capability),
            exclusion_reason: Some(
                ToolCapabilityExclusionReason::CompoundSourceRequiresVerifiedAction,
            ),
            evidence: capability.clone(),
        };
    }
    if capability.scope == ToolCapabilityScope::Tool
        && (capability.is_writable() || has_any_verified_managed_skill_action(capability))
    {
        let actions = skill_tool_directory_actions(capability, &role);
        return ToolCapabilityProjection {
            module: ToolCapabilityModule::Skills,
            source_role: role,
            actions,
            exclusion_reason: None,
            evidence: capability.clone(),
        };
    }
    non_actionable_projection(
        ToolCapabilityModule::Skills,
        role,
        capability,
        ToolCapabilityExclusionReason::ReadOnly,
    )
}

fn project_mcp_capability(capability: &ToolCapability) -> ToolCapabilityProjection {
    if capability.kind != ToolCapabilityKind::Mcp {
        return wrong_kind_projection(
            ToolCapabilityModule::Mcp,
            ToolCapabilitySourceRole::McpNonActionableEvidence,
            capability,
        );
    }
    if capability.scope == ToolCapabilityScope::Project {
        return non_actionable_projection(
            ToolCapabilityModule::Mcp,
            ToolCapabilitySourceRole::McpProjectConfig,
            capability,
            ToolCapabilityExclusionReason::ProjectScoped,
        );
    }
    if capability.access == ToolCapabilityAccess::Unsupported {
        return non_actionable_projection(
            ToolCapabilityModule::Mcp,
            ToolCapabilitySourceRole::McpNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::Unsupported,
        );
    }
    if capability.access == ToolCapabilityAccess::Unknown {
        return non_actionable_projection(
            ToolCapabilityModule::Mcp,
            ToolCapabilitySourceRole::McpNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::Unknown,
        );
    }
    if !matches!(
        capability.format,
        ToolCapabilityFormat::Json
            | ToolCapabilityFormat::Jsonc
            | ToolCapabilityFormat::Toml
            | ToolCapabilityFormat::Yaml
    ) {
        return non_actionable_projection(
            ToolCapabilityModule::Mcp,
            ToolCapabilitySourceRole::McpGlobalConfig,
            capability,
            ToolCapabilityExclusionReason::UnsupportedFormat,
        );
    }
    if capability.is_writable() || has_verified_action(capability, &ToolCapabilityAction::Save) {
        return ToolCapabilityProjection {
            module: ToolCapabilityModule::Mcp,
            source_role: ToolCapabilitySourceRole::McpGlobalConfig,
            actions: explicit_supported_actions(capability).unwrap_or_else(|| {
                vec![
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                    ToolCapabilityAction::Edit,
                    ToolCapabilityAction::Save,
                    ToolCapabilityAction::Sync,
                ]
            }),
            exclusion_reason: None,
            evidence: capability.clone(),
        };
    }
    non_actionable_projection(
        ToolCapabilityModule::Mcp,
        ToolCapabilitySourceRole::McpGlobalConfig,
        capability,
        ToolCapabilityExclusionReason::ReadOnly,
    )
}

fn project_ordinary_config_capability(capability: &ToolCapability) -> ToolCapabilityProjection {
    if capability.kind != ToolCapabilityKind::OrdinaryConfig {
        return wrong_kind_projection(
            ToolCapabilityModule::OrdinaryConfig,
            ToolCapabilitySourceRole::OrdinaryConfigNonActionableEvidence,
            capability,
        );
    }
    if capability.source_kind == ToolCapabilitySourceKind::PrimaryConfigDirectory {
        return non_actionable_projection(
            ToolCapabilityModule::OrdinaryConfig,
            ToolCapabilitySourceRole::OrdinaryConfigNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::OutOfScope,
        );
    }
    if capability.access == ToolCapabilityAccess::Unsupported {
        return non_actionable_projection(
            ToolCapabilityModule::OrdinaryConfig,
            ToolCapabilitySourceRole::OrdinaryConfigNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::Unsupported,
        );
    }
    if capability.access == ToolCapabilityAccess::Unknown {
        return non_actionable_projection(
            ToolCapabilityModule::OrdinaryConfig,
            ToolCapabilitySourceRole::OrdinaryConfigNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::Unknown,
        );
    }
    if capability.scope == ToolCapabilityScope::Project {
        return non_actionable_projection(
            ToolCapabilityModule::OrdinaryConfig,
            ToolCapabilitySourceRole::OrdinaryConfigNonActionableEvidence,
            capability,
            ToolCapabilityExclusionReason::ProjectScoped,
        );
    }
    if !matches!(
        capability.format,
        ToolCapabilityFormat::Json
            | ToolCapabilityFormat::Jsonc
            | ToolCapabilityFormat::Toml
            | ToolCapabilityFormat::Yaml
    ) {
        return non_actionable_projection(
            ToolCapabilityModule::OrdinaryConfig,
            ToolCapabilitySourceRole::OrdinaryConfigFile,
            capability,
            ToolCapabilityExclusionReason::UnsupportedFormat,
        );
    }
    if capability.is_writable() || has_verified_action(capability, &ToolCapabilityAction::Save) {
        return ToolCapabilityProjection {
            module: ToolCapabilityModule::OrdinaryConfig,
            source_role: ToolCapabilitySourceRole::OrdinaryConfigFile,
            actions: explicit_supported_actions(capability).unwrap_or_else(|| {
                vec![
                    ToolCapabilityAction::View,
                    ToolCapabilityAction::Read,
                    ToolCapabilityAction::Diagnose,
                    ToolCapabilityAction::Edit,
                    ToolCapabilityAction::Save,
                    ToolCapabilityAction::Sync,
                ]
            }),
            exclusion_reason: None,
            evidence: capability.clone(),
        };
    }
    non_actionable_projection(
        ToolCapabilityModule::OrdinaryConfig,
        ToolCapabilitySourceRole::OrdinaryConfigFile,
        capability,
        ToolCapabilityExclusionReason::ReadOnly,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::tool_capabilities::{
        tool_capability, ToolCapabilityActionEvidence, ToolCapabilitySourceConfidence,
        ToolCapabilitySourceKind, ToolCapabilitySupportingSource,
        ToolCapabilitySupportingSourceRole, ToolSourceDiagnosticState,
    };

    fn capability(
        kind: ToolCapabilityKind,
        scope: ToolCapabilityScope,
        access: ToolCapabilityAccess,
        format: ToolCapabilityFormat,
        source_path: &str,
    ) -> ToolCapability {
        tool_capability(
            "source",
            kind,
            scope,
            access,
            format,
            source_path,
            "Source",
            ToolCapabilitySourceConfidence::OfficialDocs,
            "test source",
        )
    }

    fn unsupported(kind: ToolCapabilityKind) -> ToolCapability {
        ToolCapability {
            id: "unsupported".to_string(),
            kind,
            scope: ToolCapabilityScope::Global,
            access: ToolCapabilityAccess::Unsupported,
            format: ToolCapabilityFormat::Unknown,
            source_path: String::new(),
            label: "Unsupported".to_string(),
            diagnostics: vec![ToolSourceDiagnosticState::Unsupported],
            source_confidence: ToolCapabilitySourceConfidence::Unknown,
            notes: "unsupported".to_string(),
            source_kind: ToolCapabilitySourceKind::FeatureSource,
            primary_config_dir: None,
            supporting_sources: vec![],
            action_evidence: vec![],
        }
    }

    fn supported_action(action: ToolCapabilityAction) -> ToolCapabilityActionEvidence {
        ToolCapabilityActionEvidence {
            action,
            supported: true,
            evidence: "verified in test".to_string(),
            variant: Some("Test Variant".to_string()),
            version: Some("1.0.0".to_string()),
            verified_at: Some("2026-05-15".to_string()),
        }
    }

    fn metadata_source() -> ToolCapabilitySupportingSource {
        ToolCapabilitySupportingSource {
            id: "metadata".to_string(),
            role: ToolCapabilitySupportingSourceRole::Metadata,
            source_path: "/tmp/skills.json".to_string(),
            format: ToolCapabilityFormat::Json,
            required: true,
            diagnostics: vec![ToolSourceDiagnosticState::Missing],
            notes: "test metadata".to_string(),
        }
    }

    #[test]
    fn writable_project_rule_does_not_allow_global_injection() {
        let source = capability(
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Project,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Markdown,
            ".cursor/rules/project.mdc",
        );

        let projection = project_capability("cursor", ToolCapabilityModule::Rules, &source);

        assert_eq!(
            projection.source_role,
            ToolCapabilitySourceRole::RuleProjectSource
        );
        assert_eq!(
            projection.exclusion_reason,
            Some(ToolCapabilityExclusionReason::ProjectScoped)
        );
        assert!(!projection.allows(&ToolCapabilityAction::Inject));
    }

    #[test]
    fn writable_global_rule_requires_trusted_rule_projection() {
        let source = capability(
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Markdown,
            "~/.codex/AGENTS.md",
        );

        let projection = project_capability("codex", ToolCapabilityModule::Rules, &source);

        assert_eq!(
            projection.source_role,
            ToolCapabilitySourceRole::RuleGlobalTarget
        );
        assert_eq!(projection.exclusion_reason, None);
        assert!(projection.allows(&ToolCapabilityAction::Inject));
    }

    #[test]
    fn writable_global_rule_directory_projects_as_native_source_without_injection() {
        let source = capability(
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Directory,
            "~/.claude/rules",
        );

        let projection = project_capability("claude-code", ToolCapabilityModule::Rules, &source);

        assert_eq!(
            projection.source_role,
            ToolCapabilitySourceRole::RuleNativeFileSource
        );
        assert_eq!(projection.exclusion_reason, None);
        assert!(projection.allows(&ToolCapabilityAction::Create));
        assert!(projection.allows(&ToolCapabilityAction::Save));
        assert!(!projection.allows(&ToolCapabilityAction::Inject));
    }

    #[test]
    fn read_only_global_rule_file_projects_as_native_source_without_injection() {
        let source = capability(
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::Markdown,
            "~/.codeium/windsurf/memories/global_rules.md",
        );

        let projection = project_capability("windsurf", ToolCapabilityModule::Rules, &source);

        assert_eq!(
            projection.source_role,
            ToolCapabilitySourceRole::RuleNativeFileSource
        );
        assert_eq!(projection.exclusion_reason, None);
        assert!(projection.allows(&ToolCapabilityAction::Read));
        assert!(!projection.allows(&ToolCapabilityAction::Create));
        assert!(!projection.allows(&ToolCapabilityAction::Inject));
    }

    #[test]
    fn writable_project_mcp_does_not_allow_global_save() {
        let source = capability(
            ToolCapabilityKind::Mcp,
            ToolCapabilityScope::Project,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            ".cursor/mcp.json",
        );

        let projection = project_capability("cursor", ToolCapabilityModule::Mcp, &source);

        assert_eq!(
            projection.source_role,
            ToolCapabilitySourceRole::McpProjectConfig
        );
        assert_eq!(
            projection.exclusion_reason,
            Some(ToolCapabilityExclusionReason::ProjectScoped)
        );
        assert!(!projection.allows(&ToolCapabilityAction::Save));
    }

    #[test]
    fn windsurf_global_mcp_and_ordinary_config_boundaries_project_separately() {
        let mcp_source = capability(
            ToolCapabilityKind::Mcp,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            "~/.codeium/windsurf/mcp_config.json",
        );
        let ordinary_config = unsupported(ToolCapabilityKind::OrdinaryConfig);

        let mcp_projection = project_capability("windsurf", ToolCapabilityModule::Mcp, &mcp_source);
        let ordinary_projection = project_capability(
            "windsurf",
            ToolCapabilityModule::OrdinaryConfig,
            &ordinary_config,
        );

        assert_eq!(
            mcp_projection.source_role,
            ToolCapabilitySourceRole::McpGlobalConfig
        );
        assert_eq!(mcp_projection.exclusion_reason, None);
        assert!(mcp_projection.allows(&ToolCapabilityAction::Save));
        assert_eq!(
            ordinary_projection.source_role,
            ToolCapabilitySourceRole::OrdinaryConfigNonActionableEvidence
        );
        assert_eq!(
            ordinary_projection.exclusion_reason,
            Some(ToolCapabilityExclusionReason::Unsupported)
        );
        assert!(!ordinary_projection.allows(&ToolCapabilityAction::Save));
    }

    #[test]
    fn writable_skill_projection_distinguishes_tool_directory_from_shared_source() {
        let tool_source = capability(
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            "~/.codex/skills",
        );
        let shared_source = capability(
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Shared,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::SkillDirectory,
            "~/.agents/skills",
        );

        let tool_projection =
            project_capability("codex", ToolCapabilityModule::Skills, &tool_source);
        let shared_projection =
            project_capability("codex", ToolCapabilityModule::Skills, &shared_source);

        assert_eq!(
            tool_projection.source_role,
            ToolCapabilitySourceRole::SkillToolDirectory
        );
        assert!(tool_projection.allows(&ToolCapabilityAction::Install));
        assert_eq!(
            shared_projection.source_role,
            ToolCapabilitySourceRole::SkillSharedSource
        );
        assert!(!shared_projection.allows(&ToolCapabilityAction::Delete));
    }

    #[test]
    fn action_evidence_separates_native_rule_reading_from_injection() {
        let mut source = capability(
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Readable,
            ToolCapabilityFormat::Directory,
            "~/.trae-cn/user_rules",
        );
        source.action_evidence = vec![
            supported_action(ToolCapabilityAction::Read),
            supported_action(ToolCapabilityAction::Diagnose),
        ];

        let projection = project_capability("trae-solo-cn", ToolCapabilityModule::Rules, &source);

        assert_eq!(
            projection.source_role,
            ToolCapabilitySourceRole::RuleNativeFileSource
        );
        assert!(projection.allows(&ToolCapabilityAction::Read));
        assert!(projection.allows(&ToolCapabilityAction::View));
        assert!(projection.allows(&ToolCapabilityAction::Diagnose));
        assert!(!projection.allows(&ToolCapabilityAction::Inject));
        assert_eq!(
            projection.exclusion_reason,
            Some(ToolCapabilityExclusionReason::ActionNotVerified)
        );
    }

    #[test]
    fn verified_native_rule_writes_still_do_not_enable_injection() {
        let mut source = capability(
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Directory,
            "~/.trae-cn/user_rules",
        );
        source.source_confidence = ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior;
        source.action_evidence = vec![
            supported_action(ToolCapabilityAction::Read),
            supported_action(ToolCapabilityAction::Create),
            supported_action(ToolCapabilityAction::Save),
            supported_action(ToolCapabilityAction::Delete),
        ];

        let projection = project_capability("trae-cn", ToolCapabilityModule::Rules, &source);

        assert_eq!(
            projection.source_role,
            ToolCapabilitySourceRole::RuleNativeFileSource
        );
        assert_eq!(projection.exclusion_reason, None);
        assert!(projection.allows(&ToolCapabilityAction::Create));
        assert!(projection.allows(&ToolCapabilityAction::Save));
        assert!(projection.allows(&ToolCapabilityAction::Delete));
        assert!(!projection.allows(&ToolCapabilityAction::Inject));
    }

    #[test]
    fn writable_tool_skill_directory_enables_shared_install_deployment() {
        let mut source = capability(
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            "~/.trae-cn/skills",
        );
        source.source_confidence = ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior;
        source.action_evidence = vec![
            supported_action(ToolCapabilityAction::Read),
            supported_action(ToolCapabilityAction::Copy),
            supported_action(ToolCapabilityAction::Delete),
            supported_action(ToolCapabilityAction::Save),
        ];

        let projection = project_capability("trae-cn", ToolCapabilityModule::Skills, &source);

        assert_eq!(
            projection.source_role,
            ToolCapabilitySourceRole::SkillToolDirectory
        );
        assert_eq!(projection.exclusion_reason, None);
        assert!(projection.allows(&ToolCapabilityAction::Copy));
        assert!(projection.allows(&ToolCapabilityAction::Delete));
        assert!(projection.allows(&ToolCapabilityAction::Save));
        assert!(projection.allows(&ToolCapabilityAction::Install));
        assert!(projection.allows(&ToolCapabilityAction::Link));
        assert!(projection.allows(&ToolCapabilityAction::Uninstall));
    }

    #[test]
    fn primary_config_directory_is_not_an_ordinary_config_file() {
        let mut source = capability(
            ToolCapabilityKind::OrdinaryConfig,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Readable,
            ToolCapabilityFormat::Directory,
            "~/.trae-cn",
        );
        source.source_kind = ToolCapabilitySourceKind::PrimaryConfigDirectory;

        let projection =
            project_capability("trae-cn", ToolCapabilityModule::OrdinaryConfig, &source);

        assert_eq!(
            projection.source_role,
            ToolCapabilitySourceRole::OrdinaryConfigNonActionableEvidence
        );
        assert_eq!(
            projection.exclusion_reason,
            Some(ToolCapabilityExclusionReason::OutOfScope)
        );
        assert!(!projection.allows(&ToolCapabilityAction::Save));
    }

    #[test]
    fn metadata_backed_skill_source_blocks_managed_actions_without_evidence() {
        let mut source = capability(
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            "~/.trae-cn/skills",
        );
        source.supporting_sources = vec![metadata_source()];
        source.source_kind = ToolCapabilitySourceKind::FeatureSource;
        source.action_evidence = vec![
            supported_action(ToolCapabilityAction::Read),
            supported_action(ToolCapabilityAction::Diagnose),
        ];

        let projection = project_capability("trae-solo-cn", ToolCapabilityModule::Skills, &source);

        assert_eq!(
            projection.source_role,
            ToolCapabilitySourceRole::SkillCompoundSource
        );
        assert!(projection.allows(&ToolCapabilityAction::Read));
        assert!(!projection.allows(&ToolCapabilityAction::Install));
        assert!(!projection.allows(&ToolCapabilityAction::Delete));
        assert!(!projection.allows(&ToolCapabilityAction::Update));
        assert_eq!(
            projection.exclusion_reason,
            Some(ToolCapabilityExclusionReason::CompoundSourceRequiresVerifiedAction)
        );
    }

    #[test]
    fn metadata_backed_skill_source_without_action_evidence_stays_non_actionable() {
        let mut source = capability(
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::SkillDirectory,
            "~/.trae-cn/skills",
        );
        source.supporting_sources = vec![metadata_source()];
        source.source_kind = ToolCapabilitySourceKind::FeatureSource;

        let projection = project_capability("trae-solo-cn", ToolCapabilityModule::Skills, &source);

        assert_eq!(
            projection.source_role,
            ToolCapabilitySourceRole::SkillCompoundSource
        );
        assert!(!projection.allows(&ToolCapabilityAction::Install));
        assert!(!projection.allows(&ToolCapabilityAction::Delete));
        assert_eq!(
            projection.exclusion_reason,
            Some(ToolCapabilityExclusionReason::CompoundSourceRequiresVerifiedAction)
        );
    }

    #[test]
    fn representative_projection_fixture_preserves_non_actionable_states() {
        let fixtures = vec![
            capability(
                ToolCapabilityKind::Rule,
                ToolCapabilityScope::Global,
                ToolCapabilityAccess::Writable,
                ToolCapabilityFormat::Markdown,
                "~/.codex/AGENTS.md",
            ),
            capability(
                ToolCapabilityKind::Rule,
                ToolCapabilityScope::Project,
                ToolCapabilityAccess::Writable,
                ToolCapabilityFormat::Markdown,
                ".cursor/rules/project.mdc",
            ),
            capability(
                ToolCapabilityKind::Skill,
                ToolCapabilityScope::Tool,
                ToolCapabilityAccess::Writable,
                ToolCapabilityFormat::SkillDirectory,
                "~/.codex/skills",
            ),
            capability(
                ToolCapabilityKind::Skill,
                ToolCapabilityScope::Shared,
                ToolCapabilityAccess::ReadOnly,
                ToolCapabilityFormat::SkillDirectory,
                "~/.agents/skills",
            ),
            capability(
                ToolCapabilityKind::Mcp,
                ToolCapabilityScope::Global,
                ToolCapabilityAccess::Writable,
                ToolCapabilityFormat::Json,
                "~/.cursor/mcp.json",
            ),
            capability(
                ToolCapabilityKind::Mcp,
                ToolCapabilityScope::Global,
                ToolCapabilityAccess::ReadOnly,
                ToolCapabilityFormat::Json,
                "~/.gemini/settings.json#mcpServers",
            ),
            capability(
                ToolCapabilityKind::Mcp,
                ToolCapabilityScope::Global,
                ToolCapabilityAccess::Unknown,
                ToolCapabilityFormat::Unknown,
                "",
            ),
            unsupported(ToolCapabilityKind::Skill),
        ];

        let rule_projections =
            project_capabilities("codex", ToolCapabilityModule::Rules, &fixtures);
        let skill_projections =
            project_capabilities("codex", ToolCapabilityModule::Skills, &fixtures);
        let mcp_projections = project_capabilities("codex", ToolCapabilityModule::Mcp, &fixtures);

        assert!(rule_projections
            .iter()
            .any(|projection| projection.allows(&ToolCapabilityAction::Inject)));
        assert!(skill_projections.iter().any(
            |projection| projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
        ));
        assert!(mcp_projections.iter().any(|projection| {
            projection.exclusion_reason == Some(ToolCapabilityExclusionReason::Unknown)
        }));
        assert!(skill_projections.iter().any(|projection| {
            projection.exclusion_reason == Some(ToolCapabilityExclusionReason::Unsupported)
        }));
    }
}
