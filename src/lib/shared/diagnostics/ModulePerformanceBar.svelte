<!-- Purpose: Compact shared display for the active module's latest performance diagnostic run. -->
<script>
  import { modulePerformanceDiagnosticsEnabled, modulePerformanceSummaries } from "$lib/shared/diagnostics/modulePerformance.js";
  import { t } from "$lib/shared/i18n/index.js";
  import { ChevronDown, ChevronUp } from "lucide-svelte";

  let { moduleId = "" } = $props();
  let summary = $derived($modulePerformanceSummaries[moduleId] || null);
  let expanded = $state(false);
  let expandedSummaryId = $state("");

  $effect(() => {
    const summaryId = summary?.id || "";
    if (summaryId === expandedSummaryId) return;
    expanded = false;
    expandedSummaryId = summaryId;
  });

  /** @param {number | null | undefined} value */
  function formatMs(value) {
    if (!Number.isFinite(value)) return "-";
    return `${value}ms`;
  }

  /** @param {string} id */
  function moduleLabel(id) {
    /** @type {Record<string, string>} */
    const keys = {
      dashboard: "sidebar.dashboard",
      rules: "sidebar.rules",
      skills: "sidebar.skills",
      mcp: "sidebar.mcp",
      config: "sidebar.config",
      settings: "sidebar.settings",
      tools: "settings.tab.tools",
    };
    const key = keys[id] || "";
    return key ? $t(key) : id;
  }

  /** @param {string | null | undefined} value */
  function humanizeLabel(value) {
    const raw = String(value || "").trim();
    if (!raw) return "";
    /** @type {Record<string, string>} */
    const known = {
      "claude-code": "Claude Code",
      codex: "Codex",
      openclaw: "OpenClaw",
      cursor: "Cursor",
      qoder: "Qoder",
      opencode: "OpenCode",
      windsurf: "Windsurf",
      "github-copilot": "GitHub Copilot",
      "trae-cn": "Trae CN",
      "trae-solo-cn": "Trae Solo CN",
    };
    if (known[raw]) return known[raw];
    return raw
      .split(/[-_\s]+/)
      .filter(Boolean)
      .map((part) => part.length <= 3 ? part.toUpperCase() : `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
      .join(" ");
  }

  /** @param {any} run */
  function scopeLabel(run) {
    const moduleName = moduleLabel(run?.module || "");
    const view = String(run?.view || "").trim();
    if (!view) return moduleName;
    /** @type {Record<string, string>} */
    const viewKeys = {
      overview: "performance_diagnostics.scope.overview",
      common: "performance_diagnostics.scope.shared",
      tool: "performance_diagnostics.scope.tools",
      global: "performance_diagnostics.scope.global",
      general: "settings.tab.general",
      tools: "settings.tab.tools",
    };
    const key = viewKeys[view] || "";
    const viewName = key ? $t(key) : humanizeLabel(view);
    return viewName ? `${moduleName} · ${viewName}` : moduleName;
  }

  /** @param {any} run */
  function requestCountLabel(run) {
    return String(run?.requestCount || 0);
  }

  /** @param {any} run */
  function milestoneTitle(run) {
    const milestones = /** @type {Array<{ name?: string }>} */ (run?.milestones || []);
    const keyMilestones = milestones.slice(-3).map((entry) => entry.name).filter(Boolean);
    return keyMilestones.length ? keyMilestones.join(" / ") : "-";
  }

  /** @param {Record<string, number> | null | undefined} record */
  function entries(record) {
    return Object.entries(record || {});
  }

  /** @param {string} label @param {number | string} value */
  function chipText(label, value) {
    return `${label}: ${value}`;
  }
</script>

{#if $modulePerformanceDiagnosticsEnabled && summary}
  <section class="module-performance-bar" class:module-performance-bar--expanded={expanded} aria-label={$t("performance_diagnostics.label")}>
    <div class="module-performance-bar__summary">
      <span class="module-performance-bar__title">{$t("performance_diagnostics.title")}</span>
      <span class="module-performance-bar__scope">{scopeLabel(summary)}</span>
      <span class="module-performance-bar__metric">{$t("performance_diagnostics.visible")}: {formatMs(summary.visibleMs)}</span>
      <span class="module-performance-bar__metric">{$t("performance_diagnostics.interactive")}: {formatMs(summary.interactiveMs)}</span>
      <span class="module-performance-bar__metric">{$t("performance_diagnostics.background")}: {formatMs(summary.backgroundCompleteMs)}</span>
      <span class="module-performance-bar__metric">{$t("performance_diagnostics.requests")}: {requestCountLabel(summary)}</span>
      <span class="module-performance-bar__status">{summary.status}</span>
      <button
        type="button"
        class="module-performance-bar__toggle"
        aria-expanded={expanded}
        aria-label={expanded ? $t("performance_diagnostics.collapse") : $t("performance_diagnostics.expand")}
        title={expanded ? $t("performance_diagnostics.collapse") : $t("performance_diagnostics.expand")}
        onclick={() => { expanded = !expanded; }}
      >
        {#if expanded}
          <ChevronUp size={14} aria-hidden="true" />
        {:else}
          <ChevronDown size={14} aria-hidden="true" />
        {/if}
      </button>
    </div>

    {#if expanded}
      <div class="module-performance-bar__details">
        <div class="module-performance-bar__detail-row">
          <span class="module-performance-bar__detail-label">{$t("performance_diagnostics.reason")}</span>
          <span class="module-performance-bar__chip">{humanizeLabel(summary.reason)}</span>
          <span class="module-performance-bar__detail-label">{$t("performance_diagnostics.total")}</span>
          <span class="module-performance-bar__chip">{formatMs(summary.totalMs)}</span>
          <span class="module-performance-bar__detail-label">{$t("performance_diagnostics.status")}</span>
          <span class="module-performance-bar__chip">{summary.status}</span>
        </div>

        <div class="module-performance-bar__detail-block">
          <span class="module-performance-bar__detail-label">{$t("performance_diagnostics.request_details")}</span>
          <div class="module-performance-bar__chips">
            {#each entries(summary.requestCounts) as [label, value]}
              <span class="module-performance-bar__chip">{chipText(label, value)}</span>
            {:else}
              <span class="module-performance-bar__chip module-performance-bar__chip--muted">{$t("performance_diagnostics.empty")}</span>
            {/each}
          </div>
        </div>

        <div class="module-performance-bar__detail-block">
          <span class="module-performance-bar__detail-label">{$t("performance_diagnostics.milestones")}</span>
          <div class="module-performance-bar__chips" title={milestoneTitle(summary)}>
            {#each summary.milestones || [] as milestone}
              <span class="module-performance-bar__chip">{milestone.name}{Number.isFinite(milestone.atMs) ? ` · ${formatMs(milestone.atMs)}` : ""}</span>
            {:else}
              <span class="module-performance-bar__chip module-performance-bar__chip--muted">{$t("performance_diagnostics.empty")}</span>
            {/each}
          </div>
        </div>

        <div class="module-performance-bar__detail-block">
          <span class="module-performance-bar__detail-label">{$t("performance_diagnostics.counters")}</span>
          <div class="module-performance-bar__chips">
            {#each entries(summary.counters) as [label, value]}
              <span class="module-performance-bar__chip">{chipText(label, value)}</span>
            {:else}
              <span class="module-performance-bar__chip module-performance-bar__chip--muted">{$t("performance_diagnostics.empty")}</span>
            {/each}
          </div>
        </div>
      </div>
    {/if}
  </section>
{/if}

<style>
  .module-performance-bar {
    min-height: 28px;
    margin: 0 0 14px;
    padding: 6px 10px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: color-mix(in srgb, var(--bg-hover) 72%, transparent);
    color: var(--color-text-muted);
    font-size: 11px;
    line-height: 1.35;
    overflow: hidden;
  }

  .module-performance-bar__summary {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .module-performance-bar__title {
    flex: 0 0 auto;
    font-weight: 650;
    color: var(--color-text-main);
  }

  .module-performance-bar__scope,
  .module-performance-bar__metric,
  .module-performance-bar__status {
    flex: 0 1 auto;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .module-performance-bar__scope {
    max-width: 24ch;
  }

  .module-performance-bar__metric,
  .module-performance-bar__status {
    flex: 0 0 auto;
  }

  .module-performance-bar__status {
    margin-left: auto;
    text-transform: capitalize;
  }

  .module-performance-bar__toggle {
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .module-performance-bar__toggle:hover {
    background: var(--bg-hover);
    color: var(--color-text-main);
  }

  .module-performance-bar__details {
    display: grid;
    gap: 8px;
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--border-color);
  }

  .module-performance-bar__detail-row,
  .module-performance-bar__detail-block {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    min-width: 0;
  }

  .module-performance-bar__detail-label {
    flex: 0 0 auto;
    color: var(--color-text-main);
    font-weight: 650;
  }

  .module-performance-bar__chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
  }

  .module-performance-bar__chip {
    max-width: 100%;
    padding: 2px 6px;
    border-radius: 6px;
    background: var(--bg-card);
    color: var(--color-text-muted);
    overflow-wrap: anywhere;
  }

  .module-performance-bar__chip--muted {
    opacity: 0.72;
  }

  @media (max-width: 980px) {
    .module-performance-bar__summary {
      flex-wrap: wrap;
      gap: 6px 10px;
    }

    .module-performance-bar__status {
      margin-left: 0;
    }

    .module-performance-bar__detail-row,
    .module-performance-bar__detail-block {
      flex-wrap: wrap;
    }
  }
</style>
