// Purpose: Keep Rules command wrappers in one frontend domain API.
import { invoke } from "$lib/shared/api/invoke.js";

/**
 * @param {string} toolId
 * @param {string} path
 * @param {string} content
 */
export async function writeRule(toolId, path, content) {
  return invoke("write_rule", { toolId, path, content });
}

/**
 * @param {string} toolId
 * @param {string} path
 * @param {string} content
 */
export async function createRuleFile(toolId, path, content = "") {
  return invoke("create_rule_file", { toolId, path, content });
}

/**
 * @param {string} toolId
 * @param {string} path
 */
export async function createRuleDirectory(toolId, path) {
  return invoke("create_rule_directory", { toolId, path });
}

/**
 * @param {string} toolId
 * @param {string} path
 * @param {boolean} dryRun
 */
export async function deleteRuleEntry(toolId, path, dryRun = true) {
  return invoke("delete_rule_entry", { toolId, path, dryRun });
}

/**
 * @param {string} toolId
 * @param {string} path
 * @param {string} newName
 */
export async function renameRuleEntry(toolId, path, newName) {
  return invoke("rename_rule_entry", { toolId, path, newName });
}

/**
 * @param {string} path
 */
export async function readRuleContent(path) {
  return invoke("read_rule_content", { path });
}

/**
 * @param {string} sourceToolId
 * @param {string} targetToolId
 * @param {string} targetPath
 * @param {string} content
 * @param {boolean} append
 */
export async function copyRule(sourceToolId, targetToolId, targetPath, content, append) {
  return invoke("copy_rule", { sourceToolId, targetToolId, targetPath, content, append });
}

/**
 * @param {string} leftContent
 * @param {string} leftLabel
 * @param {string} rightContent
 * @param {string} rightLabel
 */
export async function diffRules(leftContent, leftLabel, rightContent, rightLabel) {
  return invoke("diff_rules", { leftContent, leftLabel, rightContent, rightLabel });
}

export async function listDefaultRules() {
  return invoke("list_default_rules");
}

export async function getDefaultRuleInjectionBaselines() {
  return invoke("get_default_rule_injection_baselines");
}

/**
 * @param {object} baselines
 */
export async function setDefaultRuleInjectionBaselines(baselines) {
  return invoke("set_default_rule_injection_baselines", { baselines });
}

/**
 * @param {object} rule
 */
export async function saveDefaultRule(rule) {
  return invoke("save_default_rule", { rule });
}

/**
 * @param {string} ruleId
 */
export async function deleteDefaultRule(ruleId) {
  return invoke("delete_default_rule", { ruleId });
}

/**
 * @param {string} toolId
 */
export async function injectDefaultRules(toolId) {
  return invoke("inject_default_rules", { toolId });
}

export async function getManagedRulesState() {
  return invoke("get_managed_rules_state");
}

/**
 * @param {string} ruleId
 * @param {string[]} toolIds
 */
export async function adoptRuleManagementTargets(ruleId, toolIds) {
  return invoke("adopt_rule_management_targets", { ruleId, toolIds });
}

/**
 * @param {string[]} toolIds
 */
export async function syncManagedRuleTargets(toolIds) {
  return invoke("sync_managed_rule_targets", { toolIds });
}

/**
 * @param {string} ruleId
 * @param {string[]} toolIds
 * @param {{ removeManagedBlock?: boolean, dryRun?: boolean }} [options]
 */
export async function leaveRuleManagementTargets(ruleId, toolIds, options = {}) {
  return invoke("leave_rule_management_targets", {
    ruleId,
    toolIds,
    removeManagedBlock: Boolean(options.removeManagedBlock),
    dryRun: Boolean(options.dryRun),
  });
}
