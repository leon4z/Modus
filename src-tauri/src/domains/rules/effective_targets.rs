// Purpose: Resolve the effective Global Rule target shared by Rules status and writes.

use super::*;
use crate::adapters::{
    capability_declared_source_path, capability_projections, ToolCapability, ToolRegistry,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EffectiveGlobalRuleTargetSource {
    UserOverride,
    CertifiedDefault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectiveGlobalRuleTarget {
    pub tool_id: String,
    pub raw_path: String,
    pub source: EffectiveGlobalRuleTargetSource,
}

impl EffectiveGlobalRuleTarget {
    pub(crate) fn expanded_key(&self) -> String {
        expanded_target_key(&self.raw_path)
    }
}

fn expanded_target_key(raw: &str) -> String {
    PathBuf::from(shellexpand::tilde(raw).to_string())
        .to_string_lossy()
        .to_string()
}

fn normalized_exact_target(raw: &str) -> Option<String> {
    let target = raw.trim();
    if target.is_empty() || target.contains('*') {
        return None;
    }
    Some(target.to_string())
}

fn override_target_for_overrides(
    overrides: &app_config::ToolCapabilityOverrides,
) -> Option<String> {
    overrides
        .custom_global_rule_target
        .as_deref()
        .and_then(normalized_exact_target)
}

fn normalized_override_target(
    config: &app_config::AppConfig,
    canonical_id: &str,
) -> Option<String> {
    if let Some(target) = config
        .tool_capability_overrides
        .get(canonical_id)
        .and_then(override_target_for_overrides)
    {
        return Some(target);
    }

    let mut aliases = BTreeMap::new();
    for (tool_id, overrides) in &config.tool_capability_overrides {
        if canonical_tool_id(tool_id, &[]) == canonical_id {
            if let Some(target) = override_target_for_overrides(overrides) {
                aliases.insert(tool_id.clone(), target);
            }
        }
    }
    aliases.into_values().next().or_else(|| {
        config
            .custom_tools
            .iter()
            .find(|tool| canonical_tool_id(&tool.id, &[]) == canonical_id)
            .and_then(|tool| {
                normalized_exact_target(&tool.global_rule_file)
                    .or_else(|| normalized_exact_target(&tool.rule_file))
            })
    })
}

fn certified_default_global_rule_target(
    registry: &ToolRegistry,
    canonical_tool_id: &str,
) -> Option<String> {
    let adapter = registry.get_adapter(canonical_tool_id)?;
    capability_projections::project_capabilities(
        adapter.id(),
        capability_projections::ToolCapabilityModule::Rules,
        &adapter.capabilities(),
    )
    .into_iter()
    .find(|projection| {
        projection.source_role == capability_projections::ToolCapabilitySourceRole::RuleGlobalTarget
            && projection.allows(&capability_projections::ToolCapabilityAction::Inject)
            && projection.exclusion_reason.is_none()
    })
    .and_then(|projection| capability_declared_source_path(&projection.evidence))
    .map(|path| path.to_string_lossy().to_string())
}

pub(crate) fn resolve_effective_global_rule_target(
    registry: &ToolRegistry,
    config: &app_config::AppConfig,
    tool_id: &str,
) -> Option<EffectiveGlobalRuleTarget> {
    let canonical = canonical_tool_id(tool_id, &[]);
    if let Some(raw_path) = normalized_override_target(config, &canonical) {
        return Some(EffectiveGlobalRuleTarget {
            tool_id: canonical,
            raw_path,
            source: EffectiveGlobalRuleTargetSource::UserOverride,
        });
    }
    certified_default_global_rule_target(registry, &canonical).map(|raw_path| {
        EffectiveGlobalRuleTarget {
            tool_id: canonical,
            raw_path,
            source: EffectiveGlobalRuleTargetSource::CertifiedDefault,
        }
    })
}

pub(crate) fn resolved_effective_global_rule_targets(
    registry: &ToolRegistry,
    config: &app_config::AppConfig,
) -> BTreeMap<String, String> {
    let mut ids: Vec<String> = registry.tool_ids();
    ids.extend(config.tool_capability_overrides.keys().cloned());
    ids.extend(config.custom_tools.iter().map(|tool| tool.id.clone()));
    ids.sort();
    ids.dedup();

    ids.into_iter()
        .filter_map(|tool_id| resolve_effective_global_rule_target(registry, config, &tool_id))
        .map(|target| (target.tool_id, target.raw_path))
        .collect()
}

pub(crate) fn effective_capabilities_for_global_rule_target(
    adapter_id: &str,
    base_capabilities: &[ToolCapability],
    config: &app_config::AppConfig,
    target: &EffectiveGlobalRuleTarget,
) -> Vec<ToolCapability> {
    let mut scoped_config = config.clone();
    if target.source == EffectiveGlobalRuleTargetSource::UserOverride {
        scoped_config
            .tool_capability_overrides
            .entry(target.tool_id.clone())
            .or_default()
            .custom_global_rule_target = Some(target.raw_path.clone());
    }
    crate::adapters::effective_capabilities::resolve_effective_capabilities(
        adapter_id,
        base_capabilities,
        &scoped_config,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{ToolAdapter, ToolCapabilityAccess, ToolCapabilityFormat};
    use crate::platform::tool_capabilities::{
        tool_capability, ToolCapabilityKind, ToolCapabilityScope, ToolCapabilitySourceConfidence,
    };
    use std::path::PathBuf;

    struct EffectiveTargetTestAdapter {
        id: &'static str,
        source_path: String,
    }

    impl ToolAdapter for EffectiveTargetTestAdapter {
        fn id(&self) -> &str {
            self.id
        }
        fn name(&self) -> &str {
            self.id
        }
        fn icon(&self) -> &str {
            "T"
        }
        fn config_dir(&self) -> PathBuf {
            PathBuf::new()
        }
        fn detect(&self) -> bool {
            true
        }
        fn read_rules(&self) -> Result<Vec<crate::adapters::RuleSource>, String> {
            Ok(vec![])
        }
        fn write_rule(&self, _path: &str, _content: &str) -> Result<(), String> {
            Ok(())
        }
        fn capabilities(&self) -> Vec<ToolCapability> {
            vec![tool_capability(
                "global-rule",
                ToolCapabilityKind::Rule,
                ToolCapabilityScope::Global,
                ToolCapabilityAccess::Writable,
                ToolCapabilityFormat::Markdown,
                &self.source_path,
                "Global Rule",
                ToolCapabilitySourceConfidence::OfficialDocs,
                "certified global rule target",
            )]
        }
    }

    fn registry(id: &'static str, source_path: String) -> ToolRegistry {
        ToolRegistry::from_adapters_for_tests(vec![Box::new(EffectiveTargetTestAdapter {
            id,
            source_path,
        })])
    }

    #[test]
    fn resolves_certified_default_without_persisted_injection_target() {
        let tmp = tempfile::tempdir().unwrap();
        let default_target = tmp.path().join("SOUL.md").to_string_lossy().to_string();
        let registry = registry("hermes-agent", default_target.clone());
        let mut config = app_config::AppConfig::default();
        config.injection_targets.clear();

        let target =
            resolve_effective_global_rule_target(&registry, &config, "hermes-agent").unwrap();

        assert_eq!(target.tool_id, "hermes-agent");
        assert_eq!(target.raw_path, default_target);
        assert_eq!(
            target.source,
            EffectiveGlobalRuleTargetSource::CertifiedDefault
        );
    }

    #[test]
    fn user_override_alias_precedes_certified_default() {
        let tmp = tempfile::tempdir().unwrap();
        let default_target = tmp.path().join("CLAUDE.md").to_string_lossy().to_string();
        let override_target = tmp.path().join("custom.md").to_string_lossy().to_string();
        let registry = registry("claude-code", default_target);
        let mut config = app_config::AppConfig::default();
        config.injection_targets.clear();
        config.tool_capability_overrides.insert(
            "claude_code".to_string(),
            app_config::ToolCapabilityOverrides {
                custom_rule_source_type: None,
                custom_rule_source_path: None,
                custom_global_rule_target: Some(format!("  {}  ", override_target)),
                shared_skill_direct_read: None,
                ..Default::default()
            },
        );

        let target =
            resolve_effective_global_rule_target(&registry, &config, "claude_code").unwrap();

        assert_eq!(target.tool_id, "claude-code");
        assert_eq!(target.raw_path, override_target);
        assert_eq!(target.source, EffectiveGlobalRuleTargetSource::UserOverride);
    }

    #[test]
    fn stale_injection_target_is_ignored_before_certified_default() {
        let tmp = tempfile::tempdir().unwrap();
        let default_target = tmp.path().join("CLAUDE.md").to_string_lossy().to_string();
        let stale_target = tmp.path().join("stale.md").to_string_lossy().to_string();
        let registry = registry("claude-code", default_target.clone());
        let mut config = app_config::AppConfig::default();
        config.injection_targets.clear();
        config
            .injection_targets
            .insert("claude_code".to_string(), stale_target);

        let target =
            resolve_effective_global_rule_target(&registry, &config, "claude_code").unwrap();

        assert_eq!(target.tool_id, "claude-code");
        assert_eq!(target.raw_path, default_target);
        assert_eq!(
            target.source,
            EffectiveGlobalRuleTargetSource::CertifiedDefault
        );
    }

    #[test]
    fn directory_override_does_not_constrain_global_rule_target() {
        let tmp = tempfile::tempdir().unwrap();
        let default_target = tmp.path().join("CLAUDE.md").to_string_lossy().to_string();
        let rules_dir = tmp.path().join("rules").to_string_lossy().to_string();
        let outside_target = tmp
            .path()
            .join("outside")
            .join("AGENTS.md")
            .to_string_lossy()
            .to_string();
        let registry = registry("claude-code", default_target);
        let mut config = app_config::AppConfig::default();
        config.tool_capability_overrides.insert(
            "claude-code".to_string(),
            app_config::ToolCapabilityOverrides {
                custom_rule_source_type: Some("directory".to_string()),
                custom_rule_source_path: Some(rules_dir),
                custom_global_rule_target: Some(outside_target.clone()),
                shared_skill_direct_read: None,
                ..Default::default()
            },
        );

        let target = resolve_effective_global_rule_target(&registry, &config, "claude-code")
            .expect("expected independent user override target");

        assert_eq!(target.raw_path, outside_target);
        assert_eq!(target.source, EffectiveGlobalRuleTargetSource::UserOverride);
    }
}
