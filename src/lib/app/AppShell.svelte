<script>
  import ToolIcon from "$lib/shared/components/ToolIcon.svelte";
  import { Telescope, Check, PanelLeft } from "lucide-svelte";
  import { onMount } from "svelte";
  import Sidebar from "$lib/app/components/Sidebar.svelte";
  import RulesModule from "$lib/features/rules/components/RulesModule.svelte";
  import AppContextMenu from "$lib/shared/components/AppContextMenu.svelte";
  import ConfirmDialog from "$lib/shared/components/ConfirmDialog.svelte";

  import Dashboard from "$lib/features/dashboard/components/Dashboard.svelte";
  import SkillsModule from "$lib/features/skills/components/SkillsModule.svelte";
  import ConfigPanel from "$lib/features/config/components/ConfigPanel.svelte";
  import McpPanel from "$lib/features/mcp/components/McpPanel.svelte";
  import SettingsModule from "$lib/features/settings/components/SettingsModule.svelte";
  import DesignPreview from "$lib/dev/components/DesignPreview.svelte";
  import { normalizeCommunityView } from "$lib/app/edition.js";
  import { loadTools, activeView, activeToolId, detectedTools, managedTools, managedToolIds, appInitialized, pendingSubTab, theme } from "$lib/features/tools/index.js";
  import {
    DEFAULT_SKILL_INVENTORY_TTL_MS,
    prewarmSkillInventory,
  } from "$lib/features/skills/queries/skillInventoryQuery.js";
  import { loadAppUpdateState, runAppUpdateCheck } from "$lib/features/appUpdates/index.js";
  import { applyLanguagePreference, t } from "$lib/shared/i18n/index.js";
  import { isInitialized, getLanguage, getTheme, getModulePerformanceDiagnosticsEnabled } from "$lib/features/settings/index.js";
  import { setModulePerformanceDiagnosticsEnabledState } from "$lib/shared/diagnostics/modulePerformance.js";
  import { getHandledNewToolIds, getManagedTools, setHandledNewToolIds, setManagedTools } from "$lib/features/tools/index.js";
  import {
    getVisualVerificationToolId,
    getVisualVerificationView,
    isVisualVerificationMode,
  } from "$lib/dev/visualVerification/fixtures.js";

  /** @type {any} */
  let rulesModuleRef = $state(null);
  /** @type {any} */
  let skillsModuleRef = $state(null);
  /** @type {any} */
  let mcpModuleRef = $state(null);
  /** @type {any} */
  let configModuleRef = $state(null);
  let confirmDialog = $state(/** @type {any} */ (null));
  let confirmOpen = $state(false);

  // First-run onboarding
  let showOnboarding = $state(false);
  /** @type {Record<string,boolean>} */
  let onboardingSelections = $state({});
  /** @type {any[]} */
  let newToolsDetected = $state([]);
  let showNewToolsPrompt = $state(false);
  /** @type {string[]} */
  let handledNewToolIds = $state([]);

  // Shared search state for SkillsModule
  let globalSearchQuery = $state("");
  let searchOpen = $state(false);
  let sidebarCollapsed = $state(false);
  let sidebarWidth = $state(200);
  const SIDEBAR_COLLAPSED_KEY = "modus.sidebar.collapsed";
  const FOCUS_DISCOVERY_COOLDOWN_MS = 30000;
  const SKILL_INVENTORY_PREWARM_DELAY_MS = 250;
  const APP_UPDATE_STARTUP_DELAY_MS = 800;
  const SKILL_INVENTORY_KEEPWARM_INTERVAL_MS = Math.max(
    15000,
    Math.floor(DEFAULT_SKILL_INVENTORY_TTL_MS * 0.75)
  );
  let lastFocusDiscoveryAt = Date.now();
  /** @type {ReturnType<typeof setTimeout> | null} */
  let skillInventoryPrewarmTimer = null;
  /** @type {ReturnType<typeof setTimeout> | null} */
  let appUpdateStartupTimer = null;

  function cancelSkillInventoryPrewarm() {
    if (skillInventoryPrewarmTimer === null) return;
    clearTimeout(skillInventoryPrewarmTimer);
    skillInventoryPrewarmTimer = null;
  }

  function scheduleSkillInventoryPrewarm(delayMs = SKILL_INVENTORY_PREWARM_DELAY_MS) {
    cancelSkillInventoryPrewarm();
    skillInventoryPrewarmTimer = setTimeout(() => {
      skillInventoryPrewarmTimer = null;
      void prewarmSkillInventory({ ttl: SKILL_INVENTORY_KEEPWARM_INTERVAL_MS }).finally(() => {
        if ($appInitialized && !showOnboarding) {
          scheduleSkillInventoryPrewarm(SKILL_INVENTORY_KEEPWARM_INTERVAL_MS);
        }
      });
    }, delayMs);
  }

  function cancelAppUpdateStartupCheck() {
    if (appUpdateStartupTimer === null) return;
    clearTimeout(appUpdateStartupTimer);
    appUpdateStartupTimer = null;
  }

  onMount(() => {
    let cancelled = false;

    async function startup() {
      try {
        sidebarCollapsed = localStorage.getItem(SIDEBAR_COLLAPSED_KEY) === "true";
      } catch {
        /* ignore */
      }

      if (isVisualVerificationMode()) {
        activeView.set(normalizeCommunityView(getVisualVerificationView()));
        activeToolId.set(getVisualVerificationToolId());
      }

      const savedLang = await getLanguage();
      if (cancelled) return;
      applyLanguagePreference(savedLang);
      const savedTheme = await getTheme();
      if (cancelled) return;
      theme.set(savedTheme);
      const diagnosticsEnabled = await getModulePerformanceDiagnosticsEnabled().catch(() => false);
      if (cancelled) return;
      setModulePerformanceDiagnosticsEnabledState(diagnosticsEnabled === true);

      const loadedTools = await loadTools();
      if (cancelled) return;
      if (isVisualVerificationMode()) {
        activeView.set(normalizeCommunityView(getVisualVerificationView()));
        activeToolId.set(getVisualVerificationToolId());
      }
      const initialized = await isInitialized();
      const [managed, handled] = await Promise.all([
        getManagedTools(),
        getHandledNewToolIds(),
      ]);
      if (cancelled) return;
      managedToolIds.set(managed);
      handledNewToolIds = normalizeToolIds(handled);
      appInitialized.set(initialized);
      if (initialized) scheduleSkillInventoryPrewarm();
      void loadAppUpdateState()
        .then((state) => {
          if (cancelled || !state.canCheck) return;
          cancelAppUpdateStartupCheck();
          appUpdateStartupTimer = setTimeout(() => {
            appUpdateStartupTimer = null;
            if (!cancelled) void runAppUpdateCheck("startup");
          }, APP_UPDATE_STARTUP_DELAY_MS);
        })
        .catch(() => {});

      if (!initialized) {
        const detected = $detectedTools;
        const selections = /** @type {Record<string,boolean>} */ ({});
        detected.forEach((/** @type {any} */ t) => { selections[t.id] = true; });
        onboardingSelections = selections;
        showOnboarding = true;
      } else {
        showNewToolPromptForCandidates(loadedTools || $detectedTools);
      }
    }

    async function handleWindowFocus() {
      if (!$appInitialized || showOnboarding || showNewToolsPrompt) return;
      const now = Date.now();
      if (now - lastFocusDiscoveryAt < FOCUS_DISCOVERY_COOLDOWN_MS) return;
      lastFocusDiscoveryAt = now;
      try {
        const loadedTools = await loadTools();
        if (!cancelled) {
          showNewToolPromptForCandidates(loadedTools || $detectedTools);
          if ($appInitialized && !showOnboarding && !showNewToolsPrompt) {
            scheduleSkillInventoryPrewarm();
          }
        }
      } catch {
        /* keep focus refresh best-effort */
      }
    }

    startup();
    window.addEventListener("focus", handleWindowFocus);
    return () => {
      cancelled = true;
      cancelSkillInventoryPrewarm();
      cancelAppUpdateStartupCheck();
      window.removeEventListener("focus", handleWindowFocus);
    };
  });

  let globalRefreshing = $state(false);
  let globalRefreshMsg = $state("");
  async function handleRefresh() {
    globalRefreshing = true;
    try {
      const loadedTools = await loadTools();
      if (currentView === "skills" && skillsModuleRef) {
        await skillsModuleRef.reload();
      }
      if ($appInitialized) {
        showNewToolPromptForCandidates(loadedTools || $detectedTools);
      }
    } finally {
      setTimeout(() => globalRefreshing = false, 400);
    }
  }

  /** @param {string[]} ids */
  function normalizeToolIds(ids) {
    return [...new Set((ids || []).map(String).filter(Boolean))].sort();
  }

  /** @param {string[]} toolIds */
  async function persistHandledNewToolIds(toolIds) {
    const next = normalizeToolIds([...handledNewToolIds, ...toolIds]);
    await setHandledNewToolIds(next);
    handledNewToolIds = next;
    return next;
  }

  /** @param {any[] | undefined | null} toolList */
  function newToolCandidates(toolList) {
    const managed = new Set($managedToolIds.map(String));
    const handled = new Set(handledNewToolIds.map(String));
    return (toolList || [])
      .filter((/** @type {any} */ tool) => tool?.detected)
      .filter((/** @type {any} */ tool) => !managed.has(String(tool.id)))
      .filter((/** @type {any} */ tool) => !handled.has(String(tool.id)));
  }

  /** @param {any[] | undefined | null} toolList */
  function showNewToolPromptForCandidates(toolList) {
    if (!$appInitialized || showOnboarding || showNewToolsPrompt) return;
    const candidates = newToolCandidates(toolList);
    if (!candidates.length) return;
    const selections = /** @type {Record<string,boolean>} */ ({});
    candidates.forEach((/** @type {any} */ tool) => { selections[tool.id] = true; });
    onboardingSelections = selections;
    newToolsDetected = candidates;
    showNewToolsPrompt = true;
  }

  async function completeOnboarding(/** @type {string[]} */ selected) {
    const handled = normalizeToolIds($detectedTools.map((/** @type {any} */ tool) => tool.id));
    await setManagedTools(selected);
    await setHandledNewToolIds(handled);
    handledNewToolIds = handled;
    managedToolIds.set(selected);
    appInitialized.set(true);
    scheduleSkillInventoryPrewarm();
    showOnboarding = false;
    showNewToolsPrompt = false;
    await loadTools();
  }

  async function confirmOnboarding() {
    const selected = Object.entries(onboardingSelections).filter(([_, v]) => v).map(([k]) => k);
    await completeOnboarding(selected);
  }

  async function skipOnboarding() {
    await completeOnboarding([]);
  }

  function selectAllOnboardingTools() {
    onboardingSelections = Object.fromEntries(
      $detectedTools.map((/** @type {any} */ tool) => [tool.id, true])
    );
  }

  function clearOnboardingTools() {
    onboardingSelections = Object.fromEntries(
      $detectedTools.map((/** @type {any} */ tool) => [tool.id, false])
    );
  }

  async function confirmNewTools() {
    const current = $managedToolIds;
    const newSelected = Object.entries(onboardingSelections).filter(([_, v]) => v).map(([k]) => k);
    const promptToolIds = newToolsDetected.map((/** @type {any} */ tool) => tool.id);
    const merged = [...new Set([...current, ...newSelected])];
    await setManagedTools(merged);
    await persistHandledNewToolIds(promptToolIds);
    managedToolIds.set(merged);
    scheduleSkillInventoryPrewarm();
    showNewToolsPrompt = false;
    newToolsDetected = [];
    await loadTools();
  }
  async function dismissNewTools() {
    const promptToolIds = newToolsDetected.map((/** @type {any} */ tool) => tool.id);
    await persistHandledNewToolIds(promptToolIds);
    showNewToolsPrompt = false;
    newToolsDetected = [];
  }

  function onboardingConfigPath(/** @type {any} */ tool) {
    return tool.primary_config_dir || tool.config_dir || "";
  }

  let onboardingSelectedTools = $derived(
    $detectedTools.filter((/** @type {any} */ tool) => onboardingSelections[tool.id])
  );

  function toggleSidebarCollapsed() {
    sidebarCollapsed = !sidebarCollapsed;
    try {
      localStorage.setItem(SIDEBAR_COLLAPSED_KEY, sidebarCollapsed ? "true" : "false");
    } catch {
      /* ignore */
    }
  }

  let currentView = $derived($activeView);
  let lastSearchView = $state("");

  /** @param {KeyboardEvent} event */
  function isPlatformFindShortcut(event) {
    const key = String(event.key || "").toLowerCase();
    const platformModifier = event.metaKey || event.ctrlKey;
    return platformModifier && !event.altKey && key === "f";
  }

  function currentModuleSearchRef() {
    switch (currentView) {
      case "rules":
        return rulesModuleRef;
      case "skills":
        return skillsModuleRef;
      case "mcp":
        return mcpModuleRef;
      case "config":
        return configModuleRef;
      default:
        return null;
    }
  }

  /** @param {KeyboardEvent} event */
  function handleAppKeydown(event) {
    if (!isPlatformFindShortcut(event)) return;
    const searchTarget = currentModuleSearchRef();
    if (!searchTarget || typeof searchTarget.focusModuleSearch !== "function") return;
    event.preventDefault();
    event.stopPropagation();
    searchTarget.focusModuleSearch();
  }

  $effect(() => {
    const normalizedView = normalizeCommunityView(currentView);
    if (normalizedView !== currentView) {
      activeView.set(normalizedView);
      return;
    }
    if (currentView === "config" && $pendingSubTab === "default") {
      const first = $managedTools[0];
      if (first) activeToolId.set(first.id);
      pendingSubTab.set(null);
    }
  });

  $effect(() => {
    if (!lastSearchView) {
      lastSearchView = currentView;
      return;
    }
    if (lastSearchView === currentView) return;
    lastSearchView = currentView;
    searchOpen = false;
    globalSearchQuery = "";
  });

