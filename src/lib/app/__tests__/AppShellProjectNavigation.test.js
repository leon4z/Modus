// @ts-nocheck
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { readFileSync } from "node:fs";
import { get } from "svelte/store";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const settingsMocks = vi.hoisted(() => ({
  getLanguage: vi.fn(),
  getModulePerformanceDiagnosticsEnabled: vi.fn(),
  getTheme: vi.fn(),
  isInitialized: vi.fn(),
}));

const toolApiMocks = vi.hoisted(() => ({
  getHandledNewToolIds: vi.fn(),
  getManagedTools: vi.fn(),
  loadTools: vi.fn(),
  setHandledNewToolIds: vi.fn(),
  setManagedTools: vi.fn(),
}));

const skillInventoryMocks = vi.hoisted(() => ({
  DEFAULT_SKILL_INVENTORY_TTL_MS: 60_000,
  prewarmSkillInventory: vi.fn(),
}));

const appUpdateMocks = vi.hoisted(() => ({
  loadAppUpdateState: vi.fn(),
  runAppUpdateCheck: vi.fn(),
}));

vi.mock("$lib/features/settings/index.js", () => settingsMocks);
vi.mock("$lib/features/tools/index.js", async () => {
  const { derived, writable } = await import("svelte/store");
  const tools = writable([]);
  const activeToolId = writable(null);
  const activeView = writable("dashboard");
  const detectedTools = derived(tools, ($tools) => $tools.filter((tool) => tool.detected));
  const managedToolIds = writable([]);
  const managedTools = derived(
    [detectedTools, managedToolIds],
    ([$detectedTools, $managedToolIds]) => $detectedTools.filter((tool) => $managedToolIds.includes(tool.id))
  );

  return {
    ...toolApiMocks,
    tools,
    activeToolId,
    activeView,
    detectedTools,
    managedTools,
    managedToolIds,
    appInitialized: writable(true),
    pendingSubTab: writable(null),
    theme: writable("system"),
  };
});
vi.mock("$lib/features/skills/queries/skillInventoryQuery.js", () => skillInventoryMocks);
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
  appUpdateMocks.runAppUpdateCheck.mockImplementation(async () => get(appUpdateState));
  return {
    appUpdateAvailable,
    appUpdateState,
    loadAppUpdateState: appUpdateMocks.loadAppUpdateState,
    runAppUpdateCheck: appUpdateMocks.runAppUpdateCheck,
  };
});
vi.mock("$lib/features/dashboard/components/Dashboard.svelte", async () => {
  const mod = await import("../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});
vi.mock("$lib/features/rules/components/RulesModule.svelte", async () => {
  const mod = await import("../../../test/stubs/RefreshableStub.svelte");
  return { default: mod.default };
});
vi.mock("$lib/features/skills/components/SkillsModule.svelte", async () => {
  const mod = await import("../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});
vi.mock("$lib/features/config/components/ConfigPanel.svelte", async () => {
  const mod = await import("../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});
vi.mock("$lib/features/mcp/components/McpPanel.svelte", async () => {
  const mod = await import("../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});
vi.mock("$lib/features/settings/components/SettingsModule.svelte", async () => {
  const mod = await import("../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});
vi.mock("$lib/dev/components/DesignPreview.svelte", async () => {
  const mod = await import("../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});
vi.mock("$lib/shared/components/ConfirmDialog.svelte", async () => {
  const mod = await import("../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});
vi.mock("$lib/shared/components/ToolIcon.svelte", async () => {
  const mod = await import("../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});

import AppShell from "$lib/app/AppShell.svelte";
import { appUpdateState } from "$lib/features/appUpdates/index.js";
import { activeView, tools } from "$lib/features/tools/index.js";
import { setModulePerformanceDiagnosticsEnabledState } from "$lib/shared/diagnostics/modulePerformance.js";
import { locale } from "$lib/shared/i18n/index.js";

function installLocalStorageStub(seed = {}) {
  const storage = new Map(Object.entries(seed));
  vi.stubGlobal("localStorage", {
    getItem: vi.fn((key) => storage.has(key) ? storage.get(key) : null),
    setItem: vi.fn((key, value) => storage.set(key, String(value))),
  });
  return storage;
}

describe("AppShell Community navigation", () => {
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

  beforeEach(() => {
    installLocalStorageStub({
      "modus.sidebar.collapsed": "false",
      "modus.sidebar.width": "200",
    });
    locale.set("en");
    activeView.set("dashboard");
    tools.set([]);
    setModulePerformanceDiagnosticsEnabledState(false);
    settingsMocks.getLanguage.mockResolvedValue("en");
    settingsMocks.getModulePerformanceDiagnosticsEnabled.mockResolvedValue(false);
    settingsMocks.getTheme.mockResolvedValue("system");
    settingsMocks.isInitialized.mockResolvedValue(true);
    toolApiMocks.getManagedTools.mockResolvedValue([]);
    toolApiMocks.getHandledNewToolIds.mockResolvedValue([]);
    toolApiMocks.loadTools.mockResolvedValue(undefined);
    toolApiMocks.setManagedTools.mockResolvedValue(undefined);
    toolApiMocks.setHandledNewToolIds.mockResolvedValue(undefined);
    skillInventoryMocks.prewarmSkillInventory.mockResolvedValue(null);
    appUpdateState.set(defaultAppUpdateState());
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.restoreAllMocks();
  });

  it("omits Project navigation and redirects stale Project state", async () => {
    activeView.set("project");
    render(AppShell);

    await waitFor(() => {
      expect(screen.getByText("Modus")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Project" })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /Pro|Upgrade|Paid/i })).not.toBeInTheDocument();
    });

    await waitFor(() => {
      expect(get(activeView)).toBe("dashboard");
    });
  });

  it("prewarms Skill inventory after initialized startup", async () => {
    render(AppShell);

    await waitFor(() => {
      expect(skillInventoryMocks.prewarmSkillInventory).toHaveBeenCalledTimes(1);
    });
  });

  it("checks for app updates after startup without blocking navigation", async () => {
    render(AppShell);

    await waitFor(() => {
      expect(screen.getByText("Modus")).toBeInTheDocument();
      expect(appUpdateMocks.loadAppUpdateState).toHaveBeenCalled();
    });
    await waitFor(() => {
      expect(appUpdateMocks.runAppUpdateCheck).toHaveBeenCalledWith("startup");
    }, { timeout: 2000 });
  });

  it("shows a Settings navigation update tag when an app update is available", async () => {
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

    render(AppShell);

    await waitFor(() => {
      expect(screen.getByText("Update")).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: "Settings" })).toBeInTheDocument();
  });

  it("collapses the sidebar behind a toolbar restore control without changing the active section", async () => {
    const user = userEvent.setup();
    activeView.set("rules");

    render(AppShell);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Collapse sidebar" })).toBeInTheDocument();
    });

    const collapseToggle = screen.getByRole("button", { name: "Collapse sidebar" });
    expect(collapseToggle).toHaveClass("refresh-btn");
    expect(collapseToggle).not.toHaveAttribute("title");
    expect(document.querySelector(".app")?.getAttribute("style")).toContain("--current-sidebar-width: 200px");

    const sidebar = document.querySelector(".sidebar");
    expect(sidebar).not.toHaveClass("collapsed");
    expect(screen.getByRole("button", { name: "Rules" })).toHaveClass("active");

    await user.click(collapseToggle);

    expect(sidebar).toHaveClass("collapsed");
    expect(localStorage.getItem("modus.sidebar.collapsed")).toBe("true");
    expect(document.querySelector(".app")?.getAttribute("style")).toContain("--current-sidebar-width: 0px");
    expect(screen.queryByRole("button", { name: "Rules" })).not.toBeInTheDocument();
    expect(get(activeView)).toBe("rules");
    expect(screen.getByRole("button", { name: "Expand sidebar" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Settings" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Expand sidebar" }));

    expect(sidebar).not.toHaveClass("collapsed");
    expect(localStorage.getItem("modus.sidebar.collapsed")).toBe("false");
    expect(get(activeView)).toBe("rules");
    expect(screen.getByRole("button", { name: "Rules" })).toHaveClass("active");
  });

  it("routes the platform find shortcut to the active module search", async () => {
    activeView.set("rules");

    render(AppShell);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Run refresh" })).toBeInTheDocument();
    });

    const isMac = /Mac|iPhone|iPad|iPod/.test(navigator.platform);
    await fireEvent.keyDown(window, {
      key: "f",
      code: "KeyF",
      ctrlKey: !isMac,
      metaKey: isMac,
    });

    await waitFor(() => {
      expect(screen.getByLabelText("Stub module search")).toHaveFocus();
    });
  });

  it("localizes the Skills sidebar item in Chinese", async () => {
    settingsMocks.getLanguage.mockResolvedValue("zh");
    activeView.set("skills");

    render(AppShell);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "技能" })).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: "技能" })).toHaveClass("active");
    expect(screen.queryByRole("button", { name: "Skills" })).not.toBeInTheDocument();
  });

  it("keeps sidebar restore motion lateral by not animating top padding", () => {
    const source = readFileSync("src/lib/app/components/Sidebar.svelte", "utf8");

    expect(source).toContain("transition: width 0.42s cubic-bezier(0.22, 1, 0.36, 1);");
    expect(source).not.toContain("transition: width 0.42s cubic-bezier(0.22, 1, 0.36, 1), padding");
    expect(source).toContain(".sidebar.collapsed {\n    padding: 0;\n    overflow: visible;\n  }");
  });

  it("shows the primary config directory in first-run onboarding", async () => {
    settingsMocks.isInitialized.mockResolvedValue(false);
    toolApiMocks.getManagedTools.mockResolvedValue(["codex"]);
    tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/Users/leon/.codex",
        primary_config_dir: "/Users/leon/.codex",
      },
      {
        id: "trae-solo-cn",
        name: "Trae Solo CN",
        detected: true,
        config_dir: "/Users/leon/.trae-cn",
        primary_config_dir: "/Users/leon/.trae-cn",
      },
    ]);

    render(AppShell);

    await waitFor(() => {
      expect(screen.getByText("Choose tools to manage")).toBeInTheDocument();
    });
    expect(screen.getByText("Trae Solo CN")).toBeInTheDocument();
    expect(screen.getByText("/Users/leon/.trae-cn")).toBeInTheDocument();
  });

  it("records first-run detected tools as handled when onboarding completes", async () => {
    const user = userEvent.setup();
    settingsMocks.isInitialized.mockResolvedValue(false);
    tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/Users/leon/.codex",
        primary_config_dir: "/Users/leon/.codex",
      },
      {
        id: "trae-solo-cn",
        name: "Trae Solo CN",
        detected: true,
        config_dir: "/Users/leon/.trae-cn",
        primary_config_dir: "/Users/leon/.trae-cn",
      },
    ]);

    render(AppShell);

    await screen.findByText("Choose tools to manage");
    await user.click(screen.getByRole("button", { name: "Start" }));

    await waitFor(() => {
      expect(toolApiMocks.setManagedTools).toHaveBeenCalledWith(["codex", "trae-solo-cn"]);
    });
    expect(toolApiMocks.setHandledNewToolIds).toHaveBeenCalledWith(["codex", "trae-solo-cn"]);
  });

  it("allows first-run onboarding to be skipped into an empty managed scope", async () => {
    const user = userEvent.setup();
    settingsMocks.isInitialized.mockResolvedValue(false);
    tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/Users/leon/.codex",
        primary_config_dir: "/Users/leon/.codex",
      },
      {
        id: "trae-solo-cn",
        name: "Trae Solo CN",
        detected: true,
        config_dir: "/Users/leon/.trae-cn",
        primary_config_dir: "/Users/leon/.trae-cn",
      },
    ]);

    render(AppShell);

    await screen.findByText("Choose tools to manage");
    await user.click(screen.getByRole("button", { name: "Skip" }));

    await waitFor(() => {
      expect(toolApiMocks.setManagedTools).toHaveBeenCalledWith([]);
    });
    expect(toolApiMocks.setHandledNewToolIds).toHaveBeenCalledWith(["codex", "trae-solo-cn"]);
  });

  it("prompts to enable unhandled detected tools after onboarding", async () => {
    toolApiMocks.getManagedTools.mockResolvedValue(["codex"]);
    tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/Users/leon/.codex",
        primary_config_dir: "/Users/leon/.codex",
      },
      {
        id: "trae-solo-cn",
        name: "Trae Solo CN",
        detected: true,
        config_dir: "/Users/leon/.trae-cn",
        primary_config_dir: "/Users/leon/.trae-cn",
      },
    ]);

    render(AppShell);

    await waitFor(() => {
      expect(screen.getByText("New Tools Detected")).toBeInTheDocument();
    });
    expect(screen.getByText("Trae Solo CN")).toBeInTheDocument();
    expect(screen.getByText("/Users/leon/.trae-cn")).toBeInTheDocument();
  });

  it("prompts for unhandled detected tools after manual refresh", async () => {
    const user = userEvent.setup();
    const codex = {
      id: "codex",
      name: "Codex",
      detected: true,
      config_dir: "/Users/leon/.codex",
      primary_config_dir: "/Users/leon/.codex",
    };
    const traeSoloCn = {
      id: "trae-solo-cn",
      name: "Trae Solo CN",
      detected: true,
      config_dir: "/Users/leon/.trae-cn",
      primary_config_dir: "/Users/leon/.trae-cn",
    };
    activeView.set("rules");
    toolApiMocks.getManagedTools.mockResolvedValue(["codex"]);
    tools.set([codex]);
    toolApiMocks.loadTools
      .mockResolvedValueOnce([codex])
      .mockImplementationOnce(async () => {
        const refreshed = [codex, traeSoloCn];
        tools.set(refreshed);
        return refreshed;
      });

    render(AppShell);

    await screen.findByRole("button", { name: "Run refresh" });
    expect(screen.queryByText("New Tools Detected")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Run refresh" }));

    await waitFor(() => {
      expect(screen.getByText("New Tools Detected")).toBeInTheDocument();
    });
    expect(screen.getByText("Trae Solo CN")).toBeInTheDocument();
  });

  it("prompts for unhandled detected tools after the window regains focus", async () => {
    const nowSpy = vi.spyOn(Date, "now").mockReturnValue(1000);
    const codex = {
      id: "codex",
      name: "Codex",
      detected: true,
      config_dir: "/Users/leon/.codex",
      primary_config_dir: "/Users/leon/.codex",
    };
    const traeSoloCn = {
      id: "trae-solo-cn",
      name: "Trae Solo CN",
      detected: true,
      config_dir: "/Users/leon/.trae-cn",
      primary_config_dir: "/Users/leon/.trae-cn",
    };
    toolApiMocks.getManagedTools.mockResolvedValue(["codex"]);
    tools.set([codex]);
    toolApiMocks.loadTools
      .mockResolvedValueOnce([codex])
      .mockImplementationOnce(async () => {
        const refreshed = [codex, traeSoloCn];
        tools.set(refreshed);
        return refreshed;
      });

    render(AppShell);

    await waitFor(() => {
      expect(screen.getByText("Modus")).toBeInTheDocument();
    });
    expect(screen.queryByText("New Tools Detected")).not.toBeInTheDocument();

    nowSpy.mockReturnValue(32000);
    window.dispatchEvent(new Event("focus"));

    await waitFor(() => {
      expect(screen.getByText("New Tools Detected")).toBeInTheDocument();
    });
    expect(screen.getByText("Trae Solo CN")).toBeInTheDocument();
  });

  it("records skipped newly detected tools without enabling them", async () => {
    const user = userEvent.setup();
    toolApiMocks.getManagedTools.mockResolvedValue(["codex"]);
    tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/Users/leon/.codex",
        primary_config_dir: "/Users/leon/.codex",
      },
      {
        id: "trae-solo-cn",
        name: "Trae Solo CN",
        detected: true,
        config_dir: "/Users/leon/.trae-cn",
        primary_config_dir: "/Users/leon/.trae-cn",
      },
    ]);

    render(AppShell);

    await screen.findByText("New Tools Detected");
    await user.click(screen.getByRole("button", { name: "Skip" }));

    await waitFor(() => {
      expect(toolApiMocks.setHandledNewToolIds).toHaveBeenCalledWith(["trae-solo-cn"]);
    });
    expect(toolApiMocks.setManagedTools).not.toHaveBeenCalled();
    expect(screen.queryByText("New Tools Detected")).not.toBeInTheDocument();
  });

  it("enables selected newly detected tools and records the prompt as handled", async () => {
    const user = userEvent.setup();
    toolApiMocks.getManagedTools.mockResolvedValue(["codex"]);
    tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/Users/leon/.codex",
        primary_config_dir: "/Users/leon/.codex",
      },
      {
        id: "trae-solo-cn",
        name: "Trae Solo CN",
        detected: true,
        config_dir: "/Users/leon/.trae-cn",
        primary_config_dir: "/Users/leon/.trae-cn",
      },
    ]);

    render(AppShell);

    await screen.findByText("New Tools Detected");
    await user.click(screen.getByRole("button", { name: "Confirm Enable" }));

    await waitFor(() => {
      expect(toolApiMocks.setManagedTools).toHaveBeenCalledWith(["codex", "trae-solo-cn"]);
    });
    expect(toolApiMocks.setHandledNewToolIds).toHaveBeenCalledWith(["trae-solo-cn"]);
    expect(screen.queryByText("New Tools Detected")).not.toBeInTheDocument();
  });

  it("does not prompt handled detected tools after onboarding", async () => {
    toolApiMocks.getManagedTools.mockResolvedValue(["codex"]);
    toolApiMocks.getHandledNewToolIds.mockResolvedValue(["trae-solo-cn"]);
    tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        config_dir: "/Users/leon/.codex",
        primary_config_dir: "/Users/leon/.codex",
      },
      {
        id: "trae-solo-cn",
        name: "Trae Solo CN",
        detected: true,
        config_dir: "/Users/leon/.trae-cn",
        primary_config_dir: "/Users/leon/.trae-cn",
      },
    ]);

    render(AppShell);

    await waitFor(() => {
      expect(screen.getByText("Modus")).toBeInTheDocument();
    });
    expect(screen.queryByText("New Tools Detected")).not.toBeInTheDocument();
    expect(screen.queryByText("Trae Solo CN")).not.toBeInTheDocument();
  });
});
