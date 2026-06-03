// @ts-nocheck
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { tick } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const skillApiMocks = vi.hoisted(() => ({
  listGenericSkills: vi.fn(),
  listSkills: vi.fn(),
  previewDeleteFromTool: vi.fn(),
  executeDeleteFromTool: vi.fn(),
}));

const inventoryMocks = vi.hoisted(() => ({
  DEFAULT_SKILL_INVENTORY_TTL_MS: 60_000,
  getCachedSkillInventory: vi.fn(),
  getSkillInventory: vi.fn(),
  invalidateSkillInventory: vi.fn(),
}));

const toolStoreMocks = vi.hoisted(() => ({
  loadTools: vi.fn(),
}));

const settingsApiMocks = vi.hoisted(() => ({
  getSkillPerformanceDiagnosticsEnabled: vi.fn(),
}));

const loggingApiMocks = vi.hoisted(() => ({
  writeSkillPerformanceLog: vi.fn(),
  writeModulePerformanceLog: vi.fn(),
}));

vi.mock("$lib/features/skills/api/skills.js", () => skillApiMocks);
vi.mock("$lib/features/skills/queries/skillInventoryQuery.js", () => inventoryMocks);
vi.mock("$lib/features/settings/index.js", () => settingsApiMocks);
vi.mock("$lib/shared/logging/api.js", () => loggingApiMocks);
vi.mock("$lib/features/skills/components/SkillViewer.svelte", async () => {
  const mod = await import("../../../../test/stubs/SkillViewerStub.svelte");
  return { default: mod.default };
});
vi.mock("$lib/shared/components/BaseCard.svelte", async () => {
  const mod = await import("../../../../test/stubs/BaseCardStub.svelte");
  return { default: mod.default };
});
vi.mock("$lib/shared/components/ConfirmDialog.svelte", async () => {
  const mod = await import("../../../../test/stubs/ConfirmDialogStub.svelte");
  return { default: mod.default };
});
vi.mock("$lib/features/tools/index.js", async () => {
  const { derived, writable } = await import("svelte/store");
  const tools = writable([{ id: "codex", name: "Codex", detected: true }]);
  const activeToolId = writable("codex");
  const activeTool = derived([tools, activeToolId], ([$tools, $activeToolId]) =>
    $tools.find((tool) => tool.id === $activeToolId) || null
  );
  const managedTools = derived(tools, ($tools) => $tools.filter((tool) => tool.detected));
  const pendingSubTab = writable(null);
  return {
    loadTools: toolStoreMocks.loadTools,
    activeTool,
    activeToolId,
    managedTools,
    pendingSubTab,
  };
});
import SkillsModule from "$lib/features/skills/components/SkillsModule.svelte";
import { setModulePerformanceDiagnosticsEnabledState } from "$lib/shared/diagnostics/modulePerformance.js";
import { locale } from "$lib/shared/i18n/index.js";

