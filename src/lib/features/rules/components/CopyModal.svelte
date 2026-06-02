<script>
  import { Check } from "lucide-svelte";
  import { detectedTools, loadTools } from "$lib/features/tools/index.js";
  import { copyRule } from "$lib/features/rules/api/rules.js";
  import { t } from "$lib/shared/i18n/index.js";
  import { overlayClose } from "$lib/shared/utils/overlayClose.js";

  let { rule = null, sourceToolId, onClose } = $props();

  let selectedToolId = $state("");
  let selectedPath = $state("");
  let append = $state(true);
  let copying = $state(false);
  let error = $state("");

  let currentDetected = $derived($detectedTools);
  let targetTool = $derived(currentDetected.find((/** @type {any} */ t) => t.id === selectedToolId));
  let targetPaths = $derived(targetTool?.rule_sources?.map((/** @type {any} */ r) => r.path) || []);

  $effect(() => { if (targetPaths.length > 0 && !selectedPath) selectedPath = targetPaths[0]; });

  async function doCopy() {
    copying = true; error = "";
    try { await copyRule(sourceToolId, selectedToolId, selectedPath, rule.content, append); await loadTools(); onClose(); }
    catch (/** @type {any} */ e) { error = e.toString(); }
    finally { copying = false; }
  }
	  /** @param {MouseEvent} event */
	  function handleOverlayClick(event) {
	    overlayClose(onClose)(event);
	  }
</script>

<svelte:window onkeydown={(e) => { if (rule && e.key === 'Escape') onClose(); }} />

{#if rule}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="overlay"
    data-tauri-drag-region
    role="button"
    tabindex="0"
    aria-label={$t('rules.copy.cancel')}
	    onclick={handleOverlayClick}
  >
    <div class="modal">
      <div class="modal-header">
        <div class="modal-title">{$t('rules.copy.title')}</div>
        <div class="modal-subtitle">{$t('rules.copy.source', { label: rule.label })}</div>
      </div>
      <div class="modal-body">
        <label class="field">
          <span class="field-label">{$t('rules.copy.target_tool')}</span>
          <select bind:value={selectedToolId} onchange={() => { selectedPath = ""; }}>
            <option value="">{$t('rules.copy.select_tool')}</option>
            {#each currentDetected.filter((t) => t.id !== sourceToolId) as tool}
              <option value={tool.id}>{tool.name}</option>
            {/each}
          </select>
        </label>
        {#if targetPaths.length > 0}
          <label class="field">
            <span class="field-label">{$t('rules.copy.target_file')}</span>
            <select bind:value={selectedPath}>
              {#each targetPaths as path}<option value={path}>{path}</option>{/each}
            </select>
          </label>
          <label class="checkbox"><input type="checkbox" bind:checked={append} /><span>{$t('rules.copy.append')}</span></label>
        {/if}
        {#if error}<div class="error">{error}</div>{/if}
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" onclick={onClose}>{$t('rules.copy.cancel')}</button>
          <button type="button" class="btn btn-primary" onclick={doCopy} disabled={!selectedToolId || !selectedPath || copying}><Check size={14} /> {copying ? $t('rules.copy.copying') : $t('rules.copy.confirm')}</button>
        </div>
      </div>
  </div>
{/if}

<style>
  /* Modal size — structure from app.css */
  .modal { width: 480px; max-width: 90vw; }
  .modal-subtitle { font-size: 11px; color: var(--color-text-muted); margin-top: 4px; }
  .field { display: block; margin-bottom: 12px; }
  .field-label { display: block; font-size: 11px; color: var(--color-text-muted); margin-bottom: 4px; }
  .field select { width: 100%; padding: 8px 10px; background: var(--bg-app); border: 1px solid var(--border-color); border-radius: 8px; color: var(--color-text-main); font-size: 12px; }
  .checkbox { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--color-text-muted); }
  .error { padding: 8px; font-size: 12px; color: #f87171; background: rgba(248,113,113,0.1); border-radius: 8px; }
</style>
