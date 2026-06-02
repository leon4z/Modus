<script>
  let { skill = null, onChanged = () => {} } = $props();

  function triggerChanged() {
    if (!skill?.name) return;
    onChanged({
      ...skill,
      path: skill.path || `/stub/${skill.name}`,
      tool_statuses: skill.tool_statuses || skill.toolStatuses || [],
    });
  }
</script>

{#if skill}
  <div data-testid="skill-viewer-stub">Viewing {skill.display_name || skill.name}</div>
  <button type="button" data-testid="skill-viewer-stub-change" onclick={triggerChanged} hidden>change</button>
{/if}
