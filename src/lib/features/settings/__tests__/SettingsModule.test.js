// @ts-nocheck
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/svelte";
import { get } from "svelte/store";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const loggerMocks = vi.hoisted(() => ({
  logAppEvent: vi.fn(),
}));

const dialogMocks = vi.hoisted(() => ({
  open: vi.fn(),
  save: vi.fn(),
}));

const openerMocks = vi.hoisted(() => ({
  openUrl: vi.fn(),
}));

const skillInventoryMocks = vi.hoisted(() => ({
  invalidateSkillInventory: vi.fn(),
}));

const appUpdateMocks = vi.hoisted(() => ({
  installAvailableAppUpdate: vi.fn(),
  loadAppUpdateState: vi.fn(),
  restartIntoAppUpdate: vi.fn(),
  runAppUpdateCheck: vi.fn(),
  skipAvailableAppUpdate: vi.fn(),
}));

const apiMocks = vi.hoisted(() => ({
  addCustomTool: vi.fn(),
  exportApplicationLogs: vi.fn(),
  exportModulePerformanceLogs: vi.fn(),
  getCustomTools: vi.fn(),
  getInjectionTargets: vi.fn(),
  getManagedTools: vi.fn(),
  getModulePerformanceDiagnosticsEnabled: vi.fn(),
  getRuntimeInfo: vi.fn(),
  getSkillPerformanceDiagnosticsEnabled: vi.fn(),
  getToolCapabilityOverrides: vi.fn(),
  getToolPaths: vi.fn(),
  listApplicationLogs: vi.fn(),
  listModulePerformanceLogs: vi.fn(),
  readApplicationLog: vi.fn(),
  readModulePerformanceLog: vi.fn(),
  removeCustomTool: vi.fn(),
  setInjectionTarget: vi.fn(),
  setLanguage: vi.fn(),
  setManagedTools: vi.fn(),
  setModulePerformanceDiagnosticsEnabled: vi.fn(),
  setSkillPerformanceDiagnosticsEnabled: vi.fn(),
  setTheme: vi.fn(),
  setToolCapabilityOverrides: vi.fn(),
  setToolPath: vi.fn(),
  writeModulePerformanceLog: vi.fn(),
}));

