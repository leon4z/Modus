<script>
  // Purpose: File-workspace adapter for the shared module search result popover.
  import { t } from "$lib/shared/i18n/index.js";
  import ModuleSearchResults from "$lib/shared/components/ModuleSearchResults.svelte";
  import { FILE_WORKSPACE_SEARCH_GROUPS, countFileWorkspaceSearchResults } from "$lib/shared/utils/fileWorkspaceSearch.js";

  let {
    query = "",
    groups = [],
    visible = false,
    ariaLabel = "",
    emptyLabel = "",
    dismissRoot = null,
    onActivate = (/** @type {any} */ _result) => {},
    onDismiss = () => {},
  } = $props();

  let resultCount = $derived(countFileWorkspaceSearchResults(groups));

  /** @param {string} key */
  function groupTitle(key) {
    switch (key) {
      case FILE_WORKSPACE_SEARCH_GROUPS.FILE_PATH:
        return $t("file_workspace_search.group_file_path");
      case FILE_WORKSPACE_SEARCH_GROUPS.CURRENT_CONTENT:
        return $t("file_workspace_search.group_current_content");
      case FILE_WORKSPACE_SEARCH_GROUPS.OTHER_CONTENT:
        return $t("file_workspace_search.group_other_content");
      default:
        return key;
    }
  }

  /** @param {any} result */
  function resultMeta(result) {
    if (result.group === FILE_WORKSPACE_SEARCH_GROUPS.FILE_PATH) {
      return result.file?.path || "";
    }
    return $t("file_workspace_search.line_match", { line: result.line || 1 });
  }

  /** @param {any} result */
  function resultDetail(result) {
    if (result.group === FILE_WORKSPACE_SEARCH_GROUPS.FILE_PATH) return result.file?.path || "";
    return result.excerpt || result.file?.path || "";
  }
</script>

<ModuleSearchResults
  {query}
  {groups}
  {visible}
  ariaLabel={ariaLabel || $t("file_workspace_search.results_label")}
  summaryLabel={$t("file_workspace_search.summary", { count: resultCount })}
  emptyLabel={emptyLabel || $t("file_workspace_search.empty_current_tree")}
  {groupTitle}
  resultTitle={(/** @type {any} */ result) => result.file?.label || result.file?.path || ""}
  resultDetail={resultDetail}
  resultMeta={resultMeta}
  {dismissRoot}
  onActivate={onActivate}
  onDismiss={onDismiss}
/>
