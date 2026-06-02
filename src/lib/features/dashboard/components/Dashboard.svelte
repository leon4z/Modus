<script>
  import ToolIcon from "$lib/shared/components/ToolIcon.svelte";
  import Tooltip from "$lib/shared/components/Tooltip.svelte";
  import ModulePerformanceBar from "$lib/shared/diagnostics/ModulePerformanceBar.svelte";
  import { Blocks, ScrollText, Settings2, PlugZap, RefreshCw } from "lucide-svelte";
  import { onMount } from "svelte";
  import { getDashboard } from "$lib/features/dashboard/api/dashboard.js";
  import {
    activeView,
    activeToolId,
    pendingSubTab,
    managedToolIds,
    loadTools,
    sortToolsByDisplayName,
  } from "$lib/features/tools/index.js";
  import { activeSettingsTab, focusedSettingsToolId } from "$lib/features/settings/index.js";
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

  /** @type {any} */
  let data = $state(null);
  let loading = $state(true);
  let isRefreshing = $state(false);
  let dashboardLoaded = $state(false);
  let lastManagedKey = $state("");
  let managedKey = $derived($managedToolIds.join("|"));
  let visibleTools = $derived(sortToolsByDisplayName((data?.tools || []).filter((/** @type {any} */ t) => t.detected)));

  /** @param {string} reason */
  function startDashboardRun(reason) {
    const run = beginModulePerformanceRun({
      module: "dashboard",
      reason,
      counters: dashboardCounters(data),
    });
    markModulePerformance(run, "dashboard-visible", { role: MODULE_PERFORMANCE_ROLE_VISIBLE });
    updateModulePerformanceSummary(run);
    return run;
  }

  /** @param {any} value */
  function dashboardCounters(value) {
    return {
      tools: value?.detected_count || 0,
      skills: value?.total_skills || 0,
      rules: value?.total_rules || 0,
    };
  }

  /** @param {{ reason?: string }} [options] */
  async function loadDashboard(options = {}) {
    const run = startDashboardRun(options.reason || "entry");
    try {
      const res = await trackModulePerformanceRequest(run, "dashboard", () => getDashboard());
      data = res;
      setModulePerformanceCounters(run, dashboardCounters(res));
      markModulePerformance(run, "dashboard-ready", { role: MODULE_PERFORMANCE_ROLE_INTERACTIVE });
      markModulePerformance(run, "dashboard-complete", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "success");
    } catch (e) {
      console.error("Dashboard error:", e);
      markModulePerformance(run, "dashboard-failed", { role: MODULE_PERFORMANCE_ROLE_BACKGROUND_COMPLETE });
      finishAndRecordModulePerformanceRun(run, "failed");
    } finally {
      loading = false;
      dashboardLoaded = true;
    }
  }

  async function handleRefresh() {
    isRefreshing = true;
    try {
      await loadDashboard({ reason: "manual-refresh" });
    } finally {
      setTimeout(() => isRefreshing = false, 400);
    }
  }

  onMount(async () => {
    await loadDashboard({ reason: "entry" });
  });

  $effect(() => {
    if (!dashboardLoaded) return;
    if (managedKey === lastManagedKey) return;
    lastManagedKey = managedKey;
    loadDashboard({ reason: "managed-tools-change" }).catch(() => {});
  });

  /** @param {string} view @param {string | null | undefined} toolId */
  function goToView(view, toolId) {
    if (toolId) {
      activeToolId.set(toolId);
      pendingSubTab.set("tool");
    } else {
      pendingSubTab.set("default");
    }
    activeView.set(view);
  }

  /** @param {any} tool */
  function primaryConfigHealthLabel(tool) {
    if (tool?.primary_config_health === "missing") return $t("dashboard.config_health.missing");
    if (tool?.primary_config_health === "unreadable") return $t("dashboard.config_health.unreadable");
    return "";
  }

  /** @param {any} tool */
  async function goToToolSettings(tool) {
    if (!tool?.tool_id) return;
    activeToolId.set(tool.tool_id);
    activeSettingsTab.set("tools");
    focusedSettingsToolId.set(tool.tool_id);
    activeView.set("settings");
    await loadTools().catch(() => {});
  }

</script>

