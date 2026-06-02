import { get } from "svelte/store";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  beginModulePerformanceRun,
  createModulePerformanceRun,
  finishAndRecordModulePerformanceRun,
  finishModulePerformanceRun,
  isModulePerformanceDiagnosticsEnabled,
  markModulePerformance,
  modulePerformanceSummaries,
  MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE,
  MODULE_PERFORMANCE_ROLE_INTERACTIVE,
  MODULE_PERFORMANCE_ROLE_VISIBLE,
  safeDiagnosticLabel,
  setModulePerformanceDiagnosticsEnabledState,
  summarizeModulePerformanceRun,
  trackModulePerformanceRequest,
  updateModulePerformanceSummary,
} from "../modulePerformance.js";
import { writeModulePerformanceLog } from "$lib/shared/logging/api.js";

vi.mock("$lib/shared/logging/api.js", () => ({
  writeModulePerformanceLog: vi.fn(),
}));

afterEach(() => {
  vi.restoreAllMocks();
  vi.clearAllMocks();
  setModulePerformanceDiagnosticsEnabledState(false);
});

describe("modulePerformance diagnostics", () => {
  it("enables diagnostics only from the persisted Settings value", () => {
    expect(isModulePerformanceDiagnosticsEnabled(true)).toBe(true);
    expect(isModulePerformanceDiagnosticsEnabled(false)).toBe(false);
    expect(isModulePerformanceDiagnosticsEnabled(null)).toBe(false);
    expect(isModulePerformanceDiagnosticsEnabled(undefined)).toBe(false);
  });

  it("does not create, display, or write runs while disabled", () => {
    const run = beginModulePerformanceRun({ module: "skills", view: "overview", reason: "entry" });

    expect(run).toBeNull();
    expect(get(modulePerformanceSummaries)).toEqual({});
    expect(writeModulePerformanceLog).not.toHaveBeenCalled();
  });

  it("summarizes visible, interactive, background, total, requests, and counters", async () => {
    let now = 100;
    vi.spyOn(performance, "now").mockImplementation(() => now);
    const run = createModulePerformanceRun({
      module: "skills",
      view: "overview",
      reason: "entry",
      counters: { items: 3 },
    }, { startedAt: 90 });

    markModulePerformance(run, "visible-lists-ready", { at: 92, role: MODULE_PERFORMANCE_ROLE_VISIBLE });
    markModulePerformance(run, "interactive-ready", { at: 100, role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
    await trackModulePerformanceRequest(run, "inventory", async () => {
      now = 130;
      return { ok: true };
    }, { startedAt: 110 });
    markModulePerformance(run, "refresh-complete", { at: 140, role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });

    expect(finishModulePerformanceRun(run, "success")).toMatchObject({
      module: "skills",
      view: "overview",
      reason: "entry",
      status: "success",
      visibleMs: 2,
      interactiveMs: 10,
      backgroundCompleteMs: 50,
      requestCount: 1,
      requestCounts: { inventory: 1 },
      counters: { items: 3 },
      requests: [{ label: "inventory", status: "success", durationMs: 20, errorKind: "" }],
    });
  });

  it("records failed request outcome without preserving raw error text", async () => {
    let now = 200;
    vi.spyOn(performance, "now").mockImplementation(() => now);
    const run = createModulePerformanceRun({ module: "rules", reason: "entry" }, { startedAt: 190 });

    await expect(trackModulePerformanceRequest(run, "rule-list", async () => {
      now = 260;
      throw new TypeError("path /Users/example/secret failed");
    }, { startedAt: 220 })).rejects.toThrow(TypeError);

    expect(finishModulePerformanceRun(run, "failed")).toMatchObject({
      status: "failed",
      requestCount: 1,
      requests: [{ label: "rule-list", status: "failed", durationMs: 40, errorKind: "TypeError" }],
    });
  });

  it("prevents older overlapping runs from replacing the latest displayed summary", () => {
    setModulePerformanceDiagnosticsEnabledState(true);
    const older = beginModulePerformanceRun({ module: "skills", view: "overview", reason: "entry" }, { startedAt: 10 });
    const newer = beginModulePerformanceRun({ module: "skills", view: "overview", reason: "manual-refresh" }, { startedAt: 30 });
    if (!newer) throw new Error("expected enabled diagnostics to create a newer run");

    markModulePerformance(older, "older-visible", { at: 50, role: MODULE_PERFORMANCE_ROLE_VISIBLE });
    updateModulePerformanceSummary(older);
    markModulePerformance(newer, "newer-visible", { at: 40, role: MODULE_PERFORMANCE_ROLE_VISIBLE });
    updateModulePerformanceSummary(newer);

    expect(get(modulePerformanceSummaries).skills).toMatchObject({
      id: newer.id,
      reason: "manual-refresh",
      milestones: expect.arrayContaining([expect.objectContaining({ name: "newer-visible" })]),
    });
    expect(get(modulePerformanceSummaries).skills.milestones).not.toEqual(
      expect.arrayContaining([expect.objectContaining({ name: "older-visible" })])
    );
  });

  it("prevents older runs from another view from replacing the latest module summary", () => {
    setModulePerformanceDiagnosticsEnabledState(true);
    const older = beginModulePerformanceRun({ module: "config", view: "codex", reason: "entry" }, { startedAt: 10 });
    const newer = beginModulePerformanceRun({ module: "config", view: "claude-code", reason: "tool-switch" }, { startedAt: 30 });
    if (!newer) throw new Error("expected enabled diagnostics to create a newer run");

    markModulePerformance(newer, "new-tool-visible", { at: 40, role: MODULE_PERFORMANCE_ROLE_VISIBLE });
    updateModulePerformanceSummary(newer);
    markModulePerformance(older, "old-tool-complete", { at: 80, role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
    finishAndRecordModulePerformanceRun(older, "success");

    expect(get(modulePerformanceSummaries).config).toMatchObject({
      id: newer.id,
      view: "claude-code",
      reason: "tool-switch",
    });
    expect(writeModulePerformanceLog).not.toHaveBeenCalledWith(expect.objectContaining({
      id: older?.id,
      view: "codex",
    }));
  });

  it("writes only the current enabled summary", () => {
    setModulePerformanceDiagnosticsEnabledState(true);
    const run = beginModulePerformanceRun({ module: "settings", reason: "entry" }, { startedAt: 10 });
    markModulePerformance(run, "visible", { at: 12, role: MODULE_PERFORMANCE_ROLE_VISIBLE });

    finishAndRecordModulePerformanceRun(run, "success");

    expect(writeModulePerformanceLog).toHaveBeenCalledWith(expect.objectContaining({
      module: "settings",
      reason: "entry",
      status: "success",
      visibleMs: 2,
    }));
  });

  it("does not republish an active run after diagnostics are disabled", () => {
    setModulePerformanceDiagnosticsEnabledState(true);
    const run = beginModulePerformanceRun({ module: "mcp", reason: "entry" }, { startedAt: 10 });
    setModulePerformanceDiagnosticsEnabledState(false);

    markModulePerformance(run, "complete", { at: 20, role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
    finishAndRecordModulePerformanceRun(run, "success");

    expect(get(modulePerformanceSummaries)).toEqual({});
    expect(writeModulePerformanceLog).not.toHaveBeenCalled();
  });

  it("redacts path-like and secret-like labels before frontend summary publication", () => {
    const run = createModulePerformanceRun({
      module: "/Users/example/project",
      reason: "entry token=abc123",
      counters: { "/tmp/count": 1 },
    }, { startedAt: 10 });
    markModulePerformance(run, "/Users/example/project/file", { at: 11, role: "visible" });

    const summary = summarizeModulePerformanceRun(run);
    if (!summary) throw new Error("expected summary");

    expect(safeDiagnosticLabel("/Users/example/project")).toBe("[REDACTED_PATH]");
    expect(summary.module).toBe("[REDACTED_PATH]");
    expect(summary.reason).toBe("[REDACTED_SECRET]");
    expect(summary.counters).toEqual({ "[REDACTED_PATH]": 1 });
    expect(summary.milestones[0].name).toBe("[REDACTED_PATH]");
  });
});
