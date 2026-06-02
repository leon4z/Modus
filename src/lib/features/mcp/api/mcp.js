// Purpose: Keep MCP command wrappers in one frontend domain API.
import { invoke } from "$lib/shared/api/invoke.js";

/**
 * @param {string} toolId
 */
export async function listMcpServers(toolId) {
  return invoke("list_mcp_servers", { toolId });
}

/**
 * @param {string} toolId
 */
export async function getMcpDiagnostics(toolId) {
  return invoke("get_mcp_diagnostics", { toolId });
}

/**
 * @param {string} toolId
 */
export async function listMcpConfigSources(toolId) {
  return invoke("list_mcp_config_sources", { toolId });
}

/**
 * @param {string} toolId
 * @param {string} sourceId
 * @param {string} serverName
 */
export async function readMcpServerConfigFragment(toolId, sourceId, serverName) {
  return invoke("read_mcp_server_config_fragment", { toolId, sourceId, serverName });
}

/**
 * @param {string} toolId
 * @param {string} sourceId
 * @param {string} serverName
 * @param {string} content
 */
export async function saveMcpServerConfigFragment(toolId, sourceId, serverName, content) {
  return invoke("save_mcp_server_config_fragment", { toolId, sourceId, serverName, content });
}
