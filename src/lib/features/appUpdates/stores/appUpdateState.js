// Purpose: Share Modus app update state between the app shell, sidebar, and Settings.
import { derived, get, writable } from "svelte/store";
import {
  checkAppUpdate,
  getAppUpdateState,
  installAppUpdate,
  restartAppForUpdate,
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
  ($state) => Boolean($state?.availableUpdate?.version)
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
  appUpdateState.set(next);
  return next;
}

/**
 * @param {"startup" | "manual"} reason
 */
export async function runAppUpdateCheck(reason) {
  const current = get(appUpdateState);
  appUpdateState.set({ ...current, status: "checking", lastFailureSummary: null });
  try {
    const next = normalizeAppUpdateState(await checkAppUpdate(reason));
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

export function postponeAvailableAppUpdate() {
  const current = get(appUpdateState);
  if (!current.availableUpdate) return current;
  const next = { ...current, status: "postponed" };
  appUpdateState.set(next);
  return next;
}

export async function restartIntoAppUpdate() {
  await restartAppForUpdate();
}