vi.mock("$lib/shared/logging/api.js", () => apiMocks);
vi.mock("$lib/features/settings/api/settings.js", () => apiMocks);
vi.mock("$lib/shared/logging/appLogger.js", () => loggerMocks);
vi.mock("@tauri-apps/plugin-dialog", () => dialogMocks);
vi.mock("@tauri-apps/plugin-opener", () => openerMocks);
vi.mock("$lib/features/skills/index.js", () => skillInventoryMocks);
vi.mock("$lib/features/appUpdates/index.js", async () => {
  const { derived, get, writable } = await import("svelte/store");
  const defaultState = {
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
  const appUpdateState = writable(defaultState);
  const appUpdateAvailable = derived(
    appUpdateState,
    ($state) => Boolean($state?.availableUpdate?.version),
  );
  appUpdateMocks.loadAppUpdateState.mockImplementation(async () => get(appUpdateState));
  appUpdateMocks.runAppUpdateCheck.mockImplementation(async () => {
    const next = { ...get(appUpdateState), status: "current", availableUpdate: null };
    appUpdateState.set(next);
    return next;
  });
  appUpdateMocks.installAvailableAppUpdate.mockImplementation(async () => {
    const next = { ...get(appUpdateState), status: "restart_needed" };
    appUpdateState.set(next);
    return next;
  });
  appUpdateMocks.skipAvailableAppUpdate.mockImplementation(async () => {
    const next = { ...get(appUpdateState), status: "skipped", availableUpdate: null };
    appUpdateState.set(next);
    return next;
  });
  appUpdateMocks.restartIntoAppUpdate.mockResolvedValue(undefined);

  return {
    appUpdateAvailable,
    appUpdateState,
    installAvailableAppUpdate: appUpdateMocks.installAvailableAppUpdate,
    loadAppUpdateState: appUpdateMocks.loadAppUpdateState,
    restartIntoAppUpdate: appUpdateMocks.restartIntoAppUpdate,
    runAppUpdateCheck: appUpdateMocks.runAppUpdateCheck,
    skipAvailableAppUpdate: appUpdateMocks.skipAvailableAppUpdate,
  };
});
vi.mock("$lib/features/tools/index.js", async (importOriginal) => {
  const { derived, writable } = await import("svelte/store");
  const toolFeature = await importOriginal();
  const tools = writable([]);
  const managedToolIds = writable([]);
  const detectedTools = derived(tools, ($tools) => $tools.filter((tool) => tool.detected));
  const managedTools = derived(
    [detectedTools, managedToolIds],
    ([$detectedTools, $managedToolIds]) => $detectedTools.filter((tool) => $managedToolIds.includes(tool.id))
  );
  const theme = writable("system");
  const loadTools = vi.fn();

  return {
    ...toolFeature,
    ...apiMocks,
    tools,
    detectedTools,
    managedToolIds,
    managedTools,
    theme,
    loadTools,
    getToolName(toolId, toolsList = []) {
      const tool = toolsList.find((entry) => entry.id === toolId);
      return tool ? tool.name : toolId;
    },
  };
});
vi.mock("$lib/shared/components/ToolIcon.svelte", async () => {
  const mod = await import("../../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});
import SettingsModule from "$lib/features/settings/components/SettingsModule.svelte";
import * as toolStores from "$lib/features/tools/index.js";
import { appUpdateState } from "$lib/features/appUpdates/index.js";
import { activeSettingsTab, focusedSettingsToolId } from "$lib/features/settings/stores/settingsPage.js";
import { modulePerformanceSummaries, setModulePerformanceDiagnosticsEnabledState } from "$lib/shared/diagnostics/modulePerformance.js";
import { languagePreference, locale } from "$lib/shared/i18n/index.js";

describe("SettingsModule", () => {
  function defaultAppUpdateState(overrides = {}) {
    return {
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
      ...overrides,
    };
  }

  function settingsToolRow(toolId) {
    return /** @type {HTMLElement} */ (document.querySelector(`[data-settings-tool-id="${toolId}"]`));
  }

  async function expandSettingsToolRow(toolId) {
    await waitFor(() => {
      expect(settingsToolRow(toolId)).toBeTruthy();
    });
    const row = settingsToolRow(toolId);
    const expandButton = within(row).queryByRole("button", { name: "Expand settings" });
    if (expandButton) await fireEvent.click(expandButton);
    return row;
  }

  async function editSettingsToolRow(toolId) {
    await waitFor(() => {
      expect(settingsToolRow(toolId)).toBeTruthy();
    });
    const row = settingsToolRow(toolId);
    await fireEvent.click(within(row).getByRole("button", { name: "Edit" }));
    return row;
  }

  beforeEach(() => {
    locale.set("en");
    languagePreference.set("en");
    activeSettingsTab.set("general");
    focusedSettingsToolId.set(null);
    toolStores.tools.set([
      { id: "cursor", name: "Cursor", detected: true, config_dir: "/cursor/config", skills_dir: "/cursor/skills" },
      { id: "codex", name: "Codex", detected: true, config_dir: "/codex/config", skills_dir: "/codex/skills" },
    ]);
    toolStores.managedToolIds.set(["cursor", "codex"]);
    toolStores.theme.set("system");
    toolStores.loadTools.mockResolvedValue([]);
    apiMocks.getToolPaths.mockResolvedValue({
      cursor: { config_dir: "/cursor/config", skills_dir: "/cursor/skills" },
      codex: { config_dir: "/codex/config", skills_dir: "/codex/skills" },
    });
    apiMocks.getInjectionTargets.mockResolvedValue({
      cursor: "/cursor/rules.md",
      codex: "/codex/rules.md",
    });
    apiMocks.getToolCapabilityOverrides.mockResolvedValue({});
    apiMocks.getCustomTools.mockResolvedValue([]);
    apiMocks.getManagedTools.mockResolvedValue(["cursor", "codex"]);
    apiMocks.getModulePerformanceDiagnosticsEnabled.mockResolvedValue(false);
    apiMocks.getRuntimeInfo.mockResolvedValue({
      shape: "release",
      updateChannel: "stable",
      canCheckForUpdates: true,
      usesSandboxTools: false,
      usesRealTools: true,
    });
    apiMocks.getSkillPerformanceDiagnosticsEnabled.mockResolvedValue(false);
    apiMocks.setManagedTools.mockResolvedValue(undefined);
    apiMocks.setModulePerformanceDiagnosticsEnabled.mockResolvedValue(undefined);
    apiMocks.setSkillPerformanceDiagnosticsEnabled.mockResolvedValue(undefined);
    apiMocks.setToolPath.mockResolvedValue(undefined);
    apiMocks.setInjectionTarget.mockResolvedValue(undefined);
    apiMocks.setToolCapabilityOverrides.mockResolvedValue(undefined);
    apiMocks.setTheme.mockResolvedValue(undefined);
    apiMocks.setLanguage.mockResolvedValue(undefined);
    apiMocks.listApplicationLogs.mockResolvedValue([{ id: "app-2026-05-27.log", label: "2026-05-27" }]);
    apiMocks.readApplicationLog.mockResolvedValue({ id: "app-2026-05-27.log", label: "2026-05-27", content: "{\"level\":\"info\"}\n", truncated: false });
    apiMocks.listModulePerformanceLogs.mockResolvedValue([{ id: "module-performance-2026-05-27.log", label: "2026-05-27" }]);
    apiMocks.readModulePerformanceLog.mockResolvedValue({ id: "module-performance-2026-05-27.log", label: "2026-05-27", content: "{\"module\":\"settings\"}\n", truncated: false });
    apiMocks.exportApplicationLogs.mockResolvedValue("/tmp/app-log.txt");
    apiMocks.exportModulePerformanceLogs.mockResolvedValue("/tmp/perf-log.txt");
    apiMocks.writeModulePerformanceLog.mockResolvedValue(undefined);
    dialogMocks.save.mockResolvedValue("/tmp/log-export.txt");
    dialogMocks.open.mockResolvedValue(null);
    openerMocks.openUrl.mockResolvedValue(undefined);
    appUpdateState.set(defaultAppUpdateState());
    setModulePerformanceDiagnosticsEnabledState(false);
    loggerMocks.logAppEvent.mockResolvedValue(undefined);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.unstubAllEnvs();
    focusedSettingsToolId.set(null);
  });

  it("only shows the add-tool action in the Tools section", async () => {
    render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getToolPaths).toHaveBeenCalled();
    });

    expect(screen.queryByLabelText("Add Custom Tool")).not.toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: "Tools" }));

    expect(screen.getByLabelText("Add Custom Tool")).toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: "General" }));
    expect(screen.queryByLabelText("Add Custom Tool")).not.toBeInTheDocument();
  });

  it("creates a custom tool with expanded configuration fields and pickers", async () => {
    activeSettingsTab.set("tools");
    render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getToolPaths).toHaveBeenCalled();
    });
    toolStores.loadTools.mockClear();
    apiMocks.setManagedTools.mockClear();

    await fireEvent.click(screen.getByLabelText("Add Custom Tool"));
    const dialog = screen.getByRole("dialog");

    expect(
      within(dialog).getByText(
        "Custom tools use generic sources. Confirm that the target tool actually reads these Rules, Skills, MCP, or configuration files.",
      ),
    ).toBeInTheDocument();
    expect(
      within(dialog).getByText(
        "Fill at least one configuration item to connect Rules, Skills, MCP, or Config modules; you can still save an empty draft.",
      ),
    ).toBeInTheDocument();

    for (const input of within(dialog).getAllByRole("textbox")) {
      expect(input).not.toHaveAttribute("placeholder");
    }

    const nameInput = within(dialog).getByLabelText("Name");
    const confirmButton = within(dialog).getByRole("button", { name: "Confirm" });
    expect(within(dialog).getByText("*")).toBeInTheDocument();
    expect(nameInput).toBeRequired();
    expect(confirmButton).toBeDisabled();

    await fireEvent.input(nameInput, { target: { value: "Nova Tool" } });
    expect(confirmButton).toBeEnabled();

    dialogMocks.open
      .mockResolvedValueOnce("/picked/rules")
      .mockResolvedValueOnce("/picked/global.md");
    await fireEvent.click(within(dialog).getAllByRole("button", { name: "Pick Folder" })[0]);
    await fireEvent.click(within(dialog).getAllByRole("button", { name: "Pick File" })[0]);
    await waitFor(() => {
      expect(
        within(dialog).getByText("At least one configuration item is filled. Saving will connect the matching feature module."),
      ).toBeInTheDocument();
    });

    await fireEvent.input(within(dialog).getByLabelText("Skills Directory"), { target: { value: "/picked/skills" } });
    await fireEvent.input(within(dialog).getByLabelText("MCP Configuration"), { target: { value: "/picked/mcp.json" } });
    await fireEvent.input(within(dialog).getByLabelText("Tool Configuration"), { target: { value: "/picked/settings.json" } });
    await fireEvent.click(within(dialog).getByRole("button", { name: "Direct" }));
    await fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(apiMocks.addCustomTool).toHaveBeenCalledWith({
        id: "nova_tool",
        name: "Nova Tool",
        icon: "wrench",
        config_dir: "",
        rule_directory: "/picked/rules",
        global_rule_file: "/picked/global.md",
        skills_dir: "/picked/skills",
        shared_skill_direct_read: true,
        mcp_config: "/picked/mcp.json",
        tool_config: "/picked/settings.json",
        rule_file: "/picked/global.md",
      });
    });
    expect(apiMocks.setManagedTools).toHaveBeenCalledWith(["cursor", "codex", "nova_tool"]);
    expect(get(toolStores.managedToolIds)).toEqual(["cursor", "codex", "nova_tool"]);
    expect(toolStores.loadTools).toHaveBeenCalledTimes(1);
  });

  it("keeps empty custom tool definitions outside managed scope", async () => {
    activeSettingsTab.set("tools");
    render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getToolPaths).toHaveBeenCalled();
    });
    toolStores.loadTools.mockClear();
    apiMocks.setManagedTools.mockClear();

    await fireEvent.click(screen.getByLabelText("Add Custom Tool"));
    const dialog = screen.getByRole("dialog");
    await fireEvent.input(within(dialog).getByLabelText("Name"), { target: { value: "Draft Tool" } });
    await fireEvent.click(within(dialog).getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(apiMocks.addCustomTool).toHaveBeenCalledWith({
        id: "draft_tool",
        name: "Draft Tool",
        icon: "wrench",
        config_dir: "",
        rule_directory: "",
        global_rule_file: "",
        skills_dir: "",
        shared_skill_direct_read: false,
        mcp_config: "",
        tool_config: "",
        rule_file: "",
      });
    });
    expect(apiMocks.setManagedTools).not.toHaveBeenCalled();
    expect(get(toolStores.managedToolIds)).toEqual(["cursor", "codex"]);
    expect(toolStores.loadTools).toHaveBeenCalledTimes(1);
  });

  it("omits withheld Settings surfaces and redirects stale Project state", async () => {
    activeSettingsTab.set("project");
    render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getToolPaths).toHaveBeenCalled();
    });

    await waitFor(() => {
      expect(get(activeSettingsTab)).toBe("general");
    });

    expect(screen.queryByRole("button", { name: "Project" })).not.toBeInTheDocument();
    expect(screen.queryByText("Project Path")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Backup Now" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "View Backups" })).not.toBeInTheDocument();
    expect(screen.queryByText("Editor")).not.toBeInTheDocument();
    expect(screen.queryByText("Skill Source Handling")).not.toBeInTheDocument();
  });

  it("shows Modus About copy without account or license actions", async () => {
    render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getToolPaths).toHaveBeenCalled();
    });

    expect(screen.getByText("About Modus")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /License|Account/i })).not.toBeInTheDocument();
  });

  it("opens the project GitHub page from About", async () => {
    render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getToolPaths).toHaveBeenCalled();
    });

    await fireEvent.click(screen.getByRole("button", { name: "GitHub" }));

    await waitFor(() => {
      expect(openerMocks.openUrl).toHaveBeenCalledWith("https://github.com/leon4z/Modus");
    });
  });

  it("runs a manual app update check from About", async () => {
    render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getToolPaths).toHaveBeenCalled();
    });

    const aboutRow = screen.getByText("About Modus").closest(".settings-list-row");
    expect(within(aboutRow).getAllByRole("button").map((button) => button.textContent.trim())).toEqual([
      "GitHub",
      "Check for Updates",
    ]);
    expect(within(aboutRow).getByRole("button", { name: "Check for Updates" }).querySelector("svg")).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "Check for Updates" }));

    await waitFor(() => {
      expect(appUpdateMocks.runAppUpdateCheck).toHaveBeenCalledWith("manual");
    });
    expect(screen.getByText("Modus is up to date.")).toBeInTheDocument();
  });

  it("shows available app updates with skip, install, and restart actions", async () => {
    appUpdateState.set(defaultAppUpdateState({
      status: "available",
      availableUpdate: {
        version: "0.2.0",
        currentVersion: "0.1.0",
        channel: "stable",
        date: null,
        body: null,
      },
    }));
    render(SettingsModule);

    await waitFor(() => {
      expect(screen.getByText("Version 0.2.0 is available on the Release channel.")).toBeInTheDocument();
    });

    const aboutRow = screen.getByText("About Modus").closest(".settings-list-row");
    expect(screen.getByTestId("settings-update-status")).toHaveClass("settings-update-status--available");
    expect(within(aboutRow).getAllByRole("button").map((button) => button.textContent.trim())).toEqual([
      "GitHub",
      "Check for Updates",
      "Skip",
      "Install Update",
    ]);
    expect(within(aboutRow).getByRole("button", { name: "Check for Updates" }).querySelector("svg")).toBeNull();
    expect(within(aboutRow).getByRole("button", { name: "Install Update" }).querySelector("svg")).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "Skip" }));
    expect(appUpdateMocks.skipAvailableAppUpdate).toHaveBeenCalled();
    expect(screen.getByText("Skipped this update version.")).toBeInTheDocument();

    appUpdateState.set(defaultAppUpdateState({
      status: "available",
      availableUpdate: {
        version: "0.2.0",
        currentVersion: "0.1.0",
        channel: "stable",
        date: null,
        body: null,
      },
    }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Install Update" })).toBeInTheDocument();
    });
    await fireEvent.click(screen.getByRole("button", { name: "Install Update" }));
    expect(appUpdateMocks.installAvailableAppUpdate).toHaveBeenCalled();

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Restart" })).toBeInTheDocument();
    });
    expect(screen.getByTestId("settings-update-status")).toHaveClass("settings-update-status--restart");
    await fireEvent.click(screen.getByRole("button", { name: "Restart" }));
    expect(appUpdateMocks.restartIntoAppUpdate).toHaveBeenCalled();
  });

  it("disables app update checks in development diagnostics", async () => {
    appUpdateState.set(defaultAppUpdateState({
      status: "disabled",
      channel: "disabled",
      canCheck: false,
    }));
    apiMocks.getRuntimeInfo.mockResolvedValue({
      shape: "development-sandbox",
      updateChannel: "disabled",
      canCheckForUpdates: false,
      usesSandboxTools: true,
      usesRealTools: false,
    });

    render(SettingsModule);

    expect(await screen.findByText("This runtime does not check for app updates.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Check for Updates" })).toBeDisabled();
  });

  it("hides internal runtime labels in release Settings", async () => {
    render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getRuntimeInfo).toHaveBeenCalled();
    });

    expect(screen.getByText("About Modus")).toBeInTheDocument();
    expect(screen.queryByTestId("settings-runtime-marker")).not.toBeInTheDocument();
    expect(screen.queryByText("Development Sandbox")).not.toBeInTheDocument();
    expect(screen.queryByText("Pre-release Version")).not.toBeInTheDocument();
    expect(screen.queryByText(/stable channel/i)).not.toBeInTheDocument();
  });

  it("shows the development sandbox marker only in Settings", async () => {
    apiMocks.getRuntimeInfo.mockResolvedValue({
      shape: "development-sandbox",
      updateChannel: "disabled",
      canCheckForUpdates: false,
      usesSandboxTools: true,
      usesRealTools: false,
    });

    render(SettingsModule);

    expect(await screen.findByTestId("settings-runtime-marker")).toBeInTheDocument();
    expect(screen.getByText("Development Sandbox")).toBeInTheDocument();
    expect(screen.getByText(/isolated data and sandbox tools/i)).toBeInTheDocument();
    expect(screen.getByText("About Modus")).toBeInTheDocument();
    expect(screen.queryByText(/Modus Dev/i)).not.toBeInTheDocument();
  });

  it("shows pre-release real-tool context without changing the product name", async () => {
    apiMocks.getRuntimeInfo.mockResolvedValue({
      shape: "pre-release",
      updateChannel: "test",
      canCheckForUpdates: true,
      usesSandboxTools: false,
      usesRealTools: true,
    });

    render(SettingsModule);

    expect(await screen.findByTestId("settings-runtime-marker")).toBeInTheDocument();
    expect(screen.getByText("Pre-release Version")).toBeInTheDocument();
    expect(screen.getByText(/read or write real tool configuration/i)).toBeInTheDocument();
    expect(screen.getByText(/test channel/i)).toBeInTheDocument();
    expect(screen.getByText("About Modus")).toBeInTheDocument();
    expect(screen.queryByText(/Modus Pre-release/i)).not.toBeInTheDocument();
  });

  it("shows a local diagnostic when runtime identity is unavailable in a non-release build", async () => {
    apiMocks.getRuntimeInfo.mockRejectedValue(new Error("runtime command unavailable"));

    render(SettingsModule);

    expect(await screen.findByTestId("settings-runtime-marker")).toBeInTheDocument();
    expect(screen.getByText("Runtime Unavailable")).toBeInTheDocument();
    expect(screen.getByText(/could not be read/i)).toBeInTheDocument();
    expect(screen.getByText("About Modus")).toBeInTheDocument();
  });

  it("hides runtime identity failures in a release frontend build", async () => {
    vi.stubEnv("DEV", false);
    vi.stubEnv("PROD", true);
    apiMocks.getRuntimeInfo.mockRejectedValue(new Error("runtime command unavailable"));

    render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getRuntimeInfo).toHaveBeenCalled();
    });

    expect(screen.getByText("About Modus")).toBeInTheDocument();
    expect(screen.queryByTestId("settings-runtime-marker")).not.toBeInTheDocument();
    expect(screen.queryByText("Runtime Unavailable")).not.toBeInTheDocument();
    expect(screen.queryByText(/could not be read/i)).not.toBeInTheDocument();
  });

  it("does not expose pre-release local data in General settings", async () => {
    render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getToolPaths).toHaveBeenCalled();
    });

    expect(screen.queryByRole("heading", { name: /local data/i })).not.toBeInTheDocument();
    expect(apiMocks.getLegacyLocalDataReport).toBeUndefined();
  });

  it("keeps tool editing scoped to a single row at a time", async () => {
    activeSettingsTab.set("tools");
    render(SettingsModule);

    await editSettingsToolRow("cursor");

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
    });
    await fireEvent.input(screen.getByDisplayValue("/cursor/skills"), { target: { value: "/cursor/changed" } });
    const codexRow = settingsToolRow("codex");
    const secondEditButton = within(codexRow).getByRole("button", { name: "Edit" });
    expect(secondEditButton).toBeEnabled();

    await fireEvent.click(secondEditButton);

    await waitFor(() => {
      expect(screen.queryByDisplayValue("/cursor/changed")).not.toBeInTheDocument();
      expect(screen.getByDisplayValue("/codex/skills")).toBeInTheDocument();
    });
  });

  it("keeps tool details collapsed by default and uses icon actions in edit mode", async () => {
    activeSettingsTab.set("tools");
    render(SettingsModule);

    await waitFor(() => {
      expect(settingsToolRow("cursor")).toBeTruthy();
    });
    expect(screen.queryByText("/cursor/skills")).not.toBeInTheDocument();

    await expandSettingsToolRow("cursor");
    expect(screen.getByText("/cursor/skills")).toBeInTheDocument();

    const expandButton = within(settingsToolRow("cursor")).getByRole("button", { name: "Collapse settings" });
    expect(expandButton).toHaveClass("st-row-expand-btn");
    expect(expandButton.querySelector("svg")).toHaveAttribute("width", "13");
    expect(expandButton.querySelector("svg")).toHaveAttribute("height", "13");

    const editButton = within(settingsToolRow("cursor")).getByRole("button", { name: "Edit" });
    expect(editButton).toHaveClass("icon-btn");
    expect(editButton).toHaveClass("st-row-action-btn");
    expect(editButton.textContent?.trim()).toBe("");

    await editSettingsToolRow("cursor");
    const cursorRow = settingsToolRow("cursor");
    const cancelButton = within(cursorRow).getByRole("button", { name: "Cancel" });
    expect(cancelButton).toHaveClass("icon-btn");
    expect(cancelButton).toHaveClass("st-row-action-btn");
    expect(cancelButton.textContent?.trim()).toBe("");

    const picker = within(cursorRow).getAllByRole("button", { name: "Pick Folder" })[0];
    expect(picker).toHaveClass("icon-btn");
    expect(picker).toHaveClass("st-picker-btn");
    expect(picker.querySelector("svg")).toHaveAttribute("width", "13");
    expect(picker.querySelector("svg")).toHaveAttribute("height", "13");

    const fieldRows = Array.from(cursorRow.querySelectorAll(".st-field-row"));
    const skillsRow = /** @type {HTMLElement} */ (fieldRows.find((fieldRow) => fieldRow.textContent?.includes("Skills Directory")));

    await fireEvent.input(within(skillsRow).getByDisplayValue("/cursor/skills"), { target: { value: "/cursor/skills-changed" } });
    const skillsResetButton = within(skillsRow).getByRole("button", { name: "Reset" });
    expect(skillsResetButton).toHaveClass("icon-btn");
    expect(skillsResetButton).toHaveClass("st-picker-btn");
    expect(skillsResetButton.textContent?.trim()).toBe("");
    await fireEvent.click(skillsResetButton);
    expect(within(skillsRow).getByDisplayValue("/cursor/skills")).toBeInTheDocument();

    await fireEvent.input(within(skillsRow).getByDisplayValue("/cursor/skills"), { target: { value: "/cursor/changed" } });
    const confirmButton = within(cursorRow).getByRole("button", { name: "Confirm" });
    expect(confirmButton).toHaveClass("icon-btn");
    expect(confirmButton).toHaveClass("st-row-action-btn");
    expect(confirmButton.textContent?.trim()).toBe("");

    await fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => {
      expect(within(cursorRow).queryByRole("button", { name: "Cancel" })).not.toBeInTheDocument();
    });
    expect(screen.queryByDisplayValue("/cursor/changed")).not.toBeInTheDocument();
  });

  it("shows unsupported rule defaults while keeping local custom controls available", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "cursor",
        name: "Cursor",
        detected: true,
        config_dir: "/cursor/config",
        skills_dir: "/cursor/skills",
        capabilities: [
          {
            id: "user-rules",
            kind: "rule",
            scope: "global",
            access: "unsupported",
            format: "unknown",
            source_path: "",
            source_confidence: "official_docs",
            notes: "Cursor User Rules are app-internal settings and do not expose a stable file-backed sync target.",
          },
        ],
      },
      {
        id: "trae",
        name: "Trae",
        detected: true,
        config_dir: "/trae/config",
        skills_dir: "",
        capabilities: [
          {
            id: "user-rules",
            kind: "rule",
            scope: "global",
            access: "unsupported",
            format: "unknown",
            source_path: "",
            source_confidence: "official_community",
            notes: "Trae global rules are not confirmed as a stable external file-backed target.",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["cursor", "trae"]);
    apiMocks.getInjectionTargets.mockResolvedValue({});

    render(SettingsModule);

    await expandSettingsToolRow("cursor");
    await expandSettingsToolRow("trae");
    expect((await screen.findAllByText("Unsupported")).length).toBeGreaterThanOrEqual(4);
    expect(screen.queryByText(/built-in user rules do not support file-form sync/)).not.toBeInTheDocument();

    await editSettingsToolRow("cursor");

    expect(screen.getByPlaceholderText("Rule directory path")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Global Rule file path")).toBeInTheDocument();
  });

  it("shows missing Global Rule target guidance without creating a target", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "trae",
        name: "Trae",
        detected: true,
        config_dir: "/trae/config",
        skills_dir: "",
        capabilities: [
          {
            id: "user-rules",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/trae/user_rules",
            source_confidence: "certified_local_product_behavior",
            action_evidence: [
              { action: "create", supported: true, evidence: "verified" },
              { action: "read", supported: true, evidence: "verified" },
            ],
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["trae"]);
    apiMocks.getToolPaths.mockResolvedValue({});
    apiMocks.getInjectionTargets.mockResolvedValue({});

    render(SettingsModule);

    await expandSettingsToolRow("trae");
    expect(await screen.findByText("/trae/user_rules")).toBeInTheDocument();
    expect(screen.getAllByText("Not configured").length).toBeGreaterThanOrEqual(1);
    expect(screen.queryByText("This tool does not support a Global Rule file")).not.toBeInTheDocument();
    expect(screen.queryByText("Missing target")).not.toBeInTheDocument();

    await editSettingsToolRow("trae");

    expect(screen.queryByPlaceholderText("Target file path for generic rule injection")).not.toBeInTheDocument();
    expect(screen.getByPlaceholderText("Global Rule file path")).toBeInTheDocument();
  });

  it("lets unsupported and unconfirmed rule fields be corrected with custom overrides", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "unsupported-rules",
        name: "Unsupported Rules",
        detected: true,
        config_dir: "/unsupported/config",
        skills_dir: "/unsupported/skills",
        capabilities: [
          {
            id: "rules",
            kind: "rule",
            scope: "global",
            access: "unsupported",
            format: "unknown",
            source_path: "",
            source_confidence: "certified_local_product_behavior",
            notes: "Local verification proved this tool does not support Rules management.",
          },
        ],
      },
      {
        id: "unknown-rules",
        name: "Unknown Rules",
        detected: true,
        config_dir: "/unknown/config",
        skills_dir: "/unknown/skills",
        capabilities: [
          {
            id: "rules",
            kind: "rule",
            scope: "global",
            access: "unknown",
            format: "unknown",
            source_path: "",
            source_confidence: "unknown",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["unsupported-rules", "unknown-rules"]);
    apiMocks.getToolPaths.mockResolvedValue({});
    apiMocks.getInjectionTargets.mockResolvedValue({
      "unsupported-rules": "/unsupported/legacy.md",
    });
    apiMocks.getToolCapabilityOverrides.mockResolvedValue({
      "unsupported-rules": {
        customGlobalRuleTarget: "/unsupported/custom.md",
      },
    });

    render(SettingsModule);

    await expandSettingsToolRow("unsupported-rules");
    expect((await screen.findAllByText("Unsupported")).length).toBeGreaterThanOrEqual(2);
    expect(screen.queryByText("Rule management is not confirmed for this tool")).not.toBeInTheDocument();
    expect(screen.queryByText("Missing target")).not.toBeInTheDocument();
    expect(screen.queryByText("/unsupported/legacy.md")).not.toBeInTheDocument();
    expect(await screen.findByText("/unsupported/custom.md")).toBeInTheDocument();
    expect(screen.getAllByText("Custom").length).toBeGreaterThanOrEqual(1);

    await editSettingsToolRow("unsupported-rules");

    expect(screen.queryByPlaceholderText("Target file path for generic rule injection")).not.toBeInTheDocument();
    await fireEvent.input(screen.getByPlaceholderText("Rule directory path"), {
      target: { value: "/unsupported/rules" },
    });
    await fireEvent.input(screen.getByDisplayValue("/unsupported/custom.md"), {
      target: { value: "/unsupported/new-global.md" },
    });
    await fireEvent.input(screen.getByPlaceholderText("MCP configuration file path"), {
      target: { value: "/unsupported/mcp.json" },
    });
    await fireEvent.input(screen.getByPlaceholderText("Tool configuration file path"), {
      target: { value: "/unsupported/settings.json" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(apiMocks.setToolCapabilityOverrides).toHaveBeenCalledWith("unsupported-rules", {
        customRuleSourceType: "directory",
        customRuleSourcePath: "/unsupported/rules",
        customGlobalRuleTarget: "/unsupported/new-global.md",
        customMcpConfigPath: "/unsupported/mcp.json",
        customToolConfigPath: "/unsupported/settings.json",
        sharedSkillDirectRead: null,
      });
    });
  });

  it("shows direct shared Skill availability from certified defaults", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/codex/config",
        skills_dir: "/codex/skills",
        supports_generic_skills: true,
        capabilities: [],
      },
    ]);
    toolStores.managedToolIds.set(["codex"]);

    render(SettingsModule);

    await expandSettingsToolRow("codex");
    expect(await screen.findByText("Direct")).toBeInTheDocument();
    expect(screen.queryByText(/Default:/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Current:/)).not.toBeInTheDocument();
    expect(screen.queryByText("Certified default")).not.toBeInTheDocument();
  });

  it("marks customized primary Skill directories", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "windsurf",
        name: "Windsurf",
        detected: true,
        config_dir: "/Users/leon/.codeium/windsurf1",
        default_config_dir: "/Users/leon/.codeium/windsurf",
        skills_dir: "/Users/leon/.codeium/windsurf/skills1",
        default_skills_dir: "/Users/leon/.codeium/windsurf/skills",
        supports_generic_skills: true,
      },
    ]);
    toolStores.managedToolIds.set(["windsurf"]);
    apiMocks.getToolPaths.mockResolvedValue({
      windsurf: {
        config_dir: "/Users/leon/.codeium/windsurf1",
        skills_dir: "/Users/leon/.codeium/windsurf/skills1",
      },
    });
    apiMocks.getInjectionTargets.mockResolvedValue({});
    apiMocks.getToolCapabilityOverrides.mockResolvedValue({});

    render(SettingsModule);

    await expandSettingsToolRow("windsurf");
    expect(await screen.findByText("/Users/leon/.codeium/windsurf/skills1")).toBeInTheDocument();
    expect(screen.queryByText("/Users/leon/.codeium/windsurf1")).not.toBeInTheDocument();

    const row = /** @type {HTMLElement} */ (document.querySelector('[data-settings-tool-id="windsurf"]'));
    expect(row).toBeTruthy();
    const fieldRows = Array.from(row.querySelectorAll(".st-field-row"));
    const skillsRow = fieldRows.find((fieldRow) => fieldRow.textContent?.includes("Skills Directory"));
    expect(skillsRow).toHaveTextContent("Custom");
  });

  it("confirms and resets custom tool settings back to defaults", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "windsurf",
        name: "Windsurf",
        detected: true,
        config_dir: "/Users/leon/.codeium/windsurf",
        skills_dir: "/Users/leon/.codeium/windsurf/skills",
        supports_generic_skills: true,
        capabilities: [
          {
            id: "mcp-config",
            kind: "mcp",
            scope: "global",
            access: "writable",
            format: "json",
            source_path: "/Users/leon/.codeium/windsurf/mcp_config.json",
          },
          {
            id: "user-configured-mcp-config",
            kind: "mcp",
            scope: "global",
            access: "writable",
            format: "json",
            source_path: "/Users/leon/.codeium/windsurf/mcp_config.json",
            source_confidence: "user_configured",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["windsurf"]);
    apiMocks.getToolPaths.mockResolvedValue({
      windsurf: {
        config_dir: "/Users/leon/.codeium/windsurf-custom",
        skills_dir: "/Users/leon/.codeium/windsurf/skills-custom",
      },
    });
    apiMocks.getToolCapabilityOverrides.mockResolvedValue({
      windsurf: {
        customMcpConfigPath: "/Users/leon/.codeium/windsurf/custom-mcp.json",
        sharedSkillDirectRead: false,
      },
    });
    apiMocks.getInjectionTargets.mockResolvedValue({});

    render(SettingsModule);

    const windsurfRow = await expandSettingsToolRow("windsurf");
    expect(windsurfRow).toHaveTextContent("Custom");
    const resetButton = within(windsurfRow).getByRole("button", { name: "Reset Defaults" });
    expect(resetButton).toHaveClass("icon-btn");
    expect(resetButton).toHaveClass("st-row-action-btn");
    expect(resetButton.textContent?.trim()).toBe("");

    await fireEvent.click(within(windsurfRow).getByRole("button", { name: "Edit" }));
    expect(within(windsurfRow).queryByRole("button", { name: "Reset Defaults" })).not.toBeInTheDocument();
    await fireEvent.click(within(windsurfRow).getByRole("button", { name: "Cancel" }));

    await fireEvent.click(within(windsurfRow).getByRole("button", { name: "Reset Defaults" }));

    const dialog = await screen.findByRole("dialog", { name: "Reset to Defaults" });
    expect(dialog).not.toHaveTextContent("Config Directory");
    expect(dialog).toHaveTextContent("Skills Directory");
    expect(dialog).toHaveTextContent("Shared Skills Support");
    expect(dialog).toHaveTextContent("MCP Configuration");
    expect(dialog).not.toHaveTextContent("/Users/leon/.codeium/windsurf-custom");
    expect(dialog).toHaveTextContent("/Users/leon/.codeium/windsurf/skills-custom");

    await fireEvent.click(within(dialog).getByRole("button", { name: "Reset" }));

    await waitFor(() => {
      expect(apiMocks.setToolPath).toHaveBeenCalledWith(
        "windsurf",
        "/Users/leon/.codeium/windsurf-custom",
        "/Users/leon/.codeium/windsurf/skills"
      );
      expect(apiMocks.setToolCapabilityOverrides).toHaveBeenCalledWith("windsurf", {
        customRuleSourceType: null,
        customRuleSourcePath: null,
        customGlobalRuleTarget: null,
        customMcpConfigPath: null,
        customToolConfigPath: null,
        sharedSkillDirectRead: null,
      });
      expect(skillInventoryMocks.invalidateSkillInventory).toHaveBeenCalledTimes(1);
    });
  });

  it("does not mark redundant MCP defaults as custom", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "windsurf",
        name: "Windsurf",
        detected: true,
        config_dir: "/Users/leon/.codeium/windsurf",
        skills_dir: "/Users/leon/.codeium/windsurf/skills",
        default_skills_dir: "/Users/leon/.codeium/windsurf/skills",
        capabilities: [
          {
            id: "mcp-config",
            kind: "mcp",
            scope: "global",
            access: "writable",
            format: "json",
            source_path: "/Users/leon/.codeium/windsurf/mcp_config.json",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["windsurf"]);
    apiMocks.getToolCapabilityOverrides.mockResolvedValue({
      windsurf: {
        customMcpConfigPath: "/Users/leon/.codeium/windsurf/mcp_config.json",
      },
    });

    render(SettingsModule);

    const windsurfRow = await expandSettingsToolRow("windsurf");
    const fieldRows = Array.from(windsurfRow.querySelectorAll(".st-field-row"));
    const mcpRow = fieldRows.find((fieldRow) => fieldRow.textContent?.includes("MCP Configuration"));
    expect(mcpRow).toBeTruthy();
    expect(mcpRow).toHaveTextContent("/Users/leon/.codeium/windsurf/mcp_config.json");
    expect(mcpRow).not.toHaveTextContent("Custom");
    expect(within(windsurfRow).queryByRole("button", { name: "Reset Defaults" })).not.toBeInTheDocument();

    await editSettingsToolRow("windsurf");
    const mcpFilePicker = within(mcpRow).getByRole("button", { name: "Pick File" });
    expect(mcpFilePicker.closest(".st-field-actions")).toBeTruthy();
    await fireEvent.input(screen.getByDisplayValue("/Users/leon/.codeium/windsurf/skills"), {
      target: { value: "/Users/leon/.codeium/windsurf/skills2" },
    });
    await fireEvent.click(within(windsurfRow).getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(apiMocks.setToolCapabilityOverrides).toHaveBeenCalledWith("windsurf", {
        customRuleSourceType: null,
        customRuleSourcePath: null,
        customGlobalRuleTarget: null,
        customMcpConfigPath: null,
        customToolConfigPath: null,
        sharedSkillDirectRead: null,
      });
    });
  });

  it("keeps installed tools visible without showing default path health as Settings warnings", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "cursor",
        name: "Cursor",
        detected: true,
        primary_config_health: "missing",
        primary_skills_health: "missing",
        config_dir: "/cursor/config",
        skills_dir: "/cursor/skills",
      },
      {
        id: "leftover",
        name: "Leftover",
        detected: false,
        primary_config_health: "missing",
        config_dir: "/leftover/config",
        skills_dir: "",
      },
    ]);
    toolStores.managedToolIds.set(["cursor", "leftover"]);

    render(SettingsModule);

    expect(await screen.findByText("Cursor")).toBeInTheDocument();
    expect(screen.queryByText("Installed")).not.toBeInTheDocument();
    await expandSettingsToolRow("cursor");
    expect(screen.queryByText("Config directory missing")).not.toBeInTheDocument();
    expect(screen.queryByText("Skills directory missing")).not.toBeInTheDocument();
    expect(screen.queryByText("Leftover")).not.toBeInTheDocument();
  });

  it("refreshes tool health when opening Tools settings before showing row-level reasons", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "kiro",
        name: "Kiro",
        detected: true,
        primary_config_health: "ok",
        primary_skills_health: "ok",
        config_dir: "/Users/leon/.kiro",
        skills_dir: "/Users/leon/.kiro/skills",
      },
    ]);
    toolStores.managedToolIds.set(["kiro"]);
    toolStores.loadTools.mockImplementationOnce(async () => {
      toolStores.tools.set([
        {
          id: "kiro",
          name: "Kiro",
          detected: true,
          primary_config_health: "missing",
          primary_skills_health: "missing",
          config_dir: "/Users/leon/.kiro",
          skills_dir: "/Users/leon/.kiro/skills",
        },
      ]);
      return [];
    });

    render(SettingsModule);

    expect(await screen.findByText("Kiro")).toBeInTheDocument();
    await expandSettingsToolRow("kiro");
    await waitFor(() => {
      expect(toolStores.loadTools).toHaveBeenCalled();
      expect(screen.queryByText("Skills directory missing")).not.toBeInTheDocument();
    });
  });

  it("focuses routed tools without entering edit mode or surfacing default path health", async () => {
    const scrollIntoView = vi.fn();
    const originalScrollIntoView = Element.prototype.scrollIntoView;
    Element.prototype.scrollIntoView = scrollIntoView;
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "cursor",
        name: "Cursor",
        detected: true,
        primary_config_health: "unreadable",
        primary_skills_health: "unreadable",
        config_dir: "/cursor/config",
        skills_dir: "/cursor/skills",
      },
      {
        id: "codex",
        name: "Codex",
        detected: true,
        primary_config_health: "ok",
        config_dir: "/codex/config",
        skills_dir: "/codex/skills",
      },
    ]);
    toolStores.managedToolIds.set(["cursor", "codex"]);

    render(SettingsModule);

    expect(await screen.findByText("Cursor")).toBeInTheDocument();
    await expandSettingsToolRow("cursor");
    const cursorRow = /** @type {HTMLElement} */ (document.querySelector('[data-settings-tool-id="cursor"]'));
    expect(cursorRow).toBeTruthy();
    expect(cursorRow).toHaveTextContent("/cursor/skills");
    expect(cursorRow).not.toHaveTextContent("Skills directory unreadable");

    focusedSettingsToolId.set("cursor");

    await waitFor(() => {
      expect(scrollIntoView).toHaveBeenCalledWith({ block: "center", behavior: "smooth" });
      expect(cursorRow).toHaveClass("st-row-block--focused");
    });
    expect(screen.queryByRole("button", { name: "Cancel" })).not.toBeInTheDocument();
    Element.prototype.scrollIntoView = originalScrollIntoView;
  });

  it("shows missing diagnostics consistently for custom source paths", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "claude-code",
        name: "Claude Code",
        detected: true,
        config_dir: "/Users/leon/.claude",
        skills_dir: "/Users/leon/.claude/skills1",
        default_skills_dir: "/Users/leon/.claude/skills",
        supports_generic_skills: true,
        rule_directory_health: "missing",
        global_rule_file_health: "missing",
        primary_skills_health: "missing",
        mcp_config_health: "missing",
        tool_config_health: "missing",
      },
    ]);
    toolStores.managedToolIds.set(["claude-code"]);
    apiMocks.getManagedTools.mockResolvedValue(["claude-code"]);
    apiMocks.getToolPaths.mockResolvedValue({
      "claude-code": {
        config_dir: "/Users/leon/.claude",
        skills_dir: "/Users/leon/.claude/skills1",
      },
    });
    apiMocks.getToolCapabilityOverrides.mockResolvedValue({
      "claude-code": {
        customRuleSourcePath: "/Users/leon/.claude/rules1",
        customGlobalRuleTarget: "/Users/leon/.claude/CLAUDE.md1",
        customMcpConfigPath: "/Users/leon/.claude.json1",
        customToolConfigPath: "/Users/leon/.claude/settings.json1",
      },
    });

    render(SettingsModule);

    const row = await expandSettingsToolRow("claude-code");

    await waitFor(() => {
      const missingLabels = Array.from(row.querySelectorAll(".st-field-attention")).map((label) =>
        label.textContent?.trim()
      );
      expect(missingLabels).toEqual(["Missing", "Missing", "Missing", "Missing", "Missing"]);
    });
  });

  it("marks user capability overrides and persists them through Settings", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/codex/config",
        skills_dir: "/codex/skills",
        supports_generic_skills: true,
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/AGENTS.md",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["codex"]);
    apiMocks.getToolCapabilityOverrides.mockResolvedValue({
      codex: {
        customGlobalRuleTarget: "/codex/CUSTOM.md",
        sharedSkillDirectRead: false,
      },
    });

    render(SettingsModule);

    await expandSettingsToolRow("codex");
    expect((await screen.findAllByText("/codex/CUSTOM.md")).length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("Custom").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Symlink")).toBeInTheDocument();
    expect(screen.queryByText(/Default:/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Current:/)).not.toBeInTheDocument();

    await editSettingsToolRow("codex");
    expect(screen.queryByRole("button", { name: "Default" })).not.toBeInTheDocument();
    const codexRow = settingsToolRow("codex");
    const fieldRows = Array.from(codexRow.querySelectorAll(".st-field-row"));
    const sharedSkillRow = fieldRows.find((fieldRow) => fieldRow.textContent?.includes("Shared Skills Support"));
    expect(sharedSkillRow).toBeTruthy();
    const sharedResetButton = within(sharedSkillRow).getByRole("button", { name: "Reset" });
    expect(sharedResetButton).toHaveClass("icon-btn");
    expect(sharedResetButton).toHaveClass("st-picker-btn");
    expect(sharedResetButton.closest(".st-field-actions")).toBeTruthy();
    expect(sharedResetButton.textContent?.trim()).toBe("");
    await fireEvent.click(sharedResetButton);
    expect(within(sharedSkillRow).queryByRole("button", { name: "Reset" })).not.toBeInTheDocument();

    const globalRuleRow = fieldRows.find((fieldRow) => fieldRow.textContent?.includes("Global Rule File"));
    expect(globalRuleRow).toBeTruthy();
    const resetButton = within(globalRuleRow).getByRole("button", { name: "Reset" });
    expect(resetButton).toHaveClass("icon-btn");
    expect(resetButton).toHaveClass("st-picker-btn");
    expect(resetButton.textContent?.trim()).toBe("");
    await fireEvent.click(resetButton);
    expect(screen.getByDisplayValue("/codex/AGENTS.md")).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(apiMocks.setToolCapabilityOverrides).toHaveBeenCalledWith("codex", {
        customRuleSourceType: null,
        customRuleSourcePath: null,
        customGlobalRuleTarget: null,
        customMcpConfigPath: null,
        customToolConfigPath: null,
        sharedSkillDirectRead: null,
      });
    });
    expect(apiMocks.setInjectionTarget).not.toHaveBeenCalledWith("codex", "/codex/AGENTS.md");
  });

  it("invalidates Skill inventory after saving shared Skill direct-read override changes", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/codex/config",
        skills_dir: "/codex/skills",
        supports_generic_skills: true,
        capabilities: [],
      },
    ]);
    toolStores.managedToolIds.set(["codex"]);
    apiMocks.getToolPaths.mockResolvedValue({
      codex: { config_dir: "/codex/config", skills_dir: "/codex/skills" },
    });
    apiMocks.getInjectionTargets.mockResolvedValue({});
    apiMocks.getToolCapabilityOverrides.mockResolvedValue({});

    render(SettingsModule);

    await editSettingsToolRow("codex");
    expect(await screen.findByText("Direct")).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("button", { name: "Symlink" }));
    await fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(apiMocks.setToolCapabilityOverrides).toHaveBeenCalledWith("codex", {
        customRuleSourceType: null,
        customRuleSourcePath: null,
        customGlobalRuleTarget: null,
        customMcpConfigPath: null,
        customToolConfigPath: null,
        sharedSkillDirectRead: false,
      });
      expect(skillInventoryMocks.invalidateSkillInventory).toHaveBeenCalledTimes(1);
    });
  });

  it("persists source edits for detected tools that are not enabled for management", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/codex/config",
        skills_dir: "/codex/skills",
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/AGENTS.md",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set([]);
    apiMocks.getToolPaths.mockResolvedValue({
      codex: { config_dir: "/codex/config", skills_dir: "/codex/skills" },
    });
    apiMocks.getInjectionTargets.mockResolvedValue({});
    apiMocks.getToolCapabilityOverrides.mockResolvedValue({});

    render(SettingsModule);

    await editSettingsToolRow("codex");
    await fireEvent.input(screen.getByDisplayValue("/codex/AGENTS.md"), {
      target: { value: "/codex/CUSTOM.md" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(apiMocks.setToolCapabilityOverrides).toHaveBeenCalledWith("codex", {
        customRuleSourceType: null,
        customRuleSourcePath: null,
        customGlobalRuleTarget: "/codex/CUSTOM.md",
        customMcpConfigPath: null,
        customToolConfigPath: null,
        sharedSkillDirectRead: null,
      });
    });
  });

  it("does not invalidate Skill inventory for unrelated Global Rule target changes", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/codex/config",
        skills_dir: "/codex/skills",
        supports_generic_skills: true,
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/AGENTS.md",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["codex"]);
    apiMocks.getToolPaths.mockResolvedValue({
      codex: { config_dir: "/codex/config", skills_dir: "/codex/skills" },
    });
    apiMocks.getInjectionTargets.mockResolvedValue({});
    apiMocks.getToolCapabilityOverrides.mockResolvedValue({});

    render(SettingsModule);

    await editSettingsToolRow("codex");
    expect(screen.getByDisplayValue("/codex/AGENTS.md")).toBeInTheDocument();
    await fireEvent.input(screen.getByDisplayValue("/codex/AGENTS.md"), { target: { value: "/codex/NEW.md" } });
    await fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(apiMocks.setToolCapabilityOverrides).toHaveBeenCalledWith("codex", {
        customRuleSourceType: null,
        customRuleSourcePath: null,
        customGlobalRuleTarget: "/codex/NEW.md",
        customMcpConfigPath: null,
        customToolConfigPath: null,
        sharedSkillDirectRead: null,
      });
    });
    expect(skillInventoryMocks.invalidateSkillInventory).not.toHaveBeenCalled();
  });

  it("ignores stale configured Global Rule targets when a certified default exists", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/codex/config",
        skills_dir: "/codex/skills",
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/AGENTS.md",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["codex"]);
    apiMocks.getInjectionTargets.mockResolvedValue({
      codex: "/codex/legacy.md",
    });

    render(SettingsModule);

    await expandSettingsToolRow("codex");
    expect((await screen.findAllByText("/codex/AGENTS.md")).length).toBeGreaterThanOrEqual(1);
    expect(screen.queryByText("/codex/legacy.md")).not.toBeInTheDocument();
    expect(screen.queryByText("Default")).not.toBeInTheDocument();
  });

  it("does not persist certified default Global Rule targets during unrelated Settings saves", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/codex/config",
        skills_dir: "/codex/skills",
        supports_generic_skills: true,
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/AGENTS.md",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["codex"]);
    apiMocks.getInjectionTargets.mockResolvedValue({
      codex: "/codex/AGENTS.md",
    });

    render(SettingsModule);

    await waitFor(() => expect(apiMocks.getInjectionTargets).toHaveBeenCalledTimes(1));
    await editSettingsToolRow("codex");
    expect(screen.getByDisplayValue("/codex/AGENTS.md")).toBeInTheDocument();
    await screen.findByRole("button", { name: "Symlink" });
    await fireEvent.click(screen.getByRole("button", { name: "Symlink" }));
    await fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(apiMocks.setToolCapabilityOverrides).toHaveBeenCalledWith("codex", {
        customRuleSourceType: null,
        customRuleSourcePath: null,
        customGlobalRuleTarget: null,
        customMcpConfigPath: null,
        customToolConfigPath: null,
        sharedSkillDirectRead: false,
      });
    });
    expect(apiMocks.setInjectionTarget).not.toHaveBeenCalledWith("codex", "/codex/AGENTS.md");
  });

  it("shows capability source rows and persists independent custom paths", async () => {
    activeSettingsTab.set("tools");
    toolStores.tools.set([
      {
        id: "trae-cn",
        name: "Trae CN",
        detected: true,
        config_dir: "/trae/config",
        skills_dir: "/trae/skills",
        capabilities: [
          {
            id: "user-rules",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/trae/rules",
            source_confidence: "certified_local_product_behavior",
            action_evidence: [
              { action: "read", supported: true, evidence: "verified" },
              { action: "create", supported: true, evidence: "verified" },
              { action: "save", supported: true, evidence: "verified" },
            ],
          },
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/trae/AGENTS.md",
            source_confidence: "official_docs",
            action_evidence: [
              { action: "inject", supported: true, evidence: "verified" },
            ],
          },
          {
            id: "mcp-config",
            kind: "mcp",
            scope: "global",
            access: "writable",
            format: "json",
            source_path: "/trae/mcp.json",
          },
          {
            id: "ordinary-config",
            kind: "ordinary_config",
            scope: "global",
            access: "writable",
            format: "json",
            source_path: "/trae/settings.json",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["trae-cn"]);
    apiMocks.getToolPaths.mockResolvedValue({
      "trae-cn": { config_dir: "/trae/config", skills_dir: "/trae/skills" },
    });
    apiMocks.getToolCapabilityOverrides.mockResolvedValue({
      "trae-cn": {
        customRuleSourceType: "directory",
        customRuleSourcePath: "/custom/rules",
        customGlobalRuleTarget: "/custom/rules/AGENTS.md",
        customMcpConfigPath: "/custom/mcp.json",
        customToolConfigPath: "/custom/settings.json",
      },
    });

    render(SettingsModule);

    await expandSettingsToolRow("trae-cn");
    expect(await screen.findByText("/custom/rules")).toBeInTheDocument();
    expect(screen.getByText("/custom/rules/AGENTS.md")).toBeInTheDocument();
    expect(screen.getByText("/custom/mcp.json")).toBeInTheDocument();
    expect(screen.getByText("/custom/settings.json")).toBeInTheDocument();
    const labels = Array.from(
      document.querySelectorAll('[data-settings-tool-id="trae-cn"] .st-field-label')
    ).map((label) => label.textContent.trim());
    expect(labels.slice(0, 6)).toEqual([
      "Rule Directory",
      "Global Rule File",
      "Skills Directory",
      "Shared Skills Support",
      "MCP Configuration",
      "Tool Configuration",
    ]);
    expect(screen.getAllByText("Custom").length).toBeGreaterThanOrEqual(4);

    await editSettingsToolRow("trae-cn");
    await fireEvent.input(screen.getByDisplayValue("/custom/rules/AGENTS.md"), {
      target: { value: "/outside/AGENTS.md" },
    });
    await fireEvent.input(screen.getByDisplayValue("/custom/mcp.json"), {
      target: { value: "/outside/mcp.json" },
    });
    await fireEvent.input(screen.getByDisplayValue("/custom/settings.json"), {
      target: { value: "/outside/settings.json" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(apiMocks.setToolCapabilityOverrides).toHaveBeenCalledWith("trae-cn", {
        customRuleSourceType: "directory",
        customRuleSourcePath: "/custom/rules",
        customGlobalRuleTarget: "/outside/AGENTS.md",
        customMcpConfigPath: "/outside/mcp.json",
        customToolConfigPath: "/outside/settings.json",
        sharedSkillDirectRead: null,
      });
    });
  });

  it("shows confirm and persists expanded custom tool edits", async () => {
    activeSettingsTab.set("tools");
    apiMocks.getCustomTools.mockResolvedValue([
      {
        id: "zed",
        name: "Zed",
        icon: "Z",
        config_dir: "/zed/config",
        rule_directory: "/zed/rules",
        global_rule_file: "/zed/rules-old.md",
        skills_dir: "/zed/skills",
        shared_skill_direct_read: false,
        mcp_config: "/zed/mcp.json",
        tool_config: "/zed/settings.json",
        rule_file: "/zed/rules-old.md",
      },
    ]);
    render(SettingsModule);

    await waitFor(() => {
      expect(screen.getByText("Zed")).toBeInTheDocument();
    });

    const editButtons = screen.getAllByRole("button", { name: "Edit" });
    await fireEvent.click(editButtons[editButtons.length - 1]);

    await fireEvent.input(screen.getByDisplayValue("/zed/rules"), { target: { value: "/zed/rules-new" } });
    await fireEvent.input(screen.getByDisplayValue("/zed/rules-old.md"), { target: { value: "/zed/rules-new.md" } });
    await fireEvent.input(screen.getByDisplayValue("/zed/mcp.json"), { target: { value: "/zed/mcp-new.json" } });
    await fireEvent.input(screen.getByDisplayValue("/zed/settings.json"), { target: { value: "/zed/settings-new.json" } });
    await fireEvent.click(within(settingsToolRow("zed")).getByRole("button", { name: "Direct" }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Confirm" })).toBeInTheDocument();
    });

    await fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(apiMocks.addCustomTool).toHaveBeenCalledWith({
        id: "zed",
        name: "Zed",
        icon: "Z",
        config_dir: "/zed/config",
        rule_directory: "/zed/rules-new",
        global_rule_file: "/zed/rules-new.md",
        skills_dir: "/zed/skills",
        shared_skill_direct_read: true,
        mcp_config: "/zed/mcp-new.json",
        tool_config: "/zed/settings-new.json",
        rule_file: "/zed/rules-new.md",
      });
    });

    const reopenedEditButtons = screen.getAllByRole("button", { name: "Edit" });
    await fireEvent.click(reopenedEditButtons[reopenedEditButtons.length - 1]);

    await waitFor(() => {
      expect(screen.queryByRole("button", { name: "Confirm" })).not.toBeInTheDocument();
    });
  });

  it("adds an edited custom tool to managed scope when a first source is configured", async () => {
    activeSettingsTab.set("tools");
    apiMocks.getCustomTools.mockResolvedValue([
      {
        id: "zed",
        name: "Zed",
        icon: "Z",
        config_dir: "",
        rule_directory: "",
        global_rule_file: "",
        skills_dir: "",
        shared_skill_direct_read: false,
        mcp_config: "",
        tool_config: "",
        rule_file: "",
      },
    ]);
    render(SettingsModule);

    await waitFor(() => {
      expect(screen.getByText("Zed")).toBeInTheDocument();
    });
    apiMocks.setManagedTools.mockClear();

    const editButtons = screen.getAllByRole("button", { name: "Edit" });
    await fireEvent.click(editButtons[editButtons.length - 1]);
    const row = settingsToolRow("zed");
    await fireEvent.input(within(row).getAllByRole("textbox")[0], { target: { value: "/zed/rules" } });
    await fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(apiMocks.addCustomTool).toHaveBeenCalledWith(expect.objectContaining({
        id: "zed",
        rule_directory: "/zed/rules",
      }));
    });
    expect(apiMocks.setManagedTools).toHaveBeenCalledWith(["cursor", "codex", "zed"]);
    expect(get(toolStores.managedToolIds)).toEqual(["cursor", "codex", "zed"]);
  });

  it("logs General settings operations", async () => {
    render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getToolPaths).toHaveBeenCalled();
    });

    await fireEvent.click(screen.getByRole("button", { name: "English" }));
    await fireEvent.click(screen.getByRole("button", { name: "Open Logs" }));

    await waitFor(() => {
      expect(apiMocks.listApplicationLogs).toHaveBeenCalled();
      expect(apiMocks.readApplicationLog).toHaveBeenCalledWith("app-2026-05-27.log");
      expect(loggerMocks.logAppEvent).toHaveBeenCalledWith(expect.objectContaining({
        category: "settings",
        action: "settings_language_update",
        result: "ok",
        targetRole: "language",
      }));
      expect(loggerMocks.logAppEvent).toHaveBeenCalledWith(expect.objectContaining({
        category: "settings",
        action: "settings_logs_open",
        result: "ok",
      }));
    });
  });

  it("places system first for appearance and language preference controls", async () => {
    const { container } = render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getToolPaths).toHaveBeenCalled();
    });

    const rows = Array.from(container.querySelectorAll(".settings-list-row"));
    const appearanceRow = rows.find((row) => row.textContent?.includes("Appearance"));
    const languageRow = rows.find((row) => row.textContent?.includes("Language"));

    expect(Array.from(appearanceRow.querySelectorAll("button")).map((button) => button.textContent?.trim())).toEqual(["System", "Light", "Dark"]);
    expect(Array.from(languageRow.querySelectorAll("button")).map((button) => button.textContent?.trim())).toEqual(["System", "中文", "English"]);
  });

  it("persists system language preference while resolving visible language", async () => {
    render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getToolPaths).toHaveBeenCalled();
    });

    const systemButtons = screen.getAllByRole("button", { name: "System" });
    await fireEvent.click(systemButtons[systemButtons.length - 1]);

    await waitFor(() => {
      expect(apiMocks.setLanguage).toHaveBeenCalledWith("system");
      expect(get(languagePreference)).toBe("system");
      expect(get(locale)).toMatch(/zh|en/);
    });
  });

  it("persists module performance diagnostics from General settings and opens its in-app log viewer", async () => {
    render(SettingsModule);

    await waitFor(() => {
      expect(apiMocks.getToolPaths).toHaveBeenCalled();
    });

    const toggle = screen.getByRole("checkbox", { name: "Enable module performance diagnostics" });
    expect(toggle).not.toBeChecked();
    expect(screen.queryByText("Performance diagnostics")).not.toBeInTheDocument();

    await fireEvent.click(toggle);

    await waitFor(() => {
      expect(apiMocks.setModulePerformanceDiagnosticsEnabled).toHaveBeenCalledWith(true);
      expect(loggerMocks.logAppEvent).toHaveBeenCalledWith(expect.objectContaining({
        category: "settings",
        action: "settings_module_performance_diagnostics_update",
        result: "ok",
        targetRole: "module-performance-diagnostics",
      }));
    });
    await waitFor(() => {
      expect(screen.getByText("Performance diagnostics")).toBeInTheDocument();
      expect(screen.getByText("Settings · General")).toBeInTheDocument();
      expect(get(modulePerformanceSummaries).settings).toMatchObject({
        module: "settings",
        reason: "diagnostics-toggle",
        status: "success",
      });
      expect(apiMocks.writeModulePerformanceLog).toHaveBeenCalledWith(expect.objectContaining({
        module: "settings",
        reason: "diagnostics-toggle",
        status: "success",
      }));
    });

    await fireEvent.click(screen.getByRole("button", { name: "Open Performance Log" }));

    await waitFor(() => {
      expect(apiMocks.listModulePerformanceLogs).toHaveBeenCalled();
      expect(apiMocks.readModulePerformanceLog).toHaveBeenCalledWith("module-performance-2026-05-27.log");
      expect(screen.getByText("Performance Logs")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Export" })).toBeInTheDocument();
      expect(loggerMocks.logAppEvent).toHaveBeenCalledWith(expect.objectContaining({
        category: "settings",
        action: "settings_module_performance_log_open",
        result: "ok",
      }));
    });
  });

  it("logs Tools settings operations without full config payloads", async () => {
    activeSettingsTab.set("tools");
    render(SettingsModule);

    await editSettingsToolRow("cursor");
    await fireEvent.input(screen.getByDisplayValue("/cursor/skills"), { target: { value: "/Users/leon/.cursor/password=secret/skills" } });
    await fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(loggerMocks.logAppEvent).toHaveBeenCalledWith(expect.objectContaining({
        category: "settings",
        action: "settings_tool_configuration_update",
        result: "ok",
        toolId: "cursor",
      }));
    });

    const toolConfigurationEvents = loggerMocks.logAppEvent.mock.calls
      .map(([event]) => event)
      .filter((event) => event.action === "settings_tool_configuration_update");
    expect(toolConfigurationEvents).toHaveLength(1);
    expect(toolConfigurationEvents[0]).toEqual(expect.objectContaining({ toolId: "cursor" }));
    expect(toolConfigurationEvents).not.toEqual(expect.arrayContaining([
      expect.objectContaining({ toolId: "codex" }),
    ]));

    const loggedEvents = loggerMocks.logAppEvent.mock.calls.map(([event]) => JSON.stringify(event));
    expect(loggedEvents.join("\n")).not.toContain("password=secret");
    expect(loggedEvents.join("\n")).not.toContain("/Users/leon/.cursor/password=secret/skills");
  });

  it("preserves settings operation success when logging fails", async () => {
    loggerMocks.logAppEvent.mockRejectedValue(new Error("logger unavailable"));
    activeSettingsTab.set("tools");
    render(SettingsModule);

    await expandSettingsToolRow("cursor");

    await fireEvent.click(screen.getAllByRole("checkbox")[0]);

    await waitFor(() => {
      expect(apiMocks.setManagedTools).toHaveBeenCalledWith(["codex"]);
    });
    expect(screen.getAllByText("/cursor/skills").length).toBeGreaterThanOrEqual(1);
  });
});
