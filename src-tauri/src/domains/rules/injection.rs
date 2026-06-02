// Purpose: Own rule injection targets and marker-delimited default-rule injection execution.

use super::*;
use std::collections::{HashMap, HashSet};

pub(crate) const MARKER_START: &str = "<!-- ACC:DEFAULT:START -->";
pub(crate) const MARKER_END: &str = "<!-- ACC:DEFAULT:END -->";

fn unique_sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.retain(|id| !id.trim().is_empty());
    ids.sort();
    ids.dedup();
    ids
}

pub(crate) fn community_default_rules(
    rules: &[app_config::DefaultRule],
) -> Vec<app_config::DefaultRule> {
    rules
        .iter()
        .filter(|rule| rule.id == "common_rule")
        .cloned()
        .collect()
}

pub(crate) fn canonical_tool_id(tool_id: &str, _all_tool_ids: &[String]) -> String {
    crate::platform::tool_catalog::normalization::canonical_tool_id(tool_id)
}

pub(crate) fn managed_rule_tool_ids(
    rule: &app_config::DefaultRule,
    all_tool_ids: &[String],
) -> Vec<String> {
    let active: HashSet<&str> = all_tool_ids.iter().map(String::as_str).collect();
    if let Some(targets) = &rule.managed_targets {
        return unique_sorted(
            targets
                .iter()
                .map(|id| canonical_tool_id(id, all_tool_ids))
                .filter(|id| active.contains(id.as_str()))
                .collect(),
        );
    }
    if rule.inject_to.is_empty() {
        return unique_sorted(all_tool_ids.to_vec());
    }
    unique_sorted(
        rule.inject_to
            .iter()
            .map(|id| canonical_tool_id(id, all_tool_ids))
            .filter(|id| active.contains(id.as_str()))
            .collect(),
    )
}

pub(crate) fn rule_applies_to_tool(
    rule: &app_config::DefaultRule,
    tool_id: &str,
    all_tool_ids: &[String],
) -> bool {
    managed_rule_tool_ids(rule, all_tool_ids)
        .iter()
        .any(|id| id == tool_id)
}

pub(crate) fn build_managed_rules_block(
    rules: &[app_config::DefaultRule],
    tool_id: &str,
    all_tool_ids: &[String],
) -> Option<String> {
    build_managed_rules_block_for_tools(rules, &[tool_id.to_string()], all_tool_ids)
}

pub(crate) fn build_managed_rules_block_for_tools(
    rules: &[app_config::DefaultRule],
    tool_ids: &[String],
    all_tool_ids: &[String],
) -> Option<String> {
    let rules_content: Vec<String> = rules
        .iter()
        .filter(|r| {
            tool_ids
                .iter()
                .any(|tool_id| rule_applies_to_tool(r, tool_id, all_tool_ids))
        })
        .filter(|r| !r.content.trim().is_empty())
        .map(|r| format!("## {}\n\n{}", r.name, r.content))
        .collect();

    if rules_content.is_empty() {
        return None;
    }

    Some(format!(
        "{}\n{}\n{}",
        MARKER_START,
        rules_content.join("\n\n---\n\n"),
        MARKER_END
    ))
}

/// Insert or replace a marker-delimited block in existing content.
/// Pure function - no file I/O.
pub fn inject_block(existing: &str, block: &str) -> String {
    if existing.contains(MARKER_START) && existing.contains(MARKER_END) {
        let start_idx = existing.find(MARKER_START).unwrap();
        let end_idx = existing.find(MARKER_END).unwrap() + MARKER_END.len();
        format!(
            "{}{}{}",
            &existing[..start_idx],
            block,
            &existing[end_idx..]
        )
    } else if existing.is_empty() {
        block.to_string()
    } else {
        format!("{}\n\n{}", existing.trim_end(), block)
    }
}

/// Remove a marker-delimited block from existing content.
/// Pure function - no file I/O.
pub fn remove_block(existing: &str) -> String {
    if existing.contains(MARKER_START) && existing.contains(MARKER_END) {
        let start_idx = existing.find(MARKER_START).unwrap();
        let end_idx = existing.find(MARKER_END).unwrap() + MARKER_END.len();
        let before = existing[..start_idx].trim_end();
        let after = existing[end_idx..].trim_start();
        if before.is_empty() {
            after.to_string()
        } else if after.is_empty() {
            before.to_string()
        } else {
            format!("{}\n\n{}", before, after)
        }
    } else {
        existing.to_string()
    }
}

pub(crate) fn inject_default_rules_domain(
    registry: &crate::adapters::ToolRegistry,
    tool_id: String,
) -> Result<String, String> {
    inject_default_rules_for_tools_domain(registry, tool_id.clone(), vec![tool_id])
}

pub(crate) fn inject_default_rules_for_tools_domain(
    registry: &crate::adapters::ToolRegistry,
    target_tool_id: String,
    covered_tool_ids: Vec<String>,
) -> Result<String, String> {
    let config = app_config::load_config();
    inject_default_rules_for_tools_with_config(registry, &config, target_tool_id, covered_tool_ids)
}

