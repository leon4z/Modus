import { cleanup, fireEvent, render, screen, within } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import ModuleSearchResults from "$lib/shared/components/ModuleSearchResults.svelte";
import { locale } from "$lib/shared/i18n/index.js";
import { MODULE_SEARCH_GROUPS } from "$lib/shared/utils/moduleSearch.js";

describe("ModuleSearchResults", () => {
  afterEach(() => {
    cleanup();
    locale.set("zh");
  });

  it("renders grouped results and activates the selected item", async () => {
    locale.set("en");
    const onActivate = vi.fn();
    render(ModuleSearchResults, {
      query: "alpha",
      visible: true,
      groups: [
        {
          key: MODULE_SEARCH_GROUPS.LIST_ITEM,
          results: [
            {
              id: "item:alpha",
              group: MODULE_SEARCH_GROUPS.LIST_ITEM,
              query: "alpha",
              label: "Alpha Skill",
              detail: "Skill metadata",
              meta: "Shared",
            },
          ],
        },
      ],
      onActivate,
    });

    const results = screen.getByRole("region", { name: "Module search results" });
    expect(results).toHaveTextContent("1 matching results");
    expect(results).toHaveTextContent("Items");

    await fireEvent.click(within(results).getByRole("button", { name: /Alpha Skill/ }));

    expect(onActivate).toHaveBeenCalledWith(expect.objectContaining({ id: "item:alpha" }));
  });

  it("shows scoped empty state and dismisses on Escape", async () => {
    locale.set("en");
    const onDismiss = vi.fn();
    render(ModuleSearchResults, {
      query: "missing",
      visible: true,
      emptyLabel: "No matches in visible Skill list",
      groups: [{ key: MODULE_SEARCH_GROUPS.LIST_ITEM, results: [] }],
      onDismiss,
    });

    const results = screen.getByRole("region", { name: "Module search results" });
    expect(results).toHaveTextContent("No matches in visible Skill list");

    await fireEvent.keyDown(results, { key: "Escape" });

    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it("dismisses on outside click while keeping the search affordance interactive", async () => {
    locale.set("en");
    const onDismiss = vi.fn();
    const dismissRoot = document.createElement("div");
    const searchInput = document.createElement("input");
    dismissRoot.append(searchInput);
    document.body.append(dismissRoot);

    try {
      render(ModuleSearchResults, {
        query: "alpha",
        visible: true,
        dismissRoot,
        groups: [
          {
            key: MODULE_SEARCH_GROUPS.LIST_ITEM,
            results: [{ id: "item:alpha", group: MODULE_SEARCH_GROUPS.LIST_ITEM, query: "alpha", label: "Alpha" }],
          },
        ],
        onDismiss,
      });

      await fireEvent.pointerDown(searchInput);
      expect(onDismiss).not.toHaveBeenCalled();

      await fireEvent.pointerDown(document.body);
      expect(onDismiss).toHaveBeenCalledTimes(1);
    } finally {
      dismissRoot.remove();
    }
  });

  it("supports keyboard navigation and activation", async () => {
    locale.set("en");
    const onActivate = vi.fn();
    render(ModuleSearchResults, {
      query: "a",
      visible: true,
      groups: [
        {
          key: MODULE_SEARCH_GROUPS.LIST_ITEM,
          results: [
            { id: "item:alpha", group: MODULE_SEARCH_GROUPS.LIST_ITEM, query: "a", label: "Alpha" },
            { id: "item:beta", group: MODULE_SEARCH_GROUPS.LIST_ITEM, query: "a", label: "Beta" },
          ],
        },
      ],
      onActivate,
    });

    const results = screen.getByRole("region", { name: "Module search results" });
    await fireEvent.keyDown(results, { key: "ArrowDown" });
    await fireEvent.keyDown(results, { key: "Enter" });

    expect(onActivate).toHaveBeenCalledWith(expect.objectContaining({ id: "item:beta" }));
  });
});
