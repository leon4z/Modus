<script>
  // Purpose: Skill detail view for shared-directory and tool-directory operations.
  import { onMount, tick, untrack } from "svelte";
  import { AlertCircle, ArrowLeft, Blocks, Languages, X, Pencil, Trash2, Search } from "lucide-svelte";
  import ConfirmDialog from "$lib/shared/components/ConfirmDialog.svelte";
  import ContentEditor from "$lib/shared/components/ContentEditor.svelte";
  import FileWorkspaceSearchResults from "$lib/shared/components/FileWorkspaceSearchResults.svelte";
  import FileWorkspaceNavigation from "$lib/shared/components/FileWorkspaceNavigation.svelte";
  import FileViewerShell from "$lib/shared/components/FileViewerShell.svelte";
  import PrimaryState from "$lib/shared/components/PrimaryState.svelte";
  import SourceSelectorCapsule from "$lib/features/skills/components/SourceSelectorCapsule.svelte";
  import SourceReadModeToggle from "$lib/shared/components/SourceReadModeToggle.svelte";
  import ToolIcon from "$lib/shared/components/ToolIcon.svelte";
  import Tooltip from "$lib/shared/components/Tooltip.svelte";
  import { refreshTranslationProviderState, translationProviderState } from "$lib/features/settings/index.js";
  import {
    MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE,
    MODULE_PERFORMANCE_ROLE_INTERACTIVE,
    MODULE_PERFORMANCE_ROLE_VISIBLE,
    beginModulePerformanceRun,
    finishAndRecordModulePerformanceRun,
    markModulePerformance,
    trackModulePerformanceRequest,
  } from "$lib/shared/diagnostics/modulePerformance.js";
  import { readSkillContent, listSkillFiles, readSkillFile, writeSkillFile, translateMarkdown, scanSkillInventoryEntry, previewInstall, executeInstall, previewCopySkillToTool, executeCopySkillToTool, previewRenameSkillSource, executeRenameSkillSource, previewUninstall, executeUninstall, previewDeleteFromTool, executeDeleteFromTool, previewCleanupDuplicateSkillSources, executeCleanupDuplicateSkillSources, previewDelete, executeDelete } from "$lib/features/skills/api/skills.js";
  import { getSkillInventory, invalidateSkillInventory } from "$lib/features/skills/queries/skillInventoryQuery.js";
  import { normalizeSkillStatus, parseOperationPreview, statusToInstallMode } from "$lib/features/skills/domain/skillDomain.js";
  import { detectedTools, managedTools, tools, getToolName, hasProjectionAction } from "$lib/features/tools/index.js";
  import { t } from "$lib/shared/i18n/index.js";
  import { buildFileWorkspaceSearchGroups } from "$lib/shared/utils/fileWorkspaceSearch.js";
  import { formatFrontmatterForMarkdown } from "$lib/shared/utils/utils.js";

  let { skill = null, initialTab = "content", initialAction = "", onClose, onDelete = () => {}, onChanged = () => {} } = $props();

  let deletingSkill = $state(false);
  let confirmDialog = $state(/** @type {any} */ (null));
  let confirmOpen = $state(false);

  let fileContent = $state("");
  let loadingContent = $state(false);
  let selectedFile = $state("SKILL.md");
  /** @type {any[]} */
  let fileTree = $state([]);
  /** @type {Set<string>} */
  let expandedDirs = $state(new Set());
  let skillFileNavigationCollapsed = $state(false);
  let mdPreview = $state(true);
  let editingFile = $state(false);
  let editContent = $state("");
  let savingFile = $state(false);
  let hasLoadedFileContent = $state(false);
  let contentLoadId = 0;
  let selectedFileBySkillPath = $state(/** @type {Record<string, string>} */ ({}));
  let skillSearchOpen = $state(false);
  let skillSearchQuery = $state("");
  let skillSearchTarget = $state({
    fileId: "",
    query: "",
    matchIndex: 0,
    version: 0,
  });
  let skillSearchResultsOpen = $state(false);
  let skillSearchAutoOpenKey = $state("");
  let skillSearchContentCache = $state(/** @type {Record<string, string>} */ ({}));
  let skillSearchInput = $state(/** @type {HTMLInputElement | undefined} */ (undefined));
  let skillSearchAnchor = $state(/** @type {HTMLDivElement | undefined} */ (undefined));
  let skillSearchLoadKey = "";
  let skillSearchLoadId = 0;
  let translationMenuOpen = $state(false);
  let translatingFile = $state(false);
  let translationResult = $state(/** @type {{ content: string, targetLanguage: string } | null} */ (null));
  let translationMsg = $state("");

  let activeTab = $state("content");
  let loadingInstall = $state(false);
  let installMsg = $state("");
  let installMsgType = $state("ok");
  let sourcePickerOpen = $state(false);
  /** @type {any[]} */
  let sourcePickerCandidates = $state([]);
  let selectedSourceKey = $state("");
  /** @type {((value: any) => void) | null} */
  let sourcePickerResolver = null;

  /** @type {any[]} */
  let installedPresences = $state([]);
  /** @type {any[] | null} */
  let fetchedToolStatuses = $state(null);
  let duplicateHandlingOpen = $state(false);
  /** @type {any | null} */
  let duplicateHandlingStatus = $state(null);
  /** @type {any[]} */
  let duplicateHandlingSources = $state([]);
  let selectedDuplicateKeepPath = $state("");
  let duplicateRenameActive = $state(false);
  let duplicateRenameName = $state("");
  let selectedVariant = $state("generic");
  let activeSkillPath = $state("");
  let lastSkillIdentity = $state("");
  let lastSkillName = $state("");
  let requireInventoryBackedContentSources = $state(false);
  let suppressVariantEffect = false;

  onMount(() => {
    void refreshTranslationProviderState().catch(() => {});
  });

  let isMarkdown = $derived(selectedFile.toLowerCase().endsWith(".md"));
  let showFileModeToggle = $derived(isMarkdown && hasLoadedFileContent);
  let isFileDirty = $derived(String(editContent ?? "") !== String(fileContent ?? ""));
  let selectedFileLineCount = $derived(String((translationResult ? translationResult.content : editContent) ?? "").split("\n").length);
  let selectedFilePath = $derived(activeSkillPath && selectedFile ? joinSkillFilePath(activeSkillPath, selectedFile) : "");
  let isTranslationView = $derived(Boolean(translationResult));
  let translationProviderEnabled = $derived($translationProviderState.enabled === true);
  let canTranslateCurrentSkillFile = $derived(translationProviderEnabled && isMarkdown && hasLoadedFileContent && !editingFile && !loadingContent && !isTranslationView);
  let skillSearchFiles = $derived(displayableSkillFiles(fileTree).map((/** @type {any} */ file) => ({
    id: file.relative_path,
    label: fileTreeLabel(file.relative_path),
    path: file.relative_path,
    content: selectedFile === file.relative_path
      ? editContent
      : skillSearchContentCache[file.relative_path] || "",
    raw: file,
  })));
  let skillSearchGroups = $derived.by(() => buildFileWorkspaceSearchGroups({
    query: skillSearchQuery,
    files: skillSearchFiles,
    currentFileId: selectedFile,
    currentContent: editContent,
  }));
  let canShowSkillSearchResults = $derived(
    skillSearchOpen && activeTab === "content" && skillSearchResultsOpen && Boolean(skillSearchQuery.trim()) && skillSearchFiles.length > 0
  );
  let activeSkillExternalSearchQuery = $derived(
    skillSearchOpen && activeTab === "content" && !isTranslationView && Boolean(skillSearchQuery.trim()) ? skillSearchQuery.trim() : ""
  );
  let activeSkillExternalSearchMatchIndex = $derived(
    skillSearchTarget.query === activeSkillExternalSearchQuery && skillSearchTarget.fileId === selectedFile
      ? skillSearchTarget.matchIndex
      : 0
  );
  let activeSkillExternalSearchVersion = $derived(
    skillSearchTarget.query === activeSkillExternalSearchQuery && skillSearchTarget.fileId === selectedFile
      ? skillSearchTarget.version
      : 0
  );

  $effect(() => {
    const nextKey = skillSearchOpen && activeTab === "content" ? skillSearchQuery.trim() : "";
    if (!nextKey) {
      skillSearchAutoOpenKey = "";
      skillSearchResultsOpen = false;
      return;
    }
    if (nextKey !== skillSearchAutoOpenKey) {
      skillSearchAutoOpenKey = nextKey;
      skillSearchResultsOpen = true;
    }
  });

  /** @param {string} status */
  const statusToMode = statusToInstallMode;

  /** @param {string | null | undefined} status */
  function isSkillStatusInstalledLike(status) {
    const normalized = normalizeSkillStatus(status);
    return Boolean(normalized)
      && normalized !== "notInstalled"
      && normalized !== "variantNotInstalled"
      && normalized !== "noVariant";
  }

  /** @param {string | null | undefined} status */
  function isSkillStatusActionableSource(status) {
    const normalized = normalizeSkillStatus(status);
    return isSkillStatusInstalledLike(status)
      && normalized !== "variantDrifted"
      && normalized !== "brokenSymlink";
  }

  /** @param {any} status */
  function canDeleteToolDirectorySource(status) {
    return Boolean(status?.canDeleteSkill)
      && !isSharedBackedTool(status)
      && isSkillStatusActionableSource(status?.rawStatus || status?.status);
  }

  /** @param {string} toolId */
  function getDetectedTool(toolId) {
    return $detectedTools.find((item) => item.id === toolId);
  }

  /** @param {string} toolId @param {string} action */
  function canUseSkillActionForTool(toolId, action) {
    const tool = getDetectedTool(toolId);
    return hasProjectionAction(tool, "skills", action);
  }

  /** @param {string} toolId */
  function canInstallSkillForTool(toolId) {
    return canUseSkillActionForTool(toolId, "install");
  }

  /** @param {string} toolId */
  function canCopySkillForTool(toolId) {
    return canUseSkillActionForTool(toolId, "copy");
  }

  /** @param {string} toolId */
  function canDeleteSkillForTool(toolId) {
    return canUseSkillActionForTool(toolId, "delete");
  }

  /** @param {string} toolId */
  function canUninstallSkillForTool(toolId) {
    return canUseSkillActionForTool(toolId, "uninstall");
  }

  /** @param {string} toolId */
  function canSaveSkillForTool(toolId) {
    return canUseSkillActionForTool(toolId, "save") || canUseSkillActionForTool(toolId, "edit");
  }

  /** @param {string} toolId */
  function canManageSkillForTool(toolId) {
    return ["install", "copy", "delete", "uninstall", "save", "edit"].some((action) =>
      canUseSkillActionForTool(toolId, action)
    );
  }

  /** @param {string} toolId */
  function getSkillActionStateForTool(toolId) {
    const tool = getDetectedTool(toolId);
    return {
      canInstallSkill: hasProjectionAction(tool, "skills", "install"),
      canCopySkill: hasProjectionAction(tool, "skills", "copy"),
      canDeleteSkill: hasProjectionAction(tool, "skills", "delete"),
      canUninstallSkill: hasProjectionAction(tool, "skills", "uninstall"),
      canSaveSkill: hasProjectionAction(tool, "skills", "save") || hasProjectionAction(tool, "skills", "edit"),
    };
  }

  /** @param {string} toolId */
  function hasReadableSkillSourceForTool(toolId) {
    const tool = getDetectedTool(toolId);
    return hasProjectionAction(tool, "skills", "view") || hasProjectionAction(tool, "skills", "read");
  }

  /** @param {string} toolId */
  function canEditToolSkillForTool(toolId) {
    const tool = $detectedTools.find((item) => item.id === toolId);
    return canSaveSkillForTool(tool?.id || toolId);
  }

  /** @param {string} toolId */
  function canRenameSkillSourceForTool(toolId) {
    const resolvedToolId = $detectedTools.find((item) => item.id === toolId)?.id || toolId;
    return canSaveSkillForTool(resolvedToolId)
      || (canInstallSkillForTool(resolvedToolId) && canDeleteSkillForTool(resolvedToolId));
  }

  /** @param {any} status */
  function getRawSharedSourcePath(status) {
    return status?.symlink_target || status?.symlinkTarget || status?.target_path || status?.targetPath || status?.path || "";
  }

  /** @param {any} status */
  function getMappedSharedSourcePath(status) {
    return status?.target_path || status?.targetPath || status?.symlink_target || status?.symlinkTarget || status?.path || "";
  }

  /** @param {any} status */
  function isSharedBackedTool(status) {
    return status?.pathOrigin === "generic";
  }

  /** @param {any} source */
  function sourceTargetPath(source) {
    return source?.target_path || source?.targetPath || source?.symlink_target || source?.symlinkTarget || "";
  }

  /** @param {any} source */
  function isLinkedSharedSource(source) {
    const targetPath = sourceTargetPath(source);
    return source?.pathOrigin !== "generic"
      && typeof targetPath === "string"
      && targetPath.length > 0
      && targetPath !== source?.path;
  }

  /** @param {any} source */
  function contentPathForSource(source) {
    if (source?.pathOrigin === "generic") return sourceTargetPath(source) || source?.path || "";
    if (isLinkedSharedSource(source)) return sourceTargetPath(source);
    return source?.path || "";
  }

  /** @param {any} source */
  function contentPathOriginForSource(source) {
    return (source?.pathOrigin === "generic" || isLinkedSharedSource(source)) ? "generic" : "tool";
  }

  /** @param {any} status */
  function isToolDirectorySymlink(status) {
    return status?.mode === "symlink"
      && typeof status?.path === "string"
      && status.path.length > 0
      && typeof status?.target_path === "string"
      && status.target_path.length > 0
      && status.path !== status.target_path;
  }

  /** @param {any} change */
  function isExecutableStructuredChange(change) {
    const kind = change?.changeKind || change?.change_kind || change?.action;
    return kind === "create" || kind === "delete" || kind === "overwrite";
  }

  /** @param {any} preview */
  function previewHasExecutableChanges(preview) {
    const normalized = parseOperationPreview(preview);
    const hasStructuredExecutableChanges = normalized.changes.some(isExecutableStructuredChange);
    const hasExplicitChanges = normalized.creates.length > 0
      || normalized.deletes.length > 0
      || normalized.overwrites.length > 0
      || hasStructuredExecutableChanges;
    if (hasExplicitChanges) return true;
    if (normalized.preserves.length > 0) return false;
    if (normalized.blocked.length > 0 || normalized.blockedItems.length > 0) return false;
    if (typeof normalized.message === "string" && normalized.message.includes("无需")) return false;
    return true;
  }

  /**
   * @param {string} title
   * @param {any} preview
   * @param {{ confirmLabel?: string, cancelLabel?: string, variant?: "default" | "danger" }} options
   */
  async function confirmOperation(title, preview, options = {}) {
    const normalized = parseOperationPreview(preview);
    confirmOpen = true;
    try {
      return await confirmDialog.show({
        title,
        preview: normalized,
        variant: options.variant || "default",
        confirmLabel: options.confirmLabel || "",
        cancelLabel: options.cancelLabel || "",
      });
    } finally {
      confirmOpen = false;
    }
  }

  /** @param {any} skillEntry */
  function getRawToolStatuses(skillEntry) {
    return filterStatusesForManagedTools(fetchedToolStatuses || skillEntry?.tool_statuses || skillEntry?.toolStatuses || []);
  }

  function currentManagedToolIdSet() {
    return new Set($managedTools.map((/** @type {any} */ tool) => String(tool.id)));
  }

  /** @param {any[]} statuses */
  function filterStatusesForManagedTools(statuses) {
    const managedIds = currentManagedToolIdSet();
    return (Array.isArray(statuses) ? statuses : []).filter((/** @type {any} */ status) => {
      const toolId = status?.tool_id || status?.toolId;
      return toolId && managedIds.has(String(toolId));
    });
  }

  /**
   * @param {any} status
   * @param {any} [tool]
   * @returns {any[]}
   */
  function normalizeSourceEntries(status, tool = null) {
    const rawSources = Array.isArray(status?.sources) ? status.sources : [];
    const fallbackPath = status?.path || "";
    const sourceItems = /** @type {any[]} */ (rawSources.length > 0
      ? rawSources
      : (fallbackPath ? [status] : []));
    return sourceItems
      .map((/** @type {any} */ source) => {
        const toolId = source?.tool_id || source?.toolId || status?.tool_id || status?.toolId || tool?.id || "";
        const pathOrigin = source?.path_origin || source?.pathOrigin || status?.path_origin || status?.pathOrigin || "tool";
        const path = source?.path || "";
        const targetPath = source?.symlink_target || source?.symlinkTarget || source?.target_path || source?.targetPath || null;
        return {
          tool_id: toolId,
          tool_name: source?.tool_name || source?.toolName || status?.tool_name || status?.toolName || tool?.name || toolId,
          status: normalizeSkillStatus(source?.status || status?.status),
          rawStatus: normalizeSkillStatus(source?.status || status?.status),
          path,
          pathOrigin,
          target_path: targetPath,
          updatedAt: source?.updated_at || source?.updatedAt || status?.updated_at || status?.updatedAt || "",
          contentHash: source?.content_hash || source?.contentHash || status?.content_hash || status?.contentHash || "",
        };
      })
      .filter((/** @type {any} */ source) => source.path);
  }

  /** @param {any} source */
  function sourceKey(source) {
    const scope = source?.pathOrigin === "generic" ? "generic" : (source?.tool_id || "tool");
    return `${scope}:${source?.path || ""}`;
  }

  /** @param {any} source */
  function sourceLabel(source) {
    if (source?.pathOrigin === "generic") return $t("skills.viewer.source_shared");
    return source?.tool_name || getToolName(source?.tool_id, $tools) || source?.tool_id || $t("skills.viewer.source_diff_unknown");
  }

  /**
   * @param {string} skillPath
   * @param {any[]} statuses
   */
  function canonicalContentPathForSkillPath(skillPath, statuses) {
    if (!skillPath) return "";
    for (const status of statuses) {
      for (const source of normalizeSourceEntries(status)) {
        const canonicalPath = source.target_path || source.path;
        if (source.path === skillPath || source.target_path === skillPath) {
          return canonicalPath;
        }
      }
    }
    return skillPath;
  }

  /** @param {any} status */
  function hasDuplicateSources(status) {
    const sources = Array.isArray(status?.sources) ? status.sources : [];
    return status?.abnormalState === "duplicate_sources"
      || status?.abnormal_state === "duplicate_sources"
      || sources.length > 1;
  }

  /** @param {any} status */
  function displaySourcesForStatus(status) {
    const sources = /** @type {any[]} */ (Array.isArray(status?.sources) && status.sources.length > 0
      ? status.sources
      : normalizeSourceEntries(status));
    return sources
      .map((/** @type {any} */ source) => ({
        ...source,
        tool_id: source.tool_id || source.toolId || status?.tool_id || status?.toolId,
        tool_name: source.tool_name || source.toolName || status?.name || status?.tool_name || status?.toolName,
        pathOrigin: source.pathOrigin || source.path_origin || "tool",
        target_path: source.target_path || source.targetPath || source.symlink_target || source.symlinkTarget || null,
      }))
      .filter((/** @type {any} */ source) => source.path);
  }

  /** @param {any} source @param {any} status */
  function sourceToolId(source, status = null) {
    return source?.tool_id || source?.toolId || status?.tool_id || status?.toolId || "";
  }

  /** @param {any} source @param {any} status */
  function canDeleteDuplicateSource(source, status = null) {
    if (source?.pathOrigin === "generic") return true;
    return canDeleteSkillForTool(sourceToolId(source, status));
  }

  /** @param {any} source @param {any} status */
  function canRenameDuplicateSource(source, status = null) {
    if (source?.pathOrigin === "generic") return true;
    return canRenameSkillSourceForTool(sourceToolId(source, status));
  }

  /** @param {any} status */
  function canHandleDuplicateStatus(status) {
    if (!hasDuplicateSources(status)) return false;
    return displaySourcesForStatus(status).some((source) =>
      canDeleteDuplicateSource(source, status) || canRenameDuplicateSource(source, status)
    );
  }

  /** @param {any[]} statuses */
  function sortToolStatusesForDisplay(statuses) {
    return (Array.isArray(statuses) ? statuses : [])
      .map((item, index) => ({
        item,
        index,
        installedRank: item.status === "installed" ? 0 : 1,
      }))
      .sort((a, b) => a.installedRank - b.installedRank || a.index - b.index)
      .map(({ item }) => item);
  }

  /** @param {any} tool */
  function createAvailableToolStatus(tool) {
    const actionState = getSkillActionStateForTool(tool.id);
    return {
      tool_id: tool.id,
      name: tool.name,
      status: "available",
      rawStatus: "notInstalled",
      mode: null,
      path: "",
      pathOrigin: "tool",
      target_path: null,
      updatedAt: "",
      contentHash: "",
      sources: [],
      abnormalState: null,
      skills_dir: tool.skills_dir || tool.skillsDir || "",
      canWriteSkill: canManageSkillForTool(tool.id),
      ...actionState,
    };
  }

  /** @param {any} ts @param {any} tool */
  function mapInventoryToolStatus(ts, tool) {
    const toolId = ts.tool_id || ts.toolId;
    const normalized = normalizeSkillStatus(ts.status);
    const sources = normalizeSourceEntries(ts, tool);
    const primarySource = sources[0] || null;
    const pathOrigin = ts.path_origin || ts.pathOrigin || primarySource?.pathOrigin || "tool";
    const targetPath = ts.symlink_target || ts.symlinkTarget || ts.target_path || ts.targetPath || primarySource?.target_path || null;
    const hasPath = (typeof ts.path === "string" && ts.path.length > 0) || Boolean(primarySource?.path);
    const installed = isSkillStatusInstalledLike(ts.status) && (normalized !== "noVariant" || hasPath);
    const actionState = getSkillActionStateForTool(toolId);
    const mode = installed
      ? (targetPath || normalized.includes("Symlink") ? "symlink" : statusToMode(normalized) || "copy")
      : null;
    return {
      tool_id: toolId,
      name: tool?.name || ts.tool_name || ts.toolName || toolId,
      status: installed ? "installed" : "available",
      rawStatus: normalized,
      mode,
      path: ts.path || primarySource?.path || "",
      pathOrigin,
      target_path: targetPath,
      updatedAt: ts.updated_at || ts.updatedAt || "",
      contentHash: ts.content_hash || ts.contentHash || "",
      sources,
      abnormalState: ts.abnormal_state || ts.abnormalState || (sources.length > 1 ? "duplicate_sources" : null),
      skills_dir: tool?.skills_dir || tool?.skillsDir || "",
      canWriteSkill: canManageSkillForTool(toolId),
      ...actionState,
    };
  }

  /** @param {any} status @param {string} sharedInstallPath @param {boolean} hasCopyAction */
  function getInstallUnavailableReason(status, sharedInstallPath, hasCopyAction = false) {
    if (!status || status.status === "installed") return "";
    if (!hasContentSources) return $t("skills.viewer.install_unavailable_no_source");
    if (hasCopyAction) return "";
    if (!sharedInstallPath) return $t("skills.viewer.install_unavailable_no_shared_source");
    if (status.canInstallSkill) return "";
    if (hasReadableSkillSourceForTool(status.tool_id)) {
      return $t("skills.viewer.install_unavailable_read_only");
    }
    return $t("skills.viewer.install_unavailable_no_deploy_target");
  }

  let toolStatuses = $derived.by(() => {
    if (!skill) return [];
    const displayedTools = $managedTools.filter((tool) => tool.detected);
    const displayedToolIds = new Set(displayedTools.map((tool) => tool.id));
    const inventoryStatuses = filterStatusesForManagedTools(fetchedToolStatuses || skill.tool_statuses || skill.toolStatuses);
    if (Array.isArray(inventoryStatuses) && inventoryStatuses.length > 0) {
      const mappedByToolId = new Map();
      for (const ts of inventoryStatuses) {
        const toolId = ts.tool_id || ts.toolId;
        if (!displayedToolIds.has(toolId)) continue;
        const tool = displayedTools.find((item) => item.id === toolId);
        mappedByToolId.set(toolId, mapInventoryToolStatus(ts, tool));
      }
      for (const tool of displayedTools) {
        if (!mappedByToolId.has(tool.id)) {
          mappedByToolId.set(tool.id, createAvailableToolStatus(tool));
        }
      }
      return sortToolStatusesForDisplay(Array.from(mappedByToolId.values()));
    }

    const presences = installedPresences;
    return sortToolStatusesForDisplay(displayedTools.map((tool) => {
      const presence = presences.find((item) => item.tool_id === tool.id);
      if (presence) {
        const actionState = getSkillActionStateForTool(tool.id);
        return {
          tool_id: tool.id,
          name: tool.name,
          status: "installed",
          rawStatus: "installed",
          mode: presence.mode || "copy",
          path: presence.path || "",
          pathOrigin: presence.pathOrigin || "tool",
          target_path: presence.target_path || null,
          updatedAt: "",
          contentHash: "",
          sources: presence.path ? [{
            tool_id: tool.id,
            tool_name: tool.name,
            status: "installed",
            rawStatus: "installed",
            path: presence.path,
            pathOrigin: presence.pathOrigin || "tool",
            target_path: presence.target_path || null,
            updatedAt: "",
            contentHash: "",
          }] : [],
          abnormalState: null,
          skills_dir: tool.skills_dir || tool.skillsDir || "",
          canWriteSkill: canManageSkillForTool(tool.id),
          ...actionState,
        };
      }
      return createAvailableToolStatus(tool);
    }));
  });

  let installedCount = $derived(toolStatuses.filter((item) => item.status === "installed").length);
  let totalToolCount = $derived(toolStatuses.length);

  /**
   * @param {any} skillEntry
   * @returns {{ key: string, label: string, path: string, pathOrigin: string, toolId: string }[]}
   */
  function getContentSources(skillEntry) {
    const seen = new Set();
    /** @type {{ key: string, label: string, path: string, pathOrigin: string, toolId: string }[]} */
    const sources = [];
    const statuses = Array.isArray(getRawToolStatuses(skillEntry)) ? getRawToolStatuses(skillEntry) : [];
    for (const status of statuses) {
      for (const source of normalizeSourceEntries(status)) {
        const normalized = normalizeSkillStatus(source?.status);
        const path = contentPathForSource(source);
        const pathOrigin = contentPathOriginForSource(source);
        const installed = normalized !== "notInstalled"
          && normalized !== "variantNotInstalled"
          && normalized !== "noVariant"
          && typeof path === "string"
          && path.length > 0;
        if (!installed) continue;
        const key = sourceKey({
          ...source,
          path,
          pathOrigin,
          tool_id: pathOrigin === "generic" ? "generic" : source.tool_id,
        });
        if (!key || seen.has(key)) continue;
        seen.add(key);
        sources.push({
          key,
          label: pathOrigin === "generic" ? $t("skills.viewer.source_shared") : sourceLabel(source),
          path,
          pathOrigin,
          toolId: pathOrigin === "generic" ? "generic" : source.tool_id,
        });
      }
    }

    // Once a refreshed inventory exists, do not resurrect a removed source from
    // the dialog's previous active path.
    const fallbackPath = requireInventoryBackedContentSources
      ? ""
      : canonicalContentPathForSkillPath(skillEntry?.path || "", statuses);
    if (fallbackPath && !sources.some((source) => source.path === fallbackPath)) {
      sources.push({
        key: sourceKey({ path: fallbackPath, pathOrigin: "generic", tool_id: "generic" }),
        label: $t("skills.viewer.source_shared"),
        path: fallbackPath,
        pathOrigin: "generic",
        toolId: "generic",
      });
    }
    return sources.sort((a, b) => {
      if (a.pathOrigin === "generic" && b.pathOrigin !== "generic") return -1;
      if (b.pathOrigin === "generic" && a.pathOrigin !== "generic") return 1;
      return String(a.key).localeCompare(String(b.key));
    });
  }

  /** @returns {Set<string>} */
  function getAbnormalContentSourcePaths() {
    const paths = new Set();
    for (const status of toolStatuses) {
      if (!hasDuplicateSources(status)) continue;
      for (const source of displaySourcesForStatus(status)) {
        const path = contentPathForSource(source);
        if (path) paths.add(path);
      }
    }
    return paths;
  }

  /** @param {any} source */
  function getContentSourceGroupKey(source) {
    if (source?.pathOrigin === "generic") return "generic";
    return source?.toolId || source?.label || "tool";
  }

  let contentSourceOptions = $derived.by(() => {
    const sources = getContentSources(skill);
    const abnormalPaths = getAbnormalContentSourcePaths();
    const counts = new Map();
    const indexes = new Map();
    for (const source of sources) {
      const groupKey = getContentSourceGroupKey(source);
      counts.set(groupKey, (counts.get(groupKey) || 0) + 1);
    }
    return sources.map((source) => {
      const groupKey = getContentSourceGroupKey(source);
      const nextIndex = (indexes.get(groupKey) || 0) + 1;
      indexes.set(groupKey, nextIndex);
      const duplicateCount = counts.get(groupKey) || 0;
      return {
        ...source,
        abnormal: abnormalPaths.has(source.path),
        duplicateCount,
        duplicateIndex: duplicateCount > 1 ? nextIndex : 0,
      };
    });
  });

  let skillVariants = $derived.by(() => contentSourceOptions.map((source) => source.key));
  let hasExtraVariants = $derived.by(() => skillVariants.length > 1);
  let hasContentSources = $derived.by(() => contentSourceOptions.length > 0);

  /** @param {any} skillEntry */
  function resolveInitialVariant(skillEntry) {
    const sources = getContentSources(skillEntry);
    const fromPath = sources.find((source) => source.path === skillEntry?.path);
    return fromPath?.key || sources[0]?.key || "generic";
  }

  /** @param {string} variant */
  function getVariantPath(variant) {
    return getContentSources(skill).find((source) => source.key === variant)?.path || "";
  }

  /** @param {string} variant */
  function canEditSkillSource(variant) {
    if (!skill?.path) return false;
    const source = getContentSources(skill).find((item) => item.key === variant);
    if (!source || source.pathOrigin === "generic") return true;
    return canEditToolSkillForTool(source.toolId);
  }

  let canEditCurrentSkillFile = $derived.by(() => canEditSkillSource(selectedVariant));

  function getSelectedSharedUsageText() {
    const selectedSource = getContentSources(skill).find((source) => source.key === selectedVariant);
    if (selectedSource?.pathOrigin !== "generic") return "";
    const sharedPath = getVariantPath(selectedVariant);
    const usingTools = toolStatuses
      .filter((item) => displaySourcesForStatus(item).some((source) =>
        contentPathOriginForSource(source) === "generic" && contentPathForSource(source) === sharedPath
      ))
      .map((item) => item.name || item.tool_id)
      .filter(Boolean);
    if (usingTools.length === 0) return $t("skills.viewer.shared_not_used");
    return $t("skills.viewer.shared_in_use_by", { tools: usingTools.join("、") });
  }

  /** @param {string} skillPath @param {string} relativePath */
  function rememberSelectedSkillFile(skillPath, relativePath) {
    if (!skillPath || !relativePath) return;
    selectedFileBySkillPath = { ...selectedFileBySkillPath, [skillPath]: relativePath };
  }

  /** @param {string} skillPath @param {string} relativePath */
  function joinSkillFilePath(skillPath, relativePath) {
    const root = String(skillPath || "").replace(/\/+$/, "");
    const child = String(relativePath || "").replace(/^\/+/, "");
    if (!root) return child;
    if (!child) return root;
    return `${root}/${child}`;
  }

  function nextSkillSearchVersion() {
    return untrack(() => skillSearchTarget.version + 1);
  }

  function clearTranslationResult() {
    translationMenuOpen = false;
    translationResult = null;
    translationMsg = "";
  }

  function bumpSkillSearchLoadId() {
    skillSearchLoadId = untrack(() => skillSearchLoadId + 1);
  }

  /** @param {any[]} files */
  function displayableSkillFiles(files) {
    return (Array.isArray(files) ? files : []).filter((/** @type {any} */ file) =>
      !file?.is_dir && String(file?.relative_path || "").length > 0
    );
  }

  /** @param {any[]} files @param {string} relativePath */
  function hasSkillFile(files, relativePath) {
    return displayableSkillFiles(files).some((/** @type {any} */ file) => file.relative_path === relativePath);
  }

  /** @param {any[]} files @param {any} full */
  function withSkillMdFallback(files, full) {
    const nextFiles = Array.isArray(files) ? [...files] : [];
    if (typeof full?.skill_md_content === "string" && !hasSkillFile(nextFiles, "SKILL.md")) {
      nextFiles.unshift({ relative_path: "SKILL.md", is_dir: false });
    }
    return nextFiles;
  }

  /** @param {string} skillPath @param {string} relativePath @param {any} full */
  async function loadSkillFileContent(skillPath, relativePath, full) {
    if (!relativePath) return "";
    if (relativePath === "SKILL.md" && typeof full?.skill_md_content === "string") {
      return full.skill_md_content;
    }
    try {
      return await readSkillFile(skillPath, relativePath);
    } catch (e) {
      return `(无法读取：${e})`;
    }
  }

  /** @param {string} skillPath */
  async function loadVariantContent(skillPath) {
    const loadId = ++contentLoadId;
    clearTranslationResult();
    if (!skillPath) {
      fileTree = [];
      fileContent = "";
      editContent = "";
      skillSearchOpen = false;
      skillSearchResultsOpen = false;
      skillSearchQuery = "";
      skillSearchTarget = { fileId: "", query: "", matchIndex: 0, version: nextSkillSearchVersion() };
      skillSearchContentCache = {};
      skillSearchLoadKey = "";
      bumpSkillSearchLoadId();
      loadingContent = false;
      hasLoadedFileContent = false;
      return;
    }
    loadingContent = true;
    hasLoadedFileContent = false;
    try {
      const [files, full] = await Promise.all([
        listSkillFiles(skillPath).catch(() => []),
        readSkillContent(skillPath).catch(() => null),
      ]);
      if (loadId !== contentLoadId) return;
      const nextFileTree = withSkillMdFallback(files, full);
      const displayableFiles = displayableSkillFiles(nextFileTree);
      const rememberedFile = selectedFileBySkillPath[skillPath] || selectedFile;
      const nextSelectedFile = displayableFiles.find((/** @type {any} */ file) => file.relative_path === rememberedFile)?.relative_path
        || displayableFiles.find((/** @type {any} */ file) => file.relative_path === "SKILL.md")?.relative_path
        || displayableFiles[0]?.relative_path
        || "";
      const nextContent = await loadSkillFileContent(skillPath, nextSelectedFile, full);
      if (loadId !== contentLoadId) return;
      fileTree = nextFileTree;
      selectedFile = nextSelectedFile;
      if (nextSelectedFile) rememberSelectedSkillFile(skillPath, nextSelectedFile);
      fileContent = nextContent;
      editContent = nextContent;
      skillSearchTarget = { fileId: "", query: "", matchIndex: 0, version: nextSkillSearchVersion() };
      skillSearchContentCache = nextSelectedFile ? { [nextSelectedFile]: nextContent } : {};
      skillSearchLoadKey = "";
      bumpSkillSearchLoadId();
      hasLoadedFileContent = Boolean(nextSelectedFile);
    } finally {
      if (loadId === contentLoadId) loadingContent = false;
    }
  }

  /** @param {string} variant */
  async function switchVariant(variant) {
    if (!skill) return;
    const nextPath = getVariantPath(variant);
    if (!nextPath) return;
    if (selectedVariant === variant && activeSkillPath === nextPath) return;
    suppressVariantEffect = true;
    try {
      lastSkillIdentity = `${skill.name || ""}::${nextPath || ""}`;
      selectedVariant = variant;
      activeSkillPath = nextPath;
      skill.path = nextPath;
      selectedFile = "SKILL.md";
      expandedDirs = new Set();
      skillFileNavigationCollapsed = false;
      editingFile = false;
      mdPreview = true;
      clearTranslationResult();
      await loadVariantContent(nextPath);
    } finally {
      suppressVariantEffect = false;
    }
  }

  $effect(() => {
    const s = skill;
    if (!s) {
      lastSkillIdentity = "";
      lastSkillName = "";
      selectedVariant = "generic";
      contentLoadId += 1;
      loadingContent = false;
      fileTree = [];
      fileContent = "";
      editContent = "";
      activeSkillPath = "";
      clearTranslationResult();
      requireInventoryBackedContentSources = false;
      skillFileNavigationCollapsed = false;
      hasLoadedFileContent = false;
      mdPreview = true;
      sourcePickerOpen = false;
      sourcePickerCandidates = [];
      selectedSourceKey = "";
      skillSearchOpen = false;
      skillSearchResultsOpen = false;
      skillSearchQuery = "";
      skillSearchTarget = { fileId: "", query: "", matchIndex: 0, version: nextSkillSearchVersion() };
      skillSearchContentCache = {};
      skillSearchLoadKey = "";
      bumpSkillSearchLoadId();
      if (sourcePickerResolver) {
        sourcePickerResolver(null);
        sourcePickerResolver = null;
      }
      return;
    }

    let currentSkillIdentity = `${s.name || ""}::${s.path || ""}`;
    const currentSkillName = s.name || "";
    const isNewSkillIdentity = currentSkillIdentity !== lastSkillIdentity;
    const isNewSkillName = currentSkillName !== lastSkillName;
    const initialVariant = resolveInitialVariant(s);
    const initialPath = getVariantPath(initialVariant) || s.path || "";
    lastSkillIdentity = currentSkillIdentity;
    lastSkillName = currentSkillName;
    selectedVariant = initialVariant;
    activeSkillPath = initialPath;
    if (initialPath && s.path !== initialPath) {
      currentSkillIdentity = `${s.name || ""}::${initialPath || ""}`;
      lastSkillIdentity = currentSkillIdentity;
      s.path = initialPath;
    }

    if (isNewSkillIdentity) {
      if (isNewSkillName) {
        requireInventoryBackedContentSources = false;
        activeTab = initialTab || "content";
        installMsg = "";
      }
      fileTree = [];
      selectedFile = "SKILL.md";
      expandedDirs = new Set();
      skillFileNavigationCollapsed = false;
      editingFile = false;
      mdPreview = true;
      clearTranslationResult();
      sourcePickerOpen = false;
      sourcePickerCandidates = [];
      selectedSourceKey = "";
      skillSearchOpen = false;
      skillSearchResultsOpen = false;
      skillSearchQuery = "";
      skillSearchTarget = { fileId: "", query: "", matchIndex: 0, version: nextSkillSearchVersion() };
      skillSearchContentCache = {};
      skillSearchLoadKey = "";
      bumpSkillSearchLoadId();
      loadVariantContent(initialPath).catch(() => {});
    } else if (suppressVariantEffect) {
      return;
    }

    installedPresences = s.installed_in || [];
    if (!s.installed_in) s.installed_in = [];

    if (s.name && isNewSkillName) {
      fetchedToolStatuses = null;
      const skillName = s.name;
      const requestIdentity = currentSkillIdentity;
      getSkillInventory().then((/** @type {any} */ inv) => {
        const liveIdentity = `${skill?.name || ""}::${skill?.path || ""}`;
        if (liveIdentity !== requestIdentity) return;
        const entry = inv?.skills?.find((/** @type {any} */ item) => item.name === skillName);
        const statuses = entry?.toolStatuses || entry?.tool_statuses;
        if (Array.isArray(statuses)) fetchedToolStatuses = filterStatusesForManagedTools(statuses);
      }).catch(() => {});
    }
  });

  $effect(() => {
    if (!skill || suppressVariantEffect) return;
    const sources = getContentSources(skill);
    if (sources.length === 0) return;
    const currentSource = sources.find((source) => source.key === selectedVariant && source.path === activeSkillPath);
    if (currentSource) return;
    const next = sources[0];
    selectedVariant = next.key;
    activeSkillPath = next.path;
    skill.path = next.path;
    selectedFile = "SKILL.md";
    expandedDirs = new Set();
    editingFile = false;
    mdPreview = true;
    clearTranslationResult();
    skillSearchOpen = false;
    skillSearchResultsOpen = false;
    skillSearchQuery = "";
    skillSearchTarget = { fileId: "", query: "", matchIndex: 0, version: nextSkillSearchVersion() };
    skillSearchContentCache = {};
    skillSearchLoadKey = "";
    bumpSkillSearchLoadId();
    loadVariantContent(next.path).catch(() => {});
  });

  $effect(() => {
    if (canEditCurrentSkillFile || !editingFile) return;
    editingFile = false;
    editContent = fileContent;
  });

  $effect(() => {
    if (activeTab === "content") return;
    if (translationResult || translationMenuOpen || translationMsg) clearTranslationResult();
    if (!skillSearchOpen && !skillSearchQuery) return;
    skillSearchOpen = false;
    skillSearchResultsOpen = false;
    skillSearchQuery = "";
    skillSearchTarget = { fileId: "", query: "", matchIndex: 0, version: nextSkillSearchVersion() };
  });

  $effect(() => {
    if (translationProviderEnabled) return;
    if (translationResult || translationMenuOpen || translationMsg) clearTranslationResult();
  });

  $effect(() => {
    const query = skillSearchQuery.trim();
    const skillPath = activeSkillPath;
    const files = skillSearchFiles;
    const cache = skillSearchContentCache;
    if (!skillSearchOpen || activeTab !== "content" || !query || !skillPath || !hasFileTreeNavigation) return;
    const missingFiles = files.filter((file) => file.id && cache[file.id] == null);
    if (missingFiles.length === 0) return;
    const loadKey = `${skillPath}::${missingFiles.map((file) => file.id).join("|")}`;
    if (loadKey === skillSearchLoadKey) return;
    skillSearchLoadKey = loadKey;
    const loadId = untrack(() => skillSearchLoadId + 1);
    skillSearchLoadId = loadId;
    Promise.all(missingFiles.map(async (file) => {
      try {
        return [file.id, await readSkillFile(skillPath, file.id)];
      } catch {
        return [file.id, ""];
      }
    })).then((entries) => {
      if (loadId !== skillSearchLoadId) return;
      const nextCache = { ...skillSearchContentCache };
      for (const [fileId, content] of entries) nextCache[fileId] = content;
      skillSearchContentCache = nextCache;
    });
  });

  /** @param {string} relativePath */
  async function selectFile(relativePath) {
    if (!activeSkillPath) return;
    clearTranslationResult();
    editingFile = false;
    selectedFile = relativePath;
    rememberSelectedSkillFile(activeSkillPath, relativePath);
    loadingContent = true;
    hasLoadedFileContent = false;
    let nextContent = "";
    try {
      nextContent = await readSkillFile(activeSkillPath, relativePath);
    } catch (e) {
      nextContent = `(无法读取：${e})`;
    } finally {
      fileContent = nextContent;
      editContent = nextContent;
      skillSearchContentCache = { ...skillSearchContentCache, [relativePath]: nextContent };
      hasLoadedFileContent = true;
      loadingContent = false;
    }
  }

  function startEditFile() {
    if (!canEditCurrentSkillFile) return;
    clearTranslationResult();
    editContent = fileContent;
    editingFile = true;
  }

  function cancelEditFile() {
    editingFile = false;
    editContent = fileContent;
  }

  /** @param {string} targetLanguage */
  async function translateCurrentFile(targetLanguage) {
    if (!canTranslateCurrentSkillFile || translatingFile) return;
    translationMenuOpen = false;
    translationMsg = "";
    translatingFile = true;
    try {
      const result = await translateMarkdown(fileContent, targetLanguage);
      translationResult = {
        content: String(result?.content || ""),
        targetLanguage: String(result?.targetLanguage || targetLanguage),
      };
      mdPreview = true;
    } catch (/** @type {any} */ e) {
      translationMsg = $t("skills.viewer.translation_failed", { err: e });
    } finally {
      translatingFile = false;
    }
  }

  /** @param {string} [nextContent] */
  async function saveEditFile(nextContent = editContent) {
    if (!activeSkillPath || !selectedFile || !canEditCurrentSkillFile) return;
    savingFile = true;
    try {
      await writeSkillFile(activeSkillPath, selectedFile, nextContent);
      editContent = nextContent;
      fileContent = nextContent;
      skillSearchContentCache = { ...skillSearchContentCache, [selectedFile]: nextContent };
      editingFile = false;
    } finally {
      savingFile = false;
    }
  }

  /** @param {string} dirPath */
  function toggleDir(dirPath) {
    if (expandedDirs.has(dirPath)) expandedDirs.delete(dirPath);
    else expandedDirs.add(dirPath);
    expandedDirs = new Set(expandedDirs);
  }

  /** @param {any} node */
  function getDirectSkillMdChild(node) {
    return (node?.children || []).find((/** @type {any} */ child) =>
      !child.is_dir && fileTreeLabel(child.relative_path).toLowerCase() === "skill.md"
    ) || null;
  }

  /** @param {any} node */
  function handleDirectoryClick(node) {
    toggleDir(node.relative_path);
    const skillMdChild = getDirectSkillMdChild(node);
    if (skillMdChild) selectFile(skillMdChild.relative_path);
  }

  /** @param {any} item */
  function toggleSkillFileNavigationItem(item) {
    if (item?.node?.is_dir) handleDirectoryClick(item.node);
  }

  /** @param {any} item */
  function selectSkillFileNavigationItem(item) {
    if (item?.node && !item.node.is_dir) selectFile(item.node.relative_path);
  }

  let treeStructure = $derived(buildTree(fileTree));
  let visibleTreeItems = $derived(flattenVisibleTree(treeStructure));
  let hasFileTreeNavigation = $derived(displayableSkillFiles(fileTree).length > 0);
  let skillFileNavigationItems = $derived(visibleTreeItems.map((/** @type {{ node: any, depth: number }} */ item) => {
    const relativePath = item.node.relative_path;
    if (item.node.is_dir) {
      return {
        id: `dir:${relativePath}`,
        kind: item.depth === 0 ? "root" : "directory",
        label: fileTreeLabel(relativePath),
        path: relativePath,
        depth: item.depth,
        expandable: (item.node.children || []).length > 0,
        expanded: expandedDirs.has(relativePath),
        raw: item,
        node: item.node,
      };
    }
    return {
      id: `file:${relativePath}`,
      kind: "file",
      label: fileTreeLabel(relativePath),
      path: relativePath,
      depth: item.depth,
      selected: selectedFile === relativePath,
      node: item.node,
    };
  }));
  let packageInfo = $derived(skill?.package || null);
  let packageMembers = $derived(packageInfo?.members || []);
  let isPackageSkill = $derived(Boolean(packageInfo?.isPackage || packageInfo?.is_package));
  let packageMemberGroups = $derived(groupPackageMembers(packageMembers));

  $effect(() => {
    if (activeTab === "members" && !isPackageSkill) activeTab = "content";
  });

  /** @param {any[]} files */
  function buildTree(files) {
    const nodes = new Map();
    for (const file of Array.isArray(files) ? files : []) {
      const relativePath = String(file?.relative_path || "");
      if (!relativePath) continue;
      nodes.set(relativePath, { ...file, relative_path: relativePath, children: [] });
    }
    /** @type {any[]} */
    const roots = [];
    for (const node of nodes.values()) {
      const parentPath = node.relative_path.split("/").slice(0, -1).join("/");
      const parent = parentPath ? nodes.get(parentPath) : null;
      if (parent) parent.children.push(node);
      else roots.push(node);
    }
    /** @param {any[]} list */
    const sortNodes = (list) => {
      list.sort((/** @type {any} */ a, /** @type {any} */ b) => {
        if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
        return a.relative_path.localeCompare(b.relative_path);
      });
      for (const node of list) sortNodes(node.children || []);
    };
    sortNodes(roots);
    return roots;
  }

  /** @param {any[]} nodes */
  function flattenVisibleTree(nodes) {
    /** @type {Array<{ node: any, depth: number }>} */
    const result = [];
    /**
     * @param {any[]} list
     * @param {number} depth
     */
    const visit = (list, depth) => {
      for (const node of Array.isArray(list) ? list : []) {
        result.push({ node, depth });
        if (node.is_dir && expandedDirs.has(node.relative_path)) visit(node.children || [], depth + 1);
      }
    };
    visit(nodes, 0);
    return result;
  }

  /** @param {string} relativePath */
  function fileTreeLabel(relativePath) {
    return String(relativePath || "").split("/").filter(Boolean).pop() || relativePath;
  }

  /** @param {string} kind */
  function packageMemberKindLabel(kind) {
    return $t(`skills.viewer.package_kind_${kind || "skill"}`);
  }

  /** @param {any} member */
  function getPackageMemberName(member) {
    return member?.display_name || member?.displayName || member?.name || "";
  }

  /** @param {any} member */
  function getPackageMemberPath(member) {
    return member?.relative_path || member?.relativePath || "";
  }

  /** @param {any[]} members */
  function groupPackageMembers(members) {
    const order = ["command", "agent", "workflow", "reference", "skill"];
    const rank = new Map(order.map((kind, index) => [kind, index]));
    const groups = new Map();
    for (const member of Array.isArray(members) ? members : []) {
      const kind = member?.kind || "skill";
      const group = groups.get(kind) || [];
      group.push(member);
      groups.set(kind, group);
    }
    return Array.from(groups.entries())
      .sort(([a], [b]) => (rank.get(a) ?? 99) - (rank.get(b) ?? 99) || a.localeCompare(b))
      .map(([kind, items]) => ({
        kind,
        items: items.slice().sort((/** @type {any} */ a, /** @type {any} */ b) => getPackageMemberPath(a).localeCompare(getPackageMemberPath(b))),
      }));
  }

  /** @param {string} relativePath */
  function expandTreePath(relativePath) {
    const parts = String(relativePath || "").split("/").filter(Boolean);
    if (parts.length <= 1) return;
    const next = new Set(expandedDirs);
    for (let i = 1; i < parts.length; i += 1) next.add(parts.slice(0, i).join("/"));
    expandedDirs = next;
  }

  function closeSkillSearch() {
    skillSearchOpen = false;
    skillSearchResultsOpen = false;
    skillSearchQuery = "";
    skillSearchTarget = { fileId: "", query: "", matchIndex: 0, version: nextSkillSearchVersion() };
  }

  function openSkillSearch() {
    if (activeTab !== "content" || !hasFileTreeNavigation) return;
    skillSearchOpen = true;
    skillSearchResultsOpen = true;
    tick().then(() => skillSearchInput?.focus());
  }

  export function focusModuleSearch() {
    openSkillSearch();
  }

  /** @param {any} result */
  async function activateSkillSearchResult(result) {
    const relativePath = result?.file?.id || result?.file?.path || "";
    if (!relativePath) return;
    const query = String(result?.query || skillSearchQuery || "").trim();
    const matchIndex = Number.isFinite(Number(result?.matchIndex)) ? Number(result.matchIndex) : 0;
    expandTreePath(relativePath);
    skillSearchResultsOpen = false;
    if (selectedFile !== relativePath) await selectFile(relativePath);
    skillSearchTarget = {
      fileId: relativePath,
      query,
      matchIndex,
      version: nextSkillSearchVersion(),
    };
  }

  /** @param {PointerEvent} event */
  function handleSkillSearchOutsidePointerDown(event) {
    if (!skill || !skillSearchResultsOpen) return;
    const target = event.target;
    if (skillSearchAnchor && target instanceof Node && skillSearchAnchor.contains(target)) return;
    skillSearchResultsOpen = false;
  }

  /** @param {any} member */
  function openPackageMember(member) {
    const memberPath = getPackageMemberPath(member);
    if (!memberPath) return;
    const targetPath = memberPath.endsWith("/SKILL.md") || memberPath === "SKILL.md"
      ? memberPath
      : `${memberPath}/SKILL.md`;
    activeTab = "content";
    expandTreePath(targetPath);
    const hasTarget = fileTree.some((file) => !file.is_dir && file.relative_path === targetPath);
    if (hasTarget || fileTree.length === 0) selectFile(targetPath);
  }

  /** @param {{ force?: boolean }} [options] */
  async function fetchLatestSkillEntry(options = {}) {
    if (!skill?.name) return null;
    try {
      const entry = options.force
        ? await scanSkillInventoryEntry(skill.name)
        : (await getSkillInventory())?.skills?.find((/** @type {any} */ item) => item.name === skill.name);
      const statuses = entry?.toolStatuses || entry?.tool_statuses;
      if (Array.isArray(statuses)) {
        const filteredStatuses = filterStatusesForManagedTools(statuses);
        fetchedToolStatuses = filteredStatuses;
        return {
          ...entry,
          tool_statuses: filteredStatuses,
          toolStatuses: filteredStatuses,
        };
      }
      if (options.force) throw $t("skills.viewer.refresh_after_write_failed");
    } catch {
      if (options.force) throw $t("skills.viewer.refresh_after_write_failed");
    }
    return null;
  }

  /** @param {{ force?: boolean }} [options] */
  async function fetchLatestToolStatuses(options = {}) {
    const entry = await fetchLatestSkillEntry(options);
    const statuses = entry?.toolStatuses || entry?.tool_statuses;
    if (Array.isArray(statuses)) return filterStatusesForManagedTools(statuses);
    return filterStatusesForManagedTools(fetchedToolStatuses || skill?.tool_statuses || skill?.toolStatuses || []);
  }

  /** @param {number} value */
  function padDatePart(value) {
    return String(value).padStart(2, "0");
  }

  /** @param {string | null | undefined} value */
  function formatLocalDateTime(value) {
    const raw = String(value || "").trim();
    if (!raw) return "";
    const date = new Date(raw);
    if (!Number.isFinite(date.getTime())) return "";
    return [
      `${date.getFullYear()}-${padDatePart(date.getMonth() + 1)}-${padDatePart(date.getDate())}`,
      `${padDatePart(date.getHours())}:${padDatePart(date.getMinutes())}`,
    ].join(" ");
  }

  /**
   * @param {Array<any>} statuses
   * @param {string | null} targetToolId
   */
  function buildCopyCandidates(statuses, targetToolId = null) {
    const candidates = [];
    for (const ts of Array.isArray(statuses) ? statuses : []) {
      const toolId = ts.tool_id || ts.toolId;
      if (targetToolId && toolId === targetToolId) continue;
      for (const source of displaySourcesForStatus(ts)) {
        const installed = isSkillStatusActionableSource(source.status || ts.status)
          && typeof source.path === "string"
          && source.path.length > 0;
        if (!installed || source.pathOrigin === "generic" || isLinkedSharedSource(source)) continue;
        candidates.push({
          key: sourceKey(source),
          tool_id: toolId,
          tool_name: source.tool_name || ts.tool_name || ts.toolName || getToolName(toolId, $tools) || toolId,
          path: source.path,
          sourceKind: "tool",
          versionLabel: formatLocalDateTime(source.updatedAt || ts.updated_at || ts.updatedAt)
            || $t("skills.viewer.source_version_current"),
        });
      }
    }
    return candidates;
  }

  /** @param {string | null} targetToolId */
  function copyCandidatesForTool(targetToolId = null) {
    return buildCopyCandidates(toolStatuses, targetToolId);
  }

  function getSharedInstallSourcePath() {
    return getContentSources(skill).find((source) => source.pathOrigin === "generic")?.path || "";
  }

  /** @param {any[]} candidates */
  async function openSourcePicker(candidates) {
    if (!Array.isArray(candidates) || candidates.length === 0) return null;
    if (sourcePickerResolver) {
      sourcePickerResolver(null);
      sourcePickerResolver = null;
    }
    sourcePickerCandidates = candidates;
    selectedSourceKey = candidates.length === 1 ? candidates[0].key : "";
    sourcePickerOpen = true;
    return new Promise((resolve) => {
      sourcePickerResolver = resolve;
    });
  }

  function cancelSourcePicker() {
    const resolver = sourcePickerResolver;
    sourcePickerOpen = false;
    sourcePickerCandidates = [];
    selectedSourceKey = "";
    sourcePickerResolver = null;
    if (resolver) resolver(null);
  }

  function confirmSourcePicker() {
    const selected = sourcePickerCandidates.find((item) => item.key === selectedSourceKey);
    if (!selected) return;
    const resolver = sourcePickerResolver;
    sourcePickerOpen = false;
    sourcePickerCandidates = [];
    selectedSourceKey = "";
    sourcePickerResolver = null;
    if (resolver) resolver(selected);
  }

  /** @param {string | null} targetToolId @param {any | null} [run] */
  async function resolveCopySourceContext(targetToolId = null, run = null) {
    if (!skill) return null;
    const statuses = await trackModulePerformanceRequest(run, "source-refresh", () => fetchLatestToolStatuses({ force: true }));
    const candidates = buildCopyCandidates(statuses, targetToolId);
    markModulePerformance(run, "copy-candidates-ready");
    if (candidates.length > 1) {
      markModulePerformance(run, "source-picker-open");
      const selected = await openSourcePicker(candidates);
      markModulePerformance(run, selected ? "source-picker-selected" : "source-picker-cancelled");
      if (!selected) return null;
      return {
        path: selected.path,
        action: "copy_tool",
        sourceTool: selected.tool_name,
        sourceVersion: selected.versionLabel || $t("skills.viewer.source_version_current"),
      };
    }
    if (candidates.length === 1) {
      const selected = candidates[0];
      return {
        path: selected.path,
        action: "copy_tool",
        sourceTool: selected.tool_name,
        sourceVersion: selected.versionLabel || $t("skills.viewer.source_version_current"),
      };
    }
    return null;
  }

  function resolveInstallSourceContext() {
    if (!skill) return null;
    const sharedPath = getSharedInstallSourcePath();
    if (!sharedPath) return null;
    return {
      path: sharedPath,
      action: "install",
      sourceTool: $t("skills.viewer.source_shared"),
      sourceVersion: $t("skills.viewer.source_version_current"),
    };
  }

  /**
   * @param {string} toolId
   * @param {{ action?: string, path?: string } | null} sourceContext
   */
  async function previewInstallForContext(toolId, sourceContext) {
    if (sourceContext?.action === "copy_tool") {
      return await previewCopySkillToTool(skill.name, toolId, sourceContext.path || "");
    }
    return await previewInstall(skill.name, toolId, "symlink", sourceContext?.path || null);
  }

  /**
   * @param {string} toolId
   * @param {{ action?: string, path?: string } | null} sourceContext
   */
  async function executeInstallForContext(toolId, sourceContext) {
    if (sourceContext?.action === "copy_tool") {
      return await executeCopySkillToTool(skill.name, toolId, sourceContext.path || "");
    }
    return await executeInstall(skill.name, toolId, "symlink", sourceContext?.path || null);
  }

  /**
   * @param {any} preview
   * @param {{ sourceTool?: string } | null | undefined} sourceContext
   */
  function withInstallSourceMessage(preview, sourceContext) {
    const normalized = parseOperationPreview(preview);
    if (!sourceContext) return normalized;
    const sourceLine = $t("skills.viewer.install_preview_source", {
      sourceTool: sourceContext.sourceTool || $t("skills.viewer.source_diff_unknown"),
    });
    normalized.message = normalized.message ? `${sourceLine}\n${normalized.message}` : sourceLine;
    return normalized;
  }

  async function refreshToolStatuses() {
    return await fetchLatestToolStatuses({ force: true });
  }

  /** @param {string} reason */
  function startSkillDetailOperationRun(reason) {
    const run = beginModulePerformanceRun({
      module: "skills",
      view: "detail",
      reason,
      counters: {
        contentSources: contentSourceOptions.length,
        toolStatuses: toolStatuses.length,
        installedTools: installedCount,
      },
    });
    markModulePerformance(run, "operation-start", { role: MODULE_PERFORMANCE_ROLE_VISIBLE });
    markModulePerformance(run, "operation-interactive", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
    return run;
  }

  /** @param {any | null | undefined} run @param {string} status @param {string} milestone */
  function finishSkillDetailOperationRun(run, status, milestone) {
    markModulePerformance(run, milestone, { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
    finishAndRecordModulePerformanceRun(run, status);
  }

  /** @param {any | null | undefined} run */
  async function refreshDetailAfterSourceWrite(run) {
    markSkillSourcesChanged();
    invalidateSkillInventory();
    const refreshedEntry = await trackModulePerformanceRequest(run, "detail-refresh", () =>
      fetchLatestSkillEntry({ force: true })
    );
    if (!refreshedEntry) throw $t("skills.viewer.refresh_after_write_failed");
    onChanged(refreshedEntry);
  }

  function markSkillSourcesChanged() {
    requireInventoryBackedContentSources = true;
  }

  /** @param {any | null | undefined} run */
  function cancelSkillDetailOperationRun(run) {
    finishSkillDetailOperationRun(run, "cancelled", "operation-cancelled");
  }

  /** @param {string} toolId */
  async function handleSingleInstall(toolId) {
    if (!skill || !canInstallSkillForTool(toolId)) return;
    loadingInstall = true;
    installMsg = "";
    try {
      const tool = $detectedTools.find((item) => item.id === toolId);
      if (!tool?.skills_dir) return;
      const sourceContext = resolveInstallSourceContext();
      if (!sourceContext) return;
      const preview = await previewInstallForContext(toolId, sourceContext).catch(() => null);
      if (preview) {
        const normalized = withInstallSourceMessage(preview, sourceContext);
        const confirmed = await confirmOperation(
          sourceContext.action === "copy_tool"
            ? $t("skills.viewer.copy_from_other_dialog_title")
            : $t("skills.viewer.install_dialog_title"),
          normalized,
          { confirmLabel: $t("skills.viewer.confirm_execute") }
        );
        if (!confirmed || !previewHasExecutableChanges(normalized)) return;
      }
      await executeInstallForContext(toolId, sourceContext);
      await refreshDetailAfterSourceWrite(null);
      installMsg = $t("skills.viewer.install_ok_with_source", {
        tool: tool.name,
        mode: $t("skills.viewer.path_origin_generic"),
        sourceTool: sourceContext.sourceTool,
        sourceVersion: sourceContext.sourceVersion,
      });
      installMsgType = "ok";
    } catch (e) {
      installMsg = $t("skills.viewer.install_fail", { err: String(e) });
      installMsgType = "err";
    } finally {
      loadingInstall = false;
      setTimeout(() => { installMsg = ""; }, 8000);
    }
  }

  /**
   * @param {string} toolId
   * @param {"uninstall"|"delete"} intent
   * @param {string | null} [sourcePath]
   */
  async function handleUninstall(toolId, intent = "delete", sourcePath = null) {
    if (!skill) return;
    if (intent === "delete" && !canDeleteSkillForTool(toolId)) return;
    if (intent !== "delete" && !canUninstallSkillForTool(toolId)) return;
    loadingInstall = true;
    installMsg = "";
    const isDeleteAction = intent === "delete";
    const run = startSkillDetailOperationRun(isDeleteAction ? "delete-from-tool" : "uninstall-from-tool");
    const previewLabel = isDeleteAction ? "delete-preview" : "uninstall-preview";
    const executeLabel = isDeleteAction ? "delete-execute" : "uninstall-execute";
    try {
      const preview = await trackModulePerformanceRequest(run, previewLabel, () => (
        isDeleteAction
          ? previewDeleteFromTool(skill.name, toolId, sourcePath)
          : previewUninstall(skill.name, toolId)
      ).catch(() => null));
      if (!preview) markModulePerformance(run, "preview-unavailable");
      if (preview) {
        const confirmed = await confirmOperation(
          $t(isDeleteAction ? "skills.viewer.delete_from_tool_dialog_title" : "skills.viewer.uninstall_dialog_title"),
          preview,
          { variant: "danger" }
        );
        if (!confirmed || !previewHasExecutableChanges(preview)) {
          cancelSkillDetailOperationRun(run);
          return;
        }
        markModulePerformance(run, "confirmation-accepted");
      }
      await trackModulePerformanceRequest(run, executeLabel, () => (
        isDeleteAction
          ? executeDeleteFromTool(skill.name, toolId, sourcePath)
          : executeUninstall(skill.name, toolId)
      ));
      await refreshDetailAfterSourceWrite(run);
      finishSkillDetailOperationRun(run, "success", "operation-complete");
      installMsg = $t(isDeleteAction ? "skills.viewer.delete_from_tool_ok" : "skills.viewer.uninstall_ok");
      installMsgType = "ok";
    } catch (e) {
      finishSkillDetailOperationRun(run, "failed", "operation-failed");
      installMsg = $t(
        isDeleteAction ? "skills.viewer.delete_from_tool_fail" : "skills.viewer.uninstall_fail",
        { err: String(e) }
      );
      installMsgType = "err";
    } finally {
      loadingInstall = false;
      setTimeout(() => { installMsg = ""; }, 8000);
    }
  }

  /**
   * @param {string} targetToolId
   * @param {string} sourcePath
   * @param {string} sourceToolName
   * @param {any | null} [existingRun]
   */
  async function handleCopyFromTool(targetToolId, sourcePath, sourceToolName, existingRun = null) {
    const run = existingRun || startSkillDetailOperationRun("copy-from-tool");
    if (!skill || !targetToolId || !sourcePath || !canCopySkillForTool(targetToolId)) {
      cancelSkillDetailOperationRun(run);
      return;
    }
    loadingInstall = true;
    installMsg = "";
    try {
      const preview = await trackModulePerformanceRequest(run, "copy-preview", () =>
        previewCopySkillToTool(skill.name, targetToolId, sourcePath)
      );
      const title = $t("skills.viewer.btn_copy_from_tool", { tool: sourceToolName || "" });
      const confirmed = await confirmOperation(title, preview, { confirmLabel: $t("skills.viewer.confirm_execute") });
      if (!confirmed || !previewHasExecutableChanges(preview)) {
        cancelSkillDetailOperationRun(run);
        return;
      }
      markModulePerformance(run, "confirmation-accepted");
      await trackModulePerformanceRequest(run, "copy-execute", () =>
        executeCopySkillToTool(skill.name, targetToolId, sourcePath)
      );
      await refreshDetailAfterSourceWrite(run);
      finishSkillDetailOperationRun(run, "success", "operation-complete");
      installMsg = $t("skills.viewer.copy_from_tool_ok", {
        source: sourceToolName || "",
        target: getToolName(targetToolId, $tools) || targetToolId,
      });
      installMsgType = "ok";
    } catch (e) {
      finishSkillDetailOperationRun(run, "failed", "operation-failed");
      installMsg = $t("skills.viewer.copy_from_tool_fail", { err: String(e) });
      installMsgType = "err";
    } finally {
      loadingInstall = false;
      setTimeout(() => { installMsg = ""; }, 8000);
    }
  }

  /** @param {string} targetToolId */
  async function handleCopyFromOtherTool(targetToolId) {
    const run = startSkillDetailOperationRun("copy-from-tool");
    try {
      const sourceContext = await resolveCopySourceContext(targetToolId, run);
      if (!sourceContext) {
        cancelSkillDetailOperationRun(run);
        return;
      }
      await handleCopyFromTool(targetToolId, sourceContext.path, sourceContext.sourceTool, run);
    } catch (e) {
      finishSkillDetailOperationRun(run, "failed", "operation-failed");
      installMsg = $t("skills.viewer.copy_from_tool_fail", { err: String(e) });
      installMsgType = "err";
      setTimeout(() => { installMsg = ""; }, 8000);
    }
  }

  function startDuplicateSourceRename() {
    if (!selectedDuplicateKeepPath || !skill?.name || !canRenameDuplicateSelection()) return;
    duplicateRenameName = `${skill.name}-local`;
    duplicateRenameActive = true;
  }

  function cancelDuplicateSourceRename() {
    duplicateRenameActive = false;
    duplicateRenameName = "";
  }

  async function confirmDuplicateSourceRename() {
    if (!skill || !selectedDuplicateKeepPath) return;
    const renamed = duplicateRenameName.trim();
    if (!renamed) return;
    loadingInstall = true;
    installMsg = "";
    try {
      const preview = await previewRenameSkillSource(skill.name, selectedDuplicateKeepPath, renamed);
      const confirmed = await confirmOperation(
        $t("skills.viewer.rename_local_source"),
        preview,
        { confirmLabel: $t("skills.viewer.confirm_execute") }
      );
      if (!confirmed || !previewHasExecutableChanges(preview)) return;
      await executeRenameSkillSource(skill.name, selectedDuplicateKeepPath, renamed);
      await refreshDetailAfterSourceWrite(null);
      closeDuplicateHandling();
      installMsg = $t("skills.viewer.rename_source_ok", { name: renamed });
      installMsgType = "ok";
    } catch (e) {
      installMsg = $t("skills.viewer.rename_source_fail", { err: String(e) });
      installMsgType = "err";
    } finally {
      loadingInstall = false;
      setTimeout(() => { installMsg = ""; }, 8000);
    }
  }

  /** @param {any} status */
  function openDuplicateHandling(status) {
    const sources = displaySourcesForStatus(status);
    duplicateHandlingStatus = status;
    duplicateHandlingSources = sources;
    selectedDuplicateKeepPath = sources[0]?.path || "";
    duplicateRenameActive = false;
    duplicateRenameName = "";
    duplicateHandlingOpen = true;
  }

  function closeDuplicateHandling() {
    duplicateHandlingOpen = false;
    duplicateHandlingStatus = null;
    duplicateHandlingSources = [];
    selectedDuplicateKeepPath = "";
    duplicateRenameActive = false;
    duplicateRenameName = "";
  }

  function duplicateDeletePaths() {
    return duplicateDeleteSources().map((source) => source.path).filter(Boolean);
  }

  function duplicateDeleteSources() {
    return duplicateHandlingSources.filter((source) => source.path && source.path !== selectedDuplicateKeepPath);
  }

  function canKeepDuplicateSelection() {
    return Boolean(selectedDuplicateKeepPath)
      && duplicateDeleteSources().length > 0
      && duplicateDeleteSources().every((source) => canDeleteDuplicateSource(source, duplicateHandlingStatus));
  }

  function selectedDuplicateSource() {
    return duplicateHandlingSources.find((source) => source.path === selectedDuplicateKeepPath) || null;
  }

  function canRenameDuplicateSelection() {
    const source = selectedDuplicateSource();
    return Boolean(source && canRenameDuplicateSource(source, duplicateHandlingStatus));
  }

  async function handleKeepDuplicateSource() {
    if (!skill || !selectedDuplicateKeepPath || !canKeepDuplicateSelection()) return;
    const deletePaths = duplicateDeletePaths();
    if (deletePaths.length === 0) return;
    loadingInstall = true;
    installMsg = "";
    try {
      const preview = await previewCleanupDuplicateSkillSources(skill.name, selectedDuplicateKeepPath, deletePaths);
      const confirmed = await confirmOperation(
        $t("skills.viewer.abnormal_keep_confirm_title"),
        preview,
        { variant: "danger", confirmLabel: $t("skills.viewer.confirm_execute") }
      );
      if (!confirmed || !previewHasExecutableChanges(preview)) return;
      await executeCleanupDuplicateSkillSources(skill.name, selectedDuplicateKeepPath, deletePaths);
      await refreshDetailAfterSourceWrite(null);
      closeDuplicateHandling();
      installMsg = $t("skills.viewer.abnormal_keep_ok");
      installMsgType = "ok";
    } catch (e) {
      installMsg = $t("skills.viewer.abnormal_keep_fail", { err: String(e) });
      installMsgType = "err";
    } finally {
      loadingInstall = false;
      setTimeout(() => { installMsg = ""; }, 8000);
    }
  }

  function requestClose() {
    if (typeof onClose === "function") onClose();
  }

  function handleEscapeClose() {
    if (!skill || confirmOpen) return;
    if (duplicateHandlingOpen) {
      closeDuplicateHandling();
      return;
    }
    if (sourcePickerOpen) {
      cancelSourcePicker();
      return;
    }
    if (skillSearchOpen) {
      closeSkillSearch();
      return;
    }
    requestClose();
  }

  /** @param {number} count */
  function formatLineLabel(count) {
    return $t("skills.viewer.lines", { count });
  }

  async function handleDeleteSkill() {
    if (!skill) return;
    deletingSkill = true;
    try {
      const preview = await previewDelete(skill.name, true);
      const confirmed = await confirmOperation(`${$t("skills.delete.btn")}: ${skill.name}`, preview, { variant: "danger" });
      if (!confirmed || !previewHasExecutableChanges(preview)) return;
      const result = await executeDelete(skill.name, true);
      invalidateSkillInventory();
      onDelete(
        skill.name,
        result.deletes || result.pathsToDelete || result.paths_to_delete || result.deletedPaths || result.deleted_paths || []
      );
      onClose();
    } catch (e) {
      installMsg = $t("skills.delete.failed", { err: String(e) });
      installMsgType = "err";
      setTimeout(() => { installMsg = ""; }, 8000);
    } finally {
      deletingSkill = false;
    }
  }
</script>

<svelte:window onpointerdown={handleSkillSearchOutsidePointerDown} onkeydown={(e) => {
  if (!skill || e.key !== "Escape") return;
  handleEscapeClose();
}} />

{#if skill}
  <div class="overlay" role="presentation" tabindex="-1">
    <div class="viewer-modal" role="dialog" aria-modal="true" tabindex="0">
      <div
        class="view-header management-header-row skill-viewer-management-header"
        class:skill-viewer-management-header--search-popover={skillSearchOpen && activeTab === "content" && hasFileTreeNavigation}
        data-tauri-drag-region
      >
        <div class="skill-viewer-header-side skill-viewer-header-side--start">
          <div class="management-header-title skill-viewer-header-title-block" data-tauri-drag-region>
            <div class="header-left">
              <div class="icon-box">
                <Blocks size={18} color="currentColor" />
              </div>
              <div class="viewer-info">
                <div class="viewer-title-row">
                  <span class="viewer-title">{skill.display_name || skill.name}</span>
                </div>
                {#if totalToolCount > 0}
                  <div class="viewer-subtitle-row">
                    <span class="install-count">{$t("skills.viewer.installed_count", { installed: installedCount, total: totalToolCount })}</span>
                  </div>
                {/if}
              </div>
            </div>
          </div>
        </div>

        <div
          class="level1-tabs"
          class:level1-tabs--skill-viewer-pair={!isPackageSkill}
          class:level1-tabs--skill-viewer-triple={isPackageSkill}
        >
          <button type="button" class="l1-tab" class:active={activeTab === "content"} onclick={() => activeTab = "content"}>{$t("skills.viewer.tab_content")}</button>
          {#if isPackageSkill}
            <button type="button" class="l1-tab" class:active={activeTab === "members"} onclick={() => activeTab = "members"}>{$t("skills.viewer.tab_members")}</button>
          {/if}
          <button type="button" class="l1-tab" class:active={activeTab === "install"} onclick={() => activeTab = "install"}>{$t("skills.viewer.tab_install")}</button>
        </div>

        <div class="skill-viewer-header-side skill-viewer-header-side--end">
          <div class="header-actions management-header-actions">
            {#if activeTab === "content" && hasFileTreeNavigation}
              {#if skillSearchOpen}
                <div class="workspace-search-anchor skill-viewer-search-anchor" bind:this={skillSearchAnchor}>
                  <div class="search-input-wrap skill-viewer-search-wrap">
                    <span class="search-input-wrap__icon" aria-hidden="true"><Search size={13} strokeWidth={1.8} /></span>
                    <input
                      bind:this={skillSearchInput}
                      class="search-input"
                      type="text"
                      placeholder={$t("file_workspace_search.placeholder_tree")}
                      bind:value={skillSearchQuery}
                      onpointerdown={() => { skillSearchResultsOpen = true; }}
                      onfocus={() => { skillSearchResultsOpen = true; }}
                      oninput={() => { skillSearchResultsOpen = true; }}
                    />
                    <button class="icon-btn-tiny" onclick={closeSkillSearch} aria-label={$t("global.search.close")}>
                      <X size={13} />
                    </button>
                  </div>
                  <FileWorkspaceSearchResults
                    query={skillSearchQuery}
                    groups={skillSearchGroups}
                    visible={canShowSkillSearchResults}
                    dismissRoot={skillSearchAnchor}
                    onActivate={activateSkillSearchResult}
                    onDismiss={() => (skillSearchResultsOpen = false)}
                  />
                </div>
              {:else}
                <Tooltip label={$t("global.search.title")} placement="bottom-end">
                  <button class="icon-btn" onclick={openSkillSearch} aria-label={$t("global.search.title")}><Search size={18} /></button>
                </Tooltip>
              {/if}
            {/if}
            <Tooltip label={$t("skills.viewer.close")} placement="bottom-end">
              <button class="icon-btn" onclick={requestClose} aria-label={$t("skills.viewer.close")}><X size={18} /></button>
            </Tooltip>
          </div>
        </div>
      </div>

      {#if activeTab === "install"}
        <div class="install-tab">
          {#if installMsg}
            <div class="install-msg" class:install-msg-err={installMsgType === "err"}>{installMsg}</div>
          {/if}

          <div class="install-section">
            <div class="section-title-row">
              <div class="section-title">{$t("skills.viewer.tool_status")}</div>
            </div>
            {#if toolStatuses.length === 0}
              <div class="install-empty">{$t("skills.viewer.no_tools")}</div>
            {:else}
              <div class="tool-status-list">
                {#each toolStatuses as ts}
                  {@const canInstallToolSkill = !!ts.canInstallSkill}
                  {@const canCopyToolSkill = !!ts.canCopySkill}
                  {@const canUninstallToolSkill = !!ts.canUninstallSkill}
                  {@const copyCandidates = copyCandidatesForTool(ts.tool_id)}
                  {@const sharedInstallPath = getSharedInstallSourcePath()}
                  {@const duplicateStatus = hasDuplicateSources(ts)}
                  {@const displayedSources = displaySourcesForStatus(ts)}
                  {@const hasCopyAction = canCopyToolSkill && copyCandidates.length > 0}
                  {@const installUnavailableReason = getInstallUnavailableReason(ts, sharedInstallPath, hasCopyAction)}
                  <div class="tool-status-item" class:installed={ts.status === "installed"}>
                    <div class="tool-status-header">
                      <div class="tool-status-left">
                        <ToolIcon toolId={ts.tool_id} size={18} />
                        <span class="tool-status-name">{ts.name || ts.tool_id}</span>
                        {#if duplicateStatus}
                          <Tooltip label={$t("skills.viewer.abnormal_marker_tooltip")} placement="left" maxWidth="280px">
                            <button type="button" class="tool-status-warning-marker" aria-label={$t("skills.viewer.abnormal_marker_tooltip")}>
                              <AlertCircle size={14} strokeWidth={1.9} />
                            </button>
                          </Tooltip>
                        {/if}
                        {#if installUnavailableReason}
                          <Tooltip label={installUnavailableReason} placement="left" maxWidth="280px">
                            <button type="button" class="tool-status-warning-marker" aria-label={installUnavailableReason}>
                              <AlertCircle size={14} strokeWidth={1.9} />
                            </button>
                          </Tooltip>
                        {/if}
                      </div>
                      <div class="tool-status-right">
                        {#if ts.status === "installed"}
                          {#if duplicateStatus}
                            <button class="btn-action-install" onclick={() => openDuplicateHandling(ts)} disabled={loadingInstall || !canHandleDuplicateStatus(ts)}>{$t("skills.viewer.abnormal_action")}</button>
                          {:else if canUninstallToolSkill && isToolDirectorySymlink(ts)}
                            <button class="btn-action-install" onclick={() => handleUninstall(ts.tool_id, "uninstall")} disabled={loadingInstall}>{$t("skills.viewer.uninstall")}</button>
                          {:else if canDeleteToolDirectorySource(ts)}
                            <button class="btn-action-danger" onclick={() => handleUninstall(ts.tool_id, "delete", ts.path)} disabled={loadingInstall}>{$t("skills.viewer.delete_from_tool")}</button>
                          {/if}
                        {:else if hasContentSources}
                          {#if canInstallToolSkill && sharedInstallPath}
                            <button class="btn-action-install" onclick={() => handleSingleInstall(ts.tool_id)} disabled={loadingInstall}>{$t("skills.viewer.install")}</button>
                          {/if}
                          {#if canCopyToolSkill && copyCandidates.length > 0}
                            <button class="btn-action-install" onclick={() => handleCopyFromOtherTool(ts.tool_id)} disabled={loadingInstall}>{$t("skills.viewer.btn_copy_from_other")}</button>
                          {/if}
                        {/if}
                      </div>
                    </div>
                    {#if ts.rawStatus === "variantDrifted"}
                      <div class="tool-status-warning">{$t("skills.viewer.alert_metadata_drift")}</div>
                    {/if}
                    {#if displayedSources.length > 0}
                      <div class="tool-status-path-list">
                        {#each displayedSources as source (sourceKey(source))}
                          <div class="tool-status-path-container">
                            <div class="tool-status-path">{source.path}</div>
                            {#if source.pathOrigin === "generic"}
                              <Tooltip label={$t("skills.viewer.path_origin_generic_tooltip")} placement="bottom">
                                <span class="tool-status-origin-badge">{$t("skills.viewer.path_origin_generic")}</span>
                              </Tooltip>
                            {/if}
                            {#if source.target_path}
                              <span class="symlink-arrow">-&gt;</span>
                              <span class="symlink-target-path">{source.target_path}</span>
                            {/if}
                          </div>
                        {/each}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <div class="delete-skill-section">
            <div class="delete-skill-warning">{$t("skills.delete.warning")}</div>
            <button class="btn-delete-skill" onclick={handleDeleteSkill} disabled={deletingSkill}>
              <Trash2 size={14} />
              {deletingSkill ? $t("skills.delete.deleting") : $t("skills.delete.btn")}
            </button>
          </div>
        </div>
      {:else if activeTab === "members"}
        <div class="package-members-tab">
          <div class="package-members-shell">
            <div class="package-members-header">
              <div>
                <div class="package-members-title">{$t("skills.viewer.package_members")}</div>
                <div class="package-members-subtitle">{$t("skills.viewer.package_members_hint")}</div>
              </div>
              <span class="package-member-count">{$t("skills.card.package_members", { count: packageMembers.length })}</span>
            </div>
            <div class="package-member-groups">
              {#each packageMemberGroups as group}
                <section class="package-member-group">
                  <div class="package-member-group-header">
                    <span>{packageMemberKindLabel(group.kind)}</span>
                    <span>{$t("skills.card.package_members", { count: group.items.length })}</span>
                  </div>
                  <div class="package-member-list">
                    {#each group.items as member}
                      <button
                        type="button"
                        class="package-member-item"
                        onclick={() => openPackageMember(member)}
                      >
                        <span class="package-member-name">{getPackageMemberName(member)}</span>
                        <span class="package-member-path">{getPackageMemberPath(member)}</span>
                      </button>
                    {/each}
                  </div>
                </section>
              {/each}
            </div>
          </div>
        </div>
      {:else}
        <div class="viewer-main-container">
          <FileViewerShell
            variant={hasFileTreeNavigation ? "navigation-backed" : "single-file"}
            title={selectedFile || $t("skills.viewer.no_files_title")}
            subtitle={selectedFilePath || activeSkillPath || skill.path || ""}
            badge=""
            modeLabel={editingFile ? $t("skills.viewer.editing") : ""}
            navigationVisible={Boolean(activeSkillPath && hasFileTreeNavigation)}
            navigationCollapsible={Boolean(hasFileTreeNavigation)}
            bind:navigationCollapsed={skillFileNavigationCollapsed}
            navigationResizeLabel={$t("global.a11y.resize_skill_file_tree")}
            navigationCollapseLabel={$t("global.a11y.collapse_file_tree")}
            navigationExpandLabel={$t("global.a11y.expand_file_tree")}
            contextVisible={hasExtraVariants}
          >
            {#snippet navigation()}
              <FileWorkspaceNavigation
                title={$t("skills.viewer.files")}
                items={skillFileNavigationItems}
                onToggle={toggleSkillFileNavigationItem}
                onSelect={selectSkillFileNavigationItem}
              />
            {/snippet}

            {#snippet actions()}
              {#if editingFile}
                <span class="file-viewer-inline-status">
                  {#if isFileDirty}{$t("skills.viewer.unsaved")} · {/if}{$t("skills.viewer.lines", { count: selectedFileLineCount })}
                </span>
                <button type="button" class="file-viewer-btn" onclick={cancelEditFile} disabled={savingFile}>
                  {$t("skills.viewer.cancel")}
                </button>
                <button type="button" class="file-viewer-primary-btn" onclick={() => saveEditFile(editContent)} disabled={savingFile || !isFileDirty}>
                  {savingFile ? $t("skills.viewer.saving") : $t("skills.viewer.save")}
                </button>
              {:else if translationResult}
                <span class="file-viewer-inline-status">{$t("skills.viewer.lines", { count: selectedFileLineCount })}</span>
                <button
                  type="button"
                  class="file-viewer-btn translation-toolbar-btn translation-toolbar-btn--active"
                  aria-label={$t("skills.viewer.translation_action")}
                  aria-pressed="true"
                  onclick={clearTranslationResult}
                >
                  <Languages size={14} />
                </button>
                <button type="button" class="file-viewer-btn" onclick={clearTranslationResult}>
                  <ArrowLeft size={14} /> {$t("skills.viewer.translation_back")}
                </button>
              {:else}
                {#if showFileModeToggle}
                  <SourceReadModeToggle
                    bind:preview={mdPreview}
                    editing={editingFile}
                    sourceLabel={$t("skills.viewer.mode_src")}
                    readLabel={$t("skills.viewer.mode_read")}
                  />
                {/if}
                {#if canTranslateCurrentSkillFile}
                  <div class="translation-action-wrap">
                    <button
                      type="button"
                      class="file-viewer-btn translation-toolbar-btn"
                      class:translation-toolbar-btn--active={translationMenuOpen}
                      aria-label={$t("skills.viewer.translation_action")}
                      aria-pressed={translationMenuOpen}
                      onclick={() => (translationMenuOpen = !translationMenuOpen)}
                      disabled={translatingFile}
                    >
                      <Languages size={14} />
                    </button>
                    {#if translationMenuOpen}
                      <div class="translation-menu" role="menu" aria-label={$t("skills.viewer.translation_target")}>
                        <button type="button" role="menuitem" onclick={() => translateCurrentFile("zh-CN")}>
                          {$t("skills.viewer.translation_target_zh")}
                        </button>
                        <button type="button" role="menuitem" onclick={() => translateCurrentFile("en")}>
                          {$t("skills.viewer.translation_target_en")}
                        </button>
                      </div>
                    {/if}
                  </div>
                {/if}
                {#if translatingFile}
                  <span class="file-viewer-inline-status">{$t("skills.viewer.translating")}</span>
                {:else if translationMsg}
                  <span class="file-viewer-inline-status translation-error">{translationMsg}</span>
                {/if}
                {#if canEditCurrentSkillFile && hasFileTreeNavigation}
                  <button type="button" class="file-viewer-btn" onclick={startEditFile}>
                    <Pencil size={14} /> {$t("skills.viewer.edit")}
                  </button>
                {/if}
              {/if}
            {/snippet}

            {#snippet context()}
              <div class="source-selector-row">
                <span class="variant-switcher-label">{$t("skills.viewer.content_source_label")}</span>
                <SourceSelectorCapsule
                  sources={contentSourceOptions}
                  activeKey={selectedVariant}
                  ariaLabel={$t("skills.viewer.content_source_label")}
                  onSelect={switchVariant}
                />
                {#if getContentSources(skill).find((source) => source.key === selectedVariant)?.pathOrigin === "generic"}
                  <span class="variant-usage-hint">{getSelectedSharedUsageText()}</span>
                {/if}
              </div>
            {/snippet}

            <div class="viewer-body">
              {#if loadingContent && !fileContent}
                <div class="content-scrollable content-scrollable--padded">
                  <div class="content-loading">{$t("skills.viewer.loading")}</div>
                </div>
              {:else if !hasFileTreeNavigation}
                <PrimaryState
                  message={$t("skills.viewer.no_files_title")}
                  detail={$t("skills.viewer.no_files_detail")}
                />
              {:else if translationResult}
                <ContentEditor
                  value={translationResult.content}
                  bind:preview={mdPreview}
                  originalValue={translationResult.content}
                  editing={false}
                  markdown={isMarkdown}
                  filename={selectedFile}
                  fill={true}
                  minHeight="0"
                  showModeToggle={false}
                  framed={false}
                  sourceLabel={$t("skills.viewer.mode_src")}
                  readLabel={$t("skills.viewer.mode_read")}
                  editLabel={$t("skills.viewer.edit")}
                  cancelLabel={$t("skills.viewer.cancel")}
                  saveLabel={$t("skills.viewer.save")}
                  savingLabel={$t("skills.viewer.saving")}
                  unsavedLabel={$t("skills.viewer.unsaved")}
                  saving={false}
                  showActions={false}
                  showFooter={false}
                  formatPreviewContent={formatFrontmatterForMarkdown}
                  formatLineLabel={formatLineLabel}
                  externalSearchQuery=""
                  externalSearchMatchIndex={0}
                  externalSearchVersion={0}
                />
              {:else}
                <ContentEditor
                  bind:value={editContent}
                  bind:preview={mdPreview}
                  originalValue={fileContent}
                  editing={editingFile && canEditCurrentSkillFile}
                  markdown={isMarkdown}
                  filename={selectedFile}
                  fill={true}
                  minHeight="0"
                  showModeToggle={false}
                  framed={false}
                  sourceLabel={$t("skills.viewer.mode_src")}
                  readLabel={$t("skills.viewer.mode_read")}
                  editLabel={$t("skills.viewer.edit")}
                  cancelLabel={$t("skills.viewer.cancel")}
                  saveLabel={$t("skills.viewer.save")}
                  savingLabel={$t("skills.viewer.saving")}
                  unsavedLabel={$t("skills.viewer.unsaved")}
                  saving={savingFile}
                  showActions={false}
                  showFooter={false}
                  formatPreviewContent={formatFrontmatterForMarkdown}
                  formatLineLabel={formatLineLabel}
                  onEdit={startEditFile}
                  onCancel={cancelEditFile}
                  onSave={saveEditFile}
                  externalSearchQuery={activeSkillExternalSearchQuery}
                  externalSearchMatchIndex={activeSkillExternalSearchMatchIndex}
                  externalSearchVersion={activeSkillExternalSearchVersion}
                />
              {/if}
            </div>
          </FileViewerShell>
        </div>
      {/if}

      {#if duplicateHandlingOpen}
        <div class="batch-overlay" role="presentation" tabindex="-1">
          <div class="abnormal-modal" role="dialog" aria-modal="true" tabindex="0">
            <div class="batch-header">
              <span class="batch-title">{$t("skills.viewer.abnormal_title")}</span>
              <button class="icon-btn" onclick={closeDuplicateHandling}><X size={16} /></button>
            </div>
            <div class="source-picker-body">
              <div class="source-picker-desc">{$t("skills.viewer.abnormal_desc")}</div>
              <div class="abnormal-source-list">
                {#each duplicateHandlingSources as source (sourceKey(source))}
                  <label class="abnormal-source-item" class:active={selectedDuplicateKeepPath === source.path}>
                    <input
                      type="radio"
                      name="duplicate-source-keep"
                      value={source.path}
                      bind:group={selectedDuplicateKeepPath}
                    />
                    <div class="abnormal-source-main">
                      <div class="abnormal-source-title-row">
                        <span class="abnormal-source-title">{sourceLabel(source)}</span>
                        {#if source.pathOrigin === "generic"}
                          <Tooltip label={$t("skills.viewer.path_origin_generic_tooltip")} placement="bottom">
                            <span class="tool-status-origin-badge">{$t("skills.viewer.path_origin_generic")}</span>
                          </Tooltip>
                        {/if}
                      </div>
                      <div class="abnormal-source-path">
                        {source.path}
                        {#if sourceTargetPath(source)}
                          <span class="symlink-arrow">-&gt;</span>
                          <span class="symlink-target-path">{sourceTargetPath(source)}</span>
                        {/if}
                      </div>
                    </div>
                  </label>
                {/each}
              </div>
              {#if duplicateRenameActive}
                <div class="abnormal-rename-panel">
                  <label class="abnormal-rename-label" for="duplicate-source-rename">
                    {$t("skills.viewer.abnormal_rename_name_label")}
                  </label>
                  <div class="abnormal-rename-row">
                    <input
                      id="duplicate-source-rename"
                      class="abnormal-rename-input"
                      type="text"
                      bind:value={duplicateRenameName}
                      aria-label={$t("skills.viewer.abnormal_rename_name_label")}
                    />
                    <button class="btn-text" onclick={cancelDuplicateSourceRename} disabled={loadingInstall}>
                      {$t("skills.viewer.cancel")}
                    </button>
                    <button
                      class="primary-pill-btn"
                      onclick={confirmDuplicateSourceRename}
                      disabled={!duplicateRenameName.trim() || loadingInstall}
                    >
                      {$t("skills.viewer.abnormal_rename_confirm")}
                    </button>
                  </div>
                </div>
              {/if}
            </div>
            <div class="batch-footer">
              <div></div>
              <div class="batch-footer-actions">
                <button class="btn-text" onclick={closeDuplicateHandling}>{$t("skills.viewer.cancel")}</button>
                <button class="btn-text" onclick={startDuplicateSourceRename} disabled={!canRenameDuplicateSelection() || loadingInstall}>
                  {$t("skills.viewer.abnormal_rename")}
                </button>
                <button class="primary-pill-btn" onclick={handleKeepDuplicateSource} disabled={!canKeepDuplicateSelection() || loadingInstall}>
                  {$t("skills.viewer.abnormal_keep")}
                </button>
              </div>
            </div>
          </div>
        </div>
      {/if}

      {#if sourcePickerOpen}
        <div class="batch-overlay" role="presentation" tabindex="-1">
          <div class="source-picker-modal" role="dialog" aria-modal="true" tabindex="0">
            <div class="batch-header">
              <span class="batch-title">{$t("skills.viewer.source_picker_title")}</span>
              <button class="icon-btn" onclick={cancelSourcePicker}><X size={16} /></button>
            </div>
            <div class="source-picker-body">
              <div class="source-picker-desc">{$t("skills.viewer.source_picker_desc")}</div>
              <div class="source-picker-list">
                {#each sourcePickerCandidates as source}
                  <button
                    class="source-picker-item"
                    class:active={selectedSourceKey === source.key}
                    onclick={() => { selectedSourceKey = source.key; }}
                  >
                    <div class="source-picker-row">
                      <div class="source-picker-main">
                        <ToolIcon toolId={source.tool_id} size={16} />
                        <span class="source-picker-name">{source.tool_name}</span>
                      </div>
                      <span class="source-picker-time">{source.versionLabel || ""}</span>
                    </div>
                    <div class="source-picker-path">{source.path}</div>
                    {#if source.sourceKind === "shared" && Array.isArray(source.consumerTools) && source.consumerTools.length > 0}
                      <div class="source-picker-path">{$t("skills.viewer.source_picker_shared_consumers", { tools: source.consumerTools.join("、") })}</div>
                    {/if}
                  </button>
                {/each}
              </div>
            </div>
            <div class="batch-footer">
              <div></div>
              <div class="batch-footer-actions">
                <button class="btn-text" onclick={cancelSourcePicker}>{$t("skills.viewer.source_picker_cancel")}</button>
                <button class="primary-pill-btn" onclick={confirmSourcePicker} disabled={!selectedSourceKey}>
                  {$t("skills.viewer.source_picker_confirm")}
                </button>
              </div>
            </div>
          </div>
        </div>
      {/if}
    </div>
  </div>

  <ConfirmDialog bind:this={confirmDialog} />
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    backdrop-filter: var(--overlay-filter);
    -webkit-backdrop-filter: var(--overlay-filter);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 36px 48px;
    box-sizing: border-box;
  }
  .viewer-modal {
    width: min(1120px, calc(100vw - 96px));
    height: min(860px, calc(100vh - 72px));
    max-height: calc(100vh - 72px);
    background: var(--bg-card);
    color: var(--color-text-main);
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-floating);
    border: 1px solid var(--border-color);
    border-radius: 16px;
    overflow: hidden;
  }
  .skill-viewer-management-header {
    height: 76px;
    flex-shrink: 0;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 16px;
    padding: 0 28px;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-panel);
  }
  .skill-viewer-management-header--search-popover {
    position: relative;
    z-index: 30;
    overflow: visible;
  }
  .skill-viewer-header-side {
    min-width: 0;
    display: flex;
    align-items: center;
  }
  .skill-viewer-header-side--start { justify-content: flex-start; }
  .skill-viewer-header-side--end { justify-content: flex-end; }
  .header-left {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .icon-box {
    width: 34px;
    height: 34px;
    border-radius: 8px;
    background: var(--bg-hover);
    color: var(--color-text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .viewer-info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .viewer-title-row {
    min-width: 0;
    display: flex;
    align-items: center;
  }
  .viewer-title {
    font-size: 16px;
    font-weight: 700;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .viewer-subtitle-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-size: 11px;
    color: var(--color-text-muted);
  }
  .tool-status-origin-badge {
    display: inline-flex;
    align-items: center;
    min-height: 22px;
    padding: 0 9px;
    border-radius: 999px;
    background: var(--bg-elevated-subtle);
    border: 1px solid var(--border-color);
    color: var(--color-text-muted);
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
  }
  .level1-tabs {
    display: flex;
    align-items: center;
    background: var(--bg-elevated-subtle);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 3px;
    gap: 2px;
  }
  .l1-tab {
    min-height: 30px;
    padding: 0 14px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--color-text-muted);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .l1-tab.active {
    background: var(--bg-card);
    color: var(--color-text-main);
    box-shadow: var(--shadow-soft);
  }
  .icon-btn {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    border: none;
    background: transparent;
    color: var(--color-text-muted);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    opacity: 0.42;
    transition: opacity 0.15s ease, background 0.15s ease, color 0.15s ease;
  }
  .icon-btn:hover,
  .icon-btn:focus-visible {
    opacity: 1;
    background: var(--bg-hover);
    color: var(--color-text-main);
  }
  .icon-btn:focus-visible {
    outline: 1px solid var(--toolbar-control-border-active);
    outline-offset: 1px;
  }
  .install-tab,
  .package-members-tab,
  .viewer-main-container {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 28px 40px;
    background: var(--bg-page);
  }
  .viewer-main-container {
    display: flex;
    overflow: hidden;
  }
  .install-msg {
    margin-bottom: 14px;
    border: 1px solid rgba(34, 197, 94, 0.3);
    background: rgba(34, 197, 94, 0.08);
    color: #16a34a;
    border-radius: 8px;
    padding: 10px 12px;
    font-size: 12px;
  }
  .install-msg-err {
    border-color: rgba(248, 113, 113, 0.35);
    background: rgba(248, 113, 113, 0.08);
    color: #ef4444;
  }
  .install-section,
  .delete-skill-section,
  .package-members-shell {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 18px;
  }
  .delete-skill-section { margin-top: 16px; }
  .section-title-row,
  .tool-status-header,
  .tool-status-left,
  .tool-status-right,
  .source-selector-row,
  .batch-header,
  .batch-footer,
  .batch-footer-actions,
  .source-picker-row,
  .source-picker-main {
    display: flex;
    align-items: center;
  }
  .section-title-row,
  .tool-status-header,
  .batch-header,
  .batch-footer,
  .source-picker-row {
    justify-content: space-between;
  }
  .section-title {
    font-size: 13px;
    font-weight: 700;
  }
  .install-empty {
    padding: 24px;
    color: var(--color-text-muted);
    text-align: center;
    font-size: 13px;
  }
  .tool-status-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 12px;
  }
  .tool-status-item {
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 13px 14px;
    background: var(--bg-elevated-subtle);
  }
  .tool-status-left,
  .tool-status-right {
    gap: 8px;
    min-width: 0;
  }
  .tool-status-left { flex: 1; }
  .tool-status-name {
    font-size: 13px;
    font-weight: 650;
  }
  .tool-status-warning-marker {
    position: relative;
    color: #f59e0b;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    line-height: 0;
    border-radius: 999px;
    border: none;
    background: transparent;
    padding: 0;
    cursor: default;
    outline: none;
  }
  .tool-status-warning-marker:focus-visible {
    box-shadow: 0 0 0 2px rgba(245, 158, 11, 0.28);
  }
  .tool-status-path-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .tool-status-path-container {
    margin-top: 8px;
    margin-left: 26px;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .tool-status-path,
  .symlink-target-path,
  .source-picker-path,
  .package-member-path {
    color: var(--color-text-muted);
    font-size: 12px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .tool-status-path { min-width: 0; }
  .symlink-arrow {
    color: var(--color-text-muted);
    font-size: 12px;
  }
  .tool-status-warning {
    margin: 8px 0 0 26px;
    color: var(--color-text-muted);
    font-size: 12px;
  }
  .btn-action-install,
  .btn-action-danger,
  .btn-delete-skill,
  .btn-text,
  .primary-pill-btn {
    min-height: 34px;
    border-radius: 8px;
    padding: 0 12px;
    font-size: 12px;
    font-weight: 650;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    box-sizing: border-box;
  }
  .btn-action-install,
  .btn-text {
    border: none;
    background: var(--bg-subtle);
    color: var(--color-text-main);
  }
  .btn-action-danger,
  .btn-delete-skill {
    border: none;
    background: var(--bg-danger);
    color: #ef4444;
  }
  .primary-pill-btn {
    border: none;
    background: var(--color-text-main);
    color: var(--bg-card);
  }
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .delete-skill-warning {
    color: var(--color-text-muted);
    font-size: 12px;
    margin-bottom: 12px;
  }
  .viewer-body {
    min-height: 0;
    height: 100%;
    display: flex;
  }
  .content-scrollable--padded {
    padding: 20px;
  }
  .content-loading {
    color: var(--color-text-muted);
    font-size: 13px;
  }
  .file-viewer-inline-status {
    color: var(--color-text-muted);
    font-size: 12px;
  }
  .translation-error {
    max-width: 260px;
    color: #dc2626;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .translation-action-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
  }
  :global(.file-viewer-actions .translation-toolbar-btn) {
    width: 30px;
    min-width: 30px;
    padding: 0;
  }
  :global(.file-viewer-actions .translation-toolbar-btn--active) {
    background: var(--toolbar-control-active);
    color: var(--color-text-main);
    box-shadow: inset 0 0 0 1px var(--toolbar-control-border-active);
  }
  .translation-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 20;
    min-width: 118px;
    padding: 5px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-card);
    box-shadow: var(--shadow-popover, var(--shadow-soft));
  }
  .translation-menu button {
    width: 100%;
    min-height: 28px;
    padding: 0 9px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--color-text-main);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }
  .translation-menu button:hover,
  .translation-menu button:focus-visible {
    background: var(--bg-hover);
    outline: none;
  }
  .skill-viewer-search-anchor {
    flex: 0 1 260px;
  }
  .skill-viewer-search-wrap {
    width: min(260px, 32vw);
  }
  .source-selector-row {
    gap: 8px;
    flex-wrap: wrap;
    min-width: 0;
  }
  .variant-switcher-label,
  .variant-usage-hint {
    font-size: 11px;
    color: var(--color-text-muted);
  }
  .package-members-header,
  .package-member-group-header,
  .package-member-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .package-members-title,
  .package-member-name {
    font-weight: 700;
  }
  .package-members-subtitle,
  .package-member-count,
  .package-member-group-header {
    color: var(--color-text-muted);
    font-size: 12px;
  }
  .package-member-groups {
    margin-top: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .package-member-list {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .package-member-item {
    width: 100%;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-elevated-subtle);
    color: var(--color-text-main);
    padding: 10px 12px;
    cursor: pointer;
  }
  .batch-overlay {
    position: fixed;
    inset: 0;
    z-index: 1010;
    background: rgba(0, 0, 0, 0.26);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .source-picker-modal {
    width: min(680px, calc(100vw - 48px));
    max-height: min(720px, calc(100vh - 48px));
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    box-shadow: var(--shadow-floating);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .abnormal-modal {
    width: min(760px, calc(100vw - 48px));
    max-height: min(760px, calc(100vh - 48px));
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    box-shadow: var(--shadow-floating);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .batch-header,
  .batch-footer {
    padding: 14px 18px;
    border-bottom: 1px solid var(--border-color);
  }
  .batch-footer {
    border-top: 1px solid var(--border-color);
    border-bottom: 0;
  }
  .batch-title {
    font-weight: 700;
  }
  .source-picker-body {
    padding: 16px 18px;
    overflow: auto;
  }
  .source-picker-desc {
    color: var(--color-text-muted);
    font-size: 12px;
    margin-bottom: 12px;
  }
  .source-picker-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .abnormal-source-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .abnormal-source-item {
    width: 100%;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-elevated-subtle);
    color: var(--color-text-main);
    padding: 10px 12px;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px;
    align-items: flex-start;
    cursor: pointer;
    box-sizing: border-box;
  }
  .abnormal-source-item.active {
    border-color: var(--border-active);
    background: var(--bg-hover);
  }
  .abnormal-source-main {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .abnormal-source-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .abnormal-source-title {
    font-weight: 650;
  }
  .abnormal-source-path {
    color: var(--color-text-muted);
    font-size: 12px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .abnormal-rename-panel {
    margin-top: 12px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-elevated-subtle);
    padding: 12px;
  }
  .abnormal-rename-label {
    display: block;
    margin-bottom: 8px;
    color: var(--color-text-muted);
    font-size: 12px;
    font-weight: 650;
  }
  .abnormal-rename-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .abnormal-rename-input {
    min-width: 0;
    flex: 1;
    height: 34px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-card);
    color: var(--color-text-main);
    padding: 0 10px;
    font: inherit;
    font-size: 12px;
    box-sizing: border-box;
  }
  .abnormal-rename-input:focus {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .source-picker-item {
    width: 100%;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-elevated-subtle);
    color: var(--color-text-main);
    padding: 10px 12px;
    text-align: left;
    cursor: pointer;
  }
  .source-picker-item.active {
    border-color: var(--border-active);
    background: var(--bg-hover);
  }
  .source-picker-main,
  .batch-footer-actions {
    gap: 8px;
  }
  .source-picker-name {
    font-weight: 650;
  }
  .source-picker-time {
    color: var(--color-text-muted);
    font-size: 11px;
  }
</style>
