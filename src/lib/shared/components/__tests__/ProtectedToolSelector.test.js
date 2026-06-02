import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("$lib/shared/components/ToolIcon.svelte", async () => {
  const mod = await import("../../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});

import ProtectedToolSelector from "$lib/shared/components/ProtectedToolSelector.svelte";

const tools = [
  { id: "openclaw", name: "OpenClaw" },
  { id: "claude-code", name: "Claude Code" },
  { id: "codex", name: "Codex" },
];

/** @type {undefined | (() => void)} */
let resizeCallback;
/** @type {Element[]} */
let observedElements = [];

describe("ProtectedToolSelector", () => {
  beforeEach(() => {
    resizeCallback = undefined;
    observedElements = [];
    vi.stubGlobal("ResizeObserver", class {
      /** @param {() => void} callback */
      constructor(callback) {
        resizeCallback = callback;
      }
      /** @param {Element} element */
      observe(element) {
        observedElements.push(element);
      }
      disconnect() {}
    });
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
      callback(0);
      return 1;
    });
    vi.spyOn(window, "cancelAnimationFrame").mockImplementation(() => {});
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("renders full-label tool segments with stable selected state", async () => {
    const onSelect = vi.fn();
    const user = userEvent.setup();
    const { container } = render(ProtectedToolSelector, {
      props: {
        tools,
        activeToolId: "codex",
        ariaLabel: "Tools",
        onSelect,
      },
    });

    const group = screen.getByRole("group", { name: "Tools" });
    expect(group).toHaveAttribute("data-mode", "full");
    expect(observedElements).toContain(group);
    expect(observedElements).toContain(group.parentElement);
    expect(group).toHaveStyle({ minWidth: "116px" });
    expect(screen.getByRole("button", { name: "Codex" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "Claude Code" })).not.toHaveAttribute("title");
    expect(document.querySelector('[role="tooltip"]')).toBeNull();
    const measure = /** @type {HTMLElement} */ (container.querySelector(".tool-selector-capsule__measure"));
    expect(measure).toHaveAttribute("aria-hidden", "true");
    expect(getComputedStyle(measure).position).toBe("fixed");
    expect(getComputedStyle(measure).visibility).toBe("hidden");

    await user.click(screen.getByRole("button", { name: "OpenClaw" }));
    expect(onSelect).toHaveBeenCalledWith("openclaw");
  });

  it("collapses the whole group to icon-only mode when labels do not fit", async () => {
    const { container } = render(ProtectedToolSelector, {
      props: {
        tools,
        activeToolId: "codex",
        ariaLabel: "Tools",
      },
    });

    const group = screen.getByRole("group", { name: "Tools" });
    const measure = /** @type {HTMLElement} */ (container.querySelector(".tool-selector-capsule__measure"));
    Object.defineProperty(group, "clientWidth", { configurable: true, value: 120 });
    Object.defineProperty(measure, "scrollWidth", { configurable: true, value: 320 });

    resizeCallback?.();
    await waitFor(() => expect(group).toHaveAttribute("data-mode", "icon"));

    const claudeButton = screen.getByRole("button", { name: "Claude Code" });
    expect(claudeButton).not.toHaveAttribute("title");
    expect([...document.querySelectorAll('[role="tooltip"]')].map((node) => node.textContent)).toContain("Claude Code");
    expect(claudeButton).not.toHaveTextContent("Claude Code");

    Object.defineProperty(group, "clientWidth", { configurable: true, value: 335 });
    resizeCallback?.();
    await waitFor(() => expect(group).toHaveAttribute("data-mode", "icon"));

    Object.defineProperty(group, "clientWidth", { configurable: true, value: 336 });
    resizeCallback?.();
    await waitFor(() => expect(group).toHaveAttribute("data-mode", "full"));
    expect(screen.getByRole("button", { name: "Claude Code" })).toHaveTextContent("Claude Code");
  });

  it("uses parent content width rather than toolbar padding when resolving fit", async () => {
    const { container } = render(ProtectedToolSelector, {
      props: {
        tools,
        activeToolId: "codex",
        ariaLabel: "Tools",
      },
    });

    const group = screen.getByRole("group", { name: "Tools" });
    const parent = /** @type {HTMLElement} */ (group.parentElement);
    const measure = /** @type {HTMLElement} */ (container.querySelector(".tool-selector-capsule__measure"));
    parent.style.paddingLeft = "20px";
    parent.style.paddingRight = "20px";
    Object.defineProperty(parent, "clientWidth", { configurable: true, value: 340 });
    Object.defineProperty(measure, "scrollWidth", { configurable: true, value: 320 });

    resizeCallback?.();
    await waitFor(() => expect(group).toHaveAttribute("data-mode", "icon"));

    Object.defineProperty(parent, "clientWidth", { configurable: true, value: 376 });
    resizeCallback?.();
    await waitFor(() => expect(group).toHaveAttribute("data-mode", "full"));
  });

  it("keeps icon-only buttons focusable and named after resize fallback", async () => {
    vi.unstubAllGlobals();
    const { container } = render(ProtectedToolSelector, {
      props: {
        tools,
        activeToolId: "openclaw",
        ariaLabel: "Tools",
      },
    });

    const group = screen.getByRole("group", { name: "Tools" });
    const measure = /** @type {HTMLElement} */ (container.querySelector(".tool-selector-capsule__measure"));
    Object.defineProperty(group, "clientWidth", { configurable: true, value: 100 });
    Object.defineProperty(measure, "scrollWidth", { configurable: true, value: 260 });

    await fireEvent(window, new Event("resize"));
    await waitFor(() => expect(group).toHaveAttribute("data-mode", "icon"));

    const activeButton = screen.getByRole("button", { name: "OpenClaw" });
    activeButton.focus();
    expect(activeButton).toHaveFocus();
    expect(activeButton).toHaveAttribute("aria-pressed", "true");
  });
});
