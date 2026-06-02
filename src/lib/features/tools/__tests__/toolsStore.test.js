import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";

const toolsApiMocks = vi.hoisted(() => ({
  listTools: vi.fn(),
}));

vi.mock("$lib/features/tools/api/tools.js", () => toolsApiMocks);

import { activeToolId, loadTools, tools } from "$lib/features/tools/stores/tools.js";

describe("tools store", () => {
  beforeEach(() => {
    tools.set([]);
    activeToolId.set(null);
    vi.clearAllMocks();
  });

  it("preserves the selected detected tool when tools refresh", async () => {
    activeToolId.set("dev-tool3");
    toolsApiMocks.listTools.mockResolvedValue([
      { id: "dev-tool1", detected: true },
      { id: "dev-tool3", detected: true },
    ]);

    await loadTools();

    expect(get(activeToolId)).toBe("dev-tool3");
  });

  it("falls back to the first detected tool when the previous selection is gone", async () => {
    activeToolId.set("missing-tool");
    toolsApiMocks.listTools.mockResolvedValue([
      { id: "dev-tool3", name: "Zed", detected: true },
      { id: "dev-tool1", name: "Codex", detected: true },
    ]);

    await loadTools();

    expect(get(activeToolId)).toBe("dev-tool1");
  });

  it("stores tools in alphabetic display order", async () => {
    toolsApiMocks.listTools.mockResolvedValue([
      { id: "zed", name: "Zed", detected: true },
      { id: "codex", name: "Codex", detected: true },
      { id: "claude-code", name: "Claude Code", detected: true },
    ]);

    await loadTools();

    expect(get(tools).map((tool) => tool.id)).toEqual(["claude-code", "codex", "zed"]);
  });
});
