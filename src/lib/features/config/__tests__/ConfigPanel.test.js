import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { get } from "svelte/store";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const apiMocks = vi.hoisted(() => ({
  listConfigFiles: vi.fn(),
  readConfigFile: vi.fn(),
  saveConfigFile: vi.fn(),
}));

vi.mock("$lib/features/config/api/config.js", () => apiMocks);
vi.mock("$lib/shared/components/ToolIcon.svelte", async () => {
  const mod = await import("../../../../test/stubs/EmptyStub.svelte");
  return { default: mod.default };
});

import ConfigPanel from "$lib/features/config/components/ConfigPanel.svelte";
import { activeToolId, managedToolIds, tools } from "$lib/features/tools/index.js";
import {
  modulePerformanceSummaries,
  setModulePerformanceDiagnosticsEnabledState,
} from "$lib/shared/diagnostics/modulePerformance.js";
import { locale } from "$lib/shared/i18n/index.js";

/**
 * @param {import("@testing-library/user-event").UserEvent} user
 * @param {string} text
 */
async function replaceSourceEditorText(user, text) {
  const editor = screen.getByRole("textbox");
  await user.click(editor);
  await user.keyboard("{Control>}a{/Control}");
  await user.paste(text);
  return editor;
}

/** @param {ParentNode} container */
function sourceEditorLineNumbers(container = document) {
  return [...container.querySelectorAll(".cm-lineNumbers .cm-gutterElement")]
    .filter((node) => !node.getAttribute("style")?.includes("visibility: hidden"))
    .map((node) => node.textContent);
}

const writableFile = {
  id: "settings",
  label: "Settings",
  path: "/Users/visual/.claude/settings.json",
  format: "json",
  access: "writable",
  editable: true,
  exists: true,
  size_bytes: 18,
  modified_unix: 1_700_000_000,
};

const missingFile = {
  id: "missing",
  label: "Missing Settings",
  path: "/tmp/missing.json",
  format: "json",
  access: "writable",
  editable: false,
  exists: false,
  size_bytes: null,
  modified_unix: null,
};

const readOnlyFile = {
  id: "readonly",
  label: "Read Only Settings",
  path: "/tmp/readonly.json",
  format: "json",
  access: "read_only",
  editable: false,
  exists: true,
  size_bytes: 2,
  modified_unix: 1_700_000_002,
};

const unknownFile = {
  id: "unknown",
  label: "Unknown Settings",
  path: "/tmp/unknown.json",
  format: "json",
  access: "unknown",
  editable: false,
  exists: true,
  size_bytes: 2,
  modified_unix: null,
};

const unsupportedFile = {
  id: "unsupported",
  label: "Unsupported Settings",
  path: "/tmp/unsupported.json",
  format: "json",
  access: "unsupported",
  editable: false,
  exists: true,
  size_bytes: 2,
  modified_unix: null,
};

function configureTool() {
  tools.set([{
    id: "codex",
    name: "Codex",
    detected: true,
    capabilities: [{ kind: "ordinary_config", access: "writable" }],
  }]);
  managedToolIds.set(["codex"]);
  activeToolId.set("codex");
}