<div class="view-panel dashboard">
  <div class="view-fixed-header dashboard-fixed-header">
    <div class="view-header dashboard-header" data-tauri-drag-region>
      <h2 data-tauri-drag-region>{$t("sidebar.dashboard")}</h2>
      <div class="header-actions">
        <button
          type="button"
          class="refresh-btn"
          disabled={isRefreshing}
          onclick={handleRefresh}
          aria-label={$t("global.action.refresh")}
        >
          <RefreshCw size={14} strokeWidth={1.8} class={isRefreshing ? "spin" : ""} />
        </button>
      </div>
    </div>
  </div>

  <div class="view-scroll-content dashboard-scroll">
    <div class="management-content-shell">
      <p class="dashboard-subtitle dashboard-subtitle-below">
        {#if loading}
          {$t('dashboard.initializing')}
        {:else}
          {$t('dashboard.summary', { tools: visibleTools.length })}
        {/if}
      </p>
      <ModulePerformanceBar moduleId="dashboard" />
      {#if loading}
        <div class="loading">{$t('common.loading')}</div>
      {:else if data}
        <div class="bento-grid">
          {#each visibleTools as tool}
            <div class="bento-card">
              <div class="card-header">
                <div class="tool-brand">
                  <span class="tool-icon"><ToolIcon toolId={tool.tool_id} size={32} /></span>
                  <span class="tool-name">{tool.tool_name}</span>
                </div>
                {#if primaryConfigHealthLabel(tool)}
                  <Tooltip label={primaryConfigHealthLabel(tool)} placement="bottom-end">
                    <button
                      type="button"
                      class="status-badge warning"
                      aria-label={`${$t("dashboard.config_health.abnormal")} · ${primaryConfigHealthLabel(tool)}`}
                      onclick={() => { void goToToolSettings(tool); }}
                    >{$t("dashboard.config_health.abnormal")}</button>
                  </Tooltip>
                {/if}
              </div>

              <div class="metrics-grid">
                <button class="metric-glass rules" disabled={tool.rule_count === 0} onclick={() => goToView("rules", tool.tool_id)}>
                  <div class="metric-val">{tool.rule_count}</div>
                  <div class="metric-lbl"><ScrollText size={12} /> {$t('dashboard.metrics.rules')}</div>
                </button>
                
                <button class="metric-glass skills" disabled={tool.skill_count === 0} onclick={() => goToView("skills", tool.tool_id)}>
                  <div class="metric-val">{tool.skill_count}</div>
                  <div class="metric-lbl"><Blocks size={12} /> {$t('dashboard.metrics.skills')}</div>
                </button>
                
                <button class="metric-glass mcp" disabled={tool.mcp_count === 0} onclick={() => goToView("mcp", tool.tool_id)}>
                  <div class="metric-val">{tool.mcp_count}</div>
                  <div class="metric-lbl"><PlugZap size={12} /> {$t('dashboard.metrics.mcp')}</div>
                </button>
                
                <button class="metric-glass config" disabled={tool.config_count === 0} onclick={() => goToView("config", tool.tool_id)}>
                  <div class="metric-val">{tool.config_count}</div>
                  <div class="metric-lbl"><Settings2 size={12} /> {$t('dashboard.metrics.configs')}</div>
                </button>
              </div>
            </div>
          {/each}
          {#if visibleTools.length === 0}
            <div class="no-tools-tip">{$t('dashboard.no_tools')}</div>
          {/if}
        </div>
      {:else}
        <div class="loading">{$t('common.error_loading')}</div>
      {/if}
    </div>
  </div>
</div>

<style>
  /* Dashboard wrapper */
  .dashboard {
    height: 100%;
  }

  .dashboard-subtitle {
    font-size: 12px;
    color: var(--color-text-muted);
  }
  .dashboard-subtitle-below {
    margin: 0 0 16px;
  }

  /* Bento Grid */
  .bento-grid { 
    display: grid; 
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 20px; 
  }
  
  /* Bento Card */
  .bento-card { 
    background: var(--bg-card); 
    border-radius: 16px; 
    padding: 24px; 
    border: 1px solid var(--border-color); 
    transition: background 0.15s ease, border-color 0.15s ease;
  }
  .bento-card:hover {
    background: var(--bg-hover);
    border-color: var(--border-active);
  }

  .no-tools-tip {
    grid-column: 1 / -1;
    text-align: center;
    padding: 60px 24px;
    font-size: 14px;
    color: var(--color-text-muted);
    opacity: 0.7;
  }

  /* Card Header */
  .card-header { 
    display: flex; justify-content: space-between; align-items: flex-start;
    margin-bottom: 18px;
  }
  .tool-brand {
    display: flex; align-items: center; gap: 12px;
  }
  .tool-icon { 
    display: flex; align-items: center; justify-content: center; 
    /* Removed background/border-radius to prevent double-box effect for icons */
  }
  .tool-name { font-size: 18px; font-weight: 600; color: var(--color-text-main); letter-spacing: -0.3px; }
  
  .status-badge {
    font-size: 11px; font-weight: 600; padding: 4px 8px; border-radius: 8px;
    border: 1px solid transparent; cursor: pointer; line-height: 1.2;
  }
  .status-badge.warning { background: rgba(245, 158, 11, 0.12); color: #f59e0b; }
  .status-badge.warning:hover { border-color: rgba(245, 158, 11, 0.32); background: rgba(245, 158, 11, 0.18); }
  .status-badge.warning:focus-visible { outline: 2px solid rgba(245, 158, 11, 0.45); outline-offset: 2px; }

  /* Glass Metrics (Inner Buttons) */
  .metrics-grid { 
    display: grid; grid-template-columns: repeat(2, 1fr); gap: 12px; 
  }
  .metric-glass { 
    background: var(--bg-subtle); 
    border-radius: 16px; 
    padding: 16px 12px; 
    text-align: center; 
    border: 1px solid var(--border-color); 
    color: var(--color-text-main);
    cursor: pointer; 
    display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px;
    transition: all 0.2s cubic-bezier(0.18, 0.89, 0.32, 1.28); 
  }
  .metric-glass:disabled {
    opacity: 0.35;
    cursor: not-allowed;
    pointer-events: none;
  }
  
  .metric-val { font-size: 20px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .metric-lbl { display: flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 500; opacity: 0.6; color: var(--color-text-main); }
  
  
  /* Colorized Hover States for Metrics */
  .metric-glass:hover {
    background: var(--bg-active);
    border-color: var(--border-active);
  }
  .metric-glass:active:not(:disabled) {
    transform: scale(0.96);
    opacity: 0.9;
  }
  .metric-glass.rules:hover { color: #a78bfa; }
  .metric-glass.skills:hover { color: #4ade80; }
  .metric-glass.config:hover { color: #f59e0b; }
  .metric-glass.mcp:hover { color: var(--module-mcp-accent); }



  .loading { text-align: center; padding: 60px; color: var(--color-text-muted); opacity: 0.6; font-size: 14px; }

  :global(.spin) { animation: spin 0.8s linear infinite; }
  @keyframes spin { 100% { transform: rotate(360deg); } }
</style>
