// Purpose: Tauri command adapter for MCP domain behavior.

use super::*;
use crate::adapters::skills::McpServerInfo;
use crate::adapters::ToolRegistry;
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn list_mcp_servers(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
) -> Vec<McpServerInfo> {
    let registry = registry.lock().unwrap();
    list_mcp_servers_domain(&registry, tool_id)
}

#[tauri::command]
pub fn get_mcp_diagnostics(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
) -> McpDiagnosticResult {
    let registry = registry.lock().unwrap();
    get_mcp_diagnostics_domain(&registry, tool_id)
}

#[tauri::command]
pub fn list_mcp_config_sources(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
) -> Vec<McpConfigSourceInfo> {
    let registry = registry.lock().unwrap();
    list_mcp_config_sources_domain(&registry, tool_id)
}

#[tauri::command]
pub fn read_mcp_server_config_fragment(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
    source_id: String,
    server_name: String,
) -> Result<McpServerConfigFragment, String> {
    let registry = registry.lock().unwrap();
    read_mcp_server_config_fragment_domain(&registry, tool_id, source_id, server_name)
}

#[tauri::command]
pub fn save_mcp_server_config_fragment(
    registry: State<'_, Mutex<ToolRegistry>>,
    tool_id: String,
    source_id: String,
    server_name: String,
    content: String,
) -> Result<McpServerConfigSaveResult, String> {
    let registry = registry.lock().unwrap();
    save_mcp_server_config_fragment_domain(&registry, tool_id, source_id, server_name, content)
}
