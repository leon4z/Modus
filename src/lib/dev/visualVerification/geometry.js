// Purpose: Measure top-level visual verification surfaces for header rhythm review.

export const PRIMARY_SURFACE_GEOMETRY_SELECTORS = {
  header: ".view-fixed-header",
  titleRow: ".view-header",
  pinnedToolbar: ".view-pinned-toolbar",
  selectorRow: ".view-pinned-toolbar .level2-tabs, .view-pinned-toolbar .project-context-row, .view-pinned-toolbar .tool-selector-capsule",
  content: ".view-scroll-content",
  primaryState: ".primary-state, .empty",
};

/**
 * @param {DOMRect | { x?: number, y?: number, width?: number, height?: number }} rect
 */
function normalizeRect(rect) {
  return {
    x: Math.round(Number(rect.x ?? 0)),
    y: Math.round(Number(rect.y ?? 0)),
    width: Math.round(Number(rect.width ?? 0)),
    height: Math.round(Number(rect.height ?? 0)),
  };
}

/**
 * @param {ParentNode} root
 * @param {string} selector
 */
function measureSelector(root, selector) {
  const element = root.querySelector(selector);
  if (!element || typeof element.getBoundingClientRect !== "function") return null;
  return normalizeRect(element.getBoundingClientRect());
}

/**
 * @param {ParentNode} root
 */
export function measurePrimarySurfaceGeometry(root = document) {
  return Object.fromEntries(
    Object.entries(PRIMARY_SURFACE_GEOMETRY_SELECTORS).map(([key, selector]) => [
      key,
      measureSelector(root, selector),
    ])
  );
}

/**
 * @param {number | undefined} left
 * @param {number | undefined} right
 * @param {number} tolerance
 */
function differs(left, right, tolerance) {
  if (typeof left !== "number" || typeof right !== "number") return false;
  return Math.abs(left - right) > tolerance;
}

/**
 * @param {Record<string, any>} reference
 * @param {Record<string, any>} candidate
 * @param {{ tolerance?: number }} options
 */
export function comparePrimaryHeaderGeometry(reference, candidate, options = {}) {
  const tolerance = options.tolerance ?? 1;
  const checks = [
    ["header.height", reference.header?.height, candidate.header?.height],
    ["titleRow.height", reference.titleRow?.height, candidate.titleRow?.height],
    ["content.y", reference.content?.y, candidate.content?.y],
    ["pinnedToolbar.y", reference.pinnedToolbar?.y, candidate.pinnedToolbar?.y],
    ["selectorRow.y", reference.selectorRow?.y, candidate.selectorRow?.y],
    ["primaryState.y", reference.primaryState?.y, candidate.primaryState?.y],
  ];
  return checks
    .filter(([, left, right]) => differs(left, right, tolerance))
    .map(([field, expected, actual]) => ({ field, expected, actual }));
}