pub(crate) fn inject_default_rules_for_tools_with_config(
    registry: &crate::adapters::ToolRegistry,
    config: &app_config::AppConfig,
    target_tool_id: String,
    covered_tool_ids: Vec<String>,
) -> Result<String, String> {
    let all_tool_ids = all_rule_tool_ids(registry, &config);
    let canonical_target_tool_id = canonical_tool_id(&target_tool_id, &all_tool_ids);
    if !all_tool_ids.contains(&canonical_target_tool_id) {
        return Err(format!(
            "Tool is not enabled for management: {}",
            target_tool_id
        ));
    }
    let target = resolve_effective_global_rule_target(registry, config, &canonical_target_tool_id)
        .ok_or("No injection target configured for this tool")?;

    let expanded = shellexpand::tilde(&target.raw_path).to_string();
    let target_path = std::path::Path::new(&expanded);
    let base_capabilities = registry
        .get_adapter(&target.tool_id)
        .map(|adapter| adapter.capabilities())
        .or_else(|| {
            registry
                .detect_all_for_config(config)
                .into_iter()
                .find(|tool| tool.id == target.tool_id && tool.detected)
                .map(|tool| tool.capabilities)
        })
        .ok_or_else(|| format!("Unknown tool: {}", target.tool_id))?;
    let capabilities = effective_capabilities_for_global_rule_target(
        &target.tool_id,
        &base_capabilities,
        config,
        &target,
    );
    if global_rule_target_capabilities_for_target(&target.tool_id, &capabilities, target_path)
        .is_empty()
    {
        return Err("No eligible global rule target configured for this tool".to_string());
    }

    let active_tool_ids: HashSet<&str> = all_tool_ids.iter().map(String::as_str).collect();
    let covered_tool_ids = unique_sorted(
        covered_tool_ids
            .into_iter()
            .map(|tool_id| canonical_tool_id(&tool_id, &all_tool_ids))
            .filter(|tool_id| active_tool_ids.contains(tool_id.as_str()))
            .collect(),
    );
    if covered_tool_ids.is_empty() {
        return Err("No enabled tools selected for rule injection".to_string());
    }
    let rules = community_default_rules(&config.default_rules);
    let block = if covered_tool_ids.len() == 1 {
        build_managed_rules_block(&rules, &covered_tool_ids[0], &all_tool_ids)
    } else {
        build_managed_rules_block_for_tools(&rules, &covered_tool_ids, &all_tool_ids)
    };

    if block.is_none() {
        return Ok("No default rules to inject".to_string());
    }
    let block = block.unwrap();

    let existing = if target_path.exists() {
        std::fs::read_to_string(target_path).map_err(|e| e.to_string())?
    } else {
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        String::new()
    };

    let new_content = inject_block(&existing, &block);

    std::fs::write(target_path, new_content).map_err(|e| e.to_string())?;
    Ok(format!("Injected managed rules to {}", target.raw_path))
}

pub(crate) fn get_injection_targets_domain() -> HashMap<String, String> {
    let config = app_config::load_config();
    let registry = crate::adapters::ToolRegistry::new();
    resolved_effective_global_rule_targets(&registry, &config)
        .into_iter()
        .map(|(k, v)| (k.replace('_', "-"), v))
        .collect()
}

