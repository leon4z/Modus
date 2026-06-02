import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import CompactTextEditor from "$lib/shared/components/CompactTextEditor.svelte";
import AppContextMenu from "$lib/shared/components/AppContextMenu.svelte";
import ContentEditor from "$lib/shared/components/ContentEditor.svelte";

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

function sourceEditorLineNumbers() {
  return [...document.querySelectorAll(".cm-lineNumbers .cm-gutterElement")]
    .filter((node) => !node.getAttribute("style")?.includes("visibility: hidden"))
    .map((node) => node.textContent);
}

/**
 * @param {Element | Window} target
 * @param {string} key
 * @param {string} code
 */
async function pressPlatformShortcut(target, key, code) {
  const isMac = /Mac|iPhone|iPad|iPod/.test(navigator.platform);
  await fireEvent.keyDown(target, {
    key,
    code,
    ctrlKey: !isMac,
    metaKey: isMac,
  });
}

function stubClipboard(overrides = {}) {
  const clipboard = {
    readText: vi.fn().mockResolvedValue("pasted"),
    writeText: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: clipboard,
  });
  return clipboard;
}

describe("ContentEditor", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders line-numbered source view", () => {
    render(ContentEditor, {
      value: "alpha\nbeta",
      originalValue: "alpha\nbeta",
      editing: false,
      markdown: false,
      filename: "demo.txt",
      showFooter: false,
    });

    expect(document.querySelector(".source-view")).not.toBeNull();
    expect(screen.getByRole("textbox")).toHaveTextContent("alpha");
    expect(sourceEditorLineNumbers()).toEqual(["1", "2"]);
  });

  it("can render inside an existing file shell without an extra body frame", () => {
    render(ContentEditor, {
      value: "alpha",
      originalValue: "alpha",
      editing: false,
      markdown: false,
      filename: "demo.txt",
      showFooter: false,
      framed: false,
    });

    expect(document.querySelector(".content-editor")).toHaveClass("unframed");
  });

  it("preserves the trailing empty line in source line numbers", () => {
    render(ContentEditor, {
      value: "alpha\n",
      originalValue: "alpha\n",
      editing: false,
      markdown: false,
      filename: "demo.txt",
      showFooter: false,
    });

    expect(sourceEditorLineNumbers()).toEqual(["1", "2"]);
  });

  it("escapes unsafe markdown preview content", () => {
    render(ContentEditor, {
      value: '<script>alert(1)</script>\n[bad](javascript:alert(1))',
      originalValue: '<script>alert(1)</script>\n[bad](javascript:alert(1))',
      editing: false,
      markdown: true,
      preview: true,
      filename: "demo.md",
      showFooter: false,
    });

    expect(document.querySelector("script")).toBeNull();
    expect(document.querySelector("a[href^='javascript']")).toBeNull();
    expect(screen.getByText(/<script>alert\(1\)<\/script>/)).toBeInTheDocument();
  });

  it("shows an explicit empty state for blank markdown preview", () => {
    render(ContentEditor, {
      value: "",
      originalValue: "",
      editing: false,
      markdown: true,
      preview: true,
      filename: "demo.md",
      showFooter: false,
      emptyPreviewLabel: "File is empty",
    });

    expect(screen.getByText("File is empty")).toBeInTheDocument();
    expect(document.querySelector(".md-rendered")).toBeNull();
  });

  it("highlights external search matches in markdown preview", () => {
    render(ContentEditor, {
      value: "alpha beta alpha",
      originalValue: "alpha beta alpha",
      editing: false,
      markdown: true,
      preview: true,
      filename: "demo.md",
      showFooter: false,
      externalSearchQuery: "alpha",
      externalSearchMatchIndex: 1,
      externalSearchVersion: 1,
    });

    expect([...document.querySelectorAll(".workspace-search-highlight")].map((node) => node.textContent)).toEqual(["alpha", "alpha"]);
    expect(document.querySelector(".workspace-search-highlight--active")?.textContent).toBe("alpha");
  });

  it("does not steal focus from an external search input while highlighting source matches", async () => {
    const searchInput = document.createElement("input");
    document.body.append(searchInput);
    try {
      const view = render(ContentEditor, {
        value: "alpha beta",
        originalValue: "alpha beta",
        editing: false,
        markdown: false,
        filename: "demo.txt",
        showFooter: false,
      });

      searchInput.focus();
      expect(searchInput).toHaveFocus();

      await view.rerender({
        value: "alpha beta",
        originalValue: "alpha beta",
        editing: false,
        markdown: false,
        filename: "demo.txt",
        showFooter: false,
        externalSearchQuery: "alpha",
        externalSearchVersion: 1,
      });

      expect(searchInput).toHaveFocus();
      await waitFor(() => {
        expect(document.querySelector(".cm-searchMatch")).not.toBeNull();
      });
    } finally {
      searchInput.remove();
    }
  });

  it("uses accessible icon controls for source/read mode switching", async () => {
    const user = userEvent.setup();
    render(ContentEditor, {
      value: "# Hello",
      originalValue: "# Hello",
      editing: false,
      markdown: true,
      preview: true,
      filename: "demo.md",
      showFooter: false,
      sourceLabel: "源码",
      readLabel: "阅读",
    });

    const sourceButton = screen.getByRole("button", { name: "源码" });
    const readButton = screen.getByRole("button", { name: "阅读" });
    expect(sourceButton).not.toHaveAttribute("title");
    expect(readButton).not.toHaveAttribute("title");
    const tooltipTexts = [...document.querySelectorAll('[role="tooltip"]')].map((node) => node.textContent);
    expect(tooltipTexts).not.toContain("源码");
    expect(tooltipTexts).not.toContain("阅读");

    sourceButton.focus();
    expect(sourceButton).toHaveFocus();
    await user.click(sourceButton);

    expect(document.querySelector(".source-view")).not.toBeNull();
  });

  it("passes edited content to save callbacks", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    render(ContentEditor, {
      value: "old",
      originalValue: "old",
      editing: true,
      markdown: false,
      saveLabel: "Save",
      cancelLabel: "Cancel",
      onSave,
    });

    await replaceSourceEditorText(user, "new");
    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(onSave).toHaveBeenCalledWith("new");
  });

  it("saves dirty editor content with the platform save shortcut", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    render(ContentEditor, {
      value: "old",
      originalValue: "old",
      editing: true,
      markdown: false,
      onSave,
    });

    const editor = await replaceSourceEditorText(user, "shortcut");
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Save" })).not.toBeDisabled();
    });
    await pressPlatformShortcut(editor, "s", "KeyS");

    expect(onSave).toHaveBeenCalledWith("shortcut");
  });

  it("does not run save shortcut when content is unchanged", async () => {
    const onSave = vi.fn();
    render(ContentEditor, {
      value: "same",
      originalValue: "same",
      editing: true,
      markdown: false,
      onSave,
    });

    await pressPlatformShortcut(screen.getByRole("textbox"), "s", "KeyS");

    expect(onSave).not.toHaveBeenCalled();
  });

  it("does not render a local file-search bar when the platform find shortcut is pressed", async () => {
    render(ContentEditor, {
      value: "alpha\nbeta",
      originalValue: "alpha\nbeta",
      editing: true,
      markdown: false,
    });

    await pressPlatformShortcut(screen.getByRole("textbox"), "f", "KeyF");

    expect(screen.queryByRole("search")).not.toBeInTheDocument();
    expect(document.querySelector(".source-search-panel")).toBeNull();
    expect(document.querySelector(".cm-search")).toBeNull();
  });

  it("uses the app context menu to paste into the source editor selection", async () => {
    const user = userEvent.setup();
    stubClipboard({ readText: vi.fn().mockResolvedValue("new") });
    render(AppContextMenu);
    render(ContentEditor, {
      value: "old",
      originalValue: "old",
      editing: true,
      markdown: false,
      showFooter: false,
    });

    const editor = screen.getByRole("textbox");
    await user.click(editor);
    await user.keyboard("{Control>}a{/Control}");
    await fireEvent.contextMenu(editor, { clientX: 10, clientY: 10 });
    await user.click(await screen.findByRole("button", { name: "粘贴" }));

    await waitFor(() => {
      expect(editor).toHaveTextContent("new");
      expect(editor).not.toHaveTextContent("old");
    });
  });
});

describe("CompactTextEditor", () => {
  afterEach(() => {
    cleanup();
  });

  it("keeps inline config editing save and cancel actions", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    const onCancel = vi.fn();
    render(CompactTextEditor, {
      value: "one\ntwo",
      saveLabel: "Save",
      cancelLabel: "Cancel",
      onSave,
      onCancel,
    });

    const editor = screen.getByRole("textbox");
    await user.clear(editor);
    await user.type(editor, "three");
    await user.click(screen.getByRole("button", { name: "Save" }));
    await user.click(screen.getByRole("button", { name: "Cancel" }));

    expect(onSave).toHaveBeenCalledWith("three");
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});
