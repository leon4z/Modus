import { afterEach, describe, expect, it, vi } from "vitest";
import { writable } from "svelte/store";
import {
  applyThemePreference,
  watchSystemThemePreference,
  SYSTEM_COLOR_SCHEME_QUERY,
} from "$lib/app/themePreference.js";

function createSystemThemeWindow(matches = false) {
  /** @type {null | ((event: { matches: boolean }) => void)} */
  let changeHandler = null;
  const media = {
    matches,
    addEventListener: vi.fn((event, handler) => {
      if (event === "change") changeHandler = handler;
    }),
    removeEventListener: vi.fn((event, handler) => {
      if (event === "change" && changeHandler === handler) changeHandler = null;
    }),
    /** @param {boolean} nextMatches */
    dispatch(nextMatches) {
      media.matches = nextMatches;
      changeHandler?.({ matches: nextMatches });
    },
  };
  const win = /** @type {Window} */ (/** @type {unknown} */ ({
    matchMedia: vi.fn(
      /** @param {string} query */
      (query) => {
        expect(query).toBe(SYSTEM_COLOR_SCHEME_QUERY);
        return /** @type {MediaQueryList} */ (/** @type {unknown} */ (media));
      }
    ),
  }));
  return { win, media };
}

afterEach(() => {
  document.documentElement.removeAttribute("data-theme");
  vi.restoreAllMocks();
});

describe("theme preference synchronization", () => {
  function createDeferredNativeTheme() {
    /** @type {Array<() => void>} */
    const resolvers = [];
    const setNativeTheme = vi.fn(() => new Promise((resolve) => {
      resolvers.push(() => resolve(undefined));
    }));
    return {
      setNativeTheme,
      resolveNext() {
        const resolve = resolvers.shift();
        resolve?.();
      },
    };
  }

  it("releases native theme override when system preference is selected", () => {
    const { win } = createSystemThemeWindow(false);
    const setNativeTheme = vi.fn().mockResolvedValue(undefined);
    document.documentElement.setAttribute("data-theme", "dark");

    applyThemePreference("system", { win, doc: document, setNativeTheme });

    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
    expect(setNativeTheme).toHaveBeenCalledWith(null);
    expect(setNativeTheme).not.toHaveBeenCalledWith("light");
    expect(setNativeTheme).not.toHaveBeenCalledWith("dark");
  });

  it("keeps system mode following automatic changes back to light", () => {
    const { win, media } = createSystemThemeWindow(true);
    const setNativeTheme = vi.fn().mockResolvedValue(undefined);
    const theme = writable("system");
    document.documentElement.setAttribute("data-theme", "dark");

    const stop = watchSystemThemePreference(theme, { win, doc: document, setNativeTheme });
    media.dispatch(false);

    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
    expect(setNativeTheme).toHaveBeenCalledWith(null);
    expect(setNativeTheme).not.toHaveBeenCalledWith("light");
    expect(setNativeTheme).not.toHaveBeenCalledWith("dark");

    stop();
    expect(media.removeEventListener).toHaveBeenCalled();
  });

  it("does not let a stale system apply override a later explicit dark preference", async () => {
    const { win } = createSystemThemeWindow(false);
    const nativeTheme = createDeferredNativeTheme();

    applyThemePreference("system", { win, doc: document, setNativeTheme: nativeTheme.setNativeTheme });
    applyThemePreference("dark", { win, doc: document, setNativeTheme: nativeTheme.setNativeTheme });

    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    nativeTheme.resolveNext();
    await Promise.resolve();

    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    expect(nativeTheme.setNativeTheme).toHaveBeenNthCalledWith(1, null);
    expect(nativeTheme.setNativeTheme).toHaveBeenNthCalledWith(2, "dark");
  });

  it("ignores system changes when explicit dark mode is selected", () => {
    const { win, media } = createSystemThemeWindow(true);
    const setNativeTheme = vi.fn().mockResolvedValue(undefined);
    const theme = writable("dark");
    document.documentElement.setAttribute("data-theme", "dark");

    const stop = watchSystemThemePreference(theme, { win, doc: document, setNativeTheme });
    media.dispatch(false);

    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    expect(setNativeTheme).not.toHaveBeenCalled();

    stop();
  });
});