pub(crate) fn set_injection_target_domain(tool_id: String, path: String) -> Result<(), String> {
    app_config::update_config(|config| {
        let canonical = canonical_tool_id(&tool_id, &[]);
        let target = path.trim();
        if target.is_empty() {
            if let Some(overrides) = config.tool_capability_overrides.get_mut(&canonical) {
                overrides.custom_global_rule_target = None;
            }
            config
                .tool_capability_overrides
                .retain(|_, overrides| !overrides.is_empty());
        } else {
            let overrides = config
                .tool_capability_overrides
                .entry(canonical)
                .or_default();
            overrides.custom_global_rule_target = Some(target.to_string());
        }
        Ok(())
    })
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{dev_tool::DevToolAdapter, ToolRegistry};

    #[test]
    fn inject_block_into_empty() {
        let result = inject_block("", "BLOCK");
        assert_eq!(result, "BLOCK");
    }

    #[test]
    fn inject_block_appends_to_existing() {
        let result = inject_block("existing content", "BLOCK");
        assert_eq!(result, "existing content\n\nBLOCK");
    }

    #[test]
    fn inject_block_replaces_marker() {
        let existing = format!("before\n{}\nold rules\n{}\nafter", MARKER_START, MARKER_END);
        let result = inject_block(&existing, "NEW");
        assert_eq!(result, "before\nNEW\nafter");
    }

    #[test]
    fn inject_block_trims_trailing_whitespace() {
        let result = inject_block("content   \n\n\n", "BLOCK");
        assert_eq!(result, "content\n\nBLOCK");
    }

    #[test]
    fn remove_block_no_marker() {
        let result = remove_block("no markers here");
        assert_eq!(result, "no markers here");
    }

    #[test]
    fn remove_block_only_marker() {
        let existing = format!("{}\nrules\n{}", MARKER_START, MARKER_END);
        let result = remove_block(&existing);
        assert_eq!(result, "");
    }

    #[test]
    fn remove_block_with_before() {
        let existing = format!("before\n{}\nrules\n{}", MARKER_START, MARKER_END);
        let result = remove_block(&existing);
        assert_eq!(result, "before");
    }

    #[test]
    fn remove_block_with_after() {
        let existing = format!("{}\nrules\n{}\nafter", MARKER_START, MARKER_END);
        let result = remove_block(&existing);
        assert_eq!(result, "after");
    }

    #[test]
    fn inject_default_rules_creates_missing_user_override_target_file() {
        let tmp = tempfile::tempdir().unwrap();
        let base_dir = tmp.path().join("tool-a");
        let target_path = base_dir.join("RULES.md");
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(DevToolAdapter::new(
            "tool-a", "Tool A", "T", base_dir, false,
        ))]);
        let mut config = app_config::AppConfig::default();
        config.tool_capability_overrides.insert(
            "tool-a".to_string(),
            app_config::ToolCapabilityOverrides {
                custom_rule_source_type: None,
                custom_rule_source_path: None,
                custom_global_rule_target: Some(target_path.to_string_lossy().to_string()),
                shared_skill_direct_read: None,
                ..Default::default()
            },
        );
        config.default_rules = vec![app_config::DefaultRule {
            id: "common_rule".to_string(),
            name: "Global Rules".to_string(),
            content: "managed instruction".to_string(),
            inject_to: vec![],
            managed_targets: Some(vec!["tool-a".to_string()]),
        }];

        let result = inject_default_rules_for_tools_with_config(
            &registry,
            &config,
            "tool-a".to_string(),
            vec!["tool-a".to_string()],
        )
        .unwrap();

        assert!(result.contains("Injected managed rules"));
        let content = std::fs::read_to_string(&target_path).unwrap();
        assert!(content.contains(MARKER_START));
        assert!(content.contains("## Global Rules"));
        assert!(content.contains("managed instruction"));
        assert!(content.contains(MARKER_END));
    }

    #[test]
    fn inject_default_rules_creates_missing_certified_default_target_file() {
        let tmp = tempfile::tempdir().unwrap();
        let target_path = tmp.path().join(".hermes").join("SOUL.md");
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![crate::adapters::hermes_agent::create(
                tmp.path(),
            )]);
        let mut config = app_config::AppConfig::default();
        config.injection_targets.clear();
        config.default_rules = vec![app_config::DefaultRule {
            id: "common_rule".to_string(),
            name: "Global Rules".to_string(),
            content: "hermes managed instruction".to_string(),
            inject_to: vec![],
            managed_targets: Some(vec!["hermes-agent".to_string()]),
        }];

        let result = inject_default_rules_for_tools_with_config(
            &registry,
            &config,
            "hermes-agent".to_string(),
            vec!["hermes-agent".to_string()],
        )
        .unwrap();

        assert!(result.contains("Injected managed rules"));
        let content = std::fs::read_to_string(&target_path).unwrap();
        assert!(content.contains(MARKER_START));
        assert!(content.contains("hermes managed instruction"));
        assert!(content.contains(MARKER_END));
    }

    #[test]
    fn inject_default_rules_without_active_rule_keeps_existing_managed_block() {
        let tmp = tempfile::tempdir().unwrap();
        let base_dir = tmp.path().join("tool-a");
        let target_path = base_dir.join("RULES.md");
        std::fs::create_dir_all(&base_dir).unwrap();
        let existing = format!("before\n{}\nold\n{}\nafter", MARKER_START, MARKER_END);
        std::fs::write(&target_path, &existing).unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(DevToolAdapter::new(
            "tool-a", "Tool A", "T", base_dir, false,
        ))]);
        let mut config = app_config::AppConfig::default();
        config.initialized = true;
        config.managed_tools = vec!["tool-a".to_string()];
        config.tool_capability_overrides.insert(
            "tool-a".to_string(),
            app_config::ToolCapabilityOverrides {
                custom_rule_source_type: None,
                custom_rule_source_path: None,
                custom_global_rule_target: Some(target_path.to_string_lossy().to_string()),
                shared_skill_direct_read: None,
                ..Default::default()
            },
        );
        config.default_rules = vec![app_config::DefaultRule {
            id: "custom_rule".to_string(),
            name: "Custom Rule".to_string(),
            content: "disabled target only".to_string(),
            inject_to: vec![],
            managed_targets: Some(vec!["tool-b".to_string()]),
        }];

        let result = inject_default_rules_for_tools_with_config(
            &registry,
            &config,
            "tool-a".to_string(),
            vec!["tool-a".to_string()],
        )
        .unwrap();

        assert_eq!(result, "No default rules to inject");
        assert_eq!(std::fs::read_to_string(&target_path).unwrap(), existing);
    }

    #[test]
    fn remove_block_with_before_and_after() {
        let existing = format!("before\n{}\nrules\n{}\nafter", MARKER_START, MARKER_END);
        let result = remove_block(&existing);
        assert_eq!(result, "before\n\nafter");
    }
}
