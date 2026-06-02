// Purpose: Keep general application settings command wrappers in one frontend domain API.
import { invoke } from "$lib/shared/api/invoke.js";

export async function isInitialized() {
  return invoke("is_initialized");
}

export async function getLanguage() {
  return invoke("get_language");
}

/**
 * @param {string} lang
 */
export async function setLanguage(lang) {
  return invoke("set_language", { lang });
}

export async function getTheme() {
  return invoke("get_theme");
}

export async function getRuntimeInfo() {
  return invoke("get_runtime_info");
}

/**
 * @param {string} theme - "light", "dark", or "system"
 */
export async function setTheme(theme) {
  return invoke("set_theme", { theme });
}

export async function getSkillPerformanceDiagnosticsEnabled() {
  return getModulePerformanceDiagnosticsEnabled();
}

/**
 * @param {boolean} enabled
 */
export async function setSkillPerformanceDiagnosticsEnabled(enabled) {
  return setModulePerformanceDiagnosticsEnabled(enabled);
}

export async function getModulePerformanceDiagnosticsEnabled() {
  return invoke("get_module_performance_diagnostics_enabled");
}

/**
 * @param {boolean} enabled
 */
export async function setModulePerformanceDiagnosticsEnabled(enabled) {
  return invoke("set_module_performance_diagnostics_enabled", { enabled });
}
