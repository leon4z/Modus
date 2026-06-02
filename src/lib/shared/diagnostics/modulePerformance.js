// Purpose: Shared opt-in module performance diagnostics for top-level app modules.
import { derived, get, writable } from "svelte/store";
import { writeModulePerformanceLog } from "$lib/shared/logging/api.js";

/**
 * @typedef {{ name: string, role: string, at: number }} ModulePerformanceMilestone
 * @typedef {{ label: string, startedAt: number, completedAt: number | null, durationMs: number | null, status: string, errorKind: string }} ModulePerformanceRequest
 * @typedef {{
 *   id: string,
 *   key: string,
 *   module: string,
 *   view: string,
 *   reason: string,
 *   startedAt: number,
 *   completedAt: number | null,
 *   status: string,
 *   milestones: ModulePerformanceMilestone[],
 *   requests: ModulePerformanceRequest[],
 *   counters: Record<string, number>,
 * }} ModulePerformanceRun
 */

export const MODULE_PERFORMANCE_ROLE_VISIBLE = "visible";
export const MODULE_PERFORMANCE_ROLE_INTERACTIVE = "interactive";
export const MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE = "background-complete";

let runSequence = 0;
const latestRunIdsByKey = new Map();
const latestRunIdsByModule = new Map();

export const modulePerformanceDiagnosticsEnabled = writable(false);
export const modulePerformanceSummaries = writable(/** @type {Record<string, any>} */ ({}));

export const activeModulePerformanceSummary = derived(
  modulePerformanceSummaries,
  ($summaries) => {
    /** @param {string} moduleId */
    return (moduleId) => $summaries[moduleId] || null;
  }
);

/** @param {boolean | null | undefined} enabled */
export function isModulePerformanceDiagnosticsEnabled(enabled) {
  return enabled === true;
}

/** @param {boolean} enabled */
export function setModulePerformanceDiagnosticsEnabledState(enabled) {
  modulePerformanceDiagnosticsEnabled.set(enabled === true);
  if (enabled !== true) {
    modulePerformanceSummaries.set({});
  }
}

function getNow() {
  if (typeof performance !== "undefined" && typeof performance.now === "function") {
    return performance.now();
  }
  return Date.now();
}

/** @param {number} value */
function roundMs(value) {
  return Math.round(value * 10) / 10;
}

/** @param {unknown} err */
function safeErrorKind(err) {
  if (err instanceof Error && err.name) return safeDiagnosticLabel(err.name);
  return "error";
}

/** @param {string | null | undefined} value */
function normalizeOptionalLabel(value) {
  return safeDiagnosticLabel(String(value || "").trim());
}

/** @param {string} value */
export function safeDiagnosticLabel(value) {
  const text = String(value || "").trim();
  if (!text) return "";
  if (/[\\/]/.test(text)) return "[REDACTED_PATH]";
  if (/(password|passwd|secret|token|api[_-]?key|access[_-]?key)\s*[:=]/i.test(text)) return "[REDACTED_SECRET]";
  return text.slice(0, 80);
}

/** @param {string} module @param {string} view */
function runKey(module, view) {
  return `${module || "unknown"}:${view || ""}`;
}

/**
 * @param {{ module: string, view?: string, reason: string, counters?: Record<string, number> }} params
 * @param {{ startedAt?: number }} [options]
 * @returns {ModulePerformanceRun}
 */
export function createModulePerformanceRun(params, options = {}) {
  const startedAt = Number.isFinite(options.startedAt) ? Number(options.startedAt) : getNow();
  const module = normalizeOptionalLabel(params.module) || "unknown";
  const view = normalizeOptionalLabel(params.view);
  runSequence += 1;
  const run = /** @type {ModulePerformanceRun} */ ({
    id: `module-perf-${runSequence}`,
    key: runKey(module, view),
    module,
    view,
    reason: normalizeOptionalLabel(params.reason) || "entry",
    startedAt,
    completedAt: null,
    status: "running",
    milestones: [],
    requests: [],
    counters: normalizeCounters(params.counters || {}),
  });
  latestRunIdsByKey.set(run.key, run.id);
  latestRunIdsByModule.set(run.module, run.id);
  return run;
}

/**
 * @param {{ module: string, view?: string, reason: string, counters?: Record<string, number> }} params
 * @param {{ startedAt?: number }} [options]
 */
export function beginModulePerformanceRun(params, options = {}) {
  if (!get(modulePerformanceDiagnosticsEnabled)) return null;
  const run = createModulePerformanceRun(params, options);
  markModulePerformance(run, "run-start");
  updateModulePerformanceSummary(run);
  return run;
}

/** @param {ModulePerformanceRun | null | undefined} run */
export function isCurrentModulePerformanceRun(run) {
  return Boolean(run && latestRunIdsByKey.get(run.key) === run.id && latestRunIdsByModule.get(run.module) === run.id);
}

/**
 * @param {ModulePerformanceRun | null | undefined} run
 * @param {string} name
 * @param {{ at?: number, role?: string }} [options]
 */
export function markModulePerformance(run, name, options = {}) {
  if (!run || !isCurrentModulePerformanceRun(run)) return null;
  const at = Number.isFinite(options.at) ? Number(options.at) : getNow();
  run.milestones.push({
    name: normalizeOptionalLabel(name) || "milestone",
    role: normalizeOptionalLabel(options.role),
    at,
  });
  return run;
}

