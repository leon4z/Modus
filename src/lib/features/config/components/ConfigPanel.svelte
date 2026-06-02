<script>
  import { ArrowLeft, ChevronDown, ChevronUp, Pencil, Search, X } from "lucide-svelte";
  import ContentEditor from "$lib/shared/components/ContentEditor.svelte";
  import FileViewerShell from "$lib/shared/components/FileViewerShell.svelte";
  import ModulePerformanceBar from "$lib/shared/diagnostics/ModulePerformanceBar.svelte";
  import PrimaryState from "$lib/shared/components/PrimaryState.svelte";
  import ProtectedToolSelector from "$lib/shared/components/ProtectedToolSelector.svelte";
  import { activeToolId, managedTools } from "$lib/features/tools/index.js";
  import { listConfigFiles, readConfigFile, saveConfigFile } from "$lib/features/config/api/config.js";
  import { t } from "$lib/shared/i18n/index.js";
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
  import { countCurrentContentMatches, stepCurrentContentMatchIndex } from "$lib/shared/utils/currentContentSearch.js";

  let {
    searchOpen = $bindable(false),
    globalSearchQuery = $bindable(""),
  } = $props();

  let currentToolId = $derived($activeToolId);
  let configTools = $derived($managedTools);
  let loadingList = $state(false);
  let loadingContent = $state(false);
  let saving = $state(false);
  let error = $state("");
  let success = $state("");
  /** @type {any[]} */
  let files = $state([]);
  /** @type {any | null} */
  let selectedFile = $state(null);
  let fileContent = $state("");
  let draftContent = $state("");
  let editing = $state(false);
  let loadedToolId = $state("");
  let autoOpenedFileKey = $state("");
  let configSearchInput = $state(/** @type {HTMLInputElement | undefined} */ (undefined));
  let currentContentSearchMatchIndex = $state(0);
  let currentContentSearchVersion = $state(0);
  let currentContentSearchKey = $state("");
  let isDirty = $derived(String(draftContent ?? "") !== String(fileContent ?? ""));
  let lineCount = $derived(String(draftContent ?? "").split("\n").length);
  let showBackToConfigList = $derived(files.length > 1);
  let selectedFileId = $derived(String(selectedFile?.id || ""));
  let configListFilterQuery = $derived(searchOpen && !selectedFile ? globalSearchQuery.trim().toLowerCase() : "");
  let filteredFiles = $derived.by(() => {
    if (!configListFilterQuery) return files;
    return files.filter((/** @type {any} */ file) => configFileSearchText(file).includes(configListFilterQuery));
  });
  let activeConfigExternalSearchQuery = $derived(
    searchOpen && selectedFileId && Boolean(globalSearchQuery.trim()) ? globalSearchQuery.trim() : ""
  );
  let currentContentSearchMatchCount = $derived(
    countCurrentContentMatches(draftContent, activeConfigExternalSearchQuery)
  );
  let showCurrentContentSearchControls = $derived(Boolean(activeConfigExternalSearchQuery && selectedFileId));

  $effect(() => {
    if (configTools.length === 0) return;
    const hasCurrent = currentToolId && configTools.some((/** @type {any} */ tool) => tool.id === currentToolId);
    if (!hasCurrent) {
      $activeToolId = configTools[0].id;
      return;
    }
    if (currentToolId && loadedToolId !== currentToolId) {
      const reason = loadedToolId ? "tool-switch" : "entry";
      loadedToolId = currentToolId;
      selectedFile = null;
      fileContent = "";
      draftContent = "";
      editing = false;
      autoOpenedFileKey = "";
      loadFiles(currentToolId, { reason }).catch(() => {});
    }
  });

  $effect(() => {
    const key = `${selectedFileId}\u0000${activeConfigExternalSearchQuery}\u0000${String(draftContent ?? "")}`;
    if (!activeConfigExternalSearchQuery) {
      currentContentSearchKey = "";
      currentContentSearchMatchIndex = 0;
      return;
    }
    if (key === currentContentSearchKey) return;
    currentContentSearchKey = key;
    currentContentSearchMatchIndex = 0;
    if (currentContentSearchMatchCount > 0) currentContentSearchVersion += 1;
  });

  $effect(() => {
    if (loadingList || selectedFile || error || files.length !== 1 || !currentToolId) return;
    const file = files[0];
    const fileKey = `${currentToolId}:${file?.id || ""}`;
    if (!file?.id || autoOpenedFileKey === fileKey) return;
    autoOpenedFileKey = fileKey;
    openFile(file, "auto-open-file").catch(() => {});
  });

  /** @param {string} reason */
  function startConfigRun(reason) {
    const run = beginModulePerformanceRun({
      module: "config",
      view: currentToolId || "",
      reason,
      counters: { files: files.length },
    });
    markModulePerformance(run, "config-visible", { role: MODULE_PERFORMANCE_ROLE_VISIBLE });
    updateModulePerformanceSummary(run);
    return run;
  }

  /** @param {string} toolId @param {{ reason?: string }} [options] */
  async function loadFiles(toolId, options = {}) {
    const run = startConfigRun(options.reason || "entry");
    loadingList = true;
    error = "";
    try {
      files = await trackModulePerformanceRequest(run, "config-file-list", () => listConfigFiles(toolId));
      setModulePerformanceCounters(run, { files: files.length });
      markModulePerformance(run, "config-list-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
      markModulePerformance(run, "config-list-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "success");
    } catch (/** @type {any} */ e) {
      files = [];
      error = String(e);
      markModulePerformance(run, "config-list-failed", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "failed");
    } finally {
      loadingList = false;
    }
  }

  /** @param {any} file @param {string} [reason] */
  async function openFile(file, reason = "open-file") {
    const openedFromList = !selectedFile;
    const run = startConfigRun(reason);
    selectedFile = file;
    fileContent = "";
    draftContent = "";
    editing = false;
    error = "";
    success = "";
    if (openedFromList && searchOpen) {
      searchOpen = false;
      globalSearchQuery = "";
    }
    if (!file.exists) {
      markModulePerformance(run, "config-file-missing", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "partial");
      return;
    }

    loadingContent = true;
    try {
      const result = await trackModulePerformanceRequest(run, "config-file-read", () => readConfigFile(/** @type {string} */ (currentToolId), file.id));
      fileContent = result.content || "";
      draftContent = fileContent;
      selectedFile = { ...file, ...result };
      markModulePerformance(run, "config-file-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
      markModulePerformance(run, "config-file-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "success");
    } catch (/** @type {any} */ e) {
      error = String(e);
      selectedFile = { ...file, editable: false, readError: true };
      markModulePerformance(run, "config-file-failed", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "failed");
    } finally {
      loadingContent = false;
    }
  }

  function closeFile() {
    selectedFile = null;
    fileContent = "";
    draftContent = "";
    editing = false;
    error = "";
    success = "";
  }

  function startEdit() {
    if (!selectedFile?.editable || !selectedFile?.exists) return;
    draftContent = fileContent;
    editing = true;
    error = "";
    success = "";
  }

  function cancelEdit() {
    editing = false;
    draftContent = fileContent;
    error = "";
  }

  /** @param {string} nextContent */
  async function saveFile(nextContent) {
    if (!selectedFile?.editable || !selectedFile?.exists || !currentToolId) return;
    const run = startConfigRun("save-file");
    saving = true;
    error = "";
    success = "";
    try {
      await trackModulePerformanceRequest(run, "config-file-save", () => saveConfigFile(currentToolId, selectedFile.id, nextContent));
      fileContent = nextContent;
      draftContent = nextContent;
      editing = false;
      success = $t("config.panel.save_success");
      const updatedFiles = await trackModulePerformanceRequest(run, "config-file-list", () => listConfigFiles(currentToolId));
      files = updatedFiles;
      selectedFile = updatedFiles.find((/** @type {any} */ file) => file.id === selectedFile.id) || selectedFile;
      setModulePerformanceCounters(run, { files: files.length });
      markModulePerformance(run, "config-save-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
      markModulePerformance(run, "config-save-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "success");
    } catch (/** @type {any} */ e) {
      error = String(e);
      markModulePerformance(run, "config-save-failed", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "failed");
    } finally {
      saving = false;
    }
  }

  /** @param {any} file */
  function displayFileName(file) {
    const normalizedPath = String(file?.path || "").replace(/\\/g, "/");
    const name = normalizedPath.split("/").filter(Boolean).pop();
    return name || file?.label || file?.id || "";
  }

  /** @param {any} file */
  function fileStatusLabel(file) {
    if (!file.exists) return $t("config.panel.status_missing");
    if (file.editable) return $t("config.panel.status_editable");
    if (file.access === "read_only" || file.access === "readable") return $t("config.panel.status_read_only");
    if (file.access === "unknown") return $t("config.panel.status_unknown");
    if (file.access === "unsupported") return $t("config.panel.status_unsupported");
    return $t("config.panel.status_unavailable");
  }

  /** @param {any} file */
  function fileMeta(file) {
    const parts = [];
    if (file.size_bytes != null) parts.push($t("config.panel.bytes", { count: file.size_bytes }));
    if (file.modified_unix) {
      parts.push(new Date(file.modified_unix * 1000).toLocaleString());
    }
    return parts.join(" · ");
  }

  /** @param {string} toolId */
  function selectTool(toolId) {
    $activeToolId = toolId;
  }

  export function focusModuleSearch() {
    searchOpen = true;
    setTimeout(() => configSearchInput?.focus(), 0);
  }

  /** @param {"next" | "previous"} direction */
  function moveCurrentContentSearch(direction) {
    if (currentContentSearchMatchCount <= 0) return;
    currentContentSearchMatchIndex = stepCurrentContentMatchIndex(
      currentContentSearchMatchIndex,
      currentContentSearchMatchCount,
      direction,
    );
    currentContentSearchVersion += 1;
  }

  /** @param {any} file */
  function configFileSearchText(file) {
    return [
      displayFileName(file),
      file.label,
      file.path,
      file.format,
      file.access,
      fileMeta(file),
      fileStatusLabel(file),
    ].filter(Boolean).join(" ").toLowerCase();
  }

</script>

<svelte:window onkeydown={(e) => {
  if (e.key !== "Escape") return;
  if (searchOpen) { searchOpen = false; globalSearchQuery = ""; }
}} />

<div class="view-panel config-panel">
  <div class="view-fixed-header">
    <div
      class="view-header management-header-row"
      class:management-header-row--search-open={searchOpen}
      data-tauri-drag-region
    >
      <h2 data-tauri-drag-region>{$t("sidebar.config")}</h2>
      <div class="header-actions management-header-actions">
        {#if searchOpen}
          <div class="workspace-search-anchor">
            <div class="search-input-wrap">
              <span class="search-input-wrap__icon" aria-hidden="true"><Search size={13} strokeWidth={1.8} /></span>
              <input
                bind:this={configSearchInput}
                class="search-input"
                type="text"
                placeholder={selectedFileId ? $t("file_workspace_search.placeholder_current_content") : $t("module_search.placeholder_config")}
                bind:value={globalSearchQuery}
              />
              {#if showCurrentContentSearchControls}
                <span class="search-match-count">
                  {currentContentSearchMatchCount > 0 ? `${currentContentSearchMatchIndex + 1}/${currentContentSearchMatchCount}` : "0/0"}
                </span>
                <button
                  class="icon-btn-tiny search-nav-btn"
                  onclick={() => moveCurrentContentSearch("previous")}
                  disabled={currentContentSearchMatchCount <= 0}
                  aria-label={$t("editor.search.previous")}
                >
                  <ChevronUp size={13} strokeWidth={1.8} />
                </button>
                <button
                  class="icon-btn-tiny search-nav-btn"
                  onclick={() => moveCurrentContentSearch("next")}
                  disabled={currentContentSearchMatchCount <= 0}
                  aria-label={$t("editor.search.next")}
                >
                  <ChevronDown size={13} strokeWidth={1.8} />
                </button>
              {/if}
              <button class="icon-btn-tiny" onclick={() => { searchOpen = false; globalSearchQuery = ""; }} aria-label={$t("global.search.close")}>
                <X size={13} />
              </button>
            </div>
          </div>
        {:else}
          <button
            type="button"
            class="refresh-btn"
            onclick={focusModuleSearch}
            aria-label={$t("global.search.title")}
          >
            <Search size={14} strokeWidth={1.8} />
          </button>
        {/if}
      </div>
    </div>
  </div>

  {#if configTools.length > 0}
    <div class="view-pinned-toolbar">
      <ProtectedToolSelector
        tools={configTools}
        activeToolId={currentToolId}
        ariaLabel={$t("sidebar.config")}
        onSelect={selectTool}
      />
    </div>
  {/if}

  <div class="view-scroll-content config-scroll">
    <ModulePerformanceBar moduleId="config" />

    {#if selectedFile && error}<div class="error">{error}</div>{/if}
    {#if success}<div class="success">{success}</div>{/if}

    {#if configTools.length === 0}
      <PrimaryState message={$t("config.panel.no_managed_tools")} />
    {:else if selectedFile}
      <div class="file-detail">
        {#if !selectedFile.exists}
          <PrimaryState message={$t("config.panel.file_missing")} />
        {:else if selectedFile.readError}
          <PrimaryState message={$t("config.panel.file_unreadable")} />
        {:else if loadingContent}
          <PrimaryState message={$t("config.panel.loading")} />
        {:else}
          <FileViewerShell
            title={displayFileName(selectedFile)}
            subtitle={selectedFile.path}
            modeLabel={editing ? $t("config.panel.editing") : ""}
          >
            {#snippet leading()}
              {#if showBackToConfigList}
                <button type="button" class="file-viewer-icon-btn" aria-label={$t("config.panel.back_to_list")} onclick={closeFile} disabled={saving}>
                  <ArrowLeft size={16} strokeWidth={1.8} />
                </button>
              {/if}
            {/snippet}

            {#snippet actions()}
              {#if editing && selectedFile.editable}
                <span class="file-viewer-inline-status">
                  {#if isDirty}{$t("config.panel.unsaved")} · {/if}{$t("config.panel.lines", { count: lineCount })}
                </span>
                <button type="button" class="file-viewer-btn" onclick={cancelEdit} disabled={saving}>
                  {$t("config.panel.cancel")}
                </button>
                <button type="button" class="file-viewer-primary-btn" onclick={() => saveFile(draftContent)} disabled={saving || !isDirty}>
                  {saving ? $t("config.panel.saving") : $t("config.panel.save")}
                </button>
              {:else if selectedFile.editable}
                <button type="button" class="file-viewer-btn" onclick={startEdit}>
                  <Pencil size={14} strokeWidth={1.8} /> {$t("config.panel.edit")}
                </button>
              {/if}
            {/snippet}

            <ContentEditor
              bind:value={draftContent}
              originalValue={fileContent}
              editing={editing && selectedFile.editable}
              filename={selectedFile.path}
              fill={true}
              minHeight="0"
              showModeToggle={false}
              showActions={false}
              showFooter={false}
              framed={false}
              editLabel={$t("config.panel.edit")}
              cancelLabel={$t("config.panel.cancel")}
              saveLabel={$t("config.panel.save")}
              savingLabel={$t("config.panel.saving")}
              unsavedLabel={$t("config.panel.unsaved")}
              saving={saving}
              formatLineLabel={(/** @type {number} */ count) => $t("config.panel.lines", { count })}
              onEdit={startEdit}
              onCancel={cancelEdit}
              onSave={saveFile}
              externalSearchQuery={activeConfigExternalSearchQuery}
              externalSearchMatchIndex={currentContentSearchMatchIndex}
              externalSearchVersion={currentContentSearchVersion}
            />
          </FileViewerShell>
        {/if}
      </div>
    {:else if error}
      <PrimaryState message={error} tone="error" />
    {:else if loadingList}
      <PrimaryState message={$t("config.panel.loading")} />
    {:else if files.length === 0}
      <PrimaryState message={$t("config.panel.empty")} />
    {:else if filteredFiles.length === 0}
      <PrimaryState message={$t("module_search.empty_scope", { scope: $t("module_search.scope_config") })} />
    {:else}
      <div class="file-list">
        {#each filteredFiles as file}
          <button type="button" class="file-card" onclick={() => openFile(file)}>
            <div class="file-card-body">
              <div class="file-card-top">
                <span class="file-card-title">{displayFileName(file)}</span>
              </div>
              <div class="file-card-path">{file.path}</div>
              <div class="file-card-meta">
                <span>{fileStatusLabel(file)}</span>
                {#if fileMeta(file)}
                  <span>{fileMeta(file)}</span>
                {/if}
              </div>
            </div>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .config-panel {
    height: 100%;
  }
  .config-scroll {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  .error,
  .success {
    flex-shrink: 0;
    padding: 8px 12px;
    font-size: 12px;
    border-radius: 8px;
    margin-bottom: 12px;
  }
  .error {
    color: #f87171;
    background: rgba(248,113,113,0.1);
  }
  .success {
    color: #22c55e;
    background: rgba(34,197,94,0.1);
  }
  .file-list {
    display: grid;
    grid-auto-rows: max-content;
    align-content: start;
    gap: 8px;
    flex: 1 1 0;
    overflow-y: auto;
    min-height: 0;
  }
  .file-card {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    width: 100%;
    padding: 12px 16px;
    text-align: left;
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    color: var(--color-text-main);
    cursor: pointer;
  }
    .file-card:hover {
      border-color: var(--border-active);
      background: var(--bg-hover);
    }
  .file-card-body {
    min-width: 0;
    flex: 1;
  }
  .file-card-top {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .file-card-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-text-main);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file-card-path {
    margin-top: 4px;
    font-size: 11px;
    color: var(--color-text-muted);
    overflow-wrap: anywhere;
  }
  .file-card-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 8px;
    font-size: 11px;
    color: var(--color-text-muted);
  }
  .file-detail {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1 1 0;
  }
  .file-viewer-inline-status {
    font-size: 11px;
    color: var(--color-text-muted);
    white-space: nowrap;
  }
</style>
