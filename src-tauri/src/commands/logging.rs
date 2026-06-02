// Purpose: Compatibility command surface for application logging commands.

use crate::domains::logging;
use crate::platform::logging::{LogEvent, ModulePerformanceLogEvent, SkillPerformanceLogEvent};

#[tauri::command]
pub fn write_application_log(event: LogEvent) -> Result<(), String> {
    logging::write_application_log(event)
}

#[tauri::command]
pub fn get_application_log_path() -> Result<String, String> {
    logging::get_application_log_path()
}

#[tauri::command]
pub fn list_application_logs() -> Result<Vec<crate::platform::logging::ManagedLogFile>, String> {
    logging::list_application_logs()
}

#[tauri::command]
pub fn read_application_log(
    id: String,
) -> Result<crate::platform::logging::ManagedLogReadResult, String> {
    logging::read_application_log(id)
}

#[tauri::command]
pub fn export_application_logs(ids: Vec<String>, destination: String) -> Result<String, String> {
    logging::export_application_logs(ids, destination)
}

#[tauri::command]
pub fn write_skill_performance_log(event: SkillPerformanceLogEvent) -> Result<(), String> {
    logging::write_skill_performance_log(event)
}

#[tauri::command]
pub fn get_skill_performance_log_path() -> Result<String, String> {
    logging::get_skill_performance_log_path()
}

#[tauri::command]
pub fn write_module_performance_log(event: ModulePerformanceLogEvent) -> Result<(), String> {
    logging::write_module_performance_log(event)
}

#[tauri::command]
pub fn get_module_performance_log_path() -> Result<String, String> {
    logging::get_module_performance_log_path()
}

#[tauri::command]
pub fn list_module_performance_logs(
) -> Result<Vec<crate::platform::logging::ManagedLogFile>, String> {
    logging::list_module_performance_logs()
}

#[tauri::command]
pub fn read_module_performance_log(
    id: String,
) -> Result<crate::platform::logging::ManagedLogReadResult, String> {
    logging::read_module_performance_log(id)
}

#[tauri::command]
pub fn export_module_performance_logs(
    ids: Vec<String>,
    destination: String,
) -> Result<String, String> {
    logging::export_module_performance_logs(ids, destination)
}
