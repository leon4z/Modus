// Purpose: Keep the saved appearance preference synchronized with the page and native window.
import { get } from "svelte/store";

export const SYSTEM_COLOR_SCHEME_QUERY = "(prefers-color-scheme: dark)";
let themeSyncGeneration = 0;

function nextThemeSyncGeneration() {
  themeSyncGeneration += 1;
  return themeSyncGeneration;
}

/** @param {number} generation */
function isCurrentThemeSync(generation) {
  return generation === themeSyncGeneration;
}

/** @param {boolean} isDark @param {Document | undefined} doc */
function setDocumentDarkTheme(isDark, doc = globalThis.document) {
  if (!doc) return;
  if (isDark) {
    doc.documentElement.setAttribute("data-theme", "dark");
  } else {
    doc.documentElement.removeAttribute("data-theme");
  }
}

/** @param {Window | undefined} win */
function systemPrefersDark(win = globalThis.window) {
  return Boolean(win?.matchMedia?.(SYSTEM_COLOR_SCHEME_QUERY).matches);
}

/** @param {Window | undefined} win @param {Document | undefined} doc */
export function applySystemDocumentTheme(win = globalThis.window, doc = globalThis.document) {
  setDocumentDarkTheme(systemPrefersDark(win), doc);
}

/** @param {"light" | "dark" | null} nextTheme */
async function setNativeWindowTheme(nextTheme) {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().setTheme(nextTheme);
}

/**
 * @param {string} nextTheme
 * @param {{ win?: Window, doc?: Document, setNativeTheme?: (nextTheme: "light" | "dark" | null) => Promise<void> }} [options]
 */
export function applyThemePreference(nextTheme, options = {}) {
  const win = options.win ?? globalThis.window;
  const doc = options.doc ?? globalThis.document;
  const setNativeTheme = options.setNativeTheme ?? setNativeWindowTheme;
  const generation = nextThemeSyncGeneration();
  if (!doc) return;

  if (nextTheme === "dark") {
    setDocumentDarkTheme(true, doc);
    void setNativeTheme("dark").catch(() => {});
    return;
  }

  if (nextTheme === "light") {
    setDocumentDarkTheme(false, doc);
    void setNativeTheme("light").catch(() => {});
    return;
  }

  if (nextTheme === "system") {
    // In system mode, release any explicit native override and keep the page matched to the OS.
    applySystemDocumentTheme(win, doc);
    void setNativeTheme(null)
      .then(() => {
        if (isCurrentThemeSync(generation)) applySystemDocumentTheme(win, doc);
      })
      .catch(() => {});
    return;
  }

  setDocumentDarkTheme(false, doc);
  void setNativeTheme("light").catch(() => {});
}

/**
 * @param {import("svelte/store").Readable<string>} themeStore
 * @param {{ win?: Window, doc?: Document, setNativeTheme?: (nextTheme: "light" | "dark" | null) => Promise<void> }} [options]
 * @returns {() => void}
 */
export function watchSystemThemePreference(themeStore, options = {}) {
  const win = options.win ?? globalThis.window;
  const doc = options.doc ?? globalThis.document;
  const setNativeTheme = options.setNativeTheme ?? setNativeWindowTheme;
  const media = win?.matchMedia?.(SYSTEM_COLOR_SCHEME_QUERY);
  if (!media) return () => {};

  /** @param {MediaQueryListEvent | { matches: boolean }} event */
  const handleChange = (event) => {
    if (get(themeStore) !== "system") return;
    const generation = nextThemeSyncGeneration();
    setDocumentDarkTheme(event.matches, doc);
    void setNativeTheme(null)
      .then(() => {
        if (isCurrentThemeSync(generation) && get(themeStore) === "system") {
          applySystemDocumentTheme(win, doc);
        }
      })
      .catch(() => {});
  };

  if (typeof media.addEventListener === "function") {
    media.addEventListener("change", handleChange);
    return () => media.removeEventListener("change", handleChange);
  }

  media.addListener(handleChange);
  return () => media.removeListener(handleChange);
}
