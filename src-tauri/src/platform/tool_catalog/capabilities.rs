// Purpose: Convert catalog capability definitions into runtime adapter capability records.

use super::types::{CapabilityDiagnostics, ToolCapabilityDefinition, ToolDefinition};
use crate::platform::tool_adapters::declared as declared_tool;
use crate::platform::tool_capabilities::{
    ToolCapability, ToolCapabilityAction, ToolCapabilityActionEvidence, ToolCapabilitySourceKind,
    ToolSourceDiagnosticState,
};
use std::path::Path;

fn diagnostics(kind: CapabilityDiagnostics) -> Vec<ToolSourceDiagnosticState> {
    match kind {
        CapabilityDiagnostics::File => declared_tool::file_diagnostics(),
        CapabilityDiagnostics::Directory => declared_tool::directory_diagnostics(),
        CapabilityDiagnostics::Unsupported => vec![ToolSourceDiagnosticState::Unsupported],
        CapabilityDiagnostics::Unknown => vec![ToolSourceDiagnosticState::Unknown],
    }
}

pub(crate) fn capability_from_definition(
    tool_id: &str,
    definition: &ToolCapabilityDefinition,
    home: &Path,
) -> ToolCapability {
    let mut capability = ToolCapability {
        id: definition.id.to_string(),
        kind: definition.kind.clone(),
        scope: definition.scope.clone(),
        access: definition.access.clone(),
        format: definition.format.clone(),
        source_path: definition.source_path.resolve_string(home),
        label: definition.label.to_string(),
        diagnostics: diagnostics(definition.diagnostics),
        source_confidence: definition.source_confidence.clone(),
        notes: definition.notes.to_string(),
        source_kind: ToolCapabilitySourceKind::FeatureSource,
        primary_config_dir: None,
        supporting_sources: vec![],
        action_evidence: vec![],
    };
    enrich_capability(tool_id, &mut capability, home);
    capability
}

pub(crate) fn declared_capabilities_for_tool(
    definition: &ToolDefinition,
    home: &Path,
) -> Vec<declared_tool::DeclaredCapability> {
    definition
        .capabilities
        .iter()
        .map(|definition_capability| {
            let capability = capability_from_definition(definition.id, definition_capability, home);
            declared_tool::DeclaredCapability {
                id: definition_capability.id,
                kind: capability.kind,
                scope: capability.scope,
                access: capability.access,
                format: capability.format,
                source_path: capability.source_path,
                label: definition_capability.label,
                diagnostics: capability.diagnostics,
                source_confidence: capability.source_confidence,
                notes: definition_capability.notes,
                source_kind: capability.source_kind,
                primary_config_dir: capability.primary_config_dir,
                supporting_sources: capability.supporting_sources,
                action_evidence: capability.action_evidence,
            }
        })
        .collect()
}

fn verified_actions(
    feature: &str,
    source: &str,
    variant: &str,
    version: Option<&str>,
    verified_at: &str,
    actions: &[ToolCapabilityAction],
) -> Vec<ToolCapabilityActionEvidence> {
    actions
        .iter()
        .map(|action| ToolCapabilityActionEvidence {
            action: action.clone(),
            supported: true,
            evidence: format!("{feature} action {:?} verified from {source}.", action),
            variant: Some(variant.to_string()),
            version: version.map(ToString::to_string),
            verified_at: Some(verified_at.to_string()),
        })
        .collect()
}

fn enrich_capability(tool_id: &str, capability: &mut ToolCapability, _home: &Path) {
    match tool_id {
        "opencode" => enrich_opencode_capability(capability),
        _ => {}
    }
}

fn enrich_opencode_capability(capability: &mut ToolCapability) {
    if capability.id == "global-rule" {
        capability.action_evidence = verified_actions(
            "OpenCode global AGENTS.md",
            "~/.config/opencode/AGENTS.md",
            "OpenCode CLI",
            None,
            "2026-05-21",
            &[
                ToolCapabilityAction::View,
                ToolCapabilityAction::Read,
                ToolCapabilityAction::Diagnose,
                ToolCapabilityAction::Edit,
                ToolCapabilityAction::Save,
                ToolCapabilityAction::Inject,
            ],
        );
    }
}
