// Purpose: Keep application logging command wrappers in one frontend domain API.
import { invoke } from "$lib/shared/api/invoke.js";

/**
 * @param {object} event
 */
export async function writeApplicationLog(event) {
  return invoke("write_application_log", { event });
}

export async function getApplicationLogPath() {
  return invoke("get_application_log_path");
}

export async function listApplicationLogs() {
  return invoke("list_application_logs");
}

/**
 * @param {string} id
 */
export async function readApplicationLog(id) {
  return invoke("read_application_log", { id });
}

/**
 * @param {string[]} ids
 * @param {string} destination
 */
export async function exportApplicationLogs(ids, destination) {
  return invoke("export_application_logs", { ids, destination });
}

/**
 * @param {object} event
 */
export async function writeSkillPerformanceLog(event) {
  return writeModulePerformanceLog({ ...event, module: "skills" });
}

export async function getSkillPerformanceLogPath() {
  return getModulePerformanceLogPath();
}

/**
 * @param {object} event
 */
export async function writeModulePerformanceLog(event) {
  return invoke("write_module_performance_log", { event });
}

export async function getModulePerformanceLogPath() {
  return invoke("get_module_performance_log_path");
}

export async function listModulePerformanceLogs() {
  return invoke("list_module_performance_logs");
}

/**
 * @param {string} id
 */
export async function readModulePerformanceLog(id) {
  return invoke("read_module_performance_log", { id });
}

/**
 * @param {string[]} ids
 * @param {string} destination
 */
export async function exportModulePerformanceLogs(ids, destination) {
  return invoke("export_module_performance_logs", { ids, destination });
}
