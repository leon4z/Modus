<script>
  import { tick } from "svelte";
  import { X, ChevronRight, ChevronDown, Check } from "lucide-svelte";
  import { t } from "$lib/shared/i18n/index.js";
  import { diffRules } from "$lib/features/rules/index.js";
  import SkillChangeConfirmationPreview from "$lib/features/skills/components/SkillChangeConfirmationPreview.svelte";

  let { layer = "default" } = $props();

  /**
   * Promise-based confirm dialog.
   * Usage:
   *   let confirmDialog;
   *   const ok = await confirmDialog.show({ title, preview });
   *   if (!ok) return;
   */

  let visible = $state(false);
  let title = $state("");
  /** @type {any} */
  let preview = $state(null);   // { creates: [], deletes: [], overwrites: [], preserves: [], message: null, blockedItems?: [], operationGuideTitle?: string, operationGuide?: string[] }
  let variant = $state("default"); // "default" | "danger"
  let confirmLabel = $state("");
  let cancelLabel = $state("");
  /** @type {any} */
  let fileDiffs = $state(null); // { files: [{relativePath, tag}], unchangedCount }

  // Inline diff state
  let expandedFile = $state(null);
  /** @type {any[]} */
  let inlineDiffLines = $state([]);
  let loadingDiff = $state(false);
  let diffRequestId = 0;
  /** @type {HTMLDivElement | null} */
  let dialogEl = $state(null);

  /** @type {((v: boolean) => void) | null} */
  let resolver = null;

  /**
   * @param {{
   *   title: string,
   *   preview?: {
   *     creates?: string[],
   *     deletes?: string[],
   *     overwrites?: string[],
   *     preserves?: string[],
   *     changes?: Array<{ action?: string, changeKind?: string, change_kind?: string, skillName?: string, skill_name?: string, subject?: string, path?: string, entryKind?: string, entry_kind?: string }>,
    *     message?: string | null,
   *     blocked?: Array<{ action?: string, skillName?: string, skill_name?: string, subject?: string, reason?: { message?: string } }>,
   *     blockedItems?: Array<{ skillName: string, toolName: string, reason: string }>,
    *     operationGuideTitle?: string,
    *     operationGuide?: string[],
    *   } | null,
   *   variant?: "default" | "danger",
   *   confirmLabel?: string,
   *   cancelLabel?: string,
   *   fileDiffs?: { files: Array<{relativePath: string, tag: string}>, unchangedCount: number } | null,
   * }} opts
   * @returns {Promise<boolean>}
   */
  export function show(opts) {
    title = opts.title || "";
    preview = opts.preview || null;
    variant = opts.variant || "default";
    confirmLabel = opts.confirmLabel || "";
    cancelLabel = opts.cancelLabel || "";
    fileDiffs = opts.fileDiffs || null;
    expandedFile = null;
    inlineDiffLines = [];
    loadingDiff = false;
    diffRequestId++;
    visible = true;
    return new Promise((resolve) => {
      resolver = resolve;
    });
  }

  $effect(() => {
    if (!visible) return;
    // Keep keyboard focus on the top-most confirm dialog so Escape/Enter are deterministic.
    tick().then(() => {
      dialogEl?.focus();
    });
  });

  $effect(() => {
    if (!visible) return;
    /** @param {KeyboardEvent} e */
    const onKeydownCapture = (e) => {
      handleKeydown(e);
    };
    window.addEventListener("keydown", onKeydownCapture, true);
    return () => {
      window.removeEventListener("keydown", onKeydownCapture, true);
    };
  });

  function handleConfirm() {
    visible = false;
    resolver?.(true);
    resolver = null;
  }

  function handleCancel() {
    visible = false;
    resolver?.(false);
    resolver = null;
  }

  /** @param {KeyboardEvent} e */
  function handleKeydown(e) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      handleCancel();
    }
    if (e.key === "Enter") {
      // Only confirm when focus is NOT on an interactive element other than the confirm button
      const tag = /** @type {HTMLElement | null | undefined} */ (e.target)?.tagName;
      const isInteractive = tag === "BUTTON" || tag === "A" || tag === "INPUT" || tag === "TEXTAREA";
      if (!isInteractive) handleConfirm();
    }
  }

  /** @param {MouseEvent} e */
  function handleOverlay(e) {
    if (e.target === e.currentTarget) handleCancel();
  }

  /** @param {any} file */
  async function toggleFileDiff(file) {
    if (expandedFile === file.relativePath) {
      expandedFile = null;
      inlineDiffLines = [];
      return;
    }
    if (file.tag !== "modified") return;
    expandedFile = file.relativePath;
    inlineDiffLines = [];
    loadingDiff = true;
    const myRequestId = ++diffRequestId;
    try {
      if (fileDiffs?.sourcePath && fileDiffs?.targetPath && file.relativePath) {
        const { readSkillFile } = await import("$lib/features/skills/index.js");
        const [sourceContent, targetContent] = await Promise.all([
          readSkillFile(fileDiffs.sourcePath, file.relativePath).catch(() => ""),
          readSkillFile(fileDiffs.targetPath, file.relativePath).catch(() => ""),
        ]);
        // Stale check: user may have switched to another file while we were loading
        if (myRequestId !== diffRequestId) return;
        if (sourceContent || targetContent) {
          const result = await diffRules(
            sourceContent, $t("confirm.preview.diff_label_source"),
            targetContent, $t("confirm.preview.diff_label_local")
          );
          if (myRequestId !== diffRequestId) return;
          inlineDiffLines = result.changes || [];
        } else {
          inlineDiffLines = [];
        }
      }
    } catch (e) {
      console.error("Failed to load diff:", e);
      if (myRequestId === diffRequestId) inlineDiffLines = [];
    } finally {
      if (myRequestId === diffRequestId) loadingDiff = false;
    }
  }

