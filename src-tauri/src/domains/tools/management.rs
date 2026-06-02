// Purpose: Own managed tools, custom tools, and tool path persistence.

use super::*;
use std::collections::BTreeSet;
use std::collections::HashMap;

type ToolCapabilityOverridesMap = HashMap<String, app_config::ToolCapabilityOverrides>;

pub(crate) fn canonical_managed_tool_ids(tool_ids: &[String]) -> Vec<String> {
    let mut ids = BTreeSet::new();
    for tool_id in tool_ids {
        let canonical = crate::platform::tool_catalog::normalization::canonical_tool_id(tool_id);
        if !canonical.trim().is_empty() {
            ids.insert(canonical);
        }
    }
    ids.into_iter().collect()
}

pub(crate) fn get_tool_paths_domain() -> HashMap<String, app_config::ToolPaths> {
    let config = app_config::load_config();
    config.tool_paths
}

pub(crate) fn get_tool_capability_overrides_domain() -> ToolCapabilityOverridesMap {
    let config = app_config::load_config();
    config.tool_capability_overrides
}

pub(crate) fn set_tool_capability_overrides_domain(
    tool_id: String,
    custom_rule_source_type: Option<String>,
    custom_rule_source_path: Option<String>,
    custom_global_rule_target: Option<String>,
    custom_mcp_config_path: Option<String>,
    custom_tool_config_path: Option<String>,
    shared_skill_direct_read: Option<bool>,
) -> Result<(), String> {
    app_config::update_config(|config| {
        let canonical = crate::platform::tool_catalog::normalization::canonical_tool_id(&tool_id);
        let trimmed_source_path = trimmed_non_empty(custom_rule_source_path.as_deref());
        let trimmed_source_type = normalized_rule_source_type(custom_rule_source_type.as_deref())
            .filter(|_| trimmed_source_path.is_some());
        let trimmed_target = trimmed_non_empty(custom_global_rule_target.as_deref());

        let overrides = app_config::ToolCapabilityOverrides {
            custom_rule_source_type: trimmed_source_type,
            custom_rule_source_path: trimmed_source_path,
            custom_global_rule_target: trimmed_target,
            custom_mcp_config_path: trimmed_non_empty(custom_mcp_config_path.as_deref()),
            custom_tool_config_path: trimmed_non_empty(custom_tool_config_path.as_deref()),
            shared_skill_direct_read,
        };
        if overrides.is_empty() {
            config.tool_capability_overrides.remove(&canonical);
        } else {
            config
                .tool_capability_overrides
                .insert(canonical, overrides);
        }
        Ok(())
    })
    .map(|_| ())
}

fn trimmed_non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.contains('*'))
        .map(str::to_string)
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

pub(crate) fn set_tool_path_domain(
    tool_id: String,
    config_dir: String,
    skills_dir: String,
) -> Result<(), String> {
    app_config::update_config(|config| {
        let tool_id = crate::platform::tool_catalog::normalization::canonical_tool_id(&tool_id);
        config.tool_paths.insert(
            tool_id,
            app_config::ToolPaths {
                config_dir,
                skills_dir,
            },
        );
        Ok(())
    })
    .map(|_| ())
}

pub(crate) fn get_custom_tools_domain() -> Vec<app_config::CustomTool> {
    let config = app_config::load_config();
    config.custom_tools
}

pub(crate) fn add_custom_tool_domain(tool: app_config::CustomTool) -> Result<(), String> {
    let canonical = crate::platform::tool_catalog::normalization::canonical_tool_id(&tool.id);
    if canonical.trim().is_empty() {
        return Err("Custom tool id is required".to_string());
    }
    if crate::platform::tool_catalog::registry::definition(&canonical).is_some() {
        return Err("Custom tool id conflicts with a built-in tool".to_string());
    }
    app_config::update_config(|config| {
        config.custom_tools.retain(|existing| {
            crate::platform::tool_catalog::normalization::canonical_tool_id(&existing.id)
                != canonical
        });
        let mut tool = tool;
        tool.id = canonical;
        config.custom_tools.push(tool);
        Ok(())
    })
    .map(|_| ())
}

fn remove_custom_tool_from_config(config: &mut app_config::AppConfig, canonical: &str) {
    config.custom_tools.retain(|tool| {
        crate::platform::tool_catalog::normalization::canonical_tool_id(&tool.id) != canonical
    });
    config.managed_tools.retain(|id| {
        crate::platform::tool_catalog::normalization::canonical_tool_id(id) != canonical
    });
    config.handled_new_tool_ids.retain(|id| {
        crate::platform::tool_catalog::normalization::canonical_tool_id(id) != canonical
    });
    config.tool_paths.remove(canonical);
    config.tool_capability_overrides.remove(canonical);
    config.injection_targets.remove(canonical);
}

