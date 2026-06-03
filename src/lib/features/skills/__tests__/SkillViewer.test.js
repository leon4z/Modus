// @ts-nocheck
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { readFileSync } from "node:fs";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const apiMocks = vi.hoisted(() => ({
  readSkillContent: vi.fn(),
  listSkillFiles: vi.fn(),
  readSkillFile: vi.fn(),
  writeSkillFile: vi.fn(),
  scanSkillInventory: vi.fn(),
  scanSkillInventoryEntry: vi.fn(),
  previewInstall: vi.fn(),
  executeInstall: vi.fn(),
  previewCopySkillToTool: vi.fn(),
  executeCopySkillToTool: vi.fn(),
  previewRenameSkillSource: vi.fn(),
  executeRenameSkillSource: vi.fn(),
  previewUninstall: vi.fn(),
  executeUninstall: vi.fn(),
  previewDeleteFromTool: vi.fn(),
  executeDeleteFromTool: vi.fn(),
  previewCleanupDuplicateSkillSources: vi.fn(),
  executeCleanupDuplicateSkillSources: vi.fn(),
  previewDelete: vi.fn(),
  executeDelete: vi.fn(),
}));

const loggingApiMocks = vi.hoisted(() => ({
  writeSkillPerformanceLog: vi.fn(),
  writeModulePerformanceLog: vi.fn(),
}));

vi.mock("$lib/features/skills/api/skills.js", () => apiMocks);
vi.mock("$lib/features/rules/index.js", () => ({ diffRules: vi.fn() }));
vi.mock("$lib/shared/logging/api.js", () => loggingApiMocks);

import SkillViewer from "$lib/features/skills/components/SkillViewer.svelte";
import { invalidateSkillInventory } from "$lib/features/skills/queries/skillInventoryQuery.js";
import { managedToolIds, tools } from "$lib/features/tools/index.js";
import { setModulePerformanceDiagnosticsEnabledState } from "$lib/shared/diagnostics/modulePerformance.js";
import { locale } from "$lib/shared/i18n/index.js";

function skillCapability(tool, access = "writable") {
  return {
    id: "tool-skills",
    kind: "skill",
    scope: "tool",
    access,
    format: "skill_directory",
    source_path: tool?.skills_dir || "",
    label: `${tool?.name || tool?.id || "Tool"} Skills`,
  };
}

function skillCapabilityWithActions(tool, actions) {
  return {
    ...skillCapability(tool, "writable"),
    action_evidence: actions.map((action) => ({
      action,
      supported: true,
      evidence: "verified",
    })),
  };
}

function setToolsForTest(nextTools) {
  tools.set(
    nextTools.map((tool) => ({
      ...tool,
      capabilities: Array.isArray(tool.capabilities) ? tool.capabilities : [skillCapability(tool)],
    }))
  );
  managedToolIds.set(nextTools.filter((tool) => tool.detected).map((tool) => tool.id));
}

function createSkill(overrides = {}) {
  return {
    name: "demo-skill",
    display_name: "Demo Skill",
    path: "/Users/leon/.codex/skills/demo-skill",
    installed_in: [],
    tool_statuses: [],
    ...overrides,
  };
}