</script>

{#if visible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="confirm-overlay" class:confirm-overlay--nested={layer === "nested"} onmousedown={handleOverlay}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="confirm-dialog modal"
      class:has-diffs={fileDiffs?.files?.length > 0}
      bind:this={dialogEl}
      tabindex="-1"
      onmousedown={(e) => e.stopPropagation()}
    >
      <div class="modal-header">
        <div class="modal-title">{title}</div>
        <button class="icon-btn" onclick={handleCancel} aria-label={$t("confirm.cancel")}><X size={16} /></button>
      </div>

      {#if preview}
        <div class="modal-body">
          {#if Array.isArray(preview.operationGuide) && preview.operationGuide.length > 0}
            <div class="operation-guide">
              <div class="operation-guide-title">{preview.operationGuideTitle || $t("confirm.preview.operation_guide_title")}</div>
              <ul class="operation-guide-list">
                {#each preview.operationGuide as line}
                  <li>{line}</li>
                {/each}
              </ul>
            </div>
          {/if}
          <div class="preview-block">
            <SkillChangeConfirmationPreview {preview} showEmpty={!fileDiffs?.files?.length} />

            {#if fileDiffs?.files?.length > 0}
              <div class="preview-group">
                <div class="preview-label file-diff-label">{$t("confirm.preview.file_changes")}</div>
                {#each fileDiffs.files as file}
                  <button
                    class="file-diff-row"
                    class:expanded={expandedFile === file.relativePath}
                    onclick={() => toggleFileDiff(file)}
                  >
                    <span class="file-diff-tag {file.tag}">{file.tag === "added" ? "+" : file.tag === "deleted" ? "-" : "~"}</span>
                    <span class="file-diff-name">{file.relativePath}</span>
                    {#if file.tag === "modified"}
                      {#if expandedFile === file.relativePath}
                        <ChevronDown size={12} />
                      {:else}
                        <ChevronRight size={12} />
                      {/if}
                    {/if}
                  </button>
                  {#if expandedFile === file.relativePath && file.tag === "modified"}
                    <div class="inline-diff">
                      {#if loadingDiff}
                        <div class="diff-loading">...</div>
                      {:else if inlineDiffLines.length > 0}
                        {#each inlineDiffLines as line}
                          <div class="diff-line" class:diff-insert={line.tag === "insert"} class:diff-delete={line.tag === "delete"}>{line.content}</div>
                        {/each}
                      {:else}
                        <div class="diff-loading">{$t("confirm.preview.diff_load_failed")}</div>
                      {/if}
                    </div>
                  {/if}
                {/each}
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <div class="modal-footer">
        <button class="btn btn-secondary" onclick={handleCancel}>
          {cancelLabel || $t("confirm.cancel")}
        </button>
        <button
          type="button"
          class="btn"
          class:btn-danger={variant === "danger"}
          class:btn-primary={variant !== "danger"}
          onclick={handleConfirm}
        >
          {#if variant !== "danger"}<Check size={14} />{/if}
          {confirmLabel || $t("confirm.ok")}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .confirm-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    backdrop-filter: var(--overlay-filter);
    -webkit-backdrop-filter: var(--overlay-filter);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
  }
  .confirm-overlay--nested {
    background: var(--overlay-bg-nested);
    backdrop-filter: var(--overlay-filter-nested);
    -webkit-backdrop-filter: var(--overlay-filter-nested);
  }

  .confirm-dialog {
    width: 560px;
    max-width: 90vw;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    animation: confirmIn 0.15s ease-out;
  }
  .confirm-dialog.has-diffs {
    width: 720px;
  }

  .modal-body {
    overflow-y: auto;
  }

  @keyframes confirmIn {
    from { opacity: 0; transform: scale(0.96) translateY(4px); }
    to { opacity: 1; transform: scale(1) translateY(0); }
  }

  .preview-block {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .preview-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .preview-label {
    font-family: inherit;
    font-weight: 600;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 2px;
  }
  .preview-label.file-diff-label { color: var(--color-text-muted); }

  .operation-guide {
    margin-bottom: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-hover);
  }

  .operation-guide-title {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 6px;
  }

  .operation-guide-list {
    margin: 0;
    padding-left: 16px;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    color: var(--color-text-main);
    font-size: 12px;
    line-height: 1.5;
  }

  /* File diff styles */
  .file-diff-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: default;
    background: transparent;
    border: none;
    font-family: inherit;
    font-size: 12px;
    color: var(--color-text-main);
    width: 100%;
    text-align: left;
    transition: 0.1s;
  }
  .file-diff-row:has(.file-diff-tag.modified) {
    cursor: pointer;
  }
  .file-diff-row:has(.file-diff-tag.modified):hover {
    background: var(--bg-hover);
  }

  .file-diff-tag {
    font-weight: 600;
    font-size: 11px;
    width: 14px;
    text-align: center;
    flex-shrink: 0;
  }
  .file-diff-tag.added { color: #4ade80; }
  .file-diff-tag.deleted { color: #f87171; }
  .file-diff-tag.modified { color: #60a5fa; }

  .file-diff-name {
    flex: 1;
    word-break: break-all;
  }

  .inline-diff {
    margin: 2px 0 6px 20px;
    padding: 6px 8px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.2);
    max-height: 200px;
    overflow-y: auto;
    font-size: 11px;
    line-height: 1.5;
  }

  .diff-line {
    white-space: pre-wrap;
    word-break: break-all;
    padding: 0 4px;
    border-radius: 2px;
  }
  .diff-insert {
    color: #4ade80;
    background: rgba(74, 222, 128, 0.08);
  }
  .diff-delete {
    color: #f87171;
    background: rgba(248, 113, 113, 0.08);
  }

  .diff-loading {
    color: var(--color-text-muted);
    font-style: italic;
    padding: 4px;
  }

  @media (max-width: 720px) {
    .preview-skill-item {
      grid-template-columns: auto 1fr;
    }

    .preview-skill-path {
      grid-column: 1 / -1;
    }

    .blocked-item-line {
      gap: 4px 8px;
    }
  }
</style>
