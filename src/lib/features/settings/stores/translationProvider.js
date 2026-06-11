// Purpose: Share translation provider availability between Settings and Markdown viewers.
import { writable } from "svelte/store";
import { getTranslationProviderConfig } from "$lib/features/settings/api/settings.js";

export function defaultTranslationProviderState() {
  return {
    enabled: false,
    provider: "openai-compatible",
    baseUrl: "https://api.openai.com/v1",
    model: "",
    apiKeyConfigured: false,
  };
}

/** @param {any} value */
export function normalizeTranslationProviderState(value) {
  const provider = String(value?.provider || "openai-compatible").trim().toLowerCase();
  return {
    ...defaultTranslationProviderState(),
    ...(value || {}),
    enabled: value?.enabled === true,
    provider: provider === "cc-router" || provider === "anthropic" || provider === "anthropic-compatible"
      ? "anthropic-messages"
      : provider || "openai-compatible",
    baseUrl: String(value?.baseUrl || "https://api.openai.com/v1"),
    model: String(value?.model || ""),
    apiKeyConfigured: value?.apiKeyConfigured === true,
  };
}

export const translationProviderState = writable(defaultTranslationProviderState());

/** @param {any} value */
export function setTranslationProviderState(value) {
  const next = normalizeTranslationProviderState(value);
  translationProviderState.set(next);
  return next;
}

export async function refreshTranslationProviderState() {
  const next = await getTranslationProviderConfig();
  return setTranslationProviderState(next);
}
