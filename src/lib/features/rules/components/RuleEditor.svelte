<script>
  import { writeRule } from "$lib/features/rules/api/rules.js";
  import ContentEditor from "$lib/shared/components/ContentEditor.svelte";
  import FileViewerShell from "$lib/shared/components/FileViewerShell.svelte";
  import SourceReadModeToggle from "$lib/shared/components/SourceReadModeToggle.svelte";
  import { t } from "$lib/shared/i18n/index.js";
  import { loadTools } from "$lib/features/tools/index.js";
  import { overlayClose } from "$lib/shared/utils/overlayClose.js";
  import { X, Pencil } from "lucide-svelte";
  import { logAppEvent } from "$lib/shared/logging/appLogger.js";

  let { rule = null, toolId, onClose } = $props();

  let content = $state("");
  let saving = $state(false);
  let error = $state("");
  let isEditing = $state(false);

  $effect(() => { 
    if (rule) content = rule.content || ""; 
  });

  let isMarkdown = $derived(rule?.path?.toLowerCase().endsWith(".md"));
  let mdPreview = $state(true);
  let isDirty = $derived(String(content ?? "") !== String(rule?.content ?? ""));
  let lineCount = $derived(String(content ?? "").split("\n").length);

  async function save() {
    saving = true; error = "";
    try { 
      await writeRule(toolId, rule.path, content); 
      await logAppEvent({
        level: "info",
        category: "rules",
        action: "tool_rule_save",
        result: "ok",
        toolId,
        targetRole: "tool_rule",
        targetPath: rule.path,
      });
      rule.content = content;
      await loadTools(); 
      isEditing = false;
    }
    catch (_e) {
      const e = /** @type {any} */ (_e);
      error = e.toString();
      await logAppEvent({
        level: "error",
        category: "rules",
        action: "tool_rule_save",
        result: "failed",
        toolId,
        targetRole: "tool_rule",
        targetPath: rule?.path,
        error: e.toString(),
      });
    }
    finally { saving = false; }
  }

  function cancelEdit() {
    isEditing = false;
    content = rule.content || "";
    error = "";
  }

  /** @param {KeyboardEvent} e */
  function handleKeydown(e) {
    if (!rule) return;
    // Esc to close (when not editing) or cancel (when editing)
    if (e.key === "Escape") {
      if (isEditing) cancelEdit();
      else onClose();
    }
    // Cmd+S to save (when editing)
    if ((e.metaKey || e.ctrlKey) && e.key === "s" && isEditing) {
      e.preventDefault();
      if (content !== rule.content) save();
    }
  }

  /** @param {MouseEvent} event */
  function handleOverlayClose(event) {
    overlayClose(onClose)(event);
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if rule}
  <div class="overlay" data-tauri-drag-region role="presentation" onclick={handleOverlayClose}>
    <div class="editor-modal" role="presentation" onclick={(e) => e.stopPropagation()}>
      <!-- Error -->
      {#if error}<div class="error">{error}</div>{/if}

      <div class="editor-body-container">
        <FileViewerShell title={rule.label} subtitle={rule.path} modeLabel={isEditing ? $t("rules.editor.editing") : ""}>
          {#snippet actions()}
            {#if isEditing}
              <span class="file-viewer-inline-status">
                {#if isDirty}{$t("rules.editor.unsaved")} · {/if}{$t("rules.editor.lines", { count: lineCount })}
              </span>
              <button type="button" class="file-viewer-btn" onclick={cancelEdit} disabled={saving}>
                {$t("rules.editor.cancel")}
              </button>
              <button type="button" class="file-viewer-primary-btn" onclick={save} disabled={saving || !isDirty}>
                {saving ? $t("rules.editor.saving") : $t("rules.editor.save")}
              </button>
            {:else}
              {#if isMarkdown}
                <SourceReadModeToggle
                  bind:preview={mdPreview}
                  editing={isEditing}
                  sourceLabel={$t("rules.editor.mode_src")}
                  readLabel={$t("rules.editor.mode_read")}
                />
              {/if}
              <button type="button" class="file-viewer-btn" onclick={() => (isEditing = true)}>
                <Pencil size={14} /> {$t("rules.editor.edit")}
              </button>
            {/if}
            <button type="button" class="file-viewer-icon-btn" aria-label={$t("rules.editor.close")} onclick={onClose}>
              <X size={18} />
            </button>
          {/snippet}

          <ContentEditor
            bind:value={content}
            bind:preview={mdPreview}
            originalValue={rule.content || ""}
            editing={isEditing}
            markdown={isMarkdown}
            filename={rule.path || ""}
            fill={true}
            minHeight="0"
            showModeToggle={false}
            showActions={false}
            showFooter={false}
            framed={false}
            sourceLabel={$t("rules.editor.mode_src")}
            readLabel={$t("rules.editor.mode_read")}
            editLabel={$t("rules.editor.edit")}
            cancelLabel={$t("rules.editor.cancel")}
            saveLabel={$t("rules.editor.save")}
            savingLabel={$t("rules.editor.saving")}
            unsavedLabel={$t("rules.editor.unsaved")}
            saving={saving}
            formatLineLabel={(/** @type {number} */ count) => $t("rules.editor.lines", { count })}
            onEdit={() => (isEditing = true)}
            onCancel={cancelEdit}
            onSave={save}
          />
        </FileViewerShell>
      </div>
    </div>
  </div>
{/if}

<style>
  .editor-modal { background: var(--bg-modal); border-radius: 16px; width: 85vw; max-width: 1000px; height: 80vh; display: flex; flex-direction: column; border: 1px solid var(--border-modal); box-shadow: var(--shadow-modal); overflow: hidden; padding: 20px; box-sizing: border-box; }
  .error { padding: 8px 20px; font-size: 12px; color: #EF4444; background: rgba(239,68,68,0.1); border-radius: 8px; margin: 0 20px 16px 20px; }
  .editor-body-container { flex: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden; }
  .file-viewer-inline-status {
    font-size: 11px;
    color: var(--color-text-muted);
    white-space: nowrap;
  }
</style>
