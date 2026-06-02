// Purpose: Normalize persisted tool ids through the central tool registry.

use super::registry;
use crate::platform::config::{
    AppConfig, SkillLocalSourceRecord, ToolCapabilityOverrides, ToolPaths,
};
use std::collections::{BTreeSet, HashMap};

pub(crate) fn canonical_tool_id(tool_id: &str) -> String {
    registry::canonical_id(tool_id)
}

fn is_retired_tool_id(tool_id: &str) -> bool {
    registry::is_retired_builtin_id(tool_id)
        || registry::is_retired_builtin_id(&canonical_tool_id(tool_id))
}

pub(crate) fn normalize_tool_id_list(tool_ids: &[String]) -> Vec<String> {
    let mut ids = BTreeSet::new();
    for tool_id in tool_ids {
        if is_retired_tool_id(tool_id) {
            continue;
        }
        let canonical = canonical_tool_id(tool_id);
        if !canonical.trim().is_empty() {
            ids.insert(canonical);
        }
    }
    ids.into_iter().collect()
}

fn normalize_tool_path_map(map: &mut HashMap<String, ToolPaths>) {
    let mut normalized = HashMap::new();
    let mut aliases = Vec::new();
    for (tool_id, paths) in std::mem::take(map) {
        if is_retired_tool_id(&tool_id) {
            continue;
        }
        let canonical = canonical_tool_id(&tool_id);
        if tool_id == canonical {
            normalized.insert(canonical, paths);
        } else {
            aliases.push((canonical, paths));
        }
    }
    for (canonical, paths) in aliases {
        normalized.entry(canonical).or_insert(paths);
    }
    *map = normalized;
}

fn normalize_injection_target_map(map: &mut HashMap<String, String>) {
    map.clear();
}

fn normalize_tool_capability_overrides_map(map: &mut HashMap<String, ToolCapabilityOverrides>) {
    let mut normalized: HashMap<String, ToolCapabilityOverrides> = HashMap::new();
    let mut aliases = Vec::new();
    for (tool_id, mut overrides) in std::mem::take(map) {
        if is_retired_tool_id(&tool_id) {
            continue;
        }
        normalize_tool_capability_overrides(&mut overrides);
        if overrides.is_empty() {
            continue;
        }
        let canonical = canonical_tool_id(&tool_id);
        if tool_id == canonical {
            normalized.insert(canonical, overrides);
        } else {
            aliases.push((canonical, overrides));
        }
    }
    for (canonical, overrides) in aliases {
        merge_tool_capability_overrides(normalized.entry(canonical).or_default(), overrides);
    }
    normalized.retain(|_, overrides| !overrides.is_empty());
    *map = normalized;
}

fn normalize_tool_capability_overrides(overrides: &mut ToolCapabilityOverrides) {
    overrides.custom_rule_source_path =
        normalized_override_path(overrides.custom_rule_source_path.as_deref());
    overrides.custom_global_rule_target =
        normalized_override_path(overrides.custom_global_rule_target.as_deref());
    overrides.custom_mcp_config_path =
        normalized_override_path(overrides.custom_mcp_config_path.as_deref());
    overrides.custom_tool_config_path =
        normalized_override_path(overrides.custom_tool_config_path.as_deref());
    overrides.custom_rule_source_type =
        normalized_rule_source_type(overrides.custom_rule_source_type.as_deref());

    if overrides.custom_rule_source_path.is_none() {
        overrides.custom_rule_source_type = None;
    } else {
        overrides.custom_rule_source_type = Some(
            overrides
                .custom_rule_source_type
                .as_deref()
                .map(str::to_string)
                .unwrap_or_else(|| "directory".to_string()),
        );
    }
}

