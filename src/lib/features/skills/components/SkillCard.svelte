<script>
  import { tools, getToolName } from "$lib/features/tools/index.js";

  let { 
    skill, 
    onView, 
    showBadges = false, 
    allToolIds = []
  } = $props();

  // For overview mode: skill has installed_in array
  // For tool/global mode: skill is a SkillInfo object
  let installedIn = $derived(skill.installed_in || []);
</script>

<button class="skill-card" onclick={() => onView(skill)}>
  <div class="card-body">
    <div class="card-top">
      <div class="card-info">
        <div class="card-title">{skill.name}</div>
        {#if skill.description}
          <div class="card-desc">{skill.description}</div>
        {/if}
      </div>
    </div>
    {#if skill.path}
      <div class="card-path">{skill.path}</div>
    {/if}
    {#if showBadges && allToolIds.length > 0}
      <div class="card-badges">
        {#each allToolIds as tid}
          <span class="tool-badge" class:installed={installedIn.includes(tid)} class:common={tid === "generic"}>
            {getToolName(tid, $tools)}
          </span>
        {/each}
      </div>
    {/if}
  </div>
</button>

<style>
  .skill-card { 
    display: flex; gap: 14px; 
    background: var(--bg-card); border-radius: 8px; 
    padding: 16px 18px; align-items: flex-start; 
    width: 100%; text-align: left; 
    border: 1px solid var(--border-color); 
    cursor: pointer; 
    transition: all 0.2s cubic-bezier(0.18, 0.89, 0.32, 1.28); 
  }
  .skill-card:hover { border-color: var(--border-active); background: var(--bg-hover); }
  .skill-card:active { opacity: 0.9; }
  
  .card-body { flex: 1; min-width: 0; }
  .card-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }
  .card-info { min-width: 0; flex: 1; }
  .card-title { font-size: 13px; font-weight: 600; color: var(--color-text-main); margin-bottom: 3px; }
  .card-desc { font-size: 11px; color: var(--color-text-muted); line-height: 1.5; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .card-note { font-size: 10px; color: var(--color-text-muted); opacity: 0.7; flex-shrink: 0; max-width: 150px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .card-path { font-size: 11px; color: var(--color-text-muted); opacity: 0.45; font-family: "SF Mono", monospace; margin-top: 6px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  
  /* Tool badges */
  .card-badges { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 10px; }
  .tool-badge { 
    font-size: 10px; padding: 2px 8px; 
    border-radius: 4px; 
    background: var(--bg-subtle); 
    color: var(--color-text-muted); 
    opacity: 0.4;
    transition: all 0.15s;
  }
  .tool-badge.installed { 
    background: var(--bg-active); 
    color: var(--color-text-main); 
    opacity: 1; 
  }
  .tool-badge.common.installed { 
    background: rgba(74,222,128,0.12); 
    color: #4ade80; 
  }
</style>
