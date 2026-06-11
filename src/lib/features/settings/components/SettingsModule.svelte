<script>
  // SettingsModule owns the Settings page behavior while the main page remains the section shell.
  import ToolIcon from "$lib/shared/components/ToolIcon.svelte";
  import ContentEditor from "$lib/shared/components/ContentEditor.svelte";
  import FileViewerShell from "$lib/shared/components/FileViewerShell.svelte";
  import ModulePerformanceBar from "$lib/shared/diagnostics/ModulePerformanceBar.svelte";
  import Tooltip from "$lib/shared/components/Tooltip.svelte";
  import { onDestroy, onMount, tick } from "svelte";
  import {
    AlertTriangle,
    Beaker,
    ChevronDown,
    ChevronRight,
    Check,
    Download,
    FileText,
    Folder,
    Languages,
    Pencil,
    Plus,
    RotateCcw,
    Settings2,
    Wrench,
    X,
  } from "lucide-svelte";
  import { detectedTools, managedToolIds, managedTools, theme } from "$lib/features/tools/index.js";
  import { activeSettingsTab, focusedSettingsToolId } from "$lib/features/settings/stores/settingsPage.js";
  import {
    appUpdateState,
    installAvailableAppUpdate,
    loadAppUpdateState,
    restartIntoAppUpdate,
    runAppUpdateCheck,
    skipAvailableAppUpdate,
  } from "$lib/features/appUpdates/index.js";
  import { applyLanguagePreference, languagePreference, t } from "$lib/shared/i18n/index.js";
  import { setTranslationProviderState } from "$lib/features/settings/stores/translationProvider.js";
  import { getVisualVerificationState, isVisualVerificationMode } from "$lib/dev/visualVerification/fixtures.js";
  import { logAppEvent } from "$lib/shared/logging/appLogger.js";
  import {
    getRuntimeInfo,
    getModulePerformanceDiagnosticsEnabled,
    getTranslationProviderConfig,
    setTranslationProviderConfig,
    setTranslationApiKey,
    testTranslationProvider,
    setModulePerformanceDiagnosticsEnabled,
    setLanguage,
    setTheme,
  } from "$lib/features/settings/api/settings.js";
  import {
    exportApplicationLogs,
    exportModulePerformanceLogs,
    listApplicationLogs,
    listModulePerformanceLogs,
    readApplicationLog,
    readModulePerformanceLog,
  } from "$lib/shared/logging/api.js";
  import {
    beginModulePerformanceRun,
    finishAndRecordModulePerformanceRun,
    markModulePerformance,
    MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE,
    MODULE_PERFORMANCE_ROLE_INTERACTIVE,
    MODULE_PERFORMANCE_ROLE_VISIBLE,
    setModulePerformanceCounters,
    setModulePerformanceDiagnosticsEnabledState,
    trackModulePerformanceRequest,
    updateModulePerformanceSummary,
  } from "$lib/shared/diagnostics/modulePerformance.js";
  import {
    addCustomTool,
    getCustomTools,
    getInjectionTargets,
    getToolName,
    getToolCapabilityOverrides,
    getCertifiedGlobalRuleTarget,
    getCertifiedSharedSkillDirectReadDefault,
    getManagedTools,
    getToolPaths,
    loadTools,
    removeCustomTool,
    setManagedTools,
    setToolCapabilityOverrides,
    setToolPath,
    listCapabilityProjections,
    summarizeRuleSourceState,
  } from "$lib/features/tools/index.js";
  import { invalidateSkillInventory } from "$lib/features/skills/index.js";

  const PROJECT_GITHUB_URL = "https://github.com/leon4z/Modus";

  /** @type {Record<string, any>} */
  let settingsToolPaths = $state({});
  /** @type {Record<string, string>} */
  let settingsInjectionTargets = $state({});
  /** @type {Record<string, any>} */
  let settingsCapabilityOverrides = $state({});
  let settingsOriginal = $state("");
  let settingsMsg = $state("");
  let settingsSaving = $state(false);
  /** @type {string | null} 工具配置同一时间仅一行处于编辑态 */
  let settingsEditingId = $state(/** @type {string | null} */ (null));
  /** @type {any[]} 与 customTools 同步，用于取消编辑时还原 rule_file 等 */
  let customToolsBaseline = $state(/** @type {any[]} */ []);
  /** @type {any[]} */
  let customTools = $state([]);
  let modulePerformanceDiagnosticsEnabled = $state(false);
  let modulePerformancePreferenceVersion = 0;
  let translationProviderConfig = $state(defaultTranslationProviderConfig());
  let translationProviderOriginal = $state(defaultTranslationProviderConfig());
  let translationApiKeyDraft = $state("");
  let translationApiKeyFocused = $state(false);
  let translationConfigSaving = $state(false);
  let translationConfigTesting = $state(false);
  let translationConfigMsg = $state(/** @type {{ key: string, vars?: Record<string, any> } | null} */ (null));
  let translationSettingsExpanded = $state(false);
  /** @type {null | "application" | "modulePerformance"} */
  let activeLogViewer = $state(null);
  /** @type {any[]} */
  let logFiles = $state([]);
  let selectedLogId = $state("");
  let logContent = $state("");
  let logLoading = $state(false);
  let logTruncated = $state(false);
  let logExporting = $state(false);
  let showAddTool = $state(false);
  let newTool = $state(createEmptyCustomToolDraft());
  let highlightedSettingsToolId = $state("");
  let expandedSettingsToolIds = $state(new Set());
  let resetDefaultsToolId = $state("");
  let runtimeInfo = $state(defaultRuntimeInfo());
  let runtimeInfoLoadFailed = $state(false);
  /** @type {ReturnType<typeof setTimeout> | null} */
  let highlightTimer = null;

  if (isVisualVerificationMode() && getVisualVerificationState() === "settings-tools") {
    activeSettingsTab.set("tools");
  }

  let settingsDirty = $derived(
    JSON.stringify({ p: settingsToolPaths, t: settingsInjectionTargets, o: settingsCapabilityOverrides }) !== settingsOriginal ||
    JSON.stringify(customTools) !== JSON.stringify(customToolsBaseline)
  );
  let customToolIds = $derived(new Set(customTools.map((/** @type {any} */ tool) => tool.id)));
  let builtInSettingsTools = $derived($detectedTools.filter((/** @type {any} */ tool) => !customToolIds.has(tool.id)));
  let settingsDiagnosticsModule = $derived($activeSettingsTab === "tools" ? "tools" : "settings");
  let runtimeShape = $derived(String(runtimeInfo?.shape || "release"));
  let showRuntimeMarker = $derived(runtimeShape === "development-sandbox" || runtimeShape === "pre-release");
  let showRuntimeDiagnostic = $derived(runtimeInfoLoadFailed && isFrontendDevBuild());
  let appUpdateBusy = $derived($appUpdateState.status === "checking" || $appUpdateState.status === "downloading" || $appUpdateState.status === "installing");
  let showInstallUpdateAction = $derived(Boolean($appUpdateState.availableUpdate?.version) && !appUpdateBusy && $appUpdateState.status !== "restart_needed");
  let showSkipUpdateAction = $derived(showInstallUpdateAction);
  let showRestartUpdateAction = $derived($appUpdateState.status === "restart_needed");
  let translationApiKeyDisplayValue = $derived(
    translationApiKeyDraft || (!translationApiKeyFocused && translationProviderConfig.apiKeyConfigured ? "••••••••••••" : "")
  );
  let translationSettingsDirty = $derived(
    translationApiKeyDraft.trim().length > 0 ||
    translationProviderConfig.enabled !== translationProviderOriginal.enabled ||
    translationProviderConfig.provider !== translationProviderOriginal.provider ||
    translationProviderConfig.baseUrl !== translationProviderOriginal.baseUrl ||
    translationProviderConfig.model !== translationProviderOriginal.model
  );
  let translationConfigMessageText = $derived(translationConfigMsg ? $t(translationConfigMsg.key, translationConfigMsg.vars ?? {}) : "");

  onMount(() => {
    void loadSettingsData({ reason: "entry" });
    void loadAppUpdateState().catch(() => {});
  });

  onDestroy(() => {
    if (highlightTimer) clearTimeout(highlightTimer);
  });

  /** @param {any} event */
  function logSettingsEvent(event) {
    void logAppEvent({ category: "settings", ...event }).catch(() => {});
  }

  $effect(() => {
    if ($activeSettingsTab === "project") {
      activeSettingsTab.set("general");
    }
  });

  $effect(() => {
    const toolId = $focusedSettingsToolId;
    const visibleToolCount = $detectedTools.length + customTools.length;
    if ($activeSettingsTab !== "tools" || !toolId || visibleToolCount === 0) return;
    void focusSettingsToolRow(toolId);
  });

  /** @param {string} toolId */
  async function focusSettingsToolRow(toolId) {
    await tick();
    if (typeof document === "undefined") return;
    const row = Array.from(document.querySelectorAll("[data-settings-tool-id]"))
      .find((element) => /** @type {HTMLElement} */ (element).dataset.settingsToolId === toolId);
    if (!row) return;
    row.scrollIntoView({ block: "center", behavior: "smooth" });
    highlightedSettingsToolId = toolId;
    if (highlightTimer) clearTimeout(highlightTimer);
    highlightTimer = setTimeout(() => {
      if (highlightedSettingsToolId === toolId) highlightedSettingsToolId = "";
      highlightTimer = null;
    }, 2200);
    focusedSettingsToolId.set(null);
  }

  /** @param {string} tab @returns {"general" | "tools"} */
  function normalizeSettingsDiagnosticsTab(tab) {
    return tab === "tools" ? "tools" : "general";
  }

  function parseSettingsOriginal() {
    try {
      return JSON.parse(settingsOriginal || "{}");
    } catch {
      return {};
    }
  }

  function defaultRuntimeInfo() {
    return {
      shape: "release",
      updateChannel: "stable",
      canCheckForUpdates: true,
      usesSandboxTools: false,
      usesRealTools: true,
    };
  }

  function defaultTranslationProviderConfig() {
    return {
      enabled: false,
      provider: "openai-compatible",
      baseUrl: "https://api.openai.com/v1",
      model: "",
      apiKeyConfigured: false,
    };
  }

  /** @param {any} config */
  function normalizeTranslationProviderConfig(config) {
    const provider = String(config?.provider || "openai-compatible").trim().toLowerCase();
    return {
      ...defaultTranslationProviderConfig(),
      ...(config || {}),
      provider: provider === "cc-router" || provider === "anthropic" || provider === "anthropic-compatible"
        ? "anthropic-messages"
        : provider || "openai-compatible",
      baseUrl: String(config?.baseUrl || config?.base_url || "https://api.openai.com/v1"),
      model: String(config?.model || ""),
      apiKeyConfigured: Boolean(config?.apiKeyConfigured || config?.api_key_configured),
    };
  }

  /** @param {any} info */
  function normalizeRuntimeInfo(info) {
    return {
      ...defaultRuntimeInfo(),
      ...(info || {}),
      shape: String(info?.shape || "release"),
    };
  }

  function isFrontendDevBuild() {
    return Boolean(import.meta.env?.DEV);
  }

  async function loadRuntimeInfo() {
    try {
      return { failed: false, info: await getRuntimeInfo() };
    } catch {
      return { failed: true, info: defaultRuntimeInfo() };
    }
  }

  function runtimeMarkerLabel() {
    if (showRuntimeDiagnostic) return $t("settings.runtime.unavailable.label");
    if (runtimeShape === "pre-release") return $t("settings.runtime.pre_release.label");
    return $t("settings.runtime.development_sandbox.label");
  }

  function runtimeMarkerDescription() {
    if (showRuntimeDiagnostic) return $t("settings.runtime.unavailable.desc");
    if (runtimeShape === "pre-release") return $t("settings.runtime.pre_release.desc");
    return $t("settings.runtime.development_sandbox.desc");
  }

  function appUpdateVersionLabel() {
    const version = String($appUpdateState.currentVersion || "0.1.0");
    return version.startsWith("v") ? version : `v${version}`;
  }

  /** @param {string} channel */
  function appUpdateChannelLabel(channel) {
    if (channel === "test") return $t("settings.update.channel.test");
    if (channel === "stable") return $t("settings.update.channel.stable");
    return $t("settings.update.channel.disabled");
  }

  function appUpdateStatusText() {
    const state = $appUpdateState;
    const update = state.availableUpdate;
    if (!state.canCheck || state.status === "disabled") return $t("settings.update.status_disabled");
    if (state.status === "checking") return $t("settings.update.status_checking");
    if (state.status === "current") return $t("settings.update.status_current");
    if (state.status === "skipped") return $t("settings.update.status_skipped");
    if (state.status === "downloading") return $t("settings.update.status_downloading");
    if (state.status === "installing") return $t("settings.update.status_installing");
    if (state.status === "restart_needed") return $t("settings.update.status_restart_needed");
    if (state.status === "failed") {
      return $t("settings.update.status_failed", { err: state.lastFailureSummary || $t("settings.update.failure_unknown") });
    }
    if (update?.version) {
      return $t("settings.update.status_available", {
        version: update.version,
        channel: appUpdateChannelLabel(update.channel || state.channel),
      });
    }
    return $t("settings.update.status_idle");
  }

  function appUpdateStatusTone() {
    const status = $appUpdateState.status;
    if (!$appUpdateState.canCheck || status === "disabled") return "muted";
    if (status === "failed") return "failed";
    if (status === "restart_needed") return "restart";
    if (status === "available" || $appUpdateState.availableUpdate?.version) return "available";
    return "muted";
  }

  async function manualAppUpdateCheck() {
    if (!$appUpdateState.canCheck || appUpdateBusy) return;
    await runAppUpdateCheck("manual");
  }

  async function openProjectGitHub() {
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(PROJECT_GITHUB_URL);
      logSettingsEvent({ action: "settings_project_github_open", result: "ok" });
    } catch (/** @type {any} */ e) {
      logSettingsEvent({ action: "settings_project_github_open", result: "failed", error: e });
      settingsMsg = $t("settings.github.open_failed", { err: e });
      setTimeout(() => {
        settingsMsg = "";
      }, 4000);
    }
  }

  async function installAppUpdateFromSettings() {
    if (!showInstallUpdateAction) return;
    await installAvailableAppUpdate();
  }

  async function restartForAppUpdate() {
    if (!showRestartUpdateAction) return;
    await restartIntoAppUpdate();
  }

  /** @param {Record<string, any>} paths */
  function skillPathSnapshot(paths) {
    return Object.fromEntries(
      Object.entries(paths || {})
        .map(([toolId, value]) => [toolId, String(value?.skills_dir || value?.skillsDir || "")])
        .sort(([a], [b]) => a.localeCompare(b))
    );
  }

  /** @param {Record<string, any>} overrides */
  function sharedSkillOverrideSnapshot(overrides) {
    const entries = Object.entries(overrides || {})
      .map(([toolId, value]) => {
        const directRead = value?.sharedSkillDirectRead ?? value?.shared_skill_direct_read;
        return [toolId, typeof directRead === "boolean" ? directRead : null];
      });
    entries.sort((left, right) => String(left[0]).localeCompare(String(right[0])));
    return Object.fromEntries(entries);
  }

  /** @param {any[]} tools */
  function customToolSkillSnapshot(tools) {
    return Object.fromEntries(
      (tools || [])
        .map((tool) => [String(tool?.id || ""), String(tool?.skills_dir || tool?.skillsDir || "")])
        .filter(([toolId]) => toolId)
        .sort(([a], [b]) => a.localeCompare(b))
    );
  }

  function skillAffectingSettingsChanged() {
    const original = parseSettingsOriginal();
    const before = {
      paths: skillPathSnapshot(original.p || {}),
      overrides: sharedSkillOverrideSnapshot(original.o || {}),
      customTools: customToolSkillSnapshot(customToolsBaseline),
    };
    const after = {
      paths: skillPathSnapshot(settingsToolPaths),
      overrides: sharedSkillOverrideSnapshot(settingsCapabilityOverrides),
      customTools: customToolSkillSnapshot(customTools),
    };
    return JSON.stringify(before) !== JSON.stringify(after);
  }

  /** @param {string} reason @param {"general" | "tools"} [tab] */
  function startSettingsRun(reason, tab = normalizeSettingsDiagnosticsTab($activeSettingsTab)) {
    const moduleId = tab === "tools" ? "tools" : "settings";
    const run = beginModulePerformanceRun({
      module: moduleId,
      view: tab,
      reason,
      counters: {
        detectedTools: $detectedTools.length,
        managedTools: $managedToolIds.length,
        customTools: customTools.length,
      },
    });
    markModulePerformance(run, "settings-visible", { role: MODULE_PERFORMANCE_ROLE_VISIBLE });
    updateModulePerformanceSummary(run);
    return run;
  }

  /** @param {{ reason?: string }} [options] */
  async function loadSettingsData(options = {}) {
    const run = startSettingsRun(options.reason || "entry");
    const preferenceVersion = modulePerformancePreferenceVersion;
    try {
      const [
        paths,
        injectionTargets,
        capabilityOverrides,
        nextCustomTools,
        nextModulePerformanceDiagnosticsEnabled,
        nextRuntimeInfo,
        nextTranslationProviderConfig,
      ] = await Promise.all([
        trackModulePerformanceRequest(run, "tool-paths", () => getToolPaths()),
        trackModulePerformanceRequest(run, "injection-targets", () => getInjectionTargets()),
        trackModulePerformanceRequest(run, "capability-overrides", () => getToolCapabilityOverrides()),
        trackModulePerformanceRequest(run, "custom-tools", () => getCustomTools()),
        trackModulePerformanceRequest(run, "diagnostics-preference", () => getModulePerformanceDiagnosticsEnabled().catch(() => false)),
        trackModulePerformanceRequest(run, "runtime-info", () => loadRuntimeInfo()),
        trackModulePerformanceRequest(run, "translation-provider", () => getTranslationProviderConfig().catch(() => defaultTranslationProviderConfig())),
        trackModulePerformanceRequest(run, "tool-list", () => loadTools().catch(() => [])),
      ]);
      settingsToolPaths = paths;
      settingsInjectionTargets = injectionTargets;
      settingsCapabilityOverrides = capabilityOverrides || {};
      settingsOriginal = JSON.stringify({ p: paths, t: injectionTargets, o: capabilityOverrides || {} });
      customTools = (nextCustomTools || []).map(normalizeCustomTool);
      customToolsBaseline = JSON.parse(JSON.stringify(customTools));
      runtimeInfo = normalizeRuntimeInfo(nextRuntimeInfo?.info);
      runtimeInfoLoadFailed = nextRuntimeInfo?.failed === true;
      translationProviderConfig = normalizeTranslationProviderConfig(nextTranslationProviderConfig);
      translationProviderOriginal = translationProviderConfig;
      setTranslationProviderState(translationProviderConfig);
      translationApiKeyDraft = "";
      if (preferenceVersion === modulePerformancePreferenceVersion) {
        modulePerformanceDiagnosticsEnabled = nextModulePerformanceDiagnosticsEnabled === true;
        setModulePerformanceDiagnosticsEnabledState(modulePerformanceDiagnosticsEnabled);
      }
      setModulePerformanceCounters(run, {
        detectedTools: $detectedTools.length,
        managedTools: $managedToolIds.length,
        customTools: customTools.length,
      });
      markModulePerformance(run, "settings-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
      markModulePerformance(run, "settings-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "success");
    } catch (_) {
      markModulePerformance(run, "settings-failed", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "failed");
    }
  }

  /** @param {"general" | "tools"} tab */
  function setSettingsTab(tab) {
    if ($activeSettingsTab === tab) return;
    const run = startSettingsRun("tab-switch", tab);
    activeSettingsTab.set(tab);
    markModulePerformance(run, "settings-tab-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
    markModulePerformance(run, "settings-tab-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
    finishAndRecordModulePerformanceRun(run, "success");
  }

  function closeLogViewer() {
    activeLogViewer = null;
    logFiles = [];
    selectedLogId = "";
    logContent = "";
    logTruncated = false;
  }

  /** @returns {"application" | "modulePerformance"} */
  function currentLogViewerKind() {
    return activeLogViewer === "modulePerformance" ? "modulePerformance" : "application";
  }

  /** @param {"application" | "modulePerformance"} kind */
  async function openLogViewer(kind) {
    activeLogViewer = kind;
    logFiles = [];
    selectedLogId = "";
    logContent = "";
    logTruncated = false;
    logLoading = true;
    try {
      const files = kind === "application" ? await listApplicationLogs() : await listModulePerformanceLogs();
      logFiles = files || [];
      const firstId = logFiles[0]?.id || "";
      selectedLogId = firstId;
      if (firstId) await loadSelectedLog(kind, firstId);
      logSettingsEvent({
        action: kind === "application" ? "settings_logs_open" : "settings_module_performance_log_open",
        result: "ok",
      });
    } catch (/** @type {any} */ e) {
      logSettingsEvent({
        action: kind === "application" ? "settings_logs_open" : "settings_module_performance_log_open",
        result: "failed",
        error: e,
      });
      settingsMsg = kind === "application"
        ? $t("settings.logs.open_failed", { err: e })
        : $t("settings.skill_perf.open_failed", { err: e });
      setTimeout(() => {
        settingsMsg = "";
      }, 4000);
    } finally {
      logLoading = false;
    }
  }

  /** @param {"application" | "modulePerformance"} kind @param {string} id */
  async function loadSelectedLog(kind, id) {
    selectedLogId = id;
    logLoading = true;
    try {
      const result = kind === "application" ? await readApplicationLog(id) : await readModulePerformanceLog(id);
      logContent = result?.content || "";
      logTruncated = result?.truncated === true;
    } catch (/** @type {any} */ e) {
      logContent = "";
      logTruncated = false;
      settingsMsg = $t("settings.log_viewer.read_failed", { err: e });
      setTimeout(() => {
        settingsMsg = "";
      }, 4000);
    } finally {
      logLoading = false;
    }
  }

  /** @param {"application" | "modulePerformance"} kind */
  async function exportSelectedLog(kind) {
    if (!selectedLogId || logExporting) return;
    logExporting = true;
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const defaultName = `${kind === "application" ? "application-log" : "module-performance-log"}-${selectedLogId}`;
      const destination = await save({ title: $t("settings.log_viewer.export"), defaultPath: defaultName });
      if (!destination) return;
      if (kind === "application") {
        await exportApplicationLogs([selectedLogId], destination);
      } else {
        await exportModulePerformanceLogs([selectedLogId], destination);
      }
      settingsMsg = $t("settings.log_viewer.export_ok");
      setTimeout(() => {
        settingsMsg = "";
      }, 2000);
    } catch (/** @type {any} */ e) {
      settingsMsg = $t("settings.log_viewer.export_failed", { err: e });
      setTimeout(() => {
        settingsMsg = "";
      }, 4000);
    } finally {
      logExporting = false;
    }
  }

  /** @param {boolean} enabled */
  async function toggleModulePerformanceDiagnostics(enabled) {
    const previous = modulePerformanceDiagnosticsEnabled;
    modulePerformancePreferenceVersion += 1;
    modulePerformanceDiagnosticsEnabled = enabled;
    setModulePerformanceDiagnosticsEnabledState(enabled);
    const run = enabled ? startSettingsRun("diagnostics-toggle") : null;
    try {
      await trackModulePerformanceRequest(run, "diagnostics-preference-save", () =>
        setModulePerformanceDiagnosticsEnabled(enabled)
      );
      if (run) {
        markModulePerformance(run, "settings-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
        markModulePerformance(run, "settings-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
        finishAndRecordModulePerformanceRun(run, "success");
      }
      logSettingsEvent({
        action: "settings_module_performance_diagnostics_update",
        result: "ok",
        targetRole: "module-performance-diagnostics",
        message: enabled ? "enabled" : "disabled",
      });
    } catch (/** @type {any} */ e) {
      if (run) {
        markModulePerformance(run, "settings-failed", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
        finishAndRecordModulePerformanceRun(run, "failed");
      }
      modulePerformancePreferenceVersion += 1;
      modulePerformanceDiagnosticsEnabled = previous;
      setModulePerformanceDiagnosticsEnabledState(previous);
      logSettingsEvent({
        action: "settings_module_performance_diagnostics_update",
        result: "failed",
        targetRole: "module-performance-diagnostics",
        error: e,
      });
      settingsMsg = $t("settings.skill_perf.update_failed", { err: e });
      setTimeout(() => {
        settingsMsg = "";
      }, 4000);
    }
  }

  /** @param {string} toolId */
  function revertToolSettingsFromOriginal(toolId) {
    const original = JSON.parse(settingsOriginal);
    if (original.p && original.p[toolId]) {
      settingsToolPaths[toolId] = {
        config_dir: original.p[toolId].config_dir || "",
        skills_dir: original.p[toolId].skills_dir || "",
      };
    } else {
      const nextPaths = { ...settingsToolPaths };
      delete nextPaths[toolId];
      settingsToolPaths = nextPaths;
    }
    const nextTargets = { ...settingsInjectionTargets };
    if (original.t && Object.prototype.hasOwnProperty.call(original.t, toolId)) {
      nextTargets[toolId] = original.t[toolId];
    } else {
      delete nextTargets[toolId];
    }
    settingsInjectionTargets = nextTargets;
    const nextOverrides = { ...settingsCapabilityOverrides };
    if (original.o && Object.prototype.hasOwnProperty.call(original.o, toolId)) {
      nextOverrides[toolId] = original.o[toolId];
    } else {
      delete nextOverrides[toolId];
    }
    settingsCapabilityOverrides = nextOverrides;
    settingsToolPaths = { ...settingsToolPaths };
    const baselineCustomTool = customToolsBaseline.find((tool) => tool.id === toolId);
    if (baselineCustomTool) {
      customTools = customTools.map((tool) => (tool.id === toolId ? { ...baselineCustomTool } : tool));
    }
  }

  /** @param {string} toolId */
  function openSettingsEdit(toolId) {
    if (settingsSaving || settingsEditingId === toolId) return;
    if (settingsEditingId) revertToolSettingsFromOriginal(settingsEditingId);
    expandSettingsTool(toolId);
    settingsEditingId = toolId;
  }

  /** @param {string} toolId */
  function expandSettingsTool(toolId) {
    expandedSettingsToolIds.add(toolId);
    expandedSettingsToolIds = new Set(expandedSettingsToolIds);
  }

  /** @param {string} toolId */
  function settingsToolExpanded(toolId) {
    return settingsEditingId === toolId || expandedSettingsToolIds.has(toolId);
  }

  /** @param {string} toolId */
  function toggleSettingsToolDetails(toolId) {
    if (settingsEditingId === toolId) return;
    if (expandedSettingsToolIds.has(toolId)) expandedSettingsToolIds.delete(toolId);
    else expandedSettingsToolIds.add(toolId);
    expandedSettingsToolIds = new Set(expandedSettingsToolIds);
  }

  /** @param {any} tool @param {"rule_directory" | "global_rule_file" | "skills_dir" | "mcp_config" | "tool_config"} field */
  function sourcePathIsUserConfigured(tool, field) {
    if (field === "rule_directory") return Boolean(customRuleSourcePath(tool));
    if (field === "global_rule_file") return Boolean(customGlobalRuleTarget(tool));
    if (field === "skills_dir") return primaryToolPathIsCustom(tool, "skills_dir");
    if (field === "mcp_config") return Boolean(customMcpConfigPath(tool));
    if (field === "tool_config") return Boolean(customToolConfigPath(tool));
    return false;
  }

  /** @param {any} tool @param {"rule_directory" | "global_rule_file" | "skills_dir" | "mcp_config" | "tool_config"} field */
  function sourcePathHealth(tool, field) {
    if (field === "rule_directory") return tool?.rule_directory_health;
    if (field === "global_rule_file") return tool?.global_rule_file_health;
    if (field === "skills_dir") return tool?.primary_skills_health;
    if (field === "mcp_config") return tool?.mcp_config_health;
    if (field === "tool_config") return tool?.tool_config_health;
    return "";
  }

  /** @param {"rule_directory" | "global_rule_file" | "skills_dir" | "mcp_config" | "tool_config"} field @param {"missing" | "unreadable"} state */
  function sourcePathHealthLabel(field, state) {
    const keyByField = {
      rule_directory: state === "missing" ? "settings.tools.rule_directory_missing" : "settings.tools.rule_directory_unreadable",
      global_rule_file: state === "missing" ? "settings.tools.global_rule_file_missing" : "settings.tools.global_rule_file_unreadable",
      skills_dir: state === "missing" ? "settings.tools.skills_attention_missing" : "settings.tools.skills_attention_unreadable",
      mcp_config: state === "missing" ? "settings.tools.mcp_config_missing" : "settings.tools.mcp_config_unreadable",
      tool_config: state === "missing" ? "settings.tools.tool_config_missing" : "settings.tools.tool_config_unreadable",
    };
    return $t(keyByField[field]);
  }

  /** @param {any} tool @param {"rule_directory" | "global_rule_file" | "skills_dir" | "mcp_config" | "tool_config"} field */
  function sourcePathAttentionLabel(tool, field) {
    if (!sourcePathIsUserConfigured(tool, field)) return "";
    const health = sourcePathHealth(tool, field);
    if (health === "missing" || health === "unreadable") {
      return sourcePathHealthLabel(field, health);
    }
    return "";
  }

  function cancelSettingsEdit() {
    if (settingsSaving || !settingsEditingId) return;
    revertToolSettingsFromOriginal(settingsEditingId);
    settingsEditingId = null;
  }

  /** @param {KeyboardEvent} event */
  function handleSettingsKeydown(event) {
    if (event.key !== "Escape" || settingsSaving || !settingsEditingId) return;
    if (event.target instanceof Element && event.target.closest('[role="dialog"]')) return;
    event.preventDefault();
    cancelSettingsEdit();
  }

  async function confirmSettingsEdit() {
    if (settingsSaving || !settingsEditingId || !settingsDirty) return;
    await saveSettings();
  }

  /** @param {string} nextTheme */
  function changeTheme(nextTheme) {
    theme.set(nextTheme);
    void setTheme(nextTheme)
      .then(() => {
        logSettingsEvent({ action: "settings_theme_update", result: "ok", targetRole: "theme" });
      })
      .catch((/** @type {any} */ error) => {
        logSettingsEvent({ action: "settings_theme_update", result: "failed", targetRole: "theme", error });
      });
  }


  /** @param {string} toolId */
  async function toggleManaged(toolId) {
    let nextIds = [...$managedToolIds];
    if (nextIds.includes(toolId)) {
      nextIds = nextIds.filter((id) => id !== toolId);
    } else {
      nextIds.push(toolId);
    }
    managedToolIds.set(nextIds);
    try {
      await setManagedTools(nextIds);
      logSettingsEvent({ action: "settings_tool_management_update", result: "ok", toolId });
    } catch (/** @type {any} */ error) {
      logSettingsEvent({ action: "settings_tool_management_update", result: "failed", toolId, error });
      throw error;
    }
  }

  /** @param {string} toolId @param {string} field */
  async function pickFolder(toolId, field) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: true, multiple: false, title: $t("settings.tools.pick_folder") });
    if (!selected) return;
    if (field === "rule_source") {
      const tool = [...$detectedTools, ...customTools].find((entry) => entry.id === toolId);
      if (tool) setRuleSourcePath(tool, selected);
      return;
    }
    setSettingsPath(toolId, field, selected);
  }

  /** @param {string} toolId @param {string} field */
  async function pickFile(toolId, field) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: false, multiple: false, title: $t("settings.tools.pick_file") });
    if (!selected) return;
    if (field === "global_rule_file") {
      const tool = [...$detectedTools, ...customTools].find((entry) => entry.id === toolId);
      if (tool) setGlobalRuleTarget(tool, selected);
      return;
    }
    if (field === "mcp_config") {
      const tool = [...$detectedTools, ...customTools].find((entry) => entry.id === toolId);
      if (tool) setMcpConfigPath(tool, selected);
      return;
    }
    if (field === "tool_config") {
      const tool = [...$detectedTools, ...customTools].find((entry) => entry.id === toolId);
      if (tool) setToolConfigPath(tool, selected);
      return;
    }
    if (field === "injection") {
      const tool = [...$detectedTools, ...customTools].find((entry) => entry.id === toolId);
      if (tool) {
        setGlobalRuleTarget(tool, selected);
      } else {
        setSettingsInjection(toolId, selected);
      }
      return;
    }
    if (field === "rule_file") {
      customTools = customTools.map((tool) => (tool.id === toolId ? { ...tool, rule_file: selected } : tool));
      return;
    }
    setSettingsPath(toolId, field, selected);
  }

  /** @param {string} field */
  async function pickNewToolFolder(field) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: true, multiple: false, title: $t("settings.tools.pick_folder") });
    if (!selected) return;
    newTool = { ...newTool, [field]: selected };
  }

  /** @param {string} field */
  async function pickNewToolFile(field) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: false, multiple: false, title: $t("settings.tools.pick_file") });
    if (!selected) return;
    newTool = { ...newTool, [field]: selected };
  }

  /** @param {string} toolId @param {string} field */
  async function pickCustomToolFolder(toolId, field) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: true, multiple: false, title: $t("settings.tools.pick_folder") });
    if (!selected) return;
    if (field === "skills_dir") {
      setSettingsPath(toolId, "skills_dir", selected);
      return;
    }
    setCustomToolField(toolId, field, selected);
  }

  /** @param {string} toolId @param {string} field */
  async function pickCustomToolFile(toolId, field) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: false, multiple: false, title: $t("settings.tools.pick_file") });
    if (!selected) return;
    setCustomToolField(toolId, field, selected);
  }

  /** @param {string} toolId @param {string} field */
  function getToolPath(toolId, field) {
    const paths = settingsToolPaths[toolId];
    return paths ? paths[field] || "" : "";
  }

  /** @param {string} toolId @param {string} field */
  function hasToolPathField(toolId, field) {
    return Boolean(settingsToolPaths[toolId] && Object.prototype.hasOwnProperty.call(settingsToolPaths[toolId], field));
  }

  /** @param {any} tool @param {"config_dir" | "skills_dir"} field */
  function defaultToolPath(tool, field) {
    if (field === "config_dir") return String(tool?.default_config_dir || tool?.config_dir || "");
    return String(tool?.default_skills_dir || tool?.skills_dir || "");
  }

  /** @param {any} tool @param {"config_dir" | "skills_dir"} field */
  function currentPrimaryToolPath(tool, field) {
    if (hasToolPathField(tool.id, field)) return String(settingsToolPaths[tool.id]?.[field] || "");
    return String(tool?.[field] || "");
  }

  /** @param {string} value */
  function comparableToolPath(value) {
    return String(value || "").trim().replace(/\/+$/, "");
  }

  /** @param {any} tool @param {"config_dir" | "skills_dir"} field */
  function primaryToolPathIsCustom(tool, field) {
    return comparableToolPath(currentPrimaryToolPath(tool, field)) !== comparableToolPath(defaultToolPath(tool, field));
  }

  /** @param {any} tool @param {"config_dir" | "skills_dir"} field */
  function primaryToolPathSourceLabel(tool, field) {
    return primaryToolPathIsCustom(tool, field) ? $t("settings.tools.source_user_override") : "";
  }

  /** @param {any} tool @param {"config_dir" | "skills_dir"} field */
  function resetPrimaryToolPath(tool, field) {
    setSettingsPath(tool.id, field, defaultToolPath(tool, field));
  }

  /** @param {any} tool @param {string} kind @param {string} access */
  function hasCapability(tool, kind, access) {
    const module = kind === "rule" ? "rules" : kind === "skill" ? "skills" : kind;
    return listCapabilityProjections(tool, module).some((/** @type {any} */ projection) => {
      if (access === "unsupported") return projection.exclusionReason === "unsupported";
      if (access === "unknown") return projection.exclusionReason === "unknown";
      return projection.evidence?.access === access;
    });
  }

  /** @param {string} toolId @param {string} target */
  function sharedInjectionLabel(toolId, target) {
    if (!target) return "";
    const peers = Object.entries(settingsInjectionTargets)
      .filter(([peerId, peerTarget]) => peerId !== toolId && peerTarget === target)
      .map(([peerId]) => getToolName(peerId));
    return peers.length > 0
      ? $t("settings.tools.shared_injection", { target, peers: peers.join("、") })
      : target;
  }

  /** @param {string} toolId */
  function capabilityOverride(toolId) {
    return settingsCapabilityOverrides[toolId] || settingsCapabilityOverrides[toolId.replaceAll("-", "_")] || {};
  }

  /** @param {string} toolId @param {{ customRuleSourceType?: string | null, customRuleSourcePath?: string | null, customGlobalRuleTarget?: string | null, customMcpConfigPath?: string | null, customToolConfigPath?: string | null, sharedSkillDirectRead?: boolean | null }} patch */
  function setCapabilityOverride(toolId, patch) {
    const current = capabilityOverride(toolId);
    const next = {
      customRuleSourceType: current.customRuleSourceType ?? current.custom_rule_source_type ?? null,
      customRuleSourcePath: current.customRuleSourcePath ?? current.custom_rule_source_path ?? null,
      customGlobalRuleTarget: current.customGlobalRuleTarget ?? current.custom_global_rule_target ?? null,
      customMcpConfigPath: current.customMcpConfigPath ?? current.custom_mcp_config_path ?? null,
      customToolConfigPath: current.customToolConfigPath ?? current.custom_tool_config_path ?? null,
      sharedSkillDirectRead: current.sharedSkillDirectRead ?? current.shared_skill_direct_read ?? null,
      ...patch,
    };
    if (
      !next.customRuleSourceType
      && !next.customRuleSourcePath
      && !next.customGlobalRuleTarget
      && !next.customMcpConfigPath
      && !next.customToolConfigPath
      && typeof next.sharedSkillDirectRead !== "boolean"
    ) {
      const withoutTool = { ...settingsCapabilityOverrides };
      delete withoutTool[toolId];
      delete withoutTool[toolId.replaceAll("-", "_")];
      settingsCapabilityOverrides = withoutTool;
      return;
    }
    settingsCapabilityOverrides = {
      ...settingsCapabilityOverrides,
      [toolId]: next,
    };
  }

  /** @param {any} tool */
  function rawCustomGlobalRuleTarget(tool) {
    const override = capabilityOverride(tool.id);
    return String(override.customGlobalRuleTarget ?? override.custom_global_rule_target ?? "").trim();
  }

  /** @param {any} tool */
  function rawCustomRuleSourcePath(tool) {
    const override = capabilityOverride(tool.id);
    return String(override.customRuleSourcePath ?? override.custom_rule_source_path ?? "").trim();
  }

  /** @param {any} tool */
  function rawCustomMcpConfigPath(tool) {
    const override = capabilityOverride(tool.id);
    return String(override.customMcpConfigPath ?? override.custom_mcp_config_path ?? "").trim();
  }

  /** @param {any} tool */
  function rawCustomToolConfigPath(tool) {
    const override = capabilityOverride(tool.id);
    return String(override.customToolConfigPath ?? override.custom_tool_config_path ?? "").trim();
  }

  /** @param {string} value @param {string} defaultValue */
  function customPathValue(value, defaultValue) {
    const trimmed = String(value || "").trim();
    if (!trimmed) return "";
    return comparableToolPath(trimmed) === comparableToolPath(defaultValue) ? "" : trimmed;
  }

  /** @param {any} tool */
  function customGlobalRuleTarget(tool) {
    return customPathValue(rawCustomGlobalRuleTarget(tool), globalRuleDefaultTarget(tool));
  }

  /** @param {any} tool */
  function customRuleSourcePath(tool) {
    return customPathValue(rawCustomRuleSourcePath(tool), defaultRuleSourcePath(tool));
  }

  /** @param {any} tool */
  function customMcpConfigPath(tool) {
    return customPathValue(rawCustomMcpConfigPath(tool), defaultMcpConfigPath(tool));
  }

  /** @param {any} tool */
  function customToolConfigPath(tool) {
    return customPathValue(rawCustomToolConfigPath(tool), defaultOrdinaryConfigPath(tool));
  }

  /** @param {any} tool */
  function ruleSourceState(tool) {
    return summarizeRuleSourceState(tool, settingsCapabilityOverrides);
  }

  /** @param {any} tool */
  function globalRuleDefaultTarget(tool) {
    const certified = getCertifiedGlobalRuleTarget(tool);
    if (certified) return certified;
    return "";
  }

  /** @param {any} tool */
  function effectiveGlobalRuleTarget(tool) {
    const state = ruleSourceState(tool);
    return state.globalRuleFile || globalRuleDefaultTarget(tool);
  }

  /** @param {any} tool */
  function canEditGlobalRuleTarget(tool) {
    return Boolean(tool);
  }

  /** @param {any} tool */
  function canEditRuleSource(tool) {
    return Boolean(tool);
  }

  /** @param {any} tool */
  function globalRuleTargetSource(tool) {
    if (customGlobalRuleTarget(tool)) return $t("settings.tools.source_user_override");
    return "";
  }

  /** @param {any} tool */
  function globalRuleTargetPrompt(tool) {
    const state = ruleSourceState(tool);
    if (effectiveGlobalRuleTarget(tool)) return sharedInjectionLabel(tool.id, effectiveGlobalRuleTarget(tool));
    if (state.sourceType === "directory" && state.sourcePath) return $t("settings.tools.value_missing");
    if (globalRuleFileIsUnconfigured(tool)) return $t("settings.tools.value_missing");
    return $t("settings.tools.value_unsupported");
  }

  /** @param {any} tool */
  function ruleSourceDisplayValue(tool) {
    const state = ruleSourceState(tool);
    if (state.sourcePath) return state.sourcePath;
    if (ruleDirectoryIsUnconfigured(tool)) return $t("settings.tools.value_missing");
    return $t("settings.tools.value_unsupported");
  }

  /** @param {any} tool */
  function defaultRuleSourceDisplayValue(tool) {
    const state = summarizeRuleSourceState(tool, {});
    if (state.sourcePath) return state.sourcePath;
    if (ruleDirectoryIsUnconfigured(tool)) return $t("settings.tools.value_missing");
    return $t("settings.tools.value_unsupported");
  }

  /** @param {any} tool */
  function defaultRuleSourcePath(tool) {
    return summarizeRuleSourceState(tool, {}).sourcePath || "";
  }

  /** @param {any} tool */
  function ruleSourceInputValue(tool) {
    return ruleSourceState(tool).sourcePath;
  }

  /** @param {any} tool */
  function ruleDirectorySourceLabel(tool) {
    const state = ruleSourceState(tool);
    if (!state.sourcePath) return "";
    if (customRuleSourcePath(tool)) return $t("settings.tools.source_user_override");
    return "";
  }

  /** @param {any} projection */
  function projectionSourcePath(projection) {
    return String(projection?.evidence?.source_path || "").split("#")[0].trim();
  }

  /** @param {any} projection */
  function projectionLooksLikeDirectory(projection) {
    if (projection?.evidence?.format === "directory") return true;
    const sourcePath = projectionSourcePath(projection).replace(/\/+$/, "");
    const name = sourcePath.split("/").filter(Boolean).pop() || "";
    return Boolean(sourcePath && !name.includes("."));
  }

  /** @param {any} tool */
  function ruleDirectoryIsUnconfigured(tool) {
    return listCapabilityProjections(tool, "rules").some((/** @type {any} */ projection) => (
      projection.sourceRole === "native_file_source"
      && projectionLooksLikeDirectory(projection)
      && (projection.exclusionReason === "missing_path" || !projectionSourcePath(projection))
    ));
  }

  /** @param {any} tool */
  function globalRuleFileIsUnconfigured(tool) {
    return listCapabilityProjections(tool, "rules").some((/** @type {any} */ projection) => (
      projection.sourceRole === "global_target"
      && (projection.exclusionReason === "missing_path" || !projectionSourcePath(projection))
    ));
  }

  /** @param {any} tool @param {string} value */
  function setRuleSourcePath(tool, value) {
    const trimmed = String(value || "").trim();
    setCapabilityOverride(tool.id, {
      customRuleSourceType: trimmed ? "directory" : null,
      customRuleSourcePath: trimmed || null,
    });
  }

  /** @param {any} tool @param {string} value */
  function setGlobalRuleTarget(tool, value) {
    const trimmed = String(value || "").trim();
    const certified = getCertifiedGlobalRuleTarget(tool);
    const nextTarget = { ...settingsInjectionTargets };
    if (trimmed && trimmed !== certified) {
      nextTarget[tool.id] = trimmed;
    } else {
      delete nextTarget[tool.id];
    }
    settingsInjectionTargets = nextTarget;
    setCapabilityOverride(tool.id, {
      customGlobalRuleTarget: trimmed && trimmed !== certified ? trimmed : null,
    });
  }

  /** @param {any} tool */
  function resetGlobalRuleTarget(tool) {
    setGlobalRuleTarget(tool, "");
  }

  /** @param {any} tool */
  function resetRuleSource(tool) {
    setCapabilityOverride(tool.id, {
      customRuleSourceType: null,
      customRuleSourcePath: null,
    });
  }

  /** @param {any} tool @param {string} value */
  function setMcpConfigPath(tool, value) {
    const trimmed = String(value || "").trim();
    const certified = capabilitySourcePathValue(tool, "mcp", ["global_config"]);
    setCapabilityOverride(tool.id, {
      customMcpConfigPath: trimmed && trimmed !== certified ? trimmed : null,
    });
  }

  /** @param {any} tool @param {string} value */
  function setToolConfigPath(tool, value) {
    const trimmed = String(value || "").trim();
    const certified = capabilitySourcePathValue(tool, "ordinary_config", ["config_file", "ordinary_config_file"]);
    setCapabilityOverride(tool.id, {
      customToolConfigPath: trimmed && trimmed !== certified ? trimmed : null,
    });
  }

  /** @param {any} tool */
  function sharedSkillOverrideValue(tool) {
    const override = capabilityOverride(tool.id);
    const value = override.sharedSkillDirectRead ?? override.shared_skill_direct_read;
    return typeof value === "boolean" ? value : null;
  }

  /** @param {any} tool */
  function sharedSkillDefaultValue(tool) {
    return getCertifiedSharedSkillDirectReadDefault(tool);
  }

  /** @param {any} tool */
  function sharedSkillEffectiveValue(tool) {
    const override = sharedSkillOverrideValue(tool);
    return typeof override === "boolean" ? override : sharedSkillDefaultValue(tool);
  }

  /** @param {boolean} value */
  function directReadLabel(value) {
    return value ? $t("settings.tools.direct_read_yes") : $t("settings.tools.direct_read_no");
  }

  /** @param {any} tool @param {boolean} value */
  function setSharedSkillDirectReadMode(tool, value) {
    const defaultValue = sharedSkillDefaultValue(tool);
    setCapabilityOverride(tool.id, {
      sharedSkillDirectRead: value === defaultValue ? null : value,
    });
  }

  /** @param {any} tool */
  function resetSharedSkillSupport(tool) {
    setCapabilityOverride(tool.id, {
      sharedSkillDirectRead: null,
    });
  }

  function createEmptyCustomToolDraft() {
    return {
      name: "",
      config_dir: "",
      rule_directory: "",
      global_rule_file: "",
      skills_dir: "",
      shared_skill_direct_read: false,
      mcp_config: "",
      tool_config: "",
      rule_file: "",
    };
  }

  /** @param {any} tool */
  function normalizeCustomTool(tool) {
    const globalRuleFile = String(tool?.global_rule_file ?? tool?.globalRuleFile ?? tool?.rule_file ?? "");
    const sharedSkillDirectRead = tool?.shared_skill_direct_read ?? tool?.sharedSkillDirectRead;
    return {
      id: String(tool?.id || ""),
      name: String(tool?.name || ""),
      icon: String(tool?.icon || "wrench"),
      config_dir: String(tool?.config_dir ?? tool?.configDir ?? ""),
      rule_directory: String(tool?.rule_directory ?? tool?.ruleDirectory ?? ""),
      global_rule_file: globalRuleFile,
      skills_dir: String(tool?.skills_dir ?? tool?.skillsDir ?? ""),
      shared_skill_direct_read: typeof sharedSkillDirectRead === "boolean" ? sharedSkillDirectRead : false,
      mcp_config: String(tool?.mcp_config ?? tool?.mcpConfig ?? ""),
      tool_config: String(tool?.tool_config ?? tool?.toolConfig ?? ""),
      rule_file: String(tool?.rule_file ?? globalRuleFile),
    };
  }

  /** @param {any} value */
  function customToolFeatureValueConfigured(value) {
    const trimmed = String(value ?? "").trim();
    return Boolean(trimmed && !trimmed.includes("*"));
  }

  /** @param {any} tool */
  function customToolHasFeatureSource(tool) {
    return [
      tool?.rule_directory,
      tool?.global_rule_file,
      tool?.rule_file,
      tool?.skills_dir,
      tool?.mcp_config,
      tool?.tool_config,
    ].some(customToolFeatureValueConfigured) || customToolSharedSkillDirectRead(tool);
  }

  /** @param {string[]} a @param {string[]} b */
  function sameStringList(a, b) {
    return a.length === b.length && a.every((item, index) => item === b[index]);
  }

  /** @param {any[]} nextTools @param {any[]} previousTools */
  async function syncManagedCustomToolsForSources(nextTools, previousTools = []) {
    const previousById = new Map(previousTools.map((tool) => [tool.id, tool]));
    const nextManagedIds = [...$managedToolIds];
    for (const tool of nextTools) {
      const hasFeatureSource = customToolHasFeatureSource(tool);
      if (!hasFeatureSource) {
        const index = nextManagedIds.indexOf(tool.id);
        if (index >= 0) nextManagedIds.splice(index, 1);
        continue;
      }
      const previous = previousById.get(tool.id);
      const previouslyHadFeatureSource = previous ? customToolHasFeatureSource(previous) : false;
      if (!previouslyHadFeatureSource && !nextManagedIds.includes(tool.id)) {
        nextManagedIds.push(tool.id);
      }
    }
    if (sameStringList(nextManagedIds, $managedToolIds)) return;
    managedToolIds.set(nextManagedIds);
    await setManagedTools(nextManagedIds);
  }

  /** @param {string} toolId @param {string} field @param {any} value */
  function setCustomToolField(toolId, field, value) {
    customTools = customTools.map((tool) => {
      if (tool.id !== toolId) return tool;
      const next = { ...tool, [field]: value };
      if (field === "global_rule_file") next.rule_file = value;
      return normalizeCustomTool(next);
    });
  }

  /** @param {any} tool */
  function customToolSharedSkillDirectRead(tool) {
    return Boolean(tool?.shared_skill_direct_read ?? tool?.sharedSkillDirectRead);
  }

  /** @param {any} tool */
  function customToolSkillsValue(tool) {
    return getToolPath(tool.id, "skills_dir") || tool.skills_dir || "";
  }

  /** @param {any} tool */
  function skillsDisplayValue(tool) {
    const value = currentPrimaryToolPath(tool, "skills_dir");
    if (value) return value;
    if (hasCapability(tool, "skill", "unsupported")) {
      return $t("settings.tools.skills_unsupported_file_sync");
    }
    return $t("settings.tools.value_missing");
  }

  /** @param {any} tool */
  function injectionDisplayValue(tool) {
    return globalRuleTargetPrompt(tool);
  }

  /** @param {any} capability */
  function capabilityIsUserConfigured(capability) {
    return (capability?.source_confidence ?? capability?.sourceConfidence) === "user_configured";
  }

  /** @param {any} tool @param {"mcp" | "ordinary_config"} module @param {string[]} roles @param {{ includeUserConfigured?: boolean }} [options] */
  function capabilitySourceDisplayValue(tool, module, roles, options = {}) {
    const projections = capabilitySourcePathValue(tool, module, roles, options);
    if (projections) return projections;
    const allProjections = listCapabilityProjections(tool, module);
    const unsupported = allProjections.find((/** @type {any} */ projection) => projection.exclusionReason);
    if (unsupported?.exclusionReason === "unsupported") return $t("settings.tools.value_unsupported");
    const hasRelevantProjection = allProjections.some((/** @type {any} */ projection) => (
      roles.includes(projection.sourceRole) || projection.sourceRole === "non_actionable_evidence"
    ));
    if (!hasRelevantProjection) return $t("settings.tools.value_unsupported");
    return $t("settings.tools.value_missing");
  }

  /** @param {any} tool @param {"mcp" | "ordinary_config"} module @param {string[]} roles @param {{ includeUserConfigured?: boolean }} [options] */
  function capabilitySourcePathValue(tool, module, roles, options = {}) {
    const includeUserConfigured = options.includeUserConfigured !== false;
    const seen = new Set();
    const projections = listCapabilityProjections(tool, module)
      .filter((/** @type {any} */ projection) => roles.includes(projection.sourceRole))
      .filter((/** @type {any} */ projection) => !projection.exclusionReason)
      .filter((/** @type {any} */ projection) => includeUserConfigured || !capabilityIsUserConfigured(projection.evidence))
      .map((/** @type {any} */ projection) => String(projection.evidence?.source_path || "").split("#")[0].trim())
      .filter(Boolean)
      .filter((/** @type {string} */ sourcePath) => {
        const comparable = comparableToolPath(sourcePath);
        if (seen.has(comparable)) return false;
        seen.add(comparable);
        return true;
      });
    if (projections.length > 0) return projections.join(", ");
    return "";
  }

  /** @param {any} tool */
  function mcpConfigDisplayValue(tool) {
    return customMcpConfigPath(tool) || capabilitySourceDisplayValue(tool, "mcp", ["global_config"]);
  }

  /** @param {any} tool */
  function defaultMcpConfigDisplayValue(tool) {
    return capabilitySourceDisplayValue(tool, "mcp", ["global_config"], { includeUserConfigured: false });
  }

  /** @param {any} tool */
  function defaultMcpConfigPath(tool) {
    return capabilitySourcePathValue(tool, "mcp", ["global_config"], { includeUserConfigured: false });
  }

  /** @param {any} tool */
  function mcpConfigInputValue(tool) {
    return customMcpConfigPath(tool) || capabilitySourcePathValue(tool, "mcp", ["global_config"]);
  }

  /** @param {any} tool */
  function ordinaryConfigDisplayValue(tool) {
    return customToolConfigPath(tool) || capabilitySourceDisplayValue(tool, "ordinary_config", ["config_file", "ordinary_config_file"]);
  }

  /** @param {any} tool */
  function defaultOrdinaryConfigDisplayValue(tool) {
    return capabilitySourceDisplayValue(tool, "ordinary_config", ["config_file", "ordinary_config_file"], { includeUserConfigured: false });
  }

  /** @param {any} tool */
  function defaultOrdinaryConfigPath(tool) {
    return capabilitySourcePathValue(tool, "ordinary_config", ["config_file", "ordinary_config_file"], { includeUserConfigured: false });
  }

  /** @param {any} tool */
  function ordinaryConfigInputValue(tool) {
    return customToolConfigPath(tool) || capabilitySourcePathValue(tool, "ordinary_config", ["config_file", "ordinary_config_file"]);
  }

  function validateSettingsBeforeSave() {
    return "";
  }

  /** @param {string} toolId @param {string} field @param {string} value */
  function setSettingsPath(toolId, field, value) {
    if (!settingsToolPaths[toolId]) settingsToolPaths[toolId] = { config_dir: "", skills_dir: "" };
    settingsToolPaths[toolId][field] = value;
    settingsToolPaths = { ...settingsToolPaths };
  }

  /** @param {string} toolId @param {string} value */
  function setSettingsInjection(toolId, value) {
    settingsInjectionTargets[toolId] = value;
    settingsInjectionTargets = { ...settingsInjectionTargets };
  }

  /** @param {any} tool */
  async function saveCapabilityOverride(tool) {
    const override = capabilityOverride(tool.id);
    const ruleSourcePath = customRuleSourcePath(tool);
    const sharedSkillOverride = sharedSkillOverrideValue(tool);
    const sharedSkillDefault = sharedSkillDefaultValue(tool);
    await setToolCapabilityOverrides(tool.id, {
      customRuleSourceType: ruleSourcePath ? (override.customRuleSourceType ?? override.custom_rule_source_type ?? "directory") : null,
      customRuleSourcePath: ruleSourcePath || null,
      customGlobalRuleTarget: customGlobalRuleTarget(tool) || null,
      customMcpConfigPath: customMcpConfigPath(tool) || null,
      customToolConfigPath: customToolConfigPath(tool) || null,
      sharedSkillDirectRead: sharedSkillOverride !== null && sharedSkillOverride !== sharedSkillDefault ? sharedSkillOverride : null,
    });
  }

  async function saveSettings() {
    const validationMessage = validateSettingsBeforeSave();
    if (validationMessage) {
      settingsMsg = validationMessage;
      return;
    }
    settingsSaving = true;
    settingsMsg = "";
    const editedToolId = settingsEditingId;
    const shouldInvalidateSkillInventory = skillAffectingSettingsChanged();
    try {
      /** @type {any[]} */
      const persistedCustomTools = [];

      for (const tool of $detectedTools) {
        if (customToolIds.has(tool.id)) continue;
        const paths = settingsToolPaths[tool.id];
        if (paths) {
          await setToolPath(tool.id, paths.config_dir || "", paths.skills_dir || "");
        }
        await saveCapabilityOverride(tool);
      }

      for (const customTool of customTools) {
        const paths = settingsToolPaths[customTool.id];
        if (paths) {
          await setToolPath(customTool.id, paths.config_dir || "", paths.skills_dir || "");
        }
        await saveCapabilityOverride(customTool);
        const nextCustomTool = normalizeCustomTool({
          ...customTool,
          config_dir: paths?.config_dir || customTool.config_dir || "",
          skills_dir: paths?.skills_dir || customTool.skills_dir || "",
          rule_file: customTool.global_rule_file || customTool.rule_file || "",
        });
        await addCustomTool(nextCustomTool);
        persistedCustomTools.push(nextCustomTool);
      }
      await syncManagedCustomToolsForSources(persistedCustomTools, customToolsBaseline);

      if (editedToolId) {
        logSettingsEvent({ action: "settings_tool_configuration_update", result: "ok", toolId: editedToolId });
      }
      await loadTools();
      if (shouldInvalidateSkillInventory) {
        invalidateSkillInventory();
      }
      settingsOriginal = JSON.stringify({ p: settingsToolPaths, t: settingsInjectionTargets, o: settingsCapabilityOverrides });
      settingsEditingId = null;
      customTools = persistedCustomTools;
      customToolsBaseline = JSON.parse(JSON.stringify(persistedCustomTools));
      settingsMsg = $t("global.msg.saved");
      setTimeout(() => {
        settingsMsg = "";
      }, 2000);
    } catch (/** @type {any} */ e) {
      if (editedToolId) {
        logSettingsEvent({ action: "settings_tool_configuration_update", result: "failed", toolId: editedToolId, error: e });
      }
      settingsMsg = $t("global.msg.save_failed", { err: e });
    } finally {
      settingsSaving = false;
    }
  }

  async function doAddCustomTool() {
    if (!newTool.name.trim()) return;
    const id = newTool.name.trim().toLowerCase().replace(/\s+/g, "_");
    const tool = normalizeCustomTool({
      id,
      name: newTool.name.trim(),
      icon: "wrench",
      config_dir: "",
      rule_directory: newTool.rule_directory,
      global_rule_file: newTool.global_rule_file,
      skills_dir: newTool.skills_dir,
      shared_skill_direct_read: newTool.shared_skill_direct_read,
      mcp_config: newTool.mcp_config,
      tool_config: newTool.tool_config,
      rule_file: newTool.global_rule_file,
    });
    try {
      await addCustomTool(tool);
      await syncManagedCustomToolsForSources([...customTools, tool], customTools);
      logSettingsEvent({ action: "settings_custom_tool_create", result: "ok", toolId: tool.id });
      customTools = [...customTools, tool];
      customToolsBaseline = JSON.parse(JSON.stringify(customTools));
      await loadTools();
      newTool = createEmptyCustomToolDraft();
      showAddTool = false;
      settingsMsg = $t("global.msg.add_ok", { name: tool.name });
      setTimeout(() => {
        settingsMsg = "";
      }, 2000);
    } catch (/** @type {any} */ e) {
      logSettingsEvent({ action: "settings_custom_tool_create", result: "failed", toolId: tool.id, error: e });
      settingsMsg = $t("global.msg.add_failed", { err: e });
    }
  }

  /** @param {string} toolId */
  async function doRemoveCustomTool(toolId) {
    try {
      await removeCustomTool(toolId);
      const nextManagedIds = $managedToolIds.filter((id) => id !== toolId);
      managedToolIds.set(nextManagedIds);
      logSettingsEvent({ action: "settings_custom_tool_delete", result: "ok", toolId });
      customTools = customTools.filter((tool) => tool.id !== toolId);
      customToolsBaseline = JSON.parse(JSON.stringify(customTools));
      await loadTools();
      if (settingsEditingId === toolId) settingsEditingId = null;
      expandedSettingsToolIds.delete(toolId);
      expandedSettingsToolIds = new Set(expandedSettingsToolIds);
      settingsMsg = $t("global.msg.deleted");
      setTimeout(() => {
        settingsMsg = "";
      }, 2000);
    } catch (/** @type {any} */ e) {
      logSettingsEvent({ action: "settings_custom_tool_delete", result: "failed", toolId, error: e });
      settingsMsg = $t("global.msg.delete_failed", { err: e });
    }
  }

  /** @param {string} lang */
  async function switchLanguage(lang) {
    try {
      await setLanguage(lang);
      applyLanguagePreference(lang);
      logSettingsEvent({ action: "settings_language_update", result: "ok", targetRole: "language" });
    } catch (/** @type {any} */ error) {
      logSettingsEvent({ action: "settings_language_update", result: "failed", targetRole: "language", error });
      throw error;
    }
  }

  /** @param {keyof ReturnType<typeof defaultTranslationProviderConfig>} field @param {any} value */
  function setTranslationProviderField(field, value) {
    clearTranslationConfigMsg();
    translationProviderConfig = {
      ...translationProviderConfig,
      [field]: value,
    };
  }

  function clearTranslationConfigMsg() {
    translationConfigMsg = null;
  }

  /** @param {string} key @param {Record<string, any>} [vars] */
  function setTranslationConfigMsg(key, vars = {}) {
    translationConfigMsg = { key, vars };
  }

  function cancelTranslationSettingsEdit() {
    if (translationConfigSaving || translationConfigTesting) return;
    translationProviderConfig = translationProviderOriginal;
    translationApiKeyDraft = "";
    translationApiKeyFocused = false;
    clearTranslationConfigMsg();
    translationSettingsExpanded = false;
  }

  /** @param {ReturnType<typeof defaultTranslationProviderConfig>} config */
  function translationProviderSavePayload(config) {
    return {
      enabled: config.enabled,
      provider: config.provider,
      baseUrl: config.baseUrl,
      model: config.model,
    };
  }

  /** @param {boolean} enabled */
  async function setTranslationProviderEnabled(enabled) {
    if (translationConfigSaving || translationConfigTesting) return;
    const previousConfig = translationProviderConfig;
    translationProviderConfig = {
      ...translationProviderConfig,
      enabled,
    };
    translationConfigSaving = true;
    clearTranslationConfigMsg();
    try {
      const nextConfig = await setTranslationProviderConfig(translationProviderSavePayload(translationProviderConfig));
      translationProviderConfig = normalizeTranslationProviderConfig(nextConfig);
      translationProviderOriginal = translationProviderConfig;
      setTranslationProviderState(translationProviderConfig);
    } catch (/** @type {any} */ e) {
      translationProviderConfig = previousConfig;
      setTranslationConfigMsg("settings.translation.save_failed", { err: e });
    } finally {
      translationConfigSaving = false;
    }
  }

  async function saveTranslationProviderSettings() {
    if (translationConfigSaving) return false;
    if (!translationSettingsDirty) return true;
    translationConfigSaving = true;
    clearTranslationConfigMsg();
    try {
      let nextConfig = await setTranslationProviderConfig(translationProviderSavePayload(translationProviderConfig));
      if (translationApiKeyDraft.trim()) {
        nextConfig = await setTranslationApiKey(translationApiKeyDraft);
      }
      translationProviderConfig = normalizeTranslationProviderConfig(nextConfig);
      translationProviderOriginal = translationProviderConfig;
      setTranslationProviderState(translationProviderConfig);
      translationApiKeyDraft = "";
      translationApiKeyFocused = false;
      setTranslationConfigMsg("settings.translation.saved");
      const savedMessage = translationConfigMsg;
      setTimeout(() => {
        if (translationConfigMsg === savedMessage) clearTranslationConfigMsg();
      }, 2000);
      return true;
    } catch (/** @type {any} */ e) {
      setTranslationConfigMsg("settings.translation.save_failed", { err: e });
      return false;
    } finally {
      translationConfigSaving = false;
    }
  }

  async function testTranslationProviderSettings() {
    if (translationConfigSaving || translationConfigTesting) return;
    translationConfigTesting = true;
    clearTranslationConfigMsg();
    try {
      const saved = await saveTranslationProviderSettings();
      if (!saved) return;
      await testTranslationProvider();
      setTranslationConfigMsg("settings.translation.test_ok");
    } catch (/** @type {any} */ e) {
      setTranslationConfigMsg("settings.translation.test_failed", { err: e });
    } finally {
      translationConfigTesting = false;
    }
  }

  /** @param {any} tool */
  function toolCustomResetItems(tool) {
    const items = [];
    if (customRuleSourcePath(tool)) {
      items.push({
        label: $t("settings.tools.label_rule_directory"),
        current: customRuleSourcePath(tool),
        next: defaultRuleSourceDisplayValue(tool),
      });
    }
    if (customGlobalRuleTarget(tool)) {
      items.push({
        label: $t("settings.tools.label_global_rule_file"),
        current: customGlobalRuleTarget(tool),
        next: globalRuleDefaultTarget(tool) || $t("settings.tools.value_missing"),
      });
    }
    if (primaryToolPathIsCustom(tool, "skills_dir")) {
      items.push({
        label: $t("settings.tools.label_skills"),
        current: currentPrimaryToolPath(tool, "skills_dir"),
        next: defaultToolPath(tool, "skills_dir") || $t("settings.tools.value_missing"),
      });
    }
    if (sharedSkillOverrideValue(tool) !== null) {
      items.push({
        label: $t("settings.tools.label_shared_skill_direct_read"),
        current: directReadLabel(sharedSkillEffectiveValue(tool)),
        next: directReadLabel(sharedSkillDefaultValue(tool)),
      });
    }
    if (customMcpConfigPath(tool)) {
      items.push({
        label: $t("settings.tools.label_mcp_config"),
        current: customMcpConfigPath(tool),
        next: defaultMcpConfigDisplayValue(tool),
      });
    }
    if (customToolConfigPath(tool)) {
      items.push({
        label: $t("settings.tools.label_tool_config_files"),
        current: customToolConfigPath(tool),
        next: defaultOrdinaryConfigDisplayValue(tool),
      });
    }
    return items;
  }

  /** @param {any} tool */
  function toolHasCustomSettings(tool) {
    return toolCustomResetItems(tool).length > 0;
  }

  /** @param {any} tool */
  function openResetDefaultsDialog(tool) {
    if (settingsSaving || !toolHasCustomSettings(tool)) return;
    resetDefaultsToolId = tool.id;
  }

  function resetDefaultsTool() {
    return $detectedTools.find((tool) => tool.id === resetDefaultsToolId) || null;
  }

  function closeResetDefaultsDialog() {
    resetDefaultsToolId = "";
  }

  /** @param {any} tool */
  function resetToolDefaultsInMemory(tool) {
    if (primaryToolPathIsCustom(tool, "skills_dir")) {
      settingsToolPaths = {
        ...settingsToolPaths,
        [tool.id]: {
          config_dir: currentPrimaryToolPath(tool, "config_dir"),
          skills_dir: defaultToolPath(tool, "skills_dir"),
        },
      };
    }
    const nextTargets = { ...settingsInjectionTargets };
    delete nextTargets[tool.id];
    settingsInjectionTargets = nextTargets;
    setCapabilityOverride(tool.id, {
      customRuleSourceType: null,
      customRuleSourcePath: null,
      customGlobalRuleTarget: null,
      customMcpConfigPath: null,
      customToolConfigPath: null,
      sharedSkillDirectRead: null,
    });
  }

  async function confirmResetToolDefaults() {
    const tool = resetDefaultsTool();
    if (!tool) return;
    const affectedItems = toolCustomResetItems(tool);
    const shouldResetSkillsPath = primaryToolPathIsCustom(tool, "skills_dir");
    const shouldInvalidateSkills = affectedItems.some((item) => (
      item.label === $t("settings.tools.label_skills")
      || item.label === $t("settings.tools.label_shared_skill_direct_read")
    ));
    settingsSaving = true;
    settingsMsg = "";
    try {
      resetToolDefaultsInMemory(tool);
      if (shouldResetSkillsPath) {
        await setToolPath(tool.id, currentPrimaryToolPath(tool, "config_dir"), defaultToolPath(tool, "skills_dir"));
      }
      await setToolCapabilityOverrides(tool.id, {
        customRuleSourceType: null,
        customRuleSourcePath: null,
        customGlobalRuleTarget: null,
        customMcpConfigPath: null,
        customToolConfigPath: null,
        sharedSkillDirectRead: null,
      });
      if (settingsEditingId === tool.id) settingsEditingId = null;
      resetDefaultsToolId = "";
      await loadSettingsData({ reason: "tool-reset" });
      if (shouldInvalidateSkills) invalidateSkillInventory();
      settingsMsg = $t("global.msg.saved");
      setTimeout(() => {
        settingsMsg = "";
      }, 2000);
    } catch (/** @type {any} */ e) {
      settingsMsg = $t("global.msg.save_failed", { err: e });
    } finally {
      settingsSaving = false;
    }
  }
