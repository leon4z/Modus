import { get } from "svelte/store";
import { describe, expect, it } from "vitest";

import { managedToolIds, managedTools, tools } from "$lib/features/tools/stores/tools.js";

describe("tools store", () => {
  it("matches managed tools by backend canonical IDs", () => {
    tools.set([
      { id: "claude-code", name: "Claude Code", detected: true },
      { id: "codex", name: "Codex", detected: true },
    ]);
    managedToolIds.set(["claude-code"]);

    expect(get(managedTools).map((tool) => tool.id)).toEqual(["claude-code"]);
  });

  it("orders managed tools alphabetically by display name", () => {
    tools.set([
      { id: "zed", name: "Zed", detected: true },
      { id: "claude-code", name: "Claude Code", detected: true },
      { id: "codex", name: "Codex", detected: true },
    ]);
    managedToolIds.set(["zed", "codex", "claude-code"]);

    expect(get(managedTools).map((tool) => tool.id)).toEqual(["claude-code", "codex", "zed"]);
  });
});
