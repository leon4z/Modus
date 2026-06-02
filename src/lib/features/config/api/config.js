// Purpose: Keep tool configuration command wrappers in one frontend domain API.
import { invoke } from "$lib/shared/api/invoke.js";

/**
 * @param {string} toolId
 */
export async function listConfigFiles(toolId) {
  return invoke("list_config_files", { toolId });
}

/**
 * @param {string} toolId
 * @param {string} fileId
 */
export async function readConfigFile(toolId, fileId) {
  return invoke("read_config_file", { toolId, fileId });
}

/**
 * @param {string} toolId
 * @param {string} fileId
 * @param {string} content
 */
export async function saveConfigFile(toolId, fileId, content) {
  return invoke("save_config_file", { toolId, fileId, content });
}
