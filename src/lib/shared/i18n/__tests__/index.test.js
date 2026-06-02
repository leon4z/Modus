import { describe, expect, it, vi } from "vitest";

import {
  normalizeLanguagePreference,
  resolveEffectiveLanguage,
  resolveSystemLanguage,
} from "$lib/shared/i18n/index.js";

describe("i18n language preference", () => {
  it("normalizes saved language preferences", () => {
    expect(normalizeLanguagePreference("system")).toBe("system");
    expect(normalizeLanguagePreference("zh")).toBe("zh");
    expect(normalizeLanguagePreference("en")).toBe("en");
    expect(normalizeLanguagePreference("fr")).toBe("system");
  });

  it("resolves system language to supported UI languages", () => {
    expect(resolveSystemLanguage("en-US")).toBe("en");
    expect(resolveSystemLanguage("zh-Hans-CN")).toBe("zh");
    expect(resolveSystemLanguage("fr-FR")).toBe("zh");
  });

  it("keeps explicit preferences independent from the system language", () => {
    vi.stubGlobal("navigator", { language: "en-US" });
    expect(resolveEffectiveLanguage("zh")).toBe("zh");
    expect(resolveEffectiveLanguage("en")).toBe("en");
    expect(resolveEffectiveLanguage("system")).toBe("en");
    vi.unstubAllGlobals();
  });
});
