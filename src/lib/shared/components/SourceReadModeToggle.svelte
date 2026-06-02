<script>
  // Purpose: Shared compact source/read view-mode switch for file-like content.
  import { CodeXml, Eye } from "lucide-svelte";

  let {
    preview = $bindable(true),
    editing = false,
    sourceLabel = "Source",
    readLabel = "Read",
  } = $props();
</script>

{#if editing}
  <span class="source-read-mode-indicator" aria-label={sourceLabel}>
    <CodeXml size={14} strokeWidth={1.8} aria-hidden="true" />
    <span class="sr-only">{sourceLabel}</span>
  </span>
{:else}
  <div class="source-read-mode-toggle" role="group" aria-label={`${sourceLabel} / ${readLabel}`}>
    <button
      type="button"
      class="mode-btn"
      class:active={!preview}
      onclick={() => (preview = false)}
      aria-label={sourceLabel}
      aria-pressed={!preview}
    >
      <CodeXml size={14} strokeWidth={1.8} aria-hidden="true" />
      <span class="sr-only">{sourceLabel}</span>
    </button>
    <button
      type="button"
      class="mode-btn"
      class:active={preview}
      onclick={() => (preview = true)}
      aria-label={readLabel}
      aria-pressed={preview}
    >
      <Eye size={14} strokeWidth={1.8} aria-hidden="true" />
      <span class="sr-only">{readLabel}</span>
    </button>
  </div>
{/if}

<style>
  .source-read-mode-toggle {
    display: inline-grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    width: 76px;
    height: 28px;
    flex: 0 0 auto;
    align-items: stretch;
    background: var(--toolbar-control-bg);
    border-radius: var(--segmented-track-radius);
    padding: var(--segmented-track-padding);
    border: 1px solid var(--toolbar-control-border);
    overflow: visible;
    box-sizing: border-box;
  }
  .mode-btn {
    width: 100%;
    height: 100%;
    min-width: 0;
    font-size: 11px;
    font-weight: 500;
    padding: 0;
    border: none;
    border-radius: var(--segmented-item-radius);
    cursor: pointer;
    background: transparent;
    color: var(--color-text-muted);
    transition: background 0.18s ease, color 0.18s ease, box-shadow 0.18s ease;
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .mode-btn:hover { color: var(--color-text-main); }
  .mode-btn.active {
    background: var(--bg-card);
    color: var(--color-text-main);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05), 0 2px 8px rgba(0, 0, 0, 0.05);
  }
  :global([data-theme="dark"]) .mode-btn.active {
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35), 0 2px 8px rgba(0, 0, 0, 0.25);
  }
  .mode-btn:focus-visible {
    outline: 1px solid var(--toolbar-control-border-active);
    outline-offset: -1px;
  }
  .source-read-mode-indicator {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
    min-width: 38px;
    height: 28px;
    font-size: 11px;
    font-weight: 500;
    color: var(--color-text-main);
    padding: 2px 9px;
    border: none;
    border-radius: 8px;
    background: var(--toolbar-control-bg);
    white-space: nowrap;
    box-sizing: border-box;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
