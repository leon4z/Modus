// Purpose: Share Modus app update state between the app shell, sidebar, and Settings.
import { derived, get, writable } from "svelte/store";
import {
  checkAppUpdate,
  getAppUpdateState,
  installAppUpdate,
  restartAppForUpdate,
  skipAppUpdate,
} from "$lib/features/appUpdates/api/appUpdates.js";

const DEFAULT_STATE = {
  status: "idle",
  channel: "stable",
  canCheck: true,
  source: null,
  currentVersion: "0.1.0",
  availableUpdate: null,
  lastStartupCheckAt: null,
  lastSuccessfulCheckAt: null,
  lastFailureAt: null,
  lastFailureSummary: null,
};

const LOCAL_TRANSIENT_STATUSES = new Set(["downloading", "installing"]);
const NON_ACTIONABLE_AVAILABLE_STATUSES = new Set(["downloading", "installing", "restart_needed", "skipped"]);

/** @param {any} raw */
export function normalizeAppUpdateState(raw) {
  const input = raw && typeof raw === "object" ? raw : {};
  return {
    ...DEFAULT_STATE,
    ...input,
    status: String(input.status || DEFAULT_STATE.status),
    channel: String(input.channel || DEFAULT_STATE.channel),
    canCheck: input.canCheck !== false,
    availableUpdate: input.availableUpdate || null,
    lastFailureSummary: input.lastFailureSummary || null,
  };
}

export const appUpdateState = writable(normalizeAppUpdateState(null));

export const appUpdateAvailable = derived(
  appUpdateState,
  ($state) => Boolean($state?.availableUpdate?.version) && !NON_ACTIONABLE_AVAILABLE_STATUSES.has($state.status)
);

/** @param {unknown} error */
function errorMessage(error) {
  if (error && typeof error === "object" && "message" in error) {
    return String(error.message);
  }
  return String(error);
}

export async function loadAppUpdateState() {
  const next = normalizeAppUpdateState(await getAppUpdateState());
  const current = get(appUpdateState);
  if (shouldKeepLocalAppUpdateState(current, next)) {
    return current;
  }
  appUpdateState.set(next);
  return next;
}

/** @param {any} current @param {any} next */
function shouldKeepLocalAppUpdateState(current, next) {
  if (LOCAL_TRANSIENT_STATUSES.has(current?.status)) return true;
  return current?.status === "restart_needed";
}

/**
 * @param {"startup" | "manual"} reason
 */
export async function runAppUpdateCheck(reason) {
  const current = get(appUpdateState);
  const shouldRestoreRestartNeeded = current.status === "restart_needed";
  appUpdateState.set({ ...current, status: "checking", lastFailureSummary: null });
  try {
    const next = normalizeAppUpdateState(await checkAppUpdate(reason));
    const resolved = shouldRestoreRestartNeeded ? restoreRestartNeededState(current, next) : next;
    appUpdateState.set(resolved);
    return resolved;
  } catch (error) {
    if (shouldRestoreRestartNeeded) {
      const restored = {
        ...current,
        status: "restart_needed",
        lastFailureSummary: errorMessage(error),
      };
      appUpdateState.set(restored);
      return restored;
    }
    const failed = {
      ...get(appUpdateState),
      status: "failed",
      lastFailureSummary: errorMessage(error),
    };
    appUpdateState.set(failed);
    return failed;
  }
}

/** @param {any} current @param {any} next */
function restoreRestartNeededState(current, next) {
  return {
    ...next,
    status: "restart_needed",
    availableUpdate: current.availableUpdate || next.availableUpdate || null,
    lastFailureSummary: next.lastFailureSummary || null,
  };
}

export async function installAvailableAppUpdate() {
  const current = get(appUpdateState);
  appUpdateState.set({ ...current, status: "downloading", lastFailureSummary: null });
  const installMarker = setTimeout(() => {
    const state = get(appUpdateState);
    if (state.status === "downloading") {
      appUpdateState.set({ ...state, status: "installing" });
    }
  }, 250);
  try {
    const next = normalizeAppUpdateState(await installAppUpdate());
    appUpdateState.set(next);
    return next;
  } catch (error) {
    const failed = {
      ...get(appUpdateState),
      status: "failed",
      lastFailureSummary: errorMessage(error),
    };
    appUpdateState.set(failed);
    return failed;
  } finally {
    clearTimeout(installMarker);
  }
}

export async function skipAvailableAppUpdate() {
  const current = get(appUpdateState);
  if (!current.availableUpdate) return current;
  try {
    const next = normalizeAppUpdateState(await skipAppUpdate());
    appUpdateState.set(next);
    return next;
  } catch (error) {
    const failed = {
      ...get(appUpdateState),
      status: "failed",
      lastFailureSummary: errorMessage(error),
    };
    appUpdateState.set(failed);
    return failed;
  }
}

export async function restartIntoAppUpdate() {
  await restartAppForUpdate();
}
