// Purpose: Keep tool discovery and tool-management command wrappers in one frontend domain API.
import { invoke } from "$lib/shared/api/invoke.js";

export async function listTools() {
  return invoke("list_tools");
}

/**
 * @param {string} toolId
 */
export async function refreshTool(toolId) {
  return invoke("refresh_tool", { toolId });
}

export async function getInjectionTargets() {
  return invoke("get_injection_targets");
}

/**
 * @param {string} toolId
 * @param {string} path
 */
export async function setInjectionTarget(toolId, path) {
  return invoke("set_injection_target", { toolId, path });
}

export async function getToolPaths() {
  return invoke("get_tool_paths");
}

export async function getToolCapabilityOverrides() {
  try {
    return await invoke("get_tool_capability_overrides");
  } catch (error) {
    if (String(error || "").includes("get_tool_capability_overrides")) return {};
    throw error;
  }
}

/**
 * @param {string} toolId
 * @param {{ customRuleSourceType?: string | null, customRuleSourcePath?: string | null, customGlobalRuleTarget?: string | null, customMcpConfigPath?: string | null, customToolConfigPath?: string | null, sharedSkillDirectRead?: boolean | null }} overrides
 */
export async function setToolCapabilityOverrides(toolId, overrides) {
  return invoke("set_tool_capability_overrides", {
    toolId,
    customRuleSourceType: overrides.customRuleSourceType ?? null,
    customRuleSourcePath: overrides.customRuleSourcePath ?? null,
    customGlobalRuleTarget: overrides.customGlobalRuleTarget ?? null,
    customMcpConfigPath: overrides.customMcpConfigPath ?? null,
    customToolConfigPath: overrides.customToolConfigPath ?? null,
    sharedSkillDirectRead: overrides.sharedSkillDirectRead ?? null,
  });
}

/**
 * @param {string} toolId
 * @param {string} configDir
 * @param {string} skillsDir
 */
export async function setToolPath(toolId, configDir, skillsDir) {
  return invoke("set_tool_path", { toolId, configDir, skillsDir });
}

export async function getCustomTools() {
  return invoke("get_custom_tools");
}

/**
 * @param {object} tool
 */
export async function addCustomTool(tool) {
  return invoke("add_custom_tool", { tool });
}

/**
 * @param {string} toolId
 */
export async function removeCustomTool(toolId) {
  return invoke("remove_custom_tool", { toolId });
}

export async function getManagedTools() {
  return invoke("get_managed_tools");
}

export async function getHandledNewToolIds() {
  return invoke("get_handled_new_tool_ids");
}

/**
 * @param {string[]} toolIds
 */
export async function setHandledNewToolIds(toolIds) {
  return invoke("set_handled_new_tool_ids", { toolIds });
}

/**
 * @param {string[]} toolIds
 */
export async function setManagedTools(toolIds) {
  return invoke("set_managed_tools", { toolIds });
}
