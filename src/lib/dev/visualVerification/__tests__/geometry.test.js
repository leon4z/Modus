import { describe, expect, it } from "vitest";

import {
  comparePrimaryHeaderGeometry,
  measurePrimarySurfaceGeometry,
} from "$lib/dev/visualVerification/geometry.js";

/**
 * @param {number} x
 * @param {number} y
 * @param {number} width
 * @param {number} height
 */
function elementRect(x, y, width, height) {
  return {
    getBoundingClientRect: () => ({ x, y, width, height }),
  };
}

/**
 * @param {Record<string, { getBoundingClientRect: () => { x: number, y: number, width: number, height: number } }>} rects
 * @returns {ParentNode}
 */
function rootWithRects(rects) {
  const root = {
    /** @param {string} selector */
    querySelector(selector) {
      if (rects[selector]) return rects[selector];
      for (const part of selector.split(",")) {
        const match = rects[part.trim()];
        if (match) return match;
      }
      return null;
    },
  };
  return /** @type {ParentNode} */ (/** @type {unknown} */ (root));
}

describe("primary surface geometry helpers", () => {
  it("measures primary header, selector, content, and state geometry", () => {
    const root = rootWithRects({
      ".view-fixed-header": elementRect(0, 0, 800, 59),
      ".view-header": elementRect(20, 0, 760, 58),
      ".view-pinned-toolbar": elementRect(0, 59, 800, 48),
      ".view-pinned-toolbar .level2-tabs": elementRect(20, 75, 760, 32),
      ".view-scroll-content": elementRect(0, 107, 800, 600),
      ".primary-state, .empty": elementRect(20, 123, 760, 135),
    });

    expect(measurePrimarySurfaceGeometry(root)).toMatchObject({
      header: { y: 0, height: 59 },
      titleRow: { y: 0, height: 58 },
      selectorRow: { y: 75, height: 32 },
      content: { y: 107, height: 600 },
      primaryState: { y: 123, height: 135 },
    });
  });

  it("measures project context rows as primary selector rows", () => {
    const root = rootWithRects({
      ".view-fixed-header": elementRect(0, 0, 800, 59),
      ".view-header": elementRect(20, 0, 760, 58),
      ".view-pinned-toolbar": elementRect(0, 59, 800, 48),
      ".view-pinned-toolbar .project-context-row": elementRect(20, 75, 760, 32),
      ".view-scroll-content": elementRect(0, 107, 800, 600),
      ".primary-state, .empty": elementRect(20, 123, 760, 135),
    });

    expect(measurePrimarySurfaceGeometry(root)).toMatchObject({
      selectorRow: { y: 75, height: 32 },
    });
  });

  it("measures protected tool capsules as primary selector rows", () => {
    const root = rootWithRects({
      ".view-fixed-header": elementRect(0, 0, 800, 59),
      ".view-header": elementRect(20, 0, 760, 58),
      ".view-pinned-toolbar": elementRect(0, 59, 800, 48),
      ".view-pinned-toolbar .tool-selector-capsule": elementRect(20, 75, 284, 32),
      ".view-scroll-content": elementRect(0, 107, 800, 600),
      ".primary-state, .empty": elementRect(20, 123, 760, 135),
    });

    expect(measurePrimarySurfaceGeometry(root)).toMatchObject({
      selectorRow: { y: 75, height: 32 },
    });
  });

  it("reports header rhythm differences beyond tolerance", () => {
    const reference = {
      header: { height: 59 },
      titleRow: { height: 58 },
      content: { y: 107 },
      pinnedToolbar: { y: 59 },
      selectorRow: { y: 75 },
      primaryState: { y: 123 },
    };
    const candidate = {
      header: { height: 45 },
      titleRow: { height: 44 },
      content: { y: 93 },
      pinnedToolbar: { y: 45 },
      selectorRow: { y: 61 },
      primaryState: { y: 109 },
    };

    expect(comparePrimaryHeaderGeometry(reference, candidate, { tolerance: 1 })).toEqual([
      { field: "header.height", expected: 59, actual: 45 },
      { field: "titleRow.height", expected: 58, actual: 44 },
      { field: "content.y", expected: 107, actual: 93 },
      { field: "pinnedToolbar.y", expected: 59, actual: 45 },
      { field: "selectorRow.y", expected: 75, actual: 61 },
      { field: "primaryState.y", expected: 123, actual: 109 },
    ]);
  });
});
