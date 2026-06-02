<script>
  import { ScrollText } from "lucide-svelte";
  import { t } from "$lib/shared/i18n/index.js";
  let { rule, toolId, onEdit, onCopy = undefined } = $props();

  /** @param {string} content */
  function lineCount(content) { return content ? content.split("\n").length : 0; }
  /** @param {number} ts */
  function formatDate(ts) { return ts ? new Date(ts * 1000).toLocaleDateString() : ""; }
</script>

<div class="rule-card" role="button" tabindex="0" onclick={() => onEdit(rule)} onkeydown={(e) => { if(e.key === 'Enter') onEdit(rule); }}>
  <div class="card-icon"><ScrollText size={14} strokeWidth={2} /></div>
  <div class="card-body">
    <div class="card-title">{rule.label}</div>
    <div class="card-meta">{rule.path}</div>
  </div>
  <div class="card-stats">
    <!-- svelte-ignore -->
    {$t('rules.card.lines', { count: lineCount(rule.content) })}
  </div>
</div>

<style>
  .rule-card { display: flex; align-items: center; gap: 12px; background: var(--bg-card); border-radius: 8px; padding: 10px 14px; cursor: pointer; transition: background 0.15s, border-color 0.15s, opacity 0.15s; border: 1px solid var(--border-color); outline: none; }
  .rule-card:hover, .rule-card:focus { background: var(--bg-hover); border-color: var(--border-active); box-shadow: none; }
  .rule-card:active { opacity: 0.9; }
  .card-icon { width: 24px; height: 24px; border-radius: 8px; background: var(--bg-hover); color: var(--color-text-muted); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
  .card-body { display: flex; flex-direction: column; gap: 3px; flex: 1; min-width: 0; }
  .card-title { font-size: 13px; color: var(--color-text-main); font-weight: 500; }
  .card-meta { font-size: 11px; color: var(--color-text-muted); font-family: monospace; opacity: 0.7; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .card-stats { font-size: 11px; color: var(--color-text-muted); opacity: 0.6; white-space: nowrap; padding-left: 12px; font-variant-numeric: tabular-nums; }
</style>
