<script>
  // Purpose: Shared source/read content surface for rule and Skill file editing.
  import { tick } from "svelte";
  import { renderSafeMarkdown } from "$lib/shared/utils/markdown.js";
  import { Pencil, Save } from "lucide-svelte";
  import SourceCodeEditor from "$lib/shared/components/SourceCodeEditor.svelte";
  import SourceReadModeToggle from "$lib/shared/components/SourceReadModeToggle.svelte";

  const extLangMap = /** @type {Record<string, string>} */ ({
    md: "markdown",
    js: "javascript",
    mjs: "javascript",
    ts: "typescript",
    py: "python",
    sh: "bash",
    zsh: "bash",
    bash: "bash",
    json: "json",
    jsonc: "jsonc",
    yaml: "yaml",
    yml: "yaml",
    html: "xml",
    xml: "xml",
    svg: "xml",
    css: "css",
    rs: "rust",
    toml: "toml",
  });

  /** @param {string} filename */
  function getLang(filename) {
    const ext = filename.split(".").pop()?.toLowerCase() || "";
    return extLangMap[ext] || null;
  }

  /** @param {string} text */
  function sourceLineParts(text) {
    return text ? text.split("\n") : [];
  }

  let {
    value = $bindable(""),
    originalValue = "",
    editing = false,
    markdown: isMarkdown = false,
    preview = $bindable(true),
    filename = "",
    language = null,
    placeholder = "",
    ariaLabel = "",
    sourceLabel = "Source",
    readLabel = "Read",
    editLabel = "Edit",
    cancelLabel = "Cancel",
    saveLabel = "Save",
    savingLabel = "Saving...",
    unsavedLabel = "Unsaved changes",
    saving = false,
    showActions = true,
    showFooter = true,
    showModeToggle = true,
    fill = false,
    framed = true,
    minHeight = "300px",
    saveIcon = "save",
    emptyPreviewLabel = "",
    externalSearchQuery = "",
    externalSearchMatchIndex = 0,
    externalSearchVersion = 0,
    formatPreviewContent = (/** @type {string} */ text) => text,
    formatLineLabel = (/** @type {number} */ count) => `${count} lines`,
    onEdit = () => {},
    onCancel = () => {},
    onSave = (/** @type {string} */ _value) => {},
  } = $props();

  let textValue = $derived(String(value ?? ""));
  let originalTextValue = $derived(String(originalValue ?? ""));
  let sourceLines = $derived(sourceLineParts(textValue));
  let lineCount = $derived(sourceLines.length);
  let isDirty = $derived(textValue !== originalTextValue);
  let previewHtml = $derived(renderSafeMarkdown(formatPreviewContent(textValue)));
  /** @type {HTMLDivElement | undefined} */
  let previewHost = $state(/** @type {HTMLDivElement | undefined} */ (undefined));
  let lang = $derived(language || getLang(filename));
  let saveShortcutEnabled = $derived(editing && isDirty && !saving);

  /** @param {string} value */
  function escapeRegExp(value) {
    return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  /** @param {string} html @param {string} query @param {number} activeIndex */
  function highlightPreviewHtml(html, query, activeIndex) {
    const rawQuery = String(query || "").trim();
    if (!rawQuery || typeof DOMParser === "undefined") return html;
    const parser = new DOMParser();
    const doc = parser.parseFromString(`<div>${html}</div>`, "text/html");
    const root = doc.body.firstElementChild;
    if (!root) return html;
    const pattern = new RegExp(escapeRegExp(rawQuery), "gi");
    const walker = doc.createTreeWalker(root, 4);
    /** @type {Text[]} */
    const textNodes = [];
    let current = walker.nextNode();
    while (current) {
      textNodes.push(/** @type {Text} */ (current));
      current = walker.nextNode();
    }
    let matchIndex = 0;
    for (const node of textNodes) {
      const source = node.nodeValue || "";
      pattern.lastIndex = 0;
      if (!pattern.test(source)) continue;
      pattern.lastIndex = 0;
      const fragment = doc.createDocumentFragment();
      let lastIndex = 0;
      let match = pattern.exec(source);
      while (match) {
        if (match.index > lastIndex) fragment.append(doc.createTextNode(source.slice(lastIndex, match.index)));
        const mark = doc.createElement("mark");
        mark.className = matchIndex === activeIndex
          ? "workspace-search-highlight workspace-search-highlight--active"
          : "workspace-search-highlight";
        mark.textContent = match[0];
        fragment.append(mark);
        lastIndex = match.index + match[0].length;
        matchIndex += 1;
        match = pattern.exec(source);
      }
      if (lastIndex < source.length) fragment.append(doc.createTextNode(source.slice(lastIndex)));
      node.parentNode?.replaceChild(fragment, node);
    }
    return root.innerHTML;
  }

  let displayedPreviewHtml = $derived(
    highlightPreviewHtml(previewHtml, externalSearchQuery, externalSearchMatchIndex)
  );

  $effect(() => {
    if (!previewHost || !externalSearchQuery || !preview || editing) return;
    const key = `${externalSearchQuery}\u0000${externalSearchMatchIndex}\u0000${externalSearchVersion}`;
    tick().then(() => {
      if (!key) return;
      const activeMatch = previewHost?.querySelector(".workspace-search-highlight--active");
      if (activeMatch && typeof activeMatch.scrollIntoView === "function") {
        activeMatch.scrollIntoView({ block: "center" });
      }
    });
  });
</script>

<div
  class="content-editor"
  class:fill
  class:unframed={!framed}
  style={`--content-editor-min-height: ${minHeight};`}
>
  {#if isMarkdown && showModeToggle}
    <div class="content-editor-toolbar">
      <div class="toolbar-spacer"></div>
      <SourceReadModeToggle
        bind:preview
        {editing}
        {sourceLabel}
        {readLabel}
      />
    </div>
  {/if}

  <div class="content-editor-well">
    {#if isMarkdown && preview && !editing}
      {#if textValue.trim()}
        <div class="md-rendered content-scrollable" bind:this={previewHost}>{@html displayedPreviewHtml}</div>
      {:else}
        <div class="content-empty-state content-scrollable">{emptyPreviewLabel}</div>
      {/if}
    {:else}
      <div class="source-view content-source-pre">
        <SourceCodeEditor
          bind:value
          editable={editing}
          language={lang}
          {ariaLabel}
          {placeholder}
          {saveShortcutEnabled}
          {externalSearchQuery}
          {externalSearchMatchIndex}
          {externalSearchVersion}
          onSaveShortcut={onSave}
        />
      </div>
    {/if}
  </div>

  {#if showFooter}
    <div class="content-editor-footer">
      <div class="footer-info">
        <span>{formatLineLabel(lineCount)}</span>
        {#if editing && isDirty}
          <span class="footer-sep">·</span>
          <span class="unsaved">{unsavedLabel}</span>
        {/if}
      </div>
      {#if showActions}
        <div class="footer-actions">
          {#if editing}
            <button type="button" class="btn-text" onclick={() => onCancel()}>{cancelLabel}</button>
            <button type="button" class="primary-pill-btn" onclick={() => onSave(textValue)} disabled={saving || !isDirty}>
              {#if saveIcon === "save"}<Save size={14} />{/if}
              {saving ? savingLabel : saveLabel}
            </button>
          {:else}
            <button type="button" class="secondary-pill-btn" onclick={() => onEdit()}>
              <Pencil size={14} /> {editLabel}
            </button>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .content-editor {
    display: flex;
    flex-direction: column;
    min-height: var(--content-editor-min-height);
    min-width: 0;
  }
  .content-editor.fill {
    flex: 1;
    min-height: 0;
  }
  .content-editor-toolbar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 12px 0;
    flex-shrink: 0;
  }
  .toolbar-spacer { flex: 1; }
  .content-editor-well {
    flex: 1;
    min-height: 0;
    background: var(--bg-well);
    border-radius: 12px;
    display: flex;
    overflow: hidden;
    box-shadow: inset 0 2px 10px rgba(0, 0, 0, 0.03);
    border: 1px solid var(--border-color);
  }
  .content-editor.unframed .content-editor-well {
    background: transparent;
    border: none;
    border-radius: 0;
    box-shadow: none;
  }
  .source-view {
    flex: 1;
    min-width: 0;
    margin: 0;
    padding: 0;
    background: transparent;
    overflow: hidden;
    box-sizing: border-box;
    display: flex;
  }
  .content-scrollable {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .md-rendered {
    font-size: 13px;
    color: var(--color-text-main);
    line-height: 1.7;
    padding: 16px 20px;
  }
  .content-empty-state {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    color: var(--color-text-muted);
    font-size: 12px;
    line-height: 1.5;
    text-align: center;
    box-sizing: border-box;
  }
  .md-rendered :global(:first-child) { margin-top: 0; }
  .md-rendered :global(h1) { font-size: 20px; font-weight: 700; margin: 20px 0 10px; padding-bottom: 6px; border-bottom: 1px solid var(--border-color); }
  .md-rendered :global(h2) { font-size: 16px; font-weight: 600; margin: 18px 0 8px; padding-bottom: 4px; border-bottom: 1px solid var(--border-color); }
  .md-rendered :global(h3) { font-size: 14px; font-weight: 600; margin: 14px 0 6px; }
  .md-rendered :global(p) { margin: 8px 0; }
  .md-rendered :global(ul), .md-rendered :global(ol) { padding-left: 20px; margin: 8px 0; }
  .md-rendered :global(li) { margin: 4px 0; }
  .md-rendered :global(code) { font-family: "SF Mono", "Fira Code", monospace; font-size: 11px; background: var(--bg-subtle); padding: 2px 5px; border-radius: 4px; }
  .md-rendered :global(pre) {
    background: var(--bg-elevated-subtle);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 16px;
    margin: 12px 0;
    max-width: 100%;
    box-sizing: border-box;
    overflow-x: hidden;
    box-shadow: none;
  }
  .md-rendered :global(pre code) { display: block; background: transparent; padding: 0; font-size: 12px; line-height: 1.6; white-space: pre-wrap; overflow-wrap: anywhere; }
  .md-rendered :global(blockquote) { border-left: 3px solid var(--border-active); padding-left: 14px; margin: 10px 0; color: var(--color-text-muted); }
  .md-rendered :global(hr) { border: none; border-top: 1px solid var(--border-color); margin: 16px 0; }
  .md-rendered :global(strong) { font-weight: 600; }
  .md-rendered :global(a) { color: #60a5fa; text-decoration: none; }
  .md-rendered :global(a:hover) { text-decoration: underline; }
  .md-rendered :global(.workspace-search-highlight) {
    border-radius: 3px;
    background: rgba(250, 204, 21, 0.34);
    color: inherit;
    padding: 0 1px;
  }
  .md-rendered :global(.workspace-search-highlight--active) {
    background: rgba(96, 165, 250, 0.38);
    box-shadow: 0 0 0 1px rgba(96, 165, 250, 0.36);
  }
  .md-rendered :global(table) { border-collapse: collapse; margin: 10px 0; width: 100%; }
  .md-rendered :global(th), .md-rendered :global(td) { border: 1px solid var(--border-color); padding: 6px 10px; font-size: 12px; text-align: left; }
  .md-rendered :global(th) { background: var(--bg-subtle); font-weight: 600; }
  .content-editor-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 0 20px 0;
    flex-shrink: 0;
  }
  .footer-info {
    font-size: 13px;
    color: var(--color-text-muted);
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .footer-sep { opacity: 0.5; }
  .unsaved { color: #f59e0b; }
  .footer-actions { display: flex; align-items: center; gap: 12px; }
  .btn-text {
    background: var(--bg-subtle);
    border: none;
    color: var(--color-text-muted);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    min-height: var(--control-height);
    padding: 0 16px;
    border-radius: 8px;
    transition: 0.15s;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
  }
  .btn-text:hover { background: var(--bg-hover); color: var(--color-text-main); }
  .primary-pill-btn {
    background: var(--color-text-main);
    color: var(--bg-card);
    border: none;
    border-radius: 8px;
    min-height: var(--control-height);
    padding: 0 16px;
    font-size: 12px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    cursor: pointer;
    transition: all 0.15s;
    box-sizing: border-box;
  }
  .primary-pill-btn:hover:not(:disabled) { opacity: 0.85; }
  .primary-pill-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .secondary-pill-btn {
    background: var(--bg-subtle);
    color: var(--color-text-muted);
    border: none;
    border-radius: 8px;
    min-height: var(--control-height);
    padding: 0 16px;
    font-size: 12px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
    box-sizing: border-box;
  }
  .secondary-pill-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--color-text-main);
  }
</style>
