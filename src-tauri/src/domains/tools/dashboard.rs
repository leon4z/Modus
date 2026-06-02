// Purpose: Own Dashboard aggregation over detected and managed tools.

use super::*;
use crate::adapters::skills::scan_skills_dir;
use crate::adapters::{RuleFormat, RuleSource, ToolRegistry};
use crate::domains::mcp::get_mcp_diagnostics_for_config_domain;
use crate::domains::skills::resolved_detected_tool_skills_dir;
use crate::domains::tools::canonical_managed_tool_ids;

pub(crate) fn get_dashboard_domain(registry: &ToolRegistry) -> DashboardData {
    let config = app_config::load_config();
    get_dashboard_for_config(registry, &config)
}

pub(crate) fn get_dashboard_for_config(
    registry: &ToolRegistry,
    config: &app_config::AppConfig,
) -> DashboardData {
    let all_tools = registry.detect_all_for_config(config);
    let managed_tools = canonical_managed_tool_ids(&config.managed_tools);

    let mut stats = vec![];
    let mut total_rules = 0;
    let mut total_skills = 0;
    let mut total_configs = 0;
    let mut total_mcp = 0;
    let mut detected_count = 0;

    for tool in &all_tools {
        if config.initialized && !managed_tools.contains(&tool.id) {
            continue;
        }

        let skill_count = if tool.detected {
            resolved_detected_tool_skills_dir(tool)
                .map(|dir| scan_skills_dir(&dir, &tool.id).len())
                .unwrap_or(0)
        } else {
            0
        };
        let config_count = if tool.detected {
            list_config_files_for_config_domain(registry, tool.id.clone(), config).len()
        } else {
            0
        };
        let rule_count = dashboard_rule_file_count(&tool.rule_sources);
        let mcp_count = if tool.detected {
            get_mcp_diagnostics_for_config_domain(registry, tool.id.clone(), config)
                .servers
                .len()
        } else {
            0
        };

        if tool.detected {
            detected_count += 1;
        }
        total_rules += rule_count;
        total_skills += skill_count;
        total_configs += config_count;
        total_mcp += mcp_count;

        stats.push(ToolStats {
            tool_id: tool.id.clone(),
            tool_name: tool.name.clone(),
            icon: tool.icon.clone(),
            detected: tool.detected,
            primary_config_health: tool.primary_config_health,
            rule_count,
            skill_count,
            config_count,
            mcp_count,
        });
    }

    DashboardData {
        tools: stats,
        total_rules,
        total_skills,
        total_configs,
        total_mcp,
        detected_count,
    }
}

