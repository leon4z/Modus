import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { get } from "svelte/store";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const apiMocks = vi.hoisted(() => ({
  listMcpConfigSources: vi.fn(),
  readMcpServerConfigFragment: vi.fn(),
  saveMcpServerConfigFragment: vi.fn(),
}));

vi.mock("$lib/features/mcp/api/mcp.js", () => apiMocks);
vi.mock("$lib/shared/components/ToolIcon.svelte", async () => {
  const mod = await import("../../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});

import McpPanel from "$lib/features/mcp/components/McpPanel.svelte";
import { activeToolId, managedToolIds, tools } from "$lib/features/tools/index.js";
import {
  modulePerformanceSummaries,
  setModulePerformanceDiagnosticsEnabledState,
} from "$lib/shared/diagnostics/modulePerformance.js";
import { locale } from "$lib/shared/i18n/index.js";

function configureMcpTool() {
  tools.set([{
    id: "codex",
    name: "Codex",
    detected: true,
    capabilities: [{ kind: "mcp", scope: "global", access: "writable" }],
  }]);
  managedToolIds.set(["codex"]);
  activeToolId.set("codex");
}

function source(overrides = {}) {
  return {
    id: "mcp-config",
    tool_id: "codex",
    label: "MCP configuration",
    path: "/Users/visual/.codex/config.toml",
    format: "toml",
    access: "writable",
    source_kind: "file",
    state: "loaded",
    editable: true,
    exists: true,
    size_bytes: 128,
    modified_unix: 1770000000,
    server_count: 1,
    message: "MCP configuration loaded",
    servers: [{
      name: "browser-tools",
      server_type: "stdio",
      command: "npx",
      args: [],
      env_keys: ["BROWSER_TOKEN"],
      enabled: true,
      activation_state: "enabled",
      description: "Browser MCP",
    }],
    ...overrides,
  };
}

/**
 * @param {ReturnType<typeof userEvent.setup>} user
 * @param {string} text
 */
async function replaceSourceEditorText(user, text) {
  const editor = screen.getByRole("textbox");
  await user.click(editor);
  await user.keyboard("{Control>}a{/Control}");
  await user.paste(text);
}

describe("McpPanel configuration sources", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    locale.set("en");
    setModulePerformanceDiagnosticsEnabledState(false);
    configureMcpTool();
  });

  afterEach(() => {
    cleanup();
    tools.set([]);
    managedToolIds.set([]);
    activeToolId.set(null);
  });

  it("shows MCP server cards and opens a selected server configuration fragment", async () => {
    const user = userEvent.setup();
    apiMocks.listMcpConfigSources.mockResolvedValue([source()]);
    apiMocks.readMcpServerConfigFragment.mockResolvedValue({
      ...source(),
      label: "browser-tools",
      server_name: "browser-tools",
      source_id: "mcp-config",
      server: source().servers[0],
      content: "command = \"npx\"\nenabled = false",
    });

    const { container } = render(McpPanel);

    await screen.findByText("browser-tools");
    expect(screen.getByText(/Source: MCP configuration/)).toBeInTheDocument();
    expect(screen.queryByText("Enabled")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Edit" })).not.toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /browser-tools/ }));

    const editor = await screen.findByRole("textbox");
    expect(editor).toHaveTextContent("command = \"npx\"");
    expect(editor).toHaveTextContent("enabled = false");
    expect(screen.queryByLabelText("Module performance diagnostics")).not.toBeInTheDocument();
    expect(get(modulePerformanceSummaries)).toEqual({});
    expect(container.querySelector(".file-viewer-subtitle")?.textContent).toBe("~/.codex/config.toml");
    expect(container.querySelector(".file-viewer-shell")).toHaveClass("variant-single-file");
    expect(container.querySelector(".file-viewer-navigation")).toBeNull();
    expect(screen.queryByText(/May require tool restart/)).not.toBeInTheDocument();
    expect(apiMocks.listMcpConfigSources).toHaveBeenCalledWith("codex");
    expect(apiMocks.readMcpServerConfigFragment).toHaveBeenCalledWith("codex", "mcp-config", "browser-tools");
  });

  it("filters registered MCP entry cards from the page header", async () => {
    const user = userEvent.setup();
    apiMocks.listMcpConfigSources.mockResolvedValue([source({
      server_count: 2,
      servers: [
        source().servers[0],
        {
          ...source().servers[0],
          name: "alpha-tools",
          command: "alpha",
          description: "Alpha MCP",
          env_keys: [],
        },
      ],
    })]);
    apiMocks.readMcpServerConfigFragment.mockResolvedValue({
      ...source(),
      label: "browser-tools",
      server_name: "browser-tools",
      source_id: "mcp-config",
      server: source().servers[0],
      content: "command = \"npx\"",
    });

    const { container } = render(McpPanel);

    await screen.findByText("browser-tools");
    expect(screen.getByRole("button", { name: /alpha-tools/ })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Search" }));
    await fireEvent.input(screen.getByPlaceholderText("Search registered MCP sources"), {
      target: { value: "browser" },
    });
    expect(container.querySelector(".view-fixed-header")).not.toHaveClass("view-fixed-header--search-popover");
    expect(screen.queryByRole("region", { name: "Module search results" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /browser-tools/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /alpha-tools/ })).not.toBeInTheDocument();
    expect(apiMocks.readMcpServerConfigFragment).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: /browser-tools/ }));

    await waitFor(() => {
      expect(
        screen.getAllByRole("textbox").some((textbox) => textbox.textContent?.includes("command = \"npx\""))
      ).toBe(true);
    });
    expect(apiMocks.readMcpServerConfigFragment).toHaveBeenCalledWith("codex", "mcp-config", "browser-tools");
  });

  it("searches the open MCP fragment content without a result popover", async () => {
    const user = userEvent.setup();
    apiMocks.listMcpConfigSources.mockResolvedValue([source()]);
    apiMocks.readMcpServerConfigFragment.mockResolvedValue({
      ...source(),
      label: "browser-tools",
      server_name: "browser-tools",
      source_id: "mcp-config",
      server: source().servers[0],
      content: "command = \"npx\"\nneedle = true\nneedle = false",
    });

    const { container } = render(McpPanel);

    await user.click(await screen.findByRole("button", { name: /browser-tools/ }));
    await screen.findByRole("textbox");
    await user.click(screen.getByRole("button", { name: "Search" }));
    await fireEvent.input(screen.getByPlaceholderText("Search current content"), {
      target: { value: "needle" },
    });

    expect(container.querySelector(".view-fixed-header")).not.toHaveClass("view-fixed-header--search-popover");
    expect(screen.queryByRole("region", { name: "Module search results" })).not.toBeInTheDocument();
    expect(screen.getByText("1/2")).toBeInTheDocument();
    await waitFor(() => {
      expect(document.querySelector(".cm-searchMatch")).not.toBeNull();
    });
    await user.click(screen.getByRole("button", { name: "Next match" }));
    expect(screen.getByText("2/2")).toBeInTheDocument();
    expect(
      screen.getAllByRole("textbox").some((textbox) => textbox.textContent?.includes("needle = true")),
    ).toBe(true);
  });

  it("marks manual refresh diagnostics partial when rereading the selected server fails", async () => {
    const user = userEvent.setup();
    setModulePerformanceDiagnosticsEnabledState(true);
    apiMocks.listMcpConfigSources.mockResolvedValue([source()]);
    apiMocks.readMcpServerConfigFragment
      .mockResolvedValueOnce({
        ...source(),
        label: "browser-tools",
        server_name: "browser-tools",
        source_id: "mcp-config",
        server: source().servers[0],
        content: "command = \"npx\"",
      })
      .mockRejectedValueOnce(new Error("Read failed"));

    render(McpPanel);

    await user.click(await screen.findByRole("button", { name: /browser-tools/ }));
    await screen.findByRole("textbox");
    await user.click(screen.getByRole("button", { name: "Refresh" }));

    await waitFor(() => {
      expect(screen.getByLabelText("Module performance diagnostics")).toHaveTextContent("partial");
    });
  });

  it.each([
    ["missing", "MCP configuration file was not found"],
    ["empty", "No MCP services detected"],
    ["unreadable", "MCP configuration file is unreadable"],
    ["malformed", "MCP configuration is malformed"],
    ["unknown", "This tool does not support MCP configuration"],
    ["unsupported", "This tool does not support MCP configuration"],
    ["error", "MCP configuration file is unreadable"],
  ])("shows the %s source state as an informational empty state", async (state, message) => {
    apiMocks.listMcpConfigSources.mockResolvedValue([
      source({
        id: `mcp-${state}`,
        state,
        editable: false,
        exists: !["missing", "unknown", "unsupported"].includes(state),
        server_count: state === "empty" ? 0 : null,
        path: state === "unsupported" ? null : "/Users/visual/.codex/config.toml",
        message: "",
        servers: [],
      }),
      source({
        id: "other-mcp",
        label: "Other MCP",
        state: "missing",
        editable: false,
        exists: false,
        path: "/Users/visual/.codex/other.toml",
        server_count: null,
        message: "",
        servers: [],
      }),
    ]);

    render(McpPanel);

    await waitFor(() => {
      expect(screen.getAllByText(message).length).toBeGreaterThan(0);
    });
    expect(screen.queryByRole("button", { name: /MCP configuration/ })).not.toBeInTheDocument();
    expect(screen.queryByText("Enabled")).not.toBeInTheDocument();
    expect(screen.queryByText("Not enabled")).not.toBeInTheDocument();
  });

  it("does not render configuration source cards when no MCP entries are detected", async () => {
    apiMocks.listMcpConfigSources.mockResolvedValue([
      source({
        state: "empty",
        editable: true,
        exists: true,
        server_count: 0,
        servers: [],
        message: "MCP configuration exists but has no servers",
      }),
    ]);

    render(McpPanel);

    await screen.findByText("No MCP services detected");
    expect(screen.queryByText("MCP configuration has no services")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /MCP configuration/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Edit" })).not.toBeInTheDocument();

    expect(apiMocks.readMcpServerConfigFragment).not.toHaveBeenCalled();
  });

  it("prioritizes malformed sources over empty sources in multi-source empty states", async () => {
    apiMocks.listMcpConfigSources.mockResolvedValue([
      source({
        id: "empty-source",
        state: "empty",
        editable: false,
        exists: true,
        server_count: 0,
        servers: [],
      }),
      source({
        id: "malformed-source",
        label: "Other MCP",
        state: "malformed",
        editable: false,
        exists: true,
        path: "/Users/visual/.pi/agent/mcp.json",
        server_count: null,
        message: "backend malformed details",
        servers: [],
      }),
    ]);

    render(McpPanel);

    await screen.findByText("MCP configuration is malformed");
    expect(screen.queryByText("No MCP services detected")).not.toBeInTheDocument();
    expect(screen.queryByText("backend malformed details")).not.toBeInTheDocument();
  });

  it("uses a localized adapter-required empty state for dependency-gated MCP support", async () => {
    locale.set("zh");
    apiMocks.listMcpConfigSources.mockResolvedValue([
      source({
        id: "dependency-gated-mcp",
        tool_id: "pi-agent",
        label: "pi-mcp-adapter",
        state: "unsupported",
        access: "unsupported",
        editable: false,
        exists: false,
        path: null,
        server_count: null,
        message: "Install pi-mcp-adapter in Pi Agent to enable MCP configuration sources.",
        required_adapter: {
          tool_name: "Pi Agent",
          package_name: "pi-mcp-adapter",
        },
        servers: [],
      }),
    ]);

    render(McpPanel);

    await screen.findByText("Pi Agent当前不支持mcp，请安装官方扩展包pi-mcp-adapter");
    expect(screen.queryByText("mcp.panel.adapter_required_detail")).not.toBeInTheDocument();
    expect(
      screen.queryByText("Install pi-mcp-adapter in Pi Agent to enable MCP configuration sources."),
    ).not.toBeInTheDocument();
  });

  it("uses localized source prompts instead of backend diagnostic messages", async () => {
    locale.set("zh");
    apiMocks.listMcpConfigSources.mockResolvedValue([
      source({
        state: "malformed",
        editable: false,
        exists: true,
        server_count: null,
        servers: [],
        message: "MCP configuration source is malformed",
      }),
    ]);

    render(McpPanel);

    await screen.findByText("MCP 配置格式错误");
    expect(screen.queryByText("MCP configuration source is malformed")).not.toBeInTheDocument();
    expect(screen.getByText("/Users/visual/.codex/config.toml")).toBeInTheDocument();
  });

  it("saves edited MCP server configuration fragment with backup feedback", async () => {
    const user = userEvent.setup();
    apiMocks.listMcpConfigSources.mockResolvedValue([source()]);
    apiMocks.readMcpServerConfigFragment.mockResolvedValue({
      ...source(),
      label: "browser-tools",
      server_name: "browser-tools",
      source_id: "mcp-config",
      server: source().servers[0],
      content: "command = \"npx\"",
    });
    apiMocks.saveMcpServerConfigFragment.mockResolvedValue({
      backup_path: "/Users/visual/.modus/backups/mcp-config/codex/mcp-config/config.toml",
      size_bytes: 160,
      modified_unix: 1770000100,
      servers: [],
    });

    render(McpPanel);

    await user.click(await screen.findByRole("button", { name: /browser-tools/ }));
    await screen.findByRole("button", { name: /Edit/ });
    await user.click(screen.getByRole("button", { name: /Edit/ }));
    await replaceSourceEditorText(user, "command = \"node\"");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await screen.findByText("Configuration saved");
    expect(apiMocks.saveMcpServerConfigFragment).toHaveBeenCalledWith(
      "codex",
      "mcp-config",
      "browser-tools",
      "command = \"node\"",
    );
    expect(screen.getByText(/Backup saved/)).toBeInTheDocument();
  });

  it("omits activation labels on entry cards while preserving native fields in the opened fragment", async () => {
    const user = userEvent.setup();
    const disabledSource = source({
      id: "disabled-mcp",
      label: "Disabled MCP",
      path: "/Users/visual/.codex/disabled.toml",
      servers: [
        {
          ...source().servers[0],
          name: "disabled-tools",
          enabled: false,
          activation_state: "disabled",
        },
      ],
    });
    const unknownSource = source({
      id: "unknown-mcp",
      label: "Unknown MCP",
      path: "/Users/visual/.codex/unknown.json",
      format: "json",
      servers: [
        {
          ...source().servers[0],
          name: "unknown-tools",
          activation_state: "unknown",
        },
      ],
    });
    apiMocks.listMcpConfigSources.mockResolvedValue([
      source(),
      disabledSource,
      unknownSource,
    ]);
    apiMocks.readMcpServerConfigFragment.mockResolvedValue({
      ...source(),
      label: "browser-tools",
      server_name: "browser-tools",
      source_id: "mcp-config",
      server: source().servers[0],
      content: "command = \"npx\"\ndisabled = true",
    });

    render(McpPanel);

    await screen.findByText("browser-tools");
    expect(screen.queryByText("Enabled")).not.toBeInTheDocument();
    expect(screen.queryByText("Not enabled")).not.toBeInTheDocument();
    expect(screen.queryByText("Unknown")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Edit" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /browser-tools/ }));
    expect(await screen.findByRole("textbox")).toHaveTextContent("disabled = true");
    await screen.findByRole("button", { name: "Edit" });
    await user.click(screen.getByRole("button", { name: "Edit" }));

    expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
    expect(screen.queryByText(/may contain credentials/)).not.toBeInTheDocument();
    expect(apiMocks.readMcpServerConfigFragment).toHaveBeenCalledWith("codex", "mcp-config", "browser-tools");
  });

  it("keeps the editor open when saving invalid MCP configuration text fails", async () => {
    const user = userEvent.setup();
    apiMocks.listMcpConfigSources.mockResolvedValue([source()]);
    apiMocks.readMcpServerConfigFragment.mockResolvedValue({
      ...source(),
      label: "browser-tools",
      server_name: "browser-tools",
      source_id: "mcp-config",
      server: source().servers[0],
      content: "command = \"npx\"",
    });
    apiMocks.saveMcpServerConfigFragment.mockRejectedValue(new Error("Invalid TOML"));

    render(McpPanel);

    await user.click(await screen.findByRole("button", { name: /browser-tools/ }));
    await screen.findByRole("button", { name: /Edit/ });
    await user.click(screen.getByRole("button", { name: /Edit/ }));
    await replaceSourceEditorText(user, "[mcp_servers.browser-tools");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await screen.findByText("Error: Invalid TOML");
    expect(screen.queryByText("Configuration saved")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
  });

  it("cancels editing and restores the opened content", async () => {
    const user = userEvent.setup();
    apiMocks.listMcpConfigSources.mockResolvedValue([source()]);
    apiMocks.readMcpServerConfigFragment.mockResolvedValue({
      ...source(),
      label: "browser-tools",
      server_name: "browser-tools",
      source_id: "mcp-config",
      server: source().servers[0],
      content: "command = \"npx\"",
    });

    render(McpPanel);

    await user.click(await screen.findByRole("button", { name: /browser-tools/ }));
    await screen.findByRole("button", { name: /Edit/ });
    await user.click(screen.getByRole("button", { name: /Edit/ }));
    await replaceSourceEditorText(user, "changed");
    await user.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.getByRole("textbox")).toHaveTextContent("command = \"npx\"");
    expect(screen.queryByRole("button", { name: "Save" })).not.toBeInTheDocument();
    expect(apiMocks.saveMcpServerConfigFragment).not.toHaveBeenCalled();
  });

  it("keeps managed tools visible when MCP support is undeclared", async () => {
    tools.set([{
      id: "dev_tool",
      name: "Dev Tool",
      detected: true,
      capabilities: [],
    }]);
    managedToolIds.set(["dev_tool"]);
    activeToolId.set("dev_tool");
    apiMocks.listMcpConfigSources.mockResolvedValue([]);

    render(McpPanel);

    await screen.findByRole("button", { name: /Dev Tool/ });
    await screen.findByText("This tool does not support MCP configuration");
    expect(apiMocks.listMcpConfigSources).toHaveBeenCalledWith("dev_tool");
  });

  it("shows loading while MCP sources are pending", async () => {
    apiMocks.listMcpConfigSources.mockReturnValue(new Promise(() => {}));

    render(McpPanel);

    await waitFor(() => {
      expect(screen.getByText("Loading...")).toBeInTheDocument();
    });
  });
});
