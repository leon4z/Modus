<script>
  // RulesModule owns the Rules page behavior while the main page remains the section shell.
  import BaseCard from "$lib/shared/components/BaseCard.svelte";
  import ConfirmDialog from "$lib/shared/components/ConfirmDialog.svelte";
  import ContentEditor from "$lib/shared/components/ContentEditor.svelte";
  import FileWorkspaceSearchResults from "$lib/shared/components/FileWorkspaceSearchResults.svelte";
  import FileWorkspaceNavigation from "$lib/shared/components/FileWorkspaceNavigation.svelte";
  import FileViewerShell from "$lib/shared/components/FileViewerShell.svelte";
  import ModulePerformanceBar from "$lib/shared/diagnostics/ModulePerformanceBar.svelte";
  import PrimaryState from "$lib/shared/components/PrimaryState.svelte";
  import ProtectedToolSelector from "$lib/shared/components/ProtectedToolSelector.svelte";
  import ToolIcon from "$lib/shared/components/ToolIcon.svelte";
  import Tooltip from "$lib/shared/components/Tooltip.svelte";
  import SourceReadModeToggle from "$lib/shared/components/SourceReadModeToggle.svelte";
  import CopyModal from "$lib/features/rules/components/CopyModal.svelte";
  import { Plus, RefreshCw, Search, X, ArrowDownToLine, AlertTriangle, Check, Save, Pencil, ScrollText, ChevronDown, ChevronRight, ChevronUp, Eye, Unlink, FilePlus, FolderPlus, Trash2, FileCheck2 } from "lucide-svelte";
  import { onDestroy, onMount, tick, untrack } from "svelte";
  import { get } from "svelte/store";
  import { activeRulesTab, activeTool, activeToolId, managedTools, pendingSubTab, getToolName, getCertifiedGlobalRuleTarget, getRulePromptDecision, loadTools, getInjectionTargets, projectCapability, projectionAllowsAction, setToolCapabilityOverrides, summarizeEffectiveToolCapabilities } from "$lib/features/tools/index.js";
  import { t } from "$lib/shared/i18n/index.js";
  import {
    listDefaultRules,
    getDefaultRuleInjectionBaselines,
    setDefaultRuleInjectionBaselines,
    saveDefaultRule,
    writeRule,
    createRuleFile,
    createRuleDirectory,
    deleteRuleEntry,
    renameRuleEntry,
    deleteDefaultRule,
    injectDefaultRules,
    readRuleContent,
    getManagedRulesState,
    leaveRuleManagementTargets,
    diffRules,
  } from "$lib/features/rules/api/rules.js";
  import { logAppEvent } from "$lib/shared/logging/appLogger.js";
  import { APP_CONTEXT_MENU_OPEN_EVENT } from "$lib/shared/utils/contextMenuEvents.js";
  import { buildFileWorkspaceSearchGroups } from "$lib/shared/utils/fileWorkspaceSearch.js";
  import { countCurrentContentMatches, stepCurrentContentMatchIndex } from "$lib/shared/utils/currentContentSearch.js";
  import {
    buildDefaultRuleBaselines,
    defaultRuleBaselineMatches,
    defaultRuleFingerprint,
    getCurrentCustomPendingTargetIds as resolveCurrentCustomPendingTargetIds,
    getCustomPendingRuleIds as resolveCustomPendingRuleIds,
    getMergedRuleTargetIds as resolveMergedRuleTargetIds,
    getRuleTargetIds as resolveRuleTargetIds,
    getStoredOrCurrentPendingTargetIds as resolveStoredOrCurrentPendingTargetIds,
    hasAnyDefaultRuleBaseline,
    markCustomRulePendingTargetsState,
    normalizeDefaultRuleBaselines,
    persistCustomInjectedBaselinesForTargetsState,
    persistInjectedBaselinesState,
  } from "$lib/features/rules/domain/ruleInjectionState.js";
  import { getVisualVerificationState, isVisualVerificationMode } from "$lib/dev/visualVerification/fixtures.js";
  import {
    beginModulePerformanceRun,
    finishAndRecordModulePerformanceRun,
    markModulePerformance,
    MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE,
    MODULE_PERFORMANCE_ROLE_INTERACTIVE,
    MODULE_PERFORMANCE_ROLE_VISIBLE,
    setModulePerformanceCounters,
    trackModulePerformanceRequest,
    updateModulePerformanceSummary,
  } from "$lib/shared/diagnostics/modulePerformance.js";

  let { searchOpen = $bindable(false), globalSearchQuery = $bindable(""), refreshing = false, onRefresh = async () => {} } = $props();
  const visualRulesState = isVisualVerificationMode() ? getVisualVerificationState() : "";
  if (isVisualVerificationMode() && visualRulesState.startsWith("rules-tool")) {
    activeRulesTab.set("tool");
  }
  /** @param {string} tab */
  function normalizeRulesTab(tab) {
    return tab === "tool" ? "tool" : "global";
  }

  /** @type {"global" | "custom" | "tool"} */
  let rulesTab = $state(normalizeRulesTab(get(activeRulesTab)));

  const TOOL_RULE_NAV_COLLAPSED_KEY = "modus.rules.toolFileNavigation.collapsed";
  let toolRuleNavigationCollapsed = $state(false);

  onMount(() => {
    try {
      toolRuleNavigationCollapsed = localStorage.getItem(TOOL_RULE_NAV_COLLAPSED_KEY) === "true";
    } catch {
      /* ignore */
    }
  });

  $effect(() => {
    try {
      localStorage.setItem(TOOL_RULE_NAV_COLLAPSED_KEY, toolRuleNavigationCollapsed ? "true" : "false");
    } catch {
      /* ignore */
    }
  });

  /** @param {any} rule */
  async function openToolRuleDetail(rule) {
    const run = startRulesRun("open-tool-rule");
    if (!rule) {
      finishAndRecordModulePerformanceRun(run, "partial");
      return;
    }
    if (currentTool?.id && rule.path) {
      selectedToolRulePathByTool = { ...selectedToolRulePathByTool, [currentTool.id]: rule.path };
    }
    if (isRuleUnreadable(rule)) {
      selectedToolRule = { ...rule, content: "" };
      markModulePerformance(run, "rule-unreadable", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "partial");
      return;
    }
    selectedToolRule = { ...rule, content: typeof rule.content === "string" ? rule.content : "" };
    try {
      const freshContent = await trackModulePerformanceRequest(run, "tool-rule-read", () => readRuleContent(rule.path));
      if (typeof freshContent === "string") {
        const freshRule = { ...rule, content: freshContent };
        selectedToolRule = freshRule;
        syncLoadedToolRuleContent(freshRule, freshContent);
      }
      markModulePerformance(run, "tool-rule-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
      markModulePerformance(run, "tool-rule-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "success");
      await logAppEvent({
        level: "info",
        category: "rules",
        action: "tool_rule_open",
        result: "ok",
        toolId: currentTool?.id,
        targetRole: "tool_rule",
        targetPath: rule.path,
      });
    } catch {
      selectedToolRule = rule; // fallback to cached
      markModulePerformance(run, "tool-rule-fallback", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "partial");
      await logAppEvent({
        level: "warn",
        category: "rules",
        action: "tool_rule_open",
        result: "fallback_cached",
        toolId: currentTool?.id,
        targetRole: "tool_rule",
        targetPath: rule?.path,
      });
    }
  }
  function closeToolRuleDetail() {
    selectedToolRule = null;
    isSingleToolRuleEditing = false;
    singleToolRuleMsg = "";
  }
  let currentTool = $derived($activeTool);
  let currentToolId = $derived($activeToolId);
  /** @type {any} */
  let selectedToolRule = $state(null);
  let selectedToolRulePathByTool = $state(/** @type {Record<string, string>} */ ({}));
  let autoSelectToolRuleKey = $state("");
  let toolRuleContextKey = $state("");

  // Folder-backed rule groups should expose the useful files by default.
  let collapsedGroups = $state(new Set());
  /** @param {string} group */
  function toggleGroup(group) {
    let key = (currentTool?.id || "") + ":" + group;
    let next = new Set(collapsedGroups);
    if (next.has(key)) next.delete(key); else next.add(key);
    collapsedGroups = next;
  }
  /** @param {string} group */
  function isCollapsed(group) {
    return collapsedGroups.has((currentTool?.id || "") + ":" + group);
  }
  /** @param {string} group */
  function isToolRuleDirectoryExpanded(group) {
    return !isCollapsed(group);
  }

  let toolRuleTreeRows = $derived.by(() => {
    if (!currentTool) return [];
    let rules = currentTool.rule_sources || [];
    if (selectedToolRule?.isNewlyCreated && selectedToolRule.toolId === currentTool.id && selectedToolRule.path) {
      const createdRule = {
        ...selectedToolRule,
        label: selectedToolRule.label || baseName(selectedToolRule.path),
      };
      if (!rules.some((/** @type {any} */ rule) => rule.path === createdRule.path)) {
        rules = [...rules, createdRule];
      }
    }
    const localDirectories = localToolRuleDirectoriesByTool[currentTool.id] || [];
    if (!rules.some((/** @type {any} */ rule) => !isRuleUnreadable(rule) || isRuleDirectoryPlaceholder(rule)) && localDirectories.length === 0) return [];
    /** @type {Map<string, { id: string, label: string, path: string, depth: number, files: any[], children: string[], count: number }>} */
    const directories = new Map();
    /** @type {any[]} */
    const rootFiles = [];
    /** @param {string} id @param {string} [path] */
    const ensureDirectory = (id, path = "") => {
      if (directories.has(id)) {
        const existing = directories.get(id);
        if (existing && path && !existing.path) existing.path = path;
        return existing;
      }
      const segments = id.split("/").filter(Boolean);
      const parent = segments.length > 1 ? segments.slice(0, -1).join("/") : "";
      const node = {
        id,
        label: segments.at(-1) || id,
        path,
        depth: Math.max(0, segments.length - 1),
        files: [],
        children: [],
        count: 0,
      };
      directories.set(id, node);
      if (parent) {
        const parentPath = path ? ancestorDir(path, segments.length - parent.split("/").filter(Boolean).length) : "";
        const parentNode = ensureDirectory(parent, parentPath);
        if (parentNode && !parentNode.children.includes(id)) parentNode.children.push(id);
      }
      return node;
    };

    for (const rule of rules) {
      if (isRuleDirectoryPlaceholder(rule)) {
        const key = rule.group || baseName(rule.path || "");
        const segments = key.split("/").filter(Boolean);
        if (!segments.length) continue;
        const directoryPath = withoutTrailingSlash(rule.path || "");
        for (let index = 1; index <= segments.length; index += 1) {
          const id = segments.slice(0, index).join("/");
          ensureDirectory(id, directoryPath ? ancestorDir(directoryPath, segments.length - index) : "");
        }
        continue;
      }
      const key = rule.group || "";
      const segments = key.split("/").filter(Boolean);
      if (!segments.length) {
        rootFiles.push(rule);
        continue;
      }
      const fileDir = dirName(rule.path || "");
      for (let index = 1; index <= segments.length; index += 1) {
        const id = segments.slice(0, index).join("/");
        ensureDirectory(id, fileDir ? ancestorDir(fileDir, segments.length - index) : "");
      }
      ensureDirectory(key)?.files.push(rule);
    }
    for (const entry of localDirectories) {
      const segments = String(entry.id || "").split("/").filter(Boolean);
      for (let index = 1; index <= segments.length; index += 1) {
        const id = segments.slice(0, index).join("/");
        ensureDirectory(id, ancestorDir(entry.path, segments.length - index));
      }
    }
    const sortRules = (/** @type {any[]} */ entries) => entries.sort((a, b) => String(a.label || "").localeCompare(String(b.label || "")) || String(a.path || "").localeCompare(String(b.path || "")));
    for (const node of directories.values()) {
      node.files = sortRules(node.files);
      node.children.sort((a, b) => (directories.get(a)?.label || a).localeCompare(directories.get(b)?.label || b));
    }
    /** @param {string} id */
    const countFiles = (id) => {
      const node = directories.get(id);
      if (!node) return 0;
      node.count = node.files.length + node.children.reduce((total, childId) => total + countFiles(childId), 0);
      return node.count;
    };
    for (const id of Array.from(directories.keys()).filter((id) => !id.includes("/"))) {
      countFiles(id);
    }
    /** @type {any[]} */
    const rows = sortRules(rootFiles).map((rule) => ({ type: "file", rule, depth: 0 }));
    /** @param {string} id */
    const appendDirectory = (id) => {
      const node = directories.get(id);
      if (!node) return;
      rows.push({ type: "directory", ...node });
      if (!isToolRuleDirectoryExpanded(id)) return;
      for (const file of node.files) {
        rows.push({ type: "file", rule: file, depth: node.depth + 1 });
      }
      for (const childId of node.children) appendDirectory(childId);
    };
    for (const id of Array.from(directories.keys()).filter((id) => !id.includes("/")).sort((a, b) => (directories.get(a)?.label || a).localeCompare(directories.get(b)?.label || b))) {
      appendDirectory(id);
    }
    return rows;
  });
  let unfilteredToolRules = $derived.by(() => {
    let rules = (Array.isArray(currentTool?.rule_sources) ? currentTool.rule_sources : [])
      .filter((/** @type {any} */ rule) => !isRuleDirectoryPlaceholder(rule));
    if (selectedToolRule?.isNewlyCreated && selectedToolRule.toolId === currentTool?.id && selectedToolRule.path) {
      const createdRule = {
        ...selectedToolRule,
        label: selectedToolRule.label || baseName(selectedToolRule.path),
      };
      if (!rules.some((/** @type {any} */ rule) => rule.path === createdRule.path)) {
        rules = [...rules, createdRule];
      }
    }
    return rules;
  });
  let filteredToolRules = $derived.by(() => {
    const rules = unfilteredToolRules;
    return rules;
  });
  let readableToolRules = $derived(filteredToolRules.filter((/** @type {any} */ rule) => !isRuleUnreadable(rule)));
  let ruleCreateDialogOpen = $state(false);
  let ruleCreateDialogToolId = $state("");
  let ruleCreateDialogLocationId = $state("");
  let ruleCreateDialogFileName = $state("");
  let ruleCreateDialogUseChildDir = $state(false);
  let ruleCreateDialogChildDirName = $state("");
  let toolRuleCreateMsg = $state("");
  let toolRuleCreateMsgTimer = /** @type {ReturnType<typeof setTimeout> | null} */ (null);
  const TOOL_RULE_FEEDBACK_TIMEOUT_MS = 3000;
  let ruleTreeContextMenu = $state(/** @type {any} */ (null));
  let ruleTreeNameDialog = $state(/** @type {any} */ (null));
  let ruleTreeNameDialogValue = $state("");
  let ruleTreeActionBusy = $state(false);
  let ruleFileConfirmDialog = $state(/** @type {any} */ (null));
  let localToolRuleDirectoriesByTool = $state(/** @type {Record<string, Array<{ id: string, path: string }>>} */ ({}));
  let activeToolRule = $derived(selectedToolRule);
  let toolRuleNavigationItems = $derived(toolRuleTreeRows.map((/** @type {any} */ row) => {
    if (row.type === "directory") {
      return {
        id: `dir:${row.id}`,
        kind: row.depth === 0 ? "root" : "directory",
        label: row.label,
        path: row.path || row.id,
        depth: row.depth,
        expandable: true,
        expanded: isToolRuleDirectoryExpanded(row.id),
        meta: $t("common.stats_files", { count: row.count }),
        ariaLabel: `${row.label} ${$t("common.stats_files", { count: row.count })}`,
        raw: row,
      };
    }
    const rule = row.rule;
    const injectionTarget = isEffectiveGlobalRuleTargetPath(rule.path || "");
    return {
      id: `file:${rule.path || rule.label}`,
      kind: "file",
      label: rule.label,
      path: rule.path || "",
      depth: row.depth,
      selected: Boolean(activeToolRule?.path && activeToolRule.path === rule.path),
      abnormal: isRuleUnreadable(rule),
      meta: injectionTarget ? $t("rules.tool.injection_target_marker") : "",
      ariaLabel: [
        rule.label,
        isRuleUnreadable(rule) ? $t("rules.tool.unreadable") : "",
        injectionTarget ? $t("rules.tool.injection_target_marker") : "",
      ].filter(Boolean).join(" "),
      rule,
    };
  }));
  let singleToolRuleKey = $state("");
  let singleToolRuleContent = $state("");
  let singleToolRuleOriginal = $state("");
  let singleToolRuleSaving = $state(false);
  let singleToolRuleMsg = $state("");
  let singleToolRuleMdPreview = $state(true);
  let isSingleToolRuleEditing = $state(false);
  let toolRuleSearchTarget = $state({
    fileId: "",
    query: "",
    matchIndex: 0,
    version: 0,
  });
  let toolRuleSearchResultsOpen = $state(false);
  let toolRuleSearchAutoOpenKey = $state("");
  let ruleSearchInput = $state(/** @type {HTMLInputElement | undefined} */ (undefined));
  let toolRuleSearchAnchor = $state(/** @type {HTMLDivElement | undefined} */ (undefined));
  let isSingleToolRuleMarkdown = $derived(Boolean(activeToolRule?.path?.toLowerCase().endsWith(".md")));
  let isSingleToolRuleDirty = $derived(String(singleToolRuleContent ?? "") !== String(singleToolRuleOriginal ?? ""));
  let singleToolRuleLineCount = $derived(String(singleToolRuleContent ?? "").split("\n").length);
  let toolRuleSearchFiles = $derived(unfilteredToolRules.map((/** @type {any} */ rule) => ({
    id: rule.path || rule.label,
    label: rule.label || baseName(rule.path || ""),
    path: rule.path || "",
    content: activeToolRule?.path === rule.path ? singleToolRuleContent : rule.content || "",
    raw: rule,
  })));
  let toolRuleSearchGroups = $derived.by(() => buildFileWorkspaceSearchGroups({
    query: rulesTab === "tool" ? globalSearchQuery : "",
    files: toolRuleSearchFiles,
    currentFileId: activeToolRule?.path || "",
    currentContent: activeToolRule ? singleToolRuleContent : "",
  }));
  let canShowToolRuleSearchResults = $derived(
    searchOpen && rulesTab === "tool" && toolRuleSearchResultsOpen && Boolean(globalSearchQuery.trim()) && toolRuleSearchFiles.length > 0
  );
  let activeToolRuleExternalSearchQuery = $derived(
    searchOpen && rulesTab === "tool" && Boolean(globalSearchQuery.trim()) && !isRuleUnreadable(activeToolRule) ? globalSearchQuery.trim() : ""
  );
  let activeToolRuleExternalSearchMatchIndex = $derived(
    toolRuleSearchTarget.query === activeToolRuleExternalSearchQuery && toolRuleSearchTarget.fileId === activeToolRule?.path
      ? toolRuleSearchTarget.matchIndex
      : 0
  );
  let activeToolRuleExternalSearchVersion = $derived(
    toolRuleSearchTarget.query === activeToolRuleExternalSearchQuery && toolRuleSearchTarget.fileId === activeToolRule?.path
      ? toolRuleSearchTarget.version
      : 0
  );
  let commonRuleExternalSearchQuery = $derived(
    searchOpen && rulesTab === "global" ? globalSearchQuery.trim() : ""
  );
  let showCommonRuleSearchControls = $derived(Boolean(commonRuleExternalSearchQuery && rulesTab === "global"));

  $effect(() => {
    const nextKey = searchOpen && rulesTab === "tool" ? globalSearchQuery.trim() : "";
    if (!nextKey) {
      toolRuleSearchAutoOpenKey = "";
      toolRuleSearchResultsOpen = false;
      return;
    }
    if (nextKey !== toolRuleSearchAutoOpenKey) {
      toolRuleSearchAutoOpenKey = nextKey;
      toolRuleSearchResultsOpen = true;
    }
  });

  export function focusModuleSearch() {
    searchOpen = true;
    toolRuleSearchResultsOpen = rulesTab === "tool";
    tick().then(() => ruleSearchInput?.focus());
  }

  /** @param {"next" | "previous"} direction */
  function moveCommonRuleContentSearch(direction) {
    if (commonRuleSearchMatchCount <= 0) return;
    commonRuleSearchMatchIndex = stepCurrentContentMatchIndex(
      commonRuleSearchMatchIndex,
      commonRuleSearchMatchCount,
      direction,
    );
    commonRuleSearchVersion += 1;
  }

  /** @param {any} rule @param {string} content */
  function syncLoadedToolRuleContent(rule, content) {
    if (!rule?.path || activeToolRule?.path !== rule.path) return;
    if (isSingleToolRuleEditing && isSingleToolRuleDirty) return;
    singleToolRuleContent = content;
    singleToolRuleOriginal = content;
    singleToolRuleMsg = "";
  }

  /** @param {any} rule */
  function expandToolRuleForRule(rule) {
    const group = String(rule?.group || "").trim();
    if (!group) return;
    const parts = group.split("/").filter(Boolean);
    if (parts.length === 0) return;
    const next = new Set(collapsedGroups);
    const toolId = currentTool?.id || "";
    for (let index = 1; index <= parts.length; index += 1) {
      next.delete(`${toolId}:${parts.slice(0, index).join("/")}`);
    }
    collapsedGroups = next;
  }

  /** @param {any} result */
  async function activateToolRuleSearchResult(result) {
    const rule = result?.file?.raw;
    if (!rule?.path) return;
    expandToolRuleForRule(rule);
    toolRuleSearchResultsOpen = false;
    toolRuleSearchTarget = {
      fileId: rule.path,
      query: String(result.query || globalSearchQuery || "").trim(),
      matchIndex: Number(result.matchIndex) || 0,
      version: toolRuleSearchTarget.version + 1,
    };
    if (activeToolRule?.path !== rule.path) await openToolRuleDetail(rule);
  }

  /** @param {PointerEvent} event */
  function handleToolRuleSearchOutsidePointerDown(event) {
    if (!toolRuleSearchResultsOpen) return;
    const target = event.target;
    if (toolRuleSearchAnchor && target instanceof Node && toolRuleSearchAnchor.contains(target)) return;
    toolRuleSearchResultsOpen = false;
  }

  /** @param {string} raw */
  function stripCapabilityPath(raw) {
    return String(raw || "").split("#")[0].trim();
  }

  /** @param {any} rule */
  function isRuleUnreadable(rule) {
    return rule?.diagnostic === "unreadable";
  }

  /** @param {any} rule */
  function isRuleDirectoryPlaceholder(rule) {
    return String(rule?.format || "").toLowerCase() === "directory";
  }

  /** @param {string} path */
  function baseName(path) {
    const clean = stripCapabilityPath(path).replace(/\/+$/, "");
    return clean.split("/").filter(Boolean).pop() || "";
  }

  /** @param {string} path */
  function dirName(path) {
    const clean = stripCapabilityPath(path).replace(/\/+$/, "");
    const parts = clean.split("/").filter(Boolean);
    if (parts.length <= 1) return "";
    const prefix = clean.startsWith("/") ? "/" : "";
    return prefix + parts.slice(0, -1).join("/");
  }

  /** @param {string} rootPath @param {string} filePath */
  function ruleGroupFromCreateRoot(rootPath, filePath) {
    const root = withoutTrailingSlash(rootPath);
    const parent = withoutTrailingSlash(dirName(filePath));
    if (!root || !parent || parent === root) return "";
    return parent.startsWith(`${root}/`) ? parent.slice(root.length + 1) : "";
  }

  /** @param {string} path @param {number} levels */
  function ancestorDir(path, levels) {
    let current = stripCapabilityPath(path).replace(/\/+$/, "");
    for (let index = 0; index < levels; index += 1) {
      current = dirName(current);
    }
    return current;
  }

  /** @param {string} path */
  function withoutTrailingSlash(path) {
    return stripCapabilityPath(path).replace(/\/+$/, "");
  }

  /** @param {string} value */
  function sourcePathSuffix(value) {
    const source = withoutTrailingSlash(value);
    return source.startsWith("~/") ? source.slice(2) : "";
  }

  /** @param {string} value @param {boolean} caseInsensitive */
  function normalizeRulePathForCompare(value, caseInsensitive = false) {
    const clean = withoutTrailingSlash(value);
    return caseInsensitive ? clean.toLocaleLowerCase() : clean;
  }

  /** @param {string} target @param {string} source @param {boolean} directorySource @param {{ caseInsensitive?: boolean }} [options] */
  function uiPathMatchesSource(target, source, directorySource, options = {}) {
    const caseInsensitive = Boolean(options.caseInsensitive);
    const cleanTarget = withoutTrailingSlash(target);
    const cleanSource = withoutTrailingSlash(source);
    if (!cleanTarget || !cleanSource || cleanSource.includes("*")) return false;
    const compareTarget = normalizeRulePathForCompare(cleanTarget, caseInsensitive);
    const compare = (/** @type {string} */ candidate) => {
      const cleanCandidate = normalizeRulePathForCompare(candidate, caseInsensitive);
      return directorySource
        ? compareTarget === cleanCandidate || compareTarget.startsWith(`${cleanCandidate}/`)
        : compareTarget === cleanCandidate;
    };
    if (compare(cleanSource)) return true;
    const suffix = sourcePathSuffix(cleanSource);
    if (!suffix) return false;
    const marker = caseInsensitive ? `/${suffix}`.toLocaleLowerCase() : `/${suffix}`;
    return directorySource
      ? compareTarget.endsWith(marker) || compareTarget.includes(`${marker}/`)
      : compareTarget.endsWith(marker);
  }

  /** @param {string} path */
  function looksLikeDirectory(path) {
    const name = baseName(path);
    return !name || !name.includes(".");
  }

  /** @param {string} dir @param {string} name */
  function joinRulePath(dir, name) {
    return `${String(dir || "").replace(/\/+$/, "")}/${name}`;
  }

  /** @param {string} name */
  function isPlainRuleSegment(name) {
    const value = String(name || "").trim();
    return Boolean(value)
      && value !== "."
      && value !== ".."
      && !value.includes("/")
      && !value.includes("\\")
      && !value.includes("\0");
  }

  /** @param {string} name */
  function cleanRuleFileName(name) {
    const cleaned = String(name || "").trim();
    if (!isPlainRuleSegment(cleaned)) return "";
    return /\.[a-z0-9]+$/i.test(cleaned) ? cleaned : `${cleaned}.md`;
  }

  /** @param {string} name @param {string} defaultSuffix */
  function cleanRuleFileNameForFormat(name, defaultSuffix = ".md") {
    const cleaned = String(name || "").trim();
    if (!isPlainRuleSegment(cleaned)) return "";
    const suffix = defaultSuffix || ".md";
    if (cleaned.endsWith(suffix)) return cleaned;
    const stem = cleaned.replace(/\.instructions\.md$/i, "").replace(/\.(md|mdc)$/i, "");
    return `${stem}${suffix}`;
  }

  /** @param {any} capability */
  function ruleFileDefaultSuffix(capability) {
    if (capability?.format === "mdc") return ".mdc";
    return capability?.format === "instructions_markdown" ? ".instructions.md" : ".md";
  }

  /** @param {string} suffix */
  function ruleFilePlaceholderForSuffix(suffix) {
    return $t("rules.tool.create_name_placeholder");
  }

  /** @param {string} name */
  function cleanRuleChildDirectoryName(name) {
    const cleaned = String(name || "").trim();
    return isPlainRuleSegment(cleaned) ? cleaned : "";
  }

  /** @param {string} name @param {string} fallbackExtension */
  function cleanRuleRenameFileName(name, fallbackExtension = ".md") {
    const cleaned = String(name || "").trim();
    if (!isPlainRuleSegment(cleaned)) return "";
    return /\.[a-z0-9]+$/i.test(cleaned) ? cleaned : `${cleaned}${fallbackExtension || ".md"}`;
  }

  /** @param {string} path */
  function fileExtension(path) {
    const name = baseName(path);
    const match = name.match(/(\.[a-z0-9]+)$/i);
    return match ? match[1] : ".md";
  }

  /** @param {string} path */
  function ruleFileSuffixForPath(path) {
    return String(path || "").endsWith(".instructions.md") ? ".instructions.md" : fileExtension(path);
  }

  /** @param {any} tool @param {string} path @param {{ caseInsensitive?: boolean }} [options] */
  function toolRulePathKnownForTool(tool, path, options = {}) {
    const raw = stripCapabilityPath(path);
    return (tool?.rule_sources || []).some((/** @type {any} */ rule) =>
      !isRuleDirectoryPlaceholder(rule) && uiPathMatchesSource(rule?.path || "", raw, false, options)
    );
  }

  /** @param {any} tool */
  function effectiveGlobalRuleSummary(tool) {
    return summarizeEffectiveToolCapabilities(tool || {}, {});
  }

  /** @param {any} tool */
  function effectiveGlobalRuleTargetPath(tool) {
    return stripCapabilityPath(effectiveGlobalRuleSummary(tool).rules?.effectiveTarget || "");
  }

  /** @param {any} tool */
  function userConfiguredGlobalRuleTargetPath(tool) {
    return stripCapabilityPath(effectiveGlobalRuleSummary(tool).rules?.overrideTarget || "");
  }

  /** @param {any} tool */
  function sharedSkillDirectReadOverrideValue(tool) {
    const summary = effectiveGlobalRuleSummary(tool);
    return summary.skills?.source === "user_override" ? summary.skills?.overrideValue ?? null : null;
  }

  /** @param {string} path */
  function isEffectiveGlobalRuleTargetPath(path) {
    const target = effectiveGlobalRuleTargetPath(currentTool);
    return Boolean(target) && uiPathMatchesSource(path, target, false);
  }

  /** @param {any} projection */
  function isToolRuleSourceProjection(projection) {
    if (projection?.sourceRole === "global_target" && projection?.evidence?.source_confidence === "user_configured") {
      return false;
    }
    return projection?.sourceRole === "global_target" || projection?.sourceRole === "native_file_source";
  }

  /** @param {any} tool @param {any} capability */
  function isToolRuleSourceCapability(tool, capability) {
    const projection = projectCapability(tool?.id || "", "rules", capability);
    const path = stripCapabilityPath(projection?.evidence?.source_path || "");
    return isToolRuleSourceProjection(projection)
      && projectionAllowsAction(projection, "view")
      && Boolean(path)
      && !String(projection?.evidence?.source_path || "").includes("*");
  }

  /** @param {any} tool @param {any} capability */
  function isToolRuleCreatableCapability(tool, capability) {
    const projection = projectCapability(tool?.id || "", "rules", capability);
    const path = stripCapabilityPath(projection?.evidence?.source_path || "");
    return isToolRuleSourceProjection(projection)
      && projectionAllowsAction(projection, "create")
      && Boolean(path)
      && !String(projection?.evidence?.source_path || "").includes("*");
  }

  /** @param {any} tool @param {any} capability @param {string} path @param {string} action @param {boolean} directoryTarget */
  function isToolRuleActionCapability(tool, capability, path, action, directoryTarget) {
    const projection = projectCapability(tool?.id || "", "rules", capability);
    const sourcePath = stripCapabilityPath(projection?.evidence?.source_path || capability?.source_path || "");
    const sourceIsDirectory = capability?.format === "directory" || looksLikeDirectory(sourcePath);
    return isToolRuleSourceProjection(projection)
      && projectionAllowsAction(projection, action)
      && Boolean(sourcePath)
      && !String(projection?.evidence?.source_path || capability?.source_path || "").includes("*")
      && (!directoryTarget || sourceIsDirectory)
      && uiPathMatchesSource(path, sourcePath, sourceIsDirectory);
  }

  /** @param {any} tool @param {string} path @param {string} action @param {boolean} directoryTarget */
  function toolRulePathAllowsAction(tool, path, action, directoryTarget = false) {
    if (!tool || !path) return false;
    return (tool.capabilities || []).some((/** @type {any} */ capability) =>
      isToolRuleActionCapability(tool, capability, path, action, directoryTarget)
    );
  }

  /** @param {any} tool @param {string} path */
  function ruleFileDefaultSuffixForPath(tool, path) {
    const capability = (tool?.capabilities || []).find((/** @type {any} */ item) =>
      isToolRuleActionCapability(tool, item, path, "create", true)
    );
    return ruleFileDefaultSuffix(capability);
  }

  /** @param {string} path */
  function isCertifiedGlobalRuleTargetPath(path) {
    const certifiedTarget = stripCapabilityPath(getCertifiedGlobalRuleTarget(currentTool));
    return Boolean(certifiedTarget) && uiPathMatchesSource(path, certifiedTarget, false);
  }

  /** @param {string} path */
  function isDeclaredFixedToolRuleFile(path) {
    if (!currentTool || !path) return false;
    return (currentTool.capabilities || []).some((/** @type {any} */ capability) => {
      const projection = projectCapability(currentTool?.id || "", "rules", capability);
      const sourcePath = stripCapabilityPath(projection?.evidence?.source_path || capability?.source_path || "");
      return projection?.sourceRole === "global_target"
        && Boolean(sourcePath)
        && !String(projection?.evidence?.source_path || capability?.source_path || "").includes("*")
        && uiPathMatchesSource(path, sourcePath, false);
    });
  }

  /** @param {any} item */
  function canSetToolRuleItemAsGlobalRuleFile(item) {
    if (!currentTool || !item || item.kind !== "file") return false;
    const rule = item.rule;
    const path = rule?.path || "";
    if (!path || isRuleUnreadable(rule)) return false;
    if (isEffectiveGlobalRuleTargetPath(path)) return false;
    if (isDeclaredFixedToolRuleFile(path) && !isCertifiedGlobalRuleTargetPath(path)) return false;
    return toolRulePathAllowsAction(currentTool, path, "save", false);
  }

  /** @param {any} item */
  function canRenameToolRuleItem(item) {
    if (!currentTool || !item) return false;
    const path = item.kind === "file" ? item.rule?.path : item.raw?.path;
    const parent = dirName(path);
    if (!path || !parent) return false;
    if (item.kind === "file") {
      if (isDeclaredFixedToolRuleFile(path)) return false;
      return toolRulePathAllowsAction(currentTool, path, "delete", false)
        && toolRulePathAllowsAction(currentTool, parent, "create", true);
    }
    return toolRulePathAllowsAction(currentTool, path, "delete", true)
      && toolRulePathAllowsAction(currentTool, parent, "create", true);
  }

  /** @param {any} item */
  function canDeleteToolRuleItem(item) {
    if (!currentTool || !item) return false;
    if (item.kind === "file") return toolRulePathAllowsAction(currentTool, item.rule?.path, "delete", false);
    return toolRulePathAllowsAction(currentTool, item.raw?.path, "delete", true);
  }

  /** @param {any} item */
  function canCreateInToolRuleDirectory(item) {
    if (!currentTool || !item || item.kind === "file") return false;
    return toolRulePathAllowsAction(currentTool, item.raw?.path, "create", true);
  }

  /** @param {any} tool @param {any} capability */
  function isInjectableGlobalRuleFileCapability(tool, capability) {
    const projection = projectCapability(tool?.id || "", "rules", capability);
    const path = stripCapabilityPath(capability?.source_path || "");
    return projection?.sourceRole === "global_target"
      && projectionAllowsAction(projection, "inject")
      && capability?.format !== "directory"
      && !looksLikeDirectory(path);
  }

  let hasToolRuleSourceRoot = $derived.by(() => {
    if (!currentTool) return false;
    if (unfilteredToolRules.length > 0) return true;
    return (currentTool.capabilities || []).some((/** @type {any} */ capability) => isToolRuleSourceCapability(currentTool, capability));
  });

  /** @param {any} tool */
  function getToolRuleCreateOptions(tool) {
    if (!tool) return [];
    const options = (tool.capabilities || [])
      .filter((/** @type {any} */ capability) => isToolRuleCreatableCapability(tool, capability))
      .map((/** @type {any} */ capability) => {
        const projection = projectCapability(tool?.id || "", "rules", capability);
        const path = stripCapabilityPath(projection?.evidence?.source_path || capability.source_path);
        if (!path || path.includes("*")) return null;
        const isDirectory = capability.format === "directory" || looksLikeDirectory(path);
        const filename = baseName(path);
        if (!isDirectory && toolRulePathKnownForTool(tool, path)) return null;
        return {
          id: `${capability.id || path}::${path}`,
          kind: isDirectory ? "directory" : "file",
          label: capability.label || filename || $t("rules.tool.create_rule_file"),
          path,
          filename: isDirectory ? "" : filename,
          fileSuffix: ruleFileDefaultSuffix(capability),
        };
      })
      .filter(Boolean)
      .sort((/** @type {any} */ a, /** @type {any} */ b) => {
        if (a.kind !== b.kind) return a.kind === "file" ? -1 : 1;
        return String(a.path || "").localeCompare(String(b.path || ""));
      });
    return options;
  }

  let toolRuleCreateOptions = $derived.by(() => getToolRuleCreateOptions(currentTool));
  let officialGlobalRuleFileMissing = $derived.by(() => {
    if (!currentTool) return false;
    if (userConfiguredGlobalRuleTargetPath(currentTool)) return false;
    const certifiedTarget = stripCapabilityPath(getCertifiedGlobalRuleTarget(currentTool));
    return Boolean(certifiedTarget) && !toolRulePathKnownForTool(currentTool, certifiedTarget);
  });
  let currentToolRulePromptDecision = $derived.by(() => getRulePromptDecision(currentTool, {
    context: "tool_rules",
    hasDisplayableToolRules: Boolean(activeToolRule || toolRuleTreeRows.length > 0),
    hasToolRuleSourceRoot,
    hasToolRuleCreateOptions: toolRuleCreateOptions.length > 0,
    hasMissingGlobalRuleFile: officialGlobalRuleFileMissing,
  }));
  /** @param {any} decision */
  function rulePromptText(decision) {
    if (!decision || decision.kind === "none") return "";
    const key = `rules.tool.prompt_${decision.reason}`;
    const label = $t(key);
    return label === key ? $t("rules.tool.prompt_no_rule_source") : label;
  }
  let ruleCreateToolTargets = $derived.by(() => {
    return ($managedTools || [])
      .map((/** @type {any} */ tool) => ({ tool, options: getToolRuleCreateOptions(tool) }))
      .filter((/** @type {any} */ entry) => entry.options.length > 0)
      .sort((a, b) => String(a.tool?.name || a.tool?.id || "").localeCompare(String(b.tool?.name || b.tool?.id || "")));
  });
  let toolRuleEmptyDescription = $derived.by(() => {
    return rulePromptText(currentToolRulePromptDecision);
  });
  let selectedRuleCreateToolTarget = $derived(
    ruleCreateToolTargets.find((/** @type {any} */ entry) => entry.tool?.id === ruleCreateDialogToolId) || ruleCreateToolTargets[0] || null
  );
  let ruleCreateLocationOptions = $derived(selectedRuleCreateToolTarget?.options || []);
  let selectedToolRuleCreateOption = $derived(
    ruleCreateLocationOptions.find((/** @type {any} */ option) => option.id === ruleCreateDialogLocationId) || ruleCreateLocationOptions[0] || null
  );
  let ruleCreateNamePlaceholder = $derived(ruleFilePlaceholderForSuffix(selectedToolRuleCreateOption?.fileSuffix || ".md"));
  /** @param {any} option */
  function toolRuleCreateLocationLabel(option) {
    return option?.path || "";
  }
  let ruleCreateResolvedFileName = $derived.by(() => {
    if (!selectedToolRuleCreateOption) return "";
    return selectedToolRuleCreateOption.kind === "directory"
      ? cleanRuleFileNameForFormat(ruleCreateDialogFileName, selectedToolRuleCreateOption.fileSuffix)
      : cleanRuleFileNameForFormat(selectedToolRuleCreateOption.filename || baseName(selectedToolRuleCreateOption.path), selectedToolRuleCreateOption.fileSuffix);
  });
  let ruleCreateResolvedChildDir = $derived(
    ruleCreateDialogUseChildDir ? cleanRuleChildDirectoryName(ruleCreateDialogChildDirName) : ""
  );
  let ruleCreateDraftPath = $derived.by(() => {
    if (!selectedToolRuleCreateOption || !ruleCreateResolvedFileName) return "";
    if (selectedToolRuleCreateOption.kind !== "directory") return selectedToolRuleCreateOption.path;
    const parent = ruleCreateDialogUseChildDir
      ? joinRulePath(selectedToolRuleCreateOption.path, ruleCreateResolvedChildDir)
      : selectedToolRuleCreateOption.path;
    if (ruleCreateDialogUseChildDir && !ruleCreateResolvedChildDir) return "";
    return joinRulePath(parent, ruleCreateResolvedFileName);
  });
  let ruleCreateTargetExists = $derived.by(() => {
    if (!selectedRuleCreateToolTarget?.tool || !ruleCreateDraftPath) return false;
    return toolRulePathKnownForTool(selectedRuleCreateToolTarget.tool, ruleCreateDraftPath, { caseInsensitive: true });
  });
  let ruleCreateValidationMessage = $derived.by(() => {
    if (!selectedRuleCreateToolTarget?.tool) return "";
    if (!selectedToolRuleCreateOption) return $t("rules.tool.create_dialog_no_location");
    if (selectedToolRuleCreateOption.kind === "directory" && !ruleCreateResolvedFileName) return $t("rules.tool.create_dialog_file_required");
    if (ruleCreateDialogUseChildDir && !ruleCreateResolvedChildDir) return $t("rules.tool.create_dialog_folder_required");
    if (ruleCreateTargetExists) return $t("rules.tool.create_dialog_file_exists");
    return "";
  });
  let canCreateSelectedToolRuleDraft = $derived(Boolean(ruleCreateDraftPath) && !ruleCreateValidationMessage);

  function clearToolRuleCreateMsgTimer() {
    if (!toolRuleCreateMsgTimer) return;
    clearTimeout(toolRuleCreateMsgTimer);
    toolRuleCreateMsgTimer = null;
  }

  /** @param {string} msg @param {{ autoDismiss?: boolean }} [options] */
  function showToolRuleCreateMsg(msg, options = {}) {
    clearToolRuleCreateMsgTimer();
    toolRuleCreateMsg = msg;
    if (!msg || !options.autoDismiss) return;
    const expectedMsg = msg;
    toolRuleCreateMsgTimer = setTimeout(() => {
      if (toolRuleCreateMsg === expectedMsg) {
        toolRuleCreateMsg = "";
      }
      toolRuleCreateMsgTimer = null;
    }, TOOL_RULE_FEEDBACK_TIMEOUT_MS);
  }

  onDestroy(() => {
    clearToolRuleCreateMsgTimer();
  });

  $effect(() => {
    if (rulesTab !== "tool") return;
    const hasCurrent = currentToolId && $managedTools.some((/** @type {any} */ tool) => tool.id === currentToolId);
    if (!hasCurrent && $managedTools[0]) {
      activeToolId.set($managedTools[0].id);
    }
  });

  $effect(() => {
    const key = `${rulesTab}:${currentTool?.id || ""}`;
    if (key === toolRuleContextKey) return;
    toolRuleContextKey = key;
    if (!(rulesTab === "tool" && selectedToolRule?.isNewlyCreated && selectedToolRule?.toolId === currentTool?.id)) {
      if (activeToolRule?.isNewlyCreated) {
        activeToolRule.isNewlyCreated = false;
      }
      selectedToolRule = null;
      showToolRuleCreateMsg("");
    }
    ruleCreateDialogOpen = false;
  });

  $effect(() => {
    if (rulesTab !== "tool" || !currentTool?.id || ruleCreateDialogOpen || activeToolRule?.isNewlyCreated) return;
    if (readableToolRules.length === 0) {
      if (selectedToolRule && !selectedToolRule.isNewlyCreated) selectedToolRule = null;
      return;
    }
    if (selectedToolRule && readableToolRules.some((/** @type {any} */ rule) => rule.path === selectedToolRule.path)) return;
    const rememberedPath = selectedToolRulePathByTool[currentTool.id];
    const nextRule = readableToolRules.find((/** @type {any} */ rule) => rule.path === rememberedPath) || readableToolRules[0];
    if (!nextRule) return;
    const key = `${currentTool.id}:${globalSearchQuery.trim()}:${nextRule.path || ""}`;
    if (key === autoSelectToolRuleKey) return;
    autoSelectToolRuleKey = key;
    openToolRuleDetail(nextRule).catch(() => {});
  });

  $effect(() => {
    const key = activeToolRule ? `${currentTool?.id || ""}:${activeToolRule.path || ""}` : "";
    if (key === singleToolRuleKey) return;
    singleToolRuleKey = key;
    singleToolRuleContent = activeToolRule?.content || "";
    singleToolRuleOriginal = activeToolRule?.content || "";
    singleToolRuleMsg = "";
    singleToolRuleMdPreview = true;
    isSingleToolRuleEditing = false;
  });

  function cancelSingleToolRuleEdit() {
    isSingleToolRuleEditing = false;
    singleToolRuleContent = singleToolRuleOriginal;
    singleToolRuleMsg = "";
  }

  async function saveSingleToolRule() {
    if (!activeToolRule?.path || !currentTool?.id) return;
    singleToolRuleSaving = true;
    singleToolRuleMsg = "";
    const wasNewlyCreated = Boolean(activeToolRule.isNewlyCreated);
    const savedToolId = currentTool.id;
    const savedRulePath = activeToolRule.path;
    try {
      await writeRule(currentTool.id, activeToolRule.path, singleToolRuleContent);
      activeToolRule.content = singleToolRuleContent;
      activeToolRule.isNewlyCreated = false;
      singleToolRuleOriginal = singleToolRuleContent;
      isSingleToolRuleEditing = false;
      const refreshedTools = await loadTools();
      if (wasNewlyCreated) {
        const refreshedTool = Array.isArray(refreshedTools)
          ? refreshedTools.find((/** @type {any} */ tool) => tool.id === savedToolId)
          : null;
        const createdRule = (refreshedTool?.rule_sources || []).find((/** @type {any} */ rule) => rule.path === savedRulePath);
        if (createdRule) {
          selectedToolRule = { ...createdRule, content: createdRule.content ?? singleToolRuleContent };
          selectedToolRulePathByTool = { ...selectedToolRulePathByTool, [savedToolId]: savedRulePath };
        } else if ((refreshedTool?.rule_sources || []).length > 1) {
          selectedToolRule = null;
        }
      }
      await logAppEvent({
        level: "info",
        category: "rules",
        action: "tool_rule_save",
        result: "ok",
        toolId: currentTool.id,
        targetRole: "tool_rule",
        targetPath: savedRulePath,
      });
    } catch (e) {
      singleToolRuleMsg = String(e);
      await logAppEvent({
        level: "error",
        category: "rules",
        action: "tool_rule_save",
        result: "failed",
        toolId: currentTool.id,
        targetRole: "tool_rule",
        targetPath: savedRulePath,
        error: String(e),
      });
    } finally {
      singleToolRuleSaving = false;
    }
  }

  $effect(() => {
    if (!ruleCreateDialogOpen) return;
    if (!selectedRuleCreateToolTarget?.tool) {
      ruleCreateDialogToolId = "";
      ruleCreateDialogLocationId = "";
      return;
    }
    if (ruleCreateDialogToolId !== selectedRuleCreateToolTarget.tool.id) {
      ruleCreateDialogToolId = selectedRuleCreateToolTarget.tool.id;
    }
    if (!selectedToolRuleCreateOption && ruleCreateLocationOptions[0]) {
      selectToolRuleCreateOption(ruleCreateLocationOptions[0].id);
    }
  });

  function resetRuleCreateFileFields(/** @type {any} */ option = selectedToolRuleCreateOption) {
    ruleCreateDialogFileName = option?.kind === "directory" ? "" : (option?.filename || baseName(option?.path || ""));
    ruleCreateDialogUseChildDir = false;
    ruleCreateDialogChildDirName = "";
  }

  function openToolRuleCreate() {
    if (rulesTab !== "tool" || ruleCreateToolTargets.length === 0) return;
    showToolRuleCreateMsg("");
    selectedToolRule = null;
    isSingleToolRuleEditing = false;
    const currentTarget = currentTool?.id
      ? ruleCreateToolTargets.find((/** @type {any} */ entry) => entry.tool?.id === currentTool.id)
      : null;
    const target = currentTarget || ruleCreateToolTargets[0];
    ruleCreateDialogToolId = target?.tool?.id || "";
    const option = target?.options?.[0] || null;
    ruleCreateDialogLocationId = option?.id || "";
    resetRuleCreateFileFields(option);
    ruleCreateDialogOpen = true;
  }

  function closeRuleCreateDialog() {
    ruleCreateDialogOpen = false;
  }

  /** @param {string} toolId */
  function selectRuleCreateTool(toolId) {
    const target = ruleCreateToolTargets.find((/** @type {any} */ entry) => entry.tool?.id === toolId) || ruleCreateToolTargets[0] || null;
    ruleCreateDialogToolId = target?.tool?.id || "";
    const option = target?.options?.[0] || null;
    ruleCreateDialogLocationId = option?.id || "";
    resetRuleCreateFileFields(option);
  }

  /** @param {string} optionId */
  function selectToolRuleCreateOption(optionId) {
    const option = ruleCreateLocationOptions.find((/** @type {any} */ item) => item.id === optionId) || ruleCreateLocationOptions[0] || null;
    ruleCreateDialogLocationId = option?.id || "";
    resetRuleCreateFileFields(option);
  }

  async function createSelectedToolRule() {
    if (!selectedRuleCreateToolTarget?.tool || !selectedToolRuleCreateOption || !canCreateSelectedToolRuleDraft) return;
    await createToolRuleFromOption(selectedRuleCreateToolTarget.tool, selectedToolRuleCreateOption, ruleCreateDraftPath);
  }

  /** @param {any} tool @param {{ kind: string, path: string, filename?: string }} option @param {string} path */
  async function createToolRuleFromOption(tool, option, path) {
    if (!tool?.id || !option || !path) return;
    showToolRuleCreateMsg("");
    try {
      await createRuleFile(tool.id, path, "");
      await loadTools();
      selectedToolRule = {
        label: baseName(path),
        path,
        group: option.kind === "directory" ? ruleGroupFromCreateRoot(option.path, path) : "",
        content: "",
        isNewlyCreated: true,
        toolId: tool.id,
      };
      selectedToolRulePathByTool = { ...selectedToolRulePathByTool, [tool.id]: path };
      singleToolRuleContent = "";
      singleToolRuleOriginal = "";
      singleToolRuleMsg = "";
      ruleCreateDialogOpen = false;
      setRulesTab("tool");
      activeToolId.set(tool.id);
      void tick().then(() => {
        isSingleToolRuleEditing = true;
      });
    } catch (e) {
      showToolRuleCreateMsg($t("rules.tool.context_action_failed", { err: e }));
    }
  }

  /** @param {any} item */
  function toggleToolRuleNavigationItem(item) {
    const id = item?.raw?.id || item?.path || "";
    if (id) toggleGroup(id);
  }

  /** @param {any} item */
  function selectToolRuleNavigationItem(item) {
    if (item?.rule?.isNewlyCreated) return;
    if (item?.rule) openToolRuleDetail(item.rule).catch(() => {});
  }

  /** @param {MouseEvent} event */
  function contextMenuPosition(event) {
    const width = 190;
    const height = 180;
    return {
      x: Math.min(event.clientX, Math.max(8, window.innerWidth - width - 8)),
      y: Math.min(event.clientY, Math.max(8, window.innerHeight - height - 8)),
    };
  }

  /** @param {any} item @param {MouseEvent} event */
  function openToolRuleContextMenu(item, event) {
    if (item?.rule?.isNewlyCreated) return false;
    if (!item || isSingleToolRuleEditing || ruleTreeActionBusy) return false;
    const actions = [];
    if (item.kind !== "file" && canCreateInToolRuleDirectory(item)) {
      actions.push("new_file", "new_directory");
    }
    if (canSetToolRuleItemAsGlobalRuleFile(item)) actions.push("set_global_rule_file");
    if (canRenameToolRuleItem(item)) actions.push("rename");
    if (canDeleteToolRuleItem(item)) actions.push("delete");
    if (actions.length === 0) return false;
    window.dispatchEvent(new CustomEvent(APP_CONTEXT_MENU_OPEN_EVENT, { detail: { source: "rules" } }));
    ruleTreeContextMenu = {
      item,
      actions,
      ...contextMenuPosition(event),
    };
    return true;
  }

  function closeToolRuleContextMenu() {
    ruleTreeContextMenu = null;
  }

  $effect(() => {
    if (!ruleTreeContextMenu) return;
    const close = () => closeToolRuleContextMenu();
    const onKeydown = (/** @type {KeyboardEvent} */ event) => {
      if (event.key === "Escape") closeToolRuleContextMenu();
    };
    window.addEventListener("mousedown", close);
    window.addEventListener("scroll", close);
    window.addEventListener("keydown", onKeydown, true);
    window.addEventListener(APP_CONTEXT_MENU_OPEN_EVENT, close);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("scroll", close);
      window.removeEventListener("keydown", onKeydown, true);
      window.removeEventListener(APP_CONTEXT_MENU_OPEN_EVENT, close);
    };
  });

  /** @param {any} item @param {"new_file" | "new_directory" | "rename"} mode */
  function openRuleTreeNameDialog(item, mode) {
    closeToolRuleContextMenu();
    if (!item) return;
    const isFile = item.kind === "file";
    const currentPath = isFile ? item.rule?.path : item.raw?.path;
    const parentPath = isFile ? dirName(currentPath) : currentPath;
    const initialName = mode === "rename" ? baseName(currentPath) : "";
    ruleTreeNameDialog = { item, mode, currentPath, parentPath, initialName };
    ruleTreeNameDialogValue = initialName;
  }

  function closeRuleTreeNameDialog() {
    if (ruleTreeActionBusy) return;
    ruleTreeNameDialog = null;
    ruleTreeNameDialogValue = "";
  }

  /** @param {any} item */
  async function setToolRuleItemAsGlobalRuleFile(item) {
    closeToolRuleContextMenu();
    if (!currentTool?.id || !item?.rule?.path || !canSetToolRuleItemAsGlobalRuleFile(item) || ruleTreeActionBusy) return;
    const toolId = currentTool.id;
    const targetPath = item.rule.path;
    const customGlobalRuleTarget = isCertifiedGlobalRuleTargetPath(targetPath) ? null : targetPath;
    ruleTreeActionBusy = true;
    showToolRuleCreateMsg("");
    try {
      await setToolCapabilityOverrides(toolId, {
        customGlobalRuleTarget,
        sharedSkillDirectRead: sharedSkillDirectReadOverrideValue(currentTool),
      });
      await loadTools();
      await refreshManagedRulesState();
      selectedToolRulePathByTool = { ...selectedToolRulePathByTool, [toolId]: targetPath };
      showToolRuleCreateMsg($t("rules.tool.context_set_global_rule_file_ok"), { autoDismiss: true });
      await logAppEvent({
        level: "info",
        category: "rules",
        action: "tool_rule_set_global_rule_file",
        result: "ok",
        toolId,
        targetRole: "global_rule_target",
        targetPath,
      });
    } catch (e) {
      showToolRuleCreateMsg($t("rules.tool.context_action_failed", { err: e }));
      await logAppEvent({
        level: "error",
        category: "rules",
        action: "tool_rule_set_global_rule_file",
        result: "failed",
        toolId,
        targetRole: "global_rule_target",
        targetPath,
        error: String(e),
      });
    } finally {
      ruleTreeActionBusy = false;
    }
  }

  let ruleTreeNameDialogCleanName = $derived.by(() => {
    if (!ruleTreeNameDialog) return "";
    if (ruleTreeNameDialog.mode === "new_directory") return cleanRuleChildDirectoryName(ruleTreeNameDialogValue);
    if (ruleTreeNameDialog.mode === "rename" && ruleTreeNameDialog.item?.kind !== "file") {
      return cleanRuleChildDirectoryName(ruleTreeNameDialogValue);
    }
    if (ruleTreeNameDialog.mode === "rename") {
      const suffix = ruleFileSuffixForPath(ruleTreeNameDialog.currentPath);
      return suffix === ".instructions.md"
        ? cleanRuleFileNameForFormat(ruleTreeNameDialogValue, suffix)
        : cleanRuleRenameFileName(ruleTreeNameDialogValue, suffix);
    }
    return cleanRuleFileNameForFormat(
      ruleTreeNameDialogValue,
      ruleFileDefaultSuffixForPath(currentTool, ruleTreeNameDialog.parentPath),
    );
  });
  let ruleTreeNameDialogFilePlaceholder = $derived.by(() => {
    if (!ruleTreeNameDialog) return $t("rules.tool.create_name_placeholder");
    if (ruleTreeNameDialog.mode === "rename" && ruleTreeNameDialog.item?.kind === "file") {
      return ruleFilePlaceholderForSuffix(ruleFileSuffixForPath(ruleTreeNameDialog.currentPath));
    }
    if (ruleTreeNameDialog.mode === "new_file") {
      return ruleFilePlaceholderForSuffix(ruleFileDefaultSuffixForPath(currentTool, ruleTreeNameDialog.parentPath));
    }
    return $t("rules.tool.create_name_placeholder");
  });
  let ruleTreeNameDialogTargetPath = $derived.by(() => {
    if (!ruleTreeNameDialog || !ruleTreeNameDialogCleanName) return "";
    const parent = ruleTreeNameDialog.mode === "rename"
      ? dirName(ruleTreeNameDialog.currentPath)
      : ruleTreeNameDialog.parentPath;
    return joinRulePath(parent, ruleTreeNameDialogCleanName);
  });
  let ruleTreeNameDialogTargetExists = $derived.by(() => {
    if (!currentTool || !ruleTreeNameDialog || ruleTreeNameDialog.mode !== "new_file" || !ruleTreeNameDialogTargetPath) return false;
    return toolRulePathKnownForTool(currentTool, ruleTreeNameDialogTargetPath, { caseInsensitive: true });
  });
  let ruleTreeNameDialogInvalid = $derived.by(() => {
    if (!ruleTreeNameDialog) return "";
    if (!ruleTreeNameDialogCleanName) return $t("rules.tool.context_name_required");
    if (ruleTreeNameDialogTargetExists) return $t("rules.tool.create_dialog_file_exists");
    if (ruleTreeNameDialog.mode === "rename" && ruleTreeNameDialogCleanName === ruleTreeNameDialog.initialName) {
      return $t("rules.tool.context_name_unchanged");
    }
    return "";
  });
  let canConfirmRuleTreeNameDialog = $derived(Boolean(ruleTreeNameDialog && ruleTreeNameDialogTargetPath && !ruleTreeNameDialogInvalid && !ruleTreeActionBusy));

  /** @param {string} toolId @param {string} id @param {string} path */
  function rememberLocalToolRuleDirectory(toolId, id, path) {
    if (!toolId || !id || !path) return;
    const current = localToolRuleDirectoriesByTool[toolId] || [];
    const next = current.filter((entry) => entry.id !== id && entry.path !== path);
    next.push({ id, path });
    localToolRuleDirectoriesByTool = { ...localToolRuleDirectoriesByTool, [toolId]: next };
  }

  /** @param {string} toolId @param {string} path */
  function forgetLocalToolRuleDirectory(toolId, path) {
    if (!toolId || !path) return;
    const target = withoutTrailingSlash(path);
    const current = localToolRuleDirectoriesByTool[toolId] || [];
    const next = current.filter((entry) => {
      const entryPath = withoutTrailingSlash(entry.path);
      return entryPath !== target && !entryPath.startsWith(`${target}/`);
    });
    localToolRuleDirectoriesByTool = { ...localToolRuleDirectoriesByTool, [toolId]: next };
  }

  /** @param {string} source @param {string} oldPrefix @param {string} newPrefix */
  function replacePathPrefix(source, oldPrefix, newPrefix) {
    const cleanSource = withoutTrailingSlash(source);
    const cleanOld = withoutTrailingSlash(oldPrefix);
    const cleanNew = withoutTrailingSlash(newPrefix);
    if (cleanSource === cleanOld) return cleanNew;
    if (cleanSource.startsWith(`${cleanOld}/`)) return `${cleanNew}${cleanSource.slice(cleanOld.length)}`;
    return source;
  }

  async function confirmRuleTreeNameDialog() {
    if (!canConfirmRuleTreeNameDialog || !currentTool?.id || !ruleTreeNameDialog) return;
    ruleTreeActionBusy = true;
    const dialog = ruleTreeNameDialog;
    const targetPath = ruleTreeNameDialogTargetPath;
    try {
      if (dialog.mode === "new_file") {
        await createRuleFile(currentTool.id, targetPath, "");
        await loadTools();
        selectedToolRule = {
          label: baseName(targetPath),
          path: targetPath,
          group: dialog.item?.raw?.id || "",
          content: "",
          isNewlyCreated: true,
          toolId: currentTool.id,
        };
        selectedToolRulePathByTool = { ...selectedToolRulePathByTool, [currentTool.id]: targetPath };
        singleToolRuleContent = "";
        singleToolRuleOriginal = "";
        singleToolRuleMsg = "";
        ruleTreeNameDialog = null;
        void tick().then(() => {
          isSingleToolRuleEditing = true;
        });
        return;
      }
      if (dialog.mode === "new_directory") {
        await createRuleDirectory(currentTool.id, targetPath);
        const baseId = dialog.item?.raw?.id || "";
        const directoryId = baseId ? `${baseId}/${ruleTreeNameDialogCleanName}` : ruleTreeNameDialogCleanName;
        rememberLocalToolRuleDirectory(currentTool.id, directoryId, targetPath);
        await loadTools();
        showToolRuleCreateMsg($t("rules.tool.context_directory_created"), { autoDismiss: true });
        ruleTreeNameDialog = null;
        return;
      }
      const renamedPath = await renameRuleEntry(currentTool.id, dialog.currentPath, ruleTreeNameDialogCleanName);
      const oldPath = dialog.currentPath;
      const newPath = String(renamedPath || targetPath);
      if (dialog.item?.kind === "file") {
        selectedToolRulePathByTool = { ...selectedToolRulePathByTool, [currentTool.id]: newPath };
        if (activeToolRule?.path === oldPath) {
          selectedToolRule = { ...activeToolRule, path: newPath, label: baseName(newPath) };
        }
      } else if (activeToolRule?.path && withoutTrailingSlash(activeToolRule.path).startsWith(`${withoutTrailingSlash(oldPath)}/`)) {
        selectedToolRulePathByTool = {
          ...selectedToolRulePathByTool,
          [currentTool.id]: replacePathPrefix(activeToolRule.path, oldPath, newPath),
        };
        selectedToolRule = null;
      }
      forgetLocalToolRuleDirectory(currentTool.id, oldPath);
      await loadTools();
      showToolRuleCreateMsg($t("rules.tool.context_renamed"), { autoDismiss: true });
      ruleTreeNameDialog = null;
    } catch (e) {
      showToolRuleCreateMsg($t("rules.tool.context_action_failed", { err: e }));
    } finally {
      ruleTreeActionBusy = false;
    }
  }

  /** @param {any} item */
  async function deleteToolRuleTreeItem(item) {
    closeToolRuleContextMenu();
    if (!currentTool?.id || !item || !ruleFileConfirmDialog || ruleTreeActionBusy) return;
    const path = item.kind === "file" ? item.rule?.path : item.raw?.path;
    if (!path) return;
    ruleTreeActionBusy = true;
    try {
      const preview = await deleteRuleEntry(currentTool.id, path, true);
      const ok = await ruleFileConfirmDialog.show({
        title: item.kind === "file" ? $t("rules.tool.context_delete_file_title") : $t("rules.tool.context_delete_directory_title"),
        preview,
        variant: "danger",
        confirmLabel: $t("rules.tool.context_delete_confirm"),
      });
      if (!ok) return;
      await deleteRuleEntry(currentTool.id, path, false);
      const cleanPath = withoutTrailingSlash(path);
      if (activeToolRule?.path) {
        const selectedPath = withoutTrailingSlash(activeToolRule.path);
        if (selectedPath === cleanPath || selectedPath.startsWith(`${cleanPath}/`)) {
          selectedToolRule = null;
          const nextSelected = { ...selectedToolRulePathByTool };
          delete nextSelected[currentTool.id];
          selectedToolRulePathByTool = nextSelected;
        }
      }
      forgetLocalToolRuleDirectory(currentTool.id, path);
      await loadTools();
      showToolRuleCreateMsg($t("rules.tool.context_deleted"), { autoDismiss: true });
    } catch (e) {
      showToolRuleCreateMsg($t("rules.tool.context_action_failed", { err: e }));
    } finally {
      ruleTreeActionBusy = false;
    }
  }

  // === Global Rules ===
  /** @param {string} reason @param {string} [view] */
  function startRulesRun(reason, view = rulesTab) {
    const run = beginModulePerformanceRun({
      module: "rules",
      view,
      reason,
      counters: {
        defaultRules: defaultRulesList.length,
        toolRules: readableToolRules.length,
        managedTargets: managedRulesState?.summary?.managed_targets || 0,
      },
    });
    markModulePerformance(run, "rules-visible", { role: MODULE_PERFORMANCE_ROLE_VISIBLE });
    updateModulePerformanceSummary(run);
    return run;
  }

  /** @param {"global" | "custom" | "tool"} tab */
  function setRulesTab(tab) {
    const nextTab = normalizeRulesTab(tab);
    const changed = nextTab !== rulesTab;
    const run = changed ? startRulesRun("tab-switch", nextTab) : null;
    closeRuleEditor();
    if (nextTab !== "tool") closeToolRuleDetail();
    if (changed) {
      searchOpen = false;
      globalSearchQuery = "";
      toolRuleSearchResultsOpen = false;
    }
    rulesTab = nextTab;
    activeRulesTab.set(nextTab);
    if (run) {
      markModulePerformance(run, "rules-tab-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
      markModulePerformance(run, "rules-tab-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "success");
    }
  }

  // Auto-switch sub-tab when navigating from dashboard
  $effect(() => {
    if ($activeRulesTab === "custom") {
      setRulesTab("global");
    }
  });

  $effect(() => {
    if ($pendingSubTab === "tool") {
      setRulesTab("tool");
      pendingSubTab.set(null);
    } else if ($pendingSubTab === "default") {
      setRulesTab("global");
      pendingSubTab.set(null);
    }
  });
  let commonRuleContent = $state("");
  let commonRuleOriginal = $state("");
  let commonRuleSaving = $state(false);
  let commonRuleMsg = $state("");
  /** @type {any[]} */
  let defaultRulesList = $state([]);
  const visualCopyRule = {
    label: "AGENTS.md",
    path: "/Users/visual/workspace/modus/AGENTS.md",
    content: "# Visual copy rule\n\nKeep copied rule actions visually consistent.",
  };
  let showRuleEditor = $state(false);
  /** @type {{ id: string, name: string, content: string, inject_to?: string[] } | null} */
  let editingDefaultRule = $state(null);
  let ruleEditorName = $state("");
  let ruleEditorContent = $state("");
  /** @type {string[]} */
  let ruleEditorTargets = $state([]);
  /** 注入目标为显式选中的工具 id；全选时与「全选」按钮联动，保存时全选映射为 inject_to: [] */
  /** 与工具规则弹窗一致：true = 阅读（Markdown 预览），false = 源码 */
  let ruleEditorMdPreview = $state(true);
  let commonRuleMdPreview = $state(true);
  let isCommonRuleEditing = $state(false);
  let commonRuleEditSnapshot = $state("");
  let isCommonRuleDirty = $derived(String(commonRuleContent ?? "") !== String(commonRuleOriginal ?? ""));
  let commonRuleLineCount = $derived(String(commonRuleContent ?? "").split("\n").length);
  let commonRuleSearchMatchIndex = $state(0);
  let commonRuleSearchVersion = $state(0);
  let commonRuleSearchKey = $state("");
  let commonRuleSearchMatchCount = $derived(
    countCurrentContentMatches(commonRuleContent, commonRuleExternalSearchQuery)
  );

  $effect(() => {
    const key = `global\u0000${commonRuleExternalSearchQuery}\u0000${String(commonRuleContent ?? "")}`;
    if (!commonRuleExternalSearchQuery) {
      commonRuleSearchKey = "";
      commonRuleSearchMatchIndex = 0;
      return;
    }
    if (key === commonRuleSearchKey) return;
    commonRuleSearchKey = key;
    commonRuleSearchMatchIndex = 0;
    if (commonRuleSearchMatchCount > 0) commonRuleSearchVersion += 1;
  });

  let isCommonRuleMarkdown = $derived(true); // 全局规则始终视为 Markdown

  function cancelCommonRuleEdit() {
    isCommonRuleEditing = false;
    commonRuleContent = commonRuleEditSnapshot;
  }
  let ruleEditorSaving = $state(false);
  /** false = 只读（切换源码/阅读 + 编辑）；true = 编辑中（取消 + 保存） */
  let isCrmContentEditing = $state(false);
  let crmEditSnapshot = $state(/** @type {{ name: string, content: string, targets: string[] }} */ ({ name: "", content: "", targets: [] }));
  let ruleEditorLineCount = $derived(ruleEditorContent ? ruleEditorContent.split("\n").length : 0);
  /** @param {string[]} left @param {string[]} right */
  function sameStringSet(left, right) {
    const a = [...new Set((left || []).map(String))].sort();
    const b = [...new Set((right || []).map(String))].sort();
    return a.length === b.length && a.every((value, index) => value === b[index]);
  }
  let isRuleEditorContentDirty = $derived(
    String(ruleEditorContent ?? "") !== String(crmEditSnapshot.content ?? "")
      || String(ruleEditorName ?? "") !== String(crmEditSnapshot.name ?? "")
      || !sameStringSet(ruleEditorTargets, crmEditSnapshot.targets)
  );
  let isCreatingCustomRule = $derived(showRuleEditor && !editingDefaultRule);
  /** 已选数等于当前工具数即视为全选 */
  let ruleEditorAllTargetsSelected = $derived.by(() => {
    const tools = $managedTools;
    if (tools.length === 0) return false;
    const set = new Set(ruleEditorTargets);
    return tools.every((/** @type {any} */ t) => set.has(t.id));
  });
  /** 仅在用户未实际改动目标时，才保留当前不可见的历史目标 */
  let ruleEditorTargetsTouched = $state(false);
  let canSaveRuleEditor = $derived.by(() => {
    if (ruleEditorSaving || !ruleEditorName.trim()) return false;
    return getRuleEditorInjectToForSave().ok;
  });
  let customRuleMsg = $state("");
  let rulesInjectStatusMsg = $state("");
  let linkedPendingBannerDismissed = $state(false);
  let showRulesInjectPreview = $state(false);
  let rulesInjectPreviewBusy = $state(false);
  let rulesInjectPreview = $state({
    changes: /** @type {Array<{ toolId: string, toolName: string, path: string, willCreate?: boolean }>} */ ([]),
    exceptions: /** @type {Array<{ toolId: string, toolName: string, path: string, reason: string, dismissKey?: string }>} */ ([]),
    feedbackTarget: /** @type {"global" | "custom"} */ ("global"),
  });
  /** @type {ReturnType<typeof setTimeout> | null} */
  let rulesInjectStatusTimer = null;
  let defaultRuleBaselines = $state({
    common_rule: "",
    custom_rules: /** @type {Record<string,string>} */ ({}),
    custom_rule_pending_targets: /** @type {Record<string,string[]>} */ ({}),
  });
  /** @type {any} */
  let managedRulesState = $state(null);
  let managedRulesBusy = $state(false);
  let managedRulesMsg = $state("");
  let ruleIssueDetailsOpen = $state(false);
  let dismissedRuleIssueTargetKeys = $state(new Set());
  let ruleIssueWasVisible = $state(false);
  let ruleIssueExpandedTargets = $state(new Set());
  let ruleIssueDiffs = $state(/** @type {Record<string, { status: "loading" | "loaded" | "failed", diff: any }>} */ ({}));
  let ruleIssueDiffRevision = $state(0);
  /** @type {any} */
  let inspectManagedTarget = $state(null);
  /** @type {any} */
  let inspectManagedDiff = $state(null);
  /** @type {any} */
  let leaveManagedContext = $state(null);
  let leaveManagedBusy = $state(false);

  let ruleIssueSourceKey = $derived.by(() => {
    const rule = defaultRulesList.find((/** @type {any} */ item) => item.id === "common_rule");
    return rule ? defaultRuleFingerprint(rule) : "";
  });

  /**
   * @param {string} toolId
   * @param {string} reason
   * @param {string | null | undefined} path
   */
  function ruleIssueDismissalKey(toolId, reason, path) {
    return [ruleIssueSourceKey, String(toolId || ""), String(reason || ""), String(path || "")].join("::");
  }

  /** @param {any} target */
  function isDismissedRuleIssueTarget(target) {
    return target?.classification === "unresolved"
      && dismissedRuleIssueTargetKeys.has(ruleIssueDismissalKey(target?.tool_id, target?.reason, target?.target_path));
  }

  let managedAffectedTargets = $derived.by(() => {
    const targets = managedRulesState?.targets || [];
    return targets.filter((/** @type {any} */ target) =>
      ["requires_sync", "drifted", "unresolved"].includes(target.classification)
      && !isDismissedRuleIssueTarget(target)
    );
  });
  let managedStateVisible = $derived.by(() => managedAffectedTargets.length > 0);

  /** @param {any} target */
  function ruleIssueTargetKey(target) {
    return String(target?.tool_id || "");
  }

  /** @param {any} target */
  function isRuleIssueTargetExpanded(target) {
    return ruleIssueExpandedTargets.has(ruleIssueTargetKey(target));
  }

  /** @param {any} target */
  function getRuleIssueDiffEntry(target) {
    return ruleIssueDiffs[ruleIssueTargetKey(target)];
  }

  /** @param {any[]} changes */
  function compactRuleIssueDiffLines(changes) {
    const lines = Array.isArray(changes) ? changes : [];
    const changedIndexes = lines
      .map((/** @type {any} */ line, index) => line?.tag !== "equal" ? index : -1)
      .filter((index) => index >= 0);
    if (changedIndexes.length === 0) return lines;

    const context = 2;
    const visible = new Set();
    for (const index of changedIndexes) {
      const start = Math.max(0, index - context);
      const end = Math.min(lines.length - 1, index + context);
      for (let i = start; i <= end; i += 1) visible.add(i);
    }

    const compacted = [];
    let previousVisible = -1;
    for (let i = 0; i < lines.length; i += 1) {
      if (!visible.has(i)) continue;
      if (previousVisible !== -1 && i - previousVisible > 1) {
        compacted.push({ tag: "fold", content: "..." });
      } else if (previousVisible === -1 && i > 0) {
        compacted.push({ tag: "fold", content: "..." });
      }
      compacted.push(lines[i]);
      previousVisible = i;
    }
    if (previousVisible !== -1 && previousVisible < lines.length - 1) {
      compacted.push({ tag: "fold", content: "..." });
    }
    return compacted;
  }

  /** @param {any} target */
  function shouldShowRuleIssueDiff(target) {
    return ["managed_block_drift", "missing_managed_block", "stale_managed_block", "pending_source"].includes(target?.reason)
      && Boolean(target?.expected_block || target?.current_block);
  }

  /** @param {any} target */
  function isRuleIssueTargetExpandable(target) {
    return shouldShowRuleIssueDiff(target) || target?.classification === "unresolved" || Boolean(target?.message);
  }

  /** @param {any} target */
  function targetDetailText(target) {
    const key = `rules.managed.detail.${target?.reason || "unknown"}`;
    const text = $t(key);
    if (text !== key) return text;
    if (target?.message) return target.message;
    return $t("rules.managed.detail.unknown");
  }

  function invalidateRuleIssueDiffs() {
    ruleIssueDiffRevision += 1;
    ruleIssueDiffs = {};
  }

  /** @param {any} target */
  async function loadRuleIssueTargetDiff(target) {
    const key = ruleIssueTargetKey(target);
    if (!key || ["loading", "loaded"].includes(ruleIssueDiffs[key]?.status)) return;
    if (!shouldShowRuleIssueDiff(target)) return;
    const revision = ruleIssueDiffRevision;
    ruleIssueDiffs = { ...ruleIssueDiffs, [key]: { status: "loading", diff: null } };
    const current = target?.current_block || "";
    const expected = target?.expected_block || "";
    try {
      const diff = await diffRules(
        current,
        $t("rules.managed.inspect_target", { tool: target?.tool_name || target?.tool_id || "" }),
        expected,
        $t("rules.managed.inspect_source"),
      );
      if (revision !== ruleIssueDiffRevision) return;
      ruleIssueDiffs = { ...ruleIssueDiffs, [key]: { status: "loaded", diff } };
    } catch {
      if (revision !== ruleIssueDiffRevision) return;
      ruleIssueDiffs = {
        ...ruleIssueDiffs,
        [key]: {
          status: "failed",
          diff: {
            left_label: $t("rules.managed.inspect_source"),
            right_label: $t("rules.managed.inspect_target", { tool: target?.tool_name || target?.tool_id || "" }),
            changes: [],
          },
        },
      };
    }
  }

  /** @param {any} target */
  async function toggleRuleIssueTarget(target) {
    if (!isRuleIssueTargetExpandable(target)) return;
    const key = ruleIssueTargetKey(target);
    if (!key) return;
    const next = new Set(ruleIssueExpandedTargets);
    const opening = !next.has(key);
    if (opening) {
      next.add(key);
    } else {
      next.delete(key);
    }
    ruleIssueExpandedTargets = next;
    if (opening) {
      if (shouldShowRuleIssueDiff(target)) {
        await loadRuleIssueTargetDiff(target);
      }
    }
  }
  /**
   * 规则页三标签共享的注入状态提示文案（用于跨页保持反馈一致）
   * @param {string} msg
   * @param {number} [timeoutMs]
   */
  function showRulesInjectStatus(msg, timeoutMs = 0) {
    rulesInjectStatusMsg = msg;
    if (rulesInjectStatusTimer) {
      clearTimeout(rulesInjectStatusTimer);
      rulesInjectStatusTimer = null;
    }
    if (timeoutMs > 0) {
      rulesInjectStatusTimer = setTimeout(() => {
        rulesInjectStatusMsg = "";
        rulesInjectStatusTimer = null;
      }, timeoutMs);
    }
  }

  /** @param {any | null} [run] */
  async function refreshManagedRulesState(run = null) {
    try {
      managedRulesState = await trackModulePerformanceRequest(run, "managed-rules-state", () => getManagedRulesState());
      invalidateRuleIssueDiffs();
      await logAppEvent({
        level: "debug",
        category: "rules",
        action: "managed_rules_scan",
        result: "ok",
        message: `targets=${managedRulesState?.summary?.managed_targets || 0}`,
      });
    } catch (e) {
      managedRulesMsg = $t("rules.managed.load_failed", { err: e });
      await logAppEvent({
        level: "warn",
        category: "rules",
        action: "managed_rules_scan",
        result: "failed",
        error: String(e),
      });
    }
  }

  /** @param {any} target */
  function targetReasonLabel(target) {
    if (target?.reason === "file_missing" && target?.can_write) {
      return $t("rules.managed.reason.file_missing_create");
    }
    const key = `rules.managed.reason.${target?.reason || "unknown"}`;
    const text = $t(key);
    return text === key ? (target?.reason || "") : text;
  }

  /** @param {any} target */
  async function openManagedInspect(target) {
    inspectManagedTarget = target;
    inspectManagedDiff = null;
    const expected = target?.expected_block || "";
    const current = target?.current_block || "";
    try {
      inspectManagedDiff = await diffRules(
        expected,
        $t("rules.managed.inspect_source"),
        current,
        $t("rules.managed.inspect_target", { tool: target?.tool_name || target?.tool_id || "" }),
      );
    } catch {
      inspectManagedDiff = {
        left_label: $t("rules.managed.inspect_source"),
        right_label: $t("rules.managed.inspect_target", { tool: target?.tool_name || target?.tool_id || "" }),
        changes: [],
      };
    }
  }

  function closeManagedInspect() {
    inspectManagedTarget = null;
    inspectManagedDiff = null;
  }

  async function refreshRulesPage() {
    const run = startRulesRun("manual-refresh");
    await onRefresh();
    try {
      await refreshDefaultRules({ syncGlobalEditor: rulesTab === "global", run });
      markModulePerformance(run, "rules-refresh-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "success");
    } catch {
      markModulePerformance(run, "rules-refresh-failed", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "failed");
    }
  }

  /** @param {any} target */
  function openLeaveManaged(target) {
    const ruleId = target?.rule_set_ids?.[0] || "common_rule";
    leaveManagedContext = {
      ruleId,
      toolIds: [target.tool_id],
      target,
    };
  }

  function closeLeaveManaged() {
    if (leaveManagedBusy) return;
    leaveManagedContext = null;
  }

  /** @param {boolean} removeManagedBlock */
  async function confirmLeaveManaged(removeManagedBlock) {
    if (!leaveManagedContext || leaveManagedBusy) return;
    leaveManagedBusy = true;
    managedRulesMsg = $t("rules.managed.leaving");
    await logAppEvent({
      level: "info",
      category: "rules",
      action: "managed_rules_leave",
      result: "started",
      message: `targets=${leaveManagedContext.toolIds.length} remove=${removeManagedBlock}`,
    });
    try {
      const result = await leaveRuleManagementTargets(
        leaveManagedContext.ruleId,
        leaveManagedContext.toolIds,
        { removeManagedBlock },
      );
      managedRulesState = result.state;
      await refreshDefaultRules();
      await loadTools();
      managedRulesMsg = result.failed?.length
        ? $t("rules.managed.leave_partial", { ok: result.succeeded_tool_ids?.length || 0, fail: result.failed.length })
        : $t("rules.managed.leave_ok", { count: result.succeeded_tool_ids?.length || 0 });
      await logAppEvent({
        level: result.failed?.length ? "warn" : "info",
        category: "rules",
        action: "managed_rules_leave",
        result: result.failed?.length ? "partial" : "ok",
        message: `ok=${result.succeeded_tool_ids?.length || 0} fail=${result.failed?.length || 0} remove=${removeManagedBlock}`,
      });
      leaveManagedContext = null;
    } catch (e) {
      managedRulesMsg = $t("rules.managed.leave_failed", { err: e });
      await logAppEvent({
        level: "error",
        category: "rules",
        action: "managed_rules_leave",
        result: "failed",
        error: String(e),
      });
    } finally {
      leaveManagedBusy = false;
    }
  }

  /**
   * @param {{ syncGlobalEditor?: boolean, initializeMissingBaselines?: boolean, reason?: string, run?: any | null }} [options]
   */
  async function refreshDefaultRules(options = {}) {
    const run = options.run || startRulesRun(options.reason || "entry");
    const ownsRun = !options.run;
    await logAppEvent({ level: "debug", category: "rules", action: "refresh_default_rules", result: "started" });
    try {
      const [rules, rawBaselines] = await Promise.all([
        trackModulePerformanceRequest(run, "default-rule-list", () => listDefaultRules()),
        trackModulePerformanceRequest(run, "rule-baselines", () => getDefaultRuleInjectionBaselines()),
      ]);
      defaultRulesList = rules;

      let baselines = normalizeDefaultRuleBaselines(rawBaselines);
      if (options.initializeMissingBaselines && !hasAnyDefaultRuleBaseline(baselines)) {
        baselines = buildDefaultRuleBaselines(rules);
        await trackModulePerformanceRequest(run, "rule-baselines-write", () => setDefaultRuleInjectionBaselines(baselines));
      }
      defaultRuleBaselines = baselines;

      if (options.syncGlobalEditor) {
        const main = rules.find((/** @type {any} */ x) => x.id === "common_rule");
        const c = main ? main.content : "";
        // Only overwrite textarea if no unsaved edits
        if (commonRuleContent === commonRuleOriginal) {
          commonRuleContent = c;
        }
        commonRuleOriginal = c;
      }
      await refreshManagedRulesState(run);
      setModulePerformanceCounters(run, {
        defaultRules: defaultRulesList.length,
        toolRules: readableToolRules.length,
        managedTargets: managedRulesState?.summary?.managed_targets || 0,
      });
      markModulePerformance(run, "rules-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
      if (ownsRun) {
        markModulePerformance(run, "rules-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
        finishAndRecordModulePerformanceRun(run, "success");
      }
      await logAppEvent({
        level: "info",
        category: "rules",
        action: "refresh_default_rules",
        result: "ok",
        message: `rules=${rules.length}`,
      });
    } catch (e) {
      if (ownsRun) {
        markModulePerformance(run, "rules-failed", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
        finishAndRecordModulePerformanceRun(run, "failed");
      }
      await logAppEvent({
        level: "error",
        category: "rules",
        action: "refresh_default_rules",
        result: "failed",
        error: String(e),
      });
      throw e;
    }
  }

  /** @param {any} rule */
  function getRuleTargetIds(rule) {
    return resolveRuleTargetIds(rule, $managedTools.map((/** @type {any} */ tool) => tool.id));
  }

  /** @param {any[]} rules */
  function getMergedRuleTargetIds(rules) {
    return resolveMergedRuleTargetIds(rules, $managedTools.map((/** @type {any} */ tool) => tool.id));
  }

  /** @param {string} toolId */
  function formatToolLabel(toolId) {
    return getToolName(toolId, $managedTools) || toolId;
  }

  /** @param {string} toolId */
  function selectTool(toolId) {
    const run = startRulesRun("tool-switch");
    activeToolId.set(toolId);
    markModulePerformance(run, "rules-tool-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
    markModulePerformance(run, "rules-tool-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
    finishAndRecordModulePerformanceRun(run, "success");
  }

  /** @param {string[]} toolIds */
  async function injectToToolIds(toolIds) {
    let ok = 0, fail = 0;
    /** @type {string[]} */
    const succeeded = [];
    /** @type {string[]} */
    const failed = [];
    for (const toolId of toolIds) {
      try {
        await logAppEvent({
          level: "info",
          category: "rules",
          action: "inject_default_rules",
          result: "started",
          toolId,
          targetRole: "tool_rule",
        });
        await injectDefaultRules(toolId);
        await logAppEvent({
          level: "info",
          category: "rules",
          action: "inject_default_rules",
          result: "ok",
          toolId,
          targetRole: "tool_rule",
        });
        ok++;
        succeeded.push(toolId);
      } catch (e) {
        await logAppEvent({
          level: "error",
          category: "rules",
          action: "inject_default_rules",
          result: "failed",
          toolId,
          targetRole: "tool_rule",
          error: String(e),
        });
        fail++;
        failed.push(toolId);
      }
    }
    return { ok, fail, succeeded, failed };
  }

  /**
   * @param {string} toolId
   */
  function findManagedRuleTarget(toolId) {
    return (managedRulesState?.targets || []).find((/** @type {any} */ target) => String(target.tool_id) === String(toolId));
  }

  /**
   * @param {string} toolId
   * @param {Record<string,string>} injectionTargets
   */
  function resolveRuleInjectionPath(toolId, injectionTargets) {
    const managedTarget = findManagedRuleTarget(toolId);
    if (managedTarget?.target_path) return managedTarget.target_path;
    if (injectionTargets?.[toolId]) return injectionTargets[toolId];
    return "";
  }

  /**
   * @param {string} toolId
   */
  function resolveRuleInjectionException(toolId) {
    const managedTarget = findManagedRuleTarget(toolId);
    if (!managedTarget) return "";
    if (managedTarget.can_write === false) return $t("rules.inject.preview_error_readonly");
    return "";
  }

  /**
   * @param {string[]} toolIds
   * @param {{ feedbackTarget?: "global" | "custom" }} [options]
   */
  async function openRulesInjectPreview(toolIds, options = {}) {
    if (options.feedbackTarget === "custom") return;
    const uniqueToolIds = [...new Set(toolIds.map(String))].filter(Boolean);
    let injectionTargets = /** @type {Record<string,string>} */ ({});
    try {
      injectionTargets = await getInjectionTargets();
    } catch {
      injectionTargets = /** @type {Record<string,string>} */ ({});
    }
    const changes = [];
    const exceptions = [];
    for (const toolId of uniqueToolIds) {
      const managedTarget = findManagedRuleTarget(toolId);
      const path = resolveRuleInjectionPath(toolId, injectionTargets);
      const toolName = formatToolLabel(toolId);
      const dismissKey = ruleIssueDismissalKey(toolId, managedTarget?.reason || "unconfigured_target", managedTarget?.target_path || "");
      if (!path) {
        exceptions.push({
          toolId,
          toolName,
          path: $t("rules.inject.preview_unknown_path"),
          reason: $t("rules.inject.preview_error_unconfigured"),
          dismissKey,
        });
        continue;
      }
      const reason = resolveRuleInjectionException(toolId);
      if (reason) {
        exceptions.push({ toolId, toolName, path, reason, dismissKey });
        continue;
      }
      changes.push({
        toolId,
        toolName,
        path,
        willCreate: managedTarget?.reason === "file_missing" && managedTarget?.can_write !== false,
      });
    }
    rulesInjectPreview = {
      changes,
      exceptions,
      feedbackTarget: "global",
    };
    showRulesInjectPreview = true;
  }

  function closeRulesInjectPreview() {
    if (rulesInjectPreviewBusy) return;
    showRulesInjectPreview = false;
  }

  async function confirmRulesInjectPreview() {
    if (rulesInjectPreviewBusy) return;
    const targetIds = rulesInjectPreview.changes.map((item) => item.toolId);
    if (targetIds.length === 0 && rulesInjectPreview.exceptions.length === 0) return;
    const hadPreviewExceptions = rulesInjectPreview.exceptions.length > 0;
    rulesInjectPreviewBusy = true;
    managedRulesBusy = true;
    managedRulesMsg = "";
    try {
      await logAppEvent({
        level: "info",
        category: "rules",
        action: "rules_inject_preview_confirm",
        result: "started",
        message: `targets=${targetIds.length}`,
      });
      showRulesInjectStatus($t("rules.inject.injecting_linked"));
      commonRuleMsg = "";
      customRuleMsg = "";
      const commonWasPending = commonRulePendingInject;
      const { ok, fail, succeeded } = await injectToToolIds(targetIds);
      const succeededSet = new Set(succeeded.map(String));
      const previewTargetsCovered = targetIds.every((toolId) => succeededSet.has(String(toolId)));
      await loadTools();
      if (succeeded.length > 0) {
        await persistCustomInjectedBaselinesForTargets(succeeded);
        const commonTargetsCovered = !commonWasPending || previewTargetsCovered;
        if (commonTargetsCovered && shouldPersistCommonBaselineAfterCustomInject(succeeded)) {
          const withCommon = persistInjectedBaselinesState(defaultRulesList, defaultRuleBaselines, {
            includeCommon: true,
            includeCustom: false,
          });
          await setDefaultRuleInjectionBaselines(withCommon);
          defaultRuleBaselines = withCommon;
        }
      }
      if (previewTargetsCovered && hadPreviewExceptions) {
        const nextDismissed = new Set(dismissedRuleIssueTargetKeys);
        for (const item of rulesInjectPreview.exceptions) {
          if (item.dismissKey) nextDismissed.add(item.dismissKey);
        }
        dismissedRuleIssueTargetKeys = nextDismissed;
      }
      await refreshDefaultRules({ syncGlobalEditor: rulesTab === "global" });
      const resultMessage = targetIds.length === 0
        ? $t("rules.inject.result_acknowledged")
        : buildLinkedInjectResultMessage(ok, fail);
      showRulesInjectStatus(resultMessage, 4000);
      showRulesInjectPreview = false;
      isCommonRuleEditing = false;
      commonRuleEditSnapshot = commonRuleContent;
    } catch (e) {
      showRulesInjectStatus($t("rules.inject.result_failed", { err: e }), 4000);
      await logAppEvent({
        level: "error",
        category: "rules",
        action: "rules_inject_preview_confirm",
        result: "failed",
        error: String(e),
      });
    } finally {
      rulesInjectPreviewBusy = false;
      managedRulesBusy = false;
    }
  }

  /**
   * Legacy custom-rule helpers are retained for old local data compatibility.
   * @param {number} ok
   * @param {number} fail
   */
  /** @param {any} rule */
  function buildDefaultRuleTargetBadges(rule) {
    const tools = $managedTools;
    if (!tools.length) return [];
    const injectTo = Array.isArray(rule?.inject_to) ? rule.inject_to.map(String) : [];
    const allTargets = injectTo.length === 0;
    return tools.map((/** @type {any} */ tool) => {
      const inScope = allTargets || injectTo.includes(tool.id);
      if (inScope) {
        return { label: tool.name, status: "installed", toolId: tool.id, hideKindIcon: true };
      }
      return { label: tool.name, status: "available", toolId: tool.id };
    });
  }

  /** @param {any} item @param {string} query */
  function matchSearch(item, query) {
    if (!query) return true;
    const q = query.toLowerCase();
    const name = (item.name || item.label || item.key || "").toLowerCase();
    const displayName = (item.display_name || item.displayName || "").toLowerCase();
    const desc = (item.description || "").toLowerCase();
    const content = item.content != null ? String(item.content).toLowerCase() : "";
    return name.includes(q) || displayName.includes(q) || desc.includes(q) || content.includes(q);
  }

  /** @param {number} ok @param {number} fail */
  function buildLinkedInjectResultMessage(ok, fail) {
    if (fail === 0) return $t("rules.inject.result_ok_linked", { ok });
    return $t("rules.inject.result_partial_linked", { ok, fail });
  }

  /** @param {any[]} rules */
  function getCustomPendingRuleIds(rules) {
    return resolveCustomPendingRuleIds(
      rules,
      defaultRuleBaselines,
      $managedTools.map((/** @type {any} */ tool) => tool.id),
    );
  }

  /**
   * @param {string} ruleId
   * @param {any} [rule]
   */
  function getStoredOrCurrentPendingTargetIds(ruleId, rule = null) {
    return resolveStoredOrCurrentPendingTargetIds(
      ruleId,
      rule,
      defaultRuleBaselines,
      $managedTools.map((/** @type {any} */ tool) => tool.id),
    );
  }

  /**
   * @param {string} ruleId
   * @param {string[]} targetIds
   */
  async function markCustomRulePendingTargets(ruleId, targetIds) {
    const baselines = markCustomRulePendingTargetsState(defaultRuleBaselines, ruleId, targetIds);
    await setDefaultRuleInjectionBaselines(baselines);
    defaultRuleBaselines = baselines;
  }

  /**
   * @param {string[]} targetIds
   */
  async function persistCustomInjectedBaselinesForTargets(targetIds) {
    const baselines = persistCustomInjectedBaselinesForTargetsState(
      defaultRulesList,
      defaultRuleBaselines,
      $managedTools.map((/** @type {any} */ tool) => tool.id),
      targetIds,
    );
    await setDefaultRuleInjectionBaselines(baselines);
    defaultRuleBaselines = baselines;
  }

  function getCurrentCustomPendingTargetIds() {
    return resolveCurrentCustomPendingTargetIds(
      defaultRulesList,
      defaultRuleBaselines,
      $managedTools.map((/** @type {any} */ tool) => tool.id),
    );
  }

  /**
   * @param {"save"|"delete"|"batch"} action
   * @param {string[]} targetIds
   */
  function openCustomInjectConfirm(action, targetIds) {
    void action;
    void targetIds;
  }

  /**
   * Legacy custom-rule injection state can only clear global status after a successful target write.
   * @param {string[]} succeeded
   */
  function shouldPersistCommonBaselineAfterCustomInject(succeeded) {
    return succeeded.length > 0;
  }

  let commonRulePendingInject = $derived.by(() => {
    if (commonRuleContent !== commonRuleOriginal) return false;
    const rule = defaultRulesList.find((/** @type {any} */ item) => item.id === "common_rule");
    if (!rule) return Boolean(defaultRuleBaselines.common_rule);
    return !defaultRuleBaselineMatches(
      rule,
      defaultRuleBaselines.common_rule,
      $managedTools.map((/** @type {any} */ tool) => tool.id),
    );
  });
  let customPendingRuleIds = $derived.by(() => getCustomPendingRuleIds(defaultRulesList));
  let customPendingRuleIdSet = $derived.by(() => new Set(customPendingRuleIds));
  let commonRuleTargetIdSet = $derived.by(() => {
    const rule = defaultRulesList.find((/** @type {any} */ item) => item.id === "common_rule");
    return new Set(getRuleTargetIds(rule || null).map(String));
  });
  /** @param {any} tool */
  function toolHasWritableGlobalRuleFile(tool) {
    return (tool?.capabilities || []).some((/** @type {any} */ capability) => isInjectableGlobalRuleFileCapability(tool, capability));
  }
  let injectableRuleToolIds = $derived.by(() => {
    const ids = new Set();
    for (const target of managedRulesState?.targets || []) {
      if (target?.target_path && target?.can_write) {
        ids.add(String(target.tool_id));
      }
    }
    for (const tool of $managedTools) {
      if (toolHasWritableGlobalRuleFile(tool)) {
        ids.add(String(tool.id));
      }
    }
    return ids;
  });
  /** @param {string} toolId */
  function isInjectableRuleTool(toolId) {
    return injectableRuleToolIds.has(String(toolId));
  }
  let linkedPendingToolIds = $derived.by(() => {
    const ids = new Set();
    if (commonRulePendingInject) {
      for (const tool of $managedTools) {
        if (commonRuleTargetIdSet.has(String(tool.id)) && isInjectableRuleTool(tool.id)) ids.add(String(tool.id));
      }
    }
    return Array.from(ids);
  });
  let linkedPendingSourceKey = $derived.by(() => {
    if (linkedPendingToolIds.length === 0) return "none";
    if (commonRulePendingInject) return "global";
    return "none";
  });
  let linkedHasPendingInject = $derived.by(() => linkedPendingSourceKey !== "none");
  let linkedPendingBannerVisible = $derived(
    linkedHasPendingInject && !linkedPendingBannerDismissed && !showRulesInjectPreview
  );
  let linkedInjectButtonDisabled = $derived.by(() =>
    commonRuleSaving || (rulesTab === "global" && commonRuleContent !== commonRuleOriginal)
  );
  let ruleIssueToolIds = $derived.by(() => {
    const ids = new Set();
    if (linkedPendingBannerVisible) {
      for (const toolId of linkedPendingToolIds) ids.add(String(toolId));
    }
    for (const target of managedAffectedTargets) ids.add(String(target.tool_id));
    return Array.from(ids);
  });
  let ruleIssueVisible = $derived.by(() => managedStateVisible || linkedPendingBannerVisible);
  let ruleStatusBandVisible = $derived(rulesTab !== "tool" && (ruleIssueVisible || rulesInjectStatusMsg || managedRulesMsg));
  let ruleIssueDetailTargets = $derived.by(() => {
    const byId = new Map();
    for (const target of managedAffectedTargets) {
      byId.set(String(target.tool_id), target);
    }
    if (linkedPendingBannerVisible) {
      for (const toolId of linkedPendingToolIds) {
        const id = String(toolId);
        if (!byId.has(id)) {
          byId.set(id, {
            tool_id: id,
            tool_name: formatToolLabel(id),
            target_path: "",
            rule_set_ids: [],
            rule_set_names: [],
            classification: "requires_sync",
            reason: "pending_source",
            can_read: true,
            can_write: true,
            source_pending: true,
            has_managed_block: false,
            expected_block: "",
            current_block: "",
          });
        }
      }
    }
    return Array.from(byId.values());
  });
  let ruleIssueApplyDisabled = $derived.by(() =>
    managedRulesBusy || rulesInjectPreviewBusy || ruleIssueToolIds.length === 0 || (linkedPendingBannerVisible ? linkedInjectButtonDisabled : false)
  );

  $effect(() => {
    if (!linkedHasPendingInject) {
      linkedPendingBannerDismissed = false;
    }
  });

  $effect(() => {
    if (ruleIssueVisible) {
      if (!ruleIssueWasVisible) {
        ruleIssueWasVisible = true;
      }
      return;
    }
    if (ruleIssueWasVisible) {
      ruleIssueDetailsOpen = false;
      ruleIssueExpandedTargets = new Set();
      invalidateRuleIssueDiffs();
      ruleIssueWasVisible = false;
    }
  });

  $effect(() => {
    if (!ruleIssueDetailsOpen) return;
    const expandedTargetsMissingDiff = ruleIssueDetailTargets.filter((/** @type {any} */ target) => {
      const key = ruleIssueTargetKey(target);
      return key && ruleIssueExpandedTargets.has(key) && shouldShowRuleIssueDiff(target) && !ruleIssueDiffs[key];
    });
    if (expandedTargetsMissingDiff.length > 0) {
      void Promise.all(expandedTargetsMissingDiff.map((/** @type {any} */ target) => loadRuleIssueTargetDiff(target)));
    }
  });

  async function triggerLinkedPendingInject() {
    if (!linkedHasPendingInject) return;
    await openRulesInjectPreview(linkedPendingToolIds, { feedbackTarget: "global" });
  }

  async function applyRuleIssueToTools() {
    await openRulesInjectPreview(ruleIssueToolIds, { feedbackTarget: "global" });
  }

  let visualInjectPreviewOpened = $state(false);
  $effect(() => {
    if (visualInjectPreviewOpened) return;
    if (visualRulesState !== "inject-confirm") return;
    if ($managedTools.length === 0) return;
    visualInjectPreviewOpened = true;
    const targetIds = $managedTools.map((/** @type {any} */ tool) => tool.id);
    void openRulesInjectPreview(targetIds, { feedbackTarget: "global" });
  });

  $effect(() => {
    if (rulesTab === "global") {
      untrack(() => {
        refreshDefaultRules({
          syncGlobalEditor: true,
          initializeMissingBaselines: true,
          reason: "entry",
        }).catch(() => {});
      });
    }
  });

  async function saveCommonRule() {
    commonRuleSaving = true;
    try {
      await saveDefaultRule({ id: "common_rule", name: $t("rules.global.title"), content: commonRuleContent, inject_to: [] });
      await logAppEvent({
        level: "info",
        category: "rules",
        action: "global_rule_save",
        result: "ok",
      });
      commonRuleOriginal = commonRuleContent;
      commonRuleEditSnapshot = commonRuleContent;
      isCommonRuleEditing = false;
      linkedPendingBannerDismissed = false;
      dismissedRuleIssueTargetKeys = new Set();
      await refreshDefaultRules();
    } catch (e) {
      await logAppEvent({
        level: "error",
        category: "rules",
        action: "global_rule_save",
        result: "failed",
        error: String(e),
      });
      commonRuleMsg = $t("global.msg.save_failed", { err: e });
    }
    finally { commonRuleSaving = false; }
  }
  // New/Edit rule
  function openNewRule() {
    editingDefaultRule = null;
    ruleEditorName = "";
    ruleEditorContent = "";
    ruleEditorTargets = $managedTools.map((/** @type {any} */ t) => t.id);
    crmEditSnapshot = { name: "", content: "", targets: [...ruleEditorTargets] };
    ruleEditorMdPreview = true;
    isCrmContentEditing = true;
    ruleEditorTargetsTouched = false;
    showRuleEditor = true;
  }
  /** @param {any} rule */
  function editDefaultRule(rule) {
    editingDefaultRule = rule;
    ruleEditorName = rule.name;
    ruleEditorContent = rule.content;
    const to = rule.inject_to;
    if (to && to.length > 0) {
      ruleEditorTargets = [...new Set(to.map(String))];
    } else {
      ruleEditorTargets = $managedTools.map((/** @type {any} */ t) => t.id);
    }
    ruleEditorMdPreview = true;
    isCrmContentEditing = false;
    crmEditSnapshot = { name: rule.name, content: rule.content, targets: [...ruleEditorTargets] };
    ruleEditorTargetsTouched = false;
    showRuleEditor = true;
  }
  function closeRuleEditor() {
    isCrmContentEditing = false;
    ruleEditorTargetsTouched = false;
    showRuleEditor = false;
  }
  function enterCrmContentEdit() {
    crmEditSnapshot = { name: ruleEditorName, content: ruleEditorContent, targets: [...ruleEditorTargets] };
    ruleEditorTargetsTouched = false;
    isCrmContentEditing = true;
  }
  function cancelCrmContentEdit() {
    if (!editingDefaultRule) {
      closeRuleEditor();
      return;
    }
    ruleEditorName = crmEditSnapshot.name;
    ruleEditorContent = crmEditSnapshot.content;
    ruleEditorTargets = [...crmEditSnapshot.targets];
    ruleEditorTargetsTouched = false;
    isCrmContentEditing = false;
  }
  function deleteRuleEditorRule() {
    const id = editingDefaultRule?.id;
    if (!id) return;
    closeRuleEditor();
    deleteRule(id);
  }
  /** @param {string} toolId */
  function toggleRuleTarget(toolId) {
    ruleEditorTargetsTouched = true;
    if (ruleEditorTargets.includes(toolId)) {
      ruleEditorTargets = ruleEditorTargets.filter(t => t !== toolId);
    } else {
      ruleEditorTargets = [...ruleEditorTargets, toolId];
    }
  }
  function toggleRuleEditorTargetSelectAll() {
    const tools = $managedTools;
    if (tools.length === 0) return;
    ruleEditorTargetsTouched = true;
    if (ruleEditorAllTargetsSelected) {
      ruleEditorTargets = [];
    } else {
      ruleEditorTargets = [...new Set(tools.map((/** @type {any} */ t) => t.id))];
    }
  }
  /** 全选 → 保存为 inject_to: []，否则为显式 id；无工具时允许空 */
  function getRuleEditorInjectToForSave() {
    const tools = $managedTools;
    const allIds = tools.map((/** @type {any} */ t) => t.id);
    const set = new Set(ruleEditorTargets);
    // Existing rules may still target tools that are currently not managed/detected.
    const hiddenTargetIds = ruleEditorTargets.filter((id) => !allIds.includes(id));
    if (allIds.length === 0) {
      return { ok: true, injectTo: [...new Set(ruleEditorTargets)] };
    }
    const allSelected = allIds.length === ruleEditorTargets.length && allIds.every((id) => set.has(id));
    const visibleSelected = allIds.filter((id) => set.has(id));
    if (allSelected && hiddenTargetIds.length === 0) return { ok: true, injectTo: [] };
    const preservedHiddenTargetIds = ruleEditorTargetsTouched ? [] : hiddenTargetIds;
    const injectTo = [...new Set([...visibleSelected, ...preservedHiddenTargetIds])];
    if (injectTo.length === 0) return { ok: false, injectTo: [] };
    return { ok: true, injectTo };
  }
  async function finalizeRuleEditorSave() {
    if (!ruleEditorName.trim()) return;
    const payload = getRuleEditorInjectToForSave();
    if (!payload.ok) return;
    ruleEditorSaving = true;
    try {
      /** @type {any} */
      const edRule = editingDefaultRule;
      const id = edRule?.id || `rule_${Date.now()}`;
      const savedRule = {
        id,
        name: ruleEditorName.trim(),
        content: ruleEditorContent,
        inject_to: payload.injectTo,
        managed_targets: payload.injectTo.length === 0
          ? $managedTools.map((/** @type {any} */ tool) => tool.id)
          : payload.injectTo,
      };
      await saveDefaultRule(savedRule);
      await logAppEvent({
        level: "info",
        category: "rules",
        action: edRule ? "custom_rule_update" : "custom_rule_create",
        result: "ok",
        message: `targets=${payload.injectTo.length === 0 ? "all" : payload.injectTo.length}`,
      });
      closeRuleEditor();
      await refreshDefaultRules();
      const main = defaultRulesList.find((/** @type {any} */ x) => x.id === "common_rule");
      const mainContent = main ? main.content : "";
      if (commonRuleContent === commonRuleOriginal) {
        commonRuleContent = mainContent;
        commonRuleOriginal = mainContent;
      }
      const pendingTargets = getMergedRuleTargetIds([edRule, savedRule].filter(Boolean));
      await markCustomRulePendingTargets(id, pendingTargets);
      openCustomInjectConfirm("save", pendingTargets);
    } catch (e) {
      await logAppEvent({
        level: "error",
        category: "rules",
        action: editingDefaultRule ? "custom_rule_update" : "custom_rule_create",
        result: "failed",
        error: String(e),
      });
      console.error("save rule failed:", e);
    }
    finally { ruleEditorSaving = false; }
  }

  async function saveRuleEditor() {
    if (!canSaveRuleEditor) return;
    await finalizeRuleEditorSave();
  }
  /** @param {string} ruleId */
  async function deleteRule(ruleId) {
    const removed = defaultRulesList.find((/** @type {any} */ r) => r.id === ruleId);
    try {
      await deleteDefaultRule(ruleId);
      await logAppEvent({
        level: "info",
        category: "rules",
        action: "custom_rule_delete",
        result: "ok",
      });
      defaultRulesList = defaultRulesList.filter(r => r.id !== ruleId);
      if (ruleId === "common_rule") { commonRuleContent = ""; commonRuleOriginal = ""; }
      await refreshDefaultRules();
      if (removed && ruleId !== "common_rule") {
        const pendingTargets = getRuleTargetIds(removed);
        await markCustomRulePendingTargets(ruleId, pendingTargets);
        openCustomInjectConfirm("delete", pendingTargets);
      }
    } catch (e) {
      await logAppEvent({
        level: "error",
        category: "rules",
        action: "custom_rule_delete",
        result: "failed",
        error: String(e),
      });
      console.error("delete rule failed:", e);
    }
  }

  // Additional rules: all except common_rule
  let additionalRules = $derived(defaultRulesList.filter((/** @type {any} */ r) => r.id !== "common_rule"));
  let filteredAdditionalRules = $derived.by(() =>
    globalSearchQuery.trim() ? additionalRules.filter((/** @type {any} */ r) => matchSearch(r, globalSearchQuery)) : additionalRules
  );


</script>

        <div class="view-panel">
          <div class="view-fixed-header" class:view-fixed-header--search-popover={searchOpen && rulesTab === "tool"}>
            <div
              class="view-header management-header-row"
              class:management-header-row--search-open={searchOpen}
              data-tauri-drag-region
            >
              <h2 class="management-header-title" data-tauri-drag-region>{$t("rules.title")}</h2>
              <div class="header-actions management-header-actions">
                {#if searchOpen}
                  <div class="workspace-search-anchor" bind:this={toolRuleSearchAnchor}>
                    <div class="search-input-wrap">
                      <span class="search-input-wrap__icon" aria-hidden="true"><Search size={13} strokeWidth={1.8} /></span>
                      <input
                        bind:this={ruleSearchInput}
                        class="search-input"
                        type="text"
                        placeholder={rulesTab === "tool"
                          ? $t("file_workspace_search.placeholder_tree")
                          : (rulesTab === "global" ? $t("file_workspace_search.placeholder_current_content") : $t("global.search.placeholder"))}
                        bind:value={globalSearchQuery}
                        onpointerdown={() => { if (rulesTab === "tool") toolRuleSearchResultsOpen = true; }}
                        onfocus={() => { if (rulesTab === "tool") toolRuleSearchResultsOpen = true; }}
                        oninput={() => { if (rulesTab === "tool") toolRuleSearchResultsOpen = true; }}
                      />
                      {#if showCommonRuleSearchControls}
                        <span class="search-match-count">
                          {commonRuleSearchMatchCount > 0 ? `${commonRuleSearchMatchIndex + 1}/${commonRuleSearchMatchCount}` : "0/0"}
                        </span>
                        <button
                          class="icon-btn-tiny search-nav-btn"
                          onclick={() => moveCommonRuleContentSearch("previous")}
                          disabled={commonRuleSearchMatchCount <= 0}
                          aria-label={$t("editor.search.previous")}
                        >
                          <ChevronUp size={13} strokeWidth={1.8} />
                        </button>
                        <button
                          class="icon-btn-tiny search-nav-btn"
                          onclick={() => moveCommonRuleContentSearch("next")}
                          disabled={commonRuleSearchMatchCount <= 0}
                          aria-label={$t("editor.search.next")}
                        >
                          <ChevronDown size={13} strokeWidth={1.8} />
                        </button>
                      {/if}
                      <button class="icon-btn-tiny" onclick={() => { searchOpen = false; globalSearchQuery = ""; toolRuleSearchResultsOpen = false; }} aria-label={$t("global.search.close")}>
                        <X size={12} strokeWidth={1.8} />
                      </button>
                    </div>
                    <FileWorkspaceSearchResults
                      query={globalSearchQuery}
                      groups={toolRuleSearchGroups}
                      visible={canShowToolRuleSearchResults}
                      dismissRoot={toolRuleSearchAnchor}
                      onActivate={activateToolRuleSearchResult}
                      onDismiss={() => (toolRuleSearchResultsOpen = false)}
                    />
                  </div>
                {:else}
                  <button
                    type="button"
                    class="refresh-btn"
                    onclick={() => (searchOpen = true)}
                    aria-label={$t("global.search.title")}
                  >
                    <Search size={14} strokeWidth={1.8} />
                  </button>
                {/if}
                {#if rulesTab === "tool" && ruleCreateToolTargets.length > 0}
                  <Tooltip label={$t("rules.tool.create_rule_file")} placement="bottom-end" maxWidth="240px">
                    <button
                      type="button"
                      class="refresh-btn"
                      onclick={openToolRuleCreate}
                      disabled={isSingleToolRuleEditing || ruleCreateDialogOpen}
                      aria-label={$t("rules.tool.create_rule_file")}
                    >
                      <Plus size={14} strokeWidth={1.8} />
                    </button>
                  </Tooltip>
                {/if}
                {#if linkedHasPendingInject && linkedPendingBannerDismissed}
                  <Tooltip label={$t("rules.inject.header_inject_tooltip")} placement="bottom-end" maxWidth="240px">
                    <button
                      type="button"
                      class="refresh-btn management-header-inject-icon"
                      onclick={triggerLinkedPendingInject}
                      disabled={linkedInjectButtonDisabled}
                      aria-label={$t("rules.inject.header_inject_tooltip")}
                    >
                      <ArrowDownToLine size={14} strokeWidth={1.8} />
                    </button>
                  </Tooltip>
                {/if}
                <button
                  type="button"
                  class="refresh-btn"
                  onclick={refreshRulesPage}
                  disabled={refreshing}
                  aria-label={$t("global.action.refresh")}
                >
                  <RefreshCw size={14} strokeWidth={1.8} class={refreshing ? "spin" : ""} />
                </button>
              </div>
            </div>
            <div class="content-tab-row">
              <div class="content-tabs" aria-label={$t("rules.title")}>
                <button
                  type="button"
                  class="content-tab"
                  class:active={rulesTab === "global"}
                  aria-current={rulesTab === "global" ? "page" : undefined}
                  onclick={() => { setRulesTab("global"); }}
                >{$t("rules.tab.global")}</button>
                <button
                  type="button"
                  class="content-tab"
                  class:active={rulesTab === "tool"}
                  aria-current={rulesTab === "tool" ? "page" : undefined}
                  onclick={() => { setRulesTab("tool"); }}
                >{$t("rules.tab.tool")}</button>
              </div>
            </div>
          </div>
          {#if rulesTab === "tool" || ruleStatusBandVisible}
            <div class="view-pinned-toolbar rules-pinned-toolbar">
              {#if rulesTab === "tool"}
                <ProtectedToolSelector
                  tools={$managedTools}
                  activeToolId={currentToolId}
                  ariaLabel={$t("rules.title")}
                  onSelect={selectTool}
                />
              {/if}
              {#if ruleStatusBandVisible}
                <div class="rules-status-band">
                  {#if ruleIssueVisible}
                    <div class="managed-state-inline pending-inject-inline--band rule-issue-band">
                      <div class="managed-state-inline__top">
                        <div class="pending-inject-inline__left">
                          <div class="pending-inject-hint managed-state-hint">
                            <AlertTriangle size={14} />
                            <span>{$t("rules.issue.status_label")}</span>
                          </div>
                        </div>
                        <div class="pending-inject-inline__actions">
                          <button
                            type="button"
                            class="btn btn-secondary btn-sm pending-inject-inline__btn"
                            onclick={() => (ruleIssueDetailsOpen = !ruleIssueDetailsOpen)}
                            aria-expanded={ruleIssueDetailsOpen}
                          >
                            <Eye size={14} strokeWidth={1.8} />
                            <span>{$t(ruleIssueDetailsOpen ? "rules.issue.hide_details" : "rules.issue.view_details")}</span>
                          </button>
                          <button
                            type="button"
                            class="btn btn-primary btn-sm pending-inject-inline__btn"
                            onclick={applyRuleIssueToTools}
                            disabled={ruleIssueApplyDisabled}
                          >
                            <Check size={14} />
                            <span>{$t("rules.issue.inject")}</span>
                          </button>
                        </div>
                      </div>
                      {#if ruleIssueDetailsOpen}
                        <div class="rule-issue-details">
                          {#each ruleIssueDetailTargets as target}
                            {@const targetExpandable = isRuleIssueTargetExpandable(target)}
                            <div class="rule-issue-target">
                              <button
                                type="button"
                                class="rule-issue-target__header"
                                class:rule-issue-target__header--open={targetExpandable && isRuleIssueTargetExpanded(target)}
                                class:rule-issue-target__header--static={!targetExpandable}
                                onclick={() => toggleRuleIssueTarget(target)}
                                aria-expanded={targetExpandable ? isRuleIssueTargetExpanded(target) : undefined}
                              >
                                <span
                                  class="rule-issue-target__chevron"
                                  class:rule-issue-target__chevron--open={targetExpandable && isRuleIssueTargetExpanded(target)}
                                  class:rule-issue-target__chevron--placeholder={!targetExpandable}
                                >
                                  {#if targetExpandable}
                                    <ChevronRight size={14} />
                                  {/if}
                                </span>
                                <span class="rule-issue-target__name">{target.tool_name || target.tool_id}</span>
                                <span class="rule-issue-target__reason">{targetReasonLabel(target)}</span>
                                {#if target.target_path}
                                  <span class="rule-issue-target__path">{target.target_path}</span>
                                {/if}
                              </button>
                              {#if targetExpandable && isRuleIssueTargetExpanded(target)}
                                <div class="rule-issue-target__body">
                                  {#if shouldShowRuleIssueDiff(target)}
                                    {@const diffEntry = getRuleIssueDiffEntry(target)}
                                    {#if diffEntry?.status === "loaded" || diffEntry?.status === "failed"}
                                      <div class="diff-content rule-issue-diff-content">
                                        {#if diffEntry.diff.changes?.length}
                                          <div class="diff-lines rule-issue-diff-lines">
                                            {#each compactRuleIssueDiffLines(diffEntry.diff.changes) as line}
                                              <div class="diff-line {line.tag}" class:diff-fold={line.tag === "fold"}>
                                                <span class="diff-sign">{line.tag === "delete" ? "-" : line.tag === "insert" ? "+" : line.tag === "fold" ? "" : " "}</span>
                                                <span class="diff-text">{line.content}</span>
                                              </div>
                                            {/each}
                                          </div>
                                        {:else}
                                          <div class="rule-issue-diff-empty">{$t("rules.issue.diff_empty")}</div>
                                        {/if}
                                      </div>
                                    {:else}
                                      <div class="rule-issue-diff-empty">{$t("common.loading")}</div>
                                    {/if}
                                  {:else}
                                    <div class="rule-issue-detail-text">{targetDetailText(target)}</div>
                                  {/if}
                                </div>
                              {/if}
                            </div>
                          {/each}
                        </div>
                      {/if}
                    </div>
                  {/if}
                  {#if rulesInjectStatusMsg}
                    <div class="inject-msg inject-msg--band">{rulesInjectStatusMsg}</div>
                  {/if}
                  {#if managedRulesMsg}
                    <div class="inject-msg inject-msg--band">{managedRulesMsg}</div>
                  {/if}
                </div>
              {/if}
            </div>
          {/if}
          <div class="view-scroll-content rules-scroll-content">
            <div class="management-content-shell rules-content-shell">
              <ModulePerformanceBar moduleId="rules" />

              {#if rulesTab === "global"}
                <!-- Global Rules: single editor -->
                <div class="common-rule-card">
                  {#if commonRuleMsg}
                    <div class="inject-msg">{commonRuleMsg}</div>
                  {/if}

                  <FileViewerShell
                    title={$t("rules.global.title")}
                    subtitle={$t("rules.global.desc")}
                    subtitleMonospace={false}
                    modeLabel={isCommonRuleEditing ? $t("rules.editor.editing") : ""}
                  >
                    {#snippet actions()}
                      {#if isCommonRuleEditing}
                        <span class="file-viewer-inline-status">
                          {#if isCommonRuleDirty}{$t("rules.editor.unsaved")} · {/if}{$t("rules.editor.lines", { count: commonRuleLineCount })}
                        </span>
                        <button type="button" class="file-viewer-btn" onclick={cancelCommonRuleEdit} disabled={commonRuleSaving}>
                          {$t("rules.editor.cancel")}
                        </button>
                        <button type="button" class="file-viewer-primary-btn" onclick={saveCommonRule} disabled={commonRuleSaving || !isCommonRuleDirty}>
                          {commonRuleSaving ? $t("rules.global.saving") : $t("rules.global.save")}
                        </button>
                      {:else}
                        <SourceReadModeToggle
                          bind:preview={commonRuleMdPreview}
                          editing={isCommonRuleEditing}
                          sourceLabel={$t("rules.editor.mode_src")}
                          readLabel={$t("rules.editor.mode_read")}
                        />
                        <button
                          type="button"
                          class="file-viewer-btn"
                          onclick={() => {
                            commonRuleEditSnapshot = commonRuleContent;
                            isCommonRuleEditing = true;
                          }}
                        >
                          <Pencil size={14} strokeWidth={1.8} /> {$t("rules.editor.edit")}
                        </button>
                      {/if}
                    {/snippet}

                    <ContentEditor
                      bind:value={commonRuleContent}
                      bind:preview={commonRuleMdPreview}
                      originalValue={commonRuleOriginal}
                      editing={isCommonRuleEditing}
                      markdown={isCommonRuleMarkdown}
                      filename="common-rule.md"
                      placeholder={$t("rules.global.placeholder")}
                      fill={true}
                      minHeight="0"
                      sourceLabel={$t("rules.editor.mode_src")}
                      readLabel={$t("rules.editor.mode_read")}
                      editLabel={$t("rules.editor.edit")}
                      cancelLabel={$t("rules.editor.cancel")}
                      saveLabel={$t("rules.global.save")}
                      savingLabel={$t("rules.global.saving")}
                      unsavedLabel={$t("rules.editor.unsaved")}
                      saving={commonRuleSaving}
                      showModeToggle={false}
                      showActions={false}
                      showFooter={false}
                      framed={false}
                      formatLineLabel={(/** @type {number} */ count) => $t("rules.editor.lines", { count })}
                      onEdit={() => {
                        commonRuleEditSnapshot = commonRuleContent;
                        isCommonRuleEditing = true;
                      }}
                      onCancel={cancelCommonRuleEdit}
                      onSave={saveCommonRule}
                      externalSearchQuery={commonRuleExternalSearchQuery}
                      externalSearchMatchIndex={commonRuleSearchMatchIndex}
                      externalSearchVersion={commonRuleSearchVersion}
                    />
                  </FileViewerShell>
                </div>
              {:else if rulesTab === "custom"}
              <!-- Custom rules -->
              <div class="custom-rules-section">
                {#if customRuleMsg}
                  <div class="inject-msg">{customRuleMsg}</div>
                {/if}
                {#if showRuleEditor}
                  <div class="custom-rule-detail">
                    <FileViewerShell
                      variant="collection-detail"
                      title={ruleEditorName.trim() || $t("settings.rules.name_placeholder")}
                      modeLabel={!isCreatingCustomRule && isCrmContentEditing ? $t("rules.editor.editing") : ""}
                      backLabel={$t("settings.rules.back_to_list")}
                      backDisabled={ruleEditorSaving}
                      onBack={closeRuleEditor}
                    >
                      {#snippet titleContent()}
                        {#if !editingDefaultRule || isCrmContentEditing}
                          <input
                            class="crm-title-input file-viewer-title-input"
                            bind:value={ruleEditorName}
                            placeholder={$t("settings.rules.name_placeholder")}
                            aria-label={$t("settings.rules.label_name")}
                          />
                        {:else}
                          <span class="file-viewer-title" class:crm-title-ph={!ruleEditorName.trim()}>
                            {ruleEditorName.trim() || $t("settings.rules.name_placeholder")}
                          </span>
                        {/if}
                      {/snippet}

                      {#snippet actions()}
                        {#if !isCrmContentEditing && editingDefaultRule}
                          <SourceReadModeToggle
                            bind:preview={ruleEditorMdPreview}
                            editing={isCrmContentEditing}
                            sourceLabel={$t("rules.editor.mode_src")}
                            readLabel={$t("rules.editor.mode_read")}
                          />
                          <button type="button" class="file-viewer-btn danger" onclick={deleteRuleEditorRule}>
                            {$t("settings.tools.delete")}
                          </button>
                          <button type="button" class="file-viewer-btn" onclick={enterCrmContentEdit}>
                            <Pencil size={14} strokeWidth={1.8} />
                            {$t("rules.editor.edit")}
                          </button>
                        {:else}
                          {#if isCreatingCustomRule}
                            {#if isRuleEditorContentDirty}
                              <span class="file-viewer-inline-status">{$t("rules.editor.unsaved")}</span>
                            {/if}
                          {:else}
                            <span class="file-viewer-inline-status">
                              {#if isRuleEditorContentDirty}{$t("rules.editor.unsaved")} · {/if}{$t("rules.editor.lines", { count: ruleEditorLineCount })}
                            </span>
                          {/if}
                          <button type="button" class="file-viewer-btn" onclick={cancelCrmContentEdit} disabled={ruleEditorSaving}>
                            {$t("rules.editor.cancel")}
                          </button>
                          <button
                            type="button"
                            class="file-viewer-primary-btn"
                            onclick={saveRuleEditor}
                            disabled={!canSaveRuleEditor}
                          >
                            <Save size={14} />
                            <span>{$t(ruleEditorSaving ? "settings.rules.saving" : "settings.rules.save")}</span>
                          </button>
                        {/if}
                      {/snippet}

                      {#if isCrmContentEditing}
                        <div class="custom-rule-edit-body">
                          <ContentEditor
                            bind:value={ruleEditorContent}
                            bind:preview={ruleEditorMdPreview}
                            originalValue={crmEditSnapshot.content}
                            editing={isCrmContentEditing}
                            markdown={true}
                            filename="custom-rule.md"
                            ariaLabel={$t("settings.rules.label_content")}
                            placeholder={$t("settings.rules.content_placeholder")}
                            fill={true}
                            minHeight="0"
                            showModeToggle={false}
                            showFooter={false}
                            framed={false}
                            sourceLabel={$t("rules.editor.mode_src")}
                            readLabel={$t("rules.editor.mode_read")}
                            onSave={saveRuleEditor}
                          />
                          {#if $managedTools.length > 0}
                            <div class="crm-fields-well custom-rule-targets-panel">
                              <div class="crm-targets-section">
                                <div class="crm-targets-header">
                                  <div class="crm-section-label">{$t("skills.viewer.select_tools")}</div>
                                  <button
                                    type="button"
                                    class="file-viewer-btn"
                                    onclick={toggleRuleEditorTargetSelectAll}
                                    disabled={ruleEditorSaving}
                                  >
                                    {ruleEditorAllTargetsSelected ? $t("skills.viewer.cancel_all") : $t("skills.viewer.select_all")}
                                  </button>
                                </div>
                                <div class="crm-targets">
                                  {#each $managedTools as tool}
                                    <button
                                      type="button"
                                      class="l2-tab"
                                      class:active={ruleEditorTargets.includes(tool.id)}
                                      onclick={() => toggleRuleTarget(tool.id)}
                                    >
                                      <ToolIcon toolId={tool.id} size={14} />
                                      {tool.name}
                                    </button>
                                  {/each}
                                </div>
                              </div>
                            </div>
                          {/if}
                        </div>
                      {:else}
                        <ContentEditor
                          bind:value={ruleEditorContent}
                          bind:preview={ruleEditorMdPreview}
                          originalValue={crmEditSnapshot.content}
                          editing={isCrmContentEditing}
                          markdown={true}
                          filename="custom-rule.md"
                          ariaLabel={$t("settings.rules.label_content")}
                          placeholder={$t("settings.rules.content_placeholder")}
                          fill={true}
                          minHeight="0"
                          showModeToggle={false}
                          showFooter={false}
                          framed={false}
                          sourceLabel={$t("rules.editor.mode_src")}
                          readLabel={$t("rules.editor.mode_read")}
                          onSave={saveRuleEditor}
                        />
                      {/if}
                    </FileViewerShell>
                  </div>
                {:else if additionalRules.length === 0}
                  <div class="custom-rules-empty">{$t("rules.custom.empty")}</div>
                {:else if filteredAdditionalRules.length === 0}
                  <div class="custom-rules-empty">{$t("global.search.no_results")}</div>
                {:else}
                  <div class="item-list">
                    {#each filteredAdditionalRules as rule}
                      <BaseCard
                        icon={ScrollText}
                        accentColor="rgba(167,139,250,0.10)"
                        title={rule.name}
                        description={rule.content?.length
                          ? `${rule.content.slice(0, 160)}${rule.content.length > 160 ? "..." : ""}`
                          : ""}
                        stats={$t("rules.editor.lines", { count: rule.content ? rule.content.split("\n").length : 0 })}
                        badges={buildDefaultRuleTargetBadges(rule)}
                        warning={customPendingRuleIdSet.has(rule.id)
                          ? { title: $t("rules.custom.pending_tooltip") }
                          : null}
                        onclick={() => editDefaultRule(rule)}
                      />
                    {/each}
                  </div>
                {/if}
              </div>
            {:else}
              <!-- Tool rules content -->
              {#if $managedTools.length === 0}
                <div class="empty">{$t("rules.empty.no_managed_tools")}</div>
              {:else if !currentTool}
                <div class="empty">{$t("settings.tools.status_missing")}</div>
              {:else}
                {#if toolRuleCreateMsg}
                  <div class="inject-msg">{toolRuleCreateMsg}</div>
                {/if}
                {#if activeToolRule}
                  <div class="tool-rule-single-card">
                    {#if singleToolRuleMsg}
                      <div class="inject-msg">{singleToolRuleMsg}</div>
                    {/if}
                    <FileViewerShell
                      variant="navigation-backed"
                      title={activeToolRule.label}
                      subtitle={activeToolRule.path}
                      modeLabel={isSingleToolRuleEditing ? $t("rules.editor.editing") : ""}
                      navigationVisible={toolRuleNavigationItems.length > 0}
                      navigationCollapsible={toolRuleNavigationItems.length > 0}
                      bind:navigationCollapsed={toolRuleNavigationCollapsed}
                      navigationResizeLabel={$t("global.a11y.resize_skill_file_tree")}
                      navigationCollapseLabel={$t("global.a11y.collapse_file_tree")}
                      navigationExpandLabel={$t("global.a11y.expand_file_tree")}
                    >
                      {#snippet navigation()}
                        <FileWorkspaceNavigation
                          title={$t("rules.tool.files")}
                          items={toolRuleNavigationItems}
                          onToggle={toggleToolRuleNavigationItem}
                          onSelect={selectToolRuleNavigationItem}
                          onContextMenu={openToolRuleContextMenu}
                          suppressBlankContextMenu={true}
                        />
                      {/snippet}

                      {#snippet actions()}
                        {#if isRuleUnreadable(activeToolRule)}
                          <span class="file-viewer-inline-status">{$t("rules.tool.unreadable")}</span>
                        {:else if isSingleToolRuleEditing}
                          <span class="file-viewer-inline-status">
                            {#if isSingleToolRuleDirty}{$t("rules.editor.unsaved")} · {/if}{$t("rules.editor.lines", { count: singleToolRuleLineCount })}
                          </span>
                          <button type="button" class="file-viewer-btn" onclick={cancelSingleToolRuleEdit} disabled={singleToolRuleSaving}>
                            {$t("rules.editor.cancel")}
                          </button>
                          <button type="button" class="file-viewer-primary-btn" onclick={saveSingleToolRule} disabled={singleToolRuleSaving || (!isSingleToolRuleDirty && !activeToolRule?.isNewlyCreated)}>
                            {singleToolRuleSaving ? $t("rules.editor.saving") : $t("rules.editor.save")}
                          </button>
                        {:else}
                          {#if isSingleToolRuleMarkdown}
                            <SourceReadModeToggle
                              bind:preview={singleToolRuleMdPreview}
                              editing={isSingleToolRuleEditing}
                              sourceLabel={$t("rules.editor.mode_src")}
                              readLabel={$t("rules.editor.mode_read")}
                            />
                          {/if}
                          <button type="button" class="file-viewer-btn" onclick={() => (isSingleToolRuleEditing = true)}>
                            <Pencil size={14} strokeWidth={1.8} /> {$t("rules.editor.edit")}
                          </button>
                        {/if}
                      {/snippet}

                      {#if isRuleUnreadable(activeToolRule)}
                        <PrimaryState message={$t("rules.tool.unreadable")} detail={activeToolRule.path || $t("rules.tool.unreadable_detail")} tone="warning" />
                      {:else}
                        <ContentEditor
                          bind:value={singleToolRuleContent}
                          bind:preview={singleToolRuleMdPreview}
                          originalValue={singleToolRuleOriginal}
                          editing={isSingleToolRuleEditing}
                          markdown={isSingleToolRuleMarkdown}
                          filename={activeToolRule.path || ""}
                          fill={true}
                          minHeight="0"
                          sourceLabel={$t("rules.editor.mode_src")}
                          readLabel={$t("rules.editor.mode_read")}
                          editLabel={$t("rules.editor.edit")}
                          cancelLabel={$t("rules.editor.cancel")}
                          saveLabel={$t("rules.editor.save")}
                          savingLabel={$t("rules.editor.saving")}
                          unsavedLabel={$t("rules.editor.unsaved")}
                          emptyPreviewLabel={$t("rules.editor.empty_file")}
                          saving={singleToolRuleSaving}
                          showModeToggle={false}
                          showActions={false}
                          showFooter={false}
                          framed={false}
                          formatLineLabel={(/** @type {number} */ count) => $t("rules.editor.lines", { count })}
                          externalSearchQuery={activeToolRuleExternalSearchQuery}
                          externalSearchMatchIndex={activeToolRuleExternalSearchMatchIndex}
                          externalSearchVersion={activeToolRuleExternalSearchVersion}
                          onEdit={() => (isSingleToolRuleEditing = true)}
                          onCancel={cancelSingleToolRuleEdit}
                          onSave={saveSingleToolRule}
                        />
                      {/if}
                    </FileViewerShell>
                  </div>
                {:else if toolRuleTreeRows.length === 0}
                  <div class="tool-rule-empty">
                    <div class="tool-rule-empty__title">{$t("rules.tool.empty_title")}</div>
                    {#if toolRuleEmptyDescription}
                      <div class="tool-rule-empty__desc">
                        {toolRuleEmptyDescription}
                      </div>
                    {/if}
                  </div>
                {:else}
                  <div class="tool-rule-single-card">
                    <FileViewerShell
                      variant="navigation-backed"
                      title={$t("rules.tool.select_file_title")}
                      subtitle={currentTool.name || ""}
                      subtitleMonospace={false}
                      navigationVisible={true}
                      navigationCollapsible={true}
                      bind:navigationCollapsed={toolRuleNavigationCollapsed}
                      navigationResizeLabel={$t("global.a11y.resize_skill_file_tree")}
                      navigationCollapseLabel={$t("global.a11y.collapse_file_tree")}
                      navigationExpandLabel={$t("global.a11y.expand_file_tree")}
                    >
                      {#snippet navigation()}
                        <FileWorkspaceNavigation
                          title={$t("rules.tool.files")}
                          items={toolRuleNavigationItems}
                          onToggle={toggleToolRuleNavigationItem}
                          onSelect={selectToolRuleNavigationItem}
                          onContextMenu={openToolRuleContextMenu}
                          suppressBlankContextMenu={true}
                        />
                      {/snippet}
                      <PrimaryState message={$t("rules.tool.select_file_title")} />
                    </FileViewerShell>
                  </div>
                {/if}
              {/if}
              {/if}
            </div>
          </div>
        </div>

{#if visualRulesState === "copy-modal"}
  <CopyModal rule={visualCopyRule} sourceToolId="codex" onClose={() => {}} />
{/if}
{#if ruleTreeContextMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="rule-tree-context-menu app-context-menu"
    style={`left: ${ruleTreeContextMenu.x}px; top: ${ruleTreeContextMenu.y}px;`}
    onmousedown={(event) => event.stopPropagation()}
  >
    {#if ruleTreeContextMenu.actions.includes("new_file")}
      <button type="button" class="rule-tree-context-menu__item app-context-menu__item" onclick={() => openRuleTreeNameDialog(ruleTreeContextMenu.item, "new_file")}>
        <FilePlus size={14} strokeWidth={1.8} />
        <span>{$t("rules.tool.context_new_file")}</span>
      </button>
    {/if}
    {#if ruleTreeContextMenu.actions.includes("new_directory")}
      <button type="button" class="rule-tree-context-menu__item app-context-menu__item" onclick={() => openRuleTreeNameDialog(ruleTreeContextMenu.item, "new_directory")}>
        <FolderPlus size={14} strokeWidth={1.8} />
        <span>{$t("rules.tool.context_new_directory")}</span>
      </button>
    {/if}
    {#if ruleTreeContextMenu.actions.includes("set_global_rule_file")}
      <button type="button" class="rule-tree-context-menu__item app-context-menu__item" onclick={() => setToolRuleItemAsGlobalRuleFile(ruleTreeContextMenu.item)}>
        <FileCheck2 size={14} strokeWidth={1.8} />
        <span>{$t("rules.tool.context_set_global_rule_file")}</span>
      </button>
    {/if}
    {#if ruleTreeContextMenu.actions.includes("rename")}
      <button type="button" class="rule-tree-context-menu__item app-context-menu__item" onclick={() => openRuleTreeNameDialog(ruleTreeContextMenu.item, "rename")}>
        <Pencil size={14} strokeWidth={1.8} />
        <span>{$t("rules.tool.context_rename")}</span>
      </button>
    {/if}
    {#if ruleTreeContextMenu.actions.includes("delete")}
      <button type="button" class="rule-tree-context-menu__item app-context-menu__item rule-tree-context-menu__item--danger app-context-menu__item--danger" onclick={() => deleteToolRuleTreeItem(ruleTreeContextMenu.item)}>
        <Trash2 size={14} strokeWidth={1.8} />
        <span>{$t("rules.tool.context_delete")}</span>
      </button>
    {/if}
  </div>
{/if}
{#if ruleTreeNameDialog}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onmousedown={(event) => { if (event.target === event.currentTarget) closeRuleTreeNameDialog(); }}>
    <div
      class="modal rule-tree-name-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="rule-tree-name-title"
      tabindex="-1"
      onmousedown={(event) => event.stopPropagation()}
    >
      <div class="modal-header">
        <div class="modal-title" id="rule-tree-name-title">
          {$t(ruleTreeNameDialog.mode === "new_file"
            ? "rules.tool.context_new_file_title"
            : ruleTreeNameDialog.mode === "new_directory"
              ? "rules.tool.context_new_directory_title"
              : "rules.tool.context_rename_title")}
        </div>
        <button type="button" class="icon-btn" aria-label={$t("rules.editor.close")} onclick={closeRuleTreeNameDialog}>
          <X size={18} strokeWidth={1.8} />
        </button>
      </div>
      <div class="modal-body rule-tree-name-body">
        <label class="rule-create-field">
          <span class="rule-create-label">
            {$t(ruleTreeNameDialog.mode === "new_file" || ruleTreeNameDialog.item?.kind === "file" ? "rules.tool.context_file_name" : "rules.tool.context_directory_name")}
          </span>
          <input
            bind:value={ruleTreeNameDialogValue}
            placeholder={ruleTreeNameDialog.mode === "new_file" || ruleTreeNameDialog.item?.kind === "file" ? ruleTreeNameDialogFilePlaceholder : $t("rules.tool.create_dialog_folder_placeholder")}
            aria-label={$t(ruleTreeNameDialog.mode === "new_file" || ruleTreeNameDialog.item?.kind === "file" ? "rules.tool.context_file_name" : "rules.tool.context_directory_name")}
            onkeydown={(event) => { if (event.key === "Enter" && canConfirmRuleTreeNameDialog) confirmRuleTreeNameDialog(); }}
          />
        </label>
        {#if ruleTreeNameDialogTargetPath}
          <div class="rule-create-target">{ruleTreeNameDialogTargetPath}</div>
        {/if}
        {#if ruleTreeNameDialogInvalid}
          <div class="rule-create-validation">{ruleTreeNameDialogInvalid}</div>
        {/if}
      </div>
      <div class="modal-footer rule-create-footer">
        <button type="button" class="file-viewer-btn" onclick={closeRuleTreeNameDialog} disabled={ruleTreeActionBusy}>
          {$t("rules.editor.cancel")}
        </button>
        <button type="button" class="file-viewer-primary-btn" onclick={confirmRuleTreeNameDialog} disabled={!canConfirmRuleTreeNameDialog}>
          {$t("confirm.ok")}
        </button>
      </div>
    </div>
  </div>
{/if}
{#if ruleCreateDialogOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onmousedown={(e) => { if (e.target === e.currentTarget) closeRuleCreateDialog(); }}>
    <div
      class="modal rule-create-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="rule-create-dialog-title"
      tabindex="-1"
      onmousedown={(e) => e.stopPropagation()}
    >
      <div class="modal-header">
        <div class="modal-title" id="rule-create-dialog-title">{$t("rules.tool.create_dialog_title")}</div>
        <button type="button" class="icon-btn" aria-label={$t("rules.editor.close")} onclick={closeRuleCreateDialog}>
          <X size={18} strokeWidth={1.8} />
        </button>
      </div>
      <div class="modal-body rule-create-body">
        <label class="rule-create-field">
          <span class="rule-create-label">{$t("rules.tool.create_dialog_tool")}</span>
          <select
            value={ruleCreateDialogToolId}
            onchange={(event) => selectRuleCreateTool(event.currentTarget.value)}
            aria-label={$t("rules.tool.create_dialog_tool")}
          >
            {#each ruleCreateToolTargets as entry}
              <option value={entry.tool.id}>{entry.tool.name || entry.tool.id}</option>
            {/each}
          </select>
        </label>

        <label class="rule-create-field">
          <span class="rule-create-label">{$t("rules.tool.create_target")}</span>
          <select
            value={ruleCreateDialogLocationId}
            onchange={(event) => selectToolRuleCreateOption(event.currentTarget.value)}
            aria-label={$t("rules.tool.create_target")}
            disabled={ruleCreateLocationOptions.length <= 1}
          >
            {#each ruleCreateLocationOptions as option}
              <option value={option.id}>{toolRuleCreateLocationLabel(option)}</option>
            {/each}
          </select>
        </label>

        <div class="rule-create-field">
          <span class="rule-create-label">{$t("rules.tool.create_dialog_file")}</span>
          {#if selectedToolRuleCreateOption?.kind === "directory"}
            <input
              bind:value={ruleCreateDialogFileName}
              placeholder={ruleCreateNamePlaceholder}
              aria-label={$t("rules.tool.create_name")}
            />
          {:else if selectedToolRuleCreateOption}
            <div class="rule-create-static">
              {baseName(selectedToolRuleCreateOption.path)}
            </div>
          {:else}
            <div class="rule-create-static">{$t("rules.tool.create_dialog_no_location")}</div>
          {/if}
        </div>

        {#if selectedToolRuleCreateOption?.kind === "directory"}
          <label class="rule-create-checkbox">
            <input type="checkbox" bind:checked={ruleCreateDialogUseChildDir} />
            <span>{$t("rules.tool.create_dialog_child_dir")}</span>
          </label>
          {#if ruleCreateDialogUseChildDir}
            <label class="rule-create-field">
              <span class="rule-create-label">{$t("rules.tool.create_dialog_folder_name")}</span>
              <input
                bind:value={ruleCreateDialogChildDirName}
                placeholder={$t("rules.tool.create_dialog_folder_placeholder")}
                aria-label={$t("rules.tool.create_dialog_folder_name")}
              />
            </label>
          {/if}
        {/if}

        {#if ruleCreateDraftPath}
          <div class="rule-create-target">{ruleCreateDraftPath}</div>
        {/if}
        {#if ruleCreateValidationMessage}
          <div class="rule-create-validation">{ruleCreateValidationMessage}</div>
        {/if}
      </div>
      <div class="modal-footer rule-create-footer">
        <button type="button" class="file-viewer-btn" onclick={closeRuleCreateDialog}>
          {$t("rules.editor.cancel")}
        </button>
        <button type="button" class="file-viewer-primary-btn" onclick={createSelectedToolRule} disabled={!canCreateSelectedToolRuleDraft}>
          {$t("rules.tool.create_dialog_confirm")}
        </button>
      </div>
    </div>
  </div>
{/if}
<ConfirmDialog bind:this={ruleFileConfirmDialog} layer="nested" />
{#if showRulesInjectPreview}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onmousedown={(e) => { if (e.target === e.currentTarget) closeRulesInjectPreview(); }}>
    <div class="modal inject-preview-modal" onmousedown={(e) => e.stopPropagation()}>
      <div class="modal-header inject-preview-header">
        <div class="modal-title">{$t("rules.inject.preview_title")}</div>
        <button type="button" class="icon-btn" aria-label={$t("rules.editor.close")} onclick={closeRulesInjectPreview} disabled={rulesInjectPreviewBusy}>
          <X size={18} strokeWidth={1.8} />
        </button>
      </div>
      <div class="modal-body inject-preview-body">
        <section class="inject-preview-section">
          <div class="inject-preview-section-head">
            <span class="inject-preview-section-title">{$t("rules.inject.preview_changes")}</span>
            <span class="inject-preview-section-note">{$t("rules.inject.preview_change_note")}</span>
          </div>
          {#if rulesInjectPreview.changes.length === 0}
            <div class="inject-preview-empty">{$t("rules.inject.preview_no_changes")}</div>
          {:else}
            <div class="inject-preview-list">
              {#each rulesInjectPreview.changes as item}
                <div class="inject-preview-row">
                  <span class="inject-preview-tool">{item.toolName}</span>
                  <span class="inject-preview-path">{item.path}</span>
                  {#if item.willCreate}
                    <span class="inject-preview-reason">{$t("rules.inject.preview_will_create")}</span>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </section>
        {#if rulesInjectPreview.exceptions.length > 0}
          <section class="inject-preview-section">
            <div class="inject-preview-section-head">
              <span class="inject-preview-section-title">{$t("rules.inject.preview_exceptions")}</span>
            </div>
            <div class="inject-preview-list inject-preview-list--exception">
              {#each rulesInjectPreview.exceptions as item}
                <div class="inject-preview-row inject-preview-row--exception">
                  <span class="inject-preview-tool">{item.toolName}</span>
                  <span class="inject-preview-path">{item.path}</span>
                  <span class="inject-preview-reason">{item.reason}</span>
                </div>
              {/each}
            </div>
          </section>
        {/if}
      </div>
      <div class="modal-footer inject-confirm-actions">
        <button class="btn btn-cancel" onclick={closeRulesInjectPreview} disabled={rulesInjectPreviewBusy}>
          {$t("confirm.cancel")}
        </button>
        <button
          type="button"
          class="btn btn-primary"
          onclick={confirmRulesInjectPreview}
          disabled={rulesInjectPreviewBusy || (rulesInjectPreview.changes.length === 0 && rulesInjectPreview.exceptions.length === 0)}
        >
          <Check size={14}/> {$t(rulesInjectPreviewBusy ? "global.msg.injecting" : "rules.issue.inject")}
        </button>
      </div>
    </div>
  </div>
{/if}
{#if inspectManagedTarget}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onmousedown={(e) => { if (e.target === e.currentTarget) closeManagedInspect(); }}>
    <div class="managed-inspect-modal" onmousedown={(e) => e.stopPropagation()}>
      <div class="crm-header">
        <div class="crm-header-left">
          <div class="crm-icon-box"><Eye size={18} strokeWidth={1.8} /></div>
          <div class="crm-meta">
            <div class="crm-title-row">
              <span class="crm-title">{$t("rules.managed.inspect_title")}</span>
            </div>
            <div class="managed-inspect-subtitle">
              {inspectManagedTarget.tool_name || inspectManagedTarget.tool_id} · {targetReasonLabel(inspectManagedTarget)}
            </div>
          </div>
        </div>
        <button type="button" class="icon-btn" aria-label={$t("rules.editor.close")} onclick={closeManagedInspect}>
          <X size={18} strokeWidth={1.8} />
        </button>
      </div>
      <div class="managed-inspect-body">
        {#if inspectManagedDiff}
          <div class="diff-content managed-diff-content">
            <div class="diff-header">
              <span class="diff-label">{inspectManagedDiff.left_label}</span>
              <span class="diff-label">{inspectManagedDiff.right_label}</span>
            </div>
            <div class="diff-lines">
              {#each inspectManagedDiff.changes as line}
                <div class="diff-line {line.tag}">
                  <span class="diff-sign">{line.tag === "delete" ? "-" : line.tag === "insert" ? "+" : " "}</span>
                  <span class="diff-text">{line.content}</span>
                </div>
              {/each}
            </div>
          </div>
        {:else}
          <div class="empty">{$t("common.loading")}</div>
        {/if}
      </div>
    </div>
  </div>
{/if}
{#if leaveManagedContext}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onmousedown={(e) => { if (e.target === e.currentTarget) closeLeaveManaged(); }}>
    <div class="inject-confirm-modal" onmousedown={(e) => e.stopPropagation()}>
      <div class="inject-confirm-icon"><Unlink size={24} /></div>
      <div class="inject-confirm-title">{$t("rules.managed.leave_title")}</div>
      <div class="inject-confirm-desc">
        {$t("rules.managed.leave_desc", { tool: leaveManagedContext.target?.tool_name || leaveManagedContext.target?.tool_id || "" })}
      </div>
      {#if leaveManagedContext.target?.classification === "drifted"}
        <div class="inject-confirm-sub">{$t("rules.managed.leave_drift_notice")}</div>
      {/if}
      <div class="inject-confirm-actions inject-confirm-actions--wrap">
        <button class="btn btn-cancel" onclick={closeLeaveManaged} disabled={leaveManagedBusy}>
          {$t("confirm.cancel")}
        </button>
        <button type="button" class="btn btn-secondary" onclick={() => confirmLeaveManaged(false)} disabled={leaveManagedBusy}>
          {$t("rules.managed.keep_content")}
        </button>
        <button type="button" class="btn btn-primary" onclick={() => confirmLeaveManaged(true)} disabled={leaveManagedBusy}>
          {$t("rules.managed.remove_block")}
        </button>
      </div>
    </div>
  </div>
{/if}
<svelte:window onpointerdown={handleToolRuleSearchOutsidePointerDown} onkeydown={(e) => {
  if (e.key !== "Escape") return;
  if (showRulesInjectPreview) { closeRulesInjectPreview(); return; }
  if (inspectManagedTarget) { closeManagedInspect(); return; }
  if (leaveManagedContext) { closeLeaveManaged(); return; }
  if (ruleCreateDialogOpen) { closeRuleCreateDialog(); return; }
  if (rulesTab === "tool" && selectedToolRule) {
    if (isSingleToolRuleEditing) {
      cancelSingleToolRuleEdit();
      return;
    }
    closeToolRuleDetail();
    return;
  }
  if (showRuleEditor) {
    if (isCrmContentEditing) {
      if (editingDefaultRule) {
        cancelCrmContentEdit();
        return;
      }
      closeRuleEditor();
      return;
    }
    closeRuleEditor();
    return;
  }
  if (searchOpen) { searchOpen = false; globalSearchQuery = ""; toolRuleSearchResultsOpen = false; return; }
}} />

<style>
  /* Match app.css control height — 与顶栏 36px 控件对齐；固定高度避免「保存 / 注入」因图标基线看起来一高一矮 */
  .btn { display: inline-flex; align-items: center; justify-content: center; gap: 6px; font-size: 12px; min-height: var(--control-height); padding: 0 14px; border: none; border-radius: 8px; cursor: pointer; transition: background 0.15s, color 0.15s, opacity 0.15s; font-weight: 500; box-sizing: border-box; line-height: 1; }
  .btn:hover { box-shadow: none; }
  .btn-cancel { background: var(--bg-subtle); color: var(--color-text-muted); border: none; }
  .btn-cancel:hover { background: var(--bg-hover); color: var(--color-text-main); }

  /* Tabs — defined in app.css (level1-tabs, l1-tab, level2-tabs, l2-tab) */

  /* Common Rules */
  .rules-content-shell {
    flex: 1 1 0;
    min-height: 0;
    max-width: none;
    display: flex;
    flex-direction: column;
    overflow: visible;
  }
  .rules-scroll-content {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
    padding-bottom: 20px;
  }
  .rules-pinned-toolbar {
    position: relative;
    z-index: 3;
    gap: 10px;
    padding-top: 14px;
    max-height: none;
    overflow: visible;
  }
  .rules-status-band {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: min(46vh, 420px);
    overflow-y: auto;
  }
  .common-rule-card {
    flex: 1 1 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .tool-rule-single-card {
    flex: 1 1 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .rule-create-modal {
    width: min(520px, calc(100vw - 48px));
  }
  .rule-create-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .rule-create-field {
    display: grid;
    grid-template-columns: 88px minmax(0, 1fr);
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .rule-create-label {
    color: var(--color-text-muted);
    font-size: 11px;
    font-weight: 600;
    line-height: 1.2;
  }
  .rule-create-field input,
  .rule-create-field select,
  .rule-create-static {
    width: 100%;
    height: 34px;
    min-width: 0;
    box-sizing: border-box;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-card);
    color: var(--color-text-main);
    padding: 0 10px;
    font-size: 12px;
    line-height: 32px;
    outline: none;
    box-shadow: none;
  }
  .rule-create-field select {
    appearance: none;
    -webkit-appearance: none;
    padding-right: 30px;
    background-image:
      linear-gradient(45deg, transparent 50%, var(--color-text-muted) 50%),
      linear-gradient(135deg, var(--color-text-muted) 50%, transparent 50%);
    background-position:
      calc(100% - 16px) 14px,
      calc(100% - 11px) 14px;
    background-size: 5px 5px, 5px 5px;
    background-repeat: no-repeat;
  }
  .rule-create-field select:disabled {
    opacity: 0.78;
    cursor: default;
  }
  .rule-create-field input:focus,
  .rule-create-field select:focus {
    border-color: var(--border-active);
  }
  .rule-create-static,
  .rule-create-target {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rule-create-static {
    color: var(--color-text-main);
  }
  .rule-create-checkbox {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 28px;
    padding-left: 100px;
    color: var(--color-text-muted);
    font-size: 12px;
  }
  .rule-create-checkbox input {
    margin: 0;
  }
  .rule-create-validation {
    box-sizing: border-box;
    border-radius: 8px;
    padding: 8px 10px;
    font-size: 12px;
    line-height: 1.45;
  }
  .rule-create-validation {
    color: #f59e0b;
    background: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.24);
  }
  .rule-create-target {
    margin-left: 100px;
    color: var(--color-text-muted);
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 11px;
    line-height: 1.5;
  }
  .rule-create-footer .file-viewer-btn,
  .rule-create-footer .file-viewer-primary-btn {
    min-height: var(--control-height);
    padding: 0 14px;
  }
  .tool-rule-empty {
    flex: 1 1 0;
    min-height: 180px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px;
    border: 1px dashed var(--border-color);
    border-radius: 10px;
    color: var(--color-text-muted);
    text-align: center;
    background: var(--bg-card);
    box-sizing: border-box;
  }
  .tool-rule-empty__title {
    font-size: 13px;
    font-weight: 650;
    color: var(--color-text-main);
  }
  .tool-rule-empty__desc {
    max-width: 360px;
    font-size: 12px;
    line-height: 1.5;
  }
  .pending-inject-inline {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    box-sizing: border-box;
    margin-bottom: 0;
  }
  .pending-inject-inline--band {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 10px 12px;
  }
  .pending-inject-inline__left {
    display: flex;
    flex-direction: row;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    min-width: 0;
  }
  .pending-inject-hint {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: #f59e0b;
    background: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.24);
    border-radius: 8px;
    padding: 4px 8px;
    flex-shrink: 0;
  }
  .pending-inject-meta {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--color-text-muted);
    opacity: 0.82;
    min-width: 0;
    line-height: 1.35;
  }
  .pending-inject-inline__btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex-shrink: 0;
  }
  .pending-inject-inline__actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .rule-issue-details {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: min(52vh, 460px);
    overflow-y: auto;
    padding-right: 2px;
  }
  .rule-issue-target {
    flex: 0 0 auto;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-elevated-subtle);
    overflow: hidden;
  }
  .rule-issue-target__header {
    width: 100%;
    min-height: 34px;
    display: grid;
    grid-template-columns: 18px minmax(82px, 140px) 88px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    border: none;
    background: transparent;
    color: var(--color-text-main);
    text-align: left;
    cursor: pointer;
  }
  .rule-issue-target__header--static {
    cursor: default;
  }
  .rule-issue-target__header:not(.rule-issue-target__header--static):hover,
  .rule-issue-target__header--open {
    background: var(--bg-hover);
  }
  .rule-issue-target__chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-muted);
    transition: transform 0.15s ease;
  }
  .rule-issue-target__chevron--open {
    transform: rotate(90deg);
  }
  .rule-issue-target__chevron--placeholder {
    visibility: hidden;
  }
  .rule-issue-target__name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 700;
  }
  .rule-issue-target__reason {
    flex-shrink: 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11px;
    color: #f59e0b;
  }
  .rule-issue-target__path {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 11px;
    color: var(--color-text-muted);
  }
  .rule-issue-target__body {
    border-top: 1px solid var(--border-color);
    background: var(--bg-card);
  }
  .rule-issue-target__body .rule-issue-diff-content {
    border-radius: 0;
    border: none;
    background: transparent;
    overflow: visible;
  }
  .rule-issue-target__body .rule-issue-diff-lines {
    background: var(--bg-card);
    border-radius: 0;
    max-height: none;
    overflow-y: visible;
    overflow-x: hidden;
  }
  .rule-issue-diff-lines .diff-line {
    padding: 1px 0;
    width: 100%;
    box-sizing: border-box;
  }
  .rule-issue-diff-lines .diff-sign {
    width: 26px;
    text-align: center;
    padding-left: 0;
  }
  .rule-issue-diff-lines .diff-text {
    min-width: 0;
    padding-right: 8px;
  }
  .rule-issue-diff-lines .diff-fold {
    color: var(--color-text-muted);
    background: var(--bg-elevated-subtle);
  }
  .rule-issue-diff-lines .diff-fold .diff-sign {
    display: none;
  }
  .rule-issue-diff-lines .diff-fold .diff-text {
    padding-left: 26px;
  }
  .rule-issue-diff-lines .diff-fold .diff-text {
    opacity: 0.75;
  }
  .rule-issue-diff-empty {
    min-height: 38px;
    display: flex;
    align-items: center;
    padding: 0 14px;
    font-size: 12px;
    color: var(--color-text-muted);
  }
  .rule-issue-detail-text {
    padding: 10px 14px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--color-text-muted);
  }
  .managed-state-inline {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 100%;
    box-sizing: border-box;
  }
  .managed-state-inline__top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
  }
  .managed-state-hint {
    color: #f59e0b;
  }
  .managed-target-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .managed-target-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 28px;
    padding: 3px 6px 3px 8px;
    border-radius: 6px;
    border: 1px solid var(--border-color);
    background: var(--bg-elevated-subtle);
    font-size: 11px;
    color: var(--color-text-main);
  }
  .managed-target-chip__name {
    font-weight: 600;
  }
  .managed-target-chip__reason {
    color: var(--color-text-muted);
  }
  .managed-target-chip__action {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    min-height: 22px;
    padding: 0 6px;
    border: 1px solid var(--border-color);
    border-radius: 5px;
    background: var(--bg-card);
    color: var(--color-text-main);
    cursor: pointer;
    font-size: 11px;
  }
  .managed-target-chip__action:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .inject-confirm-modal { max-width: 420px; width: 90%; text-align: left; }
  .inject-confirm-header { justify-content: flex-start; }
  .inject-confirm-body { display: flex; flex-direction: column; align-items: stretch; gap: 8px; }
  .inject-confirm-icon { width: 40px; height: 40px; border-radius: 8px; background: rgba(74,222,128,0.12); color: #4ade80; display: flex; align-items: center; justify-content: center; margin-bottom: 2px; }
  .inject-confirm-desc { font-size: 13px; color: var(--color-text-muted); text-align: left; align-self: stretch; line-height: 1.5; }
  .inject-confirm-sub { font-size: 12px; color: var(--color-text-muted); margin-bottom: 6px; text-align: left; align-self: stretch; line-height: 1.45; padding: 8px 10px; background: rgba(250, 204, 21, 0.08); border-radius: 8px; border: 1px solid rgba(250, 204, 21, 0.2); }
  .inject-tool-list { width: 100%; display: flex; flex-wrap: wrap; gap: 8px; box-sizing: border-box; padding: 8px 0; }
  .inject-tool-list-title { width: 100%; text-align: left; font-size: 11px; font-weight: 600; color: var(--color-text-main); }
  .inject-tool-chip { font-size: 11px; color: var(--color-text-main); background: var(--bg-elevated-subtle); border: 1px solid var(--border-color); border-radius: 4px; padding: 4px 8px; }
  .inject-tool-empty { font-size: 11px; color: var(--color-text-muted); }
  .inject-confirm-actions { display: flex; gap: 12px; justify-content: flex-end; }
  .inject-confirm-actions--wrap { flex-wrap: wrap; }
  .inject-preview-modal {
    width: min(640px, 92vw);
    text-align: left;
  }
  .inject-preview-header {
    justify-content: space-between;
  }
  .inject-preview-body {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .inject-preview-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .inject-preview-section-head {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
  }
  .inject-preview-section-title {
    flex-shrink: 0;
    font-size: 12px;
    font-weight: 700;
    color: var(--color-text-main);
  }
  .inject-preview-section-note {
    min-width: 0;
    font-size: 11px;
    line-height: 1.4;
    color: var(--color-text-muted);
  }
  .inject-preview-list {
    display: flex;
    flex-direction: column;
    gap: 0;
    padding: 10px 14px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-elevated-subtle);
    overflow: hidden;
  }
  .inject-preview-list--exception {
    background: rgba(245, 158, 11, 0.08);
    border-color: rgba(245, 158, 11, 0.24);
  }
  .inject-preview-row {
    display: grid;
    grid-template-columns: minmax(88px, 128px) minmax(0, 1fr);
    gap: 10px;
    align-items: center;
    min-height: 30px;
    padding: 5px 0;
  }
  .inject-preview-row + .inject-preview-row {
    border-top: 1px solid color-mix(in srgb, var(--border-color) 70%, transparent);
  }
  .inject-preview-row--exception {
    grid-template-columns: minmax(88px, 128px) minmax(0, 1fr) auto;
  }
  .inject-preview-tool {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 700;
    color: var(--color-text-main);
  }
  .inject-preview-path {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 11px;
    color: var(--color-text-muted);
  }
  .inject-preview-reason {
    flex-shrink: 0;
    font-size: 11px;
    color: #f59e0b;
  }
  .inject-preview-empty {
    min-height: 34px;
    display: flex;
    align-items: center;
    padding: 0 10px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    color: var(--color-text-muted);
    background: var(--bg-elevated-subtle);
    font-size: 12px;
  }
  .inject-msg { font-size: 11px; color: #60a5fa; padding: 6px 10px; background: rgba(96,165,250,0.08); border-radius: 8px; margin-bottom: 10px; }
  .inject-msg--band { margin-bottom: 0; }
  .managed-inspect-modal {
    background: var(--bg-modal);
    border: 1px solid var(--border-modal);
    border-radius: 16px;
    width: min(860px, 92vw);
    max-height: 82vh;
    box-shadow: var(--shadow-modal);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .managed-inspect-subtitle {
    font-size: 12px;
    color: var(--color-text-muted);
  }
  .managed-inspect-body {
    padding: 0 20px 20px;
    min-height: 0;
    overflow: auto;
  }
  .managed-diff-content {
    border: 1px solid var(--border-color);
  }
  .diff-content { background: var(--bg-card); border-radius: 8px; overflow: hidden; }
  .diff-header { display: flex; justify-content: space-between; padding: 10px 16px; border-bottom: 1px solid var(--border-color); }
  .diff-label { font-size: 11px; color: var(--color-text-muted); }
  .diff-lines { font-family: "SF Mono", "Fira Code", monospace; font-size: 12px; line-height: 1.6; max-height: 55vh; overflow-y: auto; }
  .diff-line { display: flex; padding: 1px 16px; }
  .diff-line.delete { background: rgba(248,113,113,0.08); color: #f87171; }
  .diff-line.insert { background: rgba(74,222,128,0.08); color: #4ade80; }
  .diff-line.equal { color: var(--color-text-muted); }
  .diff-sign { width: 16px; flex-shrink: 0; user-select: none; }
  .diff-text { white-space: pre-wrap; word-break: break-all; }

  /* 页头 .header-actions 间距见 app.css --header-actions-gap（勿在此用 gap:10 覆盖） */
  /* Refresh btn & spin — defined in app.css */
  :global(.lucide) { display: block; flex-shrink: 0; }
  @keyframes spin { 100% { transform: rotate(360deg); } }
  /* Overlay */


  /* Custom rules section */
  .custom-rules-section {
    flex: 1 1 0;
    min-height: 0;
    margin-top: 0;
    padding-top: 0;
    display: flex;
    flex-direction: column;
  }
  .custom-rules-empty { font-size: 12px; color: var(--color-text-muted); opacity: 0.5; padding: 20px 0; text-align: center; }
  .custom-rule-detail {
    flex: 1 1 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .crm-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 20px 20px 16px 20px;
    flex-shrink: 0;
  }
  .crm-header-left { display: flex; gap: 16px; min-width: 0; align-items: flex-start; }
  /* 与 RuleEditor 工具规则弹窗 .icon-box 一致的中性色块 */
  .crm-icon-box {
    width: 40px;
    height: 40px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: var(--bg-hover);
    color: var(--color-text-muted);
  }
  .crm-meta { min-width: 0; display: flex; flex-direction: column; gap: 6px; margin-top: 2px; }
  .crm-title-row { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .crm-title {
    font-size: 16px;
    font-weight: 600;
    color: var(--color-text-main);
    font-family: "Space Grotesk", sans-serif;
    display: block;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .crm-title-ph { color: var(--color-text-muted); opacity: 0.65; font-weight: 500; }
  .crm-title-input {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-card);
    color: var(--color-text-main);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 8px 12px;
    font-size: 16px;
    font-weight: 600;
    font-family: "Space Grotesk", sans-serif;
    outline: none;
    transition: border-color 0.2s, box-shadow 0.2s;
  }
  .crm-title-input:focus {
    border-color: var(--border-active);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--border-color) 70%, transparent);
    outline: none;
  }
  .file-viewer-title-input {
    max-width: 320px;
    min-height: 32px;
    padding: 5px 10px;
    font-size: 13px;
  }
  .custom-rule-detail .file-viewer-title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    font-weight: 600;
    color: var(--color-text-main);
  }
  .file-viewer-inline-status {
    font-size: 11px;
    color: var(--color-text-muted);
    white-space: nowrap;
  }
  .custom-rule-edit-body {
    flex: 1 1 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .crm-fields-well.custom-rule-targets-panel {
    flex: 0 0 auto;
    min-height: 0;
    margin: 0 18px 14px;
    overflow: hidden;
    box-shadow: none;
    border: 0;
    border-top: 1px solid var(--border-color);
    border-radius: 0;
    background: transparent;
    padding: 10px 0 0;
    gap: 8px;
  }
  .crm-targets-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .crm-targets-header .crm-section-label {
    margin-bottom: 0;
  }
  .crm-targets-header .file-viewer-btn {
    flex-shrink: 0;
  }
  .crm-fields-well {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    background: var(--bg-well);
    border-radius: 12px;
    border: 1px solid var(--border-color);
    box-shadow: inset 0 2px 10px rgba(0, 0, 0, 0.03);
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .crm-section-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--color-text-muted);
    margin-bottom: 8px;
  }
  .crm-targets-section {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 8px;
  }
  .crm-targets {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    max-height: 62px;
    overflow-y: auto;
    padding-right: 4px;
    align-content: flex-start;
  }
  .custom-rule-targets-panel .l2-tab {
    min-height: 28px;
    padding: 0 9px;
    gap: 4px;
    border-radius: 7px;
    font-size: 11px;
  }

  .rule-tree-context-menu {
    position: fixed;
    z-index: 2600;
    width: 190px;
    padding: 6px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-card);
    box-shadow: var(--shadow-popover, var(--shadow-soft));
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .rule-tree-context-menu__item {
    width: 100%;
    min-height: 30px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--color-text-main);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }

  .rule-tree-context-menu__item:hover {
    background: var(--bg-hover);
  }

  .rule-tree-context-menu__item--danger {
    color: #dc2626;
  }

  .rule-tree-context-menu__item--danger:hover {
    background: rgba(248, 113, 113, 0.1);
  }

  .rule-tree-name-modal {
    width: min(420px, calc(100vw - 32px));
  }

  .rule-tree-name-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

</style>
