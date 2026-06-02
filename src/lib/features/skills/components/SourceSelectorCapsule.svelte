<script>
  // Purpose: Select a concrete Skill content source with the shared capsule interaction.
  import { onDestroy, onMount, tick } from "svelte";
  import { Blocks } from "lucide-svelte";
  import ToolIcon from "$lib/shared/components/ToolIcon.svelte";
  import Tooltip from "$lib/shared/components/Tooltip.svelte";
  import {
    protectedToolSelectorMinWidth,
    resolveToolSelectorMode,
  } from "$lib/shared/utils/toolSelectorCapsule.js";

  let {
    sources = [],
    activeKey = "",
    ariaLabel = "Sources",
    class: className = "",
    onSelect = () => {},
  } = $props();

  /** @type {HTMLDivElement | null} */
  let rootElement = null;
  /** @type {HTMLDivElement | null} */
  let measureElement = null;
  /** @type {"full" | "icon"} */
  let mode = $state("full");
  let resizeObserver = /** @type {ResizeObserver | null} */ (null);
  let measureFrame = /** @type {number | null} */ (null);

  let minWidth = $derived(protectedToolSelectorMinWidth(sources.length));

  /** @param {HTMLElement | null} element */
  function contentBoxWidth(element) {
    if (!element) return 0;
    const width = element.clientWidth || element.getBoundingClientRect().width || 0;
    if (width <= 0 || typeof getComputedStyle === "undefined") return width;
    const style = getComputedStyle(element);
    const paddingLeft = Number.parseFloat(style.paddingLeft) || 0;
    const paddingRight = Number.parseFloat(style.paddingRight) || 0;
    return Math.max(0, width - paddingLeft - paddingRight);
  }

  function runMeasurement() {
    measureFrame = null;
    if (!rootElement || !measureElement || sources.length === 0) return;

    const parentWidth = contentBoxWidth(rootElement.parentElement);
    const availableWidth = parentWidth || rootElement.clientWidth || rootElement.getBoundingClientRect().width;
    const fullWidth = measureElement.scrollWidth || measureElement.getBoundingClientRect().width;
    mode = resolveToolSelectorMode({ availableWidth, fullWidth, currentMode: mode });
  }

  function scheduleMeasurement() {
    if (typeof window === "undefined") return;
    if (measureFrame != null) window.cancelAnimationFrame(measureFrame);
    measureFrame = window.requestAnimationFrame(runMeasurement);
  }

  /** @param {any} source */
  function sourceTitle(source) {
    return [source?.label, source?.path].filter(Boolean).join("\n");
  }

  /** @param {any} source */
  function sourceAriaLabel(source) {
    const suffix = source?.duplicateCount > 1 ? ` ${source.duplicateIndex}` : "";
    return `${source?.label || ""}${suffix}`.trim() || source?.path || "";
  }

  onMount(() => {
    if (typeof ResizeObserver !== "undefined") {
      resizeObserver = new ResizeObserver(scheduleMeasurement);
      if (rootElement) resizeObserver.observe(rootElement);
      if (rootElement?.parentElement) resizeObserver.observe(rootElement.parentElement);
    } else {
      window.addEventListener("resize", scheduleMeasurement);
    }
    void tick().then(scheduleMeasurement);
  });

  onDestroy(() => {
    if (resizeObserver) resizeObserver.disconnect();
    if (typeof window !== "undefined") {
      window.removeEventListener("resize", scheduleMeasurement);
    }
    if (measureFrame != null && typeof window !== "undefined") {
      window.cancelAnimationFrame(measureFrame);
    }
  });

  $effect(() => {
    sources;
    activeKey;
    void tick().then(scheduleMeasurement);
  });
</script>

<div
  bind:this={rootElement}
  class="source-selector-capsule {className}"
  data-mode={mode}
  role="group"
  aria-label={ariaLabel}
  style:min-width={`${minWidth}px`}
