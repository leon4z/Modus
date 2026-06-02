// Purpose: Tauri command adapters for Settings domain behavior.

use super::*;

#[tauri::command]
pub fn is_initialized() -> bool {
    is_initialized_domain()
}

#[tauri::command]
pub fn get_language() -> String {
    get_language_domain()
}

#[tauri::command]
pub fn set_language(lang: String) -> Result<(), String> {
    set_language_domain(lang)
}

#[tauri::command]
pub fn get_theme() -> String {
    get_theme_domain()
}

#[tauri::command]
pub fn set_theme(theme: String) -> Result<(), String> {
    set_theme_domain(theme)
}

#[tauri::command]
pub fn get_runtime_info() -> crate::platform::env::RuntimeInfo {
    crate::platform::env::runtime_info()
}

#[tauri::command]
pub fn get_skill_performance_diagnostics_enabled() -> bool {
    get_skill_performance_diagnostics_enabled_domain()
}

#[tauri::command]
pub fn set_skill_performance_diagnostics_enabled(enabled: bool) -> Result<(), String> {
    set_skill_performance_diagnostics_enabled_domain(enabled)
}

#[tauri::command]
pub fn get_module_performance_diagnostics_enabled() -> bool {
    get_module_performance_diagnostics_enabled_domain()
}

#[tauri::command]
pub fn set_module_performance_diagnostics_enabled(enabled: bool) -> Result<(), String> {
    set_module_performance_diagnostics_enabled_domain(enabled)
}
