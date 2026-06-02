// Purpose: Thin Tauri command adapters for the Skill management domain.

use super::*;
use crate::adapters::ToolRegistry;
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn list_skills(registry: State<'_, Mutex<ToolRegistry>>, tool_id: String) -> Vec<SkillInfo> {
    let registry = registry.lock().unwrap();
    super::list_skills_domain(&registry, tool_id)
}

#[tauri::command]
pub fn list_all_skills(registry: State<'_, Mutex<ToolRegistry>>) -> Vec<SkillInfo> {
    let registry = registry.lock().unwrap();
    super::list_all_skills_domain(&registry)
}

#[tauri::command]
pub fn list_generic_skills(registry: State<'_, Mutex<ToolRegistry>>) -> Vec<SkillInfo> {
    let registry = registry.lock().unwrap();
    super::list_generic_skills_domain(&registry)
}

#[tauri::command]
pub fn write_skill_file(
    registry: State<'_, Mutex<ToolRegistry>>,
    skill_path: String,
    relative_path: String,
    content: String,
) -> Result<(), String> {
    let registry = registry.lock().unwrap();
    super::write_skill_file_domain(&registry, skill_path, relative_path, content)
}

#[tauri::command]
pub fn scan_skill_inventory(
    registry: State<'_, Mutex<ToolRegistry>>,
) -> Result<SkillInventory, String> {
    let registry = registry.lock().unwrap();
    super::scan_skill_inventory_domain(&registry)
}

#[tauri::command]
pub fn scan_skill_inventory_entry(
    registry: State<'_, Mutex<ToolRegistry>>,
    skill_name: String,
) -> Result<SkillEntry, String> {
    let registry = registry.lock().unwrap();
    super::scan_skill_inventory_entry_domain(&registry, skill_name)
}

#[tauri::command]
pub fn list_skills_overview(
    registry: State<'_, Mutex<ToolRegistry>>,
) -> Result<Vec<SkillOverviewItem>, String> {
    let registry = registry.lock().unwrap();
    super::list_skills_overview_domain(&registry)
}

#[tauri::command]
pub fn read_skill_content(skill_path: String) -> Option<SkillInfo> {
    super::read_skill_content_domain(skill_path)
}

#[tauri::command]
pub fn list_skill_files(skill_path: String) -> Vec<SkillFileEntry> {
    super::list_skill_files_domain(skill_path)
}

#[tauri::command]
pub fn read_skill_file(skill_path: String, relative_path: String) -> Result<String, String> {
    super::read_skill_file_domain(skill_path, relative_path)
}

#[tauri::command]
pub fn install_skill_v2(
    registry: State<'_, Mutex<ToolRegistry>>,
    skill_name: String,
    tool_id: String,
    mode: String,
    source_path: Option<String>,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let registry = registry.lock().unwrap();
    super::install_skill_v2_domain(&registry, skill_name, tool_id, mode, source_path, dry_run)
}

#[tauri::command]
pub fn copy_skill_to_tool(
    registry: State<'_, Mutex<ToolRegistry>>,
    skill_name: String,
    tool_id: String,
    source_path: String,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let registry = registry.lock().unwrap();
    super::copy_skill_to_tool_domain(&registry, skill_name, tool_id, source_path, dry_run)
}

#[tauri::command]
pub fn link_shared_skill_to_tool(
    registry: State<'_, Mutex<ToolRegistry>>,
    skill_name: String,
    tool_id: String,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let registry = registry.lock().unwrap();
    super::link_shared_skill_to_tool_domain(&registry, skill_name, tool_id, dry_run)
}

#[tauri::command]
pub fn rename_skill_source(
    registry: State<'_, Mutex<ToolRegistry>>,
    skill_name: String,
    source_path: String,
    new_skill_name: String,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let registry = registry.lock().unwrap();
    super::rename_skill_source_domain(&registry, skill_name, source_path, new_skill_name, dry_run)
}

#[tauri::command]
pub fn uninstall_skill_v2(
    registry: State<'_, Mutex<ToolRegistry>>,
    skill_name: String,
    tool_id: String,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let registry = registry.lock().unwrap();
    super::uninstall_skill_v2_domain(&registry, skill_name, tool_id, dry_run)
}

#[tauri::command]
pub fn delete_skill_from_tool_v2(
    registry: State<'_, Mutex<ToolRegistry>>,
    skill_name: String,
    tool_id: String,
    source_path: Option<String>,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let registry = registry.lock().unwrap();
    super::delete_skill_from_tool_v2_domain(&registry, skill_name, tool_id, source_path, dry_run)
}

#[tauri::command]
pub fn cleanup_duplicate_skill_sources(
    registry: State<'_, Mutex<ToolRegistry>>,
    skill_name: String,
    keep_source_path: String,
    delete_source_paths: Vec<String>,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let registry = registry.lock().unwrap();
    super::cleanup_duplicate_skill_sources_domain(
        &registry,
        skill_name,
        keep_source_path,
        delete_source_paths,
        dry_run,
    )
}

#[tauri::command]
pub fn delete_skill_v2(
    registry: State<'_, Mutex<ToolRegistry>>,
    skill_name: String,
    dry_run: bool,
    allow_generic_write: Option<bool>,
) -> Result<OperationPreview, String> {
    let registry = registry.lock().unwrap();
    super::delete_skill_v2_domain(&registry, skill_name, dry_run, allow_generic_write)
}
