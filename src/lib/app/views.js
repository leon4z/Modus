// Purpose: Declare the primary app views used by app composition.

export const PRIMARY_VIEWS = ["dashboard", "rules", "skills", "mcp", "config", "settings"];
export const DEFAULT_PRIMARY_VIEW = "dashboard";

/** @param {string | null | undefined} view */
export function isPrimaryView(view) {
  return PRIMARY_VIEWS.includes(String(view || ""));
}

/** @param {string | null | undefined} view */
export function normalizePrimaryView(view) {
  return isPrimaryView(view) ? String(view) : DEFAULT_PRIMARY_VIEW;
}
