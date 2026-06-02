<script>
  // Purpose: Shared grouped result popover for page-header module search.
  import { t } from "$lib/shared/i18n/index.js";
  import {
    countModuleSearchResults,
    MODULE_SEARCH_GROUPS,
    moduleSearchResultDetail,
    moduleSearchResultMeta,
    moduleSearchResultTitle,
  } from "$lib/shared/utils/moduleSearch.js";

  let {
    query = "",
    groups = [],
    visible = false,
    ariaLabel = "",
    emptyLabel = "",
    summaryLabel = "",
    onActivate = (/** @type {any} */ _result) => {},
    onDismiss = () => {},
    groupTitle = defaultGroupTitle,
    resultTitle = moduleSearchResultTitle,
    resultDetail = moduleSearchResultDetail,
    resultMeta = defaultResultMeta,
    dismissRoot = null,
  } = $props();

  /** @type {HTMLDivElement | undefined} */
  let root = $state(/** @type {HTMLDivElement | undefined} */ (undefined));
  let activeIndex = $state(0);
  let resultCount = $derived(countModuleSearchResults(groups));
  let flatResults = $derived(
    (Array.isArray(groups) ? groups : []).flatMap((/** @type {any} */ group) => group?.results || [])
  );
  let displayAriaLabel = $derived(ariaLabel || $t("module_search.results_label"));
  let displaySummary = $derived(summaryLabel || $t("module_search.summary", { count: resultCount }));
  let displayEmptyLabel = $derived(emptyLabel || $t("module_search.empty_generic"));

  /** @param {string} key */
  function defaultGroupTitle(key) {
    switch (key) {
      case MODULE_SEARCH_GROUPS.LIST_ITEM:
        return $t("module_search.group_list_item");
      case MODULE_SEARCH_GROUPS.FILE_PATH:
        return $t("file_workspace_search.group_file_path");
      case MODULE_SEARCH_GROUPS.CURRENT_CONTENT:
        return $t("file_workspace_search.group_current_content");
      case MODULE_SEARCH_GROUPS.OTHER_CONTENT:
        return $t("file_workspace_search.group_other_content");
      default:
        return key;
    }
  }

  /** @param {any} result */
  function defaultResultMeta(result) {
    if (result?.line) return $t("file_workspace_search.line_match", { line: result.line });
    return moduleSearchResultMeta(result);
  }

  /** @param {number} index */
  function focusResult(index) {
    if (!flatResults.length) return;
    const nextIndex = (index + flatResults.length) % flatResults.length;
    activeIndex = nextIndex;
    const buttons = /** @type {NodeListOf<HTMLButtonElement> | undefined} */ (
      root?.querySelectorAll("button.module-search-results__item")
    );
    buttons?.[nextIndex]?.focus();
  }

  /** @param {KeyboardEvent} event */
  function handleKeydown(event) {
    if (event.key === "Escape") {
      event.preventDefault();
      onDismiss();
      return;
    }
    if (!flatResults.length) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      focusResult(activeIndex + 1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      focusResult(activeIndex - 1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      onActivate(flatResults[Math.min(activeIndex, flatResults.length - 1)]);
    }
  }

  /** @param {PointerEvent} event */
  function handleWindowPointerDown(event) {
    if (!visible) return;
    const target = event.target;
    if (!(target instanceof Node)) return;
    if (root?.contains(target)) return;
    if (dismissRoot?.contains?.(target)) return;
    onDismiss();
  }

  $effect(() => {
    if (!visible) {
      activeIndex = 0;
      return;
    }
    if (activeIndex >= flatResults.length) activeIndex = 0;
  });
</script>

<svelte:window onpointerdown={handleWindowPointerDown} />

{#if visible && query.trim()}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions: the popover handles arrow and Escape keys for its result buttons. -->
  <div
    bind:this={root}
    class="module-search-results file-workspace-search-results"
    role="region"
    aria-label={displayAriaLabel}
    tabindex="-1"
    onkeydown={handleKeydown}
  >
    <div class="module-search-results__summary file-workspace-search-results__summary">
      {#if resultCount > 0}
        {displaySummary}
      {:else}
        {displayEmptyLabel}
      {/if}
    </div>
    {#if resultCount > 0}
      {#each groups as group (group.key)}
        {#if group.results?.length}
          <section class="module-search-results__group file-workspace-search-results__group" aria-label={groupTitle(group.key)}>
            <div class="module-search-results__group-title file-workspace-search-results__group-title">
              <span>{groupTitle(group.key)}</span>
              <span>{$t("file_workspace_search.group_count", { count: group.results.length })}</span>
            </div>
            <div class="module-search-results__items file-workspace-search-results__items">
              {#each group.results as result (result.id)}
                <button
                  type="button"
                  class="module-search-results__item file-workspace-search-results__item"
                  class:module-search-results__item--active={result === flatResults[activeIndex]}
                  onmouseenter={() => (activeIndex = flatResults.indexOf(result))}
                  onclick={() => onActivate(result)}
                >
                  <span class="module-search-results__item-main file-workspace-search-results__item-main">
                    <span class="module-search-results__item-title file-workspace-search-results__item-title">{resultTitle(result)}</span>
                    <span class="module-search-results__item-detail file-workspace-search-results__item-detail">{resultDetail(result)}</span>
                  </span>
                  {#if resultMeta(result)}
                    <span class="module-search-results__item-meta file-workspace-search-results__item-meta">{resultMeta(result)}</span>
                  {/if}
                </button>
              {/each}
            </div>
          </section>
        {/if}
      {/each}
    {/if}
  </div>
{/if}

<style>
  .module-search-results {
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    width: min(380px, calc(100vw - 48px));
    max-height: min(360px, calc(100vh - 140px));
    overflow: auto;
    z-index: 3200;
    padding: 8px;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-card);
    box-shadow: var(--shadow-soft);
    backdrop-filter: blur(14px);
    pointer-events: auto;
    -webkit-app-region: no-drag;
  }

  .module-search-results__summary {
    padding: 6px 8px 8px;
    color: var(--color-text-muted);
    font-size: 11px;
    font-weight: 600;
  }

  .module-search-results__group + .module-search-results__group {
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--border-color);
  }

  .module-search-results__group-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 0 8px 5px;
    color: var(--color-text-main);
    font-size: 11px;
    font-weight: 700;
  }

  .module-search-results__items {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .module-search-results__item {
    width: 100%;
    min-width: 0;
    min-height: 36px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 7px 8px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--color-text-main);
    text-align: left;
    cursor: pointer;
  }

  .module-search-results__item:hover,
  .module-search-results__item:focus-visible,
  .module-search-results__item--active {
    background: var(--bg-hover);
    border-color: var(--border-color);
    outline: none;
  }

  .module-search-results__item-main {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .module-search-results__item-title,
  .module-search-results__item-detail,
  .module-search-results__item-meta {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .module-search-results__item-title {
    font-size: 12px;
    font-weight: 650;
  }

  .module-search-results__item-detail {
    max-width: 300px;
    color: var(--color-text-muted);
    font-size: 11px;
  }

  .module-search-results__item-meta {
    flex: 0 0 auto;
    max-width: 92px;
    color: var(--color-text-muted);
    font-size: 11px;
  }
</style>
