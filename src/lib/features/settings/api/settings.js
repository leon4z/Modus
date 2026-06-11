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

export async function getTranslationProviderConfig() {
  return invoke("get_translation_provider_config");
}

/**
 * @param {{ enabled?: boolean, provider?: string, baseUrl?: string, model?: string }} config
 */
export async function setTranslationProviderConfig(config) {
  return invoke("set_translation_provider_config", { config });
}

/**
 * @param {string} apiKey
 */
export async function setTranslationApiKey(apiKey) {
  return invoke("set_translation_api_key", { apiKey });
}

export async function clearTranslationApiKey() {
  return invoke("clear_translation_api_key");
}

export async function testTranslationProvider() {
  return invoke("test_translation_provider");
}