fn normalized_override_path(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() || trimmed.contains('*') {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalized_rule_source_type(value: Option<&str>) -> Option<String> {
    match value?
        .trim()
        .to_ascii_lowercase()
        .replace('-', "_")
        .as_str()
    {
        "single_file" | "file" => Some("single_file".to_string()),
        "directory" | "dir" => Some("directory".to_string()),
        _ => None,
    }
}

fn merge_tool_capability_overrides(
    existing: &mut ToolCapabilityOverrides,
    incoming: ToolCapabilityOverrides,
) {
    if existing.custom_global_rule_target.is_none() {
        existing.custom_global_rule_target = incoming.custom_global_rule_target;
    }
    if existing.custom_rule_source_type.is_none() {
        existing.custom_rule_source_type = incoming.custom_rule_source_type;
    }
    if existing.custom_rule_source_path.is_none() {
        existing.custom_rule_source_path = incoming.custom_rule_source_path;
    }
    if existing.custom_mcp_config_path.is_none() {
        existing.custom_mcp_config_path = incoming.custom_mcp_config_path;
    }
    if existing.custom_tool_config_path.is_none() {
        existing.custom_tool_config_path = incoming.custom_tool_config_path;
    }
    if existing.shared_skill_direct_read.is_none() {
        existing.shared_skill_direct_read = incoming.shared_skill_direct_read;
    }
}

fn normalize_skill_source_records(records: &mut Vec<SkillLocalSourceRecord>) {
    let mut normalized = Vec::with_capacity(records.len());
    for mut record in std::mem::take(records) {
        if is_retired_tool_id(&record.tool_id) {
            continue;
        }
        record.tool_id = canonical_tool_id(&record.tool_id);
        normalized.push(record);
    }
    *records = normalized;
}

fn normalize_skill_source_map(map: &mut HashMap<String, Vec<SkillLocalSourceRecord>>) {
    for records in map.values_mut() {
        normalize_skill_source_records(records);
    }
    map.retain(|_, records| !records.is_empty());
}

fn normalize_custom_tools(tools: &mut Vec<crate::platform::config::CustomTool>) {
    tools.retain(|tool| !is_retired_tool_id(&tool.id));
    let mut normalized = Vec::new();
    for tool in &mut *tools {
        tool.id = canonical_tool_id(&tool.id);
        if registry::definition(&tool.id).is_some() {
            continue;
        }
        if tool.global_rule_file.trim().is_empty() && !tool.rule_file.trim().is_empty() {
            tool.global_rule_file = tool.rule_file.clone();
        }
        if tool.rule_file.trim().is_empty() && !tool.global_rule_file.trim().is_empty() {
            tool.rule_file = tool.global_rule_file.clone();
        }
        if !tool.id.trim().is_empty() {
            normalized
                .retain(|existing: &crate::platform::config::CustomTool| existing.id != tool.id);
            normalized.push(tool.clone());
        }
    }
    *tools = normalized;
}

pub(crate) fn normalize_config(config: &mut AppConfig) {
    normalize_custom_tools(&mut config.custom_tools);
    config.managed_tools = normalize_tool_id_list(&config.managed_tools);
    config.handled_new_tool_ids = normalize_tool_id_list(&config.handled_new_tool_ids);
    normalize_injection_target_map(&mut config.injection_targets);
    normalize_tool_path_map(&mut config.tool_paths);
    normalize_tool_capability_overrides_map(&mut config.tool_capability_overrides);

    for rule in &mut config.default_rules {
        rule.inject_to = normalize_tool_id_list(&rule.inject_to);
        if let Some(targets) = &mut rule.managed_targets {
            *targets = normalize_tool_id_list(targets);
        }
    }

    for targets in config
        .default_rule_injection_baselines
        .custom_rule_pending_targets
        .values_mut()
    {
        *targets = normalize_tool_id_list(targets);
    }

    normalize_skill_source_map(&mut config.skill_local_inventory.sources);
    normalize_skill_source_map(&mut config.skill_keep_local_choices);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::config::{DefaultRule, DefaultRuleInjectionBaselines};

    #[test]
    fn normalizes_legacy_ids_across_tool_owned_config_fields() {
        let mut config = AppConfig::default();
        config.custom_tools = vec![
            crate::platform::config::CustomTool {
                id: "antigravity".to_string(),
                name: "Retired Antigravity".to_string(),
                icon: "A".to_string(),
                config_dir: "~/.gemini/antigravity".to_string(),
                rule_directory: String::new(),
                global_rule_file: "~/.gemini/GEMINI.md".to_string(),
                skills_dir: "~/.gemini/antigravity/skills".to_string(),
                shared_skill_direct_read: false,
                mcp_config: String::new(),
                tool_config: String::new(),
                rule_file: "~/.gemini/GEMINI.md".to_string(),
            },
            crate::platform::config::CustomTool {
                id: "custom_tool".to_string(),
                name: "Custom Tool".to_string(),
                icon: "C".to_string(),
                config_dir: "~/.custom".to_string(),
                rule_directory: String::new(),
                global_rule_file: "~/.custom/RULES.md".to_string(),
                skills_dir: "~/.custom/skills".to_string(),
                shared_skill_direct_read: false,
                mcp_config: String::new(),
                tool_config: String::new(),
                rule_file: "~/.custom/RULES.md".to_string(),
            },
        ];
        config.managed_tools = vec![
            "claude_code".to_string(),
            "open_code".to_string(),
            "gemini_cli".to_string(),
        ];
        config.handled_new_tool_ids = vec![
            "cursor".to_string(),
            "claude_code".to_string(),
            "gemini_cli".to_string(),
        ];
        config
            .injection_targets
            .insert("claude_code".to_string(), "~/.claude/CLAUDE.md".to_string());
        config.tool_paths.insert(
            "open_code".to_string(),
            ToolPaths {
                config_dir: "~/.config/opencode".to_string(),
                skills_dir: "~/.config/opencode/skills".to_string(),
            },
        );
        config.tool_paths.insert(
            "gemini-cli".to_string(),
            ToolPaths {
                config_dir: "~/.gemini".to_string(),
                skills_dir: "~/.gemini/skills".to_string(),
            },
        );
        config.default_rules.push(DefaultRule {
            id: "rule".to_string(),
            name: "Rule".to_string(),
            content: "content".to_string(),
            inject_to: vec!["claude_code".to_string()],
            managed_targets: Some(vec!["open_code".to_string(), "gemini_cli".to_string()]),
        });
        config.default_rule_injection_baselines = DefaultRuleInjectionBaselines::default();
        config
            .default_rule_injection_baselines
            .custom_rule_pending_targets
            .insert("rule".to_string(), vec!["claude_code".to_string()]);
        config.skill_local_inventory.sources.insert(
            "demo".to_string(),
            vec![
                SkillLocalSourceRecord {
                    tool_id: "claude_code".to_string(),
                    abs_path: "/tmp/demo".to_string(),
                    content_hash: None,
                },
                SkillLocalSourceRecord {
                    tool_id: "gemini_cli".to_string(),
                    abs_path: "/tmp/retired".to_string(),
                    content_hash: None,
                },
            ],
        );
        config.skill_keep_local_choices.insert(
            "demo".to_string(),
            vec![
                SkillLocalSourceRecord {
                    tool_id: "open_code".to_string(),
                    abs_path: "/tmp/demo".to_string(),
                    content_hash: None,
                },
                SkillLocalSourceRecord {
                    tool_id: "antigravity".to_string(),
                    abs_path: "/tmp/retired".to_string(),
                    content_hash: None,
                },
            ],
        );
        config.tool_capability_overrides.insert(
            "claude_code".to_string(),
            ToolCapabilityOverrides {
                custom_rule_source_type: Some(" file ".to_string()),
                custom_rule_source_path: Some("  ~/.claude/CUSTOM.md  ".to_string()),
                custom_global_rule_target: Some("  ~/.claude/CUSTOM.md  ".to_string()),
                custom_mcp_config_path: Some("  ~/.claude/mcp.json  ".to_string()),
                custom_tool_config_path: Some("  ~/.claude/settings.json  ".to_string()),
                shared_skill_direct_read: Some(false),
            },
        );
        normalize_config(&mut config);

        assert_eq!(
            config
                .custom_tools
                .iter()
                .map(|tool| tool.id.as_str())
                .collect::<Vec<_>>(),
            vec!["custom_tool"]
        );
        assert_eq!(config.managed_tools, vec!["claude-code", "opencode"]);
        assert_eq!(
            config.handled_new_tool_ids,
            vec!["claude-code".to_string(), "cursor".to_string()]
        );
        assert!(config.injection_targets.is_empty());
        assert!(config.tool_paths.contains_key("opencode"));
        assert!(!config.tool_paths.contains_key("gemini-cli"));
        assert_eq!(config.default_rules[0].inject_to, vec!["claude-code"]);
        assert_eq!(
            config.default_rules[0].managed_targets.as_ref().unwrap(),
            &vec!["opencode".to_string()]
        );
        assert_eq!(
            config
                .default_rule_injection_baselines
                .custom_rule_pending_targets
                .get("rule"),
            Some(&vec!["claude-code".to_string()])
        );
        assert_eq!(
            config.skill_local_inventory.sources["demo"][0].tool_id,
            "claude-code"
        );
        assert_eq!(
            config.skill_keep_local_choices["demo"][0].tool_id,
            "opencode"
        );
        assert_eq!(config.skill_local_inventory.sources["demo"].len(), 1);
        assert_eq!(config.skill_keep_local_choices["demo"].len(), 1);
        let overrides = config.tool_capability_overrides.get("claude-code").unwrap();
        assert_eq!(
            overrides.custom_rule_source_type.as_deref(),
            Some("single_file")
        );
        assert_eq!(
            overrides.custom_rule_source_path.as_deref(),
            Some("~/.claude/CUSTOM.md")
        );
        assert_eq!(
            overrides.custom_global_rule_target.as_deref(),
            Some("~/.claude/CUSTOM.md")
        );
        assert_eq!(
            overrides.custom_mcp_config_path.as_deref(),
            Some("~/.claude/mcp.json")
        );
        assert_eq!(
            overrides.custom_tool_config_path.as_deref(),
            Some("~/.claude/settings.json")
        );
        assert_eq!(overrides.shared_skill_direct_read, Some(false));
    }

    #[test]
    fn preserves_unknown_custom_tool_ids() {
        let ids = normalize_tool_id_list(&["custom_tool".to_string()]);

        assert_eq!(ids, vec!["custom_tool"]);
    }

    #[test]
    fn drops_custom_tools_that_collide_with_builtin_ids_or_aliases() {
        let mut config = AppConfig::default();
        config.custom_tools = vec![
            crate::platform::config::CustomTool {
                id: "claude_code".to_string(),
                name: "Custom Claude".to_string(),
                icon: "wrench".to_string(),
                config_dir: String::new(),
                rule_directory: "/tmp/custom/rules".to_string(),
                global_rule_file: String::new(),
                skills_dir: String::new(),
                shared_skill_direct_read: false,
                mcp_config: String::new(),
                tool_config: String::new(),
                rule_file: String::new(),
            },
            crate::platform::config::CustomTool {
                id: "custom_tool".to_string(),
                name: "Custom Tool".to_string(),
                icon: "wrench".to_string(),
                config_dir: String::new(),
                rule_directory: "/tmp/custom/rules".to_string(),
                global_rule_file: String::new(),
                skills_dir: String::new(),
                shared_skill_direct_read: false,
                mcp_config: String::new(),
                tool_config: String::new(),
                rule_file: String::new(),
            },
        ];

        normalize_config(&mut config);

        assert_eq!(config.custom_tools.len(), 1);
        assert_eq!(config.custom_tools[0].id, "custom_tool");
    }

    #[test]
    fn canonical_entries_win_over_alias_entries() {
        let mut config = AppConfig::default();
        config
            .injection_targets
            .insert("claude_code".to_string(), "~/.claude/legacy.md".to_string());
        config.injection_targets.insert(
            "claude-code".to_string(),
            "~/.claude/canonical.md".to_string(),
        );
        config.tool_paths.insert(
            "claude_code".to_string(),
            ToolPaths {
                config_dir: "~/.claude-legacy".to_string(),
                skills_dir: "~/.claude-legacy/skills".to_string(),
            },
        );
        config.tool_paths.insert(
            "claude-code".to_string(),
            ToolPaths {
                config_dir: "~/.claude".to_string(),
                skills_dir: "~/.claude/skills".to_string(),
            },
        );
        config.tool_capability_overrides.insert(
            "claude_code".to_string(),
            ToolCapabilityOverrides {
                custom_rule_source_type: Some("directory".to_string()),
                custom_rule_source_path: Some("~/.claude/legacy-rules".to_string()),
                custom_global_rule_target: Some("~/.claude/legacy.md".to_string()),
                custom_mcp_config_path: Some("~/.claude/legacy-mcp.json".to_string()),
                custom_tool_config_path: None,
                shared_skill_direct_read: Some(false),
            },
        );
        config.tool_capability_overrides.insert(
            "claude-code".to_string(),
            ToolCapabilityOverrides {
                custom_rule_source_type: Some("single_file".to_string()),
                custom_rule_source_path: Some("~/.claude/canonical.md".to_string()),
                custom_global_rule_target: Some("~/.claude/canonical.md".to_string()),
                custom_mcp_config_path: None,
                custom_tool_config_path: Some("~/.claude/settings.json".to_string()),
                shared_skill_direct_read: None,
            },
        );
        normalize_config(&mut config);

        assert!(config.injection_targets.is_empty());
        assert_eq!(
            config
                .tool_paths
                .get("claude-code")
                .map(|paths| paths.config_dir.as_str()),
            Some("~/.claude")
        );
        let overrides = config.tool_capability_overrides.get("claude-code").unwrap();
        assert_eq!(
            overrides.custom_rule_source_type.as_deref(),
            Some("single_file")
        );
        assert_eq!(
            overrides.custom_rule_source_path.as_deref(),
            Some("~/.claude/canonical.md")
        );
        assert_eq!(
            overrides.custom_global_rule_target.as_deref(),
            Some("~/.claude/canonical.md")
        );
        assert_eq!(
            overrides.custom_mcp_config_path.as_deref(),
            Some("~/.claude/legacy-mcp.json")
        );
        assert_eq!(
            overrides.custom_tool_config_path.as_deref(),
            Some("~/.claude/settings.json")
        );
        assert_eq!(overrides.shared_skill_direct_read, Some(false));
    }

    #[test]
    fn drops_empty_tool_capability_override_entries() {
        let mut config = AppConfig::default();
        config.tool_capability_overrides.insert(
            "codex".to_string(),
            ToolCapabilityOverrides {
                custom_rule_source_type: Some("directory".to_string()),
                custom_rule_source_path: Some("   ".to_string()),
                custom_global_rule_target: Some("   ".to_string()),
                custom_mcp_config_path: Some("   ".to_string()),
                custom_tool_config_path: Some("   ".to_string()),
                shared_skill_direct_read: None,
            },
        );

        normalize_config(&mut config);

        assert!(config.tool_capability_overrides.is_empty());
    }
}
