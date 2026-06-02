// @ts-nocheck
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

function readSource(path) {
  return readFileSync(path, "utf8");
}

function cssBlockAt(source, marker, markerIndex) {
  const openIndex = source.indexOf("{", markerIndex);
  expect(openIndex, `Missing CSS block for: ${marker}`).toBeGreaterThanOrEqual(0);

  let depth = 0;
  for (let index = openIndex; index < source.length; index += 1) {
    const char = source[index];
    if (char === "{") depth += 1;
    if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(markerIndex, index + 1);
      }
    }
  }

  throw new Error(`Unclosed CSS block for: ${marker}`);
}

function cssBlock(source, marker) {
  const markerIndex = source.indexOf(marker);
  expect(markerIndex, `Missing CSS marker: ${marker}`).toBeGreaterThanOrEqual(0);
  return cssBlockAt(source, marker, markerIndex);
}

function cssBlocks(source, marker) {
  const blocks = [];
  let markerIndex = source.indexOf(marker);
  while (markerIndex >= 0) {
    blocks.push(cssBlockAt(source, marker, markerIndex));
    markerIndex = source.indexOf(marker, markerIndex + marker.length);
  }
  return blocks;
}

function expectAnyCssBlockToContain(source, marker, expected) {
  const blocks = cssBlocks(source, marker);
  expect(blocks.length, `Missing CSS marker: ${marker}`).toBeGreaterThan(0);
  expect(blocks.some((block) => block.includes(expected)), `${marker} should contain ${expected}`).toBe(true);
}

