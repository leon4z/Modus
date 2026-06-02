<script>
  import { onDestroy, onMount, tick } from "svelte";
  import ToolIcon from "$lib/shared/components/ToolIcon.svelte";
  import Tooltip from "$lib/shared/components/Tooltip.svelte";
  import {
    protectedToolSelectorMinWidth,
    resolveToolSelectorMode,
  } from "$lib/shared/utils/toolSelectorCapsule.js";

  let {
    tools = [],
    activeToolId = null,
    ariaLabel = "Tools",
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

  let minWidth = $derived(protectedToolSelectorMinWidth(tools.length));

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
    if (!rootElement || !measureElement || tools.length === 0) return;

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
    tools;
    activeToolId;
    void tick().then(scheduleMeasurement);
  });
</script>

<div
  bind:this={rootElement}
  class="tool-selector-capsule {className}"
  data-mode={mode}
  role="group"
  aria-label={ariaLabel}
  style:min-width={`${minWidth}px`}
>
  <div
    bind:this={measureElement}
    class="tool-selector-capsule__measure"
    aria-hidden="true"
    style="position: fixed; top: 0; left: 0; transform: translate(-10000px, -10000px); visibility: hidden;"
  >
    {#each tools as tool (tool.id)}
      <div class="tool-selector-capsule__segment tool-selector-capsule__segment--measure">
        <ToolIcon toolId={tool.id} size={16} />
        <span class="tool-selector-capsule__label">{tool.name}</span>
      </div>
    {/each}
  </div>

  {#each tools as tool (tool.id)}
    <Tooltip label={mode === "icon" ? tool.name : ""} placement="bottom">
      <button
        type="button"
        class="tool-selector-capsule__segment"
        class:is-active={activeToolId === tool.id}
        aria-pressed={activeToolId === tool.id}
        aria-label={tool.name}
        onclick={() => onSelect(tool.id)}
      >
        <ToolIcon toolId={tool.id} size={16} />
        {#if mode === "full"}
          <span class="tool-selector-capsule__label">{tool.name}</span>
        {/if}
      </button>
    </Tooltip>
  {/each}
</div>

<style>
  .tool-selector-capsule {
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

  .tool-selector-capsule[data-mode="icon"] {
    max-width: none;
  }

  .tool-selector-capsule__measure {
    z-index: -1;
    display: inline-flex;
    gap: 2px;
    width: max-content;
    padding: 2px;
    pointer-events: none;
    box-sizing: border-box;
  }

  .tool-selector-capsule__segment {
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

  .tool-selector-capsule[data-mode="icon"] .tool-selector-capsule__segment {
    width: 36px;
    padding: 0;
  }

  .tool-selector-capsule[data-mode="icon"] .tool-selector-capsule__measure .tool-selector-capsule__segment {
    width: auto;
    padding: 0 10px;
  }

  .tool-selector-capsule__segment:hover {
    color: var(--color-text-main);
    background: var(--bg-hover);
  }

  .tool-selector-capsule__segment:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .tool-selector-capsule__segment.is-active {
    color: var(--color-text-main);
    background: var(--bg-card);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
    font-weight: 600;
  }

  .tool-selector-capsule__segment:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .tool-selector-capsule__segment--measure {
    pointer-events: none;
  }

  .tool-selector-capsule__label {
    min-width: 0;
  }

  :global([data-theme="dark"]) .tool-selector-capsule__segment.is-active {
    background: var(--bg-card);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.28);
  }
</style>
