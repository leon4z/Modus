import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const ruleApiMocks = vi.hoisted(() => ({
  writeRule: vi.fn(),
}));
const appLoggerMocks = vi.hoisted(() => ({
  logAppEvent: vi.fn(),
}));
const toolsStoreMocks = vi.hoisted(() => ({
  loadTools: vi.fn(),
}));

vi.mock("$lib/features/rules/api/rules.js", () => ruleApiMocks);
vi.mock("$lib/shared/logging/appLogger.js", () => appLoggerMocks);
vi.mock("$lib/features/tools/index.js", async () => {
  return {
    loadTools: toolsStoreMocks.loadTools,
  };
});

import RuleEditor from "$lib/features/rules/components/RuleEditor.svelte";

/**
 * @param {import("@testing-library/user-event").UserEvent} user
 * @param {string} text
 */
async function replaceSourceEditorText(user, text) {
  const editor = screen.getByRole("textbox");
  await user.click(editor);
  await user.keyboard("{Control>}a{/Control}");
  await user.paste(text);
  return editor;
}

describe("RuleEditor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    ruleApiMocks.writeRule.mockResolvedValue(undefined);
    toolsStoreMocks.loadTools.mockResolvedValue(undefined);
    appLoggerMocks.logAppEvent.mockResolvedValue(undefined);
  });

  afterEach(() => {
    cleanup();
  });

  it("uses the shared line-numbered editor for rule files", async () => {
    const user = userEvent.setup();
    const rule = {
      label: "Demo Rule",
      path: "/tmp/rule.md",
      content: "first\nsecond",
    };

    render(RuleEditor, {
      rule,
      toolId: "codex",
      onClose: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "编辑" }));
    expect(screen.getByRole("textbox")).toBeInTheDocument();
    expect(document.querySelector(".content-editor")).toHaveClass("unframed");

    await replaceSourceEditorText(user, "updated");
    await user.click(screen.getByRole("button", { name: "保存更改" }));

    await waitFor(() => {
      expect(ruleApiMocks.writeRule).toHaveBeenCalledWith("codex", "/tmp/rule.md", "updated");
    });
  });
});
