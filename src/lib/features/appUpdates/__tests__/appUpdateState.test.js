// @ts-nocheck
import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";

const apiMocks = vi.hoisted(() => ({
  checkAppUpdate: vi.fn(),
  getAppUpdateState: vi.fn(),
  installAppUpdate: vi.fn(),
  restartAppForUpdate: vi.fn(),
  skipAppUpdate: vi.fn(),
}));

vi.mock("$lib/features/appUpdates/api/appUpdates.js", () => apiMocks);

import {
  appUpdateAvailable,
  appUpdateState,
  installAvailableAppUpdate,
  loadAppUpdateState,
  normalizeAppUpdateState,
  runAppUpdateCheck,
} from "$lib/features/appUpdates/stores/appUpdateState.js";

function availableUpdate(version = "0.2.0") {
  return {
    version,
    currentVersion: "0.1.0",
    channel: "stable",
    date: null,
    body: null,
  };
}

function state(overrides = {}) {
  return normalizeAppUpdateState({
    status: "idle",
    channel: "stable",
    canCheck: true,
    currentVersion: "0.1.0",
    availableUpdate: null,
    ...overrides,
  });
}

describe("app update state store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    appUpdateState.set(state());
  });

  it("does not let persisted available state overwrite an in-progress install", async () => {
    appUpdateState.set(state({
      status: "installing",
      availableUpdate: availableUpdate("0.2.0"),
    }));
    apiMocks.getAppUpdateState.mockResolvedValue(state({
      status: "idle",
      availableUpdate: availableUpdate("0.2.0"),
    }));

    const loaded = await loadAppUpdateState();

    expect(loaded.status).toBe("installing");
    expect(get(appUpdateState).status).toBe("installing");
    expect(get(appUpdateAvailable)).toBe(false);
  });

  it("keeps restart-needed state when persisted state still reports the installed update", async () => {
    appUpdateState.set(state({
      status: "restart_needed",
      availableUpdate: availableUpdate("0.2.0"),
    }));
    apiMocks.getAppUpdateState.mockResolvedValue(state({
      status: "idle",
      availableUpdate: availableUpdate("0.2.0"),
    }));

    const loaded = await loadAppUpdateState();

    expect(loaded.status).toBe("restart_needed");
    expect(get(appUpdateState).status).toBe("restart_needed");
    expect(get(appUpdateAvailable)).toBe(false);
  });

  it("keeps restart-needed state when persisted state reports no available update", async () => {
    appUpdateState.set(state({
      status: "restart_needed",
      availableUpdate: availableUpdate("0.2.0"),
    }));
    apiMocks.getAppUpdateState.mockResolvedValue(state({
      status: "current",
      availableUpdate: null,
    }));

    const loaded = await loadAppUpdateState();

    expect(loaded.status).toBe("restart_needed");
    expect(get(appUpdateState).status).toBe("restart_needed");
    expect(get(appUpdateAvailable)).toBe(false);
  });

  it("hides the sidebar update badge after install resolves to restart needed", async () => {
    appUpdateState.set(state({
      status: "available",
      availableUpdate: availableUpdate("0.2.0"),
    }));
    apiMocks.installAppUpdate.mockResolvedValue(state({
      status: "restart_needed",
      availableUpdate: availableUpdate("0.2.0"),
    }));

    await installAvailableAppUpdate();

    expect(get(appUpdateState).status).toBe("restart_needed");
    expect(get(appUpdateAvailable)).toBe(false);
  });

  it("keeps restart-needed state across a manual recheck until the app restarts", async () => {
    appUpdateState.set(state({
      status: "restart_needed",
      availableUpdate: availableUpdate("0.2.0"),
    }));
    apiMocks.checkAppUpdate.mockResolvedValue(state({
      status: "available",
      availableUpdate: availableUpdate("0.2.0"),
    }));

    await runAppUpdateCheck("manual");

    expect(get(appUpdateState).status).toBe("restart_needed");
    expect(get(appUpdateAvailable)).toBe(false);
  });

  it("keeps restart-needed state when a manual recheck returns current", async () => {
    appUpdateState.set(state({
      status: "restart_needed",
      availableUpdate: availableUpdate("0.2.0"),
    }));
    apiMocks.checkAppUpdate.mockResolvedValue(state({
      status: "current",
      availableUpdate: null,
    }));

    await runAppUpdateCheck("manual");

    const current = get(appUpdateState);
    expect(current.status).toBe("restart_needed");
    expect(current.availableUpdate.version).toBe("0.2.0");
    expect(get(appUpdateAvailable)).toBe(false);
  });

  it("keeps restart-needed state when a manual recheck fails", async () => {
    appUpdateState.set(state({
      status: "restart_needed",
      availableUpdate: availableUpdate("0.2.0"),
    }));
    apiMocks.checkAppUpdate.mockRejectedValue(new Error("network failed"));

    await runAppUpdateCheck("manual");

    const current = get(appUpdateState);
    expect(current.status).toBe("restart_needed");
    expect(current.lastFailureSummary).toBe("network failed");
    expect(get(appUpdateAvailable)).toBe(false);
  });
});
