<script>
  // @ts-nocheck
  import { ChevronDown, ChevronRight, FileText, Link2 } from "lucide-svelte";
  import { t } from "$lib/shared/i18n/index.js";
  import { getToolName, tools } from "$lib/features/tools/index.js";
  import { buildSkillChangeConfirmation } from "$lib/features/skills/domain/skillDomain.js";
  import Tooltip from "$lib/shared/components/Tooltip.svelte";

  let { preview = null, showMessage = true, showEmpty = true } = $props();

  let expanded = $state(new Set());
  let model = $derived(buildSkillChangeConfirmation(preview));
  let groupKeys = $derived(model.groups.map((group) => group.key).join("|"));
  let visibleWarnings = $derived(model.warnings.filter((warning) => !isRedundantWarning(warning)));

  $effect(() => {
    groupKeys;
    expanded = new Set(model.groups.filter((group) => group.defaultExpanded).map((group) => group.key));
  });

  function toggle(key) {
    if (expanded.has(key)) expanded.delete(key);
    else expanded.add(key);
    expanded = new Set(expanded);
  }

  function operationLabel(operation) {
    return $t(`confirm.preview.operation.${operation}`);
  }

  function domainLabel(domain) {
    return $t(`confirm.preview.domain.${domain}`);
  }

  function riskLabel(risk) {
    return $t(`confirm.preview.risk.${risk}`);
  }

  function entryShapeIcon(shape) {
    return shape === "symlink" ? Link2 : FileText;
  }

  function entryShapeAccessibleLabel(shape) {
    if (shape === "symlink") return $t("confirm.preview.shape.symlink");
    if (shape === "metadata_record") return $t("confirm.preview.shape.metadata_record");
    return $t("confirm.preview.shape.file");
  }

  function isRedundantWarning(warning) {
    const text = String(warning || "");
    return [
      "预览中出现删除路径",
      "将同步覆盖其它工具的同名副本",
    ].some((pattern) => text.includes(pattern));
  }

  function visibleRisks(item, group) {
    return (item.risk || []).filter((risk) => {
      if (risk === "overwrite" && group.operation === "overwrite") return false;
      if (risk === "delete" && group.operation === "delete") return false;
      if (risk === "content_difference") return false;
      return true;
    });
  }

  function objectLabel(item) {
    const value = String(item?.affectedObject || "");
    if (value === "__shared__" || value === "generic") {
      return item?.skillName
        ? `${$t("confirm.preview.subject_shared")} · ${item.skillName}`
        : $t("confirm.preview.subject_shared");
    }
    return getToolName(value, $tools) || value || $t("confirm.preview.affected_unknown");
  }

  function overviewText(entry) {
    const parts = Object.entries(entry.counts || {})
      .filter(([, count]) => count > 0)
      .map(([operation, count]) => $t("confirm.preview.overview_count", {
        operation: operationLabel(operation),
        count,
      }));
    return parts.join("，");
  }
</script>