</script>

<svelte:window onkeydown={handleSettingsKeydown} />

<div class="view-panel">
  <div class="view-fixed-header">
    <div class="view-header settings-header-row" data-tauri-drag-region>
      <h2 class="settings-page-title" data-tauri-drag-region>{$t("sidebar.settings")}</h2>
      {#if !activeLogViewer}
        <div class="level1-tabs level1-tabs--settings-pair">
          <button type="button" class="l1-tab" class:active={$activeSettingsTab === "general"} onclick={() => setSettingsTab("general")}>
            <Settings2 size={14} />
            {$t("settings.tab.general")}
          </button>
          <button type="button" class="l1-tab" class:active={$activeSettingsTab === "tools"} onclick={() => setSettingsTab("tools")}>
            <Wrench size={14} />
            {$t("settings.tab.tools")}
          </button>
        </div>
      {/if}

      <div class="settings-header-right">
        {#if !activeLogViewer && $activeSettingsTab === "tools"}
          <Tooltip label={$t("settings.tools.add_title")} placement="bottom-end" maxWidth="240px">
            <button
              type="button"
              class="refresh-btn"
              onclick={() => {
                showAddTool = true;
              }}
              aria-label={$t("settings.tools.add_title")}
            >
              <Plus size={14} strokeWidth={2} />
            </button>
          </Tooltip>
        {/if}
      </div>
    </div>
  </div>

  <div class="view-scroll-content" class:settings-log-scroll={activeLogViewer}>
    <ModulePerformanceBar moduleId={settingsDiagnosticsModule} />

    {#if settingsMsg}<div class="inject-msg">{settingsMsg}</div>{/if}

    {#if activeLogViewer}
      <div class="settings-log-viewer-wrap">
        <div class="settings-log-days" aria-label={$t("settings.log_viewer.days")}>
          {#if logFiles.length}
            {#each logFiles as file}
              <button
                type="button"
                class="settings-log-day"
                class:active={selectedLogId === file.id}
                onclick={() => loadSelectedLog(currentLogViewerKind(), file.id)}
              >
                {file.label}
              </button>
            {/each}
          {:else}
            <span class="settings-log-empty">{logLoading ? $t("common.loading") : $t("settings.log_viewer.empty")}</span>
          {/if}
          {#if logTruncated}
            <span class="settings-log-truncated">{$t("settings.log_viewer.truncated")}</span>
          {/if}
        </div>

        <FileViewerShell
          variant="collection-detail"
          title={activeLogViewer === "application" ? $t("settings.log_viewer.application_title") : $t("settings.log_viewer.performance_title")}
          subtitle={logFiles.find((file) => file.id === selectedLogId)?.label || $t("settings.log_viewer.no_file")}
          subtitleMonospace={false}
          backLabel={$t("settings.log_viewer.back")}
          onBack={closeLogViewer}
        >
          {#snippet actions()}
            <button
              type="button"
              class="settings-action-btn"
              disabled={!selectedLogId || logExporting}
              onclick={() => exportSelectedLog(currentLogViewerKind())}
            >
              <Download size={14} /> {logExporting ? $t("settings.log_viewer.exporting") : $t("settings.log_viewer.export")}
            </button>
          {/snippet}

          <ContentEditor
            value={logLoading ? $t("common.loading") : logContent}
            originalValue={logLoading ? $t("common.loading") : logContent}
            editing={false}
            filename={selectedLogId || "log.jsonl"}
            language="json"
            ariaLabel={activeLogViewer === "application" ? $t("settings.log_viewer.application_title") : $t("settings.log_viewer.performance_title")}
            placeholder={$t("settings.log_viewer.empty")}
            showActions={false}
            showFooter={false}
            showModeToggle={false}
            fill={true}
            framed={false}
            minHeight="0"
            emptyPreviewLabel={$t("settings.log_viewer.empty")}
          />
        </FileViewerShell>
      </div>
    {:else if $activeSettingsTab === "general"}
      <div class="st-section">
        <div class="settings-group-card">
          <div class="settings-list-row">
            <div class="settings-row-left">
              <div class="settings-icon">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><path d="M12 16v-4"></path><path d="M12 8h.01"></path></svg>
              </div>
              <div class="settings-info">
                <div class="settings-title">{$t("settings.title.about")}</div>
                <div class="settings-desc">{$t("settings.desc.version", { version: appUpdateVersionLabel() })}</div>
                {#if showRuntimeMarker || showRuntimeDiagnostic}
                  <div class="settings-runtime-marker" data-testid="settings-runtime-marker">
                    <span class="settings-runtime-badge">{runtimeMarkerLabel()}</span>
                    <span class="settings-runtime-desc">{runtimeMarkerDescription()}</span>
                  </div>
                {/if}
                <div class={`settings-update-status settings-update-status--${appUpdateStatusTone()}`} data-testid="settings-update-status">
                  {appUpdateStatusText()}
                </div>
              </div>
            </div>
            <div class="settings-row-right">
              <button type="button" class="settings-action-btn" onclick={openProjectGitHub}>GitHub</button>
              <button
                type="button"
                class="settings-action-btn"
                disabled={!$appUpdateState.canCheck || appUpdateBusy}
                onclick={manualAppUpdateCheck}
              >
                {$appUpdateState.status === "checking" ? $t("settings.update.checking") : $t("settings.action.check_update")}
              </button>
              {#if showSkipUpdateAction}
                <button
                  type="button"
                  class="settings-action-btn"
                  onclick={skipAvailableAppUpdate}
                >
                  {$t("settings.update.skip")}
                </button>
              {/if}
              {#if showInstallUpdateAction}
                <button
                  type="button"
                  class="settings-action-btn primary"
                  onclick={installAppUpdateFromSettings}
                >
                  {$appUpdateState.status === "downloading" || $appUpdateState.status === "installing" ? $t("settings.update.installing") : $t("settings.update.install")}
                </button>
              {/if}
              {#if showRestartUpdateAction}
                <button
                  type="button"
                  class="settings-action-btn primary"
                  onclick={restartForAppUpdate}
                >
                  {$t("settings.update.restart")}
                </button>
              {/if}
            </div>
          </div>
          <div class="settings-divider"></div>

          <div class="settings-list-row">
            <div class="settings-row-left">
              <div class="settings-icon">
                <FileText size={18} />
              </div>
              <div class="settings-info">
                <div class="settings-title">{$t("settings.logs.title")}</div>
                <div class="settings-desc">{$t("settings.logs.desc")}</div>
              </div>
            </div>
            <div class="settings-row-right">
              <button class="settings-action-btn" onclick={() => openLogViewer("application")}>{$t("settings.logs.open")}</button>
            </div>
          </div>
          <div class="settings-divider"></div>

          <div class="settings-list-row">
            <div class="settings-row-left">
              <div class="settings-icon">
                <Settings2 size={18} />
              </div>
              <div class="settings-info">
                <div class="settings-title">{$t("settings.skill_perf.title")}</div>
                <div class="settings-desc">{$t("settings.skill_perf.desc")}</div>
              </div>
            </div>
            <div class="settings-row-right">
              <label class="settings-switch">
                <span class="st-toggle">
                  <input
                    type="checkbox"
                    checked={modulePerformanceDiagnosticsEnabled}
                    aria-label={$t("settings.skill_perf.enable")}
                    onchange={(event) => toggleModulePerformanceDiagnostics(/** @type {HTMLInputElement} */ (event.currentTarget).checked)}
                  />
                  <span class="st-toggle-slider"></span>
                </span>
              </label>
              <button class="settings-action-btn" onclick={() => openLogViewer("modulePerformance")}>{$t("settings.skill_perf.open")}</button>
            </div>
          </div>
          <div class="settings-divider"></div>

          <div class="settings-list-row">
            <div class="settings-row-left">
              <div class="settings-icon">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="13.5" cy="6.5" r=".5"></circle><circle cx="17.5" cy="10.5" r=".5"></circle><circle cx="8.5" cy="7.5" r=".5"></circle><circle cx="6.5" cy="12.5" r=".5"></circle><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"></path></svg>
              </div>
              <div class="settings-info">
                <div class="settings-title">{$t("settings.title.appearance")}</div>
                <div class="settings-desc">{$t("settings.desc.appearance")}</div>
              </div>
            </div>
            <div class="settings-row-right">
              <div class="segmented-control">
                <button class="segmented-btn" class:active={$theme === "system"} onclick={() => changeTheme("system")}>{$t("settings.appearance.system")}</button>
                <button class="segmented-btn" class:active={$theme === "light"} onclick={() => changeTheme("light")}>{$t("settings.appearance.light")}</button>
                <button class="segmented-btn" class:active={$theme === "dark"} onclick={() => changeTheme("dark")}>{$t("settings.appearance.dark")}</button>
              </div>
            </div>
          </div>
          <div class="settings-divider"></div>

          <div class="settings-list-row">
            <div class="settings-row-left">
              <div class="settings-icon">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path><path d="M2 12h20"></path></svg>
              </div>
              <div class="settings-info">
                <div class="settings-title">{$t("settings.language_title")}</div>
                <div class="settings-desc">{$t("settings.language_desc")}</div>
              </div>
            </div>
            <div class="settings-row-right">
              <div class="segmented-control">
                <button class="segmented-btn" class:active={$languagePreference === "system"} onclick={() => switchLanguage("system")}>{$t("settings.language.system")}</button>
                <button class="segmented-btn" class:active={$languagePreference === "zh"} onclick={() => switchLanguage("zh")}>{$t("settings.language.zh")}</button>
                <button class="segmented-btn" class:active={$languagePreference === "en"} onclick={() => switchLanguage("en")}>English</button>
              </div>
            </div>
          </div>
          <div class="settings-divider"></div>

          <div class="settings-list-row settings-list-row--translation">
            <div class="settings-translation-header">
              <div class="settings-row-left">
                <div class="settings-icon">
                  <Languages size={18} />
                </div>
                <div class="settings-info">
                  <div class="settings-title">{$t("settings.translation.title")}</div>
                  <div class="settings-desc">{$t("settings.translation.desc")}</div>
                  {#if translationConfigTesting}
                    <div class="settings-translation-message">{$t("settings.translation.testing")}</div>
                  {:else if translationConfigMessageText}
                    <div class="settings-translation-message">{translationConfigMessageText}</div>
                  {/if}
                </div>
              </div>
              <div class="settings-row-right">
                {#if translationSettingsExpanded}
                  <button
                    type="button"
                    class="icon-btn st-row-action-btn"
                    aria-label={$t("settings.translation.collapse")}
                    disabled={translationConfigSaving || translationConfigTesting}
                    onclick={cancelTranslationSettingsEdit}
                  >
                    <X size={13} />
                  </button>
                  {#if translationSettingsDirty}
                    <button
                      type="button"
                      class="icon-btn st-row-action-btn"
                      aria-label={$t("settings.tools.confirm")}
                      disabled={translationConfigSaving || translationConfigTesting}
                      onclick={saveTranslationProviderSettings}
                    >
                      <Check size={13} />
                    </button>
                  {/if}
                  <Tooltip label={translationConfigTesting ? $t("settings.translation.testing") : $t("settings.translation.test")} placement="bottom">
                    <button
                      type="button"
                      class="icon-btn st-row-action-btn"
                      aria-label={translationConfigTesting ? $t("settings.translation.testing") : $t("settings.translation.test")}
                      disabled={translationConfigSaving || translationConfigTesting}
                      onclick={testTranslationProviderSettings}
                    >
                      <Beaker size={13} />
                    </button>
                  </Tooltip>
                {:else}
                  <button
                    type="button"
                    class="icon-btn st-row-action-btn"
                    aria-label={$t("settings.translation.edit")}
                    disabled={translationConfigSaving || translationConfigTesting}
                    onclick={() => (translationSettingsExpanded = true)}
                  >
                    <Pencil size={13} />
                  </button>
                {/if}
                <label class="settings-translation-header-toggle">
                  <span class="st-toggle">
                    <input
                      type="checkbox"
                      checked={translationProviderConfig.enabled}
                      aria-label={$t("settings.translation.enabled")}
                      disabled={translationConfigSaving || translationConfigTesting}
                      onchange={(event) => setTranslationProviderEnabled(/** @type {HTMLInputElement} */ (event.currentTarget).checked)}
                    />
                    <span class="st-toggle-slider"></span>
                  </span>
                </label>
              </div>
            </div>

            {#if translationSettingsExpanded}
              <div class="settings-row-right settings-row-right--translation">
                <div class="settings-translation-grid">
                  <label class="settings-translation-field">
                    <span>{$t("settings.translation.provider")}</span>
                    <div class="segmented-control settings-translation-provider-control">
                      <button
                        type="button"
                        class="segmented-btn"
                        class:active={translationProviderConfig.provider === "openai-compatible"}
                        onclick={() => setTranslationProviderField("provider", "openai-compatible")}
                      >
                        {$t("settings.translation.provider_openai")}
                      </button>
                      <button
                        type="button"
                        class="segmented-btn"
                        class:active={translationProviderConfig.provider === "anthropic-messages"}
                        onclick={() => setTranslationProviderField("provider", "anthropic-messages")}
                      >
                        {$t("settings.translation.provider_anthropic")}
                      </button>
                    </div>
                  </label>
                  <label class="settings-translation-field">
                    <span>{$t("settings.translation.base_url")}</span>
                    <input
                      class="settings-input"
                      value={translationProviderConfig.baseUrl}
                      placeholder="https://api.openai.com/v1"
                      oninput={(event) => setTranslationProviderField("baseUrl", /** @type {HTMLInputElement} */ (event.currentTarget).value)}
                    />
                  </label>
                  <label class="settings-translation-field">
                    <span>{$t("settings.translation.model")}</span>
                    <input
                      class="settings-input"
                      value={translationProviderConfig.model}
                      placeholder={$t("settings.translation.model_placeholder")}
                      oninput={(event) => setTranslationProviderField("model", /** @type {HTMLInputElement} */ (event.currentTarget).value)}
                    />
                  </label>
                  <label class="settings-translation-field">
                    <span>{translationProviderConfig.apiKeyConfigured ? $t("settings.translation.api_key_configured") : $t("settings.translation.api_key_missing")}</span>
                    <input
                      class="settings-input"
                      type="password"
                      value={translationApiKeyDisplayValue}
                      placeholder={$t("settings.translation.api_key_placeholder")}
                      autocomplete="off"
                      onfocus={() => (translationApiKeyFocused = true)}
                      onblur={() => (translationApiKeyFocused = false)}
                      oninput={(event) => (translationApiKeyDraft = /** @type {HTMLInputElement} */ (event.currentTarget).value)}
                    />
                  </label>
                </div>
              </div>
            {/if}
          </div>
          <div class="settings-divider"></div>

        </div>
      </div>
    {/if}

    {#if $activeSettingsTab === "tools"}
      <div class="st-section">
        <div class="st-card">
          {#each builtInSettingsTools as tool, index}
            {#if index > 0}<div class="st-divider"></div>{/if}
            <div
              class="st-row-block"
              class:st-row-block--focused={highlightedSettingsToolId === tool.id}
              data-settings-tool-id={tool.id}
            >
              <div class="st-row">
                <div class="st-row-left st-row-left--tool">
                  <button
                    type="button"
                    class="icon-btn st-row-expand-btn"
                    aria-label={settingsToolExpanded(tool.id) ? $t("settings.tools.collapse_details") : $t("settings.tools.expand_details")}
                    aria-expanded={settingsToolExpanded(tool.id)}
                    onclick={() => toggleSettingsToolDetails(tool.id)}
                  >
                    {#if settingsToolExpanded(tool.id)}
                      <ChevronDown size={13} strokeWidth={2} />
                    {:else}
                      <ChevronRight size={13} strokeWidth={2} />
                    {/if}
                  </button>
                  <div class="st-row-main">
                    <div class="st-row-title">
                      <ToolIcon toolId={tool.id} size={20} style="margin-right:8px;" />
                      <span class="st-row-tool-name">{tool.name}</span>
                      {#if tool.presence_app_detected}
                        <span class="st-presence-badge">APP</span>
                      {/if}
                      {#if tool.presence_cli_detected}
                        <span class="st-presence-badge">CLI</span>
                      {/if}
                    </div>
                  </div>
                </div>
                <div class="st-row-right">
                  {#if settingsEditingId !== tool.id && toolHasCustomSettings(tool)}
                    <Tooltip label={$t("settings.tools.reset_defaults")} placement="bottom">
                      <button
                        type="button"
                        class="icon-btn st-row-action-btn"
                        aria-label={$t("settings.tools.reset_defaults")}
                        disabled={settingsSaving}
                        onclick={() => openResetDefaultsDialog(tool)}
                      >
                        <RotateCcw size={13} />
                      </button>
                    </Tooltip>
                  {/if}
                  {#if settingsEditingId !== tool.id}
                    <Tooltip label={$t("settings.tools.edit")} placement="bottom">
                      <button
                        type="button"
                        class="icon-btn st-row-action-btn"
                        aria-label={$t("settings.tools.edit")}
                        disabled={settingsSaving}
                        onclick={() => openSettingsEdit(tool.id)}
                      >
                        <Pencil size={13} />
                      </button>
                    </Tooltip>
                  {:else}
                    <div class="st-edit-actions">
                      <Tooltip label={$t("settings.tools.cancel")} placement="bottom">
                        <button
                          type="button"
                          class="icon-btn st-row-action-btn"
                          aria-label={$t("settings.tools.cancel")}
                          disabled={settingsSaving}
                          onclick={cancelSettingsEdit}
                        >
                          <X size={13} />
                      </button>
                    </Tooltip>
                    {#if settingsDirty}
                      <Tooltip label={$t("settings.tools.confirm")} placement="bottom">
                        <button
                          type="button"
                          class="icon-btn st-row-action-btn"
                          aria-label={$t("settings.tools.confirm")}
                          disabled={settingsSaving}
                          onclick={confirmSettingsEdit}
                        >
                          <Check size={13} />
                        </button>
                      </Tooltip>
                    {/if}
                    </div>
                  {/if}
                  <label class="st-toggle">
                    <input type="checkbox" checked={$managedToolIds.includes(tool.id)} onchange={() => toggleManaged(tool.id)} />
                  <span class="st-toggle-slider"></span>
                </label>
              </div>
              </div>
              {#if settingsToolExpanded(tool.id)}
              <div class="st-fields" class:st-fields--editing={settingsEditingId === tool.id}>
                <div class="st-field-row">
                  <span class="st-field-label">{$t("settings.tools.label_rule_directory")}</span>
                  {#if settingsEditingId === tool.id && canEditRuleSource(tool)}
                    <input class="settings-input" value={ruleSourceInputValue(tool)} oninput={(e) => { if (e.target) setRuleSourcePath(tool, /** @type {HTMLInputElement} */ (e.target).value); }} placeholder={$t("settings.tools.rule_source_placeholder")} />
                    <div class="st-field-actions">
                      <Tooltip label={$t("settings.tools.pick_folder")} placement="bottom">
                        <button class="icon-btn st-picker-btn" onclick={() => pickFolder(tool.id, "rule_source")} aria-label={$t("settings.tools.pick_folder")}><Folder size={13} /></button>
                      </Tooltip>
                      {#if customRuleSourcePath(tool)}
                        <Tooltip label={$t("settings.tools.reset")} placement="bottom">
                          <button type="button" class="icon-btn st-picker-btn" aria-label={$t("settings.tools.reset")} onclick={() => resetRuleSource(tool)}><RotateCcw size={13} /></button>
                        </Tooltip>
                      {/if}
                    </div>
                  {:else}
                    <span class="st-field-value">{ruleSourceDisplayValue(tool)}</span>
                    {#if ruleDirectorySourceLabel(tool)}
                      <span class="st-source-badge" class:st-source-badge--override={Boolean(customRuleSourcePath(tool))}>{ruleDirectorySourceLabel(tool)}</span>
                    {/if}
                  {/if}
                  {#if settingsEditingId !== tool.id && sourcePathAttentionLabel(tool, "rule_directory")}
                    <span class="st-field-attention">{sourcePathAttentionLabel(tool, "rule_directory")}</span>
                  {/if}
                </div>
                <div class="st-field-row">
                  <span class="st-field-label">{$t("settings.tools.label_global_rule_file")}</span>
                  {#if settingsEditingId === tool.id && canEditGlobalRuleTarget(tool)}
                    <input class="settings-input" value={effectiveGlobalRuleTarget(tool)} oninput={(e) => { if (e.target) setGlobalRuleTarget(tool, /** @type {HTMLInputElement} */ (e.target).value); }} placeholder={$t("settings.tools.global_rule_file_placeholder")} />
                    <div class="st-field-actions">
                      <Tooltip label={$t("settings.tools.pick_file")} placement="bottom">
                        <button class="icon-btn st-picker-btn" onclick={() => pickFile(tool.id, "global_rule_file")} aria-label={$t("settings.tools.pick_file")}><FileText size={13} /></button>
                      </Tooltip>
                      {#if customGlobalRuleTarget(tool)}
                        <Tooltip label={$t("settings.tools.reset")} placement="bottom">
                          <button type="button" class="icon-btn st-picker-btn" aria-label={$t("settings.tools.reset")} onclick={() => resetGlobalRuleTarget(tool)}><RotateCcw size={13} /></button>
                        </Tooltip>
                      {/if}
                    </div>
                  {:else}
                    <span class="st-field-value">{injectionDisplayValue(tool)}</span>
                    {#if globalRuleTargetSource(tool)}
                      <span class="st-source-badge st-source-badge--override">{globalRuleTargetSource(tool)}</span>
                    {/if}
                  {/if}
                  {#if settingsEditingId !== tool.id && sourcePathAttentionLabel(tool, "global_rule_file")}
                    <span class="st-field-attention">{sourcePathAttentionLabel(tool, "global_rule_file")}</span>
                  {/if}
                </div>
                <div class="st-field-row">
                  <span class="st-field-label">{$t("settings.tools.label_skills")}</span>
                  {#if settingsEditingId === tool.id}
                    <input class="settings-input" value={currentPrimaryToolPath(tool, "skills_dir")} oninput={(e) => { if (e.target) setSettingsPath(tool.id, "skills_dir", /** @type {HTMLInputElement} */ (e.target).value); }} />
                    <div class="st-field-actions">
                      <Tooltip label={$t("settings.tools.pick_folder")} placement="bottom">
                        <button class="icon-btn st-picker-btn" onclick={() => pickFolder(tool.id, "skills_dir")} aria-label={$t("settings.tools.pick_folder")}><Folder size={13} /></button>
                      </Tooltip>
                      {#if primaryToolPathIsCustom(tool, "skills_dir")}
                        <Tooltip label={$t("settings.tools.reset")} placement="bottom">
                          <button type="button" class="icon-btn st-picker-btn" aria-label={$t("settings.tools.reset")} onclick={() => resetPrimaryToolPath(tool, "skills_dir")}><RotateCcw size={13} /></button>
                        </Tooltip>
                      {/if}
                    </div>
                  {:else}
                    <span class="st-field-value">{skillsDisplayValue(tool)}</span>
                    {#if primaryToolPathSourceLabel(tool, "skills_dir")}
                      <span class="st-source-badge st-source-badge--override">{primaryToolPathSourceLabel(tool, "skills_dir")}</span>
                    {/if}
                  {/if}
                  {#if settingsEditingId !== tool.id && sourcePathAttentionLabel(tool, "skills_dir")}
                    <span class="st-field-attention">{sourcePathAttentionLabel(tool, "skills_dir")}</span>
                  {/if}
                </div>
                <div class="st-field-row">
                  <span class="st-field-label">{$t("settings.tools.label_shared_skill_direct_read")}</span>
                  {#if settingsEditingId === tool.id}
                    <div class="segmented-control st-inline-segmented">
                      <button type="button" class="segmented-btn" class:active={sharedSkillEffectiveValue(tool) === true} onclick={() => setSharedSkillDirectReadMode(tool, true)}>{$t("settings.tools.direct_read_yes")}</button>
                      <button type="button" class="segmented-btn" class:active={sharedSkillEffectiveValue(tool) === false} onclick={() => setSharedSkillDirectReadMode(tool, false)}>{$t("settings.tools.direct_read_no")}</button>
                    </div>
                    {#if sharedSkillOverrideValue(tool) !== null}
                      <div class="st-field-actions">
                        <Tooltip label={$t("settings.tools.reset")} placement="bottom">
                          <button type="button" class="icon-btn st-picker-btn" aria-label={$t("settings.tools.reset")} onclick={() => resetSharedSkillSupport(tool)}><RotateCcw size={13} /></button>
                        </Tooltip>
                      </div>
                    {/if}
                  {:else}
                    <span class="st-field-value">{directReadLabel(sharedSkillEffectiveValue(tool))}</span>
                    {#if sharedSkillOverrideValue(tool) !== null}
                      <span class="st-source-badge st-source-badge--override">{$t("settings.tools.source_user_override")}</span>
                    {/if}
                  {/if}
                </div>
                <div class="st-field-row">
                  <span class="st-field-label">{$t("settings.tools.label_mcp_config")}</span>
                  {#if settingsEditingId === tool.id}
                    <input class="settings-input" value={mcpConfigInputValue(tool)} oninput={(e) => { if (e.target) setMcpConfigPath(tool, /** @type {HTMLInputElement} */ (e.target).value); }} placeholder={$t("settings.tools.mcp_config_placeholder")} />
                    <div class="st-field-actions">
                      <Tooltip label={$t("settings.tools.pick_file")} placement="bottom">
                        <button class="icon-btn st-picker-btn" onclick={() => pickFile(tool.id, "mcp_config")} aria-label={$t("settings.tools.pick_file")}><FileText size={13} /></button>
                      </Tooltip>
                      {#if customMcpConfigPath(tool)}
                        <Tooltip label={$t("settings.tools.reset")} placement="bottom">
                          <button type="button" class="icon-btn st-picker-btn" aria-label={$t("settings.tools.reset")} onclick={() => setMcpConfigPath(tool, "")}><RotateCcw size={13} /></button>
                        </Tooltip>
                      {/if}
                    </div>
                  {:else}
                    <span class="st-field-value">{mcpConfigDisplayValue(tool)}</span>
                    {#if customMcpConfigPath(tool)}
                      <span class="st-source-badge st-source-badge--override">{$t("settings.tools.source_user_override")}</span>
                    {/if}
                  {/if}
                  {#if settingsEditingId !== tool.id && sourcePathAttentionLabel(tool, "mcp_config")}
                    <span class="st-field-attention">{sourcePathAttentionLabel(tool, "mcp_config")}</span>
                  {/if}
                </div>
                <div class="st-field-row">
                  <span class="st-field-label">{$t("settings.tools.label_tool_config_files")}</span>
                  {#if settingsEditingId === tool.id}
                    <input class="settings-input" value={ordinaryConfigInputValue(tool)} oninput={(e) => { if (e.target) setToolConfigPath(tool, /** @type {HTMLInputElement} */ (e.target).value); }} placeholder={$t("settings.tools.tool_config_placeholder")} />
                    <div class="st-field-actions">
                      <Tooltip label={$t("settings.tools.pick_file")} placement="bottom">
                        <button class="icon-btn st-picker-btn" onclick={() => pickFile(tool.id, "tool_config")} aria-label={$t("settings.tools.pick_file")}><FileText size={13} /></button>
                      </Tooltip>
                      {#if customToolConfigPath(tool)}
                        <Tooltip label={$t("settings.tools.reset")} placement="bottom">
                          <button type="button" class="icon-btn st-picker-btn" aria-label={$t("settings.tools.reset")} onclick={() => setToolConfigPath(tool, "")}><RotateCcw size={13} /></button>
                        </Tooltip>
                      {/if}
                    </div>
                  {:else}
                    <span class="st-field-value">{ordinaryConfigDisplayValue(tool)}</span>
                    {#if customToolConfigPath(tool)}
                      <span class="st-source-badge st-source-badge--override">{$t("settings.tools.source_user_override")}</span>
                    {/if}
                  {/if}
                  {#if settingsEditingId !== tool.id && sourcePathAttentionLabel(tool, "tool_config")}
                    <span class="st-field-attention">{sourcePathAttentionLabel(tool, "tool_config")}</span>
                  {/if}
                </div>
              </div>
              {/if}
            </div>
          {/each}

          {#each customTools as customTool}
            <div class="st-divider"></div>
            <div class="st-row-block" data-settings-tool-id={customTool.id}>
              <div class="st-row">
                <div class="st-row-left st-row-left--tool">
                  <button
                    type="button"
                    class="icon-btn st-row-expand-btn"
                    aria-label={settingsToolExpanded(customTool.id) ? $t("settings.tools.collapse_details") : $t("settings.tools.expand_details")}
                    aria-expanded={settingsToolExpanded(customTool.id)}
                    onclick={() => toggleSettingsToolDetails(customTool.id)}
                  >
                    {#if settingsToolExpanded(customTool.id)}
                      <ChevronDown size={13} strokeWidth={2} />
                    {:else}
                      <ChevronRight size={13} strokeWidth={2} />
                    {/if}
                  </button>
                  <div class="st-row-main">
                    <div class="st-row-title"><ToolIcon toolId={customTool.id} size={20} style="margin-right:8px;" /> {customTool.name} <span class="st-badge-custom">{$t("settings.tools.badge_custom")}</span></div>
                  </div>
                </div>
                <div class="st-row-right">
                  {#if settingsEditingId !== customTool.id}
                    <Tooltip label={$t("settings.tools.edit")} placement="bottom">
                      <button
                        type="button"
                        class="icon-btn st-row-action-btn"
                        aria-label={$t("settings.tools.edit")}
                        disabled={settingsSaving}
                        onclick={() => openSettingsEdit(customTool.id)}
                      >
                        <Pencil size={13} />
                      </button>
                    </Tooltip>
                  {:else}
                    <div class="st-edit-actions">
                      <Tooltip label={$t("settings.tools.cancel")} placement="bottom">
                        <button
                          type="button"
                          class="icon-btn st-row-action-btn"
                          aria-label={$t("settings.tools.cancel")}
                          disabled={settingsSaving}
                          onclick={cancelSettingsEdit}
                        >
                          <X size={13} />
                      </button>
                    </Tooltip>
                    {#if settingsDirty}
                      <Tooltip label={$t("settings.tools.confirm")} placement="bottom">
                        <button
                          type="button"
                          class="icon-btn st-row-action-btn"
                          aria-label={$t("settings.tools.confirm")}
                          disabled={settingsSaving}
                          onclick={confirmSettingsEdit}
                        >
                          <Check size={13} />
                        </button>
                      </Tooltip>
                    {/if}
                    </div>
                  {/if}
                  <label class="st-toggle">
                    <input type="checkbox" checked={$managedToolIds.includes(customTool.id)} onchange={() => toggleManaged(customTool.id)} />
                    <span class="st-toggle-slider"></span>
                  </label>
                  <button class="btn-delete" onclick={() => doRemoveCustomTool(customTool.id)}>{$t("settings.tools.delete")}</button>
                </div>
              </div>
              {#if settingsToolExpanded(customTool.id)}
              <div class="st-fields" class:st-fields--editing={settingsEditingId === customTool.id}>
                <div class="st-field-row">
                  <span class="st-field-label">{$t("settings.tools.label_rule_directory")}</span>
                  {#if settingsEditingId === customTool.id}
                    <input class="settings-input" value={customTool.rule_directory || ""} oninput={(e) => { if (e.target) setCustomToolField(customTool.id, "rule_directory", /** @type {HTMLInputElement} */ (e.target).value); }} />
                    <div class="st-field-actions">
                      <Tooltip label={$t("settings.tools.pick_folder")} placement="bottom">
                        <button class="icon-btn st-picker-btn" onclick={() => pickCustomToolFolder(customTool.id, "rule_directory")} aria-label={$t("settings.tools.pick_folder")}><Folder size={13} /></button>
                      </Tooltip>
                    </div>
                  {:else}
                    <span class="st-field-value">{customTool.rule_directory || $t("settings.tools.value_missing")}</span>
                  {/if}
                </div>
                <div class="st-field-row">
                  <span class="st-field-label">{$t("settings.tools.label_global_rule_file")}</span>
                  {#if settingsEditingId === customTool.id}
                    <input class="settings-input" value={customTool.global_rule_file || ""} oninput={(e) => { if (e.target) setCustomToolField(customTool.id, "global_rule_file", /** @type {HTMLInputElement} */ (e.target).value); }} />
                    <div class="st-field-actions">
                      <Tooltip label={$t("settings.tools.pick_file")} placement="bottom">
                        <button class="icon-btn st-picker-btn" onclick={() => pickCustomToolFile(customTool.id, "global_rule_file")} aria-label={$t("settings.tools.pick_file")}><FileText size={13} /></button>
                      </Tooltip>
                    </div>
                  {:else}
                    <span class="st-field-value">{customTool.global_rule_file || $t("settings.tools.value_missing")}</span>
                  {/if}
                </div>
                <div class="st-field-row">
                  <span class="st-field-label">{$t("settings.tools.label_skills")}</span>
                  {#if settingsEditingId === customTool.id}
                    <input class="settings-input" value={customToolSkillsValue(customTool)} oninput={(e) => { if (e.target) setSettingsPath(customTool.id, "skills_dir", /** @type {HTMLInputElement} */ (e.target).value); }} />
                    <div class="st-field-actions">
                      <Tooltip label={$t("settings.tools.pick_folder")} placement="bottom">
                        <button class="icon-btn st-picker-btn" onclick={() => pickCustomToolFolder(customTool.id, "skills_dir")} aria-label={$t("settings.tools.pick_folder")}><Folder size={13} /></button>
                      </Tooltip>
                    </div>
                  {:else}
                    <span class="st-field-value">{customToolSkillsValue(customTool) || $t("settings.tools.value_missing")}</span>
                  {/if}
                </div>
                <div class="st-field-row">
                  <span class="st-field-label">{$t("settings.tools.label_shared_skill_direct_read")}</span>
                  {#if settingsEditingId === customTool.id}
                    <div class="segmented-control st-inline-segmented">
                      <button type="button" class="segmented-btn" class:active={customToolSharedSkillDirectRead(customTool) === true} onclick={() => setCustomToolField(customTool.id, "shared_skill_direct_read", true)}>{$t("settings.tools.direct_read_yes")}</button>
                      <button type="button" class="segmented-btn" class:active={customToolSharedSkillDirectRead(customTool) === false} onclick={() => setCustomToolField(customTool.id, "shared_skill_direct_read", false)}>{$t("settings.tools.direct_read_no")}</button>
                    </div>
                  {:else}
                    <span class="st-field-value">{directReadLabel(customToolSharedSkillDirectRead(customTool))}</span>
                  {/if}
                </div>
                <div class="st-field-row">
                  <span class="st-field-label">{$t("settings.tools.label_mcp_config")}</span>
                  {#if settingsEditingId === customTool.id}
                    <input class="settings-input" value={customTool.mcp_config || ""} oninput={(e) => { if (e.target) setCustomToolField(customTool.id, "mcp_config", /** @type {HTMLInputElement} */ (e.target).value); }} />
                    <div class="st-field-actions">
                      <Tooltip label={$t("settings.tools.pick_file")} placement="bottom">
                        <button class="icon-btn st-picker-btn" onclick={() => pickCustomToolFile(customTool.id, "mcp_config")} aria-label={$t("settings.tools.pick_file")}><FileText size={13} /></button>
                      </Tooltip>
                    </div>
                  {:else}
                    <span class="st-field-value">{customTool.mcp_config || $t("settings.tools.value_missing")}</span>
                  {/if}
                </div>
                <div class="st-field-row">
                  <span class="st-field-label">{$t("settings.tools.label_tool_config_files")}</span>
                  {#if settingsEditingId === customTool.id}
                    <input class="settings-input" value={customTool.tool_config || ""} oninput={(e) => { if (e.target) setCustomToolField(customTool.id, "tool_config", /** @type {HTMLInputElement} */ (e.target).value); }} />
                    <div class="st-field-actions">
                      <Tooltip label={$t("settings.tools.pick_file")} placement="bottom">
                        <button class="icon-btn st-picker-btn" onclick={() => pickCustomToolFile(customTool.id, "tool_config")} aria-label={$t("settings.tools.pick_file")}><FileText size={13} /></button>
                      </Tooltip>
                    </div>
                  {:else}
                    <span class="st-field-value">{customTool.tool_config || $t("settings.tools.value_missing")}</span>
                  {/if}
                </div>
              </div>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}

  </div>
</div>

{#if showAddTool}
  <div
    class="modal-overlay"
    role="presentation"
    tabindex="-1"
    onclick={(e) => {
      if (e.target === e.currentTarget) showAddTool = false;
    }}
    onkeydown={(e) => {
      if (e.target !== e.currentTarget) return;
      if (e.key === "Escape" || e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        showAddTool = false;
      }
    }}
  >
    <div class="modal add-tool-modal" role="dialog" aria-modal="true" tabindex="0">
      <div class="modal-header add-tool-modal-header">
        <div class="add-tool-title-block">
          <h3 class="modal-title">{$t("settings.tools.add_title")}</h3>
          <p class="add-tool-intro">{$t("settings.tools.add_intro")}</p>
        </div>
        <Tooltip label={$t("settings.tools.add_close")} placement="bottom">
          <button class="btn-close" onclick={() => (showAddTool = false)} aria-label={$t("settings.tools.add_close")}><X size={16} /></button>
        </Tooltip>
      </div>
      <div class="modal-body add-tool-modal-body">
        <div class="add-tool-field">
          <label for="add-tool-name" class="add-tool-required-label">
            <span>{$t("settings.tools.add_name")}</span>
            <span class="required-mark" aria-hidden="true">*</span>
          </label>
          <input id="add-tool-name" class="settings-input" bind:value={newTool.name} required aria-required="true" aria-label={$t("settings.tools.add_name")} />
        </div>
        <div class="add-tool-field">
          <label for="add-tool-rule-directory">{$t("settings.tools.label_rule_directory")}</label>
          <div class="add-tool-path-row">
            <input id="add-tool-rule-directory" class="settings-input" bind:value={newTool.rule_directory} />
            <Tooltip label={$t("settings.tools.pick_folder")} placement="bottom">
              <button type="button" class="icon-btn st-picker-btn" onclick={() => pickNewToolFolder("rule_directory")} aria-label={$t("settings.tools.pick_folder")}><Folder size={13} /></button>
            </Tooltip>
          </div>
        </div>
        <div class="add-tool-field">
          <label for="add-tool-global-rule-file">{$t("settings.tools.label_global_rule_file")}</label>
          <div class="add-tool-path-row">
            <input id="add-tool-global-rule-file" class="settings-input" bind:value={newTool.global_rule_file} />
            <Tooltip label={$t("settings.tools.pick_file")} placement="bottom">
              <button type="button" class="icon-btn st-picker-btn" onclick={() => pickNewToolFile("global_rule_file")} aria-label={$t("settings.tools.pick_file")}><FileText size={13} /></button>
            </Tooltip>
          </div>
        </div>
        <div class="add-tool-field">
          <label for="add-tool-skills">{$t("settings.tools.label_skills")}</label>
          <div class="add-tool-path-row">
            <input id="add-tool-skills" class="settings-input" bind:value={newTool.skills_dir} />
            <Tooltip label={$t("settings.tools.pick_folder")} placement="bottom">
              <button type="button" class="icon-btn st-picker-btn" onclick={() => pickNewToolFolder("skills_dir")} aria-label={$t("settings.tools.pick_folder")}><Folder size={13} /></button>
            </Tooltip>
          </div>
        </div>
        <div class="add-tool-field">
          <span class="add-tool-label">{$t("settings.tools.label_shared_skill_direct_read")}</span>
          <div class="segmented-control st-inline-segmented add-tool-segmented">
            <button type="button" class="segmented-btn" class:active={newTool.shared_skill_direct_read === true} onclick={() => (newTool = { ...newTool, shared_skill_direct_read: true })}>{$t("settings.tools.direct_read_yes")}</button>
            <button type="button" class="segmented-btn" class:active={newTool.shared_skill_direct_read === false} onclick={() => (newTool = { ...newTool, shared_skill_direct_read: false })}>{$t("settings.tools.direct_read_no")}</button>
          </div>
        </div>
        <div class="add-tool-field">
          <label for="add-tool-mcp-config">{$t("settings.tools.label_mcp_config")}</label>
          <div class="add-tool-path-row">
            <input id="add-tool-mcp-config" class="settings-input" bind:value={newTool.mcp_config} />
            <Tooltip label={$t("settings.tools.pick_file")} placement="bottom">
              <button type="button" class="icon-btn st-picker-btn" onclick={() => pickNewToolFile("mcp_config")} aria-label={$t("settings.tools.pick_file")}><FileText size={13} /></button>
            </Tooltip>
          </div>
        </div>
        <div class="add-tool-field">
          <label for="add-tool-tool-config">{$t("settings.tools.label_tool_config_files")}</label>
          <div class="add-tool-path-row">
            <input id="add-tool-tool-config" class="settings-input" bind:value={newTool.tool_config} />
            <Tooltip label={$t("settings.tools.pick_file")} placement="bottom">
              <button type="button" class="icon-btn st-picker-btn" onclick={() => pickNewToolFile("tool_config")} aria-label={$t("settings.tools.pick_file")}><FileText size={13} /></button>
            </Tooltip>
          </div>
        </div>
        <div class="add-tool-source-hint" class:ready={customToolHasFeatureSource(newTool)} role="status">
          <AlertTriangle size={14} />
          <span>{customToolHasFeatureSource(newTool) ? $t("settings.tools.add_source_ready_hint") : $t("settings.tools.add_source_minimum_hint")}</span>
        </div>
      </div>
      <div class="modal-footer add-tool-modal-footer">
        <button class="btn btn-cancel" onclick={() => (showAddTool = false)}>{$t("settings.tools.cancel")}</button>
        <button type="button" class="btn btn-primary" disabled={!newTool.name.trim()} onclick={doAddCustomTool}><Check size={14} /> {$t("settings.tools.add_confirm")}</button>
      </div>
    </div>
  </div>
{/if}

{#if resetDefaultsTool()}
  {@const resetTool = resetDefaultsTool()}
  {@const resetItems = resetTool ? toolCustomResetItems(resetTool) : []}
  <div
    class="modal-overlay"
    role="presentation"
    tabindex="-1"
    onclick={(e) => {
      if (e.target === e.currentTarget) closeResetDefaultsDialog();
    }}
    onkeydown={(e) => {
      if (e.target !== e.currentTarget) return;
      if (e.key === "Escape") {
        e.preventDefault();
        closeResetDefaultsDialog();
      }
    }}
  >
    <div class="modal reset-defaults-modal" role="dialog" aria-modal="true" aria-label={$t("settings.tools.reset_defaults_title")} tabindex="0">
      <div class="modal-header reset-defaults-modal-header">
        <h3 class="modal-title">{$t("settings.tools.reset_defaults_title")}</h3>
        <Tooltip label={$t("settings.tools.add_close")} placement="bottom">
          <button class="icon-btn" onclick={closeResetDefaultsDialog} aria-label={$t("settings.tools.add_close")}><X size={16} /></button>
        </Tooltip>
      </div>
      <div class="modal-body reset-defaults-modal-body">
        <p class="reset-defaults-desc">{$t("settings.tools.reset_defaults_desc", { tool: resetTool?.name || "" })}</p>
        <div class="reset-defaults-list">
          <div class="reset-defaults-head">
            <span></span>
            <span>{$t("settings.tools.reset_defaults_current")}</span>
            <span>{$t("settings.tools.reset_defaults_default")}</span>
          </div>
          {#each resetItems as item}
            <div class="reset-defaults-row">
              <span class="reset-defaults-label">{item.label}</span>
              <span class="reset-defaults-value">{item.current || "—"}</span>
              <span class="reset-defaults-value">{item.next || "—"}</span>
            </div>
          {/each}
        </div>
      </div>
      <div class="modal-footer reset-defaults-modal-footer">
        <button class="btn btn-secondary" disabled={settingsSaving} onclick={closeResetDefaultsDialog}>{$t("settings.tools.cancel")}</button>
        <button type="button" class="btn btn-primary" disabled={settingsSaving} onclick={confirmResetToolDefaults}>
          <RotateCcw size={13} /> {$t("settings.tools.reset_defaults_confirm")}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .settings-group-card { background: var(--bg-card); border-radius: 16px; border: 1px solid var(--border-color); overflow: hidden; display: flex; flex-direction: column; }
  .settings-list-row { display: flex; justify-content: space-between; align-items: center; padding: 18px 20px; transition: background 0.2s; }
  .settings-list-row:hover { background: var(--bg-hover); }
  .settings-list-row--translation { flex-direction: column; align-items: stretch; gap: 12px; }
  .settings-translation-header { display: flex; align-items: center; justify-content: space-between; gap: 16px; min-width: 0; }
  .settings-row-left { display: flex; align-items: center; gap: 16px; min-width: 0; }
  .settings-icon { display: flex; align-items: center; justify-content: center; width: 36px; height: 36px; font-size: 18px; background: rgba(150, 150, 150, 0.08); border-radius: 8px; }
  .settings-info { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .settings-title { font-size: 14px; font-weight: 600; color: var(--color-text-main); }
  .settings-desc { font-size: 11px; color: var(--color-text-muted); opacity: 0.8; }
  .settings-runtime-marker { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; min-width: 0; margin-top: 2px; }
  .settings-runtime-badge { max-width: 140px; padding: 2px 7px; border-radius: 6px; border: 1px solid rgba(59,130,246,0.35); background: rgba(59,130,246,0.08); color: #3b82f6; font-size: 10px; line-height: 1.4; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .settings-runtime-desc { color: var(--color-text-muted); font-size: 10px; line-height: 1.4; overflow-wrap: anywhere; }
  .settings-update-status { max-width: 560px; color: var(--color-text-muted); font-size: 11px; line-height: 1.45; overflow-wrap: anywhere; }
  .settings-update-status--available { color: #f59e0b; }
  .settings-update-status--failed { color: #dc2626; }
  .settings-update-status--restart { color: #059669; }
  .settings-row-right { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
  .settings-row-right--translation { width: 100%; justify-content: flex-start; padding-left: 52px; box-sizing: border-box; }
  .settings-translation-grid { display: flex; flex-direction: column; gap: 10px; width: min(100%, 640px); }
  .settings-translation-field { min-width: 0; display: grid; grid-template-columns: 96px minmax(0, 1fr); align-items: center; gap: 12px; }
  .settings-translation-header-toggle { display: inline-flex; align-items: center; gap: 10px; min-height: 28px; cursor: pointer; }
  .settings-translation-field span { font-size: 10px; color: var(--color-text-muted); line-height: 1.4; }
  .settings-translation-provider-control { height: var(--control-height); min-height: var(--control-height); justify-content: stretch; }
  .settings-translation-provider-control .segmented-btn { flex: 1 1 0; min-width: 0; min-height: 0; height: 100%; padding: 0 8px; }
  .settings-translation-message { margin-top: 2px; color: #3b82f6; font-size: 11px; line-height: 1.45; }
  .settings-switch { display: inline-flex; align-items: center; gap: 8px; color: var(--color-text-muted); font-size: 11px; white-space: nowrap; cursor: pointer; }
  .settings-divider { height: 1px; background: var(--border-color); margin: 0 20px; }
  .inject-msg { font-size: 11px; color: #60a5fa; padding: 6px 10px; background: rgba(96,165,250,0.08); border-radius: 8px; margin-bottom: 10px; }
  .settings-log-scroll { display: flex; flex-direction: column; min-height: 0; overflow: hidden; }
  .settings-log-viewer-wrap { flex: 1 1 0; min-height: 0; display: flex; flex-direction: column; gap: 12px; }
  .settings-log-viewer-wrap :global(.file-viewer-shell) { flex: 1 1 0; min-height: 0; }
  .settings-log-viewer-wrap :global(.file-viewer-titlebar) { padding-left: calc((var(--file-viewer-header-height) - 30px) / 2); }
  .settings-log-viewer-wrap :global(.file-viewer-content) { background: var(--bg-card); }
  .settings-log-viewer-wrap :global(.cm-content),
  .settings-log-viewer-wrap :global(.cm-line) {
    overflow-wrap: anywhere;
    word-break: break-all;
  }
  .settings-log-days { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; min-width: 0; padding: 0 2px; }
  .settings-log-day { min-height: 28px; padding: 0 10px; border-radius: 8px; border: 1px solid var(--border-color); background: var(--bg-hover); color: var(--color-text-muted); font-size: 11px; cursor: pointer; }
  .settings-log-day.active { color: var(--color-text-main); border-color: var(--border-active); background: var(--bg-active); }
  .settings-log-empty,
  .settings-log-truncated { color: var(--color-text-muted); font-size: 11px; }
  .settings-log-truncated { padding: 3px 8px; border-radius: 6px; border: 1px solid rgba(245, 158, 11, 0.28); color: #f59e0b; background: rgba(245, 158, 11, 0.08); }

  .st-section { margin-bottom: 28px; }
  .st-card { background: var(--bg-card); border-radius: 16px; padding: 0; overflow: hidden; border: 1px solid var(--border-color); }
  .st-divider { height: 1px; background: var(--border-color); margin: 0 16px; }
  .st-row { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px; }
  .st-row-block { padding: 4px 18px 14px; border-radius: 12px; transition: background 0.2s, box-shadow 0.2s; }
  .st-row-block--focused { background: rgba(245, 158, 11, 0.08); box-shadow: inset 0 0 0 1px rgba(245, 158, 11, 0.28); }
  .st-row-block .st-row { padding: 10px 0; }
  .st-row-left { flex: 1; min-width: 0; }
  .st-row-left--tool { display: flex; align-items: center; gap: 8px; }
  .st-row-main { min-width: 0; }
  .st-row-title { display: flex; align-items: center; min-width: 0; font-size: 15px; font-weight: 600; color: var(--color-text-main); }
  .st-row-tool-name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .st-row-right { display: flex; align-items: center; gap: 12px; flex-shrink: 0; margin-left: 16px; }
  .st-row-expand-btn,
  .st-row-action-btn { width: 28px; height: 28px; border-radius: 7px; }
  .st-edit-actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }

  .st-edit-actions :global(.btn) {
    height: var(--control-height);
    min-height: var(--control-height);
    transition: none;
  }
  .st-toggle { position: relative; display: inline-block; width: 42px; height: 24px; cursor: pointer; }
  .st-toggle input { opacity: 0; width: 0; height: 0; }
  .st-toggle-slider { position: absolute; inset: 0; background: var(--border-color); border-radius: 12px; transition: background 0.25s; }
  .st-toggle-slider::before { content: ""; position: absolute; width: 18px; height: 18px; left: 3px; bottom: 3px; background: #fff; border-radius: 50%; transition: transform 0.25s; }
  .st-toggle input:checked + .st-toggle-slider { background: #3b82f6; }
  .st-toggle input:checked + .st-toggle-slider::before { transform: translateX(18px); }

  .st-fields { display: flex; flex-direction: column; gap: 8px; padding-top: 14px; margin-top: 4px; }
  .st-field-row { display: grid; grid-template-columns: 120px minmax(0, 1fr) auto auto; align-items: center; gap: 10px; min-height: 22px; }
  .st-fields--editing .st-field-row { grid-template-columns: 120px minmax(0, 1fr) auto; }
  .st-field-label { font-size: 11px; color: var(--color-text-muted); white-space: nowrap; }
  .st-field-value { min-width: 0; font-size: 11px; color: var(--color-text-muted); font-family: "SF Mono", monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .st-field-attention { max-width: 160px; padding: 2px 7px; border-radius: 6px; border: 1px solid rgba(245, 158, 11, 0.24); color: #f59e0b; background: rgba(245, 158, 11, 0.08); font-size: 10px; line-height: 1.4; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .st-source-badge { max-width: 140px; justify-self: end; padding: 2px 7px; border-radius: 6px; border: 1px solid var(--border-color); color: var(--color-text-muted); font-size: 10px; line-height: 1.4; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .st-source-badge--override { border-color: rgba(59,130,246,0.35); color: #3b82f6; background: rgba(59,130,246,0.08); }
  .st-field-actions { display: inline-flex; align-items: center; justify-content: flex-end; gap: 10px; }
  .st-inline-segmented { min-width: 0; justify-self: start; }
  .st-presence-badge { flex-shrink: 0; margin-left: 8px; padding: 1px 6px; border-radius: 5px; border: 1px solid rgba(59, 130, 246, 0.28); color: #2563eb; background: rgba(59, 130, 246, 0.08); font-size: 9px; line-height: 1.4; font-weight: 600; }
  .st-badge-custom { font-size: 9px; padding: 1px 6px; background: rgba(245,158,11,0.12); color: #f59e0b; border-radius: 3px; margin-left: 6px; font-weight: 400; }

  .settings-input { width: 100%; min-width: 0; background: var(--bg-input); color: var(--color-text-main); border: 1px solid var(--border-color); border-radius: 8px; padding: 6px 10px; font-size: 11px; font-family: "SF Mono", monospace; outline: none; transition: border-color 0.15s, background 0.15s; }
  .settings-input:focus { border-color: var(--border-active); }
  .settings-input::placeholder { color: var(--color-text-muted); opacity: 0.4; }
  .btn-delete { display: inline-flex; align-items: center; justify-content: center; font-size: 12px; font-weight: 500; min-height: var(--control-height); padding: 0 12px; background: var(--bg-danger); color: #ef4444; border: none; border-radius: 8px; cursor: pointer; transition: background 0.15s, color 0.15s; box-sizing: border-box; }
  .btn-delete:hover { background: rgba(239,68,68,0.16); color: #dc2626; }
  .st-picker-btn { width: 28px; height: 28px; border-radius: 7px; align-self: center; }

  .modal-overlay { position: fixed; inset: 0; background: var(--overlay-bg); display: flex; align-items: center; justify-content: center; z-index: 1000; backdrop-filter: var(--overlay-filter); -webkit-backdrop-filter: var(--overlay-filter); }
  .add-tool-modal { max-width: 680px; width: min(94vw, 680px); }
  .add-tool-modal-header { display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; }
  .add-tool-title-block { display: flex; flex: 1; min-width: 0; flex-direction: column; gap: 6px; }
  .add-tool-modal-header h3 { font-size: 15px; font-weight: 600; color: var(--color-text-main); margin: 0; }
  .add-tool-intro { margin: 0; max-width: none; font-size: 12px; line-height: 1.5; color: var(--color-text-muted); }
  .add-tool-modal-body { display: flex; flex-direction: column; gap: 12px; max-height: min(64vh, 540px); overflow-x: hidden; overflow-y: auto; }
  .add-tool-field { display: flex; min-width: 0; flex-direction: column; gap: 4px; }
  .add-tool-field label,
  .add-tool-label { font-size: 11px; font-weight: 500; color: var(--color-text-muted); }
  .add-tool-required-label { display: inline-flex; align-items: center; gap: 3px; width: fit-content; }
  .required-mark { color: #ef4444; font-weight: 600; }
  .add-tool-path-row { display: grid; min-width: 0; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: 8px; }
  .add-tool-path-row .settings-input { min-width: 0; box-sizing: border-box; }
  .add-tool-segmented { width: min(100%, 260px); }
  .add-tool-source-hint { display: flex; width: 100%; box-sizing: border-box; align-items: flex-start; gap: 8px; border: 1px solid color-mix(in srgb, #f59e0b 42%, transparent); border-radius: 8px; background: color-mix(in srgb, #f59e0b 10%, transparent); color: #b45309; font-size: 12px; line-height: 1.45; padding: 9px 10px; }
  .add-tool-source-hint :global(svg) { flex: 0 0 auto; margin-top: 1px; }
  .add-tool-source-hint.ready { border-color: color-mix(in srgb, var(--color-accent) 36%, transparent); background: color-mix(in srgb, var(--color-accent) 10%, transparent); color: var(--color-accent); }
  .add-tool-modal-footer { display: flex; justify-content: flex-end; gap: 12px; }
  .reset-defaults-modal { max-width: 560px; width: min(92vw, 560px); }
  .reset-defaults-modal-header { display: flex; justify-content: space-between; align-items: center; }
  .reset-defaults-modal-header h3 { font-size: 15px; font-weight: 600; color: var(--color-text-main); margin: 0; }
  .reset-defaults-modal-body { display: flex; flex-direction: column; gap: 14px; }
  .reset-defaults-desc { margin: 0; color: var(--color-text-muted); font-size: 12px; line-height: 1.5; }
  .reset-defaults-list { display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .reset-defaults-head,
  .reset-defaults-row { display: grid; grid-template-columns: 108px minmax(0, 1fr) minmax(0, 1fr); gap: 10px; align-items: center; }
  .reset-defaults-head { color: var(--color-text-muted); font-size: 10px; }
  .reset-defaults-row { padding: 7px 0; border-top: 1px solid var(--border-color); }
  .reset-defaults-label { color: var(--color-text-main); font-size: 11px; white-space: nowrap; }
  .reset-defaults-value { min-width: 0; color: var(--color-text-muted); font-size: 11px; font-family: "SF Mono", monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .reset-defaults-modal-footer { display: flex; justify-content: flex-end; gap: 12px; }

</style>
