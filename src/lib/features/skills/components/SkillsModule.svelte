<!-- Purpose: Encapsulate Skills view state and flows while preserving existing user-visible behavior. -->
<script>
  import { onDestroy, onMount, tick } from "svelte";
  import { RefreshCw, Search, X } from "lucide-svelte";
  import BaseCard from "$lib/shared/components/BaseCard.svelte";
  import ModulePerformanceBar from "$lib/shared/diagnostics/ModulePerformanceBar.svelte";
  import ProtectedToolSelector from "$lib/shared/components/ProtectedToolSelector.svelte";
  import SkillViewer from "$lib/features/skills/components/SkillViewer.svelte";
  import { activeTool, activeToolId, managedTools, pendingSubTab } from "$lib/features/tools/index.js";
  import { listSkills } from "$lib/features/skills/api/skills.js";
  import { normalizeSkillStatus } from "$lib/features/skills/domain/skillDomain.js";
  import {
    beginModulePerformanceRun,
    finishAndRecordModulePerformanceRun,
    finishModulePerformanceRun,
    markModulePerformance,
    MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE,
    MODULE_PERFORMANCE_ROLE_INTERACTIVE,
    MODULE_PERFORMANCE_ROLE_VISIBLE,
    setModulePerformanceCounters,
    trackModulePerformanceRequest,
    updateModulePerformanceSummary,
  } from "$lib/shared/diagnostics/modulePerformance.js";
  import {
    DEFAULT_SKILL_INVENTORY_TTL_MS,
    getCachedSkillInventory,
    getSkillInventory,
  } from "$lib/features/skills/queries/skillInventoryQuery.js";
  import { t } from "$lib/shared/i18n/index.js";
  import { getVisualVerificationState, isVisualVerificationMode } from "$lib/dev/visualVerification/fixtures.js";

  let {
    globalSearchQuery = $bindable(""),
    searchOpen = $bindable(false),
    refreshing = false,
    onRefresh = async () => {},
  } = $props();

  let currentTool = $derived($activeTool);
  let currentToolId = $derived($activeToolId);

  /** @type {any[]} */
  let skills = $state([]);
  /** @type {any[]} */
  let genericSkills = $state([]);
  /** @type {any[]} */
  let skillsOverview = $state([]);
  let skillsTab = $state("overview"); // "overview" | "common" | "tool"

  // Skill viewer
  /** @type {any} */
  let viewingSkill = $state(null);
  let skillViewerInitialTab = $state("content");
  let skillViewerInitialAction = $state("");

  let skillsLoadError = $state("");
  /** @type {string[]} */
  let skillsLoadErrorDetails = $state([]);
  /** @type {Record<string, any[]>} */
  let toolSkillsById = $state({});
  /** @type {Map<string, Promise<any[]>>} */
  const toolSkillsInflight = new Map();
  const STALE_BACKGROUND_REFRESH_DELAY_MS = 300;
  const SOURCE_WRITE_BACKGROUND_REFRESH_DELAY_MS = 750;
  let skillRefreshGeneration = 0;
  /** @type {ReturnType<typeof setTimeout> | null} */
  let sourceWriteBackgroundRefreshTimer = null;

  let globalRefreshMsg = $state("");
  let skillListSearchInput = $state(/** @type {HTMLInputElement | undefined} */ (undefined));
  /** @type {any} */
  let skillViewerRef = $state(null);

  onMount(() => {
    void (async () => {
      await reloadSkillsPageData({ reason: "entry" });
      if (isVisualVerificationMode()) {
        const visualState = getVisualVerificationState();
        if (visualState === "skill-detail-unconnected-connect-action") {
          const localSkill = skillsOverview.find((/** @type {any} */ item) => item.name === "visual-review");
          if (localSkill) openSkillViewer(localSkill, "install");
        }
        if (visualState === "skill-detail-shared-link") {
          const sharedSkill = skillsOverview.find((/** @type {any} */ item) => item.name === "shared-design");
          if (sharedSkill) openSkillViewer(sharedSkill, "install");
        }
        if (visualState === "skill-detail-shared-picker") {
          const sharedSkill = skillsOverview.find((/** @type {any} */ item) => item.name === "shared-picker");
          if (sharedSkill) openSkillViewer(sharedSkill, "install");
        }
        if (visualState === "skill-detail-content-source-selector") {
          const independentSkill = skillsOverview.find((/** @type {any} */ item) => item.name === "tool-config");
          if (independentSkill) openSkillViewer(independentSkill, "content");
        }
        if (visualState === "skill-detail-one-file") {
          const oneFileSkill = skillsOverview.find((/** @type {any} */ item) => item.name === "tool-config");
          if (oneFileSkill) openSkillViewer(oneFileSkill, "content");
        }
        if (visualState === "skill-detail-directory") {
          const directorySkill = skillsOverview.find((/** @type {any} */ item) => item.name === "visual-review");
          if (directorySkill) openSkillViewer(directorySkill, "content");
        }
        if (visualState === "skill-metadata-drift") {
          setSkillsTab("tool");
        }
      }
    })();
  });

  onDestroy(() => {
    if (sourceWriteBackgroundRefreshTimer) {
      clearTimeout(sourceWriteBackgroundRefreshTimer);
      sourceWriteBackgroundRefreshTimer = null;
    }
    globalSearchQuery = "";
    searchOpen = false;
  });

  export async function reload() {
    await reloadSkillsPageData({ forceInventory: true, reason: "manual-refresh" });
  }

  /** @param {string} status */
  function mapInventoryStatusToMode(status) {
    const normalized = normalizeSkillStatus(status);
    const symlinkStatuses = new Set(["variantInstalledSymlink", "brokenSymlink"]);
    return symlinkStatuses.has(normalized) ? "symlink" : "copy";
  }

  /** @param {string | null | undefined} status */
  function isSkillStatusInstalledLike(status) {
    const normalized = normalizeSkillStatus(status);
    return Boolean(normalized)
      && normalized !== "notInstalled"
      && normalized !== "variantNotInstalled"
      && normalized !== "noVariant";
  }

  function currentManagedToolIdSet() {
    return new Set($managedTools.map((/** @type {any} */ tool) => String(tool.id)));
  }

  /** @param {any[]} statuses */
  function filterStatusesForManagedTools(statuses) {
    const managedIds = currentManagedToolIdSet();
    return (statuses || []).filter((/** @type {any} */ status) => {
      const toolId = status?.tool_id || status?.toolId;
      return toolId && managedIds.has(String(toolId));
    });
  }

  /** @param {any} status */
  function contentPathForInventoryStatus(status) {
    const pathOrigin = status?.path_origin || status?.pathOrigin;
    const targetPath = status?.symlink_target || status?.symlinkTarget || status?.target_path || status?.targetPath || "";
    if (pathOrigin === "generic" && targetPath) return targetPath;
    return status?.path || targetPath || "";
  }

  /** @param {any[]} statuses */
  function primaryPathForToolStatuses(statuses) {
    const status = statuses.find((/** @type {any} */ item) =>
      isSkillStatusInstalledLike(item?.status)
        && typeof contentPathForInventoryStatus(item) === "string"
        && contentPathForInventoryStatus(item).length > 0
    );
    return contentPathForInventoryStatus(status) || "";
  }

  /** @param {Array<any>} entries */
  function mapInventoryToOverview(entries) {
    const sourceSkills = Array.isArray(entries) ? entries : [];
    return sourceSkills.map((/** @type {any} */ entry) => {
      const toolStatuses = filterStatusesForManagedTools(entry.tool_statuses || entry.toolStatuses || []);
      const installed_in = toolStatuses
        .filter((/** @type {any} */ ts) => {
          const hasPath = typeof ts?.path === "string" && ts.path.length > 0;
          return isSkillStatusInstalledLike(ts?.status) && hasPath;
        })
        .map((/** @type {any} */ ts) => ({
          tool_id: ts.tool_id || ts.toolId,
          mode: mapInventoryStatusToMode(ts.status),
          path: ts.path,
          target_path: ts.symlink_target || ts.symlinkTarget || null,
        }));

      return {
        name: entry.name,
        display_name: entry.display_name || entry.name,
        description: entry.description || "",
        path: primaryPathForToolStatuses(toolStatuses),
        installed_in,
        tool_statuses: toolStatuses,
        has_variants: entry.hasVariants || entry.has_variants || [],
        variant_paths: entry.variantPaths || entry.variant_paths || {},
        package: entry.package || null,
      };
    }).filter(inventoryEntryHasSource);
  }

  /** @param {any} item */
  function hasSharedSkillSource(item) {
    const statuses = item?.tool_statuses || item?.toolStatuses || [];
    return statuses.some((/** @type {any} */ status) => isSharedBackedStatus(status));
  }

  /** @param {any} status */
  function isSharedBackedStatus(status) {
    const pathOrigin = status?.path_origin || status?.pathOrigin;
    if (pathOrigin === "generic") return true;
    const targetPath = status?.symlink_target || status?.symlinkTarget || status?.target_path || status?.targetPath;
    return typeof targetPath === "string" && targetPath.length > 0 && targetPath !== status?.path;
  }

  /** @param {any[]} overview */
  function deriveSharedSkillsFromOverview(overview) {
    return overview.filter(hasSharedSkillSource).sort(compareByName);
  }

  /** @param {any} entry */
  function inventoryEntryHasSource(entry) {
    const statuses = entry?.tool_statuses || entry?.toolStatuses || [];
    return statuses.some((/** @type {any} */ status) =>
      isSkillStatusInstalledLike(status?.status)
        && typeof status?.path === "string"
        && status.path.length > 0
    );
  }

  /** @param {any} entry */
  function applySingleInventoryEntry(entry) {
    if (!entry?.name) return null;
    const mapped = mapInventoryToOverview([entry])[0] || null;
    if (!mapped) {
      skillsOverview = skillsOverview.filter((/** @type {any} */ item) => item.name !== entry.name);
      genericSkills = deriveSharedSkillsFromOverview(skillsOverview);
      return null;
    }
    const replaced = skillsOverview.some((/** @type {any} */ item) => item.name === mapped.name);
    skillsOverview = (replaced
      ? skillsOverview.map((/** @type {any} */ item) => item.name === mapped.name ? mapped : item)
      : [...skillsOverview, mapped]
    ).sort(compareByName);
    genericSkills = deriveSharedSkillsFromOverview(skillsOverview);
    if (viewingSkill?.name === mapped.name) {
      viewingSkill = {
        ...(/** @type {any} */ (viewingSkill)),
        display_name: viewingSkill.display_name || mapped.display_name || mapped.name,
        path: mapped.path || "",
        installed_in: mapped.installed_in || [],
        tool_statuses: mapped.tool_statuses || [],
        has_variants: mapped.has_variants || viewingSkill.has_variants || [],
        variant_paths: mapped.variant_paths || viewingSkill.variant_paths || {},
        package: mapped.package || viewingSkill.package || null,
      };
    }
    return mapped;
  }

  function scheduleSourceWriteBackgroundRefresh() {
    if (sourceWriteBackgroundRefreshTimer) clearTimeout(sourceWriteBackgroundRefreshTimer);
    sourceWriteBackgroundRefreshTimer = setTimeout(() => {
      sourceWriteBackgroundRefreshTimer = null;
      invalidateToolSkillsCache();
      void refreshSkillsData({ forceInventory: true }).catch(() => {});
    }, SOURCE_WRITE_BACKGROUND_REFRESH_DELAY_MS);
  }

  /**
   * @param {any} a
   * @param {any} b
   */
  function compareByName(a, b) {
    return String(a?.name || "").localeCompare(String(b?.name || ""));
  }

  /** @param {unknown} err */
  function setSkillsLoadError(err) {
    const rawMessage = err instanceof Error ? err.message : String(err ?? "");
    const normalizedLines = rawMessage
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
    const summary = normalizedLines[0] || rawMessage || $t("skills.load_failed_unknown");
    skillsLoadError = $t("skills.load_failed", { err: summary });
    skillsLoadErrorDetails = normalizedLines.slice(1);
  }

  /** @param {any} skill */
  function openSkillViewer(skill, initialTab = "content", initialAction = "") {
    skillViewerInitialTab = initialTab || "content";
    skillViewerInitialAction = initialAction || "";
    const overview = skillsOverview.find((item) => item.name === skill.name);
    if (!overview) {
      viewingSkill = skill;
      return;
    }
    viewingSkill = {
      ...skill,
      display_name: skill.display_name || overview.display_name || skill.name,
      path: skill.path || overview.path || "",
      installed_in: overview.installed_in || skill.installed_in || [],
      tool_statuses: overview.tool_statuses || [],
      has_variants: overview.has_variants || skill.has_variants || [],
      variant_paths: overview.variant_paths || skill.variant_paths || {},
      package: overview.package || skill.package || null,
    };
  }

  function closeSkillViewer() {
    viewingSkill = null;
    skillViewerInitialTab = "content";
    skillViewerInitialAction = "";
  }

  export function focusModuleSearch() {
    if (viewingSkill && skillViewerRef && typeof skillViewerRef.focusModuleSearch === "function") {
      skillViewerRef.focusModuleSearch();
      return;
    }
    searchOpen = true;
    tick().then(() => skillListSearchInput?.focus());
  }

  /** @param {string} reason @param {string} [view] */
  function startSkillPerformanceRun(reason, view = skillsTab) {
    return beginModulePerformanceRun({
      module: "skills",
      view,
      reason,
      counters: {
        overviewSkills: skillsOverview.length,
        sharedSkills: genericSkills.length,
        toolSkills: skills.length,
      },
    });
  }

  function nextSkillRefreshGeneration() {
    skillRefreshGeneration += 1;
    return skillRefreshGeneration;
  }

  /**
   * @param {"overview" | "common" | "tool"} tab
   */
  function setSkillsTab(tab) {
    if (tab === skillsTab) return;
    const run = startSkillPerformanceRun("tab-switch", tab);
    skillsTab = tab;
    markModulePerformance(run, "skills-tab-visible", { role: MODULE_PERFORMANCE_ROLE_VISIBLE });
    markModulePerformance(run, "skills-tab-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
    markModulePerformance(run, "skills-tab-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
    finishAndRecordModulePerformanceRun(run, "success");
  }

  /** @param {number} generation */
  function isCurrentSkillRefresh(generation) {
    return generation === skillRefreshGeneration;
  }

  /** @param {any} inventory @param {any | null} [run] @param {{ markVisible?: boolean }} [options] */
  function applySkillInventory(inventory, run = null, options = {}) {
    const overview = mapInventoryToOverview(inventory?.skills || []).sort(compareByName);
    skillsOverview = overview;
    genericSkills = deriveSharedSkillsFromOverview(overview);
    setModulePerformanceCounters(run, {
      overviewSkills: skillsOverview.length,
      sharedSkills: genericSkills.length,
      toolSkills: skills.length,
    });
    if (options.markVisible !== false) {
      markModulePerformance(run, "shared-list-derived");
      markModulePerformance(run, "visible-lists-ready", { role: MODULE_PERFORMANCE_ROLE_VISIBLE });
      markModulePerformance(run, "interactive-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
      updateModulePerformanceSummary(run);
    }
  }

  /** @param {any | null} run @param {Promise<unknown>[]} tasks @param {number} generation */
  function finishPerformanceWhenSettled(run, tasks, generation) {
    void Promise.allSettled(tasks).finally(() => {
      if (!isCurrentSkillRefresh(generation)) {
        finishModulePerformanceRun(run, "cancelled");
        return;
      }
      markModulePerformance(run, "refresh-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, run?.status === "partial" ? "partial" : "success");
    });
  }

  /** @param {number} ms */
  function delay(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  /** @param {string | null | undefined} [toolId] */
  function invalidateToolSkillsCache(toolId = null) {
    if (toolId) {
      const next = { ...toolSkillsById };
      delete next[toolId];
      toolSkillsById = next;
      toolSkillsInflight.delete(toolId);
      return;
    }
    toolSkillsById = {};
    toolSkillsInflight.clear();
  }

  /**
   * @param {string | null | undefined} toolId
   * @param {{ force?: boolean, run?: any | null, updateVisible?: boolean }} [options]
   */
  async function loadToolSkills(toolId, options = {}) {
    if (!toolId) return [];
    const updateVisible = options.updateVisible !== false;
    if (!options.force && toolSkillsById[toolId]) {
      if (updateVisible && currentToolId === toolId) skills = toolSkillsById[toolId];
      return toolSkillsById[toolId];
    }
    if (!options.force && toolSkillsInflight.has(toolId)) {
      return toolSkillsInflight.get(toolId) || [];
    }
    const request = trackModulePerformanceRequest(options.run || null, "tool-list", () => listSkills(toolId))
      .then((items) => {
        const sorted = (items || []).sort(compareByName);
        toolSkillsById = { ...toolSkillsById, [toolId]: sorted };
        if (updateVisible && currentToolId === toolId) skills = sorted;
        setModulePerformanceCounters(options.run || null, {
          overviewSkills: skillsOverview.length,
          sharedSkills: genericSkills.length,
          toolSkills: sorted.length,
        });
        return sorted;
      })
      .finally(() => {
        if (toolSkillsInflight.get(toolId) === request) toolSkillsInflight.delete(toolId);
      });
    toolSkillsInflight.set(toolId, request);
    return request;
  }

  /** @param {any | null} [run] @param {{ force?: boolean }} [options] */
  function preloadCurrentToolSkills(run = null, options = {}) {
    const toolId = currentToolId;
    const hasCurrent = toolId && $managedTools.some((/** @type {any} */ tool) => tool.id === toolId);
    if (!hasCurrent) {
      markModulePerformance(run, "tool-list-skipped");
      updateModulePerformanceSummary(run);
      return Promise.resolve([]);
    }
    const request = loadToolSkills(toolId, {
      force: Boolean(options.force),
      run,
      updateVisible: skillsTab === "tool",
    }).then((items) => {
      markModulePerformance(run, "tool-list-ready");
      updateModulePerformanceSummary(run);
      return items;
    }).catch((e) => {
      if (run) run.status = "partial";
      markModulePerformance(run, "tool-list-failed");
      if (skillsTab === "tool") {
        setSkillsLoadError(e);
        skills = [];
      }
      updateModulePerformanceSummary(run);
      return [];
    });
    return request;
  }

  /** @param {{ forceInventory?: boolean }} [options] @param {any | null} [run] */
  async function refreshSkillsData(options = {}, run = null) {
    const generation = nextSkillRefreshGeneration();
    markModulePerformance(run, "requests-start");
    updateModulePerformanceSummary(run);
    const forceInventory = Boolean(options.forceInventory);
    const freshCachedInventory = !forceInventory
      ? getCachedSkillInventory({ ttl: DEFAULT_SKILL_INVENTORY_TTL_MS })
      : null;
    const staleCachedInventory = !forceInventory && !freshCachedInventory
      ? getCachedSkillInventory()
      : null;
    const cachedInventory = freshCachedInventory || staleCachedInventory;
    if (cachedInventory) {
      markModulePerformance(run, "inventory-cache-ready");
      if (staleCachedInventory) markModulePerformance(run, "inventory-stale-cache-ready");
      applySkillInventory(cachedInventory, run);
    }
    skillsLoadError = "";
    skillsLoadErrorDetails = [];

    const needsInventoryRefresh = forceInventory || !freshCachedInventory;
    const shouldDeferBackgroundWork = Boolean(staleCachedInventory && !forceInventory);
    const startInventoryRequest = () => (
      needsInventoryRefresh
        ? trackModulePerformanceRequest(run, "inventory", () => getSkillInventory({ force: forceInventory }))
        : Promise.resolve(cachedInventory)
    );

    let inventory = cachedInventory;
    let backgroundInventoryRefresh = Promise.resolve(cachedInventory);
    if (!cachedInventory || forceInventory) {
      inventory = await startInventoryRequest();
      if (!isCurrentSkillRefresh(generation)) return skillsOverview;
      markModulePerformance(run, "inventory-ready");
      applySkillInventory(inventory, run);
    } else if (staleCachedInventory) {
      markModulePerformance(run, "background-refresh-scheduled");
      backgroundInventoryRefresh = delay(STALE_BACKGROUND_REFRESH_DELAY_MS).then(startInventoryRequest).then((freshInventory) => {
        if (!isCurrentSkillRefresh(generation)) return freshInventory;
        markModulePerformance(run, "inventory-ready");
        applySkillInventory(freshInventory, run, { markVisible: false });
        skillsLoadError = "";
        skillsLoadErrorDetails = [];
        updateModulePerformanceSummary(run);
        return freshInventory;
      }).catch((e) => {
        if (!isCurrentSkillRefresh(generation)) return null;
        if (run) run.status = "partial";
        markModulePerformance(run, "inventory-refresh-failed");
        setSkillsLoadError(e);
        updateModulePerformanceSummary(run);
        return null;
      });
    }
    const preload = (shouldDeferBackgroundWork
      ? delay(STALE_BACKGROUND_REFRESH_DELAY_MS)
      : Promise.resolve()
    ).then(() => {
      if (!isCurrentSkillRefresh(generation)) return [];
      return preloadCurrentToolSkills(run, { force: forceInventory });
    });
    const completionTasks = staleCachedInventory
      ? [backgroundInventoryRefresh, preload]
      : [preload];
    finishPerformanceWhenSettled(run, completionTasks, generation);
    return inventory?.skills || [];
  }

  /** @param {{ forceInventory?: boolean, reason?: string }} [options] */
  async function reloadSkillsPageData(options = {}) {
    const run = startSkillPerformanceRun(options.reason || "refresh");
    if (options.forceInventory) invalidateToolSkillsCache();
    skillsLoadError = "";
    skillsLoadErrorDetails = [];
    const refresh = refreshSkillsData(options, run).then(() => {
      /* Tool-list preload continues in the background after the visible lists update. */
    }).catch((e) => {
      markModulePerformance(run, "refresh-failed", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      setSkillsLoadError(e);
      finishAndRecordModulePerformanceRun(run, "failed");
    });
    await refresh;
  }

  /** @param {any} item */
  function buildSkillBadges(item) {
    const presences = item.installed_in || [];
    return $managedTools.map(tool => {
      const presence = presences.find((/** @type {any} */ p) => p.tool_id === tool.id);
      if (presence) {
        return {
          label: tool.name,
          status: "installed",
          mode: presence.mode,
          toolId: tool.id,
        };
      }
      return { label: tool.name, status: "available", toolId: tool.id };
    });
  }

  /** @param {any} skill */
  function buildGenericSkillBadges(skill) {
    const overviewItem = skillsOverview.find(o => o.name === skill.name);
    if (overviewItem) return buildSkillBadges(overviewItem);
    return $managedTools.map(tool => ({ label: tool.name, status: "available", toolId: tool.id }));
  }

  /**
   * @param {any} skill
   * @param {string | null | undefined} toolId
   */
  function getOverviewToolStatus(skill, toolId) {
    if (!skill || !toolId) return null;
    const overview = skillsOverview.find((item) => item.name === skill.name);
    if (!overview) return null;
    return (overview.tool_statuses || overview.toolStatuses || []).find(
      (/** @type {any} */ ts) => (ts?.tool_id || ts?.toolId) === toolId
    ) || null;
  }

  /**
   * @param {any} skill
   * @param {string | null | undefined} toolId
   */
  function isSharedBackedToolSkill(skill, toolId) {
    if (!skill || !toolId) return false;
    const toolStatus = getOverviewToolStatus(skill, toolId);
    if (isSharedBackedStatus(toolStatus)) return true;
    return skill.tool_id === "generic";
  }

  /**
   * @param {any} skill
   * @param {string | null | undefined} toolId
   */
  function buildToolSkillBadges(skill, toolId) {
    if (isSharedBackedToolSkill(skill, toolId)) {
      return [{ label: $t("skills.viewer.path_origin_generic") }];
    }
    return [];
  }

  /**
   * @param {any} targetSkill
   * @param {string} title
   */
  function buildToolSkillWarningPayload(targetSkill, title) {
    return {
      title,
      onClick: () => openSkillViewer(targetSkill, "install"),
    };
  }

  /**
   * @param {any} skill
   * @param {string | null | undefined} toolId
   */
  function buildToolSkillWarning(skill, toolId) {
    if (!skill || !toolId) return null;
    if (skill.tool_id && skill.tool_id !== toolId) return null;
    const overview = skillsOverview.find((item) => item.name === skill.name);
    if (!overview) {
      return null;
    }
    const toolStatus = getOverviewToolStatus(skill, toolId);
    const sources = toolStatus?.sources || [];
    const abnormalState = toolStatus?.abnormal_state || toolStatus?.abnormalState || "";
    if (abnormalState === "duplicate_sources" || sources.length > 1) {
      return buildToolSkillWarningPayload(overview, $t("skills.card.warning_duplicate_sources"));
    }
    const status = normalizeSkillStatus(toolStatus?.status);
    if (status === "brokenSymlink") {
      return buildToolSkillWarningPayload(overview, $t("skills.card.warning_broken"));
    }
    if (status === "variantDrifted") {
      return buildToolSkillWarningPayload(overview, $t("skills.card.warning_drifted"));
    }
    return null;
  }

  /** @param {any[]} entries */
  function groupToolSkillsForDisplay(entries) {
    const grouped = new Map();
    for (const entry of Array.isArray(entries) ? entries : []) {
      if (!entry?.name || grouped.has(entry.name)) continue;
      const overview = skillsOverview.find((item) => item.name === entry.name);
      grouped.set(entry.name, {
        ...entry,
        display_name: entry.display_name || overview?.display_name || entry.name,
        description: entry.description || overview?.description || "",
        path: entry.path || overview?.path || "",
        tool_statuses: overview?.tool_statuses || entry.tool_statuses || entry.toolStatuses || [],
        package: overview?.package || entry.package || null,
      });
    }
    return Array.from(grouped.values()).sort(compareByName);
  }

  /** @param {string} toolId */
  function selectTool(toolId) {
    activeToolId.set(toolId);
  }

  $effect(() => {
    if ($pendingSubTab === "tool") {
      setSkillsTab("tool");
      pendingSubTab.set(null);
    }
  });

  $effect(() => {
    if ($pendingSubTab === "default") {
      setSkillsTab("overview");
      pendingSubTab.set(null);
    }
  });

  $effect(() => {
    if (skillsTab !== "tool") return;
    const hasCurrent = currentToolId && $managedTools.some((/** @type {any} */ tool) => tool.id === currentToolId);
    if (!hasCurrent && $managedTools[0]) {
      activeToolId.set($managedTools[0].id);
    }
  });

  // 监听工具切换，仅刷新当前工具的 skill 列表
  $effect(() => {
    const toolId = currentToolId; // 同步读取，确保依赖追踪
    const hasCurrent = toolId && $managedTools.some((/** @type {any} */ tool) => tool.id === toolId);
    if (skillsTab === "tool" && hasCurrent) {
      loadToolSkills(toolId, { updateVisible: true }).catch(() => {
        if (currentToolId === toolId) skills = [];
      });
    } else if (skillsTab === "tool" && !hasCurrent) {
      skills = [];
    }
  });

  /** @param {any} item */
  function skillListSearchText(item) {
    const packageMembers = item?.package?.members || [];
    const memberText = packageMembers
      .map((/** @type {any} */ member) => [
        member.name,
        member.display_name || member.displayName,
        member.description,
        member.relative_path || member.relativePath,
      ].filter(Boolean).join(" "))
      .join(" ");
    return [
      item?.display_name || item?.displayName,
      item?.name,
      item?.description,
      item?.path,
      packageNote(item),
      memberText,
      ...(item?.tool_statuses || item?.toolStatuses || []).flatMap((/** @type {any} */ status) => [
        status.tool_name || status.toolName,
        status.tool_id || status.toolId,
        status.path,
        status.path_origin || status.pathOrigin,
      ]),
    ].filter(Boolean).join(" ").toLowerCase();
  }

  /** @param {any} item */
  function isPackageSkill(item) {
    return Boolean(item?.package?.isPackage || item?.package?.is_package);
  }

  /** @param {any} item */
  function packageNote(item) {
    if (!isPackageSkill(item)) return "";
    const count = item.package?.memberCount || item.package?.member_count || item.package?.members?.length || 0;
    return $t("skills.card.package_members", { count });
  }

  let toolSkillsForDisplay = $derived(groupToolSkillsForDisplay(skills));
  let skillListFilterQuery = $derived(searchOpen && !viewingSkill ? globalSearchQuery.trim().toLowerCase() : "");

  /** @param {any[]} items */
  function filterSkillCards(items) {
    if (!skillListFilterQuery) return items;
    return items.filter((/** @type {any} */ item) => skillListSearchText(item).includes(skillListFilterQuery));
  }

  let filteredSkillsOverview = $derived.by(() => filterSkillCards(skillsOverview));
  let filteredGenericSkills = $derived.by(() => filterSkillCards(genericSkills));
  let filteredToolSkillsForDisplay = $derived.by(() => filterSkillCards(toolSkillsForDisplay));

</script>

<svelte:window onkeydown={(e) => {
  if (e.key !== "Escape") return;
  if (viewingSkill) { closeSkillViewer(); return; }
  if (searchOpen) { searchOpen = false; globalSearchQuery = ""; return; }
}} />

<div class="view-panel">
  <div class="view-fixed-header">
    <div
      class="view-header management-header-row"
      class:management-header-row--search-open={searchOpen}
      data-tauri-drag-region
    >
      <h2 class="management-header-title" data-tauri-drag-region>{$t("skills.title")}</h2>
      <div class="header-actions management-header-actions">
        {#if searchOpen}
          <div class="workspace-search-anchor">
            <div class="search-input-wrap">
              <span class="search-input-wrap__icon" aria-hidden="true"><Search size={13} strokeWidth={1.8} /></span>
              <input
                bind:this={skillListSearchInput}
                class="search-input"
                type="text"
                placeholder={$t("module_search.placeholder_skills")}
                bind:value={globalSearchQuery}
              />
              <button
                class="icon-btn-tiny"
                onclick={() => { searchOpen = false; globalSearchQuery = ""; }}
                aria-label={$t("global.search.close")}
              ><X size={12} strokeWidth={1.8} /></button>
            </div>
          </div>
        {:else}
          <button class="refresh-btn" onclick={focusModuleSearch} aria-label={$t("global.search.title")}>
            <Search size={14} strokeWidth={1.8} />
          </button>
        {/if}
        <button
          type="button"
          class="refresh-btn"
          onclick={() => onRefresh()}
          disabled={refreshing}
          aria-label={$t("global.action.refresh")}
        >
          <RefreshCw size={14} strokeWidth={1.8} class={refreshing ? "spin" : ""} />
        </button>
      </div>
    </div>
    <div class="content-tab-row">
      <div class="content-tabs" aria-label={$t("skills.title")}>
        <button
          type="button"
          class="content-tab"
          class:active={skillsTab === "overview"}
          aria-current={skillsTab === "overview" ? "page" : undefined}
          onclick={() => { setSkillsTab("overview"); }}
        >{$t("skills.view.all")}</button>
        <button
          type="button"
          class="content-tab"
          class:active={skillsTab === "common"}
          aria-current={skillsTab === "common" ? "page" : undefined}
          onclick={() => { setSkillsTab("common"); }}
        >{$t("skills.view.common")}</button>
        <button
          type="button"
          class="content-tab"
          class:active={skillsTab === "tool"}
          aria-current={skillsTab === "tool" ? "page" : undefined}
          onclick={() => { setSkillsTab("tool"); }}
        >{$t("rules.tab.tool")}</button>
      </div>
    </div>
    {#if skillsLoadError}
      <div class="error-msg">
        <div>{skillsLoadError}</div>
        {#if skillsLoadErrorDetails.length > 0}
          <ul class="error-detail-list">
            {#each skillsLoadErrorDetails as detail}
              <li>{detail}</li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}
  </div>

  {#if skillsTab === "tool"}
    <div class="view-pinned-toolbar">
      <ProtectedToolSelector
        tools={$managedTools}
        activeToolId={currentToolId}
        ariaLabel={$t("skills.title")}
        onSelect={selectTool}
      />
    </div>
  {/if}

  <div class="view-scroll-content skills-scroll-content">
    <ModulePerformanceBar moduleId="skills" />

    {#if skillsTab === "overview"}
      {#if skillsOverview.length === 0}
        <div class="empty">{$t("skills.empty.all")}</div>
      {:else if filteredSkillsOverview.length === 0}
        <div class="empty">{$t("module_search.empty_scope", { scope: $t("module_search.scope_skill_list") })}</div>
      {:else}
        <div class="item-list">
          {#each filteredSkillsOverview as item}
            <BaseCard
              showIcon={false}
              title={item.display_name || item.name}
              description={item.description || ""}
              note={packageNote(item)}
              badges={buildSkillBadges(item)}
              onclick={() => openSkillViewer(item)}
            />
          {/each}
        </div>
      {/if}

    {:else if skillsTab === "common"}
      {#if genericSkills.length === 0}
        <div class="empty">{$t("skills.empty.common")}</div>
      {:else if filteredGenericSkills.length === 0}
        <div class="empty">{$t("module_search.empty_scope", { scope: $t("module_search.scope_skill_list") })}</div>
      {:else}
        <div class="item-list">
          {#each filteredGenericSkills as skill}
            <BaseCard
              showIcon={false}
              title={skill.display_name || skill.name}
              description={skill.description || ""}
              note={packageNote(skill)}
              badges={buildGenericSkillBadges(skill)}
              onclick={() => openSkillViewer(skill)}
            />
          {/each}
        </div>
      {/if}

    {:else}
      {#if $managedTools.length === 0}
        <div class="empty">{$t("skills.empty.no_managed_tools")}</div>
      {:else if !currentTool}
        <div class="empty">{$t("settings.tools.status_missing")}</div>
      {:else if toolSkillsForDisplay.length === 0}
        <div class="empty">{$t("skills.empty.tool")}</div>
      {:else if filteredToolSkillsForDisplay.length === 0}
        <div class="empty">{$t("module_search.empty_scope", { scope: $t("module_search.scope_skill_list") })}</div>
      {:else}
        <div class="item-list">
          {#each filteredToolSkillsForDisplay as skill}
            <BaseCard
              showIcon={false}
              title={skill.display_name || skill.name}
              description={skill.description || ""}
              note={packageNote(skill)}
              badges={buildToolSkillBadges(skill, currentToolId)}
              warning={buildToolSkillWarning(skill, currentToolId)}
              onclick={() => openSkillViewer(skill)}
            />
          {/each}
        </div>
      {/if}
    {/if}
  </div>

</div>

<SkillViewer bind:this={skillViewerRef} skill={viewingSkill} initialTab={skillViewerInitialTab} initialAction={skillViewerInitialAction} onClose={closeSkillViewer} onDelete={async (/** @type {string} */ name, /** @type {any[]} */ _paths) => {
  closeSkillViewer();
  globalRefreshMsg = $t("skills.delete.success", { name });
  setTimeout(() => { if (globalRefreshMsg) globalRefreshMsg = ""; }, 4000);
  try {
    invalidateToolSkillsCache();
    await refreshSkillsData();
  } catch (_) {}
}} onChanged={async (/** @type {any} */ changedSkill = null) => {
  try {
    invalidateToolSkillsCache();
    if (changedSkill?.name) {
      applySingleInventoryEntry(changedSkill);
      scheduleSourceWriteBackgroundRefresh();
      return;
    }
    await refreshSkillsData();
    if (viewingSkill?.name) {
      const name = viewingSkill.name;
      const refreshed = skillsOverview.find((/** @type {any} */ item) => item.name === name);
      if (refreshed) {
        viewingSkill = {
          ...(/** @type {any} */ (viewingSkill)),
          display_name: viewingSkill.display_name || refreshed.display_name || name,
          path: viewingSkill.path || refreshed.path || "",
          installed_in: refreshed.installed_in || viewingSkill.installed_in || [],
          tool_statuses: refreshed.tool_statuses || [],
          has_variants: refreshed.has_variants || viewingSkill.has_variants || [],
          variant_paths: refreshed.variant_paths || viewingSkill.variant_paths || {},
          package: refreshed.package || viewingSkill.package || null,
        };
      }
    }
  } catch (_) {}
}} />

{#if globalRefreshMsg}
  <div class="app-transient-toast" role="status" aria-live="polite">{globalRefreshMsg}</div>
{/if}

<style>
  .error-msg { padding: 12px 14px; font-size: 12px; color: #f87171; background: var(--bg-danger); border: 1px solid rgba(248,113,113,0.22); border-radius: 8px; margin: 12px 0 4px; white-space: pre-wrap; }
  .error-detail-list { margin: 8px 0 0; padding-left: 18px; }
  .error-detail-list li { margin: 4px 0; }

  .app-transient-toast {
    position: fixed;
    bottom: 28px;
    left: 50%;
    transform: translateX(-50%);
    max-width: min(420px, calc(100vw - 48px));
    padding: 10px 16px;
    font-size: 12px;
    font-weight: 500;
    color: var(--color-text-main);
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 10px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    z-index: 12000;
    pointer-events: none;
    text-align: center;
  }
</style>
