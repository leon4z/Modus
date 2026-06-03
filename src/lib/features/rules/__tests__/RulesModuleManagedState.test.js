// @ts-nocheck
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { readFileSync } from "node:fs";
import { get } from "svelte/store";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

async function replaceSourceEditorText(user, text) {
  const editor = screen.getByRole("textbox");
  await user.click(editor);
  await user.keyboard("{Control>}a{/Control}");
  await user.paste(text);
  return editor;
}

const rulesApiMocks = vi.hoisted(() => ({
  createRuleFile: vi.fn(),
  createRuleDirectory: vi.fn(),
  deleteRuleEntry: vi.fn(),
  deleteDefaultRule: vi.fn(),
  diffRules: vi.fn(),
  getDefaultRuleInjectionBaselines: vi.fn(),
  getManagedRulesState: vi.fn(),
  injectDefaultRules: vi.fn(),
  leaveRuleManagementTargets: vi.fn(),
  listDefaultRules: vi.fn(),
  readRuleContent: vi.fn(),
  renameRuleEntry: vi.fn(),
  saveDefaultRule: vi.fn(),
  setDefaultRuleInjectionBaselines: vi.fn(),
  syncManagedRuleTargets: vi.fn(),
  writeRule: vi.fn(),
}));
const loggerMocks = vi.hoisted(() => ({
  logAppEvent: vi.fn(),
}));
const toolStoreMocks = vi.hoisted(() => ({
  loadTools: vi.fn(),
}));
const toolApiMocks = vi.hoisted(() => ({
  getInjectionTargets: vi.fn(),
  setToolCapabilityOverrides: vi.fn(),
}));
vi.mock("$lib/features/rules/api/rules.js", () => rulesApiMocks);
vi.mock("$lib/shared/logging/appLogger.js", () => loggerMocks);
vi.mock("$lib/features/tools/api/tools.js", () => toolApiMocks);
vi.mock("$lib/features/tools/index.js", async (importOriginal) => {
  const { derived, writable } = await import("svelte/store");
  const toolFeature = await importOriginal();
  const activeToolId = writable("codex");
  const activeRulesTab = writable("global");
  const pendingSubTab = writable(null);
  const managedToolIds = writable([]);
  const rawTools = writable([]);
  function setToolsAndDefaultManaged(value) {
    const next = Array.isArray(value) ? value : [];
    rawTools.set(next);
    managedToolIds.set(next.filter((tool) => tool.detected).map((tool) => tool.id));
  }
  const tools = {
    subscribe: rawTools.subscribe,
    set: setToolsAndDefaultManaged,
    update(updater) {
      rawTools.update((current) => {
        const next = updater(current);
        managedToolIds.set((Array.isArray(next) ? next : []).filter((tool) => tool.detected).map((tool) => tool.id));
        return next;
      });
    },
  };
  const detectedTools = derived(rawTools, ($tools) => $tools.filter((tool) => tool.detected));
  const managedTools = derived(
    [detectedTools, managedToolIds],
    ([$detected, $managed]) => $detected.filter((tool) => $managed.includes(tool.id)),
  );
  const activeTool = derived(
    [tools, activeToolId],
    ([$tools, $activeToolId]) => $tools.find((tool) => tool.id === $activeToolId) || null,
  );
  return {
    ...toolFeature,
    tools,
    activeTool,
    activeToolId,
    activeRulesTab,
    pendingSubTab,
    managedToolIds,
    managedTools,
    detectedTools,
    loadTools: toolStoreMocks.loadTools,
    getInjectionTargets: toolApiMocks.getInjectionTargets,
    setToolCapabilityOverrides: toolApiMocks.setToolCapabilityOverrides,
    getToolName(toolId, toolsList = []) {
      return toolsList.find((tool) => tool.id === toolId)?.name || toolId;
    },
  };
});
vi.mock("$lib/shared/components/ToolIcon.svelte", async () => {
  const mod = await import("../../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});

import RulesModule from "$lib/features/rules/components/RulesModule.svelte";
import { defaultRuleFingerprint, scopedDefaultRuleFingerprint } from "$lib/features/rules/domain/ruleInjectionState.js";
import {
  modulePerformanceSummaries,
  setModulePerformanceDiagnosticsEnabledState,
} from "$lib/shared/diagnostics/modulePerformance.js";
import { locale } from "$lib/shared/i18n/index.js";
import * as toolStores from "$lib/features/tools/index.js";

const driftState = {
  rule_sets: [
    {
      rule_id: "common_rule",
      rule_name: "Global Rules",
      managed_tool_ids: ["codex"],
      source_pending: false,
    },
  ],
  targets: [
    {
      tool_id: "codex",
      tool_name: "Codex",
      target_path: "/codex/AGENTS.md",
      rule_set_ids: ["common_rule"],
      rule_set_names: ["Global Rules"],
      classification: "drifted",
      reason: "managed_block_drift",
      can_read: true,
      can_write: true,
      source_pending: false,
      has_managed_block: true,
      expected_block: "expected",
      current_block: "current",
    },
  ],
  unmanaged_tool_rule_count: 1,
  summary: {
    managed_rule_sets: 1,
    managed_targets: 1,
    in_sync_targets: 0,
    requires_sync_targets: 0,
    drifted_targets: 1,
    unresolved_targets: 0,
    pending_source_rule_sets: 0,
    affected_tool_ids: ["codex"],
  },
};

const cleanState = {
  ...driftState,
  targets: [{ ...driftState.targets[0], classification: "in_sync", reason: "in_sync" }],
  summary: {
    ...driftState.summary,
    in_sync_targets: 1,
    drifted_targets: 0,
    affected_tool_ids: [],
  },
};

function installLocalStorageStub(seed = {}) {
  const storage = new Map(Object.entries(seed));
  vi.stubGlobal("localStorage", {
    getItem: vi.fn((key) => storage.has(key) ? storage.get(key) : null),
    setItem: vi.fn((key, value) => storage.set(key, String(value))),
  });
  return storage;
}

describe("RulesModule managed state", () => {
  let defaultRulesState;
  let baselineState;
  let managedRulesStateResponse;

  beforeEach(() => {
    installLocalStorageStub({
      "modus.rules.toolFileNavigation.collapsed": "false",
    });
    setModulePerformanceDiagnosticsEnabledState(false);
    locale.set("en");
    toolStores.tools.set([
      { id: "codex", name: "Codex", detected: true, rule_sources: [] },
    ]);
    toolStores.managedToolIds.set(["codex"]);
    toolStores.activeToolId.set("codex");
    toolStores.activeRulesTab.set("global");
    defaultRulesState = [
      {
        id: "common_rule",
        name: "Global Rules",
        content: "expected",
        inject_to: [],
        managed_targets: ["codex"],
      },
    ];
    baselineState = {
      common_rule: "v1:current",
      custom_rules: {},
      custom_rule_pending_targets: {},
    };
    managedRulesStateResponse = driftState;
    rulesApiMocks.listDefaultRules.mockImplementation(() => Promise.resolve(defaultRulesState));
    rulesApiMocks.getDefaultRuleInjectionBaselines.mockImplementation(() => Promise.resolve(baselineState));
    rulesApiMocks.setDefaultRuleInjectionBaselines.mockImplementation((next) => {
      baselineState = next;
      return Promise.resolve(true);
    });
    rulesApiMocks.saveDefaultRule.mockImplementation((rule) => {
      defaultRulesState = defaultRulesState.map((item) => item.id === rule.id ? { ...item, ...rule } : item);
      return Promise.resolve(true);
    });
    rulesApiMocks.injectDefaultRules.mockResolvedValue(true);
    toolApiMocks.getInjectionTargets.mockResolvedValue({ codex: "/codex/AGENTS.md" });
    rulesApiMocks.getManagedRulesState.mockImplementation(() => Promise.resolve(managedRulesStateResponse));
    rulesApiMocks.diffRules.mockResolvedValue({
      left_label: "Rules in Codex file",
      right_label: "Rules in app",
      changes: [
        { tag: "equal", content: "unchanged before 1" },
        { tag: "equal", content: "unchanged before 2" },
        { tag: "equal", content: "unchanged before 3" },
        { tag: "delete", content: "current" },
        { tag: "insert", content: "expected" },
        { tag: "equal", content: "unchanged after 1" },
        { tag: "equal", content: "unchanged after 2" },
        { tag: "equal", content: "unchanged after 3" },
      ],
    });
    rulesApiMocks.syncManagedRuleTargets.mockImplementation(() => {
      managedRulesStateResponse = cleanState;
      return Promise.resolve({
        requested_tool_ids: ["codex"],
        succeeded_tool_ids: ["codex"],
        failed: [],
        state: cleanState,
      });
    });
    rulesApiMocks.leaveRuleManagementTargets.mockImplementation(() => {
      const state = { ...cleanState, rule_sets: [{ ...cleanState.rule_sets[0], managed_tool_ids: [] }], targets: [] };
      managedRulesStateResponse = state;
      return Promise.resolve({
        requested_tool_ids: ["codex"],
        succeeded_tool_ids: ["codex"],
        failed: [],
        state,
      });
    });
    rulesApiMocks.writeRule.mockResolvedValue(undefined);
    rulesApiMocks.createRuleFile.mockResolvedValue(undefined);
    rulesApiMocks.createRuleDirectory.mockResolvedValue(undefined);
    rulesApiMocks.deleteRuleEntry.mockResolvedValue({ deletes: [], changes: [] });
    rulesApiMocks.renameRuleEntry.mockResolvedValue("/codex/rules/renamed.md");
    toolStoreMocks.loadTools.mockResolvedValue(undefined);
    toolApiMocks.setToolCapabilityOverrides.mockResolvedValue(true);
    loggerMocks.logAppEvent.mockResolvedValue(undefined);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("switches the Tool Rules tab away from a disabled active tool", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [{ label: "AGENTS.md", path: "/codex/AGENTS.md", content: "# Codex" }],
      },
      {
        id: "cursor",
        name: "Cursor",
        detected: true,
        rule_sources: [{ label: "CURSOR.md", path: "/cursor/CURSOR.md", content: "# Cursor" }],
      },
    ]);
    toolStores.managedToolIds.set(["codex"]);
    toolStores.activeToolId.set("cursor");

    render(RulesModule);

    await waitFor(() => {
      expect(get(toolStores.activeToolId)).toBe("codex");
      expect(document.querySelector(".file-viewer-title")?.textContent).toBe("AGENTS.md");
    });
    expect(screen.queryByLabelText("Module performance diagnostics")).not.toBeInTheDocument();
    expect(get(modulePerformanceSummaries)).toEqual({});
  });

  it("shows an empty Tool Rules state when every detected tool is disabled", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [{ label: "AGENTS.md", path: "/codex/AGENTS.md", content: "# Codex" }],
      },
    ]);
    toolStores.managedToolIds.set([]);
    toolStores.activeToolId.set("codex");

    render(RulesModule);

    expect(await screen.findByText("No enabled tools")).toBeInTheDocument();
    expect(document.querySelector(".file-viewer-title")).toBeNull();
  });

  it("searches the current global rule content from the header search", async () => {
    defaultRulesState = [{
      ...defaultRulesState[0],
      content: "# Decision Boundary\n\nUse decisions carefully.",
    }];
    baselineState = {
      common_rule: defaultRuleFingerprint(defaultRulesState[0]),
      custom_rules: {},
      custom_rule_pending_targets: {},
    };

    const { container } = render(RulesModule, { searchOpen: true, globalSearchQuery: "decision" });

    expect(await screen.findByPlaceholderText("Search current content")).toBeInTheDocument();
    expect(container.querySelector(".view-fixed-header")).not.toHaveClass("view-fixed-header--search-popover");
    await waitFor(() => {
      expect(document.querySelector(".workspace-search-highlight--active")?.textContent?.toLowerCase()).toBe("decision");
    });
    expect(screen.queryByRole("region", { name: "Current file tree search results" })).not.toBeInTheDocument();
  });

  it("clears the header search when switching between Rules pages", async () => {
    const user = userEvent.setup();
    defaultRulesState = [{
      ...defaultRulesState[0],
      content: "# Decision Boundary\n\nUse decisions carefully.",
    }];

    render(RulesModule, { searchOpen: true, globalSearchQuery: "decision" });

    expect(await screen.findByPlaceholderText("Search current content")).toHaveValue("decision");
    await user.click(screen.getByRole("button", { name: "Tools" }));

    expect(screen.queryByPlaceholderText("Search current content")).not.toBeInTheDocument();
    expect(screen.queryByPlaceholderText("Search current file tree")).not.toBeInTheDocument();
    expect(screen.queryByDisplayValue("decision")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Search" }));
    expect(await screen.findByPlaceholderText("Search current file tree")).toHaveValue("");
  });

  it("shows one update banner and opens an injection preview for writable targets", async () => {
    const user = userEvent.setup();
    baselineState = {
      common_rule: defaultRuleFingerprint(defaultRulesState[0]),
      custom_rules: {},
      custom_rule_pending_targets: {},
    };
    rulesApiMocks.injectDefaultRules.mockImplementation(() => {
      managedRulesStateResponse = cleanState;
      return Promise.resolve(true);
    });

    render(RulesModule);

    await waitFor(() => {
      expect(screen.getByText("Tool rules need update")).toBeInTheDocument();
      expect(screen.queryByText("Managed block drifted")).not.toBeInTheDocument();
    });
    expect(screen.queryByText("Affects 1 tool")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "View details" }));
    await user.click(screen.getByRole("button", { name: /Codex.*Needs update/ }));
    await waitFor(() => {
      expect(rulesApiMocks.diffRules).toHaveBeenCalledWith(
        "current",
        "Rules in Codex file",
        "expected",
        "Rules in app",
      );
      expect(screen.getAllByText("expected").length).toBeGreaterThan(0);
      expect(screen.getByText("current")).toBeInTheDocument();
      expect(screen.queryByText("Rules in app")).not.toBeInTheDocument();
      expect(screen.queryByText("Rules in Codex file")).not.toBeInTheDocument();
      expect(screen.queryByText("unchanged before 1")).not.toBeInTheDocument();
      expect(screen.getAllByText("...").length).toBeGreaterThan(0);
    });
    expect(screen.getByRole("button", { name: "Hide details" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Inject" }));
    const modalTitle = await screen.findByText("Inject Rules");
    const modal = modalTitle.closest(".modal");
    await waitFor(() => {
      expect(within(modal).getByText("Changes")).toBeInTheDocument();
      expect(within(modal).getByText("/codex/AGENTS.md")).toBeInTheDocument();
    });
    expect(within(modal).getByText("/codex/AGENTS.md")).not.toHaveAttribute("title");

    await user.click(within(modal).getByRole("button", { name: "Inject" }));

    await waitFor(() => {
      expect(rulesApiMocks.injectDefaultRules).toHaveBeenCalledWith("codex");
      expect(rulesApiMocks.syncManagedRuleTargets).not.toHaveBeenCalled();
      expect(screen.getByText("Injection complete: injected 1 tools")).toBeInTheDocument();
      expect(screen.queryByText("Tool rules need update")).not.toBeInTheDocument();
      expect(screen.queryByText("Needs update")).not.toBeInTheDocument();
    });
  });

  it("keeps pending global rule updates scoped to the rule targets", async () => {
    const user = userEvent.setup();
    baselineState = {
      common_rule: "outdated",
      custom_rules: {},
      custom_rule_pending_targets: {},
    };
    toolStores.tools.set([
      { id: "codex", name: "Codex", detected: true, rule_sources: [] },
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "user-rules",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/claude/CLAUDE.md",
            label: "CLAUDE.md",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["codex", "claude_code"]);

    render(RulesModule);

    expect(await screen.findByText("Tool rules need update")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "View details" }));

    expect(screen.getByRole("button", { name: /Codex.*Needs update/ })).toBeInTheDocument();
    expect(screen.queryByText("Claude Code")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Inject" }));
    const modal = (await screen.findByText("Inject Rules")).closest(".modal");
    expect(within(modal).getByText("/codex/AGENTS.md")).toBeInTheDocument();
    expect(within(modal).queryByText("Claude Code")).not.toBeInTheDocument();
  });

  it("does not show pending global updates for disabled historical targets", async () => {
    managedRulesStateResponse = cleanState;
    defaultRulesState = [
      {
        ...defaultRulesState[0],
        managed_targets: ["codex", "disabled-tool"],
      },
    ];
    baselineState = {
      common_rule: scopedDefaultRuleFingerprint(defaultRulesState[0], ["codex"]),
      custom_rules: {},
      custom_rule_pending_targets: {},
    };

    render(RulesModule);

    expect(await screen.findByText("Global Rules")).toBeInTheDocument();
    await waitFor(() => {
      expect(rulesApiMocks.getManagedRulesState).toHaveBeenCalled();
    });
    expect(screen.queryByText("Tool rules need update")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Inject" })).not.toBeInTheDocument();
  });

  it("keeps fileless tools out of pending global rule updates", async () => {
    managedRulesStateResponse = {
      ...cleanState,
      targets: [],
      summary: {
        ...cleanState.summary,
        managed_targets: 0,
        in_sync_targets: 0,
        affected_tool_ids: [],
      },
    };
    baselineState = {
      common_rule: "outdated",
      custom_rules: {},
      custom_rule_pending_targets: {},
    };
    toolStores.tools.set([
      {
        id: "cursor",
        name: "Cursor",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "user-rules",
            kind: "rule",
            scope: "global",
            access: "unsupported",
            format: "unknown",
            source_path: "",
            label: "User Rules",
          },
        ],
      },
      {
        id: "trae",
        name: "Trae",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "user-rules",
            kind: "rule",
            scope: "global",
            access: "unsupported",
            format: "unknown",
            source_path: "",
            label: "User Rules",
            source_confidence: "official_community",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["cursor", "trae"]);

    render(RulesModule);

    expect(await screen.findByText("Global Rules")).toBeInTheDocument();
    await waitFor(() => {
      expect(rulesApiMocks.getManagedRulesState).toHaveBeenCalled();
    });
    expect(screen.queryByText("Tool rules need update")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Inject" })).not.toBeInTheDocument();
    expect(screen.queryByText("Cursor")).not.toBeInTheDocument();
    expect(screen.queryByText("Trae")).not.toBeInTheDocument();
  });

  it("keeps directory-only global rule locations out of pending global injection", async () => {
    managedRulesStateResponse = {
      ...cleanState,
      targets: [],
      summary: {
        ...cleanState.summary,
        managed_targets: 0,
        in_sync_targets: 0,
        affected_tool_ids: [],
      },
    };
    defaultRulesState = [
      {
        ...defaultRulesState[0],
        managed_targets: ["claude_code"],
      },
    ];
    baselineState = {
      common_rule: "outdated",
      custom_rules: {},
      custom_rule_pending_targets: {},
    };
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "user-rules",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["claude_code"]);

    render(RulesModule);

    expect(await screen.findByText("Global Rules")).toBeInTheDocument();
    await waitFor(() => {
      expect(rulesApiMocks.getManagedRulesState).toHaveBeenCalled();
    });
    expect(screen.queryByText("Tool rules need update")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Inject" })).not.toBeInTheDocument();
  });

  it("shows missing writable global files as createable injection targets", async () => {
    const user = userEvent.setup();
    baselineState = {
      common_rule: defaultRuleFingerprint(defaultRulesState[0]),
      custom_rules: {},
      custom_rule_pending_targets: {},
    };
    managedRulesStateResponse = {
      ...driftState,
      targets: [
        {
          ...driftState.targets[0],
          classification: "requires_sync",
          reason: "file_missing",
          can_write: true,
          has_managed_block: false,
          current_block: null,
          message: "Rule target file will be created during injection",
        },
      ],
      summary: {
        ...driftState.summary,
        requires_sync_targets: 1,
        drifted_targets: 0,
        affected_tool_ids: ["codex"],
      },
    };

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Inject" }));
    const modal = (await screen.findByText("Inject Rules")).closest(".modal");

    expect(within(modal).getByText("/codex/AGENTS.md")).toBeInTheDocument();
    expect(within(modal).getByText("Will create file")).toBeInTheDocument();
    expect(within(modal).queryByText("Exceptions")).not.toBeInTheDocument();
  });

  it("uses the effective certified target when persisted injection targets are empty", async () => {
    const user = userEvent.setup();
    defaultRulesState = [
      {
        ...defaultRulesState[0],
        managed_targets: ["hermes-agent"],
      },
    ];
    baselineState = {
      common_rule: defaultRuleFingerprint(defaultRulesState[0]),
      custom_rules: {},
      custom_rule_pending_targets: {},
    };
    toolApiMocks.getInjectionTargets.mockResolvedValue({});
    toolStores.tools.set([
      {
        id: "hermes-agent",
        name: "Hermes Agent",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "soul",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/hermes/SOUL.md",
            label: "SOUL.md",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);
    toolStores.managedToolIds.set(["hermes-agent"]);
    managedRulesStateResponse = {
      ...driftState,
      rule_sets: [
        {
          ...driftState.rule_sets[0],
          managed_tool_ids: ["hermes-agent"],
        },
      ],
      targets: [
        {
          ...driftState.targets[0],
          tool_id: "hermes-agent",
          tool_name: "Hermes Agent",
          target_path: "/hermes/SOUL.md",
          classification: "requires_sync",
          reason: "file_missing",
          can_write: true,
          has_managed_block: false,
          current_block: null,
          message: "Rule target file will be created during injection",
        },
      ],
      summary: {
        ...driftState.summary,
        requires_sync_targets: 1,
        drifted_targets: 0,
        affected_tool_ids: ["hermes-agent"],
      },
    };

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Inject" }));
    const modal = (await screen.findByText("Inject Rules")).closest(".modal");

    expect(within(modal).getByText("/hermes/SOUL.md")).toBeInTheDocument();
    expect(within(modal).getByText("Will create file")).toBeInTheDocument();
    expect(within(modal).queryByText("Exceptions")).not.toBeInTheDocument();

    await user.click(within(modal).getByRole("button", { name: "Inject" }));

    await waitFor(() => {
      expect(rulesApiMocks.injectDefaultRules).toHaveBeenCalledWith("hermes-agent");
    });
  });

  it("shows pending updates as an expandable diff", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = {
      ...driftState,
      targets: [
        {
          ...driftState.targets[0],
          classification: "requires_sync",
          reason: "missing_managed_block",
          has_managed_block: false,
          current_block: "",
        },
      ],
      summary: {
        ...driftState.summary,
        requires_sync_targets: 1,
        drifted_targets: 0,
        affected_tool_ids: ["codex"],
      },
    };
    baselineState = {
      common_rule: defaultRuleFingerprint(defaultRulesState[0]),
      custom_rules: {},
      custom_rule_pending_targets: {},
    };

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "View details" }));
    await user.click(screen.getByRole("button", { name: /Codex.*Needs update/ }));

    await waitFor(() => {
      expect(rulesApiMocks.diffRules).toHaveBeenCalledWith(
        "",
        "Rules in Codex file",
        "expected",
        "Rules in app",
      );
    });
    expect(screen.queryByText("Rules in app")).not.toBeInTheDocument();
    expect(screen.getAllByText("expected").length).toBeGreaterThan(0);
  });

  it("shows saved global source updates as an expandable diff before injection", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    baselineState = {
      common_rule: defaultRuleFingerprint(defaultRulesState[0]),
      custom_rules: {},
      custom_rule_pending_targets: {},
    };
    rulesApiMocks.saveDefaultRule.mockImplementation((rule) => {
      defaultRulesState = defaultRulesState.map((item) => item.id === rule.id ? { ...item, ...rule } : item);
      managedRulesStateResponse = {
        ...driftState,
        targets: [
          {
            ...driftState.targets[0],
            classification: "requires_sync",
            reason: "pending_source",
            source_pending: true,
            expected_block: "expected after save",
            current_block: "current before save",
          },
        ],
        summary: {
          ...driftState.summary,
          requires_sync_targets: 1,
          drifted_targets: 0,
          pending_source_rule_sets: 1,
          affected_tool_ids: ["codex"],
        },
      };
      return Promise.resolve(true);
    });

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Edit" }));
    await replaceSourceEditorText(user, "next");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(screen.getByText("Tool rules need update")).toBeInTheDocument();
    });
    await user.click(screen.getByRole("button", { name: "View details" }));
    await user.click(screen.getByRole("button", { name: /Codex.*Needs update/ }));

    await waitFor(() => {
      expect(rulesApiMocks.diffRules).toHaveBeenCalledWith(
        "current before save",
        "Rules in Codex file",
        "expected after save",
        "Rules in app",
      );
    });
    expect(screen.getByText("current")).toBeInTheDocument();
    expect(screen.getAllByText("expected").length).toBeGreaterThan(0);
  });

  it("expands unresolved targets to show the localized detailed reason", async () => {
    const user = userEvent.setup();
    locale.set("zh");
    managedRulesStateResponse = {
      ...driftState,
      targets: [
        {
          ...driftState.targets[0],
          classification: "unresolved",
          reason: "file_missing",
          can_read: false,
          can_write: false,
          has_managed_block: false,
          current_block: null,
          message: "The target rule file does not exist.",
        },
      ],
      summary: {
        ...driftState.summary,
        drifted_targets: 0,
        unresolved_targets: 1,
        affected_tool_ids: ["codex"],
      },
    };

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "查看详情" }));
    await user.click(screen.getByRole("button", { name: /Codex.*文件缺失/ }));

    expect(screen.getByText("目标规则文件不存在。")).toBeInTheDocument();
    expect(screen.queryByText("The target rule file does not exist.")).not.toBeInTheDocument();
    expect(rulesApiMocks.diffRules).not.toHaveBeenCalled();
  });

  it("does not expose backend English messages for unconfigured targets in Chinese", async () => {
    const user = userEvent.setup();
    locale.set("zh");
    managedRulesStateResponse = {
      ...driftState,
      targets: [
        {
          ...driftState.targets[0],
          classification: "unresolved",
          reason: "unconfigured_target",
          target_path: null,
          can_read: false,
          can_write: false,
          has_managed_block: false,
          current_block: null,
          message: "No injection target is configured for this tool",
        },
      ],
      summary: {
        ...driftState.summary,
        drifted_targets: 0,
        unresolved_targets: 1,
        affected_tool_ids: ["codex"],
      },
    };

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "查看详情" }));
    await user.click(screen.getByRole("button", { name: /Codex.*无法更新/ }));

    expect(screen.getByText("没有配置可写入的规则文件。")).toBeInTheDocument();
    expect(screen.queryByText("No injection target is configured for this tool")).not.toBeInTheDocument();
    expect(rulesApiMocks.diffRules).not.toHaveBeenCalled();
  });

  it("exits global edit mode after saving and clears pending state after injection", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    baselineState = {
      common_rule: defaultRuleFingerprint(defaultRulesState[0]),
      custom_rules: {},
      custom_rule_pending_targets: {},
    };
    rulesApiMocks.injectDefaultRules.mockImplementation(() => {
      managedRulesStateResponse = cleanState;
      return Promise.resolve(true);
    });

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Edit" }));
    await replaceSourceEditorText(user, "next");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(rulesApiMocks.saveDefaultRule).toHaveBeenCalledWith(expect.objectContaining({ content: "next" }));
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
      expect(screen.getByText("Tool rules need update")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "Inject" }));
    const modal = (await screen.findByText("Inject Rules")).closest(".modal");
    await user.click(within(modal).getByRole("button", { name: "Inject" }));

    await waitFor(() => {
      expect(rulesApiMocks.injectDefaultRules).toHaveBeenCalledWith("codex");
      expect(screen.queryByText("Needs update")).not.toBeInTheDocument();
      expect(screen.queryByText("Tool rules need update")).not.toBeInTheDocument();
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });
  });

  it("clears saved global pending after writable targets inject when exceptions remain", async () => {
    const user = userEvent.setup();
    toolStores.tools.set([
      { id: "codex", name: "Codex", detected: true, rule_sources: [] },
      { id: "trae", name: "Trae", detected: true, rule_sources: [] },
    ]);
    toolStores.managedToolIds.set(["codex", "trae"]);
    managedRulesStateResponse = cleanState;
    baselineState = {
      common_rule: defaultRuleFingerprint(defaultRulesState[0]),
      custom_rules: {},
      custom_rule_pending_targets: {},
    };
    const traeExceptionTarget = {
      tool_id: "trae",
      tool_name: "Trae",
      target_path: null,
      rule_set_ids: ["common_rule"],
      rule_set_names: ["Global Rules"],
      classification: "unresolved",
      reason: "unconfigured_target",
      can_read: false,
      can_write: false,
      source_pending: false,
      has_managed_block: false,
      expected_block: "expected",
      current_block: null,
      message: "No injection target is configured for this tool",
    };
    const cursorExceptionTarget = {
      ...traeExceptionTarget,
      tool_id: "cursor",
      tool_name: "Cursor",
      reason: "unknown_support",
      message: "Rule target support is not readable from the declared capability",
    };
    rulesApiMocks.saveDefaultRule.mockImplementation((rule) => {
      defaultRulesState = defaultRulesState.map((item) => item.id === rule.id ? { ...item, ...rule } : item);
      managedRulesStateResponse = {
        ...driftState,
        targets: [
          {
            ...driftState.targets[0],
            classification: "requires_sync",
            reason: "pending_source",
            source_pending: true,
            expected_block: "expected after save",
            current_block: "current before save",
          },
          { ...traeExceptionTarget, source_pending: true },
        ],
        summary: {
          ...driftState.summary,
          requires_sync_targets: 1,
          drifted_targets: 0,
          unresolved_targets: 1,
          pending_source_rule_sets: 1,
          affected_tool_ids: ["codex", "trae"],
        },
      };
      return Promise.resolve(true);
    });
    rulesApiMocks.injectDefaultRules.mockImplementation((toolId) => {
      if (toolId === "codex") {
        managedRulesStateResponse = {
          ...cleanState,
          targets: [traeExceptionTarget],
          summary: {
            ...cleanState.summary,
            in_sync_targets: 0,
            unresolved_targets: 1,
            affected_tool_ids: ["trae"],
          },
        };
      }
      return Promise.resolve(true);
    });

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Edit" }));
    await replaceSourceEditorText(user, "next");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(screen.getByText("Tool rules need update")).toBeInTheDocument();
    });
    await user.click(screen.getByRole("button", { name: "View details" }));
    expect(screen.getByRole("button", { name: /Codex.*Needs update/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Trae.*Cannot update/ })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Inject" }));
    const modal = (await screen.findByText("Inject Rules")).closest(".modal");
    expect(within(modal).getByText("/codex/AGENTS.md")).toBeInTheDocument();
    expect(within(modal).getByText("Target file pending")).toBeInTheDocument();
    await user.click(within(modal).getByRole("button", { name: "Inject" }));

    await waitFor(() => {
      expect(rulesApiMocks.injectDefaultRules).toHaveBeenCalledWith("codex");
      expect(rulesApiMocks.injectDefaultRules).not.toHaveBeenCalledWith("trae");
      expect(rulesApiMocks.setDefaultRuleInjectionBaselines).toHaveBeenLastCalledWith(expect.objectContaining({
        common_rule: defaultRuleFingerprint(defaultRulesState[0]),
      }));
      expect(screen.queryByRole("button", { name: /Codex.*Needs update/ })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /Trae.*Cannot update/ })).not.toBeInTheDocument();
      expect(screen.queryByText("Tool rules need update")).not.toBeInTheDocument();
    });

    toolStores.tools.set([
      { id: "codex", name: "Codex", detected: true, rule_sources: [] },
      { id: "trae", name: "Trae", detected: true, rule_sources: [] },
      { id: "cursor", name: "Cursor", detected: true, rule_sources: [] },
    ]);
    toolStores.managedToolIds.set(["codex", "trae", "cursor"]);
    managedRulesStateResponse = {
      ...cleanState,
      targets: [traeExceptionTarget, cursorExceptionTarget],
      summary: {
        ...cleanState.summary,
        in_sync_targets: 0,
        unresolved_targets: 2,
        affected_tool_ids: ["trae", "cursor"],
      },
    };
    await user.click(screen.getByRole("button", { name: "Refresh" }));

    await waitFor(() => {
      expect(screen.getByText("Tool rules need update")).toBeInTheDocument();
    });
    await user.click(screen.getByRole("button", { name: "View details" }));
    expect(screen.queryByRole("button", { name: /Trae.*Cannot update/ })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Cursor.*Cannot update/ })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Edit" }));
    await replaceSourceEditorText(user, "next round");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(screen.getByText("Tool rules need update")).toBeInTheDocument();
    });
    const nextRoundViewDetails = screen.queryByRole("button", { name: "View details" });
    if (nextRoundViewDetails) await user.click(nextRoundViewDetails);
    expect(screen.getByRole("button", { name: /Codex.*Needs update/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Trae.*Cannot update/ })).toBeInTheDocument();
  });

  it("acknowledges exception-only global injection rounds until the next change", async () => {
    const user = userEvent.setup();
    toolStores.tools.set([
      { id: "trae", name: "Trae", detected: true, rule_sources: [] },
    ]);
    toolStores.managedToolIds.set(["trae"]);
    defaultRulesState = [
      {
        ...defaultRulesState[0],
        managed_targets: ["trae"],
      },
    ];
    managedRulesStateResponse = cleanState;
    baselineState = {
      common_rule: defaultRuleFingerprint(defaultRulesState[0]),
      custom_rules: {},
      custom_rule_pending_targets: {},
    };
    const traeExceptionTarget = {
      tool_id: "trae",
      tool_name: "Trae",
      target_path: null,
      rule_set_ids: ["common_rule"],
      rule_set_names: ["Global Rules"],
      classification: "unresolved",
      reason: "unconfigured_target",
      can_read: false,
      can_write: false,
      source_pending: true,
      has_managed_block: false,
      expected_block: "expected",
      current_block: null,
      message: "No injection target is configured for this tool",
    };
    rulesApiMocks.saveDefaultRule.mockImplementation((rule) => {
      defaultRulesState = defaultRulesState.map((item) => item.id === rule.id ? { ...item, ...rule, managed_targets: ["trae"] } : item);
      managedRulesStateResponse = {
        ...cleanState,
        targets: [traeExceptionTarget],
        summary: {
          ...cleanState.summary,
          in_sync_targets: 0,
          unresolved_targets: 1,
          pending_source_rule_sets: 1,
          affected_tool_ids: ["trae"],
        },
      };
      return Promise.resolve(true);
    });

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Edit" }));
    await replaceSourceEditorText(user, "next");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(screen.getByText("Tool rules need update")).toBeInTheDocument();
    });
    await user.click(screen.getByRole("button", { name: "Inject" }));
    const modal = (await screen.findByText("Inject Rules")).closest(".modal");
    expect(within(modal).getByText("No injectable targets")).toBeInTheDocument();
    expect(within(modal).getByText("Target file pending")).toBeInTheDocument();
    await user.click(within(modal).getByRole("button", { name: "Inject" }));

    await waitFor(() => {
      expect(rulesApiMocks.injectDefaultRules).not.toHaveBeenCalled();
      expect(screen.queryByText("Tool rules need update")).not.toBeInTheDocument();
      expect(screen.getByText("No injectable targets this round; exceptions hidden until the next change")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "Edit" }));
    await replaceSourceEditorText(user, "next round");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(screen.getByText("Tool rules need update")).toBeInTheDocument();
    });
    await user.click(screen.getByRole("button", { name: "View details" }));
    expect(screen.getByRole("button", { name: /Trae.*Cannot update/ })).toBeInTheDocument();
  });

  it("opens a single tool rule directly without exposing custom rules", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [{ label: "AGENTS.md", path: "/Users/leon/.codex/AGENTS.md", content: "# Tool Rule\n\none" }],
      },
    ]);

    render(RulesModule);

    await waitFor(() => {
      expect(document.querySelector(".file-viewer-title")?.textContent).toBe("AGENTS.md");
      expect(document.querySelector(".file-viewer-subtitle")?.textContent).toBe("~/.codex/AGENTS.md");
      expect(document.querySelector(".file-viewer-subtitle")).not.toHaveAttribute("title");
      expect(screen.getByText("Tool Rule")).toBeInTheDocument();
      expect(document.querySelector(".tool-rule-single-card .content-editor")).toHaveClass("unframed");
      expect(document.querySelector(".tool-rule-single-card .file-viewer-shell")).toHaveClass("variant-navigation-backed");
      expect(document.querySelector(".tool-rule-single-card .file-viewer-shell")).toHaveClass("with-navigation");
      expect(document.querySelector(".file-workspace-nav")).not.toBeNull();
      expect(screen.getByRole("button", { name: /^AGENTS\.md/ })).not.toHaveAttribute("title");
      expect(document.querySelector(".file-workspace-nav__path")).toBeNull();
      expect(document.querySelector(".tool-rule-single-card .item-list")).toBeNull();
      expect(screen.queryByRole("button", { name: "Back to list" })).not.toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "Edit" }));
    await replaceSourceEditorText(user, "# Tool Rule\n\ntwo");
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(rulesApiMocks.writeRule).toHaveBeenCalledWith("codex", "/Users/leon/.codex/AGENTS.md", "# Tool Rule\n\ntwo");
    });

    expect(screen.queryByRole("button", { name: "Custom" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Create new custom rule" })).not.toBeInTheDocument();
  });

  it("shows an explicit blank state for empty tool rule files", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("windsurf");
    toolStores.tools.set([
      {
        id: "windsurf",
        name: "Windsurf",
        detected: true,
        rule_sources: [{ label: "Global rules", path: "/Users/leon/.codeium/windsurf/memories/global_rules.md", content: "" }],
      },
    ]);

    render(RulesModule);

    expect(await screen.findByText("File is empty")).toBeInTheDocument();
    expect(document.querySelector(".tool-rule-single-card .md-rendered")).toBeNull();
  });

  it("treats fileless rule models as unsupported in the user-facing Rules surface", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("cursor");
    toolStores.tools.set([
      {
        id: "cursor",
        name: "Cursor",
        detected: true,
        rule_sources: [],
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
        rule_sources: [],
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

    render(RulesModule);

    expect(await screen.findByText("This tool does not support Rules management")).toBeInTheDocument();
    expect(screen.queryByText(/built-in user rules do not support file-form sync/)).not.toBeInTheDocument();
  });

  it("collapses and restores tool file navigation without resetting the selected rule", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [
          { label: "AGENTS.md", path: "/codex/AGENTS.md", content: "# Agents" },
          { label: "README.md", path: "/codex/README.md", content: "# Readme" },
        ],
      },
    ]);

    render(RulesModule);

    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("AGENTS.md");
      expect(document.querySelector(".file-workspace-nav")).not.toBeNull();
      expect(document.querySelector(".file-viewer-identity .file-viewer-navigation-boundary-toggle")).toBeNull();
      expect(document.querySelector(".file-viewer-navigation-boundary-toggle")).not.toBeNull();
      expect(screen.getByRole("button", { name: "Collapse file tree" }).closest(".file-viewer-navigation-boundary-control")).not.toBeNull();
      expect(screen.getByRole("button", { name: "Collapse file tree" }).closest(".file-viewer-main")).toBeNull();
      expect(document.querySelector(".file-viewer-navigation-boundary-control .tooltip-host")).toBeNull();
      expect(screen.getByRole("separator", { name: "Drag to resize file tree" })).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: /^README\.md/ }));
    expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("README.md");
    await user.click(screen.getByRole("button", { name: "Edit" }));
    await replaceSourceEditorText(user, "# Readme\n\nchanged");
    expect(screen.getByRole("button", { name: "Save changes" })).toBeInTheDocument();
    expect(screen.getByText(/Unsaved/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Collapse file tree" }));

    expect(document.querySelector(".file-workspace-nav")).toBeNull();
    expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("README.md");
    expect(screen.getByRole("button", { name: "Save changes" })).toBeInTheDocument();
    expect(screen.getByText(/Unsaved/)).toBeInTheDocument();
    expect(document.querySelector(".tool-rule-single-card .file-viewer-shell")).toHaveClass("navigation-collapsed");
    expect(document.querySelector(".tool-rule-single-card .file-viewer-shell")).not.toHaveClass("with-navigation");
    expect(screen.queryByRole("separator", { name: "Drag to resize file tree" })).not.toBeInTheDocument();
    expect(localStorage.getItem("modus.rules.toolFileNavigation.collapsed")).toBe("true");

    await user.click(screen.getByRole("button", { name: "Expand file tree" }));

    expect(document.querySelector(".file-workspace-nav")).not.toBeNull();
    expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("README.md");
    expect(document.querySelector(".tool-rule-single-card .file-viewer-shell")).toHaveClass("with-navigation");
    expect(screen.getByRole("button", { name: /^README\.md/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save changes" })).toBeInTheDocument();
    expect(screen.getByText(/Unsaved/)).toBeInTheDocument();
    expect(localStorage.getItem("modus.rules.toolFileNavigation.collapsed")).toBe("false");
  });

  it("keeps unreadable tool rules visible without opening the editor", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [
          {
            label: "BROKEN.md",
            path: "/codex/BROKEN.md",
            content: "",
            diagnostic: "unreadable",
          },
          { label: "AGENTS.md", path: "/codex/AGENTS.md", content: "# Tool Rule" },
        ],
      },
    ]);

    render(RulesModule);

    const unreadableRule = await screen.findByRole("button", { name: /BROKEN\.md/ });
    expect(unreadableRule).not.toBeDisabled();
    expect(unreadableRule).toHaveAccessibleName("BROKEN.md Unreadable");
    expect(document.querySelector(".file-workspace-nav__path")).toBeNull();
    expect(screen.queryByText("/codex/BROKEN.md")).not.toBeInTheDocument();

    await user.click(unreadableRule);

    expect(rulesApiMocks.readRuleContent).not.toHaveBeenCalledWith("/codex/BROKEN.md");
    expect(document.querySelector(".tool-rule-single-card")).not.toBeNull();
    expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("BROKEN.md");
    expect(document.querySelector(".tool-rule-single-card .content-editor")).toBeNull();
  });

  it("opens tool rule cards in the page detail instead of a modal", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [
          { label: "AGENTS.md", path: "/codex/AGENTS.md", content: "# Cached" },
          { label: "README.md", path: "/codex/README.md", content: "# Readme" },
        ],
      },
    ]);
    rulesApiMocks.readRuleContent.mockResolvedValue("# Fresh");

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: /^AGENTS\.md/ }));

    await waitFor(() => {
      expect(rulesApiMocks.readRuleContent).toHaveBeenCalledWith("/codex/AGENTS.md");
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("AGENTS.md");
      expect(screen.getByText("Fresh")).toBeInTheDocument();
      expect(document.querySelector(".tool-rule-single-card .file-viewer-shell")).toHaveClass("variant-navigation-backed");
      expect(document.querySelector(".tool-rule-single-card .file-viewer-shell")).toHaveClass("with-navigation");
      expect(document.querySelector(".editor-modal")).toBeNull();
      expect(document.querySelector(".overlay .editor-modal")).toBeNull();
      expect(screen.queryByRole("button", { name: "Back to list" })).not.toBeInTheDocument();
    });

    expect(await screen.findByRole("button", { name: /^README\.md/ })).toBeInTheDocument();
  });

  it("restores the last selected tool rule file per tool", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("codex");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [
          { label: "AGENTS.md", path: "/codex/AGENTS.md", content: "# Agents" },
          { label: "README.md", path: "/codex/README.md", content: "# Readme" },
        ],
      },
      {
        id: "cursor",
        name: "Cursor",
        detected: true,
        rule_sources: [
          { label: "CURSOR.md", path: "/cursor/CURSOR.md", content: "# Cursor" },
        ],
      },
    ]);
    rulesApiMocks.readRuleContent.mockImplementation((path) => Promise.resolve(`# ${path.split("/").pop()}`));

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: /^README\.md/ }));
    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("README.md");
    });

    toolStores.activeToolId.set("cursor");
    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("CURSOR.md");
    });

    toolStores.activeToolId.set("codex");
    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("README.md");
    });
  });

  it("renders folder-backed tool rules as one file tree and opens the selected file", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("openclaw");
    toolStores.tools.set([
      {
        id: "openclaw",
        name: "OpenClaw",
        detected: true,
        rule_sources: [
          { label: "ROOT.md", path: "/openclaw/ROOT.md", group: "", content: "# Root" },
          { label: "AGENTS.md", path: "/openclaw/workspace/AGENTS.md", group: "workspace", content: "# Workspace" },
          { label: "NOTES.md", path: "/openclaw/workspace/NOTES.md", group: "workspace", content: "# Notes\n\nnext" },
          { label: "2026-03-18-edge-history.md", path: "/openclaw/workspace-mac/memory/2026-03-18-edge-history.md", group: "workspace-mac/memory", content: "# Memory" },
        ],
      },
    ]);
    rulesApiMocks.readRuleContent.mockResolvedValue("# Fresh Workspace");

    render(RulesModule);

    expect(await screen.findByRole("button", { name: /workspace.*2 files/ })).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("button", { name: /workspace-mac.*1 file/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /memory.*1 file/ })).toBeInTheDocument();
    const memoryDirectory = screen.getByRole("button", { name: /memory.*1 file/ });
    expect(memoryDirectory).not.toHaveAttribute("title");
    expect(memoryDirectory.getAttribute("style")).toContain("--file-workspace-row-padding: 32px");
    expect(document.querySelector(".file-workspace-nav")).not.toBeNull();
    expect(document.querySelector(".file-workspace-nav__path")).toBeNull();
    expect(document.querySelector(".file-workspace-nav__chevron")).toBeNull();
    expect(document.querySelectorAll(".file-workspace-nav__folder-icon")).toHaveLength(3);
    expect(document.querySelectorAll(".file-workspace-nav__row--file")).toHaveLength(4);
    expect(document.querySelector(".tool-rule-tree")).toBeNull();
    expect(document.querySelector(".tool-rule-root-files")).toBeNull();
    expect(screen.getAllByText("ROOT.md").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: /workspace-mac.*1 file/ }));
    expect(screen.queryByRole("button", { name: /memory.*1 file/ })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /workspace-mac.*1 file/ }));
    expect(screen.getByRole("button", { name: /memory.*1 file/ })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /AGENTS\.md/ }));

    await waitFor(() => {
      expect(rulesApiMocks.readRuleContent).toHaveBeenCalledWith("/openclaw/workspace/AGENTS.md");
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("AGENTS.md");
      expect(screen.getByText("Fresh Workspace")).toBeInTheDocument();
    });
  });

  it("keeps OpenClaw trusted AGENTS name fixed while ordinary workspace rules can rename", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("openclaw");
    toolStores.tools.set([
      {
        id: "openclaw",
        name: "OpenClaw",
        detected: true,
        rule_sources: [
          { label: "AGENTS.md", path: "/openclaw/workspace/AGENTS.md", group: "workspace", content: "# Workspace" },
          { label: "NOTES.md", path: "/openclaw/workspace/NOTES.md", group: "workspace", content: "# Notes" },
        ],
        capabilities: [
          {
            id: "workspace-rules",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/openclaw/workspace",
            label: "Workspace rules",
            source_confidence: "official_community",
          },
          {
            id: "trusted-workspace-agents",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/openclaw/workspace/AGENTS.md",
            label: "Workspace AGENTS.md",
            source_confidence: "official_community",
          },
        ],
      },
    ]);

    render(RulesModule);

    await fireEvent.contextMenu(await screen.findByRole("button", { name: /^AGENTS\.md/ }), { clientX: 120, clientY: 150 });
    expect(screen.queryByRole("button", { name: "Rename" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Delete" })).toBeInTheDocument();

    await fireEvent.contextMenu(screen.getByRole("button", { name: /^NOTES\.md/ }), { clientX: 120, clientY: 180 });
    expect(screen.getByRole("button", { name: "Rename" })).toBeInTheDocument();
  });

  it("creates a new file under an existing folder-backed tool rule directory", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [
          { label: "team.md", path: "/claude/rules/team.md", group: "", content: "# Team" },
        ],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Create rule file" }));
    const dialog = await screen.findByRole("dialog", { name: "Create rule file" });
    expect(within(dialog).getByLabelText("Tool")).toHaveValue("claude_code");
    expect(within(dialog).getByText("Location")).toBeInTheDocument();
    expect(within(dialog).getByText("/claude/rules")).toBeInTheDocument();
    const nameInput = within(dialog).getByRole("textbox", { name: "File name" });
    await user.clear(nameInput);
    await user.type(nameInput, "NEW.md");
    await user.click(within(dialog).getByRole("button", { name: "Create and edit" }));

    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("NEW.md");
      expect(screen.getByText("EDITING")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /^NEW\.md/ })).toBeInTheDocument();
      expect(rulesApiMocks.createRuleFile).toHaveBeenCalledWith("claude_code", "/claude/rules/NEW.md", "");
    });

    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(rulesApiMocks.writeRule).toHaveBeenCalledWith("claude_code", "/claude/rules/NEW.md", "");
    });
  });

  it("blocks creating a case-variant duplicate from the create dialog", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [
          { label: "Test.md", path: "/claude/rules/Test.md", group: "", content: "# Existing" },
        ],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Create rule file" }));
    const dialog = await screen.findByRole("dialog", { name: "Create rule file" });
    await user.type(within(dialog).getByRole("textbox", { name: "File name" }), "test");

    expect(within(dialog).getByText("/claude/rules/test.md")).toBeInTheDocument();
    expect(within(dialog).getByText("A rule file already exists at this location.")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Create and edit" })).toBeDisabled();
  });

  it("keeps delete available in the tree when a source cannot create", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [
          { label: "rule.md", path: "/claude/rules/team/rule.md", group: "team", content: "# Rule" },
        ],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
            action_evidence: [
              { action: "view", supported: true },
              { action: "delete", supported: true },
            ],
          },
        ],
      },
    ]);
    rulesApiMocks.deleteRuleEntry.mockImplementation((toolId, path) => Promise.resolve({
      deletes: [path],
      changes: [{ action: "delete", changeKind: "delete", subject: toolId, path, entryKind: "file" }],
    }));

    render(RulesModule);

    const directoryRow = await screen.findByRole("button", { name: /team.*1 file/ });
    await fireEvent.contextMenu(directoryRow, { clientX: 120, clientY: 120 });

    expect(screen.queryByRole("button", { name: "New rule file" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "New folder" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Delete" })).toBeInTheDocument();

    await fireEvent.contextMenu(screen.getByRole("button", { name: /^rule\.md/ }), { clientX: 120, clientY: 150 });

    expect(screen.queryByRole("button", { name: "Rename" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Delete" }));
    await screen.findByText("Delete rule file");
    await user.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() => {
      expect(rulesApiMocks.deleteRuleEntry).toHaveBeenNthCalledWith(1, "claude_code", "/claude/rules/team/rule.md", true);
      expect(rulesApiMocks.deleteRuleEntry).toHaveBeenNthCalledWith(2, "claude_code", "/claude/rules/team/rule.md", false);
    });
  });

  it("cancels file delete without executing the destructive action", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [
          { label: "rule.md", path: "/claude/rules/team/rule.md", group: "team", content: "# Rule" },
        ],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);
    rulesApiMocks.deleteRuleEntry.mockImplementation((toolId, path) => Promise.resolve({
      deletes: [path],
      changes: [{ action: "delete", changeKind: "delete", subject: toolId, path, entryKind: "file" }],
    }));

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: /^rule\.md/ }));
    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("rule.md");
    });

    await fireEvent.contextMenu(screen.getByRole("button", { name: /^rule\.md/ }), { clientX: 120, clientY: 150 });
    await user.click(screen.getByRole("button", { name: "Delete" }));
    const dialog = (await screen.findByText("Delete rule file")).closest(".modal");
    await user.click(within(dialog).getByText("Cancel").closest("button"));

    await waitFor(() => {
      expect(rulesApiMocks.deleteRuleEntry).toHaveBeenCalledTimes(1);
      expect(rulesApiMocks.deleteRuleEntry).toHaveBeenCalledWith("claude_code", "/claude/rules/team/rule.md", true);
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("rule.md");
    });
  });

  it("shows discovered empty directories as folders instead of files", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [
          {
            label: "empty",
            path: "/claude/rules/team/empty",
            group: "team/empty",
            format: "Directory",
            content: "",
          },
        ],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
            action_evidence: [
              { action: "view", supported: true },
              { action: "create", supported: true },
              { action: "delete", supported: true },
            ],
          },
        ],
      },
    ]);

    render(RulesModule);

    expect(await screen.findByRole("button", { name: /empty.*0 files/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^empty$/ })).not.toBeInTheDocument();
    expect(screen.getAllByText("Select a file").length).toBeGreaterThan(0);
    expect(rulesApiMocks.readRuleContent).not.toHaveBeenCalled();
  });

  it("marks the effective injection target and can set a normal file as the Global Rule file", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("codex");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [
          { label: "AGENTS.md", path: "/codex/AGENTS.md", content: "# Default" },
          { label: "team.md", path: "/codex/rules/team.md", group: "rules", content: "# Team" },
        ],
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/AGENTS.md",
            label: "AGENTS.md",
            source_confidence: "official_docs",
          },
          {
            id: "rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/codex/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);
    toolStoreMocks.loadTools
      .mockImplementationOnce(() => {
        const [tool] = get(toolStores.tools);
        const updated = {
          ...tool,
          capabilities: [
            ...tool.capabilities,
            {
              id: "user-configured-global-rule-target",
              kind: "rule",
              scope: "global",
              access: "writable",
              format: "markdown",
              source_path: "/codex/rules/team.md",
              label: "Custom Global Rule target",
              source_confidence: "user_configured",
            },
          ],
        };
        toolStores.tools.set([updated]);
        return Promise.resolve([updated]);
      })
      .mockImplementationOnce(() => {
        const [tool] = get(toolStores.tools);
        const updated = {
          ...tool,
          capabilities: tool.capabilities.filter((capability) => capability.id !== "user-configured-global-rule-target"),
        };
        toolStores.tools.set([updated]);
        return Promise.resolve([updated]);
      });

    render(RulesModule);

    expect(await screen.findByRole("button", { name: /AGENTS\.md.*Injection target/ })).toBeInTheDocument();

    await fireEvent.contextMenu(screen.getByRole("button", { name: /^AGENTS\.md.*Injection target/ }), { clientX: 120, clientY: 120 });
    expect(screen.queryByRole("button", { name: "Set as Global Rule file" })).not.toBeInTheDocument();

    await fireEvent.contextMenu(screen.getByRole("button", { name: /^team\.md/ }), { clientX: 120, clientY: 150 });
    await user.click(screen.getByRole("button", { name: "Set as Global Rule file" }));

    await waitFor(() => {
      expect(toolApiMocks.setToolCapabilityOverrides).toHaveBeenCalledWith("codex", {
        customGlobalRuleTarget: "/codex/rules/team.md",
        sharedSkillDirectRead: null,
      });
      expect(toolStoreMocks.loadTools).toHaveBeenCalled();
      expect(screen.getByText("Set as Global Rule file.")).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /team\.md.*Injection target/ })).toBeInTheDocument();
    });

    await fireEvent.contextMenu(screen.getByRole("button", { name: /^AGENTS\.md$/ }), { clientX: 120, clientY: 120 });
    await user.click(screen.getByRole("button", { name: "Set as Global Rule file" }));

    await waitFor(() => {
      expect(toolApiMocks.setToolCapabilityOverrides).toHaveBeenLastCalledWith("codex", {
        customGlobalRuleTarget: null,
        sharedSkillDirectRead: null,
      });
    });
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /AGENTS\.md.*Injection target/ })).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: /^team\.md$/ })).toBeInTheDocument();
    expect(rulesApiMocks.writeRule).not.toHaveBeenCalled();
    expect(rulesApiMocks.createRuleFile).not.toHaveBeenCalled();
    expect(rulesApiMocks.injectDefaultRules).not.toHaveBeenCalled();
  });

  it("marks only the user-configured injection target when it replaces the certified default", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("codex");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [
          { label: "AGENTS.md", path: "/codex/AGENTS.md", content: "# Default" },
          { label: "team.md", path: "/codex/rules/team.md", group: "rules", content: "# Team" },
        ],
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/AGENTS.md",
            label: "AGENTS.md",
            source_confidence: "official_docs",
          },
          {
            id: "rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/codex/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
          {
            id: "user-configured-global-rule-target",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/rules/team.md",
            label: "Custom Global Rule target",
            source_confidence: "user_configured",
          },
        ],
      },
    ]);

    render(RulesModule);

    expect(await screen.findByRole("button", { name: /team\.md.*Injection target/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^AGENTS\.md$/ })).toBeInTheDocument();
  });

  it("restores the OpenClaw certified default target from the file tree after a custom target override", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("openclaw");
    toolStores.tools.set([
      {
        id: "openclaw",
        name: "OpenClaw",
        detected: true,
        rule_sources: [
          { label: "AGENTS.md", path: "/openclaw/workspace/AGENTS.md", content: "# Agents" },
          { label: "BOOTSTRAP.md", path: "/openclaw/workspace/BOOTSTRAP.md", content: "# Bootstrap" },
        ],
        capabilities: [
          {
            id: "workspace-rules",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/openclaw/workspace",
            label: "Workspace rules",
            source_confidence: "official_community",
          },
          {
            id: "trusted-workspace-agents",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/openclaw/workspace/AGENTS.md",
            label: "Workspace AGENTS.md",
            source_confidence: "official_community",
          },
          {
            id: "user-configured-global-rule-target",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/openclaw/workspace/BOOTSTRAP.md",
            label: "Custom Global Rule target",
            source_confidence: "user_configured",
          },
        ],
      },
    ]);
    toolStoreMocks.loadTools.mockImplementationOnce(() => {
      const [tool] = get(toolStores.tools);
      const updated = {
        ...tool,
        capabilities: tool.capabilities.filter((capability) => capability.id !== "user-configured-global-rule-target"),
      };
      toolStores.tools.set([updated]);
      return Promise.resolve([updated]);
    });

    render(RulesModule);

    expect(await screen.findByRole("button", { name: /BOOTSTRAP\.md.*Injection target/ })).toBeInTheDocument();

    await fireEvent.contextMenu(screen.getByRole("button", { name: /^AGENTS\.md$/ }), { clientX: 120, clientY: 120 });
    await user.click(screen.getByRole("button", { name: "Set as Global Rule file" }));

    await waitFor(() => {
      expect(toolApiMocks.setToolCapabilityOverrides).toHaveBeenCalledWith("openclaw", {
        customGlobalRuleTarget: null,
        sharedSkillDirectRead: null,
      });
    });
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /AGENTS\.md.*Injection target/ })).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: /^BOOTSTRAP\.md$/ })).toBeInTheDocument();
    expect(rulesApiMocks.writeRule).not.toHaveBeenCalled();
    expect(rulesApiMocks.createRuleFile).not.toHaveBeenCalled();
    expect(rulesApiMocks.injectDefaultRules).not.toHaveBeenCalled();
  });

  it("shows a Global Rule file missing empty state for absent official targets", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("codex");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/AGENTS.md",
            label: "AGENTS.md",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    expect(await screen.findByText("Global rule file missing")).toBeInTheDocument();
  });

  it("creates folders and renames files from the tree context menu", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [
          { label: "rule.md", path: "/claude/rules/team/rule.md", group: "team", content: "# Rule" },
        ],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);
    rulesApiMocks.renameRuleEntry.mockResolvedValue("/claude/rules/team/renamed.md");

    render(RulesModule);

    const directoryRow = await screen.findByRole("button", { name: /team.*1 file/ });
    await fireEvent.contextMenu(directoryRow, { clientX: 120, clientY: 120 });
    await user.click(screen.getByRole("button", { name: "New folder" }));
    let dialog = await screen.findByRole("dialog", { name: "New folder" });
    await user.type(within(dialog).getByRole("textbox", { name: "Folder name" }), "qa");
    await user.click(within(dialog).getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(rulesApiMocks.createRuleDirectory).toHaveBeenCalledWith("claude_code", "/claude/rules/team/qa");
    });

    await fireEvent.contextMenu(screen.getByRole("button", { name: /^rule\.md/ }), { clientX: 120, clientY: 150 });
    await user.click(screen.getByRole("button", { name: "Rename" }));
    dialog = await screen.findByRole("dialog", { name: "Rename" });
    const renameInput = within(dialog).getByRole("textbox", { name: "File name" });
    await user.clear(renameInput);
    await user.type(renameInput, "renamed");
    await user.click(within(dialog).getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(rulesApiMocks.renameRuleEntry).toHaveBeenCalledWith("claude_code", "/claude/rules/team/rule.md", "renamed.md");
    });
  });

  it("auto-dismisses successful tool rule action feedback", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [
          { label: "rule.md", path: "/claude/rules/team/rule.md", group: "team", content: "# Rule" },
        ],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);
    rulesApiMocks.renameRuleEntry.mockResolvedValue("/claude/rules/team/renamed.md");

    render(RulesModule);

    await fireEvent.contextMenu(await screen.findByRole("button", { name: /^rule\.md/ }), { clientX: 120, clientY: 150 });
    await user.click(screen.getByRole("button", { name: "Rename" }));
    const dialog = await screen.findByRole("dialog", { name: "Rename" });
    const renameInput = within(dialog).getByRole("textbox", { name: "File name" });
    await user.clear(renameInput);
    await user.type(renameInput, "renamed");
    await user.click(within(dialog).getByRole("button", { name: "Confirm" }));

    expect(await screen.findByText("Renamed.")).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.queryByText("Renamed.")).not.toBeInTheDocument();
    }, { timeout: 4000 });
  });

  it("keeps failed tool rule action feedback visible", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [
          { label: "rule.md", path: "/claude/rules/team/rule.md", group: "team", content: "# Rule" },
        ],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);
    const setTimeoutSpy = vi.spyOn(globalThis, "setTimeout");
    rulesApiMocks.renameRuleEntry.mockRejectedValue(new Error("rename failed"));

    try {
      render(RulesModule);

      await fireEvent.contextMenu(await screen.findByRole("button", { name: /^rule\.md/ }), { clientX: 120, clientY: 150 });
      await user.click(screen.getByRole("button", { name: "Rename" }));
      const dialog = await screen.findByRole("dialog", { name: "Rename" });
      const renameInput = within(dialog).getByRole("textbox", { name: "File name" });
      await user.clear(renameInput);
      await user.type(renameInput, "renamed");
      await user.click(within(dialog).getByRole("button", { name: "Confirm" }));

      expect(await screen.findByText(/Action failed:/)).toBeInTheDocument();
      expect(screen.getByText(/rename failed/)).toBeInTheDocument();
      expect(setTimeoutSpy).not.toHaveBeenCalledWith(expect.any(Function), 3000);
    } finally {
      setTimeoutSpy.mockRestore();
    }
  });

  it("blocks case-variant duplicates from the tree new-file action", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [
          { label: "Test.md", path: "/claude/rules/team/Test.md", group: "team", content: "# Existing" },
        ],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    const directoryRow = await screen.findByRole("button", { name: /team.*1 file/ });
    await fireEvent.contextMenu(directoryRow, { clientX: 120, clientY: 120 });
    await user.click(screen.getByRole("button", { name: "New rule file" }));
    const dialog = await screen.findByRole("dialog", { name: "New rule file" });
    await user.type(within(dialog).getByRole("textbox", { name: "File name" }), "test");

    expect(within(dialog).getByText("/claude/rules/team/test.md")).toBeInTheDocument();
    expect(within(dialog).getByText("A rule file already exists at this location.")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Confirm" })).toBeDisabled();
  });

  it("does not show a creation menu from blank tree space", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [
          { label: "rule.md", path: "/claude/rules/team/rule.md", group: "team", content: "# Rule" },
        ],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: /^rule\.md/ }));
    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("rule.md");
    });

    const event = new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 140, clientY: 260 });
    document.querySelector(".file-workspace-nav__list")?.dispatchEvent(event);

    expect(event.defaultPrevented).toBe(true);
    expect(screen.queryByRole("button", { name: "New folder" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "New rule file" })).not.toBeInTheDocument();
    expect(rulesApiMocks.createRuleDirectory).not.toHaveBeenCalled();
  });

  it("does not expose the create entry on Global rules", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("global");
    toolStores.activeToolId.set("codex");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/AGENTS.md",
            label: "AGENTS.md",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    await screen.findByRole("button", { name: "Global" });
    expect(screen.queryByRole("button", { name: "Create rule file" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Tools" }));
    expect(await screen.findByRole("button", { name: "Create rule file" })).toBeInTheDocument();
  });

  it("opens the cross-tool create dialog from Tools when the current tool cannot create", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("cursor");
    toolStores.tools.set([
      {
        id: "cursor",
        name: "Cursor",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "user-rules",
            kind: "rule",
            scope: "global",
            access: "unsupported",
            format: "unknown",
            source_path: "",
            label: "User Rules",
          },
        ],
      },
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/AGENTS.md",
            label: "AGENTS.md",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Create rule file" }));
    const dialog = await screen.findByRole("dialog", { name: "Create rule file" });

    expect(within(dialog).getByLabelText("Tool")).toHaveValue("codex");
    expect(within(dialog).queryByText("Cursor cannot create a rule file here, so another available tool is selected.")).not.toBeInTheDocument();
    expect(within(dialog).getAllByText("/codex/AGENTS.md").length).toBeGreaterThan(0);
  });

  it("closes the create dialog without opening a draft or writing a file", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Create rule file" }));
    const dialog = await screen.findByRole("dialog", { name: "Create rule file" });
    await user.type(within(dialog).getByRole("textbox", { name: "File name" }), "cancelled.md");
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));

    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: "Create rule file" })).not.toBeInTheDocument();
    });
    expect(document.querySelector(".tool-rule-single-card")).toBeNull();
    expect(screen.getByText("No rule files yet")).toBeInTheDocument();
    expect(rulesApiMocks.createRuleFile).not.toHaveBeenCalled();
  });

  it("resets location and file inputs when the dialog tool changes", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("codex");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/AGENTS.md",
            label: "AGENTS.md",
            source_confidence: "official_docs",
          },
        ],
      },
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Create rule file" }));
    const dialog = await screen.findByRole("dialog", { name: "Create rule file" });

    expect(within(dialog).getByText("AGENTS.md")).toBeInTheDocument();
    expect(within(dialog).queryByRole("textbox", { name: "File name" })).not.toBeInTheDocument();

    await user.selectOptions(within(dialog).getByLabelText("Tool"), "claude_code");

    expect(within(dialog).getByLabelText("Location")).toHaveValue("global-rules-directory::/claude/rules");
    const fileInput = within(dialog).getByRole("textbox", { name: "File name" });
    expect(fileInput).toHaveValue("");
  });

  it("supports child-folder placement only as part of creating a file", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Create rule file" }));
    const dialog = await screen.findByRole("dialog", { name: "Create rule file" });

    expect(within(dialog).queryByRole("button", { name: /folder/i })).not.toBeInTheDocument();
    await user.click(within(dialog).getByRole("checkbox", { name: "Place inside a new folder" }));
    await user.type(within(dialog).getByRole("textbox", { name: "Folder name" }), "team");
    await user.type(within(dialog).getByRole("textbox", { name: "File name" }), "NEW");
    expect(within(dialog).getByText("/claude/rules/team/NEW.md")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Create and edit" }));

    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("NEW.md");
      expect(screen.getByText("EDITING")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /^NEW\.md/ })).toBeInTheDocument();
      expect(rulesApiMocks.createRuleFile).toHaveBeenCalledWith("claude_code", "/claude/rules/team/NEW.md", "");
    });

    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(rulesApiMocks.writeRule).toHaveBeenCalledWith("claude_code", "/claude/rules/team/NEW.md", "");
    });
  });

  it("creates Copilot instruction rules with the verified instructions suffix", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("github-copilot");
    toolStores.tools.set([
      {
        id: "github-copilot",
        name: "GitHub Copilot",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "rule-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "instructions_markdown",
            source_path: "/copilot/instructions",
            label: "Instruction files",
            source_confidence: "certified_local_product_behavior",
            action_evidence: [
              { action: "view", supported: true },
              { action: "read", supported: true },
              { action: "diagnose", supported: true },
              { action: "create", supported: true },
              { action: "edit", supported: true },
              { action: "save", supported: true },
              { action: "delete", supported: true },
            ],
          },
        ],
      },
    ]);

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Create rule file" }));
    const dialog = await screen.findByRole("dialog", { name: "Create rule file" });
    const nameInput = within(dialog).getByRole("textbox", { name: "File name" });
    expect(nameInput).toHaveAttribute("placeholder", "team");
    await user.type(nameInput, "team");
    expect(within(dialog).getByText("/copilot/instructions/team.instructions.md")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Create and edit" }));
    expect(await screen.findByRole("button", { name: /^team\.instructions\.md/ })).toBeInTheDocument();
    expect(rulesApiMocks.createRuleFile).toHaveBeenCalledWith(
      "github-copilot",
      "/copilot/instructions/team.instructions.md",
      "",
    );
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(rulesApiMocks.writeRule).toHaveBeenCalledWith(
        "github-copilot",
        "/copilot/instructions/team.instructions.md",
        "",
      );
    });
  });

  it("keeps a newly created file in navigation when editing is canceled", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("github-copilot");
    toolStores.tools.set([
      {
        id: "github-copilot",
        name: "GitHub Copilot",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "rule-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "instructions_markdown",
            source_path: "/copilot/instructions",
            label: "Instruction files",
            source_confidence: "certified_local_product_behavior",
            action_evidence: [
              { action: "view", supported: true },
              { action: "read", supported: true },
              { action: "diagnose", supported: true },
              { action: "create", supported: true },
              { action: "edit", supported: true },
              { action: "save", supported: true },
              { action: "delete", supported: true },
            ],
          },
        ],
      },
    ]);

    const view = render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Create rule file" }));
    const dialog = await screen.findByRole("dialog", { name: "Create rule file" });
    await user.type(within(dialog).getByRole("textbox", { name: "File name" }), "draft");
    await user.click(within(dialog).getByRole("button", { name: "Create and edit" }));

    expect(await screen.findByRole("button", { name: /^draft\.instructions\.md/ })).toBeInTheDocument();
    expect(rulesApiMocks.createRuleFile).toHaveBeenCalledWith("github-copilot", "/copilot/instructions/draft.instructions.md", "");
    await view.rerender({ searchOpen: true, globalSearchQuery: "draft" });
    const resultsRegion = await screen.findByRole("region", { name: "Current file tree search results" });
    expect(within(resultsRegion).getByRole("button", { name: /draft\.instructions\.md/ })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /^draft\.instructions\.md/ })).toBeInTheDocument();
      expect(screen.queryByText("EDITING")).not.toBeInTheDocument();
    });
  });

  it("creates MDC rule files from stem names and preserves pasted MDC filenames", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("codebuddy");
    toolStores.tools.set([
      {
        id: "codebuddy",
        name: "CodeBuddy",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "user-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "mdc",
            source_path: "/codebuddy/rules",
            label: "User Rules",
            source_confidence: "certified_local_product_behavior",
            action_evidence: [
              { action: "view", supported: true },
              { action: "read", supported: true },
              { action: "diagnose", supported: true },
              { action: "create", supported: true },
              { action: "edit", supported: true },
              { action: "save", supported: true },
              { action: "delete", supported: true },
            ],
          },
        ],
      },
    ]);

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Create rule file" }));
    const dialog = await screen.findByRole("dialog", { name: "Create rule file" });
    const nameInput = within(dialog).getByRole("textbox", { name: "File name" });
    expect(nameInput).toHaveAttribute("placeholder", "team");
    await user.type(nameInput, "team");
    expect(within(dialog).getByText("/codebuddy/rules/team.mdc")).toBeInTheDocument();

    await user.clear(nameInput);
    await user.type(nameInput, "team.mdc");
    expect(within(dialog).getByText("/codebuddy/rules/team.mdc")).toBeInTheDocument();

    await user.click(within(dialog).getByRole("button", { name: "Create and edit" }));
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(rulesApiMocks.createRuleFile).toHaveBeenCalledWith("codebuddy", "/codebuddy/rules/team.mdc", "");
      expect(rulesApiMocks.writeRule).toHaveBeenCalledWith("codebuddy", "/codebuddy/rules/team.mdc", "");
    });
  });

  it("creates MDC files from the rule tree new-file action", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("codebuddy");
    toolStores.tools.set([
      {
        id: "codebuddy",
        name: "CodeBuddy",
        detected: true,
        rule_sources: [
          { label: "existing.mdc", path: "/codebuddy/rules/team/existing.mdc", group: "team", content: "# Existing" },
        ],
        capabilities: [
          {
            id: "user-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "mdc",
            source_path: "/codebuddy/rules",
            label: "User Rules",
            source_confidence: "certified_local_product_behavior",
            action_evidence: [
              { action: "view", supported: true },
              { action: "read", supported: true },
              { action: "diagnose", supported: true },
              { action: "create", supported: true },
              { action: "edit", supported: true },
              { action: "save", supported: true },
              { action: "delete", supported: true },
            ],
          },
        ],
      },
    ]);

    render(RulesModule);

    const directoryRow = await screen.findByRole("button", { name: /team.*1 file/ });
    await fireEvent.contextMenu(directoryRow, { clientX: 120, clientY: 120 });
    await user.click(screen.getByRole("button", { name: "New rule file" }));
    const dialog = await screen.findByRole("dialog", { name: "New rule file" });
    const nameInput = within(dialog).getByRole("textbox", { name: "File name" });
    expect(nameInput).toHaveAttribute("placeholder", "team");
    await user.type(nameInput, "frontend");
    expect(within(dialog).getByText("/codebuddy/rules/team/frontend.mdc")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("frontend.mdc");
      expect(screen.getByText("EDITING")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /^frontend\.mdc/ })).toBeInTheDocument();
      expect(rulesApiMocks.createRuleFile).toHaveBeenCalledWith("codebuddy", "/codebuddy/rules/team/frontend.mdc", "");
    });

    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(rulesApiMocks.writeRule).toHaveBeenCalledWith("codebuddy", "/codebuddy/rules/team/frontend.mdc", "");
    });
  });

  it("returns to the tool rule list when a created file makes the refreshed source set multi-file", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    const refreshedTools = [
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [
          { label: "team.md", path: "/claude/rules/team.md", group: "", content: "# Team" },
          { label: "NEW.md", path: "/claude/rules/NEW.md", group: "", content: "# New" },
        ],
        capabilities: [
          {
            id: "global-rules-directory",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ];
    toolStores.tools.set([
      {
        ...refreshedTools[0],
        rule_sources: [
          { label: "team.md", path: "/claude/rules/team.md", group: "", content: "# Team" },
        ],
      },
    ]);
    toolStoreMocks.loadTools.mockImplementation(() => {
      toolStores.tools.set(refreshedTools);
      return Promise.resolve(refreshedTools);
    });

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Create rule file" }));
    const dialog = await screen.findByRole("dialog", { name: "Create rule file" });
    const nameInput = within(dialog).getByRole("textbox", { name: "File name" });
    await user.clear(nameInput);
    await user.type(nameInput, "NEW.md");
    await user.click(within(dialog).getByRole("button", { name: "Create and edit" }));
    await replaceSourceEditorText(user, "# New");
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(rulesApiMocks.createRuleFile).toHaveBeenCalledWith("claude_code", "/claude/rules/NEW.md", "");
      expect(rulesApiMocks.writeRule).toHaveBeenCalledWith("claude_code", "/claude/rules/NEW.md", "# New");
      expect(screen.getByRole("button", { name: /NEW\.md/ })).toBeInTheDocument();
      expect(document.querySelector(".tool-rule-single-card")).not.toBeNull();
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("NEW.md");
    });
  });

  it("keeps current-tree search results separate from tree expansion and opens matched files", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("openclaw");
    toolStores.tools.set([
      {
        id: "openclaw",
        name: "OpenClaw",
        detected: true,
        rule_sources: [
          { label: "AGENTS.md", path: "/openclaw/workspace/AGENTS.md", group: "workspace", content: "# Workspace" },
          { label: "MAC.md", path: "/openclaw/workspace-mac/MAC.md", group: "workspace-mac", content: "# Mac" },
          { label: "2026-03-18-edge-history.md", path: "/openclaw/workspace-mac/memory/2026-03-18-edge-history.md", group: "workspace-mac/memory", content: "# Edge" },
        ],
      },
    ]);

    const view = render(RulesModule);
    const workspaceMac = await screen.findByRole("button", { name: /workspace-mac.*2 files/ });
    await user.click(workspaceMac);
    expect(screen.queryByRole("button", { name: /memory.*1 file/ })).not.toBeInTheDocument();

    await view.rerender({ searchOpen: true, globalSearchQuery: "edge" });

    expect(await screen.findByRole("region", { name: "Current file tree search results" })).toBeInTheDocument();
    let nav = document.querySelector(".file-workspace-nav");
    expect(within(nav).getByRole("button", { name: /workspace-mac.*2 files/ })).toHaveAttribute("aria-expanded", "false");
    expect(within(nav).queryByRole("button", { name: /memory.*1 file/ })).not.toBeInTheDocument();
    expect(within(nav).queryByRole("button", { name: /2026-03-18-edge-history\.md/ })).not.toBeInTheDocument();
    expect(within(nav).getByRole("button", { name: /AGENTS\.md/ })).toBeInTheDocument();
    expect(within(nav).queryByRole("button", { name: /MAC\.md/ })).not.toBeInTheDocument();

    let resultsRegion = screen.getByRole("region", { name: "Current file tree search results" });
    await user.click(document.querySelector(".file-viewer-main"));
    expect(screen.queryByRole("region", { name: "Current file tree search results" })).not.toBeInTheDocument();
    await user.click(screen.getByPlaceholderText("Search current file tree"));
    resultsRegion = await screen.findByRole("region", { name: "Current file tree search results" });
    const resultButton = within(resultsRegion).getAllByRole("button", { name: /2026-03-18-edge-history\.md/ })[0];
    await user.click(resultButton);
    expect(screen.queryByRole("region", { name: "Current file tree search results" })).not.toBeInTheDocument();

    nav = document.querySelector(".file-workspace-nav");
    expect(within(nav).getByRole("button", { name: /workspace-mac.*2 files/ })).toHaveAttribute("aria-expanded", "true");
    expect(within(nav).getByRole("button", { name: /memory.*1 file/ })).toHaveAttribute("aria-expanded", "true");
    expect(within(nav).getByRole("button", { name: /2026-03-18-edge-history\.md/ })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Collapse file tree" }));
    expect(document.querySelector(".file-workspace-nav")).toBeNull();

    await user.click(screen.getByRole("button", { name: "Expand file tree" }));
    nav = document.querySelector(".file-workspace-nav");
    expect(within(nav).getByRole("button", { name: /workspace-mac.*2 files/ })).toHaveAttribute("aria-expanded", "true");
    expect(within(nav).getByRole("button", { name: /memory.*1 file/ })).toHaveAttribute("aria-expanded", "true");
    expect(within(nav).getByRole("button", { name: /2026-03-18-edge-history\.md/ })).toBeInTheDocument();
    expect(within(nav).getByRole("button", { name: /AGENTS\.md/ })).toBeInTheDocument();
    expect(within(nav).getByRole("button", { name: /MAC\.md/ })).toBeInTheDocument();
  });

  it("keeps sync diagnostics out of the Tools tab", async () => {
    managedRulesStateResponse = driftState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("codex");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/AGENTS.md",
            label: "AGENTS.md",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    expect(await screen.findByText("No rule files yet")).toBeInTheDocument();
    expect(screen.queryByText("Tool rules need update")).not.toBeInTheDocument();
    expect(screen.queryByText("Managed block drifted")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Create rule file" })).toBeInTheDocument();
  });

  it("creates a missing writable tool rule file and opens it for editing", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("codex");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/AGENTS.md",
            label: "AGENTS.md",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Create rule file" }));
    await user.click(await screen.findByRole("button", { name: "Create and edit" }));

    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("AGENTS.md");
      expect(screen.getByText("EDITING")).toBeInTheDocument();
      expect(rulesApiMocks.createRuleFile).toHaveBeenCalledWith("codex", "/codex/AGENTS.md", "");
    });

    await user.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("AGENTS.md");
      expect(screen.queryByText("EDITING")).not.toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "Create rule file" }));
    await user.click(await screen.findByRole("button", { name: "Create and edit" }));
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(rulesApiMocks.createRuleFile).toHaveBeenCalledWith("codex", "/codex/AGENTS.md", "");
      expect(rulesApiMocks.writeRule).toHaveBeenCalledWith("codex", "/codex/AGENTS.md", "");
    });
  });

  it("defaults Claude creation to CLAUDE.md when the official file is missing", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [
          { label: "CLAUDE.md", path: "/claude/rules/CLAUDE.md", group: "", content: "# Team" },
        ],
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/claude/CLAUDE.md",
            label: "CLAUDE.md",
            source_confidence: "official_docs",
          },
          {
            id: "user-rules",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Create rule file" }));

    const dialog = await screen.findByRole("dialog", { name: "Create rule file" });
    const locationSelect = within(dialog).getByLabelText("Location");
    expect(locationSelect).toHaveValue("global-rule::/claude/CLAUDE.md");
    expect(within(locationSelect).getByRole("option", { name: "/claude/CLAUDE.md" })).toBeInTheDocument();
    expect(within(locationSelect).getByRole("option", { name: "/claude/rules" })).toBeInTheDocument();
    expect(within(dialog).queryByRole("textbox", { name: "File name" })).not.toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Create and edit" }));

    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("CLAUDE.md");
      expect(screen.getByText("EDITING")).toBeInTheDocument();
      expect(rulesApiMocks.createRuleFile).toHaveBeenCalledWith("claude_code", "/claude/CLAUDE.md", "");
    });

    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(rulesApiMocks.writeRule).toHaveBeenCalledWith("claude_code", "/claude/CLAUDE.md", "");
    });
  });

  it("opens the requested Claude directory file instead of the existing CLAUDE.md file", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("claude_code");
    toolStores.tools.set([
      {
        id: "claude_code",
        name: "Claude Code",
        detected: true,
        rule_sources: [
          { label: "CLAUDE.md", path: "/claude/CLAUDE.md", group: "", content: "# Existing" },
        ],
        capabilities: [
          {
            id: "global-rule",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/claude/CLAUDE.md",
            label: "CLAUDE.md",
            source_confidence: "official_docs",
          },
          {
            id: "user-rules",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "directory",
            source_path: "/claude/rules",
            label: "Rules directory",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("CLAUDE.md");
    });

    await user.click(screen.getByRole("button", { name: "Create rule file" }));
    const dialog = await screen.findByRole("dialog", { name: "Create rule file" });
    expect(within(dialog).getByLabelText("Location")).toHaveValue("user-rules::/claude/rules");
    const nameInput = within(dialog).getByRole("textbox", { name: "File name" });
    await user.type(nameInput, "test.md");
    await user.click(within(dialog).getByRole("button", { name: "Create and edit" }));

    await waitFor(() => {
      expect(document.querySelector(".tool-rule-single-card .file-viewer-title")?.textContent).toBe("test.md");
      expect(screen.getByText("EDITING")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /^test\.md/ })).toBeInTheDocument();
      expect(rulesApiMocks.createRuleFile).toHaveBeenCalledWith("claude_code", "/claude/rules/test.md", "");
    });
  });

  it("keeps tools with no connected rule source explanatory", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("cursor");
    toolStores.tools.set([
      {
        id: "cursor",
        name: "Cursor",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "user-rules",
            kind: "rule",
            scope: "global",
            access: "unsupported",
            format: "unknown",
            source_path: "",
            label: "User Rules",
          },
        ],
      },
    ]);

    render(RulesModule);

    expect(await screen.findByText("No rule files yet")).toBeInTheDocument();
    expect(screen.getByText("This tool does not support Rules management")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Create rule file" })).not.toBeInTheDocument();
    expect(document.querySelector(".tool-rule-tree")).toBeNull();
    expect(screen.queryByText("Project Rules")).not.toBeInTheDocument();
  });

  it("treats unknown rule capability as unsupported in the user-facing Rules surface", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("unknown-rules");
    toolStores.managedToolIds.set(["unknown-rules"]);
    toolStores.tools.set([
      {
        id: "unknown-rules",
        name: "Unknown Rules",
        detected: true,
        rule_sources: [],
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

    render(RulesModule);

    expect(await screen.findByText("No rule files yet")).toBeInTheDocument();
    expect(screen.getByText("This tool does not support Rules management")).toBeInTheDocument();
    expect(screen.queryByText("Rule management is not confirmed for this tool")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Create rule file" })).not.toBeInTheDocument();
  });

  it("keeps read-only connected rule sources visible but non-createable", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("windsurf");
    toolStores.tools.set([
      {
        id: "windsurf",
        name: "Windsurf",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "global-rules",
            kind: "rule",
            scope: "global",
            access: "read_only",
            format: "markdown",
            source_path: "/windsurf/global_rules.md",
            label: "Global rules",
            source_confidence: "official_docs",
          },
        ],
      },
    ]);

    render(RulesModule);

    expect(await screen.findByText("No rule files yet")).toBeInTheDocument();
    expect(screen.queryByText("A rule source is connected for this tool, but there are no readable files and the app cannot create one here.")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Create rule file" })).not.toBeInTheDocument();
  });

  it("does not treat user-configured global injection targets as tool rule sources", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("codex");
    toolStores.tools.set([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        rule_sources: [],
        capabilities: [
          {
            id: "user-configured-global-rule-target",
            kind: "rule",
            scope: "global",
            access: "writable",
            format: "markdown",
            source_path: "/codex/CUSTOM.md",
            label: "Custom Global Rule target",
            source_confidence: "user_configured",
            action_evidence: [
              { action: "view", supported: true, evidence: "legacy user override" },
              { action: "create", supported: true, evidence: "legacy user override" },
              { action: "inject", supported: true, evidence: "legacy user override" },
            ],
          },
        ],
      },
    ]);

    render(RulesModule);

    expect(await screen.findByText("No rule files yet")).toBeInTheDocument();
    expect(screen.getByText("This tool does not support Rules management")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Create rule file" })).not.toBeInTheDocument();
  });

  it("redirects stale custom rule state to the retained global rule page", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("custom");
    defaultRulesState = [
      defaultRulesState[0],
      {
        id: "rule_custom",
        name: "Custom Alpha",
        content: "custom content",
        inject_to: ["codex"],
      },
    ];
    baselineState = {
      common_rule: defaultRuleFingerprint(defaultRulesState[0]),
      custom_rules: {},
      custom_rule_pending_targets: {
        rule_custom: ["codex"],
      },
    };

    render(RulesModule);

    await waitFor(() => {
      expect(get(toolStores.activeRulesTab)).toBe("global");
    });

    expect(screen.getByRole("button", { name: "Global" })).toHaveClass("active");
    expect(screen.queryByRole("button", { name: "Custom" })).not.toBeInTheDocument();
    expect(screen.queryByText("Custom Alpha")).not.toBeInTheDocument();
    expect(screen.queryByText("Needs update")).not.toBeInTheDocument();
    expect(document.querySelector(".custom-rule-detail")).toBeNull();
    expect(screen.getByText("Global Rules")).toBeInTheDocument();
  });

  it("uses two-cell Rules navigation and keeps long tool inventories scrollable", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.activeRulesTab.set("tool");
    toolStores.activeToolId.set("openclaw");
    toolStores.tools.set([
      {
        id: "openclaw",
        name: "OpenClaw",
        detected: true,
        rule_sources: Array.from({ length: 16 }, (_, index) => ({
          label: `RULE-${index + 1}.md`,
          path: `/openclaw/workspace-${index % 4}/RULE-${index + 1}.md`,
          group: `workspace-${index % 4}`,
          content: `# Rule ${index + 1}`,
        })),
      },
    ]);

    render(RulesModule);

    expect(await screen.findByRole("button", { name: "Global" })).toBeInTheDocument();
    expect(document.querySelector(".content-tabs")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Custom" })).not.toBeInTheDocument();
    expect(document.querySelectorAll(".content-tabs .content-tab")).toHaveLength(2);
    expect(await screen.findByRole("button", { name: /workspace-0.*4 files/ })).toBeInTheDocument();

    const appCss = readFileSync("src/app.css", "utf8");
    const rulesSource = readFileSync("src/lib/features/rules/components/RulesModule.svelte", "utf8");
    expect(appCss).toContain(".content-tab-row");
    expect(appCss).toContain(".content-tab.active::after");
    expect(appCss).toContain("--content-tab-row-height: 32px;");
    expect(rulesSource).toContain("class=\"content-tabs\"");
    expect(rulesSource).toContain("overflow-y: auto;");
    expect(rulesSource).not.toContain("overflow: hidden;\n    padding-bottom: 20px;");
  });

  it("wires hidden custom rule editors to the same save path as the visible save button", () => {
    const rulesSource = readFileSync("src/lib/features/rules/components/RulesModule.svelte", "utf8");
    const customRuleEditorBlocks = [...rulesSource.matchAll(/<ContentEditor[\s\S]*?filename="custom-rule\.md"[\s\S]*?\/>/g)]
      .map((match) => match[0]);

    expect(customRuleEditorBlocks).toHaveLength(2);
    for (const block of customRuleEditorBlocks) {
      expect(block).toContain("onSave={saveRuleEditor}");
    }
    expect(rulesSource).toContain("if (!canSaveRuleEditor) return;");
  });

  it("does not expose custom rule creation in the current Rules flow", async () => {
    managedRulesStateResponse = cleanState;
    toolStores.tools.set([
      { id: "codex", name: "Codex", detected: true, rule_sources: [] },
      { id: "cursor", name: "Cursor", detected: true, rule_sources: [] },
      { id: "claude_code", name: "Claude Code", detected: true, rule_sources: [] },
    ]);
    toolStores.managedToolIds.set(["codex", "cursor", "claude_code"]);

    render(RulesModule);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Global" })).toBeInTheDocument();
    });
    expect(screen.queryByRole("button", { name: "Create new custom rule" })).not.toBeInTheDocument();
    expect(screen.queryByRole("textbox", { name: "Rule Name" })).not.toBeInTheDocument();
    expect(rulesApiMocks.saveDefaultRule).not.toHaveBeenCalled();
  });

  it("keeps a single inconsistency banner after saving a global rule", async () => {
    const user = userEvent.setup();
    managedRulesStateResponse = cleanState;
    baselineState = {
      common_rule: defaultRuleFingerprint(defaultRulesState[0]),
      custom_rules: {},
      custom_rule_pending_targets: {},
    };

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Edit" }));
    await replaceSourceEditorText(user, "next");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(screen.getAllByText("Tool rules need update")).toHaveLength(1);
      expect(screen.getByRole("button", { name: "Inject" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Later" })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Skip" })).not.toBeInTheDocument();
      expect(screen.queryByText("Needs update")).not.toBeInTheDocument();
      expect(screen.queryByText("Update state")).not.toBeInTheDocument();
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });
  });

  it("uses the same injection preview in a combined pending and drift state", async () => {
    const user = userEvent.setup();
    rulesApiMocks.injectDefaultRules.mockImplementation(() => {
      managedRulesStateResponse = cleanState;
      return Promise.resolve(true);
    });
    render(RulesModule);

    await waitFor(() => {
      expect(screen.getByText("Tool rules need update")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Later" })).not.toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "Inject" }));
    const modal = (await screen.findByText("Inject Rules")).closest(".modal");
    await user.click(within(modal).getByRole("button", { name: "Inject" }));

    await waitFor(() => {
      expect(rulesApiMocks.injectDefaultRules).toHaveBeenCalledWith("codex");
      expect(rulesApiMocks.syncManagedRuleTargets).not.toHaveBeenCalled();
    });
  });

  it("shows non-writable targets as exceptions in the injection preview", async () => {
    const user = userEvent.setup();
    baselineState = {
      common_rule: defaultRuleFingerprint(defaultRulesState[0]),
      custom_rules: {},
      custom_rule_pending_targets: {},
    };
    managedRulesStateResponse = {
      ...driftState,
      targets: [{ ...driftState.targets[0], can_write: false, reason: "read_only_target" }],
    };

    render(RulesModule);

    await user.click(await screen.findByRole("button", { name: "Inject" }));
    const modal = (await screen.findByText("Inject Rules")).closest(".modal");
    expect(screen.getByText("Exceptions")).toBeInTheDocument();
    expect(screen.getByText("Cannot write")).toBeInTheDocument();
    expect(within(modal).getByText("/codex/AGENTS.md")).not.toHaveAttribute("title");
    const injectButton = within(modal).getByRole("button", { name: "Inject" });
    expect(injectButton).not.toBeDisabled();
    await user.click(injectButton);
    await waitFor(() => {
      expect(rulesApiMocks.injectDefaultRules).not.toHaveBeenCalled();
      expect(screen.getByText("No injectable targets this round; exceptions hidden until the next change")).toBeInTheDocument();
    });
    expect(screen.queryByRole("button", { name: "Leave" })).not.toBeInTheDocument();
  });
});