function createDeferred() {
  /** @type {(value: any) => void} */
  let resolve;
  /** @type {(reason?: any) => void} */
  let reject;
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function expectedLocalDateTime(value) {
  const date = new Date(value);
  const pad = (part) => String(part).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

const noSharedInstallWarning = "当前 Skill 没有共享目录来源，无法安装为共享目录链接。";
const noDeployTargetWarning = "该工具没有可部署的 Skill 目录，无法安装共享目录。";

describe("SkillViewer local-source behavior", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    invalidateSkillInventory();
    locale.set("zh");
    setModulePerformanceDiagnosticsEnabledState(false);
    setToolsForTest([
      { id: "codex", name: "Codex", detected: true, skills_dir: "/tmp/codex-skills" },
      { id: "cursor", name: "Cursor", detected: true, skills_dir: "/tmp/cursor-skills" },
    ]);
    apiMocks.listSkillFiles.mockResolvedValue([]);
    apiMocks.readSkillContent.mockResolvedValue({ skill_md_content: "# Demo" });
    apiMocks.readSkillFile.mockResolvedValue("# Demo");
    apiMocks.writeSkillFile.mockResolvedValue(undefined);
    apiMocks.scanSkillInventory.mockResolvedValue({ skills: [] });
    apiMocks.scanSkillInventoryEntry.mockImplementation(async (skillName) => {
      const inventory = await apiMocks.scanSkillInventory();
      return (inventory?.skills || []).find((item) => item.name === skillName) || createSkill({ name: skillName });
    });
    apiMocks.previewInstall.mockResolvedValue({});
    apiMocks.executeInstall.mockResolvedValue({});
    apiMocks.previewCopySkillToTool.mockResolvedValue({});
    apiMocks.executeCopySkillToTool.mockResolvedValue({});
    apiMocks.previewUninstall.mockResolvedValue({});
    apiMocks.executeUninstall.mockResolvedValue({});
    apiMocks.previewDeleteFromTool.mockResolvedValue({});
    apiMocks.executeDeleteFromTool.mockResolvedValue({});
    apiMocks.previewCleanupDuplicateSkillSources.mockResolvedValue({});
    apiMocks.executeCleanupDuplicateSkillSources.mockResolvedValue({});
    apiMocks.previewDelete.mockResolvedValue({});
    apiMocks.executeDelete.mockResolvedValue({});
    loggingApiMocks.writeSkillPerformanceLog.mockResolvedValue(undefined);
    loggingApiMocks.writeModulePerformanceLog.mockResolvedValue(undefined);
  });

  afterEach(() => {
    setModulePerformanceDiagnosticsEnabledState(false);
    cleanup();
    setToolsForTest([]);
    managedToolIds.set([]);
  });

  it("stays inert when no Skill is selected", async () => {
    const { container } = render(SkillViewer, {
      skill: null,
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await Promise.resolve();

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(apiMocks.listSkillFiles).not.toHaveBeenCalled();
    expect(apiMocks.readSkillContent).not.toHaveBeenCalled();
    expect(apiMocks.readSkillFile).not.toHaveBeenCalled();
  });

  it("renders Skill detail as an inset bounded dialog", async () => {
    const { container } = render(SkillViewer, {
      skill: createSkill(),
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    expect(await screen.findByRole("dialog")).toHaveClass("viewer-modal");

    const source = readFileSync("src/lib/features/skills/components/SkillViewer.svelte", "utf8");
    expect(source).toContain("width: min(1120px, calc(100vw - 96px));");
    expect(source).toContain("height: min(860px, calc(100vh - 72px));");
    expect(source).toContain("max-height: calc(100vh - 72px);");
    expect(source).toContain("border-radius: 16px;");
    expect(source).toContain("height: 76px;");
    expect(source).not.toContain("height: 116px;");
    expect(source).not.toContain("height: 100vh;");

    const sourceRead = readFileSync("src/lib/shared/components/SourceReadModeToggle.svelte", "utf8");
    expect(sourceRead).toContain("border-radius: var(--segmented-track-radius);");
    expect(sourceRead).toContain("padding: var(--segmented-track-padding);");
    expect(sourceRead).toContain("border: none;");
    expect(sourceRead).toContain("border-radius: var(--segmented-item-radius);");
    expect(sourceRead).toContain("background: var(--bg-card);");
    expect(sourceRead).not.toContain("$lib/shared/components/Tooltip.svelte");
    expect(sourceRead).not.toContain("<Tooltip");
    expect(sourceRead).not.toContain("border-color: var(--toolbar-control-border-active);");
  });

  it("keeps compact Skill detail header controls visible with a long title", async () => {
    render(SkillViewer, {
      skill: createSkill({
        display_name: "A very long Skill name that should truncate without covering tabs or close controls",
        installed_in: [{ tool_id: "codex", mode: "copy", path: "/tmp/codex-skills/demo-skill" }],
      }),
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    expect(await screen.findByText("A very long Skill name that should truncate without covering tabs or close controls")).toBeInTheDocument();
    expect(screen.getByText("可用 1/2")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "内容" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "管理" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "关闭 (Esc)" })).toBeInTheDocument();

    const source = readFileSync("src/lib/features/skills/components/SkillViewer.svelte", "utf8");
    expect(source).toContain("text-overflow: ellipsis;");
    expect(source).toContain("white-space: nowrap;");
  });

  it("shows a one-item file navigation area for one-file Skills", async () => {
    render(SkillViewer, {
      skill: createSkill(),
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    expect(await screen.findByRole("button", { name: "SKILL.md" })).toBeInTheDocument();
    expect(document.querySelector(".file-workspace-nav")).not.toBeNull();
    expect(screen.getByRole("separator", { name: "拖动调整文件树宽度" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "收起文件树" })).toBeInTheDocument();
    expect(document.querySelector(".file-viewer-shell")).toHaveClass("variant-navigation-backed");
    expect(document.querySelector(".file-viewer-shell")).toHaveClass("with-navigation");
    expect(document.querySelector(".file-viewer-subtitle")?.textContent).toBe("~/.codex/skills/demo-skill/SKILL.md");
    expect(document.querySelector(".file-viewer-subtitle")).not.toHaveAttribute("title");
    expect(await screen.findByText("Demo")).toBeInTheDocument();
  });

  it("resizes the Skill file tree without leaving the file workspace", async () => {
    render(SkillViewer, {
      skill: createSkill(),
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    const resizeHandle = await screen.findByRole("separator", { name: "拖动调整文件树宽度" });
    const navigation = document.querySelector(".file-viewer-navigation");
    expect(navigation?.getAttribute("style")).toContain("220px");
    expect(getComputedStyle(resizeHandle).borderLeftStyle).toBe("none");
    expect(getComputedStyle(resizeHandle).backgroundColor).toBe("rgba(0, 0, 0, 0)");

    await fireEvent.pointerDown(resizeHandle, { clientX: 100 });
    await fireEvent.pointerMove(window, { clientX: 160 });
    await fireEvent.pointerUp(window, { clientX: 160 });

    expect(navigation?.getAttribute("style")).toContain("280px");
    expect(getComputedStyle(resizeHandle).borderLeftStyle).toBe("none");
    expect(getComputedStyle(resizeHandle).backgroundColor).toBe("rgba(0, 0, 0, 0)");
    expect(await screen.findByRole("button", { name: "SKILL.md" })).toBeInTheDocument();
  });

  it("collapses and restores the Skill file tree from the file boundary", async () => {
    const user = userEvent.setup();
    render(SkillViewer, {
      skill: createSkill(),
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    expect(await screen.findByRole("button", { name: "SKILL.md" })).toBeInTheDocument();
    expect(document.querySelector(".file-viewer-identity .file-viewer-navigation-boundary-toggle")).toBeNull();

    await user.click(screen.getByRole("button", { name: "收起文件树" }));

    expect(document.querySelector(".file-workspace-nav")).toBeNull();
    expect(document.querySelector(".file-viewer-shell")).toHaveClass("navigation-collapsed");
    expect(document.querySelector(".file-viewer-subtitle")?.textContent).toBe("~/.codex/skills/demo-skill/SKILL.md");

    await user.click(screen.getByRole("button", { name: "展开文件树" }));

    expect(document.querySelector(".file-workspace-nav")).not.toBeNull();
    expect(await screen.findByRole("button", { name: "SKILL.md" })).toBeInTheDocument();
  });

  it("shows state feedback without file navigation when a Skill has no displayable files", async () => {
    apiMocks.readSkillContent.mockResolvedValue({});
    apiMocks.listSkillFiles.mockResolvedValue([]);

    render(SkillViewer, {
      skill: createSkill(),
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await waitFor(() => {
      expect(screen.getAllByText("暂无可读文件").length).toBeGreaterThan(0);
    });
    expect(screen.getByText("这个 Skill 没有应用可显示的文件内容。")).toBeInTheDocument();
    expect(document.querySelector(".file-workspace-nav")).toBeNull();
    expect(screen.queryByRole("separator", { name: "拖动调整文件树宽度" })).not.toBeInTheDocument();
    expect(document.querySelector(".file-viewer-shell")).toHaveClass("variant-single-file");
    expect(screen.queryByRole("button", { name: "编辑" })).not.toBeInTheDocument();
  });

  it("clears prior Skill content and source actions after switching to a Skill without a valid source", async () => {
    const user = userEvent.setup();
    apiMocks.listSkillFiles.mockResolvedValue([{ relative_path: "SKILL.md", is_dir: false }]);
    apiMocks.readSkillContent.mockResolvedValue({ skill_md_content: "# Source" });

    const sourceSkill = createSkill({
      name: "source-skill",
      display_name: "Source Skill",
      path: "/shared/source-skill",
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: "/shared/source-skill",
          path_origin: "generic",
        },
        {
          tool_id: "cursor",
          tool_name: "Cursor",
          status: "notInstalled",
          path: "",
        },
      ],
    });
    const detachedSkill = createSkill({
      name: "detached-skill",
      display_name: "Detached Skill",
      path: "",
      tool_statuses: [],
    });

    const { rerender } = render(SkillViewer, {
      skill: sourceSkill,
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    expect(await screen.findByText("Source")).toBeInTheDocument();
    expect(document.querySelector(".file-viewer-subtitle")?.textContent).toBe("/shared/source-skill/SKILL.md");

    await rerender({
      skill: detachedSkill,
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await waitFor(() => {
      expect(screen.getByText("暂无可读文件")).toBeInTheDocument();
    });
    expect(screen.queryByText("Source")).not.toBeInTheDocument();
    expect(document.querySelector(".file-viewer-subtitle")?.textContent || "").not.toContain("/shared/source-skill/SKILL.md");

    await user.click(screen.getByRole("button", { name: "管理" }));
    expect(document.querySelector(".tool-status-item .btn-action-install")).toBeNull();
    expect(screen.queryByRole("button", { name: "从其他工具复制" })).not.toBeInTheDocument();
  });

  it("ignores late inventory responses from the previously opened Skill", async () => {
    const user = userEvent.setup();
    const pendingInventory = createDeferred();
    apiMocks.scanSkillInventory.mockReturnValue(pendingInventory.promise);
    apiMocks.listSkillFiles.mockResolvedValue([{ relative_path: "SKILL.md", is_dir: false }]);
    apiMocks.readSkillContent.mockResolvedValue({ skill_md_content: "# Source" });

    const sourceSkill = createSkill({
      name: "source-skill",
      display_name: "Source Skill",
      path: "/shared/source-skill",
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: "/shared/source-skill",
          path_origin: "generic",
        },
        {
          tool_id: "cursor",
          tool_name: "Cursor",
          status: "notInstalled",
          path: "",
        },
      ],
    });
    const detachedSkill = createSkill({
      name: "detached-skill",
      display_name: "Detached Skill",
      path: "",
      tool_statuses: [],
    });

    const { rerender } = render(SkillViewer, {
      skill: sourceSkill,
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    expect(await screen.findByText("Source")).toBeInTheDocument();
    await rerender({
      skill: detachedSkill,
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    pendingInventory.resolve({
      skills: [
        {
          name: "source-skill",
          toolStatuses: sourceSkill.tool_statuses,
        },
      ],
    });

    await waitFor(() => {
      expect(apiMocks.scanSkillInventory).toHaveBeenCalledTimes(1);
    });
    await user.click(screen.getByRole("button", { name: "管理" }));

    expect(document.querySelector(".tool-status-item .btn-action-install")).toBeNull();
    expect(screen.queryByRole("button", { name: "从其他工具复制" })).not.toBeInTheDocument();
  });

  it("keeps normal tool-directory entries unlabeled and hides removed source-management actions", async () => {
    const user = userEvent.setup();
    render(SkillViewer, {
      skill: createSkill({
        tool_statuses: [
          {
            tool_id: "codex",
            tool_name: "Codex",
            status: "installed",
            path: "/tmp/codex-skills/demo-skill",
            path_origin: "tool",
          },
        ],
        installed_in: [{ tool_id: "codex", mode: "copy", path: "/tmp/codex-skills/demo-skill" }],
      }),
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    expect(await screen.findByText("Demo Skill")).toBeInTheDocument();
    expect(screen.queryByText("本地来源")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "管理" }));

    expect(screen.getByText("/tmp/codex-skills/demo-skill")).toBeInTheDocument();
    expect(screen.queryByText("工具目录来源")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "删除" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /切换为|解除/ })).not.toBeInTheDocument();
    expect(screen.queryByText(/统一版本|独立版本/)).not.toBeInTheDocument();
  });

  it("refreshes open detail sources and actions after deleting a tool-directory source", async () => {
    const user = userEvent.setup();
    setModulePerformanceDiagnosticsEnabledState(true);
    const codexPath = "/tmp/codex-skills/github-auth";
    const hermesPath = "/tmp/hermes-skills/github-auth";
    const initialSkill = createSkill({
      name: "github-auth",
      display_name: "github-auth",
      path: codexPath,
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: codexPath,
          path_origin: "tool",
        },
        {
          tool_id: "hermes-agent",
          tool_name: "Hermes Agent",
          status: "installed",
          path: hermesPath,
          path_origin: "tool",
        },
        {
          tool_id: "claude-code",
          tool_name: "Claude Code",
          status: "notInstalled",
          path: "",
        },
      ],
    });
    const refreshedSkill = createSkill({
      name: "github-auth",
      display_name: "github-auth",
      path: hermesPath,
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "notInstalled",
          path: "",
        },
        {
          tool_id: "hermes-agent",
          tool_name: "Hermes Agent",
          status: "installed",
          path: hermesPath,
          path_origin: "tool",
        },
        {
          tool_id: "claude-code",
          tool_name: "Claude Code",
          status: "notInstalled",
          path: "",
        },
      ],
    });

    setToolsForTest([
      { id: "codex", name: "Codex", detected: true, skills_dir: "/tmp/codex-skills" },
      { id: "hermes-agent", name: "Hermes Agent", detected: true, skills_dir: "/tmp/hermes-skills" },
      { id: "claude-code", name: "Claude Code", detected: true, skills_dir: "/tmp/claude-skills" },
    ]);
    apiMocks.scanSkillInventory
      .mockResolvedValueOnce({ skills: [initialSkill] });
    apiMocks.scanSkillInventoryEntry.mockResolvedValue(refreshedSkill);
    apiMocks.previewDeleteFromTool.mockResolvedValue({ deletes: [codexPath] });
    apiMocks.executeDeleteFromTool.mockResolvedValue({ deletes: [codexPath] });

    render(SkillViewer, {
      skill: initialSkill,
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    const sourceGroup = await screen.findByRole("group", { name: "来源" });
    expect(within(sourceGroup).getByRole("button", { name: "Codex" })).toBeInTheDocument();
    expect(within(sourceGroup).getByRole("button", { name: "Hermes Agent" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "管理" }));
    const rowsBeforeDelete = Array.from(document.querySelectorAll(".tool-status-item"));
    const codexRowBeforeDelete = rowsBeforeDelete.find((row) => row.textContent?.includes("Codex"));
    expect(codexRowBeforeDelete).toBeTruthy();
    await user.click(within(codexRowBeforeDelete).getByRole("button", { name: "删除" }));
    await waitFor(() => {
      expect(apiMocks.previewDeleteFromTool).toHaveBeenCalledWith("github-auth", "codex", codexPath);
    });
    await user.click(screen.getByRole("button", { name: "确认执行" }));
    await waitFor(() => {
      expect(apiMocks.executeDeleteFromTool).toHaveBeenCalledWith("github-auth", "codex", codexPath);
    });
    expect(apiMocks.scanSkillInventoryEntry).toHaveBeenCalledWith("github-auth");
    expect(apiMocks.scanSkillInventory).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole("button", { name: "内容" }));
    await waitFor(() => {
      expect(document.querySelector(".file-viewer-subtitle")?.textContent).toBe(`${hermesPath}/SKILL.md`);
    });
    expect(screen.queryByRole("button", { name: "共享目录" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Codex" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "管理" }));
    const rowsAfterDelete = Array.from(document.querySelectorAll(".tool-status-item"));
    const codexRowAfterDelete = rowsAfterDelete.find((row) => row.textContent?.includes("Codex"));
    expect(codexRowAfterDelete).toBeTruthy();
    expect(within(codexRowAfterDelete).queryByRole("button", { name: "安装" })).not.toBeInTheDocument();
    expect(within(codexRowAfterDelete).getByRole("button", { name: "从其他工具复制" })).toBeInTheDocument();
    await waitFor(() => {
      expect(loggingApiMocks.writeModulePerformanceLog).toHaveBeenCalledWith(expect.objectContaining({
        module: "skills",
        view: "detail",
        reason: "delete-from-tool",
        status: "success",
        requestCounts: expect.objectContaining({
          "delete-preview": 1,
          "delete-execute": 1,
          "detail-refresh": 1,
        }),
      }));
    });
  });

  it("keeps previous detail state and records failure when post-delete refresh fails", async () => {
    const user = userEvent.setup();
    setModulePerformanceDiagnosticsEnabledState(true);
    const codexPath = "/tmp/codex-skills/github-auth";
    const hermesPath = "/tmp/hermes-skills/github-auth";
    const initialSkill = createSkill({
      name: "github-auth",
      display_name: "github-auth",
      path: codexPath,
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: codexPath,
          path_origin: "tool",
        },
        {
          tool_id: "hermes-agent",
          tool_name: "Hermes Agent",
          status: "installed",
          path: hermesPath,
          path_origin: "tool",
        },
      ],
    });

    setToolsForTest([
      { id: "codex", name: "Codex", detected: true, skills_dir: "/tmp/codex-skills" },
      { id: "hermes-agent", name: "Hermes Agent", detected: true, skills_dir: "/tmp/hermes-skills" },
    ]);
    apiMocks.scanSkillInventory.mockResolvedValueOnce({ skills: [initialSkill] });
    apiMocks.scanSkillInventoryEntry.mockRejectedValue(new Error("scan failed"));
    apiMocks.previewDeleteFromTool.mockResolvedValue({ deletes: [codexPath] });
    apiMocks.executeDeleteFromTool.mockResolvedValue({ deletes: [codexPath] });
    const onChanged = vi.fn();

    render(SkillViewer, {
      skill: initialSkill,
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged,
    });

    await user.click(screen.getByRole("button", { name: "管理" }));
    const rowsBeforeDelete = Array.from(document.querySelectorAll(".tool-status-item"));
    const codexRowBeforeDelete = rowsBeforeDelete.find((row) => row.textContent?.includes("Codex"));
    expect(codexRowBeforeDelete).toBeTruthy();
    await user.click(within(codexRowBeforeDelete).getByRole("button", { name: "删除" }));
    await user.click(screen.getByRole("button", { name: "确认执行" }));

    await waitFor(() => {
      expect(apiMocks.executeDeleteFromTool).toHaveBeenCalledWith("github-auth", "codex", codexPath);
      expect(apiMocks.scanSkillInventoryEntry).toHaveBeenCalledWith("github-auth");
      expect(screen.getByText("删除失败：操作已执行，但刷新最新状态失败。请手动刷新后再继续。")).toBeInTheDocument();
    });
    expect(onChanged).not.toHaveBeenCalled();
    const rowsAfterFailure = Array.from(document.querySelectorAll(".tool-status-item"));
    const codexRowAfterFailure = rowsAfterFailure.find((row) => row.textContent?.includes("Codex"));
    expect(codexRowAfterFailure).toBeTruthy();
    expect(within(codexRowAfterFailure).getByRole("button", { name: "删除" })).toBeInTheDocument();
    await waitFor(() => {
      expect(loggingApiMocks.writeModulePerformanceLog).toHaveBeenCalledWith(expect.objectContaining({
        module: "skills",
        view: "detail",
        reason: "delete-from-tool",
        status: "failed",
        requestCounts: expect.objectContaining({
          "delete-preview": 1,
          "delete-execute": 1,
          "detail-refresh": 1,
        }),
      }));
    });
  });

  it("deduplicates a tool-opened shared link in content while keeping uninstall available", async () => {
    const user = userEvent.setup();
    setToolsForTest([
      { id: "claude-code", name: "Claude Code", detected: true, skills_dir: "/tmp/claude-skills" },
      { id: "cursor", name: "Cursor", detected: true, skills_dir: "/tmp/cursor-skills" },
    ]);
    apiMocks.listSkillFiles.mockResolvedValue([{ relative_path: "SKILL.md", is_dir: false }]);
    apiMocks.readSkillContent.mockResolvedValue({ skill_md_content: "# Shared Docx" });

    render(SkillViewer, {
      skill: createSkill({
        name: "docx",
        display_name: "docx",
        path: "/tmp/claude-skills/docx",
        tool_statuses: [
          {
            tool_id: "claude-code",
            tool_name: "Claude Code",
            status: "variantInstalledSymlink",
            path: "/tmp/claude-skills/docx",
            path_origin: "tool",
            symlink_target: "/shared/docx",
          },
          {
            tool_id: "cursor",
            tool_name: "Cursor",
            status: "variantInstalledCopy",
            path: "/tmp/cursor-skills/docx",
            path_origin: "tool",
          },
        ],
      }),
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await waitFor(() => {
      expect(apiMocks.listSkillFiles).toHaveBeenCalledWith("/shared/docx");
    });
    expect(document.querySelector(".file-viewer-subtitle")?.textContent).toBe("/shared/docx/SKILL.md");
    const sourceGroup = await screen.findByRole("group", { name: "来源" });
    expect(within(sourceGroup).getAllByRole("button", { name: "共享目录" })).toHaveLength(1);
    expect(within(sourceGroup).getByRole("button", { name: "Cursor" })).toBeInTheDocument();
    expect(screen.getByText("当前被 Claude Code 使用")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "管理" }));
    expect(await screen.findByRole("button", { name: "卸载" })).toBeInTheDocument();
  });

  it("labels duplicate tool links separately from their shared target", async () => {
    const user = userEvent.setup();
    setToolsForTest([
      { id: "claude-code", name: "Claude Code", detected: true, skills_dir: "/tmp/claude-skills" },
    ]);
    const toolPath = "/tmp/claude-skills/docx";
    const sharedPath = "/shared/docx";

    render(SkillViewer, {
      skill: createSkill({
        name: "docx",
        display_name: "docx",
        path: sharedPath,
        tool_statuses: [
          {
            tool_id: "claude-code",
            tool_name: "Claude Code",
            status: "variantInstalledSymlink",
            path: toolPath,
            path_origin: "tool",
            symlink_target: sharedPath,
            abnormal_state: "duplicate_sources",
            sources: [
              {
                tool_id: "claude-code",
                tool_name: "Claude Code",
                status: "variantInstalledSymlink",
                path: toolPath,
                path_origin: "tool",
                symlink_target: sharedPath,
              },
              {
                tool_id: "claude-code",
                tool_name: "Claude Code",
                status: "variantInstalledCopy",
                path: sharedPath,
                path_origin: "generic",
              },
            ],
          },
        ],
      }),
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));
    await user.click(await screen.findByRole("button", { name: "处理异常" }));

    const modal = document.querySelector(".abnormal-modal");
    expect(modal).toBeTruthy();
    const rows = Array.from(modal.querySelectorAll(".abnormal-source-item"));
    const toolRow = rows.find((row) => row.textContent?.includes(toolPath));
    const sharedRow = rows.find((row) => row.textContent?.includes(sharedPath) && !row.textContent?.includes(toolPath));

    expect(toolRow).toHaveTextContent("Claude Code");
    expect(toolRow).not.toHaveTextContent("共享目录");
    expect(sharedRow).toHaveTextContent("共享目录");
    expect(sharedRow).toHaveTextContent(sharedPath);
  });

  it("shows metadata drift as diagnostic-only without delete action", async () => {
    const user = userEvent.setup();
    render(SkillViewer, {
      skill: createSkill({
        tool_statuses: [
          {
            tool_id: "codex",
            tool_name: "Codex",
            status: "variantDrifted",
            path: "/tmp/codex-skills/demo-skill",
            path_origin: "tool",
          },
        ],
      }),
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));

    expect(await screen.findByText("Skill 内容与工具元数据不一致，请在工具内确认后处理。")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "删除" })).not.toBeInTheDocument();
  });

  it("shows duplicate source paths and resolves them through confirmation", async () => {
    const user = userEvent.setup();
    const topPath = "/tmp/codex-skills/executing-plans";
    const nestedPath = "/tmp/codex-skills/superpowers/skills/executing-plans";
    apiMocks.previewCleanupDuplicateSkillSources.mockResolvedValue({ deletes: [nestedPath] });
    apiMocks.executeCleanupDuplicateSkillSources.mockResolvedValue({ deletes: [nestedPath] });

    render(SkillViewer, {
      skill: createSkill({
        name: "executing-plans",
        display_name: "executing-plans",
        path: topPath,
        tool_statuses: [
          {
            tool_id: "codex",
            tool_name: "Codex",
            status: "installed",
            path: topPath,
            path_origin: "tool",
            abnormal_state: "duplicate_sources",
            sources: [
              {
                tool_id: "codex",
                tool_name: "Codex",
                status: "installed",
                path: topPath,
                path_origin: "tool",
              },
              {
                tool_id: "codex",
                tool_name: "Codex",
                status: "installed",
                path: nestedPath,
                path_origin: "tool",
              },
            ],
          },
        ],
      }),
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));

    expect(await screen.findByText(topPath)).toBeInTheDocument();
    expect(screen.getByText(nestedPath)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "删除" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "处理异常" }));

    await screen.findByText("选择一个来源保留，应用会通过确认流程删除其他同名来源；也可以把选中的来源重命名为新 Skill。");
    expect(screen.queryByText("内容一致")).not.toBeInTheDocument();
    expect(screen.queryByText("内容不同")).not.toBeInTheDocument();
    expect(apiMocks.readSkillFile).not.toHaveBeenCalledWith(topPath, "SKILL.md");
    expect(apiMocks.readSkillFile).not.toHaveBeenCalledWith(nestedPath, "SKILL.md");

    const keepButton = screen.getByRole("button", { name: "保留此来源" });
    expect(keepButton).not.toHaveClass("primary-pill-btn--danger");
    await user.click(keepButton);
    await waitFor(() => {
      expect(apiMocks.previewCleanupDuplicateSkillSources).toHaveBeenCalledWith(
        "executing-plans",
        topPath,
        [nestedPath]
      );
    });
    await user.click(screen.getByRole("button", { name: "执行" }));
    await waitFor(() => {
      expect(apiMocks.executeCleanupDuplicateSkillSources).toHaveBeenCalledWith(
        "executing-plans",
        topPath,
        [nestedPath]
      );
    });
  });

  it("distinguishes duplicate content sources with source capsule badges", async () => {
    const topPath = "/tmp/codex-skills/executing-plans";
    const nestedPath = "/tmp/codex-skills/superpowers/skills/executing-plans";

    render(SkillViewer, {
      skill: createSkill({
        name: "executing-plans",
        display_name: "executing-plans",
        path: topPath,
        tool_statuses: [
          {
            tool_id: "codex",
            tool_name: "Codex",
            status: "installed",
            path: topPath,
            path_origin: "tool",
            abnormal_state: "duplicate_sources",
            sources: [
              { tool_id: "codex", tool_name: "Codex", status: "installed", path: topPath, path_origin: "tool" },
              { tool_id: "codex", tool_name: "Codex", status: "installed", path: nestedPath, path_origin: "tool" },
            ],
          },
        ],
      }),
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    const sourceGroup = await screen.findByRole("group", { name: "来源" });
    const firstSource = within(sourceGroup).getByRole("button", { name: "Codex 1" });
    const secondSource = within(sourceGroup).getByRole("button", { name: "Codex 2" });

    expect(firstSource).toHaveClass("is-abnormal");
    expect(secondSource).toHaveClass("is-abnormal");
    expect(firstSource).not.toHaveAttribute("title");
    expect(secondSource).not.toHaveAttribute("title");
    expect(within(sourceGroup).getAllByText("1").length).toBeGreaterThan(0);
    expect(within(sourceGroup).getAllByText("2").length).toBeGreaterThan(0);
  });

  it("disables duplicate cleanup for read-only duplicate sources", async () => {
    const user = userEvent.setup();
    const topPath = "/tmp/codex-skills/executing-plans";
    const nestedPath = "/tmp/codex-skills/superpowers/skills/executing-plans";
    setToolsForTest([
      {
        id: "codex",
        name: "Codex",
        detected: true,
        skills_dir: "/tmp/codex-skills",
        capabilities: [skillCapability({ id: "codex", name: "Codex", skills_dir: "/tmp/codex-skills" }, "read_only")],
      },
    ]);

    render(SkillViewer, {
      skill: createSkill({
        name: "executing-plans",
        display_name: "executing-plans",
        path: topPath,
        tool_statuses: [
          {
            tool_id: "codex",
            tool_name: "Codex",
            status: "installed",
            path: topPath,
            path_origin: "tool",
            abnormal_state: "duplicate_sources",
            sources: [
              { tool_id: "codex", tool_name: "Codex", status: "installed", path: topPath, path_origin: "tool" },
              { tool_id: "codex", tool_name: "Codex", status: "installed", path: nestedPath, path_origin: "tool" },
            ],
          },
        ],
      }),
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));

    expect(await screen.findByRole("button", { name: "处理异常" })).toBeDisabled();
  });

  it("renames a selected duplicate source through the app-owned confirmation", async () => {
    const user = userEvent.setup();
    const promptSpy = vi.spyOn(window, "prompt");
    const topPath = "/tmp/codex-skills/executing-plans";
    const nestedPath = "/tmp/codex-skills/superpowers/skills/executing-plans";
    apiMocks.previewRenameSkillSource.mockResolvedValue({ creates: ["/tmp/codex-skills/executing-plans-local"], deletes: [topPath] });
    apiMocks.executeRenameSkillSource.mockResolvedValue({ creates: ["/tmp/codex-skills/executing-plans-local"], deletes: [topPath] });

    render(SkillViewer, {
      skill: createSkill({
        name: "executing-plans",
        display_name: "executing-plans",
        path: topPath,
        tool_statuses: [
          {
            tool_id: "codex",
            tool_name: "Codex",
            status: "installed",
            path: topPath,
            path_origin: "tool",
            abnormal_state: "duplicate_sources",
            sources: [
              { tool_id: "codex", tool_name: "Codex", status: "installed", path: topPath, path_origin: "tool" },
              { tool_id: "codex", tool_name: "Codex", status: "installed", path: nestedPath, path_origin: "tool" },
            ],
          },
        ],
      }),
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));
    await user.click(await screen.findByRole("button", { name: "处理异常" }));
    await user.click(await screen.findByRole("button", { name: "重命名此来源" }));
    expect(await screen.findByLabelText("新名称")).toHaveValue("executing-plans-local");
    expect(promptSpy).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "继续" }));

    await waitFor(() => {
      expect(apiMocks.previewRenameSkillSource).toHaveBeenCalledWith(
        "executing-plans",
        topPath,
        "executing-plans-local"
      );
    });
    await user.click(screen.getByRole("button", { name: "执行" }));
    await waitFor(() => {
      expect(apiMocks.executeRenameSkillSource).toHaveBeenCalledWith(
        "executing-plans",
        topPath,
        "executing-plans-local"
      );
    });

    promptSpy.mockRestore();
  });

  it("shows shared-directory impact only in the final duplicate cleanup confirmation", async () => {
    const user = userEvent.setup();
    const toolPath = "/tmp/codex-skills/demo-skill";
    const sharedPath = "/shared/demo-skill";
    apiMocks.previewCleanupDuplicateSkillSources.mockResolvedValue({
      message: "删除共享目录来源可能影响其他使用共享目录的工具。",
      deletes: [sharedPath],
    });
    apiMocks.executeCleanupDuplicateSkillSources.mockResolvedValue({ deletes: [sharedPath] });

    render(SkillViewer, {
      skill: createSkill({
        path: sharedPath,
        tool_statuses: [
          {
            tool_id: "codex",
            tool_name: "Codex",
            status: "installed",
            path: toolPath,
            path_origin: "tool",
            abnormal_state: "duplicate_sources",
            sources: [
              { tool_id: "codex", tool_name: "Codex", status: "installed", path: toolPath, path_origin: "tool" },
              { tool_id: "codex", tool_name: "Codex", status: "installed", path: sharedPath, path_origin: "generic" },
            ],
          },
        ],
      }),
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));
    expect(screen.queryByText("删除共享目录来源可能影响其他使用共享目录的工具。")).not.toBeInTheDocument();

    await user.click(await screen.findByRole("button", { name: "处理异常" }));
    expect(screen.queryByText("删除共享目录来源可能影响其他使用共享目录的工具。")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "保留此来源" }));
    expect(await screen.findByText("删除共享目录来源可能影响其他使用共享目录的工具。")).toBeInTheDocument();
  });

  it("installs shared-directory sources by symlink", async () => {
    const user = userEvent.setup();
    const skill = createSkill({
      path: "/shared/demo-skill",
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "variantInstalledSymlink",
          path: "/tmp/codex-skills/demo-skill",
          path_origin: "generic",
          symlink_target: "/shared/demo-skill",
        },
        {
          tool_id: "cursor",
          tool_name: "Cursor",
          status: "notInstalled",
          path: "",
        },
      ],
    });
    apiMocks.scanSkillInventory.mockResolvedValue({ skills: [skill] });

    render(SkillViewer, {
      skill,
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));

    let installButton;
    await waitFor(() => {
      installButton = screen.getByRole("button", { name: "安装" });
      expect(installButton).toBeInTheDocument();
    });
    expect(screen.queryByRole("button", { name: "从其他工具复制" })).not.toBeInTheDocument();
    await user.click(installButton);
    await waitFor(() => {
      expect(apiMocks.previewInstall).toHaveBeenCalledWith("demo-skill", "cursor", "symlink", "/shared/demo-skill");
    });
    await user.click(screen.getByRole("button", { name: "执行" }));
    await waitFor(() => {
      expect(apiMocks.executeInstall).toHaveBeenCalledWith("demo-skill", "cursor", "symlink", "/shared/demo-skill");
    });
    expect(apiMocks.previewCopySkillToTool).not.toHaveBeenCalled();
  });

  it("keeps file tree viewing and edit affordances available", async () => {
    const user = userEvent.setup();
    apiMocks.listSkillFiles.mockResolvedValue([
      { relative_path: "SKILL.md", is_dir: false },
      { relative_path: "docs", is_dir: true },
      { relative_path: "docs/guide.md", is_dir: false },
    ]);
    apiMocks.readSkillContent.mockResolvedValue({ skill_md_content: "# Demo" });
    apiMocks.readSkillFile.mockResolvedValue("# Guide");

    const { container } = render(SkillViewer, {
      skill: createSkill({
        path: "/shared/demo-skill",
        tool_statuses: [
          {
            tool_id: "codex",
            tool_name: "Codex",
            status: "variantInstalledSymlink",
            path: "/shared/demo-skill",
            path_origin: "generic",
          },
        ],
      }),
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    expect(await screen.findByText("文件")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "docs" }));
    await user.click(screen.getByRole("button", { name: "guide.md" }));

    await waitFor(() => {
      expect(apiMocks.readSkillFile).toHaveBeenCalledWith("/shared/demo-skill", "docs/guide.md");
    });
    expect(document.querySelector(".file-viewer-subtitle")?.textContent).toBe("/shared/demo-skill/docs/guide.md");
    expect(await screen.findByText("Guide")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "编辑" }));
    expect(screen.getByRole("button", { name: "保存" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "取消" })).toBeInTheDocument();
  });

  it("searches the current Skill file tree and opens a matching file", async () => {
    const user = userEvent.setup();
    apiMocks.listSkillFiles.mockResolvedValue([
      { relative_path: "SKILL.md", is_dir: false },
      { relative_path: "docs", is_dir: true },
      { relative_path: "docs/guide.md", is_dir: false },
    ]);
    apiMocks.readSkillContent.mockResolvedValue({ skill_md_content: "# Demo" });
    apiMocks.readSkillFile.mockImplementation((_skillPath, relativePath) => {
      if (relativePath === "docs/guide.md") return Promise.resolve("# Guide\n\nneedle");
      return Promise.resolve("# Demo");
    });

    const { container } = render(SkillViewer, {
      skill: createSkill({
        path: "/shared/demo-skill",
        tool_statuses: [
          {
            tool_id: "codex",
            tool_name: "Codex",
            status: "variantInstalledSymlink",
            path: "/shared/demo-skill",
            path_origin: "generic",
          },
        ],
      }),
      initialTab: "content",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    expect(await screen.findByText("文件")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "搜索" }));
    await user.type(screen.getByPlaceholderText("搜索当前文件树"), "needle");
    expect(container.querySelector(".skill-viewer-management-header")).toHaveClass("skill-viewer-management-header--search-popover");

    await screen.findByRole("region", { name: "当前文件树搜索结果" });
    await user.click(screen.getByText("Demo Skill"));
    expect(screen.queryByRole("region", { name: "当前文件树搜索结果" })).not.toBeInTheDocument();
    await user.click(screen.getByPlaceholderText("搜索当前文件树"));
    const reopenedRegion = await screen.findByRole("region", { name: "当前文件树搜索结果" });
    const guideResult = await within(reopenedRegion).findByRole("button", { name: /guide\.md/ });
    expect(guideResult).toHaveTextContent("needle");
    await user.click(guideResult);
    expect(screen.queryByRole("region", { name: "当前文件树搜索结果" })).not.toBeInTheDocument();

    await waitFor(() => {
      expect(apiMocks.readSkillFile).toHaveBeenCalledWith("/shared/demo-skill", "docs/guide.md");
      expect(document.querySelector(".file-viewer-subtitle")?.textContent).toBe("/shared/demo-skill/docs/guide.md");
    });
    expect(document.querySelector(".md-rendered")?.textContent).toContain("needle");
    expect(document.querySelector(".workspace-search-highlight--active")?.textContent).toBe("needle");
  });

  it("uses real tool-directory copy even when a tool supports external symlinks", async () => {
    const user = userEvent.setup();
    setModulePerformanceDiagnosticsEnabledState(true);
    setToolsForTest([
      { id: "codex", name: "Codex", detected: true, skills_dir: "/tmp/codex-skills" },
      {
        id: "cursor",
        name: "Cursor",
        detected: true,
        skills_dir: "/tmp/cursor-skills",
        allow_external_generic_symlink: true,
      },
    ]);
    const skill = createSkill({
      path: "/tmp/codex-skills/demo-skill",
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: "/tmp/codex-skills/demo-skill",
          path_origin: "tool",
        },
        {
          tool_id: "cursor",
          tool_name: "Cursor",
          status: "notInstalled",
          path: "",
        },
      ],
    });
    const copiedSkill = createSkill({
      path: "/tmp/cursor-skills/demo-skill",
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: "/tmp/codex-skills/demo-skill",
          path_origin: "tool",
        },
        {
          tool_id: "cursor",
          tool_name: "Cursor",
          status: "installed",
          path: "/tmp/cursor-skills/demo-skill",
          path_origin: "tool",
        },
      ],
    });
    apiMocks.scanSkillInventory.mockResolvedValue({ skills: [skill] });
    apiMocks.scanSkillInventoryEntry.mockResolvedValue(copiedSkill);
    apiMocks.previewCopySkillToTool.mockResolvedValue({ creates: ["/tmp/cursor-skills/demo-skill"] });

    let currentSkill = skill;
    /** @type {(props: any) => Promise<void>} */
    let rerenderViewer = async () => {};
    const onChanged = vi.fn((changedSkill) => {
      currentSkill = createSkill({
        ...currentSkill,
        path: changedSkill?.path || currentSkill.path,
        tool_statuses: changedSkill?.tool_statuses || changedSkill?.toolStatuses || currentSkill.tool_statuses,
      });
      return rerenderViewer({
        skill: currentSkill,
        initialTab: "install",
        onClose: vi.fn(),
        onDelete: vi.fn(),
        onChanged,
      });
    });
    const rendered = render(SkillViewer, {
      skill: currentSkill,
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged,
    });
    rerenderViewer = rendered.rerender;

    await user.click(screen.getByRole("button", { name: "管理" }));
    const copyButton = await screen.findByRole("button", { name: "从其他工具复制" });
    expect(screen.queryByLabelText(noSharedInstallWarning)).not.toBeInTheDocument();
    await user.click(copyButton);

    await waitFor(() => {
      expect(apiMocks.previewCopySkillToTool).toHaveBeenCalledWith("demo-skill", "cursor", "/tmp/codex-skills/demo-skill");
    });
    await user.click(screen.getByRole("button", { name: "执行" }));
    await waitFor(() => {
      expect(apiMocks.executeCopySkillToTool).toHaveBeenCalledWith("demo-skill", "cursor", "/tmp/codex-skills/demo-skill");
    });
    expect(apiMocks.scanSkillInventoryEntry).toHaveBeenCalledWith("demo-skill");
    expect(apiMocks.scanSkillInventory).toHaveBeenCalledTimes(1);
    expect(onChanged).toHaveBeenCalledWith(expect.objectContaining({ path: "/tmp/cursor-skills/demo-skill" }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "管理" })).toHaveClass("active");
      expect(screen.getByRole("button", { name: "内容" })).not.toHaveClass("active");
    });
    await waitFor(() => {
      expect(loggingApiMocks.writeModulePerformanceLog).toHaveBeenCalledWith(expect.objectContaining({
        module: "skills",
        view: "detail",
        reason: "copy-from-tool",
        status: "success",
        requestCounts: expect.objectContaining({
          "source-refresh": 1,
          "copy-preview": 1,
          "copy-execute": 1,
          "detail-refresh": 1,
        }),
      }));
    });
  });

  it("does not offer copy from another tool when the only tool source is a shared-backed link", async () => {
    const user = userEvent.setup();
    setToolsForTest([
      { id: "claude-code", name: "Claude Code", detected: true, skills_dir: "/tmp/claude-skills" },
      { id: "codex", name: "Codex", detected: true, skills_dir: "/tmp/codex-skills" },
      { id: "workbuddy", name: "WorkBuddy", detected: true, skills_dir: "/tmp/workbuddy-skills" },
    ]);
    const sharedPath = "/shared/docx";
    const skill = createSkill({
      name: "docx",
      display_name: "docx",
      path: sharedPath,
      tool_statuses: [
        {
          tool_id: "claude-code",
          tool_name: "Claude Code",
          status: "variantInstalledSymlink",
          path: "/tmp/claude-skills/docx",
          path_origin: "tool",
          symlink_target: sharedPath,
        },
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: sharedPath,
          path_origin: "generic",
        },
        {
          tool_id: "workbuddy",
          tool_name: "WorkBuddy",
          status: "notInstalled",
          path: "",
        },
      ],
    });
    apiMocks.scanSkillInventory.mockResolvedValue({ skills: [skill] });

    render(SkillViewer, {
      skill,
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));
    const rows = Array.from(document.querySelectorAll(".tool-status-item"));
    const workbuddyRow = rows.find((row) => row.textContent?.includes("WorkBuddy"));
    expect(workbuddyRow).toBeTruthy();
    expect(within(workbuddyRow).getByRole("button", { name: "安装" })).toBeInTheDocument();
    expect(within(workbuddyRow).queryByRole("button", { name: "从其他工具复制" })).not.toBeInTheDocument();
  });

  it("formats copy source timestamps as local date time", async () => {
    const user = userEvent.setup();
    const rawTimestamp = "2026-05-31T18:37:47.719910510+00:00";
    const expectedTimestamp = expectedLocalDateTime(rawTimestamp);
    setToolsForTest([
      { id: "claude-code", name: "Claude Code", detected: true, skills_dir: "/tmp/claude-skills" },
      { id: "cursor", name: "Cursor", detected: true, skills_dir: "/tmp/cursor-skills" },
      { id: "codex", name: "Codex", detected: true, skills_dir: "/tmp/codex-skills" },
    ]);
    const skill = createSkill({
      name: "audiocraft",
      display_name: "audiocraft-audio-generation",
      path: "/tmp/claude-skills/audiocraft",
      tool_statuses: [
        {
          tool_id: "claude-code",
          tool_name: "Claude Code",
          status: "installed",
          path: "/tmp/claude-skills/audiocraft",
          path_origin: "tool",
          updated_at: rawTimestamp,
        },
        {
          tool_id: "cursor",
          tool_name: "Cursor",
          status: "installed",
          path: "/tmp/cursor-skills/audiocraft",
          path_origin: "tool",
        },
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "notInstalled",
          path: "",
        },
      ],
    });
    apiMocks.scanSkillInventory.mockResolvedValue({ skills: [skill] });

    render(SkillViewer, {
      skill,
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));
    await user.click(await screen.findByRole("button", { name: "从其他工具复制" }));

    await screen.findByText("选择来源");
    const picker = document.querySelector(".source-picker-modal");
    expect(picker).toBeTruthy();
    expect(within(picker).getByText(expectedTimestamp)).toBeInTheDocument();
    expect(within(picker).getByText("当前来源")).toBeInTheDocument();
    expect(within(picker).queryByText(rawTimestamp)).not.toBeInTheDocument();
    expect(within(picker).queryByText(/T18:37:47/)).not.toBeInTheDocument();
    expect(within(picker).queryByText(/\+00:00/)).not.toBeInTheDocument();
  });

  it("offers OpenClaw as a writable Skill copy target when capability metadata allows it", async () => {
    const user = userEvent.setup();
    setToolsForTest([
      { id: "codex", name: "Codex", detected: true, skills_dir: "/tmp/codex-skills" },
      { id: "openclaw", name: "OpenClaw", detected: true, skills_dir: "/tmp/openclaw-skills" },
    ]);
    const skill = createSkill({
      path: "/shared/demo-skill",
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: "/tmp/codex-skills/demo-skill",
          path_origin: "tool",
        },
        {
          tool_id: "openclaw",
          tool_name: "OpenClaw",
          status: "notInstalled",
          path: "",
        },
      ],
    });
    apiMocks.scanSkillInventory.mockResolvedValue({ skills: [skill] });
    apiMocks.previewCopySkillToTool.mockResolvedValue({ creates: ["/tmp/openclaw-skills/demo-skill"] });

    render(SkillViewer, {
      skill,
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));
    await user.click(await screen.findByRole("button", { name: "从其他工具复制" }));

    await waitFor(() => {
      expect(apiMocks.previewCopySkillToTool).toHaveBeenCalledWith("demo-skill", "openclaw", "/tmp/codex-skills/demo-skill");
    });
  });

  it("offers shared install from a deployable tool directory without install evidence", async () => {
    const user = userEvent.setup();
    const traeCn = {
      id: "trae-cn",
      name: "Trae CN",
      detected: true,
      skills_dir: "/tmp/trae-cn-skills",
    };
    setToolsForTest([
      { id: "codex", name: "Codex", detected: true, skills_dir: "/tmp/codex-skills" },
      {
        ...traeCn,
        capabilities: [skillCapabilityWithActions(traeCn, ["view", "read", "copy", "delete", "save"])],
      },
    ]);
    const skill = createSkill({
      path: "/shared/demo-skill",
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: "/tmp/codex-skills/demo-skill",
          path_origin: "tool",
        },
        {
          tool_id: "trae-cn",
          tool_name: "Trae CN",
          status: "notInstalled",
          path: "",
        },
      ],
    });
    apiMocks.scanSkillInventory.mockResolvedValue({ skills: [skill] });
    apiMocks.previewCopySkillToTool.mockResolvedValue({ creates: ["/tmp/trae-cn-skills/demo-skill"] });

    render(SkillViewer, {
      skill,
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));
    expect(await screen.findByRole("button", { name: "从其他工具复制" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "安装" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "安装" }));
    await waitFor(() => {
      expect(apiMocks.previewInstall).toHaveBeenCalledWith("demo-skill", "trae-cn", "symlink", "/shared/demo-skill");
    });
    expect(apiMocks.previewCopySkillToTool).not.toHaveBeenCalled();
  });

  it("does not offer shared install when the tool has no deployable Skill directory", async () => {
    setToolsForTest([
      { id: "codex", name: "Codex", detected: true, skills_dir: "/tmp/codex-skills" },
      { id: "unknown-tool", name: "Unknown Tool", detected: true, skills_dir: "" },
    ]);
    const skill = createSkill({
      path: "/shared/demo-skill",
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: "/shared/demo-skill",
          path_origin: "generic",
        },
        {
          tool_id: "unknown-tool",
          tool_name: "Unknown Tool",
          status: "notInstalled",
          path: "",
        },
      ],
    });
    apiMocks.scanSkillInventory.mockResolvedValue({ skills: [skill] });

    render(SkillViewer, {
      skill,
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await screen.findByText("Unknown Tool");
    expect(screen.queryByRole("button", { name: "安装" })).not.toBeInTheDocument();
  });

  it("keeps the unavailable warning when a local-only Skill has no valid target action", async () => {
    setToolsForTest([
      { id: "codex", name: "Codex", detected: true, skills_dir: "/tmp/codex-skills" },
      {
        id: "read-only-tool",
        name: "Read Only Tool",
        detected: true,
        skills_dir: "/tmp/read-only-skills",
        capabilities: [skillCapabilityWithActions(
          { id: "read-only-tool", name: "Read Only Tool", skills_dir: "/tmp/read-only-skills" },
          ["view", "read"]
        )],
      },
    ]);
    const skill = createSkill({
      path: "/tmp/codex-skills/demo-skill",
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: "/tmp/codex-skills/demo-skill",
          path_origin: "tool",
        },
        {
          tool_id: "read-only-tool",
          tool_name: "Read Only Tool",
          status: "notInstalled",
          path: "",
        },
      ],
    });
    apiMocks.scanSkillInventory.mockResolvedValue({ skills: [skill] });

    render(SkillViewer, {
      skill,
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await screen.findByText("Read Only Tool");
    expect(screen.getByLabelText(noSharedInstallWarning)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "从其他工具复制" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "安装" })).not.toBeInTheDocument();
  });

  it("keeps managed tools visible when inventory omits non-deployable Skill targets", async () => {
    setToolsForTest([
      { id: "codex", name: "Codex", detected: true, skills_dir: "/tmp/codex-skills" },
      {
        id: "trae",
        name: "Trae",
        detected: true,
        skills_dir: "",
        capabilities: [{
          id: "skills",
          kind: "skill",
          scope: "tool",
          access: "unknown",
          format: "unknown",
          source_path: "",
          label: "Skills",
        }],
      },
    ]);
    const skill = createSkill({
      path: "/shared/demo-skill",
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: "/shared/demo-skill",
          path_origin: "generic",
        },
      ],
    });
    apiMocks.scanSkillInventory.mockResolvedValue({ skills: [skill] });

    render(SkillViewer, {
      skill,
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await screen.findByText("Trae");
    expect(screen.getByLabelText(noDeployTargetWarning)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "安装" })).not.toBeInTheDocument();
  });

  it("shows install and copy actions together when shared and tool sources both exist", async () => {
    const user = userEvent.setup();
    setToolsForTest([
      { id: "codex", name: "Codex", detected: true, skills_dir: "/tmp/codex-skills" },
      { id: "openclaw", name: "OpenClaw", detected: true, skills_dir: "/tmp/openclaw-skills" },
      { id: "cursor", name: "Cursor", detected: true, skills_dir: "/tmp/cursor-skills" },
    ]);
    const skill = createSkill({
      path: "/shared/demo-skill",
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: "/tmp/codex-skills/demo-skill",
          path_origin: "tool",
        },
        {
          tool_id: "openclaw",
          tool_name: "OpenClaw",
          status: "installed",
          path: "/shared/demo-skill",
          path_origin: "generic",
        },
        {
          tool_id: "cursor",
          tool_name: "Cursor",
          status: "notInstalled",
          path: "",
        },
      ],
    });
    apiMocks.scanSkillInventory.mockResolvedValue({ skills: [skill] });

    render(SkillViewer, {
      skill,
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "安装" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "从其他工具复制" })).toBeInTheDocument();
    });
    expect(screen.queryByLabelText(noSharedInstallWarning)).not.toBeInTheDocument();
  });

  it("uninstalls an existing shared-directory symlink without offering materialization", async () => {
    const user = userEvent.setup();
    const skill = createSkill({
      path: "/shared/demo-skill",
      tool_statuses: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "variantInstalledSymlink",
          path: "/tmp/codex-skills/demo-skill",
          path_origin: "generic",
          target_path: "/shared/demo-skill",
        },
      ],
    });
    apiMocks.scanSkillInventory.mockResolvedValue({ skills: [skill] });

    render(SkillViewer, {
      skill,
      initialTab: "install",
      onClose: vi.fn(),
      onDelete: vi.fn(),
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));
    await waitFor(() => {
      expect(screen.getAllByText("共享目录").length).toBeGreaterThan(0);
    });
    expect(screen.getByRole("button", { name: "卸载" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "转为文件" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "更多操作" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "卸载" }));
    await waitFor(() => {
      expect(apiMocks.previewUninstall).toHaveBeenCalledWith("demo-skill", "codex");
    });
    await user.click(screen.getByRole("button", { name: "确认执行" }));
    await waitFor(() => {
      expect(apiMocks.executeUninstall).toHaveBeenCalledWith("demo-skill", "codex");
    });
    expect(apiMocks.previewCopySkillToTool).not.toHaveBeenCalled();
  });

  it("keeps Skill-level delete behind preview confirmation", async () => {
    const user = userEvent.setup();
    const onDelete = vi.fn();
    const onClose = vi.fn();
    apiMocks.previewDelete.mockResolvedValue({ deletes: ["/shared/demo-skill"] });
    apiMocks.executeDelete.mockResolvedValue({ deletes: ["/shared/demo-skill"] });

    render(SkillViewer, {
      skill: createSkill({ path: "/shared/demo-skill" }),
      initialTab: "install",
      onClose,
      onDelete,
      onChanged: vi.fn(),
    });

    await user.click(screen.getByRole("button", { name: "管理" }));
    await user.click(screen.getByRole("button", { name: "删除 Skill" }));

    await waitFor(() => {
      expect(apiMocks.previewDelete).toHaveBeenCalledWith("demo-skill", true);
    });
    await user.click(screen.getByRole("button", { name: "确认执行" }));
    await waitFor(() => {
      expect(apiMocks.executeDelete).toHaveBeenCalledWith("demo-skill", true);
    });
    expect(onDelete).toHaveBeenCalledWith("demo-skill", ["/shared/demo-skill"]);
    expect(onClose).toHaveBeenCalled();
  });
});
