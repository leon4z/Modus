import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("$lib/shared/api/invoke.js", () => ({
  invoke: invokeMock,
}));

import { listConfigFiles, readConfigFile, saveConfigFile } from "$lib/features/config/api/config.js";
import { getDashboard } from "$lib/features/dashboard/api/dashboard.js";
import {
  exportApplicationLogs,
  exportModulePerformanceLogs,
  listApplicationLogs,
  listModulePerformanceLogs,
  readApplicationLog,
  readModulePerformanceLog,
  writeApplicationLog,
  writeModulePerformanceLog,
} from "$lib/shared/logging/api.js";
import {
  getMcpDiagnostics,
  listMcpConfigSources,
  listMcpServers,
  readMcpServerConfigFragment,
  saveMcpServerConfigFragment,
} from "$lib/features/mcp/api/mcp.js";
import {
  adoptRuleManagementTargets,
  createRuleFile,
  diffRules,
  getManagedRulesState,
  injectDefaultRules,
  leaveRuleManagementTargets,
  listDefaultRules,
  readRuleContent,
  syncManagedRuleTargets,
} from "$lib/features/rules/index.js";
import { getLanguage, getModulePerformanceDiagnosticsEnabled, getRuntimeInfo, setModulePerformanceDiagnosticsEnabled, setTheme } from "$lib/features/settings/index.js";
import { executeInstall, readSkillFile, scanSkillInventory, scanSkillInventoryEntry } from "$lib/features/skills/index.js";
import { getHandledNewToolIds, getManagedTools, listTools, setHandledNewToolIds, setToolPath } from "$lib/features/tools/index.js";

