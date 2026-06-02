import { describe, expect, it } from "vitest";

import {
  getVisualInvokeResponse,
  getVisualVerificationUrl,
  VISUAL_VERIFICATION_VIEWS,
} from "$lib/dev/visualVerification/fixtures.js";

describe("visual verification fixtures", () => {
  it("provides deterministic data for every top-level visual route", () => {
    for (const view of VISUAL_VERIFICATION_VIEWS) {
      expect(getVisualVerificationUrl(view)).toBe(`/?visual=1&view=${view}`);
    }
  });

  it("exposes tool capabilities and mixed configuration states", async () => {
    const tools = await getVisualInvokeResponse("list_tools");
    expect(/** @type {any[]} */ (tools).map((tool) => tool.id)).toEqual(["codex", "claude-code", "dev_tool"]);
    expect(/** @type {any[]} */ (tools)[0].capabilities.some((/** @type {any} */ capability) => capability.kind === "ordinary_config")).toBe(true);

    const files = await getVisualInvokeResponse("list_config_files", { toolId: "codex" });
    expect(/** @type {any[]} */ (files).map((file) => file.access)).toEqual(["writable", "read_only", "unknown", "unsupported"]);
  });

  it("provides many managed tools for protected selector visual states", async () => {
    window.history.replaceState({}, "", "/?visual=1&view=config&state=tool-selector-many");

    const tools = await getVisualInvokeResponse("list_tools");
    const managed = await getVisualInvokeResponse("get_managed_tools");
    expect(/** @type {any[]} */ (tools).map((tool) => tool.id)).toEqual([
      "codex",
      "claude-code",
      "dev_tool",
      "openclaw",
      "cursor",
      "trae",
    ]);
    expect(managed).toEqual([
      "codex",
      "claude-code",
      "dev_tool",
      "openclaw",
      "cursor",
      "trae",
    ]);

    window.history.replaceState({}, "", "/");
  });

  it("provides an isolated first-run onboarding visual state", async () => {
    window.history.replaceState({}, "", "/?visual=1&state=first-run-onboarding");

    const initialized = await getVisualInvokeResponse("is_initialized");
    const tools = await getVisualInvokeResponse("list_tools");
    const managed = await getVisualInvokeResponse("get_managed_tools");
    const theme = await getVisualInvokeResponse("get_theme");

    expect(initialized).toBe(false);
    expect(/** @type {any[]} */ (tools).map((tool) => tool.id)).toEqual([
      "codex",
      "claude-code",
      "dev_tool",
      "openclaw",
      "cursor",
      "trae",
    ]);
    expect(managed).toEqual([]);
    expect(theme).toBe("dark");

    window.history.replaceState({}, "", "/");
  });

  it("provides current Community file-workspace visual states without custom rules", async () => {
    window.history.replaceState({}, "", "/?visual=1&view=rules&state=rules-tool-directory");

    const tools = await getVisualInvokeResponse("list_tools");
    const codex = /** @type {any[]} */ (tools).find((tool) => tool.id === "codex");
    expect(codex.rule_sources.map((/** @type {any} */ source) => source.group)).toEqual([
      "workspace",
      "workspace/team",
      "local",
    ]);

    const rules = await getVisualInvokeResponse("list_default_rules");
    expect(/** @type {any[]} */ (rules).map((rule) => rule.id)).toEqual(["common_rule"]);
  });

  it("fails loudly when a visual command is not registered", async () => {
    await expect(getVisualInvokeResponse("unknown_visual_command")).rejects.toThrow(
      "No visual verification fixture registered"
    );
  });

  it("provides shared-source copy preview labels", async () => {
    window.history.replaceState({}, "", "/?visual=1&view=skills&state=skill-detail-shared-link");

    const preview = await getVisualInvokeResponse("copy_skill_to_tool", {
      skillName: "shared-design",
      toolId: "claude-code",
      sourcePath: "/Users/visual/.agents/skills/shared-design",
      dryRun: true,
    });

    expect(preview.changes).toEqual(expect.arrayContaining([
      expect.objectContaining({
        subject: "claude-code",
        action: "copy",
        changeKind: "create",
        entryKind: "file",
      }),
    ]));
  });

  it("provides metadata-backed drift for the Skills visual state", async () => {
    window.history.replaceState({}, "", "/?visual=1&view=skills&state=skill-metadata-drift");

    const inventory = await getVisualInvokeResponse("scan_skill_inventory");
    const drifted = inventory.skills.find((/** @type {any} */ skill) => skill.name === "metadata-drift");

    expect(drifted?.tool_statuses?.[0]?.status).toBe("variant_drifted");

    window.history.replaceState({}, "", "/");
  });
});
