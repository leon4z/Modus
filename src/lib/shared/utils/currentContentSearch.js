// Purpose: Shared match counting for page-header search scoped to one visible content body.

/** @param {unknown} value */
function text(value) {
  return value == null ? "" : String(value);
}

/**
 * @param {unknown} content
 * @param {unknown} query
 */
export function countCurrentContentMatches(content, query) {
  const rawQuery = text(query).trim();
  if (!rawQuery) return 0;
  const source = text(content).toLocaleLowerCase();
  const needle = rawQuery.toLocaleLowerCase();
  let count = 0;
  let from = 0;
  for (;;) {
    const index = source.indexOf(needle, from);
    if (index < 0) break;
    count += 1;
    from = index + Math.max(needle.length, 1);
  }
  return count;
}

/**
 * @param {unknown} current
 * @param {unknown} count
 * @param {"next" | "previous"} direction
 */
export function stepCurrentContentMatchIndex(current, count, direction) {
  const total = Number(count) || 0;
  if (total <= 0) return 0;
  const index = Math.min(Math.max(0, Number(current) || 0), total - 1);
  const delta = direction === "previous" ? -1 : 1;
  return (index + delta + total) % total;
}
