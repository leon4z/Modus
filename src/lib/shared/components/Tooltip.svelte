<script>
  // Purpose: Shared app-owned tooltip for compact controls and warning markers.
  let {
    label = "",
    placement = "top",
    maxWidth = "280px",
    disabled = false,
    class: className = "",
    children = undefined,
  } = $props();

  let hasTooltip = $derived(Boolean(label) && !disabled);
</script>

<span
  class={`tooltip-host ${className}`.trim()}
  data-placement={placement}
  style={`--tooltip-max-width: ${maxWidth}`}
>
  {@render children?.()}
  {#if hasTooltip}
    <span class="tooltip-bubble" role="tooltip" aria-hidden="true">
      <span class="tooltip-text">{label}</span>
    </span>
  {/if}
</span>

<style>
  .tooltip-host {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    overflow: visible;
    min-width: 0;
    -webkit-app-region: no-drag;
  }

  .tooltip-host:hover,
  .tooltip-host:focus-within {
    z-index: 100;
  }

  .tooltip-bubble {
    position: absolute;
    width: max-content;
    max-width: min(var(--tooltip-max-width, 280px), 52vw);
    padding: 7px 9px;
    border-radius: 8px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card) 85%, black 15%);
    color: var(--color-text-main);
    font-size: 11px;
    font-weight: 500;
    line-height: 1.4;
    text-align: left;
    white-space: normal;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.28);
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.12s ease, transform 0.12s ease;
    z-index: 80;
  }

  .tooltip-text {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    overflow: hidden;
    overflow-wrap: anywhere;
  }

  .tooltip-bubble::after {
    content: "";
    position: absolute;
    width: 10px;
    height: 10px;
    background: inherit;
    transform: rotate(45deg);
  }

  .tooltip-host[data-placement="top"] .tooltip-bubble {
    left: 50%;
    bottom: calc(100% + 8px);
    transform: translateX(-50%) translateY(4px);
  }

  .tooltip-host[data-placement="top"] .tooltip-bubble::after {
    left: calc(50% - 5px);
    bottom: -5px;
    border-right: 1px solid var(--border-color);
    border-bottom: 1px solid var(--border-color);
  }

  .tooltip-host[data-placement="bottom"] .tooltip-bubble {
    left: 50%;
    top: calc(100% + 8px);
    transform: translateX(-50%) translateY(-4px);
  }

  .tooltip-host[data-placement="bottom"] .tooltip-bubble::after {
    left: calc(50% - 5px);
    top: -5px;
    border-left: 1px solid var(--border-color);
    border-top: 1px solid var(--border-color);
  }

  .tooltip-host[data-placement="bottom-end"] .tooltip-bubble {
    right: 0;
    top: calc(100% + 8px);
    transform: translateY(-4px);
  }

  .tooltip-host[data-placement="bottom-end"] .tooltip-bubble::after {
    right: 11px;
    top: -5px;
    border-left: 1px solid var(--border-color);
    border-top: 1px solid var(--border-color);
  }

  .tooltip-host[data-placement="left"] .tooltip-bubble {
    right: calc(100% + 8px);
    top: 50%;
    transform: translateY(-50%) translateX(4px);
  }

  .tooltip-host[data-placement="left"] .tooltip-bubble::after {
    right: -5px;
    top: calc(50% - 5px);
    border-top: 1px solid var(--border-color);
    border-right: 1px solid var(--border-color);
  }

  .tooltip-host[data-placement="right"] .tooltip-bubble {
    left: calc(100% + 8px);
    top: 50%;
    transform: translateY(-50%) translateX(-4px);
  }

  .tooltip-host[data-placement="right"] .tooltip-bubble::after {
    left: -5px;
    top: calc(50% - 5px);
    border-left: 1px solid var(--border-color);
    border-bottom: 1px solid var(--border-color);
  }

  .tooltip-host:hover .tooltip-bubble,
  .tooltip-host:focus-within .tooltip-bubble {
    opacity: 1;
  }

  .tooltip-host[data-placement="top"]:hover .tooltip-bubble,
  .tooltip-host[data-placement="top"]:focus-within .tooltip-bubble {
    transform: translateX(-50%) translateY(0);
  }

  .tooltip-host[data-placement="bottom"]:hover .tooltip-bubble,
  .tooltip-host[data-placement="bottom"]:focus-within .tooltip-bubble {
    transform: translateX(-50%) translateY(0);
  }

  .tooltip-host[data-placement="bottom-end"]:hover .tooltip-bubble,
  .tooltip-host[data-placement="bottom-end"]:focus-within .tooltip-bubble {
    transform: translateY(0);
  }

  .tooltip-host[data-placement="left"]:hover .tooltip-bubble,
  .tooltip-host[data-placement="left"]:focus-within .tooltip-bubble {
    transform: translateY(-50%) translateX(0);
  }

  .tooltip-host[data-placement="right"]:hover .tooltip-bubble,
  .tooltip-host[data-placement="right"]:focus-within .tooltip-bubble {
    transform: translateY(-50%) translateX(0);
  }
</style>
