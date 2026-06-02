// Purpose: Shared current-tree search helpers for navigation-backed file workspaces.

export const FILE_WORKSPACE_SEARCH_GROUPS = Object.freeze({
  FILE_PATH: "file_path",
  CURRENT_CONTENT: "current_content",
  OTHER_CONTENT: "other_content",
});

const DEFAULT_MAX_RESULTS_PER_GROUP = 20;
const DEFAULT_MAX_CONTENT_MATCHES_PER_FILE = 5;

/**
 * @typedef {{ id: string, label: string, path: string, content?: string, raw?: any }} FileWorkspaceSearchFile
 * @typedef {{
 *   id: string,
 *   group: string,
 *   query: string,
 *   matchIndex?: number,
 *   line?: number,
 *   column?: number,
 *   excerpt?: string,
 *   file: FileWorkspaceSearchFile,
 * }} FileWorkspaceSearchResult
 * @typedef {{ key: string, results: FileWorkspaceSearchResult[] }} FileWorkspaceSearchGroup
 */

/** @param {unknown} value */
function text(value) {
  return value == null ? "" : String(value);
}

/** @param {string} value */
function normalize(value) {
  return value.trim().toLocaleLowerCase();
}

/** @param {string} source @param {number} index */
function lineInfoAt(source, index) {
  const before = source.slice(0, index);
  const line = before.split("\n").length;
  const lineStart = source.lastIndexOf("\n", Math.max(0, index - 1)) + 1;
  const lineEndIndex = source.indexOf("\n", index);
  const lineEnd = lineEndIndex === -1 ? source.length : lineEndIndex;
  const column = index - lineStart + 1;
  return {
    line,
    column,
    excerpt: source.slice(lineStart, lineEnd).trim(),
  };
}

/**
 * @param {unknown} content
 * @param {unknown} query
 * @param {{ limit?: number }} [options]
 */
export function findFileWorkspaceContentMatches(content, query, options = {}) {
  const rawQuery = text(query).trim();
  if (!rawQuery) return [];
  const source = text(content);
  const haystack = source.toLocaleLowerCase();
  const needle = rawQuery.toLocaleLowerCase();
  const limit = Math.max(1, options.limit || DEFAULT_MAX_CONTENT_MATCHES_PER_FILE);
  /** @type {Array<{ index: number, line: number, column: number, excerpt: string, matchIndex: number }>} */
  const matches = [];
  let from = 0;
  while (matches.length < limit) {
    const index = haystack.indexOf(needle, from);
    if (index < 0) break;
    matches.push({
      index,
      matchIndex: matches.length,
      ...lineInfoAt(source, index),
    });
    from = index + Math.max(needle.length, 1);
  }
  return matches;
}

/** @param {Array<{ results?: any[] }>} groups */
export function countFileWorkspaceSearchResults(groups) {
  return (Array.isArray(groups) ? groups : []).reduce((total, group) => total + (group.results || []).length, 0);
}

/**
 * @param {{
 *   query: unknown,
 *   files: Array<{ id: string, label?: string, path?: string, content?: string, raw?: any }>,
 *   currentFileId?: string,
 *   currentContent?: string,
 *   maxResultsPerGroup?: number,
 *   maxContentMatchesPerFile?: number,
 * }} options
 */
export function buildFileWorkspaceSearchGroups(options) {
  const rawQuery = text(options.query).trim();
  const normalizedQuery = normalize(rawQuery);
  const maxResultsPerGroup = Math.max(1, options.maxResultsPerGroup || DEFAULT_MAX_RESULTS_PER_GROUP);
  const maxContentMatchesPerFile = Math.max(1, options.maxContentMatchesPerFile || DEFAULT_MAX_CONTENT_MATCHES_PER_FILE);
  /** @type {FileWorkspaceSearchGroup[]} */
  const groups = [
    { key: FILE_WORKSPACE_SEARCH_GROUPS.FILE_PATH, results: [] },
    { key: FILE_WORKSPACE_SEARCH_GROUPS.CURRENT_CONTENT, results: [] },
    { key: FILE_WORKSPACE_SEARCH_GROUPS.OTHER_CONTENT, results: [] },
  ];
  if (!normalizedQuery) return groups;

  for (const file of Array.isArray(options.files) ? options.files : []) {
    const id = text(file.id || file.path || file.label);
    if (!id) continue;
    const label = text(file.label || id);
    const path = text(file.path || id);
    const searchableIdentity = `${label}\n${path}`.toLocaleLowerCase();
    const isCurrent = Boolean(options.currentFileId && id === options.currentFileId);

    if (
      searchableIdentity.includes(normalizedQuery)
      && groups[0].results.length < maxResultsPerGroup
    ) {
      groups[0].results.push({
        id: `file-path:${id}`,
        group: FILE_WORKSPACE_SEARCH_GROUPS.FILE_PATH,
        query: rawQuery,
        file: { ...file, id, label, path },
      });
    }

    const content = isCurrent && options.currentContent != null ? options.currentContent : file.content;
    const contentMatches = findFileWorkspaceContentMatches(content, rawQuery, { limit: maxContentMatchesPerFile });
    const targetGroup = isCurrent ? groups[1] : groups[2];
    for (const match of contentMatches) {
      if (targetGroup.results.length >= maxResultsPerGroup) break;
      targetGroup.results.push({
        id: `${targetGroup.key}:${id}:${match.matchIndex}`,
        group: targetGroup.key,
        query: rawQuery,
        matchIndex: match.matchIndex,
        line: match.line,
        column: match.column,
        excerpt: match.excerpt,
        file: { ...file, id, label, path },
      });
    }
  }

  return groups;
}
