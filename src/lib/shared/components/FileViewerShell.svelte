<script>
  // Purpose: Shared shell for file-like viewing/editing surfaces with a fixed title area.
  import { onDestroy } from "svelte";
  import { ArrowLeft, PanelLeft } from "lucide-svelte";
  import { formatPath } from "$lib/shared/utils/utils.js";

  let {
    variant = "single-file",
    title = "",
    titleContent = undefined,
    subtitle = "",
    subtitleMonospace = true,
    badge = "",
    modeLabel = "",
    backLabel = "Back",
    backDisabled = false,
    onBack = () => {},
    leading = undefined,
    icon = undefined,
    navigation = undefined,
    navigationVisible = undefined,
    navigationResizable = true,
    navigationCollapsible = false,
    navigationCollapsed = $bindable(false),
    navigationInitialWidth = 220,
    navigationMinWidth = 180,
    navigationMaxWidth = 420,
    navigationResizeLabel = "Resize file tree",
    navigationCollapseLabel = "Collapse file tree",
    navigationExpandLabel = "Expand file tree",
    context = undefined,
    contextVisible = undefined,
    actions = undefined,
    children = undefined,
  } = $props();

  const supportedVariants = new Set(["single-file", "collection-detail", "navigation-backed", "navigation-detail"]);

  let viewerVariant = $derived(supportedVariants.has(variant) ? variant : "single-file");
  let variantClass = $derived(`variant-${viewerVariant === "navigation-detail" ? "navigation-backed" : viewerVariant}`);
  let canRenderNavigation = $derived(Boolean(navigation) && (navigationVisible ?? true));
  let isNavigationCollapsed = $derived(Boolean(navigationCollapsible && navigationCollapsed));
  let shouldShowNavigation = $derived(canRenderNavigation && !isNavigationCollapsed);
  let canToggleNavigation = $derived(Boolean(canRenderNavigation && navigationCollapsible));
  let shouldShowContext = $derived(Boolean(context) && (contextVisible ?? true));
  let shouldShowBack = $derived(viewerVariant === "collection-detail");
  let displaySubtitle = $derived(formatPath(subtitle));
  let navigationWidth = $state(220);
  let resizingNavigation = $state(false);
  let resizeStartX = 0;
  let resizeStartWidth = 0;

  /** @param {number} width */
  function clampNavigationWidth(width) {
    return Math.min(navigationMaxWidth, Math.max(navigationMinWidth, width));
  }

  $effect(() => {
    navigationWidth = clampNavigationWidth(navigationInitialWidth);
  });

  function stopNavigationResize() {
    resizingNavigation = false;
    if (typeof window === "undefined") return;
    window.removeEventListener("pointermove", onNavigationResizeMove);
    window.removeEventListener("pointerup", stopNavigationResize);
    window.removeEventListener("pointercancel", stopNavigationResize);
  }

  /** @param {PointerEvent} event */
  function onNavigationResizeMove(event) {
    if (!resizingNavigation) return;
    navigationWidth = clampNavigationWidth(resizeStartWidth + event.clientX - resizeStartX);
  }

  /** @param {PointerEvent} event */
  function startNavigationResize(event) {
    if (!shouldShowNavigation || !navigationResizable) return;
    event.preventDefault();
    resizingNavigation = true;
    resizeStartX = event.clientX;
    resizeStartWidth = navigationWidth;
    window.addEventListener("pointermove", onNavigationResizeMove);
    window.addEventListener("pointerup", stopNavigationResize);
    window.addEventListener("pointercancel", stopNavigationResize);
  }

  /** @param {KeyboardEvent} event */
  function adjustNavigationWidth(event) {
    if (!shouldShowNavigation || !navigationResizable) return;
    const step = event.shiftKey ? 32 : 16;
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      navigationWidth = clampNavigationWidth(navigationWidth - step);
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      navigationWidth = clampNavigationWidth(navigationWidth + step);
    } else if (event.key === "Home") {
      event.preventDefault();
      navigationWidth = navigationMinWidth;
    } else if (event.key === "End") {
      event.preventDefault();
      navigationWidth = navigationMaxWidth;
    }
  }

  onDestroy(() => {
    stopNavigationResize();
  });
</script>

<div
  class={`file-viewer-shell ${variantClass}`}
  class:with-navigation={shouldShowNavigation}
  class:navigation-collapsed={isNavigationCollapsed}
  style={`--file-viewer-navigation-width: ${navigationWidth}px;`}
