<script>
  // @ts-nocheck
  import { t } from "$lib/shared/i18n/index.js";
  import { tools, getToolName } from "$lib/features/tools/index.js";

  let {
    changes = [],
    groupBy = "kind",
    defaultExpanded = ["create", "overwrite", "delete"],
    highlightDestructive = true,
  } = $props();

  let expanded = $state(new Set());
  $effect(() => {
    expanded = new Set(defaultExpanded || []);
  });

  function groupByKind() {
    const grouped = {
      create: [],
      overwrite: [],
      delete: [],
      preserve: [],
    };
    for (const change of visibleChanges()) {
      if (!grouped[change.kind]) grouped[change.kind] = [];
      grouped[change.kind].push(change);
    }
    return grouped;
  }

  function toggle(kind) {
    if (expanded.has(kind)) expanded.delete(kind);
    else expanded.add(kind);
    expanded = new Set(expanded);
  }

  const labelMap = {
    create: "confirm.preview.create",
    overwrite: "confirm.preview.overwrite",
    delete: "confirm.preview.delete",
    preserve: "confirm.preview.preserve",
  };

  function entryKindLabel(kind) {
    if (kind === "file") return "";
    if (kind === "symlink") return $t("confirm.preview.kind_symlink");
    return "";
  }

  function normalizeToolId(row) {
    return row?.toolId || row?.tool_id || "";
  }

  function subjectLabel(row) {
    const toolId = normalizeToolId(row);
    if (toolId === "__shared__" || toolId === "generic") return $t("confirm.preview.subject_shared");
    if (!toolId) return $t("confirm.preview.subject_shared");
    return getToolName(toolId, $tools) || toolId;
  }

  function isDefaultNote(note) {
    return [
      "共享目录新建",
      "共享目录覆盖",
      "共享目录",
      "共享目录将删除",
      "路径将删除",
      "保持不变",
      "内容已与 base 一致",
      "已与共享目录一致",
      "工具副本将被覆盖为 base",
      "将被共享目录覆盖",
    ].includes(String(note || ""));
  }

  function displayNote(row) {
    const note = String(row?.note || "");
    return isDefaultNote(note) ? "" : note;
  }

  function displayPath(row) {
    return String(row?.absPath || row?.abs_path || "");
  }

  function rowKey(row) {
    return [
      row?.kind || "",
      row?.absPath || row?.abs_path || "",
      row?.entryKind || row?.entry_kind || "",
    ].join("\u0000");
  }

  function preferRow(current, candidate) {
    const currentToolId = normalizeToolId(current);
    const candidateToolId = normalizeToolId(candidate);
    if (candidateToolId === "__shared__" && currentToolId !== "__shared__") return candidate;
    if (!isDefaultNote(candidate?.note) && isDefaultNote(current?.note)) return candidate;
    return current;
  }

  function visibleChanges() {
    const deduped = new Map();
    for (const change of changes || []) {
      const key = rowKey(change);
      if (!deduped.has(key)) {
        deduped.set(key, change);
        continue;
      }
      deduped.set(key, preferRow(deduped.get(key), change));
    }
    return Array.from(deduped.values());
  }

  let grouped = $derived.by(() => {
    if (groupBy === "skill") {
      const bySkill = {};
      for (const change of visibleChanges()) {
        const key = change.skillName || "-";
        if (!bySkill[key]) bySkill[key] = [];
        bySkill[key].push(change);
      }
      return bySkill;
    }
    return groupByKind();
  });
</script>

<div class="path-change-list" class:group-skill={groupBy === "skill"}>
  {#if groupBy === "skill"}
    {#each Object.entries(grouped) as [skillName, rows]}
      <div class="change-group">
        <div class="group-title">{skillName}</div>
        {#each rows as row}
          <div class="change-row" class:danger={highlightDestructive && (row.kind === "overwrite" || row.kind === "delete")}>
            <span class="kind">{$t(labelMap[row.kind] || "confirm.preview.section_title")}</span>
            <span class="meta-cluster">
              <span class="meta subject">{subjectLabel(row)}</span>
              {#if displayNote(row)}
                <span class="meta note">{displayNote(row)}</span>
              {/if}
              {#if entryKindLabel(row.entryKind || row.entry_kind)}
                <span class="meta entry-kind">{entryKindLabel(row.entryKind || row.entry_kind)}</span>
              {/if}
            </span>
            <span class="path">{displayPath(row)}</span>
          </div>
        {/each}
      </div>
    {/each}
  {:else}
    {#each Object.entries(grouped) as [kind, rows]}
      {#if rows.length > 0}
        <div class="change-group">
          <button class="group-title toggle" onclick={() => toggle(kind)}>
            <span>{$t(labelMap[kind] || "confirm.preview.section_title")}</span>
            <span>{rows.length}</span>
          </button>
          {#if expanded.has(kind)}
            {#each rows as row}
              <div class="change-row" class:danger={highlightDestructive && (kind === "overwrite" || kind === "delete")}>
                <span class="meta-cluster">
                  <span class="meta subject">{subjectLabel(row)}</span>
                  {#if displayNote(row)}
                    <span class="meta note">{displayNote(row)}</span>
                  {/if}
                  {#if entryKindLabel(row.entryKind || row.entry_kind)}
                    <span class="meta entry-kind">{entryKindLabel(row.entryKind || row.entry_kind)}</span>
                  {/if}
                </span>
                <span class="path">{displayPath(row)}</span>
              </div>
            {/each}
          {/if}
        </div>
      {/if}
    {/each}
  {/if}
</div>

<style>
  .path-change-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .change-group {
    border: 1px solid var(--border-color);
    border-radius: 8px;
    overflow: hidden;
  }
  .group-title {
    width: 100%;
    text-align: left;
    border: 0;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-hover);
    color: var(--color-text-main);
    padding: 8px 10px;
    font-size: 12px;
    font-weight: 600;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .group-title.toggle { cursor: pointer; }
  .change-row {
    display: grid;
    grid-template-columns: minmax(96px, 128px) minmax(0, 1fr);
    column-gap: 16px;
    row-gap: 6px;
    align-items: center;
    padding: 8px 10px;
    border-top: 1px solid var(--border-color);
    font-size: 12px;
    color: var(--color-text-main);
  }
  .group-skill .change-row {
    grid-template-columns: 64px minmax(96px, 128px) minmax(0, 1fr);
  }
  .change-row:first-of-type {
    border-top: 0;
  }
  .change-row.danger {
    background: rgba(239, 68, 68, 0.08);
  }
  .kind {
    color: var(--color-text-muted);
    min-width: 64px;
  }
  .meta {
    flex: 0 0 auto;
    color: var(--color-text-muted);
    font-size: 11px;
    line-height: 1.4;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--bg-hover);
    border: 1px solid var(--border-color);
    text-align: center;
  }
  .meta-cluster {
    min-width: 0;
    max-width: 128px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-wrap: nowrap;
    justify-self: start;
  }
  .meta.subject {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta.entry-kind {
    min-width: 48px;
  }
  .path {
    word-break: break-all;
    font-family: "SF Mono", monospace;
    min-width: 0;
  }
</style>