<div class="skill-change-confirmation">
  {#if visibleWarnings.length > 0}
    <div class="notice warning">
      {#each visibleWarnings as warning}
        <div>{warning}</div>
      {/each}
    </div>
  {/if}

  {#if model.overview.length > 0}
    <section class="confirmation-section">
      <div class="section-title">{$t("confirm.preview.overview_title")}</div>
      <div class="overview-grid">
        {#each model.overview as entry}
          <div class="overview-row">
            <span class="overview-domain">{domainLabel(entry.locationDomain)}</span>
            <span class="overview-detail">{overviewText(entry)}</span>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if model.groups.length > 0}
    <section class="confirmation-section">
      <div class="section-title">{$t("confirm.preview.detail_title")}</div>
      <div class="confirmation-groups">
        {#each model.groups as group}
          <div class="confirmation-group">
            <button class="group-header" type="button" onclick={() => toggle(group.key)}>
              <span class="group-left">
                {#if expanded.has(group.key)}
                  <ChevronDown size={13} />
                {:else}
                  <ChevronRight size={13} />
                {/if}
                <span>{operationLabel(group.operation)} · {domainLabel(group.locationDomain)}</span>
              </span>
              <span class="group-count">{group.items.length}</span>
            </button>
            {#if expanded.has(group.key)}
              <div class="group-rows">
                {#each group.items as item}
                  {@const ShapeIcon = entryShapeIcon(item.entryShape)}
                  {@const risks = visibleRisks(item, group)}
                  <div class="confirmation-row" class:danger={item.risk?.includes("delete") || item.risk?.includes("overwrite")}>
                    <div class="affected-object">{objectLabel(item)}</div>
                    <div class="path">
                      <Tooltip label={entryShapeAccessibleLabel(item.entryShape)} placement="bottom">
                        <span class="path-shape-icon" aria-label={entryShapeAccessibleLabel(item.entryShape)}>
                          <ShapeIcon size={13} />
                        </span>
                      </Tooltip>
                      <span class="path-text">{item.realPath}</span>
                    </div>
                    {#if risks.length > 0}
                      <div class="badges">
                        {#each risks as risk}
                          <span class="badge risk">{riskLabel(risk)}</span>
                        {/each}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if model.blockers.length > 0}
    <section class="confirmation-section">
      <div class="section-title blocked">{$t("confirm.preview.blocked_title")}</div>
      <div class="blocker-list">
        {#each model.blockers as blocker}
          <div class="blocker-row">
            <span>{blocker.skillName}</span>
            <span>{blocker.reason}</span>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if showMessage && model.message}
    <div class="preview-note">{model.message}</div>
  {/if}

  {#if showEmpty && !model.hasContent}
    <div class="preview-empty">{$t("confirm.preview.no_changes")}</div>
  {/if}
</div>

<style>
  .skill-change-confirmation {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .confirmation-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }

  .section-title.blocked {
    color: #f59e0b;
  }

  .overview-grid,
  .confirmation-groups,
  .blocker-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .overview-row,
  .blocker-row {
    display: grid;
    grid-template-columns: minmax(104px, 132px) minmax(0, 1fr);
    gap: 10px;
    align-items: start;
    padding: 8px 10px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-hover);
    font-size: 12px;
  }

  .overview-domain,
  .affected-object,
  .blocker-row span:first-child {
    color: var(--color-text-main);
    font-weight: 600;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .overview-detail,
  .blocker-row span:last-child {
    color: var(--color-text-muted);
    min-width: 0;
    word-break: break-word;
  }

  .confirmation-group {
    border: 1px solid var(--border-color);
    border-radius: 8px;
    overflow: hidden;
    background: var(--bg-card);
  }

  .group-header {
    width: 100%;
    border: 0;
    background: var(--bg-hover);
    color: var(--color-text-main);
    padding: 8px 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
  }

  .group-left {
    display: inline-flex;
    align-items: center;
    min-width: 0;
    gap: 6px;
  }

  .group-count {
    color: var(--color-text-muted);
    font-size: 11px;
  }

  .group-rows {
    display: flex;
    flex-direction: column;
  }

  .confirmation-row {
    display: grid;
    grid-template-columns: minmax(104px, 132px) minmax(0, 1fr);
    gap: 8px 10px;
    align-items: start;
    padding: 8px 10px;
    border-top: 1px solid var(--border-color);
    font-size: 12px;
  }

  .confirmation-row.danger {
    background: rgba(239, 68, 68, 0.07);
  }

  .path {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    color: var(--color-text-muted);
    font-family: "SF Mono", "Cascadia Code", "Fira Code", monospace;
    line-height: 1.45;
  }

  .path-shape-icon {
    display: inline-flex;
    flex: 0 0 auto;
    margin-top: 2px;
    color: var(--color-text-muted);
  }

  .path-text {
    min-width: 0;
    word-break: break-all;
  }

  .badges {
    grid-column: 2;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .badge {
    font-size: 11px;
    line-height: 1;
    padding: 3px 6px;
    border-radius: 999px;
    color: var(--color-text-muted);
    background: var(--bg-hover);
    border: 1px solid var(--border-color);
  }

  .badge.risk {
    color: #f59e0b;
    border-color: rgba(245, 158, 11, 0.24);
    background: rgba(245, 158, 11, 0.08);
  }

  .notice {
    padding: 8px 10px;
    border-radius: 8px;
    font-size: 12px;
    line-height: 1.5;
  }

  .notice.warning {
    color: #fbbf24;
    background: rgba(251, 191, 36, 0.08);
    border: 1px solid rgba(251, 191, 36, 0.18);
  }

  .preview-note,
  .preview-empty {
    font-size: 12px;
    color: var(--color-text-muted);
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .preview-empty {
    text-align: center;
    padding: 8px 0;
  }
</style>