describe("button visual system", () => {
  it("keeps shared neutral, primary, destructive, and icon-only actions borderless", () => {
    const appCss = readSource("src/app.css");

    expect(cssBlock(appCss, ".btn-primary,")).toContain("border: none;");
    expect(cssBlock(appCss, ".btn-secondary,")).toContain("background: var(--bg-subtle);");
    expect(cssBlock(appCss, ".btn-secondary,")).toContain("border: none;");
    expect(cssBlock(appCss, ".btn-danger")).toContain("background: var(--bg-danger);");
    expect(cssBlock(appCss, ".btn-danger")).toContain("border: none;");

    expect(cssBlock(appCss, ".file-viewer-btn,")).toContain("border: none;");
    expect(cssBlock(appCss, ".file-viewer-btn {")).toContain("background: var(--bg-subtle);");
    expect(cssBlock(appCss, ".file-viewer-primary-btn {")).toContain("background: var(--color-text-main);");
    expect(cssBlock(appCss, ".file-viewer-btn.danger")).toContain("background: var(--bg-danger);");
    expect(cssBlock(appCss, ".file-viewer-icon-btn:hover:not(:disabled),")).toContain("background: var(--bg-hover);");
    expect(cssBlock(appCss, ".file-viewer-icon-btn:hover:not(:disabled),")).toContain(
      ".file-viewer-icon-btn:focus-visible:not(:disabled)",
    );

    const iconBlock = cssBlock(appCss, "\n.icon-btn {\n");
    expect(iconBlock).toContain("background: transparent;");
    expect(iconBlock).toContain("border: none;");
    expect(iconBlock).toContain("opacity: 0.42;");
  });

  it("uses borderless action treatments in content viewer surfaces", () => {
    const contentEditor = readSource("src/lib/shared/components/ContentEditor.svelte");
    expect(cssBlock(contentEditor, ".btn-text")).toContain("background: var(--bg-subtle);");
    expect(cssBlock(contentEditor, ".btn-text")).toContain("border: none;");
    expect(cssBlock(contentEditor, ".primary-pill-btn")).toContain("border: none;");
    expect(cssBlock(contentEditor, ".secondary-pill-btn")).toContain("background: var(--bg-subtle);");
    expect(cssBlock(contentEditor, ".secondary-pill-btn")).toContain("border: none;");

    const compactEditor = readSource("src/lib/shared/components/CompactTextEditor.svelte");
    expect(cssBlock(compactEditor, ".btn-primary")).toContain("border: none;");
    expect(cssBlock(compactEditor, ".btn-secondary")).toContain("background: var(--bg-subtle);");

    for (const path of [
      "src/lib/features/config/components/ConfigPanel.svelte",
      "src/lib/features/mcp/components/McpPanel.svelte",
      "src/lib/features/rules/components/RuleEditor.svelte",
      "src/lib/features/rules/components/RulesModule.svelte",
    ]) {
      const source = readSource(path);
      expect(source).toContain("file-viewer-btn");
      expect(source).toContain("file-viewer-primary-btn");
      expect(source).not.toContain("border: 1px solid var(--border-color);\n    background: transparent");
    }

    const skillViewer = readSource("src/lib/features/skills/components/SkillViewer.svelte");
    expect(cssBlock(skillViewer, ".icon-btn")).toContain("background: transparent;");
    expect(cssBlock(skillViewer, ".icon-btn")).toContain("border: none;");
    expect(cssBlock(skillViewer, ".icon-btn")).toContain("opacity: 0.42;");
    expect(cssBlock(skillViewer, ".icon-btn:hover,")).toContain(".icon-btn:focus-visible");
    expectAnyCssBlockToContain(skillViewer, ".icon-btn:focus-visible", "outline: 1px solid var(--toolbar-control-border-active);");
    expect(cssBlock(skillViewer, ".btn-action-install,\n  .btn-text")).toContain("border: none;");
    expectAnyCssBlockToContain(skillViewer, ".btn-action-danger", "background: var(--bg-danger);");
    expectAnyCssBlockToContain(skillViewer, ".primary-pill-btn", "background: var(--color-text-main);");
  });

  it("keeps Settings form tools on shared icon and destructive button treatments", () => {
    const sourceEditor = readSource("src/lib/shared/components/SourceCodeEditor.svelte");
    expect(sourceEditor).not.toContain("source-search-panel");
    expect(sourceEditor).not.toContain("source-search-icon-btn");

    const settingsModule = readSource("src/lib/features/settings/components/SettingsModule.svelte");
    expect(settingsModule).not.toContain(".btn-pick");
    expect(settingsModule).not.toContain(".btn-reset");
    expect(settingsModule).toContain("icon-btn st-picker-btn");
    expect(cssBlock(settingsModule, ".st-picker-btn {")).not.toContain("border:");
    expect(cssBlock(settingsModule, ".btn-delete {")).toContain("background: var(--bg-danger);");
    expect(cssBlock(settingsModule, ".btn-delete {")).toContain("border: none;");
  });

  it("docks the global search close button to the search field edge", () => {
    const appCss = readSource("src/app.css");
    expect(cssBlock(appCss, "\n.search-input-wrap {\n")).toContain("--search-input-inline-padding: 10px;");
    expect(cssBlock(appCss, "\n.search-input-wrap {\n")).toContain("padding: 0 var(--search-input-inline-padding);");

    const trailingActionBlock = cssBlock(appCss, ".search-input-wrap .icon-btn-tiny");
    expect(trailingActionBlock).toContain("align-self: stretch;");
    expect(trailingActionBlock).toContain("height: auto;");
    expect(trailingActionBlock).toContain("margin-right: calc(-1 * var(--search-input-inline-padding));");
    expect(trailingActionBlock).toContain("border-radius: 7px;");
  });

  it("keeps file workspace search popovers interactive inside draggable headers", () => {
    const appCss = readSource("src/app.css");
    const resultsSource = readSource("src/lib/shared/components/ModuleSearchResults.svelte");

    const fixedHeaderBlock = cssBlock(appCss, "\n.view-fixed-header {");
    expect(fixedHeaderBlock).not.toContain("z-index:");
    expect(fixedHeaderBlock).not.toContain("overflow: visible;");
    const popoverHeaderBlock = cssBlock(appCss, ".view-fixed-header.view-fixed-header--search-popover");
    expect(popoverHeaderBlock).toContain("position: relative;");
    expect(popoverHeaderBlock).toContain("z-index: 30;");
    expect(popoverHeaderBlock).toContain("overflow: visible;");

    const anchorBlock = cssBlock(appCss, ".workspace-search-anchor");
    expect(anchorBlock).toContain("pointer-events: auto;");
    expect(anchorBlock).toContain("-webkit-app-region: no-drag;");

    const actionRailBlock = cssBlock(appCss, ".view-header.management-header-row .management-header-actions .refresh-btn,");
    expect(actionRailBlock).toContain(".workspace-search-anchor");
    expect(actionRailBlock).toContain("pointer-events: auto;");
    expect(actionRailBlock).toContain("-webkit-app-region: no-drag;");

    const searchOpenBlock = cssBlock(appCss, ".view-header.management-header-row.management-header-row--search-open .management-header-actions .workspace-search-anchor,");
    expect(searchOpenBlock).toContain(".search-input-wrap");
    expect(searchOpenBlock).toContain("flex: 1 1 160px;");
    expect(searchOpenBlock).toContain("max-width: 100%;");
    const anchoredInputBlock = cssBlock(appCss, ".view-header.management-header-row.management-header-row--search-open .management-header-actions .workspace-search-anchor > .search-input-wrap");
    expect(anchoredInputBlock).toContain("width: 100%;");

    const resultBlock = cssBlock(resultsSource, "\n  .module-search-results");
    expect(resultBlock).toContain("z-index: 3200;");
    expect(resultBlock).toContain("pointer-events: auto;");
    expect(resultBlock).toContain("-webkit-app-region: no-drag;");
  });
});
