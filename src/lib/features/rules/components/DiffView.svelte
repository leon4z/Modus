<script>
  import { detectedTools } from "$lib/features/tools/index.js";
  import { diffRules } from "$lib/features/rules/api/rules.js";
  import { t } from "$lib/shared/i18n/index.js";

  let leftToolId = $state("");
  let rightToolId = $state("");
  let leftRulePath = $state("");
  let rightRulePath = $state("");
  /** @type {any} */
  let diffResult = $state(/** @type {any} */ (null));
  let loading = $state(false);

  let currentDetected = $derived($detectedTools);
  let leftTool = $derived(currentDetected.find((t) => t.id === leftToolId));
  let rightTool = $derived(currentDetected.find((t) => t.id === rightToolId));
  let leftRules = $derived(leftTool?.rule_sources || []);
  let rightRules = $derived(rightTool?.rule_sources || []);

  async function runDiff() {
    const leftRule = leftRules.find((/** @type {any} */ r) => r.path === leftRulePath);
    const rightRule = rightRules.find((/** @type {any} */ r) => r.path === rightRulePath);
    if (!leftRule || !rightRule) return;
    loading = true;
    try {
      diffResult = await diffRules(leftRule.content, `${/** @type {any} */ (leftTool).name} / ${leftRule.label}`, rightRule.content, `${/** @type {any} */ (rightTool).name} / ${rightRule.label}`);
    } finally { loading = false; }
  }
</script>

<div class="diff-view">
  <div class="diff-header-title">{$t('rules.diff.title')}</div>
  <div class="diff-controls">
    <div class="diff-select">
      <select bind:value={leftToolId} onchange={() => { leftRulePath = ""; diffResult = null; }}>
        <option value="">{$t('rules.diff.select_left_tool')}</option>
        {#each currentDetected as tool}<option value={tool.id}>{tool.name}</option>{/each}
      </select>
      {#if leftRules.length > 0}
        <select bind:value={leftRulePath}>
          <option value="">{$t('rules.diff.select_file')}</option>
          {#each leftRules as rule}<option value={rule.path}>{rule.label}</option>{/each}
        </select>
      {/if}
    </div>
    <button class="btn-diff" onclick={runDiff} disabled={!leftRulePath || !rightRulePath || loading}>{loading ? $t('rules.diff.comparing') : $t('rules.diff.compare')}</button>
    <div class="diff-select">
      <select bind:value={rightToolId} onchange={() => { rightRulePath = ""; diffResult = null; }}>
        <option value="">{$t('rules.diff.select_right_tool')}</option>
        {#each currentDetected as tool}<option value={tool.id}>{tool.name}</option>{/each}
      </select>
      {#if rightRules.length > 0}
        <select bind:value={rightRulePath}>
          <option value="">{$t('rules.diff.select_file')}</option>
          {#each rightRules as rule}<option value={rule.path}>{rule.label}</option>{/each}
        </select>
      {/if}
    </div>
  </div>
  {#if diffResult}
    <div class="diff-content">
      <div class="diff-header"><span class="diff-label">{diffResult.left_label}</span><span class="diff-label">{diffResult.right_label}</span></div>
      <div class="diff-lines">
        {#each diffResult.changes as line}
          <div class="diff-line {line.tag}"><span class="diff-sign">{line.tag === "delete" ? "-" : line.tag === "insert" ? "+" : " "}</span><span class="diff-text">{line.content}</span></div>
        {/each}
      </div>
    </div>
  {:else}
    <div class="diff-empty">{$t('rules.diff.empty')}</div>
  {/if}
</div>

<style>
  .diff-view { padding: 20px; }
  .diff-header-title { font-size: 14px; font-weight: 600; color: var(--color-text-main); margin-bottom: 14px; }
  .diff-controls { display: flex; align-items: flex-start; gap: 12px; margin-bottom: 16px; }
  .diff-select { flex: 1; display: flex; flex-direction: column; gap: 6px; }
  .diff-select select { width: 100%; padding: 8px 10px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; color: var(--color-text-main); font-size: 12px; }
  .btn-diff { padding: 8px 16px; background: var(--bg-active); color: var(--color-text-main); border: none; border-radius: 8px; cursor: pointer; font-size: 12px; white-space: nowrap; margin-top: 24px; }
  .btn-diff:disabled { opacity: 0.5; cursor: not-allowed; }
  .diff-content { background: var(--bg-card); border-radius: 8px; overflow: hidden; }
  .diff-header { display: flex; justify-content: space-between; padding: 10px 16px; border-bottom: 1px solid var(--border-color); }
  .diff-label { font-size: 11px; color: var(--color-text-muted); }
  .diff-lines { font-family: "SF Mono", "Fira Code", monospace; font-size: 12px; line-height: 1.6; max-height: 60vh; overflow-y: auto; }
  .diff-line { display: flex; padding: 1px 16px; }
  .diff-line.delete { background: rgba(248,113,113,0.08); color: #f87171; }
  .diff-line.insert { background: rgba(74,222,128,0.08); color: #4ade80; }
  .diff-line.equal { color: var(--color-text-muted); }
  .diff-sign { width: 16px; flex-shrink: 0; user-select: none; }
  .diff-text { white-space: pre-wrap; word-break: break-all; }
  .diff-empty { text-align: center; padding: 60px; color: var(--color-text-muted); opacity: 0.5; font-size: 13px; }
</style>
