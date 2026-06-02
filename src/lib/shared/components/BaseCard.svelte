<script>
  import { FileText, Link2, AlertCircle } from "lucide-svelte";
  import Tooltip from "$lib/shared/components/Tooltip.svelte";
  let {
    icon = null,
    accentColor = "",
    title,
    path = "",
    description = "",
    stats = "",
    badges = [],
    note = "",
    compact = false,
    showIcon = true,
    selectable = false,
    selected = false,
    disabled = false,
    /** @type {any} */
    onclick = () => {},
    onBadgeClick = null, // (badge) => void, for quick-install
    /** @type {{ title?: string, onClick?: () => void } | null} */
    warning = null,
    style = "",
    /** @type {import("svelte").Snippet | undefined} */
    trailing = undefined,
  } = $props();

  /** @param {any} e */
  function handleCardClick(e) {
    if (disabled) return;
    onclick(e);
  }

  /** @param {MouseEvent} e */
  function handleWarningClick(e) {
    e.stopPropagation();
    if (warning && typeof warning.onClick === "function") {
      warning.onClick();
    }
  }
</script>

{#snippet cardInner()}
  {#if showIcon && icon}
    <div class="card-icon-wrap">
      <div class="card-icon" style="{accentColor ? `background: ${accentColor};` : ''}">
        <svelte:component this={icon} size={compact ? 12 : 14} color="currentColor" />
      </div>
      {#if selectable}
        <div class="card-checkbox" class:checked={selected}>
          {#if selected}<svg width="10" height="10" viewBox="0 0 10 10" fill="none"><path d="M2 5L4.5 7.5L8 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>{/if}
        </div>
      {/if}
    </div>
  {/if}
  <div class="card-body">
    <div class="card-top">
      <div class="card-info">
        <div class="card-title">{title}</div>
        {#if description}
          <div class="card-desc">{description}</div>
        {/if}
      </div>
      {#if note}
        <div class="card-note">{note}</div>
      {/if}
    </div>
    {#if path}
      <div class="card-path">{path}</div>
    {/if}
    {#if badges.length > 0}
      <div class="card-badges">
        {#each badges as badge}
          {#if badge.status === "installed"}
            <span class="badge installed">
              {badge.label}
              {#if !badge.hideKindIcon}
                {#if badge.mode === "symlink"}<Link2 size={9} />{:else}<FileText size={9} />{/if}
              {/if}
            </span>
          {:else if badge.status === "available" && onBadgeClick}
            <button class="badge available" onclick={(e) => { e.stopPropagation(); onBadgeClick(badge); }}>{badge.label} +</button>
          {:else}
            <span class="badge">{badge.label}</span>
          {/if}
        {/each}
      </div>
    {/if}
  </div>
  {#if stats || warning}
    <div class="card-end-cap">
      {#if warning}
        {#if typeof warning.onClick === "function"}
          <Tooltip label={warning.title || ""} placement="left" maxWidth="320px">
            <button
              type="button"
              class="card-warning clickable"
              aria-label={warning.title || "warning"}
              onclick={handleWarningClick}
            >
              <AlertCircle size={14} />
            </button>
          </Tooltip>
        {:else}
          <Tooltip label={warning.title || ""} placement="left" maxWidth="320px">
            <span class="card-warning" aria-label={warning.title || "warning"}>
              <AlertCircle size={14} />
            </span>
          </Tooltip>
        {/if}
      {/if}
      {#if stats}
        <div class="card-stats">{stats}</div>
      {/if}
    </div>
  {/if}
{/snippet}

{#if trailing}
  <div class="base-card-row">
    <button type="button" class="base-card" class:compact class:selected={selectable && selected} onclick={handleCardClick} {style} {disabled}>
      {@render cardInner()}
    </button>
    <div class="base-card-trailing">
      {@render trailing()}
    </div>
  </div>
{:else}
  <button type="button" class="base-card" class:compact class:selected={selectable && selected} onclick={handleCardClick} {style} {disabled}>
    {@render cardInner()}
  </button>
{/if}

<style>
  .base-card-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    overflow: visible;
  }
  .base-card-row .base-card {
    flex: 1;
    min-width: 0;
  }
  .base-card-trailing {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .base-card {
    display: flex; align-items: center; gap: 14px;
    background: var(--bg-card);
    border-radius: 8px; padding: 12px 14px;
    border: 1px solid var(--border-color); cursor: pointer;
    transition: border-color 0.15s, background 0.15s, color 0.15s;
    width: 100%; text-align: left;
    box-shadow: none;
    overflow: visible;
  }
  .base-card:hover {
    border-color: var(--border-active);
    background: var(--bg-hover);
    box-shadow: none;
  }
  .base-card:disabled {
    cursor: default;
  }
  .base-card:disabled:hover {
    border-color: var(--border-color);
    background: var(--bg-card);
  }
  .base-card.selected {
    border-color: var(--border-active);
    background: var(--bg-active);
    box-shadow: none;
  }
  .base-card.compact { padding: 10px 12px; }

  .card-icon-wrap { position: relative; flex-shrink: 0; }
  .card-checkbox { position: absolute; bottom: -3px; right: -3px; width: 14px; height: 14px; border-radius: 4px; border: 1.5px solid var(--border-active); background: var(--bg-card); color: var(--color-text-muted); display: flex; align-items: center; justify-content: center; transition: all 0.15s; }
  .card-checkbox.checked { background: var(--color-text-main); border-color: var(--color-text-main); color: var(--bg-card); }

  .card-icon {
    width: 28px;
    height: 28px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: var(--bg-elevated-subtle);
    color: var(--color-text-muted);
    border: 1px solid var(--border-color);
    box-shadow: none;
  }
  .base-card.compact .card-icon { width: 24px; height: 24px; border-radius: 8px; }

  .card-body { flex: 1; min-width: 0; }
  .card-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }
  .card-info { min-width: 0; flex: 1; }
  .card-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--color-text-main);
    letter-spacing: -0.01em;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card-desc { font-size: 11px; color: var(--color-text-muted); line-height: 1.55; margin-top: 4px; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .card-note { font-size: 10px; color: var(--color-text-muted); opacity: 0.7; flex-shrink: 0; max-width: 150px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .card-warning { position: relative; display: inline-flex; align-items: center; justify-content: center; color: #f59e0b; flex-shrink: 0; line-height: 0; padding: 0; background: transparent; border: none; overflow: visible; }
  .card-warning.clickable { cursor: pointer; border-radius: 4px; padding: 2px; transition: background 0.15s; }
  .card-warning.clickable:hover { background: rgba(245,158,11,0.12); }
  .card-path { font-size: 11px; color: var(--color-text-muted); opacity: 0.52; font-family: "SF Mono", "Fira Code", monospace; margin-top: 6px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  /* 警告与行数同一右列，右缘对齐，避免与正文区右边界不一致 */
  .card-end-cap {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    justify-content: center;
    gap: 4px;
    flex-shrink: 0;
    min-width: 3.5rem;
    text-align: right;
  }
  .card-stats { font-size: 11px; color: var(--color-text-muted); opacity: 0.6; white-space: nowrap; font-variant-numeric: tabular-nums; flex-shrink: 0; line-height: 1.2; }

  .card-badges { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 10px; }
  .badge {
    font-size: 10px;
    padding: 4px 9px;
    border-radius: 999px;
    background: var(--bg-subtle);
    color: var(--color-text-muted);
    opacity: 0.7;
    transition: all 0.15s;
    border: 1px solid var(--border-color);
  }
  .badge.installed { background: var(--bg-active); color: var(--color-text-main); opacity: 1; display: inline-flex; align-items: center; gap: 3px; border-color: var(--border-color); }
  .badge.available {
    cursor: pointer;
    border: 1px dashed var(--border-active);
    background: transparent;
    opacity: 0.72;
  }
  .badge.available:hover {
    border-color: var(--border-active);
    color: var(--color-text-main);
    opacity: 1;
  }

  :global([data-theme="dark"]) .base-card {
    background: var(--bg-card);
    border-color: var(--border-color);
    box-shadow: none;
  }

  :global([data-theme="dark"]) .base-card:hover {
    background: var(--bg-hover);
    border-color: var(--border-active);
    box-shadow: none;
  }
  :global([data-theme="dark"]) .base-card:disabled:hover {
    background: var(--bg-card);
    border-color: var(--border-color);
  }

  :global([data-theme="dark"]) .base-card.selected {
    background: var(--bg-active);
    border-color: var(--border-active);
    box-shadow: none;
  }

  :global([data-theme="dark"]) .card-icon {
    background: var(--bg-elevated-subtle);
    border-color: var(--border-color);
    box-shadow: none;
  }

  :global([data-theme="dark"]) .badge {
    background: var(--bg-subtle);
    border-color: var(--border-color);
  }

  :global([data-theme="dark"]) .badge.installed {
    background: var(--bg-active);
    border-color: var(--border-color);
  }

  :global([data-theme="dark"]) .badge.available {
    border-color: var(--border-active);
  }
</style>
