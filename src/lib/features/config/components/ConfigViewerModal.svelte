<script>
  import { X, Settings2 } from "lucide-svelte";
  import { overlayClose } from "$lib/shared/utils/overlayClose.js";
  import { t } from "$lib/shared/i18n/index.js";
  import Tooltip from "$lib/shared/components/Tooltip.svelte";

  let { config, onClose } = $props();

  /** @param {KeyboardEvent} e */
  function handleKeydown(e) {
    if (e.key === "Escape") onClose();
  }
  const oc = overlayClose(onClose);
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" data-tauri-drag-region onclick={oc}>
  <div class="modal" onclick={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <div class="modal-title-left">
        <div class="header-icon"><Settings2 size={16} /></div>
        <div class="title-text">
          <div class="title-name">{config.key}</div>
            <div class="title-path">{config.source_path}</div>
        </div>
      </div>
      <Tooltip label={$t("config.viewer.close")} placement="bottom">
        <button class="icon-btn" onclick={onClose} aria-label={$t("config.viewer.close")}><X size={16} /></button>
      </Tooltip>
    </div>

    <div class="modal-body">
      <div class="config-value-box">
        <pre class="config-value">{config.value}</pre>
      </div>
    </div>
  </div>
</div>

<style>
  /* Modal size — structure from app.css */
  .modal { width: 600px; max-width: 90vw; max-height: 80vh; }

	  .modal-title-left {
	    display: flex;
	    align-items: center;
	    gap: 12px;
	    flex: 1;
	    min-width: 0;
	  }

  .header-icon {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: rgba(245, 158, 11, 0.15);
    color: #f59e0b;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

	  .title-text {
	    display: flex;
	    flex-direction: column;
	    gap: 4px;
	    min-width: 0;
	  }
	  .title-name {
	    font-size: 14px;
	    font-weight: 600;
	    color: var(--color-text-main);
	    min-width: 0;
	    overflow-wrap: anywhere;
	  }
	  .title-path {
	    font-size: 11px;
	    color: var(--color-text-muted);
	    opacity: 0.7;
	    font-family: "SF Mono", monospace;
	    line-height: 1.35;
	    min-width: 0;
	    overflow-wrap: anywhere;
	  }

  .config-value-box {
    background: var(--bg-input);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 16px;
    overflow-x: auto;
  }
  
  .config-value {
    margin: 0;
    font-family: 'SF Mono', monospace;
    font-size: 13px;
    line-height: 1.6;
    color: var(--color-text-main);
    white-space: pre-wrap;
    word-break: break-all;
  }
</style>
