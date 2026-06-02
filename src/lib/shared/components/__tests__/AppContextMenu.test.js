// @ts-nocheck
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import AppContextMenu from "$lib/shared/components/AppContextMenu.svelte";
import { locale } from "$lib/shared/i18n/index.js";

function dispatchContextMenu(target, options = {}) {
  const event = new MouseEvent("contextmenu", {
    bubbles: true,
    cancelable: true,
    clientX: options.clientX ?? 10,
    clientY: options.clientY ?? 10,
  });
  target.dispatchEvent(event);
  return event;
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

function stubSelection({ text = "selected", containsPoint = true } = {}) {
  vi.spyOn(window, "getSelection").mockReturnValue({
    isCollapsed: false,
    rangeCount: 1,
    toString: () => text,
    getRangeAt: () => ({
      collapsed: false,
      getClientRects: () => [
        containsPoint
          ? { left: 0, top: 0, right: 100, bottom: 30 }
          : { left: 200, top: 200, right: 260, bottom: 230 },
      ],
    }),
  });
}

describe("AppContextMenu", () => {
  afterEach(() => {
    cleanup();
    document.body.innerHTML = "";
    vi.restoreAllMocks();
  });

  it("suppresses native menus on blank space without showing an empty menu", () => {
    locale.set("en");
    render(AppContextMenu);

    const event = dispatchContextMenu(document.body);

    expect(event.defaultPrevented).toBe(true);
    expect(screen.queryByRole("button", { name: "Copy" })).not.toBeInTheDocument();
  });

  it("copies selected text through an app-owned menu", async () => {
    const user = userEvent.setup();
    locale.set("en");
    const clipboard = stubClipboard();
    stubSelection({ text: "selected text" });
    render(AppContextMenu);
    const paragraph = document.createElement("p");
    paragraph.textContent = "selected text";
    document.body.appendChild(paragraph);

    const event = dispatchContextMenu(paragraph);

    expect(event.defaultPrevented).toBe(true);
    await user.click(await screen.findByRole("button", { name: "Copy" }));

    expect(clipboard.writeText).toHaveBeenCalledWith("selected text");
    await waitFor(() => {
      expect(screen.queryByRole("button", { name: "Copy" })).not.toBeInTheDocument();
    });
  });

  it("does not show copy when the click is outside the selected range", () => {
    locale.set("en");
    stubSelection({ containsPoint: false });
    render(AppContextMenu);
    const paragraph = document.createElement("p");
    paragraph.textContent = "selected text";
    document.body.appendChild(paragraph);

    dispatchContextMenu(paragraph, { clientX: 10, clientY: 10 });

    expect(screen.queryByRole("button", { name: "Copy" })).not.toBeInTheDocument();
  });

  it("pastes into editable inputs through an app-owned menu", async () => {
    const user = userEvent.setup();
    locale.set("en");
    stubClipboard({ readText: vi.fn().mockResolvedValue("xy") });
    render(AppContextMenu);
    const input = document.createElement("input");
    input.type = "text";
    input.value = "ab";
    document.body.appendChild(input);
    input.focus();
    input.setSelectionRange(1, 1);

    dispatchContextMenu(input);
    await user.click(await screen.findByRole("button", { name: "Paste" }));

    expect(input.value).toBe("axyb");
  });

  it("offers copy and paste for selected editable input text", async () => {
    const user = userEvent.setup();
    locale.set("en");
    const clipboard = stubClipboard();
    render(AppContextMenu);
    const input = document.createElement("input");
    input.type = "text";
    input.value = "abcd";
    vi.spyOn(input, "getBoundingClientRect").mockReturnValue({
      left: 0,
      right: 80,
      top: 0,
      bottom: 24,
      width: 80,
      height: 24,
      x: 0,
      y: 0,
      toJSON: () => {},
    });
    document.body.appendChild(input);
    input.focus();
    input.setSelectionRange(1, 3);

    dispatchContextMenu(input, { clientX: 40, clientY: 12 });
    expect(await screen.findByRole("button", { name: "Copy" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Paste" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Copy" }));

    expect(clipboard.writeText).toHaveBeenCalledWith("bc");
  });

  it("does not offer input copy when the right-click is outside the editable selection", async () => {
    locale.set("en");
    render(AppContextMenu);
    const input = document.createElement("input");
    input.type = "text";
    input.value = "abcd";
    vi.spyOn(input, "getBoundingClientRect").mockReturnValue({
      left: 0,
      right: 80,
      top: 0,
      bottom: 24,
      width: 80,
      height: 24,
      x: 0,
      y: 0,
      toJSON: () => {},
    });
    document.body.appendChild(input);
    input.focus();
    input.setSelectionRange(1, 2);

    dispatchContextMenu(input, { clientX: 70, clientY: 12 });

    expect(screen.queryByRole("button", { name: "Copy" })).not.toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "Paste" })).toBeInTheDocument();
  });

  it("does not offer textarea copy when a multiline selection is elsewhere", async () => {
    locale.set("en");
    render(AppContextMenu);
    const textarea = document.createElement("textarea");
    textarea.value = "abcd\nefgh";
    vi.spyOn(textarea, "getBoundingClientRect").mockReturnValue({
      left: 0,
      right: 80,
      top: 0,
      bottom: 40,
      width: 80,
      height: 40,
      x: 0,
      y: 0,
      toJSON: () => {},
    });
    textarea.style.padding = "0";
    textarea.style.fontSize = "10px";
    textarea.style.lineHeight = "20px";
    document.body.appendChild(textarea);
    textarea.focus();
    textarea.setSelectionRange(0, 2);

    dispatchContextMenu(textarea, { clientX: 10, clientY: 30 });

    expect(screen.queryByRole("button", { name: "Copy" })).not.toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "Paste" })).toBeInTheDocument();
  });

  it("keeps the app menu open with a controlled failure when copy is unavailable", async () => {
    const user = userEvent.setup();
    locale.set("en");
    stubClipboard({ writeText: vi.fn().mockRejectedValue(new Error("denied")) });
    stubSelection({ text: "selected text" });
    render(AppContextMenu);
    const paragraph = document.createElement("p");
    paragraph.textContent = "selected text";
    document.body.appendChild(paragraph);

    dispatchContextMenu(paragraph);
    await user.click(await screen.findByRole("button", { name: "Copy" }));

    expect(await screen.findByText("Copy failed")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Copy" })).toBeInTheDocument();
  });

  it("does not replace a handled product menu with generic copy", () => {
    locale.set("en");
    stubSelection({ text: "row label" });
    render(AppContextMenu);
    const productRow = document.createElement("button");
    productRow.textContent = "row label";
    productRow.addEventListener("contextmenu", (event) => {
      event.preventDefault();
      event.stopPropagation();
    });
    document.body.appendChild(productRow);

    dispatchContextMenu(productRow);

    expect(screen.queryByRole("button", { name: "Copy" })).not.toBeInTheDocument();
  });
});