describe("ConfigPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    locale.set("en");
    setModulePerformanceDiagnosticsEnabledState(false);
    configureTool();
    apiMocks.listConfigFiles.mockResolvedValue([writableFile, missingFile]);
    apiMocks.readConfigFile.mockResolvedValue({
      ...writableFile,
      content: "{\n  \"model\": \"gpt\"\n}",
    });
    apiMocks.saveConfigFile.mockResolvedValue({
      backup_path: "/tmp/backup/settings.json",
      size_bytes: 24,
      modified_unix: 1_700_000_001,
    });
  });

  afterEach(() => {
    cleanup();
    tools.set([]);
    managedToolIds.set([]);
    activeToolId.set(null);
  });

  it("shows registered configuration file cards without raw content", async () => {
    const { container } = render(ConfigPanel);

    await screen.findByText("settings.json");
    expect(screen.getByRole("heading", { name: "Config" })).toBeInTheDocument();
    expect(container.querySelector(".view-fixed-header .view-header")).not.toBeNull();
    expect(container.querySelector(".view-pinned-toolbar .tool-selector-capsule")).not.toBeNull();
    expect(container.querySelector(".config-tabs")).toBeNull();
    expect(screen.queryByLabelText("Module performance diagnostics")).not.toBeInTheDocument();
    expect(get(modulePerformanceSummaries)).toEqual({});
    expect(screen.getByRole("button", { name: "Codex" })).toHaveClass("tool-selector-capsule__segment", "is-active");
    expect(screen.getByRole("button", { name: "Codex" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("missing.json")).toBeInTheDocument();
    expect(screen.getByText("Editable")).toBeInTheDocument();
    expect(screen.getByText("Missing")).toBeInTheDocument();
    expect(screen.queryByText(/\"model\"/)).not.toBeInTheDocument();
    expect(apiMocks.listConfigFiles).toHaveBeenCalledWith("codex");
  });

  it("filters registered configuration cards from the page header without scanning content first", async () => {
    const user = userEvent.setup();
    const { container } = render(ConfigPanel);

    await screen.findByText("settings.json");
    expect(screen.getByText("missing.json")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Search" }));
    await fireEvent.input(screen.getByPlaceholderText("Search registered configuration files"), {
      target: { value: "claude" },
    });
    expect(container.querySelector(".view-fixed-header")).not.toHaveClass("view-fixed-header--search-popover");
    expect(screen.queryByRole("region", { name: "Module search results" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^settings\.json\b/ })).toBeInTheDocument();
    expect(screen.queryByText("missing.json")).not.toBeInTheDocument();
    expect(apiMocks.readConfigFile).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: /^settings\.json\b/ }));

    await waitFor(() => {
      expect(apiMocks.readConfigFile).toHaveBeenCalledWith("codex", "settings");
    });
  });

  it("opens a single registered configuration file directly", async () => {
    apiMocks.listConfigFiles.mockResolvedValueOnce([writableFile]);
    const { container } = render(ConfigPanel);

    await screen.findByRole("textbox");
    expect(screen.queryByRole("button", { name: /^settings\.json\b/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Back" })).not.toBeInTheDocument();
    expect(screen.getByRole("textbox")).toHaveTextContent("\"model\"");
    expect(container.querySelector(".file-viewer-title")?.textContent).toBe("settings.json");
    expect(container.querySelector(".file-viewer-subtitle")?.textContent).toBe("~/.claude/settings.json");
    expect(container.querySelector(".file-viewer-shell")).toHaveClass("variant-single-file");
    expect(container.querySelector(".file-viewer-navigation")).toBeNull();
    expect(container.querySelector(".content-editor")).toHaveClass("unframed");
  });

  it("opens an existing file and cancels editing without saving", async () => {
    const user = userEvent.setup();
    const { container } = render(ConfigPanel);

    await user.click(await screen.findByRole("button", { name: /^settings\.json\b/ }));
    expect(await screen.findByRole("textbox")).toHaveTextContent("\"model\"");
    expect(screen.getByRole("textbox")).toHaveTextContent("\"gpt\"");
    expect(sourceEditorLineNumbers(container)).toEqual(["1", "2", "3"]);
    expect(container.querySelector(".content-line-numbers")).toBeNull();

    await user.click(screen.getByRole("button", { name: "Edit" }));
    expect(screen.getByRole("textbox")).toHaveAttribute("contenteditable", "true");
    expect(container.querySelector(".file-viewer-badge")).toBeNull();
    expect(container.querySelector(".file-viewer-mode")?.textContent).toBe("EDITING");
    await replaceSourceEditorText(user, "{}");
    await user.click(screen.getByRole("button", { name: "Cancel" }));

    expect(apiMocks.readConfigFile).toHaveBeenCalledWith("codex", "settings");
    expect(apiMocks.saveConfigFile).not.toHaveBeenCalled();
    expect(screen.getByRole("textbox")).toHaveAttribute("contenteditable", "false");
    expect(screen.getByRole("textbox")).toHaveTextContent("\"model\"");
  });

  it("searches the visible draft of the open configuration file", async () => {
    const user = userEvent.setup();
    render(ConfigPanel);

    await user.click(await screen.findByRole("button", { name: /^settings\.json\b/ }));
    await user.click(screen.getByRole("button", { name: "Edit" }));
    await replaceSourceEditorText(user, "{\n  \"draftNeedle\": true,\n  \"copy\": \"draftNeedle\"\n}");
    await user.click(screen.getByRole("button", { name: "Search" }));
    await fireEvent.input(screen.getByPlaceholderText("Search current content"), {
      target: { value: "draftNeedle" },
    });

    expect(screen.queryByRole("region", { name: "Module search results" })).not.toBeInTheDocument();
    expect(screen.getByText("1/2")).toBeInTheDocument();
    await waitFor(() => {
      expect(document.querySelector(".cm-searchMatch")).not.toBeNull();
    });
    await user.click(screen.getByRole("button", { name: "Next match" }));
    expect(screen.getByText("2/2")).toBeInTheDocument();
    expect(
      screen.getAllByRole("textbox").some((textbox) => textbox.textContent?.includes("draftNeedle")),
    ).toBe(true);
    expect(apiMocks.saveConfigFile).not.toHaveBeenCalled();
  });

  it("shows missing files as unavailable without reading them", async () => {
    const user = userEvent.setup();
    render(ConfigPanel);

    await user.click(await screen.findByRole("button", { name: /missing\.json/ }));

    await screen.findByText("This configuration file is missing");
    expect(apiMocks.readConfigFile).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: "Edit" })).not.toBeInTheDocument();
  });

  it("shows read-only, unknown, and unsupported states without edit actions", async () => {
    const user = userEvent.setup();
    apiMocks.listConfigFiles.mockResolvedValueOnce([readOnlyFile, unknownFile, unsupportedFile]);
    apiMocks.readConfigFile.mockResolvedValueOnce({
      ...readOnlyFile,
      content: "{}",
    });
    render(ConfigPanel);

    await screen.findByText("Read-only");
    expect(screen.getByText("Unknown support")).toBeInTheDocument();
    expect(screen.getByText("Unsupported")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /^readonly\.json\b/ }));

    await screen.findByRole("textbox");
    expect(apiMocks.readConfigFile).toHaveBeenCalledWith("codex", "readonly");
    expect(screen.queryByRole("button", { name: "Edit" })).not.toBeInTheDocument();
    expect(screen.queryByText("1 lines")).not.toBeInTheDocument();
  });

  it("shows validation errors returned by save", async () => {
    const user = userEvent.setup();
    apiMocks.saveConfigFile.mockRejectedValueOnce("Invalid JSON: expected value");
    render(ConfigPanel);

    await user.click(await screen.findByRole("button", { name: /^settings\.json\b/ }));
    await user.click(screen.getByRole("button", { name: "Edit" }));
    await replaceSourceEditorText(user, "{invalid");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await screen.findByText("Invalid JSON: expected value");
  });

  it("shows list loading errors as the primary shared error state", async () => {
    apiMocks.listConfigFiles.mockRejectedValueOnce("List failed");
    render(ConfigPanel);

    await screen.findByText("List failed");
    expect(screen.getByRole("status")).toHaveTextContent("List failed");
    expect(screen.queryByText("No registered configuration files")).not.toBeInTheDocument();
  });

  it("keeps read failures out of edit mode", async () => {
    const user = userEvent.setup();
    apiMocks.readConfigFile.mockRejectedValueOnce("Failed to read file");
    render(ConfigPanel);

    await user.click(await screen.findByRole("button", { name: /^settings\.json\b/ }));

    await screen.findByText("Failed to read file");
    await screen.findByText("This configuration file could not be opened");
    expect(screen.queryByRole("button", { name: "Edit" })).not.toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });

  it("saves valid edits and reloads file metadata", async () => {
    const user = userEvent.setup();
    render(ConfigPanel);

    await user.click(await screen.findByRole("button", { name: /^settings\.json\b/ }));
    await user.click(screen.getByRole("button", { name: "Edit" }));
    await replaceSourceEditorText(user, "{}");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await screen.findByText("Saved");
    expect(apiMocks.saveConfigFile).toHaveBeenCalledWith("codex", "settings", "{}");
    expect(apiMocks.listConfigFiles).toHaveBeenCalledTimes(2);
  });
});