>
  {#if shouldShowNavigation}
    <div class="file-viewer-navigation" style={`--file-viewer-navigation-width: ${navigationWidth}px;`}>
      {@render navigation()}
    </div>
    {#if navigationResizable}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions: ARIA separator is keyboard-adjustable and pointer-draggable. -->
      <div
        class="file-viewer-navigation-resize"
        class:file-viewer-navigation-resize--active={resizingNavigation}
        role="separator"
        tabindex="0"
        aria-label={navigationResizeLabel}
        aria-orientation="vertical"
        aria-valuemin={navigationMinWidth}
        aria-valuemax={navigationMaxWidth}
        aria-valuenow={Math.round(navigationWidth)}
        onpointerdown={startNavigationResize}
        onkeydown={adjustNavigationWidth}
      ></div>
    {/if}
  {/if}
  {#if canToggleNavigation}
    <div class="file-viewer-navigation-boundary-control">
      <button
        type="button"
        class="file-viewer-navigation-boundary-toggle"
        class:file-viewer-navigation-boundary-toggle--collapsed={isNavigationCollapsed}
        aria-label={isNavigationCollapsed ? navigationExpandLabel : navigationCollapseLabel}
        onclick={() => (navigationCollapsed = !navigationCollapsed)}
      >
        <PanelLeft size={16} strokeWidth={1.8} />
      </button>
    </div>
  {/if}

  <div class="file-viewer-main">
    <div class="file-viewer-titlebar">
      <div class="file-viewer-identity">
        {#if shouldShowBack}
          <button
            type="button"
            class="file-viewer-back-btn"
            aria-label={backLabel}
            onclick={() => onBack()}
            disabled={backDisabled}
          >
            <ArrowLeft size={16} strokeWidth={1.8} />
          </button>
        {/if}
        {#if leading}
          <div class="file-viewer-leading">
            {@render leading()}
          </div>
        {/if}
        {#if icon}
          <div class="file-viewer-icon">
            {@render icon()}
          </div>
        {/if}
        <div class="file-viewer-title-text">
          <div class="file-viewer-title-row">
            {#if titleContent}
              {@render titleContent()}
            {:else}
              <span class="file-viewer-title">{title}</span>
            {/if}
            {#if badge}
              <span class="file-viewer-badge">{badge}</span>
            {/if}
            {#if modeLabel}
              <span class="file-viewer-mode">{modeLabel}</span>
            {/if}
          </div>
          {#if subtitle}
            <div class="file-viewer-subtitle" class:mono={subtitleMonospace}>{displaySubtitle}</div>
          {/if}
        </div>
      </div>

      {#if actions}
        <div class="file-viewer-actions">
          {@render actions()}
        </div>
      {/if}
    </div>

    {#if shouldShowContext}
      <div class="file-viewer-context">
        {@render context()}
      </div>
    {/if}

    <div class="file-viewer-content">
      {@render children?.()}
    </div>
  </div>
</div>

<style>
  .file-viewer-shell {
    flex: 1 1 0;
    min-height: 0;
    min-width: 0;
    display: flex;
    position: relative;
    overflow: hidden;
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    --file-viewer-header-height: 48px;
  }
  .file-viewer-navigation {
    width: var(--file-viewer-navigation-width, 220px);
    flex: 0 0 var(--file-viewer-navigation-width, 220px);
    min-height: 0;
    min-width: 0;
    display: flex;
    background: var(--bg-elevated-subtle);
  }
  .file-viewer-navigation-resize {
    width: 12px;
    flex: 0 0 12px;
    min-height: 0;
    position: relative;
    cursor: col-resize;
    z-index: 3;
    margin-left: -6px;
    margin-right: -6px;
    background: transparent;
    border: none;
    outline: none;
    -webkit-app-region: no-drag;
  }
  .file-viewer-navigation-boundary-toggle {
    width: var(--header-ghost-control-size, 32px);
    height: var(--header-ghost-control-size, 32px);
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
    opacity: 0.42;
    box-shadow: none;
    -webkit-app-region: no-drag;
    transition: background 0.15s ease, color 0.15s ease, opacity 0.15s ease;
  }
  .file-viewer-navigation-boundary-control {
    position: absolute;
    top: 8px;
    left: calc(var(--file-viewer-navigation-width, 220px) - 40px);
    z-index: 5;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .file-viewer-navigation-boundary-toggle:hover {
    background: var(--bg-hover);
    color: var(--color-text-main);
    opacity: 1;
  }
  .file-viewer-navigation-boundary-toggle:focus-visible {
    outline: 1px solid var(--toolbar-control-border-active);
    outline-offset: 1px;
    opacity: 1;
  }
  .navigation-collapsed .file-viewer-navigation-boundary-control {
    left: 8px;
  }
  .file-viewer-main {
    flex: 1 1 0;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .file-viewer-titlebar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    height: var(--file-viewer-header-height);
    min-height: var(--file-viewer-header-height);
    padding: 6px 16px;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-panel);
    box-sizing: border-box;
  }
  .navigation-collapsed .file-viewer-titlebar {
    padding-left: 52px;
  }
  .file-viewer-identity {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .file-viewer-leading {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .file-viewer-back-btn {
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
  .file-viewer-back-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--color-text-main);
  }
  .file-viewer-back-btn:focus-visible {
    outline: 2px solid var(--border-active);
    outline-offset: 2px;
  }
  .file-viewer-back-btn:disabled {
    cursor: default;
    opacity: 0.45;
  }
  .file-viewer-icon {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    background: var(--bg-hover);
    color: var(--color-text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .file-viewer-title-text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .file-viewer-title-row {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .file-viewer-title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    font-weight: 600;
    color: var(--color-text-main);
  }
  .file-viewer-subtitle {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11px;
    color: var(--color-text-muted);
    opacity: 0.82;
  }
  .file-viewer-subtitle.mono {
    font-family: SFMono-Regular, Consolas, monospace;
  }
  .file-viewer-mode,
  .file-viewer-badge {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    min-height: 20px;
    padding: 0 7px;
    border-radius: 999px;
    font-size: 10px;
    font-weight: 600;
    color: var(--color-text-muted);
    background: var(--bg-subtle);
  }
  .file-viewer-mode {
    color: #b45309;
    background: rgba(245, 158, 11, 0.12);
  }
  .file-viewer-actions {
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    flex-shrink: 0;
  }
  .file-viewer-actions :global(.file-viewer-btn),
  .file-viewer-actions :global(.file-viewer-primary-btn),
  .file-viewer-actions :global(.file-viewer-icon-btn) {
    min-height: 30px;
  }
  .file-viewer-context {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 42px;
    padding: 8px 18px;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-card);
    box-sizing: border-box;
  }
  .file-viewer-content {
    flex: 1 1 0;
    min-height: 0;
    min-width: 0;
    display: flex;
    overflow: hidden;
  }
</style>