/**
 * @template T
 * @param {ModulePerformanceRun | null | undefined} run
 * @param {string} label
 * @param {() => Promise<T>} task
 * @param {{ startedAt?: number }} [options]
 * @returns {Promise<T>}
 */
export async function trackModulePerformanceRequest(run, label, task, options = {}) {
  if (!run || !isCurrentModulePerformanceRun(run)) return task();
  const startedAt = Number.isFinite(options.startedAt) ? Number(options.startedAt) : getNow();
  const request = /** @type {ModulePerformanceRequest} */ ({
    label: normalizeOptionalLabel(label) || "request",
    startedAt,
    completedAt: null,
    durationMs: null,
    status: "running",
    errorKind: "",
  });
  run.requests.push(request);

  try {
    const result = await task();
    request.status = "success";
    return result;
  } catch (err) {
    request.status = "failed";
    request.errorKind = safeErrorKind(err);
    throw err;
  } finally {
    const completedAt = getNow();
    request.completedAt = completedAt;
    request.durationMs = roundMs(completedAt - startedAt);
  }
}

/**
 * @param {ModulePerformanceRun | null | undefined} run
 * @param {Record<string, number>} counters
 */
export function setModulePerformanceCounters(run, counters) {
  if (!run || !isCurrentModulePerformanceRun(run)) return;
  run.counters = normalizeCounters(counters || {});
}

/** @param {Record<string, number>} counters */
function normalizeCounters(counters) {
  return Object.entries(counters || {}).reduce((acc, [key, value]) => {
    const label = normalizeOptionalLabel(key);
    if (!label) return acc;
    const numeric = Number(value);
    if (Number.isFinite(numeric)) acc[label] = numeric;
    return acc;
  }, /** @type {Record<string, number>} */ ({}));
}

/** @param {ModulePerformanceRun | null | undefined} run @param {string} [status] */
export function finishModulePerformanceRun(run, status = "success") {
  if (!run) return null;
  if (!isCurrentModulePerformanceRun(run)) {
    run.status = "cancelled";
    run.completedAt = getNow();
    return summarizeModulePerformanceRun(run);
  }
  run.status = normalizeOptionalLabel(status) || "success";
  run.completedAt = getNow();
  return summarizeModulePerformanceRun(run);
}

/** @param {ModulePerformanceRun | null | undefined} run */
export function summarizeModulePerformanceRun(run) {
  if (!run) return null;
  const endAt = Number.isFinite(run.completedAt) ? Number(run.completedAt) : getNow();
  const requestCounts = run.requests.reduce((counts, request) => {
    counts[request.label] = (counts[request.label] || 0) + 1;
    return counts;
  }, /** @type {Record<string, number>} */ ({}));
  const milestones = run.milestones.map((milestone) => ({
    name: milestone.name,
    role: milestone.role,
    atMs: roundMs(milestone.at - run.startedAt),
  }));
  const visibleMs = firstMilestoneMs(milestones, MODULE_PERFORMANCE_ROLE_VISIBLE);
  const interactiveMs = firstMilestoneMs(milestones, MODULE_PERFORMANCE_ROLE_INTERACTIVE) ?? visibleMs;
  const backgroundCompleteMs = firstMilestoneMs(milestones, MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE) ?? roundMs(endAt - run.startedAt);

  return {
    id: run.id,
    module: run.module,
    view: run.view || null,
    reason: run.reason,
    status: run.status,
    visibleMs,
    interactiveMs,
    backgroundCompleteMs,
    totalMs: roundMs(endAt - run.startedAt),
    requestCount: run.requests.length,
    requestCounts,
    counters: { ...run.counters },
    milestones,
    requests: run.requests.map((request) => ({
      label: request.label,
      status: request.status,
      durationMs: request.durationMs,
      errorKind: request.errorKind,
    })),
  };
}

/** @param {{ role?: string, atMs?: number }[]} milestones @param {string} role */
function firstMilestoneMs(milestones, role) {
  const milestone = milestones.find((entry) => entry.role === role);
  if (!milestone || !Number.isFinite(milestone.atMs)) return null;
  return milestone.atMs ?? null;
}

/** @param {ModulePerformanceRun | null | undefined} run */
export function updateModulePerformanceSummary(run) {
  if (!run || !isCurrentModulePerformanceRun(run)) return null;
  const summary = summarizeModulePerformanceRun(run);
  if (!summary) return null;
  if (!get(modulePerformanceDiagnosticsEnabled)) return summary;
  modulePerformanceSummaries.update((summaries) => ({ ...summaries, [summary.module]: summary }));
  return summary;
}

/** @param {ModulePerformanceRun | null | undefined} run @param {string} status */
export function finishAndRecordModulePerformanceRun(run, status = "success") {
  if (!run) return null;
  const summary = finishModulePerformanceRun(run, status);
  if (!summary || !isCurrentModulePerformanceRun(run)) return summary;
  if (get(modulePerformanceDiagnosticsEnabled)) {
    modulePerformanceSummaries.update((summaries) => ({ ...summaries, [summary.module]: summary }));
    void Promise.resolve(writeModulePerformanceLog(summary)).catch(() => {});
  }
  return summary;
}
