<script>
  import { onMount } from "svelte";
  import { activeView } from "$lib/features/tools/index.js";
  import { appUpdateAvailable } from "$lib/features/appUpdates/index.js";
  import { t } from "$lib/shared/i18n/index.js";
  import { ScrollText, Blocks, Settings2, PlugZap, Settings, LayoutDashboard } from "lucide-svelte";

  let currentView = $derived($activeView);

  const SIDEBAR_W_KEY = "modus.sidebar.width";
  const SIDEBAR_W_MIN = 160;
  const SIDEBAR_W_MAX = 360;
  const SIDEBAR_W_COLLAPSED = 0;
  let { collapsed = $bindable(false), sidebarWidth = $bindable(200) } = $props();
  let isResizing = $state(false);

  onMount(() => {
    try {
      const raw = localStorage.getItem(SIDEBAR_W_KEY);
      if (raw) {
        const n = parseInt(raw, 10);
        if (!Number.isNaN(n)) {
          sidebarWidth = Math.min(SIDEBAR_W_MAX, Math.max(SIDEBAR_W_MIN, n));
        }
      }
    } catch {
      /* ignore */
    }
  });

  /** @param {string} view */
  function setView(view) {
    activeView.set(view);
  }

  /** @param {MouseEvent} e */
  function onSidebarResizeStart(e) {
    if (collapsed) return;
    e.preventDefault();
    isResizing = true;
    const startX = e.clientX;
    const startW = sidebarWidth;
    /** @param {MouseEvent} ev */
    function onMove(ev) {
      sidebarWidth = Math.min(SIDEBAR_W_MAX, Math.max(SIDEBAR_W_MIN, startW + (ev.clientX - startX)));
    }
    function finishResize() {
      isResizing = false;
      try {
        localStorage.setItem(SIDEBAR_W_KEY, String(sidebarWidth));
      } catch {
        /* ignore */
      }
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", finishResize);
      window.removeEventListener("blur", finishResize);
    }
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", finishResize);
    window.addEventListener("blur", finishResize);
  }
</script>

<aside
  class="sidebar"
  class:collapsed
  class:resizing={isResizing}
  data-tauri-drag-region
  style:width={collapsed ? `${SIDEBAR_W_COLLAPSED}px` : `${sidebarWidth}px`}
>
  {#if !collapsed}
    <div class="nav-section">
      <button class="nav-item" class:active={currentView === 'dashboard'} onclick={() => setView('dashboard')} aria-label={$t('sidebar.dashboard')}>
        <LayoutDashboard size={16} strokeWidth={1.8} /> <span class="nav-label">{$t('sidebar.dashboard')}</span>
      </button>
      <button class="nav-item rules" class:active={currentView === 'rules'} onclick={() => setView('rules')} aria-label={$t('sidebar.rules')}>
        <ScrollText size={16} strokeWidth={1.8} /> <span class="nav-label">{$t('sidebar.rules')}</span>
      </button>
      <button class="nav-item skills" class:active={currentView === 'skills'} onclick={() => setView('skills')} aria-label={$t('sidebar.skills')}>
        <Blocks size={16} strokeWidth={1.8} /> <span class="nav-label">{$t('sidebar.skills')}</span>
      </button>
      <button class="nav-item mcp" class:active={currentView === 'mcp'} onclick={() => setView('mcp')} aria-label={$t('sidebar.mcp')}>
        <PlugZap size={16} strokeWidth={1.8} /> <span class="nav-label">{$t('sidebar.mcp')}</span>
      </button>
      <button class="nav-item config" class:active={currentView === 'config'} onclick={() => setView('config')} aria-label={$t('sidebar.config')}>
        <Settings2 size={16} strokeWidth={1.8} /> <span class="nav-label">{$t('sidebar.config')}</span>
      </button>
    </div>

    <div class="nav-bottom">
      <button class="nav-item" class:active={currentView === 'settings'} onclick={() => setView('settings')} aria-label={$t('sidebar.settings')}>
        <Settings size={16} strokeWidth={1.8} /> <span class="nav-label">{$t('sidebar.settings')}</span>
        {#if $appUpdateAvailable}
          <span class="nav-update-tag" aria-label={$t("sidebar.update_available")}>{$t("sidebar.update_tag")}</span>
        {/if}
      </button>
    </div>
    <button
      type="button"
      class="sidebar-resize-handle"
      aria-label={$t("global.a11y.resize_sidebar")}
      onmousedown={onSidebarResizeStart}
    ></button>
  {/if}
</aside>

<style>
  .sidebar {
    position: relative;
    box-sizing: border-box;
    padding: 72px 10px 16px 10px;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    height: 100vh;
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--bg-sidebar);
    transition: width 0.42s cubic-bezier(0.22, 1, 0.36, 1);
    will-change: width;
  }
  .sidebar.collapsed {
    padding: 0;
    overflow: visible;
  }
  .sidebar.resizing {
    transition: none;
  }
  .sidebar-resize-handle {
    position: absolute;
    top: 0;
    right: 0;
    width: 6px;
    height: 100%;
    padding: 0;
    margin: 0;
    border: none;
    border-radius: 0;
    cursor: col-resize;
    background: transparent;
    z-index: 2;
    -webkit-app-region: no-drag;
    outline: none;
  }
  .sidebar.collapsed .sidebar-resize-handle {
    display: none;
  }
  .sidebar.collapsed .nav-section,
  .sidebar.collapsed .nav-bottom {
    display: none;
  }
  
  .nav-section { margin-bottom: 24px; }
  
  .nav-item { display: flex; align-items: center; gap: 10px; padding: 8px 10px; font-size: 13px; font-weight: 500; color: var(--color-text-main); opacity: var(--sidebar-item-opacity); background: transparent; border: 1px solid transparent; border-radius: 9px; cursor: pointer; width: 100%; text-align: left; margin-bottom: 2px; transition: background 0.15s ease, border-color 0.15s ease, opacity 0.15s ease, color 0.15s ease; }
  .nav-item:hover { background: var(--sidebar-item-hover); opacity: var(--sidebar-item-hover-opacity); }
  .nav-item.active { background: var(--sidebar-item-active); border-color: transparent; opacity: var(--sidebar-item-active-opacity); font-weight: 600; }
  .nav-item { -webkit-app-region: no-drag; }
  .nav-label { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nav-update-tag { margin-left: auto; max-width: 56px; padding: 1px 6px; border-radius: 999px; background: rgba(245, 158, 11, 0.12); color: #f59e0b; font-size: 10px; line-height: 1.5; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .nav-bottom { margin-top: auto; padding-top: 14px; }

  @media (prefers-reduced-motion: reduce) {
    .sidebar {
      transition: none;
    }
  }
</style>
