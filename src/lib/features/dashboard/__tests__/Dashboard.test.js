import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { get } from "svelte/store";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const apiMocks = vi.hoisted(() => ({
  getDashboard: vi.fn(),
}));
const toolApiMocks = vi.hoisted(() => ({
  listTools: vi.fn(),
}));

vi.mock("$lib/features/dashboard/api/dashboard.js", () => apiMocks);
vi.mock("$lib/features/tools/api/tools.js", () => toolApiMocks);
vi.mock("$lib/shared/components/ToolIcon.svelte", async () => {
  const mod = await import("../../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});

import Dashboard from "$lib/features/dashboard/components/Dashboard.svelte";
import { activeToolId, activeView, managedToolIds, tools } from "$lib/features/tools/index.js";
import { activeSettingsTab, focusedSettingsToolId } from "$lib/features/settings/index.js";
import {
  modulePerformanceSummaries,
  setModulePerformanceDiagnosticsEnabledState,
} from "$lib/shared/diagnostics/modulePerformance.js";
import { locale } from "$lib/shared/i18n/index.js";

/** @param {string[]} toolIds */
function dashboardData(toolIds) {
  return {
    tools: toolIds.map((toolId) => ({
      tool_id: toolId,
      tool_name: toolId === "openclaw" ? "OpenClaw" : toolId === "claude-code" ? "Claude Code" : "Codex",
      detected: true,
      primary_config_health: "ok",
      rule_count: 1,
      skill_count: 2,
      mcp_count: 3,
      config_count: 1,
    })),
    detected_count: toolIds.length,
    total_rules: toolIds.length,
    total_skills: 0,
    total_configs: toolIds.length,
    total_mcp: 0,
  };
}

describe("Dashboard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    locale.set("en");
    managedToolIds.set([]);
    activeToolId.set(null);
    activeView.set("dashboard");
    activeSettingsTab.set("general");
    focusedSettingsToolId.set(null);
    setModulePerformanceDiagnosticsEnabledState(false);
    tools.set([]);
    toolApiMocks.listTools.mockResolvedValue([
      { id: "codex", name: "Codex", detected: true, primary_config_health: "unreadable" },
    ]);
    apiMocks.getDashboard
      .mockResolvedValueOnce(dashboardData(["codex"]))
      .mockResolvedValueOnce(dashboardData(["codex", "openclaw"]));
  });

  afterEach(() => {
    cleanup();
    managedToolIds.set([]);
    activeToolId.set(null);
    activeSettingsTab.set("general");
    focusedSettingsToolId.set(null);
    tools.set([]);
  });

  it("refreshes dashboard data when managed tools change", async () => {
    render(Dashboard);

    await screen.findByText("Codex");
    expect(apiMocks.getDashboard).toHaveBeenCalledTimes(1);
    expect(screen.queryByLabelText("Module performance diagnostics")).not.toBeInTheDocument();
    expect(get(modulePerformanceSummaries)).toEqual({});

    managedToolIds.set(["codex", "openclaw"]);

    await waitFor(() => expect(apiMocks.getDashboard).toHaveBeenCalledTimes(2));
    await screen.findByText("OpenClaw");
  });

  it("shows managed environment count without aggregate totals", async () => {
    apiMocks.getDashboard.mockReset();
    apiMocks.getDashboard.mockResolvedValue({
      ...dashboardData(["codex"]),
      total_rules: 93,
      total_skills: 28,
    });

    render(Dashboard);

    expect(await screen.findByText("Managing 1 environments")).toBeInTheDocument();
    expect(screen.queryByText(/93/)).not.toBeInTheDocument();
    expect(screen.queryByText(/28/)).not.toBeInTheDocument();
  });

  it("renders dashboard cards alphabetically by tool name", async () => {
    apiMocks.getDashboard.mockReset();
    apiMocks.getDashboard.mockResolvedValue(dashboardData(["openclaw", "codex", "claude-code"]));

    render(Dashboard);

    await screen.findByText("Claude Code");
    const names = screen.getAllByText(/Claude Code|Codex|OpenClaw/).map((entry) => entry.textContent);
    expect(names).toEqual(["Claude Code", "Codex", "OpenClaw"]);
  });

  it("keeps cards overview-only when capability remediation exists", async () => {
    apiMocks.getDashboard.mockReset();
    apiMocks.getDashboard.mockResolvedValue(dashboardData(["codex"]));
    tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        certified_global_rule_target: "/Users/visual/.codex/AGENTS.md",
        certified_shared_skill_direct_read: true,
        capabilities: [
          {
            id: "user-configured-global-rule-target",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/Users/visual/.codex/CUSTOM.md",
            source_confidence: "user_configured",
            action_evidence: [
              { action: "inject", supported: true, evidence: "user configured" },
            ],
          },
          {
            id: "shared-skills",
            kind: "skill",
            scope: "shared",
            access: "unsupported",
            format: "skill_directory",
            source_path: "/Users/visual/.agents/skills",
            source_confidence: "user_configured",
          },
        ],
      },
    ]);

    render(Dashboard);

    expect(await screen.findByText("Codex")).toBeInTheDocument();
    expect(screen.queryByText("Online")).not.toBeInTheDocument();
    expect(screen.queryByText("Global rule target configured")).not.toBeInTheDocument();
    expect(screen.queryByText("Shared Skills: Symlink")).not.toBeInTheDocument();
    expect(screen.queryByText("Custom")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Settings" })).not.toBeInTheDocument();

    expect(screen.getByRole("button", { name: /1\s+Rules/i })).toBeEnabled();
    expect(screen.getByRole("button", { name: /2\s+Skills/i })).toBeEnabled();
    expect(screen.getByRole("button", { name: /3\s+MCP/i })).toBeEnabled();
    expect(screen.getByRole("button", { name: /1\s+Config/i })).toBeEnabled();

    await fireEvent.click(screen.getByRole("button", { name: /2\s+Skills/i }));

    expect(get(activeView)).toBe("skills");
    expect(get(activeToolId)).toBe("codex");
  });

  it("shows configuration attention only for primary configuration health issues", async () => {
    apiMocks.getDashboard.mockReset();
    apiMocks.getDashboard.mockResolvedValue({
      ...dashboardData(["codex", "openclaw"]),
      tools: [
        {
          ...dashboardData(["codex"]).tools[0],
          primary_config_health: "missing",
        },
        {
          ...dashboardData(["openclaw"]).tools[0],
          primary_config_health: "ok",
          rule_count: 0,
          skill_count: 0,
          mcp_count: 0,
          config_count: 0,
        },
      ],
    });

    render(Dashboard);

    expect(await screen.findByText("Codex")).toBeInTheDocument();
    const badge = screen.getByRole("button", { name: "Status issue · Config directory missing" });
    expect(badge).not.toHaveAttribute("title");
    expect(screen.getByText("Config directory missing")).toBeInTheDocument();
    expect(screen.queryByText("Online")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Status issue · Config directory unreadable" })).not.toBeInTheDocument();
  });

  it("routes configuration health badges to Settings Tools for the affected tool", async () => {
    apiMocks.getDashboard.mockReset();
    apiMocks.getDashboard.mockResolvedValue({
      ...dashboardData(["codex"]),
      tools: [
        {
          ...dashboardData(["codex"]).tools[0],
          primary_config_health: "unreadable",
        },
      ],
    });

    render(Dashboard);

    await fireEvent.click(await screen.findByRole("button", { name: "Status issue · Config directory unreadable" }));

    expect(get(activeView)).toBe("settings");
    expect(get(activeToolId)).toBe("codex");
    expect(get(activeSettingsTab)).toBe("tools");
    expect(get(focusedSettingsToolId)).toBe("codex");
    await waitFor(() => expect(toolApiMocks.listTools).toHaveBeenCalled());
  });
});
