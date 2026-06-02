import { describe, expect, it } from "vitest";

import {
  protectedToolSelectorMinWidth,
  resolveToolSelectorMode,
} from "$lib/shared/utils/toolSelectorCapsule.js";

describe("tool selector capsule geometry", () => {
  it("calculates the protected icon-only width from hit areas and capsule padding", () => {
    expect(protectedToolSelectorMinWidth(0)).toBe(0);
    expect(protectedToolSelectorMinWidth(1)).toBe(40);
    expect(protectedToolSelectorMinWidth(3)).toBe(116);
    expect(protectedToolSelectorMinWidth(7)).toBe(268);
  });

  it("uses whole-group full-label and icon-only transitions with hysteresis", () => {
    expect(resolveToolSelectorMode({
      availableWidth: 320,
      fullWidth: 300,
      currentMode: "full",
    })).toBe("full");

    expect(resolveToolSelectorMode({
      availableWidth: 299,
      fullWidth: 300,
      currentMode: "full",
    })).toBe("icon");

    expect(resolveToolSelectorMode({
      availableWidth: 312,
      fullWidth: 300,
      currentMode: "icon",
    })).toBe("icon");

    expect(resolveToolSelectorMode({
      availableWidth: 316,
      fullWidth: 300,
      currentMode: "icon",
    })).toBe("full");
  });
});
