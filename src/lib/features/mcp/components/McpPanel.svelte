<script>
  import { ArrowLeft, ChevronDown, ChevronUp, Pencil, RefreshCw, Search, X } from "lucide-svelte";
  import ContentEditor from "$lib/shared/components/ContentEditor.svelte";
  import FileViewerShell from "$lib/shared/components/FileViewerShell.svelte";
  import ModulePerformanceBar from "$lib/shared/diagnostics/ModulePerformanceBar.svelte";
  import ProtectedToolSelector from "$lib/shared/components/ProtectedToolSelector.svelte";
  import { activeToolId, managedTools } from "$lib/features/tools/index.js";
  import {
    listMcpConfigSources,
    readMcpServerConfigFragment,
    saveMcpServerConfigFragment,
  } from "$lib/features/mcp/api/mcp.js";
  import { t } from "$lib/shared/i18n/index.js";
  import PrimaryState from "$lib/shared/components/PrimaryState.svelte";
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
  let currentManaged = $derived($managedTools);
  let mcpTools = $derived(currentManaged);

  let loadingList = $state(false);
  let loadingContent = $state(false);
  let saving = $state(false);
  let isRefreshing = $state(false);
  let error = $state("");
  let success = $state("");
  let backupPath = $state("");
  /** @type {any[]} */
  let sources = $state([]);
  /** @type {any | null} */
  let selectedEntry = $state(null);
  let openedContent = $state("");
  let draftContent = $state("");
  let editing = $state(false);
  let loadedToolId = $state("");
  let mcpSearchInput = $state(/** @type {HTMLInputElement | undefined} */ (undefined));
  let currentContentSearchMatchIndex = $state(0);
  let currentContentSearchVersion = $state(0);
  let currentContentSearchKey = $state("");

  let entries = $derived(buildEntries(sources));
  let emptyState = $derived(emptyStateFromSources(sources));
  let isDirty = $derived(String(draftContent ?? "") !== String(openedContent ?? ""));
  let lineCount = $derived(String(draftContent ?? "").split("\n").length);
  let selectedEntryId = $derived(selectedEntry ? `${selectedEntry.sourceId}:${selectedEntry.server?.name || selectedEntry.label || ""}` : "");
  let mcpListFilterQuery = $derived(searchOpen && !selectedEntry ? globalSearchQuery.trim().toLowerCase() : "");
  let filteredEntries = $derived.by(() => {
    if (!mcpListFilterQuery) return entries;
    return entries.filter((/** @type {any} */ entry) => mcpEntrySearchText(entry).includes(mcpListFilterQuery));
  });
  let activeMcpExternalSearchQuery = $derived(
    searchOpen && selectedEntryId && Boolean(globalSearchQuery.trim()) ? globalSearchQuery.trim() : ""
  );
  let currentContentSearchMatchCount = $derived(
    countCurrentContentMatches(draftContent, activeMcpExternalSearchQuery)
  );
  let showCurrentContentSearchControls = $derived(Boolean(activeMcpExternalSearchQuery && selectedEntryId));

  $effect(() => {
    if (mcpTools.length === 0) {
      sources = [];
      selectedEntry = null;
      return;
    }
    const hasCurrent = currentToolId && mcpTools.some((/** @type {any} */ tool) => tool.id === currentToolId);
    if (!hasCurrent) {
      $activeToolId = mcpTools[0].id;
      return;
    }
    if (currentToolId && loadedToolId !== currentToolId) {
      const reason = loadedToolId ? "tool-switch" : "entry";
      loadedToolId = currentToolId;
      closeEntry();
      loadSources(currentToolId, { reason }).catch(() => {});
    }
  });

  $effect(() => {
    const key = `${selectedEntryId}\u0000${activeMcpExternalSearchQuery}\u0000${String(draftContent ?? "")}`;
    if (!activeMcpExternalSearchQuery) {
      currentContentSearchKey = "";
      currentContentSearchMatchIndex = 0;
      return;
    }
    if (key === currentContentSearchKey) return;
    currentContentSearchKey = key;
    currentContentSearchMatchIndex = 0;
    if (currentContentSearchMatchCount > 0) currentContentSearchVersion += 1;
  });

  /**
   * @param {any[]} sourceList
   */
  function buildEntries(sourceList) {
    return sourceList.flatMap((/** @type {any} */ source) => {
      if (source.state === "loaded" && source.servers?.length) {
        return source.servers.map((/** @type {any} */ server) => ({
          kind: "server",
          id: `${source.id}:${server.name}`,
          sourceId: source.id,
          sourceLabel: source.label,
          source,
          label: server.name,
          path: source.path,
          format: source.format,
          editable: source.editable,
          exists: source.exists,
          state: source.state,
          server,
        }));
      }
      return [];
    });
  }

  /** @param {string} reason */
  function startMcpRun(reason) {
    const run = beginModulePerformanceRun({
      module: "mcp",
      view: currentToolId || "",
      reason,
      counters: { sources: sources.length, entries: entries.length },
    });
    markModulePerformance(run, "mcp-visible", { role: MODULE_PERFORMANCE_ROLE_VISIBLE });
    updateModulePerformanceSummary(run);
    return run;
  }

  /** @param {string} toolId @param {{ reason?: string, run?: any | null }} [options] */
  async function loadSources(toolId, options = {}) {
    const run = options.run || startMcpRun(options.reason || "entry");
    loadingList = true;
    error = "";
    success = "";
    backupPath = "";
    try {
      sources = await trackModulePerformanceRequest(run, "mcp-source-list", () => listMcpConfigSources(toolId));
      setModulePerformanceCounters(run, { sources: sources.length, entries: buildEntries(sources).length });
      markModulePerformance(run, "mcp-list-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
      if (!options.run) {
        markModulePerformance(run, "mcp-list-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
        finishAndRecordModulePerformanceRun(run, "success");
      }
    } catch (/** @type {any} */ e) {
      sources = [];
      error = String(e);
      if (options.run && run) run.status = "partial";
      markModulePerformance(run, "mcp-list-failed", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      if (!options.run) finishAndRecordModulePerformanceRun(run, "failed");
    } finally {
      loadingList = false;
    }
  }

  async function handleRefresh() {
    const run = startMcpRun("manual-refresh");
    isRefreshing = true;
    try {
      if (currentToolId) {
        if (selectedEntry && !isDirty) {
          await rereadEntry(run);
        } else {
          closeEntry();
          await loadSources(currentToolId, { run });
        }
      }
      markModulePerformance(run, "mcp-refresh-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, run?.status === "partial" ? "partial" : "success");
    } finally {
      setTimeout(() => (isRefreshing = false), 400);
    }
  }

  /**
   * @param {any} entry
   * @param {boolean} [editAfterOpen]
   */
  /** @param {any} entry @param {boolean} [editAfterOpen] @param {any} [runOverride] */
  async function openEntry(entry, editAfterOpen = false, runOverride = null) {
    const openedFromList = !selectedEntry;
    const run = runOverride || startMcpRun("open-entry");
    selectedEntry = entry;
    openedContent = "";
    draftContent = "";
    editing = false;
    error = "";
    success = "";
    backupPath = "";
    if (openedFromList && searchOpen) {
      searchOpen = false;
      globalSearchQuery = "";
    }
    if (!canOpenEntry(entry)) {
      markModulePerformance(run, "mcp-entry-unavailable", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      if (!runOverride) finishAndRecordModulePerformanceRun(run, "partial");
      return;
    }

    loadingContent = true;
    try {
      const result = await trackModulePerformanceRequest(run, "mcp-entry-read", () => readMcpServerConfigFragment(/** @type {string} */ (currentToolId), entry.sourceId, entry.server.name));
      openedContent = result.content || "";
      draftContent = openedContent;
      selectedEntry = { ...entry, ...result, kind: entry.kind };
      if (editAfterOpen && selectedEntry.editable) {
        editing = true;
      }
      markModulePerformance(run, "mcp-entry-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
      markModulePerformance(run, "mcp-entry-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      if (!runOverride) finishAndRecordModulePerformanceRun(run, "success");
    } catch (/** @type {any} */ e) {
      error = String(e);
      selectedEntry = { ...entry, editable: false, readError: true };
      if (runOverride && run) run.status = "partial";
      markModulePerformance(run, "mcp-entry-failed", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      if (!runOverride) finishAndRecordModulePerformanceRun(run, "failed");
    } finally {
      loadingContent = false;
    }
  }

  /** @param {any} [runOverride] */
  async function rereadEntry(runOverride = null) {
    if (!selectedEntry || !currentToolId) return;
    await openEntry(selectedEntry, false, runOverride);
    await loadSources(currentToolId, { run: runOverride });
  }

  function closeEntry() {
    selectedEntry = null;
    openedContent = "";
    draftContent = "";
    editing = false;
    error = "";
    success = "";
    backupPath = "";
  }

  function startEdit() {
    if (!selectedEntry?.editable || !selectedEntry?.exists) return;
    draftContent = openedContent;
    editing = true;
    error = "";
    success = "";
    backupPath = "";
  }

  function cancelEdit() {
    editing = false;
    draftContent = openedContent;
    error = "";
  }

  /** @param {string} nextContent */
  async function saveEntry(nextContent) {
    if (!selectedEntry?.editable || !selectedEntry?.exists || !currentToolId) return;
    const run = startMcpRun("save-entry");
    saving = true;
    error = "";
    success = "";
    backupPath = "";
    try {
      const result = await trackModulePerformanceRequest(run, "mcp-entry-save", () => saveMcpServerConfigFragment(currentToolId, selectedEntry.sourceId, selectedEntry.server.name, nextContent));
      openedContent = nextContent;
      draftContent = nextContent;
      editing = false;
      success = $t("mcp.panel.save_success");
      backupPath = result.backup_path || "";
      sources = await trackModulePerformanceRequest(run, "mcp-source-list", () => listMcpConfigSources(currentToolId));
      selectedEntry = { ...selectedEntry, content: nextContent };
      setModulePerformanceCounters(run, { sources: sources.length, entries: buildEntries(sources).length });
      markModulePerformance(run, "mcp-save-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
      markModulePerformance(run, "mcp-save-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "success");
    } catch (/** @type {any} */ e) {
      error = String(e);
      markModulePerformance(run, "mcp-save-failed", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "failed");
    } finally {
      saving = false;
    }
  }

  /** @param {string} format */
  function formatLabel(format) {
    const key = `config.panel.format_${format || "unknown"}`;
    const label = $t(key);
    return label === key ? String(format || "unknown").toUpperCase() : label;
  }

  /** @param {any} source */
  function sourceStatusLabel(source) {
    if ((source.state === "loaded" || source.state === "empty") && source.server_count === 0) return $t("mcp.panel.status_empty");
    if (source.state === "loaded") return $t("mcp.panel.status_loaded");
    if (source.state === "empty") return $t("mcp.panel.status_empty");
    if (source.state === "missing") return $t("mcp.panel.status_missing");
    if (source.state === "unreadable") return $t("mcp.panel.status_unreadable");
    if (source.state === "malformed") return $t("mcp.panel.status_malformed");
    if (source.state === "unknown") return $t("mcp.panel.status_unknown");
    if (source.state === "unsupported") return $t("mcp.panel.status_unsupported");
    return $t("mcp.panel.status_error");
  }

  /** @param {any} entry */
  function entryDescription(entry) {
    if (entry.kind !== "server") {
      return sourceStatusLabel(entry);
    }
    const parts = [entry.server.server_type || "stdio"];
    if (entry.server.command) parts.push(entry.server.command);
    if (entry.server.url) parts.push(entry.server.url);
    if (entry.server.env_keys?.length) {
      parts.push($t("mcp.panel.env_count", { count: entry.server.env_keys.length }));
    }
    return parts.join(" · ");
  }

  /** @param {any} entry */
  function entryMeta(entry) {
    if (entry.kind !== "server") {
      return [sourceStatusLabel(entry)].filter(Boolean).join(" · ");
    }
    return $t("mcp.panel.entry_source", { label: entry.sourceLabel });
  }

  /** @param {any} entry */
  function canOpenEntry(entry) {
    return entry.kind === "server" && entry.exists && entry.state !== "unsupported" && entry.state !== "unknown";
  }

  /**
   * @param {any[]} sourceList
   */
  function selectEmptyStateSource(sourceList) {
    if (!sourceList.length) return null;
    for (const state of ["malformed", "unreadable", "error"]) {
      const source = sourceList.find((/** @type {any} */ candidate) => candidate.state === state);
      if (source) return source;
    }
    return sourceList[0];
  }

  /**
   * @param {any[]} sourceList
   */
  function emptyStateFromSources(sourceList) {
    const source = selectEmptyStateSource(sourceList);
    if (!source) {
      return { message: $t("mcp.panel.unsupported"), detail: "" };
    }
    if (source.required_adapter?.package_name) {
      return {
        message: $t("mcp.panel.adapter_required", {
          tool: source.required_adapter.tool_name,
          package: source.required_adapter.package_name,
        }),
        detail: "",
      };
    }
    if ((source.state === "loaded" || source.state === "empty") && (source.server_count === 0 || !source.servers?.length)) {
      return {
        message: $t("mcp.panel.no_entries_detected"),
        detail: "",
      };
    }
    const normalizedState = source.state === "unknown"
      ? "unsupported"
      : source.state === "error"
        ? "unreadable"
        : source.state;
    const messageKey = ["missing", "unreadable", "malformed", "unsupported"].includes(normalizedState)
      ? `mcp.panel.${normalizedState}`
      : "mcp.panel.unreadable";
    return {
      message: $t(messageKey),
      detail: source.path || "",
    };
  }

  /** @param {string} toolId */
  function selectTool(toolId) {
    $activeToolId = toolId;
  }

  export function focusModuleSearch() {
    searchOpen = true;
    setTimeout(() => mcpSearchInput?.focus(), 0);
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

  /** @param {any} entry */
  function mcpEntrySearchText(entry) {
    return [
      entry.label,
      entryDescription(entry),
      entryMeta(entry),
      entry.path,
      entry.format,
      entry.sourceLabel,
      entry.server?.server_type,
      entry.server?.command,
      entry.server?.url,
      ...(entry.server?.env_keys || []),
    ].filter(Boolean).join(" ").toLowerCase();
  }

</script>

<svelte:window onkeydown={(e) => {
  if (e.key !== "Escape") return;
  if (searchOpen) { searchOpen = false; globalSearchQuery = ""; }
}} />

<div class="view-panel mcp-panel">
  <div class="view-fixed-header">
    <div
      class="view-header management-header-row"
      class:management-header-row--search-open={searchOpen}
      data-tauri-drag-region
    >
      <h2 data-tauri-drag-region>{$t("mcp.panel.title")}</h2>
      <div class="header-actions management-header-actions">
        {#if searchOpen}
          <div class="workspace-search-anchor">
            <div class="search-input-wrap">
              <span class="search-input-wrap__icon" aria-hidden="true"><Search size={13} strokeWidth={1.8} /></span>
              <input
                bind:this={mcpSearchInput}
                class="search-input"
                type="text"
                placeholder={selectedEntryId ? $t("file_workspace_search.placeholder_current_content") : $t("module_search.placeholder_mcp")}
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
        <button
          type="button"
          class="refresh-btn"
          onclick={handleRefresh}
          disabled={isRefreshing || saving || (editing && isDirty)}
          aria-label={$t("mcp.panel.refresh")}
        >
          <RefreshCw size={14} strokeWidth={1.8} class={isRefreshing ? "spin" : ""} />
        </button>
      </div>
    </div>
  </div>

  {#if mcpTools.length > 0}
    <div class="view-pinned-toolbar">
      <ProtectedToolSelector
        tools={mcpTools}
        activeToolId={currentToolId}
        ariaLabel={$t("mcp.panel.title")}
        onSelect={selectTool}
      />
    </div>
  {/if}

  <div class="view-scroll-content mcp-scroll">
    <ModulePerformanceBar moduleId="mcp" />

    {#if selectedEntry && error}<div class="error">{error}</div>{/if}
    {#if success}<div class="success">{success}</div>{/if}
    {#if backupPath}<div class="backup-note">{$t("mcp.panel.backup_saved")}: {backupPath}</div>{/if}

    {#if mcpTools.length === 0}
      <PrimaryState message={$t("mcp.panel.no_managed_tools")} />
    {:else if selectedEntry}
      <div class="entry-detail">
        {#if !selectedEntry.exists}
          <PrimaryState message={$t("mcp.panel.missing")} detail={selectedEntry.message || ""} />
        {:else if selectedEntry.readError}
          <PrimaryState message={$t("mcp.panel.unreadable")} />
        {:else if loadingContent}
          <PrimaryState message={$t("mcp.panel.loading")} />
        {:else}
          <FileViewerShell
            title={selectedEntry.label}
            subtitle={selectedEntry.path}
            badge={formatLabel(selectedEntry.format)}
            modeLabel={editing ? $t("mcp.panel.editing") : ""}
          >
            {#snippet leading()}
              <button type="button" class="mcp-viewer-back-btn" aria-label={$t("mcp.panel.back_to_list")} onclick={closeEntry} disabled={saving}>
                <ArrowLeft size={16} strokeWidth={1.8} />
              </button>
            {/snippet}

            {#snippet actions()}
              {#if editing && selectedEntry.editable}
                <span class="file-viewer-inline-status">
                  {#if isDirty}{$t("mcp.panel.unsaved")} · {/if}{$t("config.panel.lines", { count: lineCount })}
                </span>
                <button type="button" class="file-viewer-btn" onclick={cancelEdit} disabled={saving}>
                  {$t("mcp.panel.cancel")}
                </button>
                <button type="button" class="file-viewer-primary-btn" onclick={() => saveEntry(draftContent)} disabled={saving || !isDirty}>
                  {saving ? $t("mcp.panel.saving") : $t("mcp.panel.save")}
                </button>
              {:else}
                {#if selectedEntry.editable}
                  <button type="button" class="settings-action-btn" onclick={startEdit}>
                    <Pencil size={14} strokeWidth={1.8} /> {$t("mcp.panel.edit")}
                  </button>
                {/if}
              {/if}
            {/snippet}

            <ContentEditor
              bind:value={draftContent}
              originalValue={openedContent}
              editing={editing && selectedEntry.editable}
              filename={selectedEntry.path}
              fill={true}
              minHeight="0"
              showModeToggle={false}
              showActions={false}
              showFooter={false}
              framed={false}
              editLabel={$t("mcp.panel.edit")}
              cancelLabel={$t("mcp.panel.cancel")}
              saveLabel={$t("mcp.panel.save")}
              savingLabel={$t("mcp.panel.saving")}
              unsavedLabel={$t("mcp.panel.unsaved")}
              saving={saving}
              formatLineLabel={(/** @type {number} */ count) => $t("config.panel.lines", { count })}
              onEdit={startEdit}
              onCancel={cancelEdit}
              onSave={saveEntry}
              externalSearchQuery={activeMcpExternalSearchQuery}
              externalSearchMatchIndex={currentContentSearchMatchIndex}
              externalSearchVersion={currentContentSearchVersion}
            />
          </FileViewerShell>
        {/if}
      </div>
    {:else if error}
      <PrimaryState message={error} tone="error" />
    {:else if loadingList}
      <PrimaryState message={$t("mcp.panel.loading")} />
    {:else if entries.length === 0}
      <PrimaryState message={emptyState.message} detail={emptyState.detail} />
    {:else if filteredEntries.length === 0}
      <PrimaryState message={$t("module_search.empty_scope", { scope: $t("module_search.scope_mcp") })} />
    {:else}
      <div class="entry-list">
        {#each filteredEntries as entry}
          <div class="entry-card" class:disabled={!canOpenEntry(entry)}>
            <button
              type="button"
              class="entry-card-main"
              onclick={() => openEntry(entry)}
              disabled={!canOpenEntry(entry)}
            >
              <div class="entry-card-body">
                <div class="entry-card-top">
                  <span class="entry-card-title">{entry.label}</span>
                  <span class="entry-chip">{formatLabel(entry.format)}</span>
                </div>
                <div class="entry-card-desc">{entryDescription(entry)}</div>
                <div class="entry-card-meta">{entryMeta(entry)}</div>
              </div>
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .mcp-panel {
    height: 100%;
  }
  .mcp-scroll {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  .error,
  .success,
  .backup-note {
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
  .backup-note {
    color: var(--color-text-muted);
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    overflow-wrap: anywhere;
  }
  .entry-list {
    display: grid;
    grid-auto-rows: max-content;
    align-content: start;
    gap: 8px;
    flex: 1 1 0;
    overflow-y: auto;
    min-height: 0;
  }
  .entry-card {
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
  }
  .entry-card-main {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    min-width: 0;
    flex: 1;
    padding: 0;
    text-align: left;
    color: inherit;
    background: transparent;
    border: 0;
    cursor: pointer;
  }
  .entry-card:hover:not(.disabled) {
    border-color: var(--border-active);
    background: var(--bg-hover);
  }
  .entry-card.disabled {
    cursor: default;
    opacity: 0.65;
  }
  .entry-card-main:disabled {
    cursor: default;
  }
  .entry-card-body {
    min-width: 0;
    flex: 1;
  }
  .entry-card-top {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .entry-card-title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    font-weight: 600;
    color: var(--color-text-main);
  }
  .entry-chip {
    flex-shrink: 0;
    padding: 4px 8px;
    border-radius: 999px;
    font-size: 10px;
    color: var(--module-mcp-accent);
    background: var(--module-mcp-accent-soft);
  }
  .entry-card-desc {
    margin-top: 4px;
    font-size: 11px;
    color: var(--color-text-muted);
    overflow-wrap: anywhere;
  }
  .entry-card-meta {
    margin-top: 8px;
    font-size: 11px;
    color: var(--color-text-muted);
    overflow-wrap: anywhere;
  }
  .entry-detail {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1 1 0;
  }
  .entry-detail :global(.file-viewer-titlebar) {
    padding-left: calc((var(--file-viewer-header-height) - 30px) / 2);
  }
  .mcp-viewer-back-btn {
    width: 30px;
    height: 30px;
    flex: 0 0 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    box-sizing: border-box;
    transition: background 0.15s ease, color 0.15s ease;
  }
  .mcp-viewer-back-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--color-text-main);
  }
  .mcp-viewer-back-btn:focus-visible {
    outline: 2px solid var(--border-active);
    outline-offset: 2px;
  }
  .mcp-viewer-back-btn:disabled {
    cursor: default;
    opacity: 0.45;
  }
  .file-viewer-inline-status {
    font-size: 11px;
    color: var(--color-text-muted);
    white-space: nowrap;
  }
</style>
