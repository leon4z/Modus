// Purpose: Derive and remediate managed Rules state for configured tool targets.

use super::*;
use crate::adapters::{
    capability_matches_eligible_global_rule_target, ToolCapabilityAccess, ToolRegistry,
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ManagedBlockParse {
    Missing,
    Present(String),
    Malformed(String),
}

#[derive(Debug, Clone)]
struct RuleCapabilityState {
    can_read: bool,
    can_write: bool,
    reason: Option<ManagedRuleTargetReason>,
}

fn unique_sorted<I>(items: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut values: Vec<String> = items
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect();
    values.sort();
    values.dedup();
    values
}

pub(crate) fn all_rule_tool_ids(
    registry: &ToolRegistry,
    config: &app_config::AppConfig,
) -> Vec<String> {
    let registry_ids = registry.tool_ids();
    if config.initialized {
        return unique_sorted(
            config
                .managed_tools
                .iter()
                .map(|id| canonical_tool_id(id, &registry_ids))
                .collect::<Vec<_>>(),
        );
    }

    let mut ids: BTreeSet<String> = registry.tool_ids().into_iter().collect();
    ids.extend(
        config
            .tool_capability_overrides
            .keys()
            .map(|id| canonical_tool_id(id, &registry_ids)),
    );
    ids.extend(
        config
            .managed_tools
            .iter()
            .map(|id| canonical_tool_id(id, &registry_ids)),
    );
    for rule in &config.default_rules {
        ids.extend(
            rule.inject_to
                .iter()
                .map(|id| canonical_tool_id(id, &registry_ids)),
        );
        if let Some(targets) = &rule.managed_targets {
            ids.extend(
                targets
                    .iter()
                    .map(|id| canonical_tool_id(id, &registry_ids)),
            );
        }
    }
    ids.into_iter().collect()
}

fn canonical_active_rule_tool_id(
    registry: &ToolRegistry,
    config: &app_config::AppConfig,
    tool_id: &str,
) -> Result<String, String> {
    let registry_ids = registry.tool_ids();
    let canonical = canonical_tool_id(tool_id, &registry_ids);
    if all_rule_tool_ids(registry, config).contains(&canonical) {
        Ok(canonical)
    } else {
        Err(format!("Tool is not enabled for management: {}", tool_id))
    }
}

fn tool_name_map(registry: &ToolRegistry) -> HashMap<String, String> {
    registry
        .detect_all()
        .into_iter()
        .map(|tool| (tool.id, tool.name))
        .collect()
}

pub(crate) fn extract_managed_block(existing: &str) -> ManagedBlockParse {
    let start_indices: Vec<usize> = existing
        .match_indices(MARKER_START)
        .map(|(idx, _)| idx)
        .collect();
    let end_indices: Vec<usize> = existing
        .match_indices(MARKER_END)
        .map(|(idx, _)| idx)
        .collect();

    if start_indices.is_empty() && end_indices.is_empty() {
        return ManagedBlockParse::Missing;
    }
    if start_indices.len() != 1 || end_indices.len() != 1 {
        return ManagedBlockParse::Malformed("duplicate or incomplete managed markers".to_string());
    }

    let start = start_indices[0];
    let end_marker_start = end_indices[0];
    if end_marker_start < start {
        return ManagedBlockParse::Malformed(
            "managed end marker appears before start marker".to_string(),
        );
    }
    let end = end_marker_start + MARKER_END.len();
    ManagedBlockParse::Present(existing[start..end].to_string())
}

fn rule_fingerprint(rule: &app_config::DefaultRule) -> String {
    let mut inject_to = rule.inject_to.clone();
    inject_to.sort();
    let mut managed_targets = rule.managed_targets.clone();
    if let Some(targets) = &mut managed_targets {
        targets.sort();
    }
    let managed_targets_json = managed_targets
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .unwrap_or_else(|_| Some("null".to_string()))
        .unwrap_or_else(|| "null".to_string());
    let payload = format!(
        "{{\"id\":{},\"name\":{},\"content\":{},\"inject_to\":{},\"managed_targets\":{}}}",
        serde_json::to_string(&rule.id).unwrap_or_else(|_| "\"\"".to_string()),
        serde_json::to_string(&rule.name).unwrap_or_else(|_| "\"\"".to_string()),
        serde_json::to_string(&rule.content).unwrap_or_else(|_| "\"\"".to_string()),
        serde_json::to_string(&inject_to).unwrap_or_else(|_| "[]".to_string()),
        managed_targets_json,
    );
    format!("v1:{:08x}", fnv1a32(&payload))
}

fn scoped_rule_for_fingerprint(
    rule: &app_config::DefaultRule,
    all_tool_ids: &[String],
) -> app_config::DefaultRule {
    let mut scoped = rule.clone();
    if scoped.managed_targets.is_some() {
        scoped.managed_targets = Some(managed_rule_tool_ids(rule, all_tool_ids));
    } else if !scoped.inject_to.is_empty() {
        scoped.inject_to = managed_rule_tool_ids(rule, all_tool_ids);
    }
    scoped
}

fn active_pending_rule_targets(
    rule_id: &str,
    baselines: &app_config::DefaultRuleInjectionBaselines,
    all_tool_ids: &[String],
) -> Vec<String> {
    let active: HashSet<&str> = all_tool_ids.iter().map(String::as_str).collect();
    unique_sorted(
        baselines
            .custom_rule_pending_targets
            .get(rule_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|target| canonical_tool_id(&target, all_tool_ids))
            .filter(|target| active.contains(target.as_str()))
            .collect::<Vec<_>>(),
    )
}

fn persisted_rule_relationship_tool_ids(
    rule: &app_config::DefaultRule,
    all_tool_ids: &[String],
) -> Vec<String> {
    if let Some(targets) = &rule.managed_targets {
        return unique_sorted(
            targets
                .iter()
                .map(|target| canonical_tool_id(target, all_tool_ids))
                .collect::<Vec<_>>(),
        );
    }
    if !rule.inject_to.is_empty() {
        return unique_sorted(
            rule.inject_to
                .iter()
                .map(|target| canonical_tool_id(target, all_tool_ids))
                .collect::<Vec<_>>(),
        );
    }
    all_tool_ids.to_vec()
}

fn fnv1a32(input: &str) -> u32 {
    let mut hash: u32 = 2166136261;
    for byte in input.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

fn source_pending_for_rule(
    rule: &app_config::DefaultRule,
    baselines: &app_config::DefaultRuleInjectionBaselines,
    all_tool_ids: &[String],
) -> bool {
    if rule.id == "common_rule" {
        let raw = rule_fingerprint(rule);
        let scoped = rule_fingerprint(&scoped_rule_for_fingerprint(rule, all_tool_ids));
        return baselines.common_rule != raw && baselines.common_rule != scoped;
    }
    let raw = rule_fingerprint(rule);
    let scoped = rule_fingerprint(&scoped_rule_for_fingerprint(rule, all_tool_ids));
    baselines
        .custom_rules
        .get(&rule.id)
        .map(|fingerprint| fingerprint != &raw && fingerprint != &scoped)
        .unwrap_or(true)
        || !active_pending_rule_targets(&rule.id, baselines, all_tool_ids).is_empty()
}

fn pending_deleted_rule_targets(
    config: &app_config::AppConfig,
    all_tool_ids: &[String],
) -> BTreeMap<String, Vec<String>> {
    let current_rule_ids: HashSet<String> = config
        .default_rules
        .iter()
        .map(|rule| rule.id.clone())
        .collect();
    let mut pending = BTreeMap::new();
    if !config
        .default_rule_injection_baselines
        .common_rule
        .is_empty()
        && !current_rule_ids.contains("common_rule")
    {
        pending.insert("common_rule".to_string(), all_tool_ids.to_vec());
    }
    pending
}

fn capability_state_for_effective_target(
    registry: &ToolRegistry,
    target: &EffectiveGlobalRuleTarget,
    target_path: &Path,
    config: &app_config::AppConfig,
) -> RuleCapabilityState {
    let Some((adapter_id, base_capabilities)) =
        rule_target_base_capabilities(registry, &target.tool_id, config)
    else {
        return RuleCapabilityState {
            can_read: false,
            can_write: false,
            reason: Some(ManagedRuleTargetReason::UnknownSupport),
        };
    };
    let capabilities = effective_capabilities_for_global_rule_target(
        &adapter_id,
        &base_capabilities,
        config,
        target,
    );
    let matching =
        global_rule_target_capabilities_for_target(&adapter_id, &capabilities, target_path);
    if matching.is_empty() {
        return RuleCapabilityState {
            can_read: false,
            can_write: false,
            reason: Some(ManagedRuleTargetReason::UnknownSupport),
        };
    }
    if matching
        .iter()
        .any(|capability| capability.access == ToolCapabilityAccess::Unsupported)
    {
        return RuleCapabilityState {
            can_read: false,
            can_write: false,
            reason: Some(ManagedRuleTargetReason::UnsupportedTarget),
        };
    }
    let can_read = matching.iter().any(|capability| capability.is_readable());
    let can_write = matching.iter().any(|capability| capability.is_writable());
    let reason = if !can_read {
        Some(ManagedRuleTargetReason::UnknownSupport)
    } else if !can_write {
        Some(ManagedRuleTargetReason::ReadOnlyTarget)
    } else {
        None
    };
    RuleCapabilityState {
        can_read,
        can_write,
        reason,
    }
}

pub(crate) fn global_rule_target_capabilities_for_target(
    adapter_id: &str,
    capabilities: &[crate::adapters::ToolCapability],
    target_path: &Path,
) -> Vec<crate::adapters::ToolCapability> {
    crate::adapters::capability_projections::project_capabilities(
        adapter_id,
        crate::adapters::capability_projections::ToolCapabilityModule::Rules,
        capabilities,
    )
    .into_iter()
    .filter(|projection| {
        projection.source_role
            == crate::adapters::capability_projections::ToolCapabilitySourceRole::RuleGlobalTarget
            && projection
                .allows(&crate::adapters::capability_projections::ToolCapabilityAction::Inject)
            && capability_matches_eligible_global_rule_target(
                adapter_id,
                &projection.evidence,
                target_path,
            )
    })
    .map(|projection| projection.evidence)
    .collect()
}

fn target_is_eligible_effective_global_rule_target(
    registry: &ToolRegistry,
    target: &EffectiveGlobalRuleTarget,
    target_path: &Path,
    config: &app_config::AppConfig,
) -> bool {
    rule_target_base_capabilities(registry, &target.tool_id, config)
        .map(|(adapter_id, base_capabilities)| {
            let capabilities = effective_capabilities_for_global_rule_target(
                &adapter_id,
                &base_capabilities,
                config,
                target,
            );
            !global_rule_target_capabilities_for_target(&adapter_id, &capabilities, target_path)
                .is_empty()
        })
        .unwrap_or(false)
}

fn rule_target_base_capabilities(
    registry: &ToolRegistry,
    tool_id: &str,
    config: &app_config::AppConfig,
) -> Option<(String, Vec<crate::adapters::ToolCapability>)> {
    if let Some(adapter) = registry.get_adapter(tool_id) {
        return Some((adapter.id().to_string(), adapter.capabilities()));
    }
    let canonical = crate::platform::tool_catalog::normalization::canonical_tool_id(tool_id);
    registry
        .detect_all_for_config(config)
        .into_iter()
        .find(|tool| tool.id == canonical && tool.detected)
        .map(|tool| (tool.id, tool.capabilities))
}

fn classify_target(
    expected_block: Option<String>,
    current: ManagedBlockParse,
    source_pending: bool,
    capability: &RuleCapabilityState,
) -> (
    ManagedRuleTargetClassification,
    ManagedRuleTargetReason,
    bool,
    Option<String>,
) {
    if let Some(reason) = &capability.reason {
        if !capability.can_read {
            return (
                ManagedRuleTargetClassification::Unresolved,
                reason.clone(),
                false,
                Some(
                    "Rule target support is not readable from the declared capability".to_string(),
                ),
            );
        }
    }

    match (expected_block.as_ref(), current) {
        (_, ManagedBlockParse::Malformed(message)) => (
            ManagedRuleTargetClassification::Unresolved,
            ManagedRuleTargetReason::MalformedMarkers,
            true,
            Some(message),
        ),
        (Some(expected), ManagedBlockParse::Present(current_block))
            if &current_block == expected =>
        {
            if source_pending {
                (
                    ManagedRuleTargetClassification::RequiresSync,
                    ManagedRuleTargetReason::PendingSource,
                    true,
                    None,
                )
            } else {
                (
                    ManagedRuleTargetClassification::InSync,
                    ManagedRuleTargetReason::InSync,
                    true,
                    None,
                )
            }
        }
        (Some(_), ManagedBlockParse::Present(_)) => {
            if source_pending {
                (
                    ManagedRuleTargetClassification::Drifted,
                    ManagedRuleTargetReason::PendingSource,
                    true,
                    None,
                )
            } else {
                (
                    ManagedRuleTargetClassification::Drifted,
                    ManagedRuleTargetReason::ManagedBlockDrift,
                    true,
                    None,
                )
            }
        }
        (Some(_), ManagedBlockParse::Missing) => (
            ManagedRuleTargetClassification::RequiresSync,
            ManagedRuleTargetReason::MissingManagedBlock,
            false,
            None,
        ),
        (None, ManagedBlockParse::Present(_)) => (
            ManagedRuleTargetClassification::RequiresSync,
            ManagedRuleTargetReason::StaleManagedBlock,
            true,
            None,
        ),
        (None, ManagedBlockParse::Missing) => (
            ManagedRuleTargetClassification::InSync,
            ManagedRuleTargetReason::NoManagedRelation,
            false,
            None,
        ),
    }
}

pub(crate) fn get_managed_rules_state_domain(registry: &ToolRegistry) -> ManagedRulesState {
    let config = app_config::load_config();
    get_managed_rules_state_for_config(registry, &config)
}

pub(crate) fn get_managed_rules_state_for_config(
    registry: &ToolRegistry,
    config: &app_config::AppConfig,
) -> ManagedRulesState {
    let mut normalized_config = config.clone();
    crate::platform::tool_catalog::normalization::normalize_config(&mut normalized_config);
    let config = &normalized_config;
    let all_tool_ids = all_rule_tool_ids(registry, config);
    let tool_names = tool_name_map(registry);
    let mut rule_sets = vec![];
    let mut target_rule_ids: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut target_rule_names: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut target_source_pending: BTreeMap<String, bool> = BTreeMap::new();
    let community_rules = community_default_rules(&config.default_rules);

    for rule in &community_rules {
        let managed_tool_ids = managed_rule_tool_ids(rule, &all_tool_ids);
        let source_pending = source_pending_for_rule(
            rule,
            &config.default_rule_injection_baselines,
            &all_tool_ids,
        );
        if source_pending {
            for tool_id in &managed_tool_ids {
                target_source_pending.insert(tool_id.clone(), true);
            }
        }
        for tool_id in &managed_tool_ids {
            target_rule_ids
                .entry(tool_id.clone())
                .or_default()
                .insert(rule.id.clone());
            target_rule_names
                .entry(tool_id.clone())
                .or_default()
                .insert(rule.name.clone());
        }
        rule_sets.push(ManagedRuleSetState {
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            managed_tool_ids,
            source_pending,
        });
    }

    for (_rule_id, targets) in pending_deleted_rule_targets(config, &all_tool_ids) {
        for tool_id in targets {
            target_source_pending.insert(tool_id, true);
        }
    }

    let mut target_ids: BTreeSet<String> = target_rule_ids.keys().cloned().collect();
    target_ids.extend(target_source_pending.keys().cloned());

    let mut target_path_groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut effective_targets: BTreeMap<String, EffectiveGlobalRuleTarget> = BTreeMap::new();
    for tool_id in &target_ids {
        if let Some(target) = resolve_effective_global_rule_target(registry, config, tool_id) {
            target_path_groups
                .entry(target.expanded_key())
                .or_default()
                .push(target.tool_id.clone());
            effective_targets.insert(target.tool_id.clone(), target);
        }
    }

    let mut targets = vec![];
    for tool_id in target_ids {
        let Some(target) = effective_targets.get(&tool_id).cloned() else {
            continue;
        };
        let group_tool_ids = target_path_groups
            .get(&target.expanded_key())
            .cloned()
            .unwrap_or_else(|| vec![tool_id.clone()]);
        let expected_block =
            build_managed_rules_block_for_tools(&community_rules, &group_tool_ids, &all_tool_ids);
        let source_pending = target_source_pending
            .get(&tool_id)
            .copied()
            .unwrap_or(false);
        let target_path_raw = target.raw_path.clone();

        let expanded = shellexpand::tilde(&target_path_raw).to_string();
        let target_path_buf = PathBuf::from(&expanded);
        if !target_is_eligible_effective_global_rule_target(
            registry,
            &target,
            &target_path_buf,
            config,
        ) {
            continue;
        }
        let capability =
            capability_state_for_effective_target(registry, &target, &target_path_buf, config);
        if !capability.can_read {
            targets.push(ManagedRuleTargetState {
                tool_id: tool_id.clone(),
                tool_name: tool_names
                    .get(&tool_id)
                    .cloned()
                    .unwrap_or_else(|| tool_id.clone()),
                target_path: Some(target_path_raw),
                rule_set_ids: target_rule_ids
                    .get(&tool_id)
                    .map(|ids| ids.iter().cloned().collect())
                    .unwrap_or_default(),
                rule_set_names: target_rule_names
                    .get(&tool_id)
                    .map(|ids| ids.iter().cloned().collect())
                    .unwrap_or_default(),
                classification: ManagedRuleTargetClassification::Unresolved,
                reason: capability
                    .reason
                    .clone()
                    .unwrap_or(ManagedRuleTargetReason::UnknownSupport),
                can_read: false,
                can_write: false,
                source_pending,
                has_managed_block: false,
                expected_block,
                current_block: None,
                message: Some(
                    "Rule target support is not readable from the declared capability".to_string(),
                ),
            });
            continue;
        }
        if !target_path_buf.exists() && !capability.can_write {
            targets.push(ManagedRuleTargetState {
                tool_id: tool_id.clone(),
                tool_name: tool_names
                    .get(&tool_id)
                    .cloned()
                    .unwrap_or_else(|| tool_id.clone()),
                target_path: Some(target_path_raw),
                rule_set_ids: target_rule_ids
                    .get(&tool_id)
                    .map(|ids| ids.iter().cloned().collect())
                    .unwrap_or_default(),
                rule_set_names: target_rule_names
                    .get(&tool_id)
                    .map(|ids| ids.iter().cloned().collect())
                    .unwrap_or_default(),
                classification: ManagedRuleTargetClassification::Unresolved,
                reason: ManagedRuleTargetReason::FileMissing,
                can_read: capability.can_read,
                can_write: capability.can_write,
                source_pending,
                has_managed_block: false,
                expected_block,
                current_block: None,
                message: Some("Rule target file does not exist".to_string()),
            });
            continue;
        }
        let target_exists = target_path_buf.exists();
        let file_content = if target_exists {
            std::fs::read_to_string(&target_path_buf)
        } else {
            Ok(String::new())
        };
        let current = match file_content {
            Ok(content) => extract_managed_block(&content),
            Err(err) => {
                targets.push(ManagedRuleTargetState {
                    tool_id: tool_id.clone(),
                    tool_name: tool_names
                        .get(&tool_id)
                        .cloned()
                        .unwrap_or_else(|| tool_id.clone()),
                    target_path: Some(target_path_raw),
                    rule_set_ids: target_rule_ids
                        .get(&tool_id)
                        .map(|ids| ids.iter().cloned().collect())
                        .unwrap_or_default(),
                    rule_set_names: target_rule_names
                        .get(&tool_id)
                        .map(|ids| ids.iter().cloned().collect())
                        .unwrap_or_default(),
                    classification: ManagedRuleTargetClassification::Unresolved,
                    reason: ManagedRuleTargetReason::UnreadableTarget,
                    can_read: capability.can_read,
                    can_write: capability.can_write,
                    source_pending,
                    has_managed_block: false,
                    expected_block,
                    current_block: None,
                    message: Some(err.to_string()),
                });
                continue;
            }
        };
        let current_block = match &current {
            ManagedBlockParse::Present(block) => Some(block.clone()),
            _ => None,
        };
        let (classification, mut reason, has_managed_block, mut message) =
            classify_target(expected_block.clone(), current, source_pending, &capability);
        if !target_exists
            && classification == ManagedRuleTargetClassification::RequiresSync
            && reason == ManagedRuleTargetReason::MissingManagedBlock
            && capability.can_write
        {
            reason = ManagedRuleTargetReason::FileMissing;
            message = Some("Rule target file will be created during injection".to_string());
        }
        if classification != ManagedRuleTargetClassification::Unresolved && !capability.can_write {
            reason = ManagedRuleTargetReason::ReadOnlyTarget;
        }
        targets.push(ManagedRuleTargetState {
            tool_id: tool_id.clone(),
            tool_name: tool_names
                .get(&tool_id)
                .cloned()
                .unwrap_or_else(|| tool_id.clone()),
            target_path: Some(target_path_raw),
            rule_set_ids: target_rule_ids
                .get(&tool_id)
                .map(|ids| ids.iter().cloned().collect())
                .unwrap_or_default(),
            rule_set_names: target_rule_names
                .get(&tool_id)
                .map(|ids| ids.iter().cloned().collect())
                .unwrap_or_default(),
            classification,
            reason,
            can_read: capability.can_read,
            can_write: capability.can_write,
            source_pending,
            has_managed_block,
            expected_block,
            current_block,
            message,
        });
    }

    let unmanaged_tool_rule_count = registry
        .detect_all()
        .into_iter()
        .map(|tool| tool.rule_sources.len())
        .sum();
    let summary = summarize_managed_state(&rule_sets, &targets);
    ManagedRulesState {
        rule_sets,
        targets,
        unmanaged_tool_rule_count,
        summary,
    }
}

fn summarize_managed_state(
    rule_sets: &[ManagedRuleSetState],
    targets: &[ManagedRuleTargetState],
) -> ManagedRulesSummary {
    let mut summary = ManagedRulesSummary {
        managed_rule_sets: rule_sets.len(),
        managed_targets: targets.len(),
        pending_source_rule_sets: rule_sets.iter().filter(|rule| rule.source_pending).count(),
        ..ManagedRulesSummary::default()
    };
    let mut affected = BTreeSet::new();
    for target in targets {
        match target.classification {
            ManagedRuleTargetClassification::InSync => summary.in_sync_targets += 1,
            ManagedRuleTargetClassification::RequiresSync => {
                summary.requires_sync_targets += 1;
                affected.insert(target.tool_id.clone());
            }
            ManagedRuleTargetClassification::Drifted => {
                summary.drifted_targets += 1;
                affected.insert(target.tool_id.clone());
            }
            ManagedRuleTargetClassification::Unresolved => {
                summary.unresolved_targets += 1;
                affected.insert(target.tool_id.clone());
            }
            ManagedRuleTargetClassification::Unmanaged => {}
        }
    }
    summary.affected_tool_ids = affected.into_iter().collect();
    summary
}

pub(crate) fn adopt_rule_management_targets_domain(
    registry: &ToolRegistry,
    rule_id: String,
    tool_ids: Vec<String>,
) -> Result<ManagedRulesActionResult, String> {
    let requested = unique_sorted(tool_ids);
    let mut failed = vec![];
    let mut succeeded = vec![];
    let mut config = app_config::load_config();
    let all_tool_ids = all_rule_tool_ids(registry, &config);
    let rule_index = config
        .default_rules
        .iter()
        .position(|rule| rule.id == rule_id)
        .ok_or_else(|| format!("Rule set not found: {}", rule_id))?;
    for tool_id in &requested {
        match validate_writable_target(registry, &config, tool_id) {
            Ok(()) => succeeded.push(tool_id.clone()),
            Err(message) => failed.push(ManagedRulesActionFailure {
                tool_id: tool_id.clone(),
                message,
            }),
        }
    }
    if !succeeded.is_empty() {
        let mut targets =
            persisted_rule_relationship_tool_ids(&config.default_rules[rule_index], &all_tool_ids);
        targets.extend(
            succeeded
                .iter()
                .map(|tool_id| canonical_tool_id(tool_id, &all_tool_ids)),
        );
        config.default_rules[rule_index].managed_targets = Some(unique_sorted(targets));
        app_config::save_config(&config)?;
    }
    let state = get_managed_rules_state_for_config(registry, &config);
    Ok(ManagedRulesActionResult {
        requested_tool_ids: requested,
        succeeded_tool_ids: succeeded,
        failed,
        state,
    })
}

pub(crate) fn sync_managed_rule_targets_domain(
    registry: &ToolRegistry,
    tool_ids: Vec<String>,
) -> Result<ManagedRulesActionResult, String> {
    let requested = unique_sorted(tool_ids);
    let mut failed = vec![];
    let mut succeeded = vec![];
    let config = app_config::load_config();
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for tool_id in &requested {
        let canonical_tool_id = match canonical_active_rule_tool_id(registry, &config, tool_id) {
            Ok(canonical) => canonical,
            Err(message) => {
                failed.push(ManagedRulesActionFailure {
                    tool_id: tool_id.clone(),
                    message,
                });
                continue;
            }
        };
        if let Some(target) =
            resolve_effective_global_rule_target(registry, &config, &canonical_tool_id)
        {
            let expanded = shellexpand::tilde(&target.raw_path).to_string();
            let target_path = PathBuf::from(expanded);
            if !target_is_eligible_effective_global_rule_target(
                registry,
                &target,
                &target_path,
                &config,
            ) {
                continue;
            }
            groups
                .entry(target.expanded_key())
                .or_default()
                .push(target.tool_id);
        }
    }
    for group_tool_ids in groups.values_mut() {
        group_tool_ids.sort();
        group_tool_ids.dedup();
    }
    for group_tool_ids in groups.values() {
        let primary_tool_id = group_tool_ids[0].clone();
        match inject_default_rules_for_tools_domain(
            registry,
            primary_tool_id.clone(),
            group_tool_ids.clone(),
        ) {
            Ok(_) => succeeded.extend(group_tool_ids.iter().cloned()),
            Err(message) => failed.push(ManagedRulesActionFailure {
                tool_id: primary_tool_id,
                message,
            }),
        }
    }
    if !succeeded.is_empty() {
        update_baselines_after_successful_sync(&succeeded)?;
    }
    let state = get_managed_rules_state_domain(registry);
    Ok(ManagedRulesActionResult {
        requested_tool_ids: requested,
        succeeded_tool_ids: succeeded,
        failed,
        state,
    })
}

pub(crate) fn leave_rule_management_targets_domain(
    registry: &ToolRegistry,
    rule_id: String,
    tool_ids: Vec<String>,
    remove_managed_block: bool,
    dry_run: bool,
) -> Result<ManagedRulesActionResult, String> {
    let requested = unique_sorted(tool_ids);
    let mut config = app_config::load_config();
    let all_tool_ids = all_rule_tool_ids(registry, &config);
    let rule_index = config
        .default_rules
        .iter()
        .position(|rule| rule.id == rule_id)
        .ok_or_else(|| format!("Rule set not found: {}", rule_id))?;
    let mut failed = vec![];
    let mut succeeded = vec![];

    for tool_id in &requested {
        if let Err(message) = validate_writable_target(registry, &config, tool_id) {
            failed.push(ManagedRulesActionFailure {
                tool_id: tool_id.clone(),
                message,
            });
            continue;
        }
        if remove_managed_block && !dry_run {
            if let Err(message) = remove_managed_block_from_target(registry, &config, tool_id) {
                failed.push(ManagedRulesActionFailure {
                    tool_id: tool_id.clone(),
                    message,
                });
                continue;
            }
        }
        succeeded.push(tool_id.clone());
    }

    if !succeeded.is_empty() && !dry_run {
        let remove_set: HashSet<String> = succeeded
            .iter()
            .map(|tool_id| canonical_tool_id(tool_id, &all_tool_ids))
            .collect();
        let remaining: Vec<String> =
            persisted_rule_relationship_tool_ids(&config.default_rules[rule_index], &all_tool_ids)
                .into_iter()
                .filter(|tool_id| !remove_set.contains(tool_id))
                .collect();
        config.default_rules[rule_index].managed_targets = Some(unique_sorted(remaining));
        app_config::save_config(&config)?;
    }
    let state = if dry_run {
        get_managed_rules_state_for_config(registry, &config)
    } else {
        get_managed_rules_state_domain(registry)
    };
    Ok(ManagedRulesActionResult {
        requested_tool_ids: requested,
        succeeded_tool_ids: succeeded,
        failed,
        state,
    })
}

fn remove_managed_block_from_target(
    registry: &ToolRegistry,
    config: &app_config::AppConfig,
    tool_id: &str,
) -> Result<(), String> {
    let target = resolve_effective_global_rule_target(registry, config, tool_id)
        .ok_or_else(|| format!("No injection target configured for {}", tool_id))?;
    let expanded = shellexpand::tilde(&target.raw_path).to_string();
    let path = PathBuf::from(expanded);
    if !path.exists() {
        return Ok(());
    }
    let existing = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let cleaned = remove_block(&existing);
    if cleaned != existing {
        std::fs::write(&path, cleaned).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn validate_writable_target(
    registry: &ToolRegistry,
    config: &app_config::AppConfig,
    tool_id: &str,
) -> Result<(), String> {
    let canonical_tool_id = canonical_active_rule_tool_id(registry, config, tool_id)?;
    let target = resolve_effective_global_rule_target(registry, config, &canonical_tool_id)
        .ok_or_else(|| format!("No injection target configured for {}", tool_id))?;
    let expanded = shellexpand::tilde(&target.raw_path).to_string();
    let path = PathBuf::from(&expanded);
    if !target_is_eligible_effective_global_rule_target(registry, &target, &path, config) {
        return Err(format!(
            "No eligible global rule target configured for {}",
            tool_id
        ));
    }
    let capability = capability_state_for_effective_target(registry, &target, &path, config);
    if capability.can_write {
        Ok(())
    } else {
        Err(format!(
            "Global rule target is not writable for {}",
            tool_id
        ))
    }
}

fn update_baselines_after_successful_sync(succeeded_tool_ids: &[String]) -> Result<(), String> {
    app_config::update_config(|config| {
        let all_tool_ids = {
            let registry = ToolRegistry::new();
            all_rule_tool_ids(&registry, config)
        };
        update_baselines_after_successful_sync_for_config(
            config,
            succeeded_tool_ids,
            &all_tool_ids,
        );
        Ok(())
    })
    .map(|_| ())
}

fn update_baselines_after_successful_sync_for_config(
    config: &mut app_config::AppConfig,
    succeeded_tool_ids: &[String],
    all_tool_ids: &[String],
) {
    let success: HashSet<String> = succeeded_tool_ids
        .iter()
        .map(|tool_id| canonical_tool_id(tool_id, all_tool_ids))
        .collect();
    let common = config
        .default_rules
        .iter()
        .find(|rule| rule.id == "common_rule")
        .cloned();
    if let Some(rule) = common {
        if !success.is_empty() {
            config.default_rule_injection_baselines.common_rule =
                rule_fingerprint(&scoped_rule_for_fingerprint(&rule, all_tool_ids));
        }
    } else if !config
        .default_rule_injection_baselines
        .common_rule
        .is_empty()
        && all_tool_ids.iter().all(|tool_id| success.contains(tool_id))
    {
        config.default_rule_injection_baselines.common_rule.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{
        RuleFormat, RuleSource, ToolAdapter, ToolCapability, ToolCapabilityFormat,
        ToolCapabilityKind, ToolCapabilityScope, ToolCapabilitySourceConfidence,
        ToolSourceDiagnosticState,
    };
    use std::path::PathBuf;

    struct RuleStateTestAdapter {
        id: String,
        name: String,
        access: ToolCapabilityAccess,
        scope: ToolCapabilityScope,
        format: ToolCapabilityFormat,
        source_confidence: ToolCapabilitySourceConfidence,
        source_path: String,
    }

    impl RuleStateTestAdapter {
        fn new(id: &str, access: ToolCapabilityAccess, source_path: String) -> Self {
            Self {
                id: id.to_string(),
                name: id.to_string(),
                access,
                scope: ToolCapabilityScope::Global,
                format: ToolCapabilityFormat::Markdown,
                source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
                source_path,
            }
        }

        fn with_scope(mut self, scope: ToolCapabilityScope) -> Self {
            self.scope = scope;
            self
        }

        fn with_format(mut self, format: ToolCapabilityFormat) -> Self {
            self.format = format;
            self
        }

        fn with_confidence(mut self, confidence: ToolCapabilitySourceConfidence) -> Self {
            self.source_confidence = confidence;
            self
        }
    }

    impl ToolAdapter for RuleStateTestAdapter {
        fn id(&self) -> &str {
            &self.id
        }
        fn name(&self) -> &str {
            &self.name
        }
        fn icon(&self) -> &str {
            "R"
        }
        fn config_dir(&self) -> PathBuf {
            PathBuf::new()
        }
        fn detect(&self) -> bool {
            true
        }
        fn read_rules(&self) -> Result<Vec<RuleSource>, String> {
            Ok(vec![RuleSource {
                path: self.source_path.clone(),
                format: RuleFormat::SingleMarkdown,
                content: std::fs::read_to_string(&self.source_path).unwrap_or_default(),
                last_modified: 0,
                label: "Rules".to_string(),
                group: String::new(),
                diagnostic: None,
            }])
        }
        fn write_rule(&self, path: &str, content: &str) -> Result<(), String> {
            std::fs::write(path, content).map_err(|e| e.to_string())
        }
        fn capabilities(&self) -> Vec<ToolCapability> {
            vec![ToolCapability {
                id: "rules".to_string(),
                kind: ToolCapabilityKind::Rule,
                scope: self.scope.clone(),
                access: self.access.clone(),
                format: self.format.clone(),
                source_path: self.source_path.clone(),
                label: "Rules".to_string(),
                diagnostics: vec![ToolSourceDiagnosticState::Loaded],
                source_confidence: self.source_confidence.clone(),
                notes: String::new(),
                source_kind: crate::adapters::ToolCapabilitySourceKind::FeatureSource,
                primary_config_dir: None,
                supporting_sources: vec![],
                action_evidence: vec![],
            }]
        }
    }

    struct FilelessRuleTestAdapter;

    impl ToolAdapter for FilelessRuleTestAdapter {
        fn id(&self) -> &str {
            "cursor"
        }
        fn name(&self) -> &str {
            "Cursor"
        }
        fn icon(&self) -> &str {
            "C"
        }
        fn config_dir(&self) -> PathBuf {
            PathBuf::new()
        }
        fn detect(&self) -> bool {
            true
        }
        fn read_rules(&self) -> Result<Vec<RuleSource>, String> {
            Ok(vec![])
        }
        fn write_rule(&self, _path: &str, _content: &str) -> Result<(), String> {
            Err("unsupported".to_string())
        }
        fn capabilities(&self) -> Vec<ToolCapability> {
            vec![ToolCapability {
                id: "user-rules".to_string(),
                kind: ToolCapabilityKind::Rule,
                scope: ToolCapabilityScope::Global,
                access: ToolCapabilityAccess::Unsupported,
                format: ToolCapabilityFormat::Unknown,
                source_path: String::new(),
                label: "User Rules".to_string(),
                diagnostics: vec![ToolSourceDiagnosticState::Unsupported],
                source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
                notes: "fileless".to_string(),
                source_kind: crate::adapters::ToolCapabilitySourceKind::FeatureSource,
                primary_config_dir: None,
                supporting_sources: vec![],
                action_evidence: vec![],
            }]
        }
    }

    struct DirectoryOnlyRuleTestAdapter {
        rules_dir: PathBuf,
        rule_file: PathBuf,
    }

    impl ToolAdapter for DirectoryOnlyRuleTestAdapter {
        fn id(&self) -> &str {
            "directory-only"
        }
        fn name(&self) -> &str {
            "Directory Only"
        }
        fn icon(&self) -> &str {
            "D"
        }
        fn config_dir(&self) -> PathBuf {
            self.rules_dir.clone()
        }
        fn detect(&self) -> bool {
            true
        }
        fn read_rules(&self) -> Result<Vec<RuleSource>, String> {
            Ok(vec![RuleSource {
                path: self.rule_file.to_string_lossy().to_string(),
                format: RuleFormat::DirectoryMarkdown,
                content: std::fs::read_to_string(&self.rule_file).unwrap_or_default(),
                last_modified: 0,
                label: "native.md".to_string(),
                group: String::new(),
                diagnostic: None,
            }])
        }
        fn write_rule(&self, path: &str, content: &str) -> Result<(), String> {
            std::fs::write(path, content).map_err(|e| e.to_string())
        }
        fn capabilities(&self) -> Vec<ToolCapability> {
            vec![ToolCapability {
                id: "native-rules".to_string(),
                kind: ToolCapabilityKind::Rule,
                scope: ToolCapabilityScope::Global,
                access: ToolCapabilityAccess::Writable,
                format: ToolCapabilityFormat::Directory,
                source_path: self.rules_dir.to_string_lossy().to_string(),
                label: "Native Rules".to_string(),
                diagnostics: vec![ToolSourceDiagnosticState::Loaded],
                source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
                notes: String::new(),
                source_kind: crate::adapters::ToolCapabilitySourceKind::FeatureSource,
                primary_config_dir: None,
                supporting_sources: vec![],
                action_evidence: vec![],
            }]
        }
    }

    fn test_config(rule_path: &Path, rule: app_config::DefaultRule) -> app_config::AppConfig {
        let mut config = app_config::default_config();
        config.default_rules = vec![rule];
        config.tool_capability_overrides.insert(
            "tool-a".to_string(),
            app_config::ToolCapabilityOverrides {
                custom_rule_source_type: None,
                custom_rule_source_path: None,
                custom_global_rule_target: Some(rule_path.to_string_lossy().to_string()),
                shared_skill_direct_read: None,
                ..Default::default()
            },
        );
        config
    }

    fn default_rule(targets: Option<Vec<String>>, content: &str) -> app_config::DefaultRule {
        app_config::DefaultRule {
            id: "common_rule".to_string(),
            name: "Global".to_string(),
            content: content.to_string(),
            inject_to: vec![],
            managed_targets: targets,
        }
    }

    #[test]
    fn initialized_scope_uses_only_enabled_managed_tools() {
        let tmp = tempfile::tempdir().unwrap();
        let rule_path = tmp.path().join("RULES.md");
        let rule = default_rule(
            Some(vec!["tool-a".to_string(), "tool-b".to_string()]),
            "managed",
        );
        let registry = ToolRegistry::from_adapters_for_tests(vec![
            Box::new(RuleStateTestAdapter::new(
                "tool-a",
                ToolCapabilityAccess::Writable,
                rule_path.to_string_lossy().to_string(),
            )),
            Box::new(RuleStateTestAdapter::new(
                "tool-b",
                ToolCapabilityAccess::Writable,
                rule_path.to_string_lossy().to_string(),
            )),
        ]);
        let mut config = app_config::default_config();
        config.initialized = true;
        config.managed_tools = vec!["tool-a".to_string()];
        config.default_rules = vec![rule];
        config.tool_capability_overrides.insert(
            "tool-b".to_string(),
            app_config::ToolCapabilityOverrides {
                custom_rule_source_type: None,
                custom_rule_source_path: None,
                custom_global_rule_target: Some(rule_path.to_string_lossy().to_string()),
                shared_skill_direct_read: None,
                ..Default::default()
            },
        );

        assert_eq!(all_rule_tool_ids(&registry, &config), vec!["tool-a"]);
        assert_eq!(
            managed_rule_tool_ids(
                &config.default_rules[0],
                &all_rule_tool_ids(&registry, &config)
            ),
            vec!["tool-a"]
        );
    }

    #[test]
    fn persisted_relationships_keep_disabled_targets_for_future_reenable() {
        let rule = default_rule(
            Some(vec!["tool-a".to_string(), "tool-b".to_string()]),
            "managed",
        );
        let active_tool_ids = vec!["tool-a".to_string()];

        assert_eq!(
            managed_rule_tool_ids(&rule, &active_tool_ids),
            vec!["tool-a"]
        );
        assert_eq!(
            persisted_rule_relationship_tool_ids(&rule, &active_tool_ids),
            vec!["tool-a", "tool-b"]
        );
    }

    #[test]
    fn disabled_pending_targets_do_not_mark_enabled_tools_pending() {
        let rule = app_config::DefaultRule {
            id: "custom_rule".to_string(),
            name: "Custom Rule".to_string(),
            content: "managed".to_string(),
            inject_to: vec![],
            managed_targets: Some(vec!["tool-a".to_string(), "tool-b".to_string()]),
        };
        let mut baselines = app_config::DefaultRuleInjectionBaselines::default();
        baselines
            .custom_rules
            .insert(rule.id.clone(), rule_fingerprint(&rule));
        baselines
            .custom_rule_pending_targets
            .insert(rule.id.clone(), vec!["tool-b".to_string()]);

        assert!(!source_pending_for_rule(
            &rule,
            &baselines,
            &["tool-a".to_string()],
        ));

        baselines
            .custom_rule_pending_targets
            .insert(rule.id.clone(), vec!["tool-a".to_string()]);
        assert!(source_pending_for_rule(
            &rule,
            &baselines,
            &["tool-a".to_string()],
        ));
    }

    #[test]
    fn scan_ignores_disabled_tool_with_eligible_target() {
        let tmp = tempfile::tempdir().unwrap();
        let tool_a_path = tmp.path().join("tool-a.md");
        let tool_b_path = tmp.path().join("tool-b.md");
        let rule = default_rule(
            Some(vec!["tool-a".to_string(), "tool-b".to_string()]),
            "managed",
        );
        let registry = ToolRegistry::from_adapters_for_tests(vec![
            Box::new(RuleStateTestAdapter::new(
                "tool-a",
                ToolCapabilityAccess::Writable,
                tool_a_path.to_string_lossy().to_string(),
            )),
            Box::new(RuleStateTestAdapter::new(
                "tool-b",
                ToolCapabilityAccess::Writable,
                tool_b_path.to_string_lossy().to_string(),
            )),
        ]);
        let mut config = app_config::default_config();
        config.initialized = true;
        config.managed_tools = vec!["tool-a".to_string()];
        config.default_rules = vec![rule];
        config.default_rule_injection_baselines.common_rule = "outdated".to_string();

        let state = get_managed_rules_state_for_config(&registry, &config);

        assert_eq!(state.rule_sets[0].managed_tool_ids, vec!["tool-a"]);
        assert_eq!(state.targets.len(), 1);
        assert_eq!(state.targets[0].tool_id, "tool-a");
        assert_eq!(state.summary.managed_targets, 1);
        assert_eq!(state.summary.affected_tool_ids, vec!["tool-a"]);
    }

    #[test]
    fn direct_injection_rejects_disabled_tool() {
        let tmp = tempfile::tempdir().unwrap();
        let tool_b_path = tmp.path().join("tool-b.md");
        let rule = default_rule(Some(vec!["tool-b".to_string()]), "managed");
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleStateTestAdapter::new(
                "tool-b",
                ToolCapabilityAccess::Writable,
                tool_b_path.to_string_lossy().to_string(),
            ))]);
        let mut config = app_config::default_config();
        config.initialized = true;
        config.managed_tools = vec!["tool-a".to_string()];
        config.default_rules = vec![rule];

        let result = inject_default_rules_for_tools_with_config(
            &registry,
            &config,
            "tool-b".to_string(),
            vec!["tool-b".to_string()],
        );

        assert!(result.unwrap_err().contains("not enabled"));
        assert!(!tool_b_path.exists());
    }

    #[test]
    fn managed_block_parser_rejects_duplicate_or_incomplete_markers() {
        assert_eq!(extract_managed_block("plain"), ManagedBlockParse::Missing);
        assert!(matches!(
            extract_managed_block("<!-- ACC:DEFAULT:START -->\nmissing end"),
            ManagedBlockParse::Malformed(_)
        ));
        assert!(matches!(
            extract_managed_block(
                "<!-- ACC:DEFAULT:START -->\na\n<!-- ACC:DEFAULT:END -->\n<!-- ACC:DEFAULT:START -->"
            ),
            ManagedBlockParse::Malformed(_)
        ));
    }

    #[test]
    fn scan_classifies_in_sync_and_ignores_outside_content() {
        let tmp = tempfile::tempdir().unwrap();
        let rule_path = tmp.path().join("RULES.md");
        let rule = default_rule(Some(vec!["tool-a".to_string()]), "managed");
        let all_tool_ids = vec!["tool-a".to_string()];
        let block = build_managed_rules_block(&[rule.clone()], "tool-a", &all_tool_ids).unwrap();
        std::fs::write(&rule_path, format!("before\n\n{}\n\nafter", block)).unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleStateTestAdapter::new(
                "tool-a",
                ToolCapabilityAccess::Writable,
                rule_path.to_string_lossy().to_string(),
            ))]);
        let mut config = test_config(&rule_path, rule.clone());
        config.default_rule_injection_baselines.common_rule = rule_fingerprint(&rule);

        let state = get_managed_rules_state_for_config(&registry, &config);

        assert_eq!(state.targets.len(), 1);
        assert_eq!(
            state.targets[0].classification,
            ManagedRuleTargetClassification::InSync
        );
    }

    #[test]
    fn scan_classifies_missing_drift_and_certified_default_targets() {
        let tmp = tempfile::tempdir().unwrap();
        let missing_path = tmp.path().join("missing.md");
        let drift_path = tmp.path().join("drift.md");
        std::fs::write(
            &drift_path,
            format!("{}\nold\n{}", MARKER_START, MARKER_END),
        )
        .unwrap();
        let rule = default_rule(
            Some(vec![
                "tool-a".to_string(),
                "tool-b".to_string(),
                "tool-c".to_string(),
            ]),
            "managed",
        );
        let registry = ToolRegistry::from_adapters_for_tests(vec![
            Box::new(RuleStateTestAdapter::new(
                "tool-a",
                ToolCapabilityAccess::Writable,
                missing_path.to_string_lossy().to_string(),
            )),
            Box::new(RuleStateTestAdapter::new(
                "tool-b",
                ToolCapabilityAccess::Writable,
                drift_path.to_string_lossy().to_string(),
            )),
            Box::new(RuleStateTestAdapter::new(
                "tool-c",
                ToolCapabilityAccess::Writable,
                tmp.path()
                    .join("unconfigured.md")
                    .to_string_lossy()
                    .to_string(),
            )),
        ]);
        let mut config = app_config::default_config();
        config.default_rules = vec![rule.clone()];

        let state = get_managed_rules_state_for_config(&registry, &config);
        let by_tool: HashMap<_, _> = state
            .targets
            .iter()
            .map(|target| (target.tool_id.as_str(), target))
            .collect();

        assert_eq!(
            by_tool["tool-a"].reason,
            ManagedRuleTargetReason::FileMissing
        );
        assert_eq!(
            by_tool["tool-a"].classification,
            ManagedRuleTargetClassification::RequiresSync
        );
        assert_eq!(
            by_tool["tool-b"].classification,
            ManagedRuleTargetClassification::Drifted
        );
        assert_eq!(
            by_tool["tool-c"].reason,
            ManagedRuleTargetReason::FileMissing
        );
        assert_eq!(
            by_tool["tool-c"].classification,
            ManagedRuleTargetClassification::RequiresSync
        );
    }

    #[test]
    fn scan_omits_ineligible_targets_and_keeps_unreadable_writable_files() {
        let tmp = tempfile::tempdir().unwrap();
        let read_only_path = tmp.path().join("readonly.md");
        let unknown_path = tmp.path().join("unknown.md");
        let unsupported_path = tmp.path().join("unsupported.md");
        let project_only_path = tmp.path().join("project.md");
        let community_only_path = tmp.path().join("community.md");
        let unreadable_path = tmp.path().join("unreadable.md");
        let rule = default_rule(
            Some(vec![
                "tool-a".to_string(),
                "tool-b".to_string(),
                "tool-c".to_string(),
                "tool-d".to_string(),
                "tool-e".to_string(),
                "tool-f".to_string(),
            ]),
            "managed",
        );
        let all_tool_ids = vec![
            "tool-a".to_string(),
            "tool-b".to_string(),
            "tool-c".to_string(),
            "tool-d".to_string(),
            "tool-e".to_string(),
            "tool-f".to_string(),
        ];
        let block = build_managed_rules_block(&[rule.clone()], "tool-a", &all_tool_ids).unwrap();
        std::fs::write(&read_only_path, &block).unwrap();
        std::fs::write(&unknown_path, &block).unwrap();
        std::fs::write(&unsupported_path, &block).unwrap();
        std::fs::write(&project_only_path, &block).unwrap();
        std::fs::write(&community_only_path, &block).unwrap();
        std::fs::write(&unreadable_path, [0xff, 0xfe, 0xfd]).unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![
            Box::new(RuleStateTestAdapter::new(
                "tool-a",
                ToolCapabilityAccess::ReadOnly,
                read_only_path.to_string_lossy().to_string(),
            )),
            Box::new(RuleStateTestAdapter::new(
                "tool-b",
                ToolCapabilityAccess::Unknown,
                unknown_path.to_string_lossy().to_string(),
            )),
            Box::new(RuleStateTestAdapter::new(
                "tool-c",
                ToolCapabilityAccess::Unsupported,
                unsupported_path.to_string_lossy().to_string(),
            )),
            Box::new(
                RuleStateTestAdapter::new(
                    "tool-d",
                    ToolCapabilityAccess::Writable,
                    project_only_path.to_string_lossy().to_string(),
                )
                .with_scope(ToolCapabilityScope::Project),
            ),
            Box::new(
                RuleStateTestAdapter::new(
                    "tool-e",
                    ToolCapabilityAccess::Writable,
                    community_only_path.to_string_lossy().to_string(),
                )
                .with_confidence(ToolCapabilitySourceConfidence::OfficialCommunity),
            ),
            Box::new(RuleStateTestAdapter::new(
                "tool-f",
                ToolCapabilityAccess::Writable,
                unreadable_path.to_string_lossy().to_string(),
            )),
        ]);
        let mut config = app_config::default_config();
        config.default_rules = vec![rule];

        let state = get_managed_rules_state_for_config(&registry, &config);
        let by_tool: HashMap<_, _> = state
            .targets
            .iter()
            .map(|target| (target.tool_id.as_str(), target))
            .collect();

        assert!(!by_tool.contains_key("tool-a"));
        assert!(!by_tool.contains_key("tool-b"));
        assert!(!by_tool.contains_key("tool-c"));
        assert!(!by_tool.contains_key("tool-d"));
        assert!(!by_tool.contains_key("tool-e"));
        assert!(by_tool["tool-f"].can_write);
        assert!(by_tool["tool-f"].can_read);
        assert_eq!(
            by_tool["tool-f"].reason,
            ManagedRuleTargetReason::UnreadableTarget
        );
        assert_eq!(
            by_tool["tool-f"].classification,
            ManagedRuleTargetClassification::Unresolved
        );
        assert!(by_tool["tool-f"].current_block.is_none());
    }

    #[test]
    fn scan_canonicalizes_legacy_underscore_tool_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let target_path = tmp.path().join("CLAUDE.md");
        let rule = default_rule(Some(vec!["claude_code".to_string()]), "managed");
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleStateTestAdapter::new(
                "claude-code",
                ToolCapabilityAccess::Writable,
                target_path.to_string_lossy().to_string(),
            ))]);
        let mut config = app_config::default_config();
        config.default_rules = vec![rule];
        config.managed_tools = vec!["claude_code".to_string()];
        config.tool_capability_overrides.insert(
            "claude_code".to_string(),
            app_config::ToolCapabilityOverrides {
                custom_rule_source_type: None,
                custom_rule_source_path: None,
                custom_global_rule_target: Some(target_path.to_string_lossy().to_string()),
                shared_skill_direct_read: None,
                ..Default::default()
            },
        );

        let state = get_managed_rules_state_for_config(&registry, &config);

        assert_eq!(state.targets.len(), 1);
        assert_eq!(state.targets[0].tool_id, "claude-code");
        assert_eq!(
            state.targets[0].classification,
            ManagedRuleTargetClassification::RequiresSync
        );
        assert_eq!(
            state.targets[0].reason,
            ManagedRuleTargetReason::FileMissing
        );
    }

    #[test]
    fn sync_baseline_cleanup_ignores_legacy_custom_pending_targets() {
        let mut config = app_config::default_config();
        config.default_rules = vec![app_config::DefaultRule {
            id: "custom_rule".to_string(),
            name: "Custom Rule".to_string(),
            content: "managed".to_string(),
            inject_to: vec![],
            managed_targets: Some(vec!["claude_code".to_string()]),
        }];
        config
            .default_rule_injection_baselines
            .custom_rule_pending_targets
            .insert("custom_rule".to_string(), vec!["claude_code".to_string()]);

        update_baselines_after_successful_sync_for_config(
            &mut config,
            &["claude-code".to_string()],
            &["claude-code".to_string()],
        );

        assert!(config
            .default_rule_injection_baselines
            .custom_rule_pending_targets
            .contains_key("custom_rule"));
        assert!(!config
            .default_rule_injection_baselines
            .custom_rules
            .contains_key("custom_rule"));
    }

    #[test]
    fn scan_excludes_fileless_user_rules_from_actionable_targets() {
        let rule = default_rule(Some(vec!["cursor".to_string()]), "managed");
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(FilelessRuleTestAdapter)]);
        let mut config = app_config::default_config();
        config.default_rules = vec![rule];
        config.injection_targets.clear();

        let state = get_managed_rules_state_for_config(&registry, &config);

        assert_eq!(state.targets.len(), 0);
        assert_eq!(state.summary.unresolved_targets, 0);
    }

    #[test]
    fn directory_backed_global_rules_require_exact_configured_file() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        std::fs::create_dir_all(&rules_dir).unwrap();
        let exact_target = rules_dir.join("managed.md");
        let rule = default_rule(Some(vec!["tool-dir".to_string()]), "managed");
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(
            RuleStateTestAdapter::new(
                "tool-dir",
                ToolCapabilityAccess::Writable,
                rules_dir.to_string_lossy().to_string(),
            )
            .with_format(ToolCapabilityFormat::Directory),
        )]);
        let mut config = app_config::default_config();
        config.default_rules = vec![rule];
        config.injection_targets.clear();

        let without_exact_file = get_managed_rules_state_for_config(&registry, &config);
        assert_eq!(without_exact_file.targets.len(), 0);

        config.injection_targets.insert(
            "tool-dir".to_string(),
            exact_target.to_string_lossy().to_string(),
        );
        let with_injection_target_only = get_managed_rules_state_for_config(&registry, &config);
        assert_eq!(with_injection_target_only.targets.len(), 0);

        config.tool_capability_overrides.insert(
            "tool-dir".to_string(),
            app_config::ToolCapabilityOverrides {
                custom_rule_source_type: Some("directory".to_string()),
                custom_rule_source_path: Some(rules_dir.to_string_lossy().to_string()),
                custom_global_rule_target: Some(exact_target.to_string_lossy().to_string()),
                shared_skill_direct_read: None,
                ..Default::default()
            },
        );
        let with_exact_file = get_managed_rules_state_for_config(&registry, &config);

        assert_eq!(with_exact_file.targets.len(), 1);
        assert_eq!(
            with_exact_file.targets[0].reason,
            ManagedRuleTargetReason::FileMissing
        );
        assert!(with_exact_file.targets[0].can_write);
    }

    #[test]
    fn directory_only_rule_source_keeps_tool_native_files_outside_global_targets() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let rule_file = rules_dir.join("native.md");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::write(&rule_file, "native").unwrap();
        let rule = default_rule(Some(vec!["directory-only".to_string()]), "managed");
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(DirectoryOnlyRuleTestAdapter {
                rules_dir,
                rule_file,
            })]);
        let mut config = app_config::default_config();
        config.default_rules = vec![rule];
        config.injection_targets.clear();

        let state = get_managed_rules_state_for_config(&registry, &config);

        assert!(state.targets.is_empty());
        assert_eq!(state.unmanaged_tool_rule_count, 1);
        assert_eq!(state.summary.managed_targets, 0);
    }

    #[test]
    fn hermes_certified_default_is_createable_without_persisted_injection_target() {
        let tmp = tempfile::tempdir().unwrap();
        let target_path = tmp.path().join(".hermes").join("SOUL.md");
        let rule = default_rule(Some(vec!["hermes-agent".to_string()]), "managed");
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![crate::adapters::hermes_agent::create(
                tmp.path(),
            )]);
        let mut config = app_config::default_config();
        config.default_rules = vec![rule];
        config.managed_tools = vec!["hermes-agent".to_string()];
        config.injection_targets.clear();

        let state = get_managed_rules_state_for_config(&registry, &config);

        assert_eq!(state.targets.len(), 1);
        assert_eq!(state.targets[0].tool_id, "hermes-agent");
        let expected_path = target_path.to_string_lossy().to_string();
        assert_eq!(
            state.targets[0].target_path.as_deref(),
            Some(expected_path.as_str())
        );
        assert_eq!(
            state.targets[0].classification,
            ManagedRuleTargetClassification::RequiresSync
        );
        assert_eq!(
            state.targets[0].reason,
            ManagedRuleTargetReason::FileMissing
        );
        assert!(state.targets[0].can_write);
    }

    #[test]
    fn shared_rule_target_uses_union_block_for_each_covered_tool() {
        let tmp = tempfile::tempdir().unwrap();
        let shared_path = tmp.path().join("GEMINI.md");
        let common_rule = app_config::DefaultRule {
            id: "common_rule".to_string(),
            name: "Common Rule".to_string(),
            content: "common".to_string(),
            inject_to: vec![],
            managed_targets: Some(vec!["tool-a".to_string(), "tool-b".to_string()]),
        };
        let registry = ToolRegistry::from_adapters_for_tests(vec![
            Box::new(RuleStateTestAdapter::new(
                "tool-a",
                ToolCapabilityAccess::Writable,
                shared_path.to_string_lossy().to_string(),
            )),
            Box::new(RuleStateTestAdapter::new(
                "tool-b",
                ToolCapabilityAccess::Writable,
                shared_path.to_string_lossy().to_string(),
            )),
        ]);
        let mut config = app_config::default_config();
        config.default_rules = vec![common_rule];

        let state = get_managed_rules_state_for_config(&registry, &config);
        let by_tool: HashMap<_, _> = state
            .targets
            .iter()
            .map(|target| (target.tool_id.as_str(), target))
            .collect();
        let expected_a = by_tool["tool-a"].expected_block.as_ref().unwrap();
        let expected_b = by_tool["tool-b"].expected_block.as_ref().unwrap();

        assert_eq!(expected_a, expected_b);
        assert!(expected_a.contains("Common Rule"));
        assert!(expected_a.contains("common"));
    }

    #[test]
    fn leave_management_remove_preserves_content_outside_block() {
        let existing = format!("before\n{}\nmanaged\n{}\nafter", MARKER_START, MARKER_END);
        assert_eq!(remove_block(&existing), "before\n\nafter");
    }
}