function createDeferred() {
  let resolve = () => {};
  let reject = () => {};
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("SkillsModule local-source behavior", () => {
  beforeEach(() => {
    locale.set("en");
    toolStoreMocks.loadTools.mockResolvedValue([]);
    settingsApiMocks.getSkillPerformanceDiagnosticsEnabled.mockResolvedValue(false);
    loggingApiMocks.writeSkillPerformanceLog.mockResolvedValue(undefined);
    loggingApiMocks.writeModulePerformanceLog.mockResolvedValue(undefined);
    setModulePerformanceDiagnosticsEnabledState(false);
    skillApiMocks.listGenericSkills.mockResolvedValue([]);
    skillApiMocks.listSkills.mockResolvedValue([]);
    inventoryMocks.getCachedSkillInventory.mockReturnValue(null);
    inventoryMocks.invalidateSkillInventory.mockImplementation(() => {});
    inventoryMocks.getSkillInventory.mockResolvedValue({
      skills: [
        {
          name: "demo-skill",
          display_name: "Demo Skill",
          description: "Demo description",
          path: "/skills/demo-skill",
          tool_statuses: [
            { tool_id: "codex", status: "installed", path: "/codex/demo-skill" },
          ],
        },
      ],
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it("shows retained single-skill flow without update or batch entry points", async () => {
    render(SkillsModule);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Demo Skill/ })).toBeInTheDocument();
    });

    expect(screen.queryByRole("button", { name: "Batch" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Check updates" })).not.toBeInTheDocument();
    expect(document.querySelector(".batch-action-bar")).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: /Demo Skill/ }));

    expect(screen.getByTestId("skill-viewer-stub")).toHaveTextContent("Viewing Demo Skill");
  });

  it("keeps performance diagnostics hidden during ordinary Skill usage", async () => {
    const { container } = render(SkillsModule);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Demo Skill/ })).toBeInTheDocument();
    });

    expect(screen.queryByLabelText("Skill performance diagnostics")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Module performance diagnostics")).not.toBeInTheDocument();
    expect(loggingApiMocks.writeModulePerformanceLog).not.toHaveBeenCalled();
  });

  it("localizes the page title in Chinese", async () => {
    locale.set("zh");

    render(SkillsModule);

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: "技能" })).toBeInTheDocument();
    });
    expect(screen.queryByRole("heading", { name: "Skills" })).not.toBeInTheDocument();
  });

  it("shows opt-in performance diagnostics with request metrics", async () => {
    setModulePerformanceDiagnosticsEnabledState(true);

    render(SkillsModule);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Demo Skill/ })).toBeInTheDocument();
    });

    const panel = await screen.findByLabelText("Module performance diagnostics");
    expect(panel).toHaveTextContent("Performance diagnostics");
    expect(panel).toHaveTextContent("Skills");
    expect(panel).toHaveTextContent("Requests:");
    expect(panel).not.toHaveTextContent("inventory");
    await waitFor(() => {
      expect(panel).toHaveTextContent("Requests: 2");
    });
    await fireEvent.click(screen.getByRole("button", { name: "Expand performance diagnostics" }));
    await waitFor(() => {
      expect(panel).toHaveTextContent("inventory");
      expect(panel).toHaveTextContent("tool-list");
    });
    await waitFor(() => {
      expect(loggingApiMocks.writeModulePerformanceLog).toHaveBeenCalledWith(expect.objectContaining({
        module: "skills",
        reason: "entry",
        status: "success",
        requestCount: 2,
        requestCounts: expect.objectContaining({
          inventory: 1,
          "tool-list": 1,
        }),
      }));
    });
    await fireEvent.click(screen.getByRole("button", { name: "Tools" }));
    await waitFor(() => {
      expect(panel).toHaveTextContent("Skills · Tools");
      expect(panel).not.toHaveTextContent("tool-list");
    });
    expect(skillApiMocks.listGenericSkills).not.toHaveBeenCalled();
  });

  it("derives the Shared tab from inventory without a separate shared scan", async () => {
    inventoryMocks.getSkillInventory.mockResolvedValueOnce({
      skills: [
        {
          name: "shared-skill",
          display_name: "Shared Skill",
          description: "Shared description",
          path: "/agents/skills/shared-skill",
          tool_statuses: [
            { tool_id: "codex", status: "variantInstalledCopy", path: "/agents/skills/shared-skill", path_origin: "generic" },
          ],
        },
        {
          name: "tool-only",
          display_name: "Tool Only",
          description: "Tool-only description",
          path: "/codex/tool-only",
          tool_statuses: [
            { tool_id: "codex", status: "installed", path: "/codex/tool-only", path_origin: "tool" },
          ],
        },
      ],
    });

    render(SkillsModule);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Shared Skill/ })).toBeInTheDocument();
    });
    await fireEvent.click(screen.getByRole("button", { name: "Shared" }));

    expect(screen.getByRole("button", { name: /Shared Skill/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Tool Only/ })).not.toBeInTheDocument();
    expect(skillApiMocks.listGenericSkills).not.toHaveBeenCalled();
  });

  it("filters visible Skill cards from the page-header search", async () => {
    inventoryMocks.getSkillInventory.mockResolvedValueOnce({
      skills: [
        {
          name: "alpha-skill",
          display_name: "Alpha Skill",
          description: "Matches the card list",
          path: "/skills/alpha-skill",
          tool_statuses: [
            { tool_id: "codex", status: "installed", path: "/codex/alpha-skill" },
          ],
        },
        {
          name: "beta-skill",
          display_name: "Beta Skill",
          description: "Different card",
          path: "/skills/beta-skill",
          tool_statuses: [
            { tool_id: "codex", status: "installed", path: "/codex/beta-skill" },
          ],
        },
      ],
    });

    const { container } = render(SkillsModule);

    expect(await screen.findByRole("button", { name: /Alpha Skill/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Beta Skill/ })).toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: "Search" }));
    await fireEvent.input(screen.getByPlaceholderText("Search visible Skills"), { target: { value: "alpha" } });
    expect(screen.queryByRole("region", { name: "Module search results" })).not.toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Alpha Skill/ })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /Beta Skill/ })).not.toBeInTheDocument();
    });

    await fireEvent.input(screen.getByPlaceholderText("Search visible Skills"), { target: { value: "" } });
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Beta Skill/ })).toBeInTheDocument();
    });
  });

  it("keeps the current list visible when a manual refresh fails", async () => {
    const rendered = render(SkillsModule);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Demo Skill/ })).toBeInTheDocument();
    });

    inventoryMocks.getCachedSkillInventory.mockReturnValue(null);
    inventoryMocks.getSkillInventory.mockRejectedValueOnce(new Error("refresh failed"));

    await rendered.component.reload();

    expect(screen.getByRole("button", { name: /Demo Skill/ })).toBeInTheDocument();
    expect(screen.getByText(/Failed to load Skills/)).toBeInTheDocument();
  });

  it("applies a detail Skill change locally before background inventory reconciliation", async () => {
    vi.useFakeTimers();
    render(SkillsModule);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Demo Skill/ })).toBeInTheDocument();
    });
    expect(inventoryMocks.getSkillInventory).toHaveBeenCalledTimes(1);

    await fireEvent.click(screen.getByRole("button", { name: /Demo Skill/ }));
    await fireEvent.click(screen.getByTestId("skill-viewer-stub-change"));
    await tick();

    expect(inventoryMocks.getSkillInventory).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(749);
    expect(inventoryMocks.getSkillInventory).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1);
    expect(inventoryMocks.getSkillInventory).toHaveBeenCalledTimes(2);
    expect(inventoryMocks.getSkillInventory).toHaveBeenLastCalledWith({ force: true });
  });

  it("renders an expired inventory snapshot immediately while refreshing inventory in the background", async () => {
    vi.useFakeTimers();
    setModulePerformanceDiagnosticsEnabledState(true);
    const staleInventory = {
      skills: [
        {
          name: "stale-skill",
          display_name: "Stale Skill",
          description: "From expired snapshot",
          path: "/skills/stale-skill",
          tool_statuses: [
            { tool_id: "codex", status: "installed", path: "/codex/stale-skill" },
          ],
        },
      ],
    };
    const freshInventory = {
      skills: [
        {
          name: "fresh-skill",
          display_name: "Fresh Skill",
          description: "From background refresh",
          path: "/skills/fresh-skill",
          tool_statuses: [
            { tool_id: "codex", status: "installed", path: "/codex/fresh-skill" },
          ],
        },
      ],
    };
    const refresh = createDeferred();
    inventoryMocks.getCachedSkillInventory.mockImplementation((options = {}) => (
      options.ttl === inventoryMocks.DEFAULT_SKILL_INVENTORY_TTL_MS ? null : staleInventory
    ));
    inventoryMocks.getSkillInventory.mockReturnValueOnce(refresh.promise);

    render(SkillsModule);

    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    await tick();

    expect(screen.getByRole("button", { name: /Stale Skill/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Fresh Skill/ })).not.toBeInTheDocument();
    expect(inventoryMocks.getSkillInventory).not.toHaveBeenCalled();
    expect(skillApiMocks.listSkills).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(300);
    expect(inventoryMocks.getSkillInventory).toHaveBeenCalledTimes(1);

    refresh.resolve(freshInventory);

    await Promise.resolve();
    await Promise.resolve();
    await tick();

    expect(screen.getByRole("button", { name: /Fresh Skill/ })).toBeInTheDocument();
    await vi.waitFor(() => {
      expect(loggingApiMocks.writeModulePerformanceLog).toHaveBeenCalledWith(expect.objectContaining({
        module: "skills",
        reason: "entry",
        status: "success",
        requestCounts: expect.objectContaining({
          inventory: 1,
          "tool-list": 1,
        }),
        milestones: expect.arrayContaining([
          expect.objectContaining({ name: "inventory-stale-cache-ready" }),
          expect.objectContaining({ name: "visible-lists-ready" }),
          expect.objectContaining({ name: "background-refresh-scheduled" }),
          expect.objectContaining({ name: "inventory-ready" }),
        ]),
      }));
    });
  });

  it("does not let an expired-snapshot background refresh overwrite a later manual refresh", async () => {
    vi.useFakeTimers();
    const staleInventory = {
      skills: [
        {
          name: "stale-skill",
          display_name: "Stale Skill",
          description: "From expired snapshot",
          path: "/skills/stale-skill",
          tool_statuses: [
            { tool_id: "codex", status: "installed", path: "/codex/stale-skill" },
          ],
        },
      ],
    };
    const backgroundInventory = {
      skills: [
        {
          name: "background-skill",
          display_name: "Background Skill",
          description: "Older background result",
          path: "/skills/background-skill",
          tool_statuses: [
            { tool_id: "codex", status: "installed", path: "/codex/background-skill" },
          ],
        },
      ],
    };
    const manualInventory = {
      skills: [
        {
          name: "manual-skill",
          display_name: "Manual Skill",
          description: "Newer manual result",
          path: "/skills/manual-skill",
          tool_statuses: [
            { tool_id: "codex", status: "installed", path: "/codex/manual-skill" },
          ],
        },
      ],
    };
    const backgroundRefresh = createDeferred();
    const manualRefresh = createDeferred();
    inventoryMocks.getCachedSkillInventory.mockImplementation((options = {}) => (
      options.ttl === inventoryMocks.DEFAULT_SKILL_INVENTORY_TTL_MS ? null : staleInventory
    ));
    inventoryMocks.getSkillInventory
      .mockReturnValueOnce(backgroundRefresh.promise)
      .mockReturnValueOnce(manualRefresh.promise);

    const rendered = render(SkillsModule);

    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    await tick();

    expect(screen.getByRole("button", { name: /Stale Skill/ })).toBeInTheDocument();

    await vi.advanceTimersByTimeAsync(300);
    expect(inventoryMocks.getSkillInventory).toHaveBeenCalledTimes(1);

    const manualReload = rendered.component.reload();
    await Promise.resolve();
    expect(inventoryMocks.getSkillInventory).toHaveBeenCalledTimes(2);

    manualRefresh.resolve(manualInventory);
    await manualReload;
    await tick();

    expect(screen.getByRole("button", { name: /Manual Skill/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Background Skill/ })).not.toBeInTheDocument();

    backgroundRefresh.resolve(backgroundInventory);
    await Promise.resolve();
    await Promise.resolve();
    await tick();

    expect(screen.getByRole("button", { name: /Manual Skill/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Background Skill/ })).not.toBeInTheDocument();
  });

  it("does not show a new-source reminder for local shared or tool-directory sources", async () => {
    inventoryMocks.getSkillInventory.mockResolvedValueOnce({
      skills: [
        {
          name: "local-tool-skill",
          display_name: "Local Tool Skill",
          description: "Plain local source",
          path: "/tools/codex/skills/local-tool-skill",
          tool_statuses: [
            {
              tool_id: "codex",
              status: "localOnly",
              path: "/tools/codex/skills/local-tool-skill",
              path_origin: "tool",
            },
          ],
        },
      ],
    });

    render(SkillsModule);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Local Tool Skill/ })).toBeInTheDocument();
    });

    expect(screen.queryByRole("region", { name: /new skill source reminder/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("dialog", { name: /new skill sources/i })).not.toBeInTheDocument();
  });

  it("surfaces metadata-backed Skill drift as a warning", async () => {
    inventoryMocks.getSkillInventory.mockResolvedValueOnce({
      skills: [
        {
          name: "drifted-skill",
          display_name: "Drifted Skill",
          description: "Metadata mismatch",
          path: "/tools/codex/skills/drifted-skill",
          tool_statuses: [
            {
              tool_id: "codex",
              tool_name: "Codex",
              status: "variant_drifted",
              path: "/tools/codex/skills/drifted-skill",
              path_origin: "tool",
            },
          ],
        },
      ],
    });
    skillApiMocks.listSkills.mockResolvedValue([
      {
        name: "drifted-skill",
        display_name: "Drifted Skill",
        description: "Metadata mismatch",
        path: "/tools/codex/skills/drifted-skill",
        tool_id: "codex",
      },
    ]);

    render(SkillsModule);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Drifted Skill/ })).toBeInTheDocument();
    });
    await fireEvent.click(screen.getByRole("button", { name: "Tools" }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Drifted Skill/ })).toBeInTheDocument();
    });
    expect(screen.getByRole("button", {
      name: "This tool's skill content and metadata are inconsistent. Click for details.",
    })).toBeInTheDocument();
  });

  it("collapses duplicate tool entries and surfaces same-name source warnings", async () => {
    inventoryMocks.getSkillInventory.mockResolvedValueOnce({
      skills: [
        {
          name: "executing-plans",
          display_name: "Executing Plans",
          description: "Duplicate source demo",
          path: "/tools/codex/skills/executing-plans",
          tool_statuses: [
            {
              tool_id: "codex",
              tool_name: "Codex",
              status: "installed",
              path: "/tools/codex/skills/executing-plans",
              path_origin: "tool",
              abnormal_state: "duplicate_sources",
              sources: [
                {
                  tool_id: "codex",
                  tool_name: "Codex",
                  status: "installed",
                  path: "/tools/codex/skills/executing-plans",
                  path_origin: "tool",
                },
                {
                  tool_id: "codex",
                  tool_name: "Codex",
                  status: "installed",
                  path: "/tools/codex/skills/superpowers/skills/executing-plans",
                  path_origin: "tool",
                },
              ],
            },
          ],
        },
      ],
    });
    skillApiMocks.listSkills.mockResolvedValue([
      {
        name: "executing-plans",
        display_name: "Executing Plans",
        description: "first",
        path: "/tools/codex/skills/executing-plans",
        tool_id: "codex",
      },
      {
        name: "executing-plans",
        display_name: "Executing Plans",
        description: "second",
        path: "/tools/codex/skills/superpowers/skills/executing-plans",
        tool_id: "codex",
      },
    ]);

    render(SkillsModule);

    await fireEvent.click(screen.getByRole("button", { name: "Tools" }));

    await waitFor(() => {
      expect(screen.getAllByRole("button", { name: /Executing Plans/ })).toHaveLength(1);
    });
    await waitFor(() => {
      expect(screen.getByRole("button", {
        name: "Multiple same-name sources exist under this tool. Click for details.",
      })).toBeInTheDocument();
    });
  });
});
