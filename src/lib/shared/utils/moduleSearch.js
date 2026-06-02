// Purpose: Shared helpers for page-header module search targets and grouped results.

export const MODULE_SEARCH_GROUPS = Object.freeze({
  LIST_ITEM: "list_item",
  FILE_PATH: "file_path",
  CURRENT_CONTENT: "current_content",
  OTHER_CONTENT: "other_content",
});

const DEFAULT_MAX_RESULTS_PER_GROUP = 30;

/**
 * @typedef {{
 *   id: string,
 *   label?: string,
 *   detail?: string,
 *   meta?: string,
 *   keywords?: Array<unknown>,
 *   raw?: any,
 * }} ModuleSearchListItem
 *
 * @typedef {{
 *   id: string,
 *   group: string,
 *   query: string,
 *   label?: string,
 *   detail?: string,
 *   meta?: string,
 *   item?: ModuleSearchListItem,
 *   file?: any,
 *   raw?: any,
 *   matchIndex?: number,
 *   line?: number,
 *   column?: number,
 *   excerpt?: string,
 * }} ModuleSearchResult
 *
 * @typedef {{ key: string, results: ModuleSearchResult[] }} ModuleSearchGroup
 */

/** @param {unknown} value */
function text(value) {
  return value == null ? "" : String(value);
}

/** @param {unknown} value */
function normalize(value) {
  return text(value).trim().toLocaleLowerCase();
}

/** @param {Array<{ results?: any[] }>} groups */
export function countModuleSearchResults(groups) {
  return (Array.isArray(groups) ? groups : []).reduce((total, group) => total + (group.results || []).length, 0);
}

/**
 * @param {any} item
 */
function itemSearchText(item) {
  return [
    item?.label,
    item?.detail,
    item?.meta,
    item?.path,
    ...(Array.isArray(item?.keywords) ? item.keywords : []),
  ]
    .map(text)
    .filter(Boolean)
    .join("\n")
    .toLocaleLowerCase();
}

/**
 * @param {{
 *   query: unknown,
 *   items: ModuleSearchListItem[],
 *   groupKey?: string,
 *   maxResultsPerGroup?: number,
 * }} options
 * @returns {ModuleSearchGroup[]}
 */
export function buildListModuleSearchGroups(options) {
  const rawQuery = text(options.query).trim();
  const normalizedQuery = normalize(rawQuery);
  const groupKey = options.groupKey || MODULE_SEARCH_GROUPS.LIST_ITEM;
  const maxResultsPerGroup = Math.max(1, options.maxResultsPerGroup || DEFAULT_MAX_RESULTS_PER_GROUP);
  /** @type {ModuleSearchGroup} */
  const group = { key: groupKey, results: [] };
  if (!normalizedQuery) return [group];

  for (const item of Array.isArray(options.items) ? options.items : []) {
    if (group.results.length >= maxResultsPerGroup) break;
    const id = text(item?.id || item?.label || group.results.length);
    if (!id) continue;
    if (!itemSearchText(item).includes(normalizedQuery)) continue;
    const normalizedItem = {
      ...item,
      id,
      label: text(item?.label || id),
      detail: text(item?.detail || ""),
      meta: text(item?.meta || ""),
    };
    group.results.push({
      id: `${groupKey}:${id}`,
      group: groupKey,
      query: rawQuery,
      label: normalizedItem.label,
      detail: normalizedItem.detail,
      meta: normalizedItem.meta,
      item: normalizedItem,
      raw: item?.raw ?? item,
    });
  }

  return [group];
}

/** @param {Array<ModuleSearchGroup[]>} groupSets */
export function mergeModuleSearchGroups(groupSets) {
  /** @type {Map<string, ModuleSearchResult[]>} */
  const merged = new Map();
  for (const groups of groupSets) {
    for (const group of Array.isArray(groups) ? groups : []) {
      const key = group?.key;
      if (!key) continue;
      const existing = merged.get(key) || [];
      merged.set(key, [...existing, ...(group.results || [])]);
    }
  }
  return Array.from(merged.entries()).map(([key, results]) => ({ key, results }));
}

/** @param {any} result */
export function moduleSearchResultTitle(result) {
  return text(result?.label || result?.item?.label || result?.file?.label || result?.file?.path);
}

/** @param {any} result */
export function moduleSearchResultDetail(result) {
  return text(result?.detail || result?.excerpt || result?.item?.detail || result?.file?.path);
}

/** @param {any} result */
export function moduleSearchResultMeta(result) {
  return text(result?.meta || result?.item?.meta || "");
}
