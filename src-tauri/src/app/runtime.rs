// Purpose: Compose the desktop runtime and desktop command registration.

use crate::adapters::ToolRegistry;
use crate::commands;
use std::sync::Mutex;

use tauri::Manager;
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let registry = ToolRegistry::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            #[cfg(target_os = "macos")]
            apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, None)
                .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

            Ok(())
        })
        .manage(crate::domains::app_updates::pending_app_update_state())
        .manage(Mutex::new(registry))
        .invoke_handler(tauri::generate_handler![
            commands::app_updates::get_app_update_state,
            commands::app_updates::check_app_update,
            commands::app_updates::install_app_update,
            commands::app_updates::skip_app_update,
            commands::app_updates::restart_app_for_update,
            commands::tools::list_tools,
            commands::tools::refresh_tool,
            commands::rules::write_rule,
            commands::rules::create_rule_file,
            commands::rules::create_rule_directory,
            commands::rules::delete_rule_entry,
            commands::rules::rename_rule_entry,
            commands::rules::copy_rule,
            commands::rules::diff_rules,
            commands::rules::read_rule_content,
            commands::skills::list_skills,
            commands::skills::list_all_skills,
            commands::skills::list_generic_skills,
            commands::skills::scan_skill_inventory,
            commands::skills::scan_skill_inventory_entry,
            commands::skills::list_skills_overview,
            commands::skills::read_skill_content,
            commands::skills::list_skill_files,
            commands::skills::read_skill_file,
            commands::skills::write_skill_file,
            commands::skills::install_skill_v2,
            commands::skills::copy_skill_to_tool,
            commands::skills::link_shared_skill_to_tool,
            commands::skills::rename_skill_source,
            commands::skills::uninstall_skill_v2,
            commands::skills::delete_skill_from_tool_v2,
            commands::skills::cleanup_duplicate_skill_sources,
            commands::skills::delete_skill_v2,
            commands::tools::list_configs,
            commands::tools::update_config,
            commands::tools::list_config_files,
            commands::tools::read_config_file,
            commands::tools::save_config_file,
            commands::tools::get_dashboard,
            commands::logging::write_application_log,
            commands::logging::get_application_log_path,
            commands::logging::list_application_logs,
            commands::logging::read_application_log,
            commands::logging::export_application_logs,
            commands::logging::write_skill_performance_log,
            commands::logging::get_skill_performance_log_path,
            commands::logging::write_module_performance_log,
            commands::logging::get_module_performance_log_path,
            commands::logging::list_module_performance_logs,
            commands::logging::read_module_performance_log,
            commands::logging::export_module_performance_logs,
            commands::app_settings::get_skill_note,
            commands::app_settings::set_skill_note,
            commands::app_settings::get_all_skill_notes,
            commands::rules::list_default_rules,
            commands::rules::get_default_rule_injection_baselines,
            commands::rules::set_default_rule_injection_baselines,
            commands::rules::save_default_rule,
            commands::rules::delete_default_rule,
            commands::rules::inject_default_rules,
            commands::rules::get_managed_rules_state,
            commands::rules::adopt_rule_management_targets,
            commands::rules::sync_managed_rule_targets,
            commands::rules::leave_rule_management_targets,
            commands::rules::get_injection_targets,
            commands::rules::set_injection_target,
            commands::tools::get_tool_paths,
            commands::tools::get_tool_capability_overrides,
            commands::tools::set_tool_capability_overrides,
            commands::tools::set_tool_path,
            commands::tools::get_custom_tools,
            commands::tools::add_custom_tool,
            commands::tools::remove_custom_tool,
            commands::tools::get_managed_tools,
            commands::tools::get_handled_new_tool_ids,
            commands::tools::set_handled_new_tool_ids,
            commands::tools::set_managed_tools,
            commands::settings::is_initialized,
            commands::settings::get_language,
            commands::settings::set_language,
            commands::settings::get_theme,
            commands::settings::set_theme,
            commands::settings::get_runtime_info,
            commands::settings::get_skill_performance_diagnostics_enabled,
            commands::settings::set_skill_performance_diagnostics_enabled,
            commands::settings::get_module_performance_diagnostics_enabled,
            commands::settings::set_module_performance_diagnostics_enabled,
            commands::mcp::list_mcp_servers,
            commands::mcp::get_mcp_diagnostics,
            commands::mcp::list_mcp_config_sources,
            commands::mcp::read_mcp_server_config_fragment,
            commands::mcp::save_mcp_server_config_fragment,
            commands::translation::get_translation_provider_config,
            commands::translation::set_translation_provider_config,
            commands::translation::set_translation_api_key,
            commands::translation::clear_translation_api_key,
            commands::translation::test_translation_provider,
            commands::translation::translate_markdown,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
