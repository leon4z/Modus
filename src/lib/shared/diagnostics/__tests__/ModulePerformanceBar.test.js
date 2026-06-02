import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import ModulePerformanceBar from "$lib/shared/diagnostics/ModulePerformanceBar.svelte";
import {
  modulePerformanceSummaries,
  setModulePerformanceDiagnosticsEnabledState,
} from "$lib/shared/diagnostics/modulePerformance.js";
import { locale } from "$lib/shared/i18n/index.js";

describe("ModulePerformanceBar", () => {
  beforeEach(() => {
    locale.set("zh");
    setModulePerformanceDiagnosticsEnabledState(true);
    modulePerformanceSummaries.set({
      settings: {
        id: "settings-run-1",
        module: "settings",
        view: "general",
        reason: "entry",
        status: "success",
        visibleMs: 0,
        interactiveMs: 0,
        backgroundCompleteMs: 0,
        totalMs: 8,
        requestCount: 7,
        requestCounts: {
          "tool-paths": 1,
          "injection-targets": 1,
          "capability-overrides": 1,
        },
        counters: {
          managedTools: 3,
          customTools: 1,
        },
        milestones: [
          { name: "settings-visible", atMs: 0 },
          { name: "settings-ready", atMs: 4 },
          { name: "settings-complete", atMs: 8 },
        ],
      },
    });
  });

  afterEach(() => {
    cleanup();
    setModulePerformanceDiagnosticsEnabledState(false);
  });

  it("keeps the default row compact and expands details on demand", async () => {
    render(ModulePerformanceBar, { props: { moduleId: "settings" } });

    expect(screen.getByText("设置 · 通用")).toBeInTheDocument();
    expect(screen.getByText("请求: 7")).toBeInTheDocument();
    expect(screen.queryByText("tool-paths: 1")).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "展开性能诊断详情" }));

    expect(screen.getByText("tool-paths: 1")).toBeInTheDocument();
    expect(screen.getByText("settings-visible · 0ms")).toBeInTheDocument();
    expect(screen.getByText("managedTools: 3")).toBeInTheDocument();
  });

  it("formats tool ids as readable scope labels", () => {
    modulePerformanceSummaries.set({
      mcp: {
        id: "mcp-run-1",
        module: "mcp",
        view: "claude-code",
        reason: "entry",
        status: "success",
        visibleMs: 0,
        interactiveMs: 1,
        backgroundCompleteMs: 1,
        totalMs: 1,
        requestCount: 1,
        requestCounts: {},
        counters: {},
        milestones: [],
      },
    });

    render(ModulePerformanceBar, { props: { moduleId: "mcp" } });

    expect(screen.getByText("MCP · Claude Code")).toBeInTheDocument();
  });
});
