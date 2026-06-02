<script>
  // Purpose: Shared file navigation used beside file-like viewer/editor surfaces.
  import { AlertTriangle, FileCode, FileText, Folder, FolderOpen } from "lucide-svelte";
  let {
    title = "Files",
    items = [],
    onToggle = () => {},
    onSelect = () => {},
    onContextMenu = null,
    suppressBlankContextMenu = false,
  } = $props();

  const CODE_FILE_EXTENSIONS = new Set([
    "bash",
    "c",
    "cjs",
    "cpp",
    "css",
    "go",
    "h",
    "hpp",
    "html",
    "java",
    "js",
    "json",
    "jsx",
    "mjs",
    "php",
    "py",
    "rb",
    "rs",
    "scss",
    "sh",
    "sql",
    "svelte",
    "toml",
    "ts",
    "tsx",
    "xml",
    "yaml",
    "yml",
    "zsh",
  ]);

  /** @param {any} item */
  function rowPadding(item) {
    return `${14 + Math.max(0, Number(item?.depth || 0)) * 18}px`;
  }

  /** @param {any} item */
  function itemTitle(item) {
    return item?.title || item?.label || item?.path || "";
  }

  /** @param {any} item */
  function isCodeFile(item) {
    const name = String(item?.label || item?.path || item?.title || "").toLowerCase();
    const extension = name.includes(".") ? name.split(".").pop() : "";
    return CODE_FILE_EXTENSIONS.has(extension || "");
  }

  /** @param {MouseEvent} event @param {any} item */
  function handleContextMenu(event, item) {
    if (typeof onContextMenu !== "function") return;
    const handled = onContextMenu(item, event) === true;
    if (!handled) return;
    event.preventDefault();
    event.stopPropagation();
  }

  /** @param {MouseEvent} event */
  function handleBlankContextMenu(event) {
    if (event.target instanceof Element && event.target.closest(".file-workspace-nav__row")) return;
    if (!suppressBlankContextMenu) return;
    event.preventDefault();
  }
</script>

<div class="file-workspace-nav">
  <div class="file-workspace-nav__header">{title}</div>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="file-workspace-nav__list" oncontextmenu={handleBlankContextMenu}>
    {#each items as item (item.id)}
      {#if item.kind === "directory" || item.kind === "root"}
        <button
          type="button"
          class="file-workspace-nav__row file-workspace-nav__row--directory"
          class:file-workspace-nav__row--root={item.kind === "root"}
          style={`--file-workspace-row-padding: ${rowPadding(item)}`}
          aria-label={item.ariaLabel || itemTitle(item)}
          aria-expanded={item.expandable ? Boolean(item.expanded) : undefined}
          onclick={() => onToggle(item)}
          oncontextmenu={(event) => handleContextMenu(event, item)}
        >
          <span class="file-workspace-nav__folder-icon">
            {#if item.expandable && !item.expanded}
              <Folder size={15} strokeWidth={1.8} />
            {:else}
              <FolderOpen size={15} strokeWidth={1.8} />
            {/if}
          </span>
          <span class="file-workspace-nav__main">
            <span class="file-workspace-nav__label">{item.label}</span>
          </span>
          {#if item.meta}
            <span class="file-workspace-nav__meta">{item.meta}</span>
          {/if}
        </button>
      {:else}
        <button
          type="button"
          class="file-workspace-nav__row file-workspace-nav__row--file"
          class:file-workspace-nav__row--selected={item.selected}
          class:file-workspace-nav__row--abnormal={item.abnormal}
          style={`--file-workspace-row-padding: ${rowPadding(item)}`}
          aria-label={item.ariaLabel || itemTitle(item)}
          aria-current={item.selected ? "page" : undefined}
          disabled={item.disabled}
          onclick={() => onSelect(item)}
          oncontextmenu={(event) => handleContextMenu(event, item)}
        >
          <span class="file-workspace-nav__file-icon">
            {#if item.abnormal}
              <AlertTriangle size={15} strokeWidth={1.8} />
            {:else if isCodeFile(item)}
              <FileCode size={15} strokeWidth={1.8} />
            {:else}
              <FileText size={15} strokeWidth={1.8} />
            {/if}
          </span>
          <span class="file-workspace-nav__main">
            <span class="file-workspace-nav__label">{item.label}</span>
          </span>
          {#if item.meta}
            <span class="file-workspace-nav__meta">{item.meta}</span>
          {/if}
        </button>
      {/if}
    {/each}
  </div>
</div>

<style>
  .file-workspace-nav {
    width: 100%;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-elevated-subtle);
  }

  .file-workspace-nav__header {
    flex-shrink: 0;
    height: var(--file-viewer-header-height, 48px);
    min-height: var(--file-viewer-header-height, 48px);
    display: flex;
    align-items: center;
    padding: 8px 14px;
    font-size: 11px;
    font-weight: 700;
    color: var(--color-text-muted);
    border-bottom: 1px solid var(--border-color);
    box-sizing: border-box;
  }

  .file-workspace-nav__list {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    padding: 8px;
  }

  .file-workspace-nav__row {
    width: 100%;
    min-height: 38px;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 8px 6px var(--file-workspace-row-padding);
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--color-text-muted);
    text-align: left;
    cursor: pointer;
    box-sizing: border-box;
  }

  .file-workspace-nav__row:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--color-text-main);
  }

  .file-workspace-nav__row:focus-visible {
    outline: 2px solid var(--border-active);
    outline-offset: 2px;
  }

  .file-workspace-nav__row:disabled {
    cursor: default;
    opacity: 0.62;
  }

  .file-workspace-nav__row--selected {
    background: var(--bg-card);
    color: var(--color-text-main);
    box-shadow: var(--shadow-soft);
  }

  .file-workspace-nav__row--abnormal {
    color: #b45309;
  }

  .file-workspace-nav__row--root {
    font-weight: 650;
  }

  .file-workspace-nav__folder-icon,
  .file-workspace-nav__file-icon {
    width: 16px;
    height: 16px;
    flex: 0 0 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .file-workspace-nav__folder-icon,
  .file-workspace-nav__file-icon {
    color: var(--color-text-muted);
  }

  .file-workspace-nav__row:hover .file-workspace-nav__folder-icon,
  .file-workspace-nav__row--selected .file-workspace-nav__file-icon {
    color: var(--color-text-main);
  }

  .file-workspace-nav__main {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .file-workspace-nav__label,
  .file-workspace-nav__meta {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-workspace-nav__label {
    font-size: 12px;
    font-weight: 600;
  }

  .file-workspace-nav__meta {
    flex: 0 0 auto;
    max-width: 72px;
    font-size: 10px;
    color: var(--color-text-muted);
  }
</style>
