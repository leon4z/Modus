<script>
  import { tick } from "svelte";

  let { onRefresh = async () => {} } = $props();
  let searchFocused = $state(false);
  let searchInput = $state(/** @type {HTMLInputElement | null} */ (null));

  function handleRefresh() {
    void onRefresh();
  }

  export function focusModuleSearch() {
    searchFocused = true;
    tick().then(() => searchInput?.focus());
  }
</script>

<button type="button" onclick={handleRefresh}>Run refresh</button>
{#if searchFocused}
  <input bind:this={searchInput} aria-label="Stub module search" />
{/if}
