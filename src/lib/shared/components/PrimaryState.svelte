<script>
  // Purpose: Shared primary-page empty/loading/error state used by management surfaces.
  let {
    message = "",
    detail = "",
    tone = "neutral",
    polite = true,
    class: className = "",
  } = $props();
</script>

<div
  class={`empty primary-state ${className}`.trim()}
  data-tone={tone}
  role={polite ? "status" : undefined}
  aria-live={polite ? "polite" : undefined}
>
  <div>{message}</div>
  {#if detail}
    <div class="primary-state-detail">{detail}</div>
  {/if}
</div>

<style>
  .primary-state {
    width: 100%;
    box-sizing: border-box;
  }

  .primary-state[data-tone="error"] {
    color: #b42318;
  }

  .primary-state-detail {
    margin-top: 8px;
    color: var(--color-text-muted);
    overflow-wrap: anywhere;
  }
</style>
