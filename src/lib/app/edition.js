// Purpose: Declare the Community edition surface used by app composition.

export const COMMUNITY_VIEWS = ["dashboard", "rules", "skills", "mcp", "config", "settings"];
export const DEFAULT_COMMUNITY_VIEW = "dashboard";

/** @param {string | null | undefined} view */
export function isCommunityView(view) {
  return COMMUNITY_VIEWS.includes(String(view || ""));
}

/** @param {string | null | undefined} view */
export function normalizeCommunityView(view) {
  return isCommunityView(view) ? String(view) : DEFAULT_COMMUNITY_VIEW;
}