pub(crate) fn remove_custom_tool_domain(tool_id: String) -> Result<(), String> {
    app_config::update_config(|config| {
        let canonical = crate::platform::tool_catalog::normalization::canonical_tool_id(&tool_id);
        remove_custom_tool_from_config(config, &canonical);
        Ok(())
    })
    .map(|_| ())
}

pub(crate) fn get_managed_tools_domain() -> Vec<String> {
    let config = app_config::load_config();
    canonical_managed_tool_ids(&config.managed_tools)
}

pub(crate) fn get_handled_new_tool_ids_domain() -> Vec<String> {
    let config = app_config::load_config();
    canonical_managed_tool_ids(&config.handled_new_tool_ids)
}

pub(crate) fn set_handled_new_tool_ids_domain(tool_ids: Vec<String>) -> Result<(), String> {
    app_config::update_config(|config| {
        config.handled_new_tool_ids = canonical_managed_tool_ids(&tool_ids);
        Ok(())
    })
    .map(|_| ())
}

pub(crate) fn set_managed_tools_domain(tool_ids: Vec<String>) -> Result<(), String> {
    app_config::update_config(|config| {
        config.managed_tools = canonical_managed_tool_ids(&tool_ids);
        config.initialized = true;
        Ok(())
    })
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_legacy_underscore_managed_tool_ids() {
        let managed = vec![
            "claude_code".to_string(),
            "claude-code".to_string(),
            "codex".to_string(),
        ];

        assert_eq!(
            canonical_managed_tool_ids(&managed),
            vec!["claude-code".to_string(), "codex".to_string()]
        );
    }

    #[test]
    fn rejects_custom_tool_ids_that_collide_with_builtin_ids_or_aliases() {
        let custom = app_config::CustomTool {
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
        };

        let err = add_custom_tool_domain(custom).unwrap_err();

        assert!(err.contains("built-in tool"));
    }

    #[test]
    fn removing_custom_tool_clears_only_modus_owned_metadata() {
        let mut config = app_config::default_config();
        config.custom_tools.push(app_config::CustomTool {
            id: "custom-tool".to_string(),
            name: "Custom Tool".to_string(),
            icon: "wrench".to_string(),
            config_dir: String::new(),
            rule_directory: "/external/rules".to_string(),
            global_rule_file: "/external/AGENTS.md".to_string(),
            skills_dir: "/external/skills".to_string(),
            shared_skill_direct_read: false,
            mcp_config: "/external/mcp.json".to_string(),
            tool_config: "/external/settings.json".to_string(),
            rule_file: "/external/AGENTS.md".to_string(),
        });
        config.custom_tools.push(app_config::CustomTool {
            id: "other-tool".to_string(),
            name: "Other Tool".to_string(),
            icon: "wrench".to_string(),
            config_dir: String::new(),
            rule_directory: "/other/rules".to_string(),
            global_rule_file: String::new(),
            skills_dir: String::new(),
            shared_skill_direct_read: false,
            mcp_config: String::new(),
            tool_config: String::new(),
            rule_file: String::new(),
        });
        config.managed_tools = vec!["custom-tool".to_string(), "other-tool".to_string()];
        config.handled_new_tool_ids = vec!["custom-tool".to_string(), "other-tool".to_string()];
        config.tool_paths.insert(
            "custom-tool".to_string(),
            app_config::ToolPaths {
                config_dir: "/external".to_string(),
                skills_dir: "/external/skills".to_string(),
            },
        );
        config.tool_capability_overrides.insert(
            "custom-tool".to_string(),
            app_config::ToolCapabilityOverrides {
                custom_rule_source_type: Some("directory".to_string()),
                custom_rule_source_path: Some("/external/rules".to_string()),
                custom_global_rule_target: Some("/external/AGENTS.md".to_string()),
                custom_mcp_config_path: Some("/external/mcp.json".to_string()),
                custom_tool_config_path: Some("/external/settings.json".to_string()),
                shared_skill_direct_read: None,
            },
        );
        config
            .injection_targets
            .insert("custom-tool".to_string(), "/external/AGENTS.md".to_string());

        remove_custom_tool_from_config(&mut config, "custom-tool");

        assert_eq!(config.custom_tools.len(), 1);
        assert_eq!(config.custom_tools[0].id, "other-tool");
        assert_eq!(config.managed_tools, vec!["other-tool"]);
        assert_eq!(config.handled_new_tool_ids, vec!["other-tool"]);
        assert!(!config.tool_paths.contains_key("custom-tool"));
        assert!(!config.tool_capability_overrides.contains_key("custom-tool"));
        assert!(!config.injection_targets.contains_key("custom-tool"));
    }
}
