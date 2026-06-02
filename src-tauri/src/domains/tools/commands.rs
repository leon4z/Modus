// Purpose: Tauri command adapters for Tools domain behavior.

use super::*;
use crate::adapters::skills::ConfigEntry;
use crate::adapters::{DetectedTool, ToolRegistry};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn list_tools(registry: State<'_, Mutex<ToolRegistry>>) -> Vec<DetectedTool> {
    let registry = registry.lock().unwrap();
    list_tools_domain(&registry)
}

#[tauri::command]
pub fn refresh_tool(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
) -> Option<DetectedTool> {
    let registry = registry.lock().unwrap();
    refresh_tool_domain(&registry, tool_id)
}

#[tauri::command]
pub fn list_configs(registry: State<'_, Mutex<ToolRegistry>>, tool_id: String) -> Vec<ConfigEntry> {
    let registry = registry.lock().unwrap();
    list_configs_domain(&registry, tool_id)
}

#[tauri::command]
pub fn update_config(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
    key: String,
    value: String,
) -> Result<(), String> {
    let registry = registry.lock().unwrap();
    update_config_domain(&registry, tool_id, key, value)
}

#[tauri::command]
pub fn list_config_files(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
) -> Vec<ConfigFileDescriptor> {
    let registry = registry.lock().unwrap();
    list_config_files_domain(&registry, tool_id)
}

#[tauri::command]
pub fn read_config_file(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
    file_id: String,
) -> Result<ConfigFileContent, String> {
    let registry = registry.lock().unwrap();
    read_config_file_domain(&registry, tool_id, file_id)
}

#[tauri::command]
pub fn save_config_file(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
    file_id: String,
    content: String,
) -> Result<ConfigFileSaveResult, String> {
    let registry = registry.lock().unwrap();
    save_config_file_domain(&registry, tool_id, file_id, content)
}

#[tauri::command]
pub fn get_dashboard(registry: State<'_, Mutex<ToolRegistry>>) -> DashboardData {
    let registry = registry.lock().unwrap();
    get_dashboard_domain(&registry)
}

#[tauri::command]
pub fn get_tool_paths() -> HashMap<String, app_config::ToolPaths> {
    get_tool_paths_domain()
}

#[tauri::command]
pub fn get_tool_capability_overrides() -> HashMap<String, app_config::ToolCapabilityOverrides> {
    get_tool_capability_overrides_domain()
}

#[tauri::command]
pub fn set_tool_capability_overrides(
    tool_id: String,
    custom_rule_source_type: Option<String>,
    custom_rule_source_path: Option<String>,
    custom_global_rule_target: Option<String>,
    custom_mcp_config_path: Option<String>,
    custom_tool_config_path: Option<String>,
    shared_skill_direct_read: Option<bool>,
) -> Result<(), String> {
    set_tool_capability_overrides_domain(
        tool_id,
        custom_rule_source_type,
        custom_rule_source_path,
        custom_global_rule_target,
        custom_mcp_config_path,
        custom_tool_config_path,
        shared_skill_direct_read,
    )
}

#[tauri::command]
pub fn set_tool_path(
    tool_id: String,
    config_dir: String,
    skills_dir: String,
) -> Result<(), String> {
    set_tool_path_domain(tool_id, config_dir, skills_dir)
}

#[tauri::command]
pub fn get_custom_tools() -> Vec<app_config::CustomTool> {
    get_custom_tools_domain()
}

#[tauri::command]
pub fn add_custom_tool(tool: app_config::CustomTool) -> Result<(), String> {
    add_custom_tool_domain(tool)
}

#[tauri::command]
pub fn remove_custom_tool(tool_id: String) -> Result<(), String> {
    remove_custom_tool_domain(tool_id)
}

#[tauri::command]
pub fn get_managed_tools() -> Vec<String> {
    get_managed_tools_domain()
}

#[tauri::command]
pub fn get_handled_new_tool_ids() -> Vec<String> {
    get_handled_new_tool_ids_domain()
}

#[tauri::command]
pub fn set_handled_new_tool_ids(tool_ids: Vec<String>) -> Result<(), String> {
    set_handled_new_tool_ids_domain(tool_ids)
}

#[tauri::command]
pub fn set_managed_tools(tool_ids: Vec<String>) -> Result<(), String> {
    set_managed_tools_domain(tool_ids)
}
