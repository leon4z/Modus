import { writable, derived, get } from "svelte/store";
import { listTools } from "$lib/features/tools/api/tools.js";

/**
 * @typedef {{id: string, detected: boolean, [key: string]: any}} Tool
 */

/** @type {import('svelte/store').Writable<Tool[]>} */
export const tools = writable([]);
/** @type {import('svelte/store').Writable<string | null>} */
export const activeToolId = writable(null);
export const activeView = writable("dashboard");
/** @type {import('svelte/store').Writable<string | null>} */
export const pendingSubTab = writable(null); // set to "tool" when navigating from dashboard with toolId
/** @type {import('svelte/store').Writable<"global" | "custom" | "tool">} */
export const activeRulesTab = writable("global");
/** @type {import('svelte/store').Writable<string[]>} */
export const managedToolIds = writable([]); // tool IDs user chose to manage
export const appInitialized = writable(false);
export const theme = writable("system"); // "light", "dark", "system"

/** @param {Tool} tool */
function toolDisplayName(tool) {
  return String(tool?.name || tool?.tool_name || tool?.id || "");
}

/**
 * Sort tools by the user-visible name with a stable id tie-breaker.
 * @param {Tool} a
 * @param {Tool} b
 */
export function compareToolsByDisplayName(a, b) {
  return toolDisplayName(a).localeCompare(toolDisplayName(b), undefined, { sensitivity: "base" })
    || String(a?.id || a?.tool_id || "").localeCompare(String(b?.id || b?.tool_id || ""));
}

/** @param {Tool[]} list */
export function sortToolsByDisplayName(list) {
  return [...(list || [])].sort(compareToolsByDisplayName);
}

export const activeTool = derived(
  [tools, activeToolId],
  ([$tools, $activeToolId]) => {
    const sorted = sortToolsByDisplayName($tools);
    if (!$activeToolId) return sorted.find((/** @type {Tool} */ t) => t.detected) || null;
    return $tools.find((/** @type {Tool} */ t) => t.id === $activeToolId) || null;
  }
);

// All detected tools (installed on disk)
export const detectedTools = derived(tools, ($tools) =>
  sortToolsByDisplayName($tools.filter((/** @type {Tool} */ t) => t.detected))
);

// Only managed & detected tools — used in rules/skills/config tabs
export const managedTools = derived(
  [detectedTools, managedToolIds],
  ([$detected, $managed]) => {
    const managed = new Set($managed.map(String));
    return sortToolsByDisplayName($detected.filter((/** @type {Tool} */ t) => managed.has(t.id)));
  }
);

export async function loadTools() {
  const previousActiveToolId = get(activeToolId);
  const result = sortToolsByDisplayName(/** @type {Tool[]} */ (await listTools()));
  tools.set(result);
  const previousStillDetected = previousActiveToolId
    && result.some((/** @type {Tool} */ t) => t.id === previousActiveToolId && t.detected);
  const first = result.find((/** @type {Tool} */ t) => t.detected);
  if (!previousStillDetected && first) {
    activeToolId.set(first.id);
  }
  return result;
}

/**
 * Returns the official display name for a given tool ID.
 * @param {string} toolId
 * @param {Tool[]} toolsList - The current list of tools (e.g. $tools)
 * @returns {string}
 */
export function getToolName(toolId, toolsList = []) {
  if (!toolId) return "";
  if (toolId === "generic") return "共享来源";
  const tool = toolsList.find((t) => t.id === toolId);
  return tool ? tool.name : toolId;
}