fn dashboard_rule_file_count(rule_sources: &[RuleSource]) -> usize {
    rule_sources
        .iter()
        .filter(|source| !matches!(source.format, RuleFormat::Directory))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{PrimaryConfigHealth, RuleFormat, RuleSource, ToolAdapter};
    use std::path::PathBuf;

    struct DashboardTestAdapter {
        id: &'static str,
    }

    struct DashboardHealthAdapter {
        id: &'static str,
        detected: bool,
        config_dir: PathBuf,
    }

    struct DashboardRulesAdapter {
        rules: Vec<RuleSource>,
    }

    impl ToolAdapter for DashboardTestAdapter {
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
    }

    impl ToolAdapter for DashboardHealthAdapter {
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
            self.config_dir.clone()
        }

        fn detect(&self) -> bool {
            self.detected
        }

        fn read_rules(&self) -> Result<Vec<crate::adapters::RuleSource>, String> {
            Ok(vec![])
        }

        fn write_rule(&self, _path: &str, _content: &str) -> Result<(), String> {
            Ok(())
        }
    }

    impl ToolAdapter for DashboardRulesAdapter {
        fn id(&self) -> &str {
            "rules-tool"
        }

        fn name(&self) -> &str {
            "Rules Tool"
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
            Ok(self.rules.clone())
        }

        fn write_rule(&self, _path: &str, _content: &str) -> Result<(), String> {
            Ok(())
        }
    }

    fn test_registry() -> ToolRegistry {
        ToolRegistry::from_adapters_for_tests(vec![
            Box::new(DashboardTestAdapter { id: "tool-a" }),
            Box::new(DashboardTestAdapter { id: "tool-b" }),
        ])
    }

    #[test]
    fn initialized_empty_managed_scope_shows_no_dashboard_tools() {
        let registry = test_registry();
        let mut config = app_config::default_config();
        config.initialized = true;

        let dashboard = get_dashboard_for_config(&registry, &config);

        assert!(dashboard.tools.is_empty());
        assert_eq!(dashboard.detected_count, 0);
    }

    #[test]
    fn uninitialized_dashboard_can_still_show_detected_tools_for_onboarding() {
        let registry = test_registry();
        let config = app_config::default_config();

        let dashboard = get_dashboard_for_config(&registry, &config);

        assert_eq!(dashboard.tools.len(), 2);
        assert_eq!(dashboard.detected_count, 2);
    }

    #[test]
    fn initialized_dashboard_includes_only_enabled_tools() {
        let registry = test_registry();
        let mut config = app_config::default_config();
        config.initialized = true;
        config.managed_tools = vec!["tool-a".to_string()];

        let dashboard = get_dashboard_for_config(&registry, &config);

        assert_eq!(dashboard.tools.len(), 1);
        assert_eq!(dashboard.tools[0].tool_id, "tool-a");
        assert_eq!(dashboard.detected_count, 1);
    }

    #[test]
    fn initialized_dashboard_includes_enabled_custom_tool_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let skills_dir = tmp.path().join("skills");
        let skill_dir = skills_dir.join("demo");
        let mcp_config = tmp.path().join("mcp.json");
        let tool_config = tmp.path().join("settings.json");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(rules_dir.join("rule.md"), "# rule").unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "---\nname: demo\n---\n").unwrap();
        std::fs::write(&mcp_config, r#"{"mcpServers":{"demo":{"command":"node"}}}"#).unwrap();
        std::fs::write(&tool_config, "{}").unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![]);
        let mut config = app_config::default_config();
        config.initialized = true;
        config.managed_tools = vec!["custom-dashboard".to_string()];
        config
            .custom_tools
            .push(crate::platform::config::CustomTool {
                id: "custom-dashboard".to_string(),
                name: "Custom Dashboard".to_string(),
                icon: "wrench".to_string(),
                config_dir: String::new(),
                rule_directory: rules_dir.to_string_lossy().to_string(),
                global_rule_file: String::new(),
                skills_dir: skills_dir.to_string_lossy().to_string(),
                shared_skill_direct_read: false,
                mcp_config: mcp_config.to_string_lossy().to_string(),
                tool_config: tool_config.to_string_lossy().to_string(),
                rule_file: String::new(),
            });

        let dashboard = get_dashboard_for_config(&registry, &config);

        assert_eq!(dashboard.tools.len(), 1);
        assert_eq!(dashboard.tools[0].tool_id, "custom-dashboard");
        assert_eq!(dashboard.tools[0].rule_count, 1);
        assert_eq!(dashboard.tools[0].skill_count, 1);
        assert_eq!(dashboard.tools[0].config_count, 1);
        assert_eq!(dashboard.tools[0].mcp_count, 1);
        assert_eq!(dashboard.detected_count, 1);
    }

    #[test]
    fn dashboard_rule_count_excludes_directory_group_placeholders() {
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(DashboardRulesAdapter {
                rules: vec![
                    RuleSource {
                        path: "/tool/workspace".to_string(),
                        format: RuleFormat::Directory,
                        content: String::new(),
                        last_modified: 0,
                        label: "workspace".to_string(),
                        group: "workspace".to_string(),
                        diagnostic: None,
                    },
                    RuleSource {
                        path: "/tool/workspace/AGENTS.md".to_string(),
                        format: RuleFormat::DirectoryMarkdown,
                        content: "agents".to_string(),
                        last_modified: 0,
                        label: "AGENTS.md".to_string(),
                        group: "workspace".to_string(),
                        diagnostic: None,
                    },
                    RuleSource {
                        path: "/tool/RULES.md".to_string(),
                        format: RuleFormat::SingleMarkdown,
                        content: "rules".to_string(),
                        last_modified: 0,
                        label: "RULES.md".to_string(),
                        group: String::new(),
                        diagnostic: None,
                    },
                ],
            })]);

        let dashboard = get_dashboard_for_config(&registry, &app_config::default_config());

        assert_eq!(dashboard.tools[0].rule_count, 2);
        assert_eq!(dashboard.total_rules, 2);
    }

    #[test]
    fn dashboard_exposes_missing_primary_config_health_for_present_tool() {
        let tmp = tempfile::tempdir().unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(DashboardHealthAdapter {
                id: "tool-a",
                detected: true,
                config_dir: tmp.path().join("missing-config"),
            })]);
        let mut config = app_config::default_config();
        config.initialized = true;
        config.managed_tools = vec!["tool-a".to_string()];

        let dashboard = get_dashboard_for_config(&registry, &config);

        assert_eq!(dashboard.tools.len(), 1);
        assert_eq!(
            dashboard.tools[0].primary_config_health,
            PrimaryConfigHealth::Missing
        );
        assert_eq!(dashboard.detected_count, 1);
    }

    #[test]
    fn dashboard_uses_user_config_dir_override_for_primary_config_health() {
        let tmp = tempfile::tempdir().unwrap();
        let override_config = tmp.path().join("override-config");
        std::fs::create_dir_all(&override_config).unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(DashboardHealthAdapter {
                id: "tool-a",
                detected: true,
                config_dir: tmp.path().join("default-missing-config"),
            })]);
        let mut config = app_config::default_config();
        config.initialized = true;
        config.managed_tools = vec!["tool-a".to_string()];
        config.tool_paths.insert(
            "tool-a".to_string(),
            app_config::ToolPaths {
                config_dir: override_config.to_string_lossy().to_string(),
                skills_dir: String::new(),
            },
        );

        let dashboard = get_dashboard_for_config(&registry, &config);

        assert_eq!(dashboard.tools.len(), 1);
        assert_eq!(
            dashboard.tools[0].primary_config_health,
            PrimaryConfigHealth::Ok
        );
        assert_eq!(dashboard.detected_count, 1);
    }

    #[test]
    fn dashboard_ignores_leftover_config_health_for_absent_tool() {
        let tmp = tempfile::tempdir().unwrap();
        let leftover = tmp.path().join("leftover-config");
        std::fs::create_dir_all(&leftover).unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(DashboardHealthAdapter {
                id: "tool-a",
                detected: false,
                config_dir: leftover,
            })]);
        let mut config = app_config::default_config();
        config.initialized = true;
        config.managed_tools = vec!["tool-a".to_string()];

        let dashboard = get_dashboard_for_config(&registry, &config);

        assert_eq!(dashboard.tools.len(), 1);
        assert!(!dashboard.tools[0].detected);
        assert_eq!(
            dashboard.tools[0].primary_config_health,
            PrimaryConfigHealth::Unknown
        );
        assert_eq!(dashboard.detected_count, 0);
    }
}
