// Purpose: Keep Community Skills inventory, file operations, and notes command wrappers in one frontend domain API.
import { invoke } from "$lib/shared/api/invoke.js";

/**
 * @param {string} toolId
 */
export async function listSkills(toolId) {
  return invoke("list_skills", { toolId });
}

export async function listAllSkills() {
  return invoke("list_all_skills");
}

/**
 * @param {string} skillPath
 */
export async function readSkillContent(skillPath) {
  return invoke("read_skill_content", { skillPath });
}

/**
 * @param {string} skillPath
 */
export async function listSkillFiles(skillPath) {
  return invoke("list_skill_files", { skillPath });
}

/**
 * @param {string} skillPath
 * @param {string} relativePath
 */
export async function readSkillFile(skillPath, relativePath) {
  return invoke("read_skill_file", { skillPath, relativePath });
}

/**
 * @param {string} skillPath
 * @param {string} relativePath
 * @param {string} content
 */
export async function writeSkillFile(skillPath, relativePath, content) {
  return invoke("write_skill_file", { skillPath, relativePath, content });
}

export async function listGenericSkills() {
  return invoke("list_generic_skills");
}

export async function listSkillsOverview() {
  return invoke("list_skills_overview");
}

export async function scanSkillInventory() {
  return invoke("scan_skill_inventory");
}

/**
 * @param {string} skillName
 */
export async function scanSkillInventoryEntry(skillName) {
  return invoke("scan_skill_inventory_entry", { skillName });
}

/**
 * @param {string} skillName
 */
export async function getSkillNote(skillName) {
  return invoke("get_skill_note", { skillName });
}

/**
 * @param {string} skillName
 * @param {string} note
 */
export async function setSkillNote(skillName, note) {
  return invoke("set_skill_note", { skillName, note });
}

export async function getAllSkillNotes() {
  return invoke("get_all_skill_notes");
}

/**
 * @param {string} skillName
 * @param {string} toolId
 * @param {string} mode - Community uses "symlink" for shared-directory install.
 * @param {string|null} sourcePath - absolute path to the source skill directory
 */
export async function previewInstall(skillName, toolId, mode, sourcePath = null) {
  return invoke("install_skill_v2", { skillName, toolId, mode, sourcePath, dryRun: true });
}

/**
 * @param {string} skillName
 * @param {string} toolId
 * @param {string} mode - Community uses "symlink" for shared-directory install.
 * @param {string|null} sourcePath - absolute path to the source skill directory
 */
export async function executeInstall(skillName, toolId, mode, sourcePath = null) {
  return invoke("install_skill_v2", { skillName, toolId, mode, sourcePath, dryRun: false });
}

/**
 * @param {string} skillName
 * @param {string} toolId - target tool
 * @param {string} sourcePath - absolute path to the source skill directory
 */
export async function previewCopySkillToTool(skillName, toolId, sourcePath) {
  return invoke("copy_skill_to_tool", { skillName, toolId, sourcePath, dryRun: true });
}

/**
 * @param {string} skillName
 * @param {string} toolId
 * @param {string} sourcePath
 */
export async function executeCopySkillToTool(skillName, toolId, sourcePath) {
  return invoke("copy_skill_to_tool", { skillName, toolId, sourcePath, dryRun: false });
}

/**
 * @param {string} skillName
 * @param {string} sourcePath
 * @param {string} newSkillName
 */
export async function previewRenameSkillSource(skillName, sourcePath, newSkillName) {
  return invoke("rename_skill_source", { skillName, sourcePath, newSkillName, dryRun: true });
}

/**
 * @param {string} skillName
 * @param {string} sourcePath
 * @param {string} newSkillName
 */
export async function executeRenameSkillSource(skillName, sourcePath, newSkillName) {
  return invoke("rename_skill_source", { skillName, sourcePath, newSkillName, dryRun: false });
}

/**
 * @param {string} skillName
 * @param {string} toolId
 */
export async function previewUninstall(skillName, toolId) {
  return invoke("uninstall_skill_v2", { skillName, toolId, dryRun: true });
}

/**
 * @param {string} skillName
 * @param {string} toolId
 */
export async function executeUninstall(skillName, toolId) {
  return invoke("uninstall_skill_v2", { skillName, toolId, dryRun: false });
}

/**
 * @param {string} skillName
 * @param {string} toolId
 * @param {string | null} [sourcePath=null]
 */
export async function previewDeleteFromTool(skillName, toolId, sourcePath = null) {
  return invoke("delete_skill_from_tool_v2", { skillName, toolId, sourcePath, dryRun: true });
}

/**
 * @param {string} skillName
 * @param {string} toolId
 * @param {string | null} [sourcePath=null]
 */
export async function executeDeleteFromTool(skillName, toolId, sourcePath = null) {
  return invoke("delete_skill_from_tool_v2", { skillName, toolId, sourcePath, dryRun: false });
}

/**
 * @param {string} skillName
 * @param {string} keepSourcePath
 * @param {string[]} deleteSourcePaths
 */
export async function previewCleanupDuplicateSkillSources(skillName, keepSourcePath, deleteSourcePaths) {
  return invoke("cleanup_duplicate_skill_sources", { skillName, keepSourcePath, deleteSourcePaths, dryRun: true });
}

/**
 * @param {string} skillName
 * @param {string} keepSourcePath
 * @param {string[]} deleteSourcePaths
 */
export async function executeCleanupDuplicateSkillSources(skillName, keepSourcePath, deleteSourcePaths) {
  return invoke("cleanup_duplicate_skill_sources", { skillName, keepSourcePath, deleteSourcePaths, dryRun: false });
}

/**
 * @param {string} skillName
 * @param {boolean} [allowGenericWrite=false]
 */
export async function previewDelete(skillName, allowGenericWrite = false) {
  return invoke("delete_skill_v2", { skillName, dryRun: true, allowGenericWrite });
}

/**
 * @param {string} skillName
 * @param {boolean} [allowGenericWrite=false]
 */
export async function executeDelete(skillName, allowGenericWrite = false) {
  return invoke("delete_skill_v2", { skillName, dryRun: false, allowGenericWrite });
}
