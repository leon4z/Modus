// Purpose: Tauri command adapters for Rules domain behavior.

use super::*;
use crate::adapters::ToolRegistry;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn write_rule(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
    path: String,
    content: String,
) -> Result<(), String> {
    let registry = registry.lock().unwrap();
    write_rule_domain(&registry, tool_id, path, content)
}

#[tauri::command]
pub fn create_rule_file(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
    path: String,
    content: String,
) -> Result<(), String> {
    let registry = registry.lock().unwrap();
    create_rule_file_domain(&registry, tool_id, path, content)
}

#[tauri::command]
pub fn create_rule_directory(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
    path: String,
) -> Result<(), String> {
    let registry = registry.lock().unwrap();
    create_rule_directory_domain(&registry, tool_id, path)
}

#[tauri::command]
pub fn delete_rule_entry(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
    path: String,
    dry_run: bool,
) -> Result<RuleFileChangePreview, String> {
    let registry = registry.lock().unwrap();
    delete_rule_entry_domain(&registry, tool_id, path, dry_run)
}

#[tauri::command]
pub fn rename_rule_entry(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
    path: String,
    new_name: String,
) -> Result<String, String> {
    let registry = registry.lock().unwrap();
    rename_rule_entry_domain(&registry, tool_id, path, new_name)
}

#[tauri::command]
pub fn copy_rule(
    registry: State<'_, Mutex<ToolRegistry>>,
    _source_tool_id: String,
    target_tool_id: String,
    target_path: String,
    content: String,
    append: bool,
) -> Result<(), String> {
    let registry = registry.lock().unwrap();
    copy_rule_domain(&registry, target_tool_id, target_path, content, append)
}

#[tauri::command]
pub fn diff_rules(
    left_content: String,
    left_label: String,
    right_content: String,
    right_label: String,
) -> DiffResult {
    diff_rules_domain(left_content, left_label, right_content, right_label)
}

#[tauri::command]
pub fn read_rule_content(path: String) -> Result<String, String> {
    read_rule_content_domain(path)
}

#[tauri::command]
pub fn list_default_rules() -> Vec<app_config::DefaultRule> {
    list_default_rules_domain()
}

#[tauri::command]
pub fn get_default_rule_injection_baselines() -> app_config::DefaultRuleInjectionBaselines {
    get_default_rule_injection_baselines_domain()
}

#[tauri::command]
pub fn set_default_rule_injection_baselines(
    baselines: app_config::DefaultRuleInjectionBaselines,
) -> Result<(), String> {
    set_default_rule_injection_baselines_domain(baselines)
}

#[tauri::command]
pub fn save_default_rule(rule: app_config::DefaultRule) -> Result<(), String> {
    save_default_rule_domain(rule)
}

#[tauri::command]
pub fn delete_default_rule(rule_id: String) -> Result<(), String> {
    delete_default_rule_domain(rule_id)
}

#[tauri::command]
pub fn inject_default_rules(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
) -> Result<String, String> {
    let registry = registry.lock().unwrap();
    inject_default_rules_domain(&registry, tool_id)
}

#[tauri::command]
pub fn get_managed_rules_state(
    registry: State<'_, Mutex<ToolRegistry>>,
) -> Result<ManagedRulesState, String> {
    let registry = registry.lock().unwrap();
    Ok(get_managed_rules_state_domain(&registry))
}

#[tauri::command]
pub fn adopt_rule_management_targets(
    registry: State<'_, Mutex<ToolRegistry>>,
    rule_id: String,
    tool_ids: Vec<String>,
) -> Result<ManagedRulesActionResult, String> {
    let registry = registry.lock().unwrap();
    adopt_rule_management_targets_domain(&registry, rule_id, tool_ids)
}

#[tauri::command]
pub fn sync_managed_rule_targets(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_ids: Vec<String>,
) -> Result<ManagedRulesActionResult, String> {
    let registry = registry.lock().unwrap();
    sync_managed_rule_targets_domain(&registry, tool_ids)
}

#[tauri::command]
pub fn leave_rule_management_targets(
    registry: State<'_, Mutex<ToolRegistry>>,
    rule_id: String,
    tool_ids: Vec<String>,
    remove_managed_block: bool,
    dry_run: Option<bool>,
) -> Result<ManagedRulesActionResult, String> {
    let registry = registry.lock().unwrap();
    leave_rule_management_targets_domain(
        &registry,
        rule_id,
        tool_ids,
        remove_managed_block,
        dry_run.unwrap_or(false),
    )
}

#[tauri::command]
pub fn get_injection_targets() -> HashMap<String, String> {
    get_injection_targets_domain()
}

#[tauri::command]
pub fn set_injection_target(tool_id: String, path: String) -> Result<(), String> {
    set_injection_target_domain(tool_id, path)
}
