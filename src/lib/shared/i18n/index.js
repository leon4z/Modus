import { writable, derived } from 'svelte/store';
import zh from './zh.json';
import en from './en.json';

const dict = { zh, en };
export const languagePreference = writable('system');
export const locale = writable('zh'); // fallback defaults to zh

/** @param {string | null | undefined} value */
export function normalizeLanguagePreference(value) {
  return value === 'system' || value === 'zh' || value === 'en' ? value : 'system';
}

/** @param {string | null | undefined} language */
export function resolveSystemLanguage(language = globalThis.navigator?.language) {
  return String(language || '').toLowerCase().startsWith('en') ? 'en' : 'zh';
}

/** @param {string | null | undefined} preference */
export function resolveEffectiveLanguage(preference) {
  const normalized = normalizeLanguagePreference(preference);
  return normalized === 'system' ? resolveSystemLanguage() : normalized;
}

/** @param {string | null | undefined} preference */
export function applyLanguagePreference(preference) {
  const normalized = normalizeLanguagePreference(preference);
  languagePreference.set(normalized);
  locale.set(resolveEffectiveLanguage(normalized));
}

export const t = derived(locale, ($l) => (/** @type {string} */ key, /** @type {Record<string,any>} */ vars = {}) => {
  const dictLang = /** @type {any} */ (/** @type {any} */ (dict)[/** @type {any} */ ($l)]);
  let str = /** @type {any} */ (dictLang)?.[/** @type {any} */ (key)] || /** @type {any} */ (dict['zh'])?.[/** @type {any} */ (key)] || key;
  if (Object.keys(vars).length > 0) {
    str = str.replace(/\{(\w+)\}/g, (/** @type {any} */ _, /** @type {string} */ k) => vars[/** @type {any} */ (k)] ?? `{${k}}`);
  }
  return str;
});
