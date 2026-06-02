export const TOOL_SELECTOR_ICON_SEGMENT_WIDTH = 36;
export const TOOL_SELECTOR_TRACK_PADDING_X = 2;
export const TOOL_SELECTOR_SEGMENT_GAP = 2;
export const TOOL_SELECTOR_LABEL_HYSTERESIS = 16;

/**
 * Calculate the protected icon-only capsule width for a managed-tool selector.
 * @param {number} toolCount
 * @param {{ segmentWidth?: number, trackPaddingX?: number, gap?: number }} [options]
 * @returns {number}
 */
export function protectedToolSelectorMinWidth(toolCount, options = {}) {
  const count = Math.max(0, Number(toolCount) || 0);
  if (count === 0) return 0;
  const segmentWidth = options.segmentWidth ?? TOOL_SELECTOR_ICON_SEGMENT_WIDTH;
  const trackPaddingX = options.trackPaddingX ?? TOOL_SELECTOR_TRACK_PADDING_X;
  const gap = options.gap ?? TOOL_SELECTOR_SEGMENT_GAP;
  return (trackPaddingX * 2) + (count * segmentWidth) + ((count - 1) * gap);
}

/**
 * Decide whether the capsule can show full labels without flickering at the threshold.
 * @param {{ availableWidth: number, fullWidth: number, currentMode?: "full" | "icon", hysteresis?: number }} input
 * @returns {"full" | "icon"}
 */
export function resolveToolSelectorMode(input) {
  const currentMode = input.currentMode === "icon" ? "icon" : "full";
  const availableWidth = Number(input.availableWidth) || 0;
  const fullWidth = Number(input.fullWidth) || 0;
  const hysteresis = input.hysteresis ?? TOOL_SELECTOR_LABEL_HYSTERESIS;

  if (availableWidth <= 0 || fullWidth <= 0) return currentMode;
  if (currentMode === "icon") {
    return fullWidth + hysteresis <= availableWidth ? "full" : "icon";
  }
  return fullWidth <= availableWidth ? "full" : "icon";
}
