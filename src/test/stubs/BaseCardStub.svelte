<script>
  let {
    title = "",
    description = "",
    badges = [],
    warning = null,
    onclick = () => {},
    /** @type {import("svelte").Snippet | undefined} */
    trailing = undefined,
  } = $props();

  /** @param {MouseEvent} event */
  function handleClick(event) {
    onclick(event);
  }
</script>

<div data-testid="base-card-stub">
  <button type="button" onclick={handleClick}>
    <span>{title}</span>
    {#if description}
      <span>{description}</span>
    {/if}
    {#each badges as badge}
      <span>{badge.label}</span>
    {/each}
  </button>
  {#if warning}
    <button type="button" title={warning.title || ""} aria-label={warning.title || ""}>
      warning
    </button>
  {/if}
  {#if trailing}
    <div data-testid="base-card-trailing">{@render trailing()}</div>
  {/if}
</div>