</script>

<svelte:window onkeydown={handleAppKeydown} />

<!-- First-run onboarding setup -->
{#if showOnboarding}
  <div class="modal-overlay onboarding-screen-overlay">
    <div
      class="onboarding-screen"
      role="dialog"
      aria-modal="true"
      aria-labelledby="onboarding-title"
      tabindex="0"
    >
      <div class="onboarding-drag-region" data-tauri-drag-region aria-hidden="true"></div>

      <main class="onboarding-screen-main">
        <div class="onboarding-hero-copy">
          <h1 id="onboarding-title">{$t("onboarding.title")}</h1>
          <p>{$t("onboarding.subtitle")}</p>
        </div>

        <section class="onboarding-tool-panel" aria-label={$t("onboarding.tool_list_title")}>
          <div class="onboarding-panel-header">
            <span class="onboarding-selected-count">{$t("onboarding.selected_count", { count: onboardingSelectedTools.length })}</span>
            <div class="onboarding-panel-actions">
              <button type="button" class="onboarding-panel-action" onclick={selectAllOnboardingTools}>
                {$t("onboarding.select_all")}
              </button>
              <button type="button" class="onboarding-panel-action" onclick={clearOnboardingTools}>
                {$t("onboarding.clear_all")}
              </button>
            </div>
          </div>

          {#if $detectedTools.length === 0}
            <div class="onboarding-empty">{$t("onboarding.empty")}</div>
          {:else}
            <div class="onboarding-list onboarding-list-full">
              {#each $detectedTools as tool}
                <button
                  type="button"
                  class="onboarding-item"
                  class:selected={onboardingSelections[tool.id]}
                  aria-pressed={onboardingSelections[tool.id] === true}
                  onclick={() => onboardingSelections[tool.id] = !onboardingSelections[tool.id]}
                >
                  <span class="onboarding-check" aria-hidden="true">
                    {#if onboardingSelections[tool.id]}
                      <Check size={16} />
                    {/if}
                  </span>
                  <ToolIcon toolId={tool.id} size={16} />
                  <span class="onboarding-name">{tool.name}</span>
                  <span class="onboarding-path">{onboardingConfigPath(tool)}</span>
                </button>
              {/each}
            </div>
          {/if}
        </section>
      </main>

      <div class="onboarding-footer">
        <span>{$t("onboarding.skip_desc")}</span>
        <div class="onboarding-actions">
          <button type="button" class="btn btn-secondary" onclick={skipOnboarding}>{$t("onboarding.skip")}</button>
          <button type="button" class="btn btn-primary" onclick={confirmOnboarding}><Check size={14} /> {$t("onboarding.confirm")}</button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- New tools detected prompt -->
{#if showNewToolsPrompt}
  <div class="modal-overlay" data-tauri-drag-region>
    <div
      class="onboarding-modal compact"
      role="dialog"
      aria-modal="true"
      tabindex="0"
      onmousedown={(e) => e.stopPropagation()}
    >
      <h3><Telescope size={14}/> {$t("onboarding.new_tool_title")}</h3>
      <p class="onboarding-desc">{$t("onboarding.new_tool_desc")}</p>
      <div class="onboarding-list">
        {#each newToolsDetected as tool}
          <button
            class="onboarding-item"
            class:selected={onboardingSelections[tool.id]}
            onclick={() => onboardingSelections[tool.id] = !onboardingSelections[tool.id]}
          >
            <ToolIcon toolId={tool.id} size={16} />
            <span class="onboarding-name">{tool.name}</span>
            <span class="onboarding-path">{onboardingConfigPath(tool)}</span>
            {#if onboardingSelections[tool.id]}
              <Check size={14} />
            {/if}
          </button>
        {/each}
      </div>
      <div class="onboarding-actions">
        <button type="button" class="btn btn-primary" onclick={confirmNewTools}><Check size={14} /> {$t("onboarding.new_tool_confirm")}</button>
        <button type="button" class="btn btn-secondary" onclick={dismissNewTools}>{$t("onboarding.new_tool_skip")}</button>
      </div>
    </div>
  </div>
{/if}

<div
  class="app"
  class:sidebar-collapsed={sidebarCollapsed}
  aria-hidden={showOnboarding ? "true" : undefined}
  inert={showOnboarding ? true : undefined}
  style={`--current-sidebar-width: ${sidebarCollapsed ? 0 : sidebarWidth}px;`}
>
  <div class="app-window-chrome" data-tauri-drag-region>
    <button
      type="button"
      class="app-sidebar-toggle refresh-btn"
      aria-label={sidebarCollapsed ? $t("global.a11y.expand_sidebar") : $t("global.a11y.collapse_sidebar")}
      onclick={toggleSidebarCollapsed}
    >
      <PanelLeft size={16} strokeWidth={1.8} />
    </button>
  </div>
  <Sidebar bind:collapsed={sidebarCollapsed} bind:sidebarWidth />
  <main class="main" class:sidebar-collapsed={sidebarCollapsed}>

    <div class="content">
      {#if currentView === "dashboard"}
        <Dashboard />


      <!-- Rules View -->
      {:else if currentView === "rules"}
        <RulesModule
          bind:this={rulesModuleRef}
          bind:searchOpen
          bind:globalSearchQuery
          refreshing={globalRefreshing}
          onRefresh={handleRefresh}
        />

      <!-- Skills View -->
      {:else if currentView === "skills"}
        <SkillsModule
          bind:this={skillsModuleRef}
          bind:globalSearchQuery={globalSearchQuery}
          bind:searchOpen={searchOpen}
          refreshing={globalRefreshing}
          onRefresh={handleRefresh}
        />

      <!-- MCP View -->
      {:else if currentView === "mcp"}
        <McpPanel
          bind:this={mcpModuleRef}
          bind:searchOpen
          bind:globalSearchQuery
        />

      <!-- Config View -->
      {:else if currentView === "config"}
        <ConfigPanel
          bind:this={configModuleRef}
          bind:searchOpen
          bind:globalSearchQuery
        />

      <!-- ==================== Debug / Design Preview ==================== -->
      {:else if currentView === "debug"}
        <DesignPreview />

      <!-- ==================== Settings View ==================== -->
      {:else if currentView === "settings"}
        <SettingsModule />
      {/if}
    </div>
  </main>
</div>

{#if globalRefreshMsg}
  <div class="app-transient-toast" role="status" aria-live="polite">{globalRefreshMsg}</div>
{/if}

<ConfirmDialog bind:this={confirmDialog} />
<AppContextMenu />

<style>
  .app {
    display: flex; height: 100vh; overflow: hidden; background: var(--bg-app);
    --window-toolbar-height: 52px;
    --window-top-row-baseline: calc(var(--window-toolbar-height) / 2);
    --window-sidebar-toggle-left: clamp(86px, calc(var(--current-sidebar-width, 200px) - 44px), 316px);
    --window-sidebar-toggle-size: var(--header-ghost-control-size, 32px);
    --window-chrome-top: calc(var(--window-top-row-baseline) - (var(--window-sidebar-toggle-size) / 2));
    --collapsed-header-safe-left: clamp(136px, calc(92px + 3vw), 168px);
    --collapsed-content-inset-x: clamp(20px, calc(168px - 9vw), 40px);
  }
  .app-window-chrome {
    position: fixed;
    top: var(--window-chrome-top);
    left: 0;
    right: 0;
    z-index: 40;
    height: var(--window-sidebar-toggle-size);
    pointer-events: none;
    -webkit-app-region: drag;
  }
  .app-sidebar-toggle.refresh-btn {
    position: absolute;
    top: 0;
    left: var(--window-sidebar-toggle-left);
    padding: 0;
    pointer-events: auto;
    -webkit-app-region: no-drag;
  }
  .main {
    flex: 1; display: flex; flex-direction: column; overflow: hidden;
    background: var(--bg-main);
    border-radius: 16px 0 0 16px;
    border-left: 1px solid var(--main-surface-separator);
    margin: 0;
    transition: border-radius 0.42s cubic-bezier(0.22, 1, 0.36, 1), border-color 0.42s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .main.sidebar-collapsed {
    border-left-color: transparent;
    border-radius: 0;
  }
  .content { flex: 1; display: flex; flex-direction: column; overflow: hidden; }

  :global(.main .view-fixed-header),
  :global(.main .view-scroll-content),
  :global(.main .view-pinned-toolbar) {
    transition: padding-left 0.42s cubic-bezier(0.22, 1, 0.36, 1), padding-right 0.42s cubic-bezier(0.22, 1, 0.36, 1);
  }

  :global(.main.sidebar-collapsed .view-fixed-header) {
    padding-left: var(--collapsed-header-safe-left);
  }

  :global(.main.sidebar-collapsed .view-scroll-content),
  :global(.main.sidebar-collapsed .view-pinned-toolbar) {
    padding-left: var(--collapsed-content-inset-x);
    padding-right: var(--collapsed-content-inset-x);
  }

  @media (prefers-reduced-motion: reduce) {
    .main,
    :global(.main .view-fixed-header),
    :global(.main .view-scroll-content),
    :global(.main .view-pinned-toolbar) {
      transition: none;
    }
  }

  /* Batch delete / skill delete / refresh-adjacent messages — not next to header icons */
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

  /* Shared layout — defined in app.css (view-panel, view-fixed-header, view-scroll-content, view-header) */

  /* Modal / Onboarding */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: var(--overlay-filter);
    -webkit-backdrop-filter: var(--overlay-filter);
  }

  .onboarding-screen-overlay {
    align-items: stretch;
    justify-content: stretch;
    background: var(--bg-main);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }

  .onboarding-screen {
    position: relative;
    width: 100%;
    height: 100vh;
    min-height: 560px;
    display: grid;
    grid-template-rows: minmax(0, 1fr) 88px;
    background: var(--bg-main);
    color: var(--color-text-main);
  }

  .onboarding-drag-region {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 64px;
    z-index: 2;
    -webkit-app-region: drag;
  }

  .onboarding-screen-main {
    width: min(1240px, calc(100vw - 160px));
    min-height: 0;
    margin: 0 auto;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 28px;
    padding: 72px 0 28px;
  }

  .onboarding-hero-copy h1 {
    margin: 0;
    font-size: clamp(30px, 3.2vw, 48px);
    line-height: 1.08;
    font-weight: 650;
    letter-spacing: 0;
    color: var(--color-text-main);
  }

  .onboarding-hero-copy p {
    max-width: 720px;
    margin: 16px 0 0;
    font-size: 16px;
    line-height: 1.52;
    color: var(--color-text-muted);
  }

  .onboarding-tool-panel {
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    overflow: hidden;
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 14px;
  }

  .onboarding-panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 28px;
  }

  .onboarding-selected-count {
    color: var(--color-text-muted);
    font-size: 12px;
    line-height: 1.5;
  }

  .onboarding-panel-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  .onboarding-panel-action {
    min-height: 30px;
    padding: 0 12px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-subtle);
    color: var(--color-text-muted);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.16s ease, border-color 0.16s ease, color 0.16s ease;
  }

  .onboarding-panel-action:hover {
    background: var(--bg-hover);
    border-color: var(--border-active);
    color: var(--color-text-main);
  }

  .onboarding-panel-action:focus-visible,
  .onboarding-item:focus-visible {
    outline: 2px solid var(--border-active);
    outline-offset: 2px;
  }

  .onboarding-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 20px;
  }

  .onboarding-list-full {
    min-height: 0;
    overflow-y: auto;
    margin: 0;
    padding: 14px 28px 22px;
  }

  .onboarding-item {
    display: grid;
    grid-template-columns: 30px 18px minmax(160px, 1fr) minmax(180px, 34%);
    align-items: center;
    gap: 10px;
    min-height: 56px;
    padding: 0 14px;
    background: var(--bg-main);
    border: 1px solid var(--border-color);
    border-radius: 10px;
    cursor: pointer;
    transition: background 0.16s ease, border-color 0.16s ease, color 0.16s ease;
    font-family: inherit;
    font-size: inherit;
    text-align: left;
    color: var(--color-text-main);
  }

  .onboarding-modal .onboarding-item {
    grid-template-columns: 18px minmax(100px, 1fr) minmax(120px, auto) 16px;
    min-height: 44px;
    padding: 0 14px;
  }

  .onboarding-item:hover {
    background: var(--bg-hover);
    border-color: var(--border-active);
  }

  .onboarding-item.selected {
    border-color: var(--border-active);
    background: var(--bg-active);
  }

  .onboarding-check {
    width: 24px;
    height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-active);
    border-radius: 7px;
    color: var(--bg-card);
    background: var(--bg-card);
  }

  .onboarding-item.selected .onboarding-check {
    background: var(--color-text-main);
    border-color: var(--color-text-main);
  }

  .onboarding-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    font-weight: 600;
    color: var(--color-text-main);
  }

  .onboarding-path {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11px;
    color: var(--color-text-muted);
    opacity: 0.78;
    font-family: "SF Mono", monospace;
  }

  .onboarding-empty {
    margin: 12px 22px 20px;
    padding: 22px;
    border: 1px dashed var(--border-color);
    border-radius: 12px;
    font-size: 12px;
    color: var(--color-text-muted);
    text-align: center;
  }

  .onboarding-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 0 72px;
    border-top: 1px solid var(--border-color);
    background: var(--bg-panel);
  }

  .onboarding-footer span {
    display: block;
    font-size: 12px;
    color: var(--color-text-muted);
  }

  .onboarding-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    flex-shrink: 0;
  }

  .onboarding-modal {
    background: var(--bg-modal);
    border: 1px solid var(--border-modal);
    border-radius: 16px;
    padding: 32px;
    max-width: 520px;
    width: 90%;
    box-shadow: var(--shadow-modal);
  }

  .onboarding-modal.compact {
    max-width: 440px;
    padding: 24px;
  }

  .onboarding-modal h3 {
    font-size: 16px;
    color: var(--color-text-main);
    margin: 0 0 8px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .onboarding-desc {
    font-size: 12px;
    color: var(--color-text-muted);
    margin-bottom: 16px;
  }

  @media (max-width: 980px) {
    .onboarding-screen-main {
      width: calc(100vw - 56px);
      padding-top: 34px;
    }

    .onboarding-footer {
      padding-left: 28px;
      padding-right: 28px;
    }

    .onboarding-item {
      grid-template-columns: 30px 18px minmax(120px, 1fr) minmax(120px, 32%);
    }
  }

  /* 页头 .header-actions 间距见 app.css --header-actions-gap（勿在此用 gap:10 覆盖） */
  /* Refresh btn & spin — defined in app.css */
  :global(.lucide) { display: block; flex-shrink: 0; }
  @keyframes spin { 100% { transform: rotate(360deg); } }
  /* Overlay */
  :global([data-theme="dark"]) .main {
    background: var(--bg-main);
  }

</style>