>
  <div
    bind:this={measureElement}
    class="source-selector-capsule__measure"
    aria-hidden="true"
    style="position: fixed; top: 0; left: 0; transform: translate(-10000px, -10000px); visibility: hidden;"
  >
    {#each sources as source (source.key)}
      <div class="source-selector-capsule__segment source-selector-capsule__segment--measure">
        {#if source.pathOrigin === "generic"}
          <Blocks size={16} />
        {:else}
          <ToolIcon toolId={source.toolId} size={16} />
        {/if}
        <span class="source-selector-capsule__label">{source.label}</span>
        {#if source.duplicateCount > 1}
          <span class="source-selector-capsule__badge">{source.duplicateIndex}</span>
        {/if}
      </div>
    {/each}
  </div>

  {#each sources as source (source.key)}
    <Tooltip label={mode === "icon" ? sourceTitle(source) : ""} placement="bottom" maxWidth="320px">
      <button
        type="button"
        class="source-selector-capsule__segment"
        class:is-active={activeKey === source.key}
        class:is-abnormal={source.abnormal}
        aria-pressed={activeKey === source.key}
        aria-label={sourceAriaLabel(source)}
        onclick={() => onSelect(source.key)}
      >
        {#if source.pathOrigin === "generic"}
          <Blocks size={16} />
        {:else}
          <ToolIcon toolId={source.toolId} size={16} />
        {/if}
        {#if mode === "full"}
          <span class="source-selector-capsule__label">{source.label}</span>
        {/if}
        {#if source.duplicateCount > 1}
          <span class="source-selector-capsule__badge">{source.duplicateIndex}</span>
        {/if}
      </button>
    </Tooltip>
  {/each}
</div>

<style>
  .source-selector-capsule {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    width: max-content;
    max-width: 100%;
    min-height: var(--header-ghost-control-size, 32px);
    padding: 2px;
    color: var(--color-text-muted);
    background: var(--bg-elevated-subtle);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    box-sizing: border-box;
    overflow: visible;
    flex: 0 0 auto;
    -webkit-app-region: no-drag;
  }

  .source-selector-capsule[data-mode="icon"] {
    max-width: none;
  }

  .source-selector-capsule__measure {
    z-index: -1;
    display: inline-flex;
    gap: 2px;
    width: max-content;
    padding: 2px;
    pointer-events: none;
    box-sizing: border-box;
  }

  .source-selector-capsule__segment {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-width: 36px;
    height: 28px;
    padding: 0 10px;
    color: var(--color-text-muted);
    background: transparent;
    border: 0;
    border-radius: 9px;
    font-size: 12px;
    font-weight: 500;
    line-height: 1;
    white-space: nowrap;
    cursor: pointer;
    transition: background 0.15s, color 0.15s, box-shadow 0.15s;
    box-sizing: border-box;
    flex: 0 0 auto;
  }

  .source-selector-capsule[data-mode="icon"] .source-selector-capsule__segment {
    width: 36px;
    padding: 0;
  }

  .source-selector-capsule[data-mode="icon"] .source-selector-capsule__measure .source-selector-capsule__segment {
    width: auto;
    padding: 0 10px;
  }

  .source-selector-capsule__segment:hover {
    color: var(--color-text-main);
    background: var(--bg-hover);
  }

  .source-selector-capsule__segment:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .source-selector-capsule__segment.is-active {
    color: var(--color-text-main);
    background: var(--bg-card);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
    font-weight: 600;
  }

  .source-selector-capsule__segment.is-abnormal {
    color: #b45309;
  }

  .source-selector-capsule__segment.is-abnormal.is-active {
    box-shadow: 0 0 0 1px rgba(217, 119, 6, 0.34), 0 1px 2px rgba(0, 0, 0, 0.04);
  }

  .source-selector-capsule__segment:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .source-selector-capsule__segment--measure {
    pointer-events: none;
  }

  .source-selector-capsule__label {
    min-width: 0;
  }

  .source-selector-capsule__badge {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    border-radius: 999px;
    border: 1px solid var(--bg-card);
    background: #d97706;
    color: #fff;
    font-size: 9px;
    font-weight: 700;
    line-height: 12px;
    box-sizing: border-box;
  }

  :global([data-theme="dark"]) .source-selector-capsule__segment.is-active {
    background: var(--bg-card);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.28);
  }

  :global([data-theme="dark"]) .source-selector-capsule__segment.is-abnormal.is-active {
    box-shadow: 0 0 0 1px rgba(245, 158, 11, 0.42), 0 1px 2px rgba(0, 0, 0, 0.28);
  }
</style>
