<script>
  // Purpose: Shared compact editor for inline tool configuration values.
  let {
    value = $bindable(""),
    saving = false,
    ariaLabel = "",
    saveLabel = "Save",
    savingLabel = "Saving...",
    cancelLabel = "Cancel",
    onSave = (/** @type {string} */ _value) => {},
    onCancel = () => {},
  } = $props();

  let textValue = $derived(String(value ?? ""));
</script>

<div class="compact-text-editor">
  <textarea
    class="compact-textarea edit-textarea"
    bind:value
    aria-label={ariaLabel}
    spellcheck="false"
  ></textarea>
  <div class="compact-actions edit-actions">
    <button class="btn btn-primary" onclick={() => onSave(textValue)} disabled={saving}>{saving ? savingLabel : saveLabel}</button>
    <button class="btn btn-secondary" onclick={() => onCancel()}>{cancelLabel}</button>
  </div>
</div>

<style>
  .compact-text-editor { margin-top: 4px; }
  .compact-textarea {
    width: 100%;
    min-height: 80px;
    background: var(--bg-app);
    color: var(--color-text-main);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 8px;
    font-family: monospace;
    font-size: 11px;
    line-height: 1.5;
    resize: vertical;
    outline: none;
    box-sizing: border-box;
  }
  .compact-actions {
    display: flex;
    gap: 8px;
    margin-top: 6px;
  }
  .btn {
    font-size: 10px;
    padding: 4px 12px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
  .btn-primary {
    background: var(--color-text-main);
    color: var(--bg-card);
    border: none;
    font-weight: 600;
  }
  .btn-primary:hover:not(:disabled) { opacity: 0.85; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-secondary { background: var(--bg-subtle); color: var(--color-text-muted); }
  .btn-secondary:hover:not(:disabled) { background: var(--bg-hover); color: var(--color-text-main); }
</style>
