<script>
  import "../app.css";
  import { theme } from "$lib/features/tools/index.js";
  let { children } = $props();

  $effect(() => {
    if (typeof document !== 'undefined') {
      if ($theme === 'system') {
        const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        if (isDark) {
          document.documentElement.setAttribute('data-theme', 'dark');
        } else {
          document.documentElement.removeAttribute('data-theme');
        }
        import('@tauri-apps/api/window').then(m => m.getCurrentWindow().setTheme(null)).catch(()=>{});
      } else if ($theme === 'dark') {
        document.documentElement.setAttribute('data-theme', 'dark');
        import('@tauri-apps/api/window').then(m => m.getCurrentWindow().setTheme('dark')).catch(()=>{});
      } else {
        document.documentElement.removeAttribute('data-theme');
        import('@tauri-apps/api/window').then(m => m.getCurrentWindow().setTheme('light')).catch(()=>{});
      }
    }
  });

  // Keep track of systemic preference changes dynamically
  if (typeof window !== 'undefined') {
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', e => {
      let currentTheme;
      theme.subscribe(t => { currentTheme = t; })();
      if (currentTheme === 'system') {
        if (e.matches) {
          document.documentElement.setAttribute('data-theme', 'dark');
          import('@tauri-apps/api/window').then(m => m.getCurrentWindow().setTheme('dark')).catch(()=>{});
        } else {
          document.documentElement.removeAttribute('data-theme');
          import('@tauri-apps/api/window').then(m => m.getCurrentWindow().setTheme('light')).catch(()=>{});
        }
      }
    });

  }
</script>

{@render children()}