describe("frontend domain API boundaries", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("routes Rules calls through the Rules boundary without changing command names", async () => {
    invokeMock.mockResolvedValueOnce([{ id: "common_rule" }]);
    await expect(listDefaultRules()).resolves.toEqual([{ id: "common_rule" }]);
    expect(invokeMock).toHaveBeenCalledWith("list_default_rules");

    await readRuleContent("/rules.md");
    expect(invokeMock).toHaveBeenLastCalledWith("read_rule_content", { path: "/rules.md" });

    await createRuleFile("codex", "/rules.md", "");
    expect(invokeMock).toHaveBeenLastCalledWith("create_rule_file", {
      toolId: "codex",
      path: "/rules.md",
      content: "",
    });

    await diffRules("a", "left", "b", "right");
    expect(invokeMock).toHaveBeenLastCalledWith("diff_rules", {
      leftContent: "a",
      leftLabel: "left",
      rightContent: "b",
      rightLabel: "right",
    });

    await injectDefaultRules("codex");
    expect(invokeMock).toHaveBeenLastCalledWith("inject_default_rules", { toolId: "codex" });

    await getManagedRulesState();
    expect(invokeMock).toHaveBeenLastCalledWith("get_managed_rules_state");

    await adoptRuleManagementTargets("common_rule", ["codex"]);
    expect(invokeMock).toHaveBeenLastCalledWith("adopt_rule_management_targets", {
      ruleId: "common_rule",
      toolIds: ["codex"],
    });

    await syncManagedRuleTargets(["codex"]);
    expect(invokeMock).toHaveBeenLastCalledWith("sync_managed_rule_targets", {
      toolIds: ["codex"],
    });

    await leaveRuleManagementTargets("common_rule", ["codex"], { removeManagedBlock: true });
    expect(invokeMock).toHaveBeenLastCalledWith("leave_rule_management_targets", {
      ruleId: "common_rule",
      toolIds: ["codex"],
      removeManagedBlock: true,
      dryRun: false,
    });
  });

  it("routes Skills calls through the Skills boundary without changing payload shapes", async () => {
    await scanSkillInventory();
    expect(invokeMock).toHaveBeenLastCalledWith("scan_skill_inventory");

    await scanSkillInventoryEntry("demo");
    expect(invokeMock).toHaveBeenLastCalledWith("scan_skill_inventory_entry", { skillName: "demo" });

    await readSkillFile("/skills/demo", "SKILL.md");
    expect(invokeMock).toHaveBeenLastCalledWith("read_skill_file", {
      skillPath: "/skills/demo",
      relativePath: "SKILL.md",
    });

    await executeInstall("demo", "codex", "symlink", "/skills/demo");
    expect(invokeMock).toHaveBeenLastCalledWith("install_skill_v2", {
      skillName: "demo",
      toolId: "codex",
      mode: "symlink",
      sourcePath: "/skills/demo",
      dryRun: false,
    });
  });

  it("keeps Settings, Tools, Config, MCP, logging, and dashboard calls domain-scoped", async () => {
    await getLanguage();
    expect(invokeMock).toHaveBeenLastCalledWith("get_language");

    await setTheme("dark");
    expect(invokeMock).toHaveBeenLastCalledWith("set_theme", { theme: "dark" });

    await getModulePerformanceDiagnosticsEnabled();
    expect(invokeMock).toHaveBeenLastCalledWith("get_module_performance_diagnostics_enabled");

    await getRuntimeInfo();
    expect(invokeMock).toHaveBeenLastCalledWith("get_runtime_info");

    await setModulePerformanceDiagnosticsEnabled(true);
    expect(invokeMock).toHaveBeenLastCalledWith("set_module_performance_diagnostics_enabled", { enabled: true });

    await listTools();
    expect(invokeMock).toHaveBeenLastCalledWith("list_tools");

    await getManagedTools();
    expect(invokeMock).toHaveBeenLastCalledWith("get_managed_tools");

    await getHandledNewToolIds();
    expect(invokeMock).toHaveBeenLastCalledWith("get_handled_new_tool_ids");

    await setHandledNewToolIds(["cursor"]);
    expect(invokeMock).toHaveBeenLastCalledWith("set_handled_new_tool_ids", { toolIds: ["cursor"] });

    await setToolPath("codex", "/config", "/skills");
    expect(invokeMock).toHaveBeenLastCalledWith("set_tool_path", {
      toolId: "codex",
      configDir: "/config",
      skillsDir: "/skills",
    });

    await listConfigFiles("codex");
    expect(invokeMock).toHaveBeenLastCalledWith("list_config_files", { toolId: "codex" });

    await readConfigFile("codex", "settings");
    expect(invokeMock).toHaveBeenLastCalledWith("read_config_file", {
      toolId: "codex",
      fileId: "settings",
    });

    await saveConfigFile("codex", "settings", "{}");
    expect(invokeMock).toHaveBeenLastCalledWith("save_config_file", {
      toolId: "codex",
      fileId: "settings",
      content: "{}",
    });

    await listMcpServers("codex");
    expect(invokeMock).toHaveBeenLastCalledWith("list_mcp_servers", { toolId: "codex" });

    await getMcpDiagnostics("codex");
    expect(invokeMock).toHaveBeenLastCalledWith("get_mcp_diagnostics", { toolId: "codex" });

    await listMcpConfigSources("codex");
    expect(invokeMock).toHaveBeenLastCalledWith("list_mcp_config_sources", { toolId: "codex" });

    await readMcpServerConfigFragment("codex", "mcp-config", "playwright");
    expect(invokeMock).toHaveBeenLastCalledWith("read_mcp_server_config_fragment", {
      toolId: "codex",
      sourceId: "mcp-config",
      serverName: "playwright",
    });

    await saveMcpServerConfigFragment("codex", "mcp-config", "playwright", "{}");
    expect(invokeMock).toHaveBeenLastCalledWith("save_mcp_server_config_fragment", {
      toolId: "codex",
      sourceId: "mcp-config",
      serverName: "playwright",
      content: "{}",
    });

    await writeApplicationLog({ level: "info" });
    expect(invokeMock).toHaveBeenLastCalledWith("write_application_log", { event: { level: "info" } });

    await listApplicationLogs();
    expect(invokeMock).toHaveBeenLastCalledWith("list_application_logs");

    await readApplicationLog("app-2026-05-27.log");
    expect(invokeMock).toHaveBeenLastCalledWith("read_application_log", { id: "app-2026-05-27.log" });

    await exportApplicationLogs(["app-2026-05-27.log"], "/tmp/app-log.txt");
    expect(invokeMock).toHaveBeenLastCalledWith("export_application_logs", {
      ids: ["app-2026-05-27.log"],
      destination: "/tmp/app-log.txt",
    });

    await writeModulePerformanceLog({ module: "skills" });
    expect(invokeMock).toHaveBeenLastCalledWith("write_module_performance_log", { event: { module: "skills" } });

    await listModulePerformanceLogs();
    expect(invokeMock).toHaveBeenLastCalledWith("list_module_performance_logs");

    await readModulePerformanceLog("module-performance-2026-05-27.log");
    expect(invokeMock).toHaveBeenLastCalledWith("read_module_performance_log", { id: "module-performance-2026-05-27.log" });

    await exportModulePerformanceLogs(["module-performance-2026-05-27.log"], "/tmp/module-log.txt");
    expect(invokeMock).toHaveBeenLastCalledWith("export_module_performance_logs", {
      ids: ["module-performance-2026-05-27.log"],
      destination: "/tmp/module-log.txt",
    });

    await getDashboard();
    expect(invokeMock).toHaveBeenLastCalledWith("get_dashboard");
  });
});
