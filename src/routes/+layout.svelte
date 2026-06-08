<script>
  import "../app.css";
  import { onMount } from "svelte";
  import { applyThemePreference, watchSystemThemePreference } from "$lib/app/themePreference.js";
  import { theme } from "$lib/features/tools/index.js";
  let { children } = $props();

  $effect(() => {
    applyThemePreference($theme);
  });

  onMount(() => watchSystemThemePreference(theme));
</script>

{@render children()}
