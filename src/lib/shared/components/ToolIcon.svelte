<script>
  import { Terminal } from 'lucide-svelte';
  
  import openclawIcon from '$lib/shared/assets/icons/openclaw-color.png';
  import codexIcon from '$lib/shared/assets/icons/codex-color.svg';
  import claudeCodeIcon from '$lib/shared/assets/icons/claude-code-color.svg';
  import cursorIconDark from '$lib/shared/assets/icons/cursor-dark.svg';
  import cursorIconLight from '$lib/shared/assets/icons/cursor-light.svg';
  import codeBuddyIcon from '$lib/shared/assets/icons/codebuddy-color.svg';
  import githubCopilotIconDark from '$lib/shared/assets/icons/github-copilot-dark.svg';
  import githubCopilotIconLight from '$lib/shared/assets/icons/github-copilot-light.svg';
  import hermesAgentIconDark from '$lib/shared/assets/icons/hermes-agent-dark.svg';
  import hermesAgentIconLight from '$lib/shared/assets/icons/hermes-agent-light.svg';
  import kiroIcon from '$lib/shared/assets/icons/kiro-color.png';
  import opencodeIconDark from '$lib/shared/assets/icons/opencode-dark.svg';
  import opencodeIconLight from '$lib/shared/assets/icons/opencode-light.svg';
  import piAgentIcon from '$lib/shared/assets/icons/pi-agent-color.svg';
  import qoderIconDark from '$lib/shared/assets/icons/qoder-color-dark.svg';
  import qoderIconLight from '$lib/shared/assets/icons/qoder-color-light.svg';
  import traeIcon from '$lib/shared/assets/icons/trae-color.svg';
  import windsurfIconDark from '$lib/shared/assets/icons/windsurf-dark.svg';
  import windsurfIconLight from '$lib/shared/assets/icons/windsurf-light.svg';
  import workBuddyIcon from '$lib/shared/assets/icons/workbuddy-color.png';

  let { toolId, size = 16, strokeWidth = 1.8, class: className = "", style = "", shadow = false } = $props();
  let fallbackGlyphSize = $derived(Math.max(10, Math.round(size * 0.62)));

  /**
   * Theme-sensitive assets need explicit pairs because SVGs loaded as images
   * cannot inherit the app's currentColor.
   * @type {Record<string, { src?: string, light?: string, dark?: string, alt: string, variant: "color" | "theme-pair", opticalScale?: number }>}
   */
  const brandIcons = {
    openclaw: { src: openclawIcon, alt: "OpenClaw Logo", variant: "color" },
    codex: { src: codexIcon, alt: "Codex Logo", variant: "color" },
    cursor: {
      light: cursorIconLight,
      dark: cursorIconDark,
      alt: "Cursor Logo",
      variant: "theme-pair",
    },
    trae: { src: traeIcon, alt: "Trae Logo", variant: "color" },
    "trae-cn": { src: traeIcon, alt: "Trae Logo", variant: "color" },
    "trae-solo": { src: traeIcon, alt: "Trae Solo Logo", variant: "color" },
    "trae-solo-cn": { src: traeIcon, alt: "Trae Solo CN Logo", variant: "color" },
    "claude-code": { src: claudeCodeIcon, alt: "Claude Code Logo", variant: "color" },
    claude: { src: claudeCodeIcon, alt: "Claude Code Logo", variant: "color" },
    qoder: {
      light: qoderIconLight,
      dark: qoderIconDark,
      alt: "Qoder Logo",
      variant: "theme-pair",
    },
    opencode: {
      light: opencodeIconLight,
      dark: opencodeIconDark,
      alt: "OpenCode Logo",
      variant: "theme-pair",
    },
    codebuddy: { src: codeBuddyIcon, alt: "CodeBuddy Logo", variant: "color" },
    workbuddy: { src: workBuddyIcon, alt: "WorkBuddy Logo", variant: "color", opticalScale: 1.24 },
    "github-copilot": {
      light: githubCopilotIconLight,
      dark: githubCopilotIconDark,
      alt: "GitHub Copilot Logo",
      variant: "theme-pair",
    },
    "hermes-agent": {
      light: hermesAgentIconLight,
      dark: hermesAgentIconDark,
      alt: "Hermes Agent Logo",
      variant: "theme-pair",
    },
    kiro: { src: kiroIcon, alt: "Kiro Logo", variant: "color" },
    "pi-agent": { src: piAgentIcon, alt: "Pi Agent Logo", variant: "color" },
    windsurf: {
      light: windsurfIconLight,
      dark: windsurfIconDark,
      alt: "Windsurf Logo",
      variant: "theme-pair",
    },
  };

  let brandIcon = $derived(brandIcons[String(toolId)]);
</script>

<div class="tool-icon-wrapper {className}" {style} style:width="{size}px" style:height="{size}px" style:display="flex" style:align-items="center" style:justify-content="center">
  {#if brandIcon}
    {#if brandIcon.variant === "theme-pair"}
      <img class="brand-icon brand-icon--light" src={brandIcon.light} alt={brandIcon.alt} style:width="{size}px" style:height="{size}px" />
      <img class="brand-icon brand-icon--dark" src={brandIcon.dark} alt={brandIcon.alt} style:width="{size}px" style:height="{size}px" />
    {:else}
      <img class="brand-icon brand-icon--single" src={brandIcon.src} alt={brandIcon.alt} style:width="{size}px" style:height="{size}px" style:transform={brandIcon.opticalScale ? `scale(${brandIcon.opticalScale})` : undefined} />
    {/if}
  {:else}
    <!-- Fallback default tool icon -->
    <div class="unknown-tool-icon" class:has-shadow={shadow} style:width="{size}px" style:height="{size}px">
      <Terminal size={fallbackGlyphSize} strokeWidth={strokeWidth} />
    </div>
  {/if}
</div>

<style>
  .tool-icon-wrapper {
    flex-shrink: 0;
  }
  .brand-icon {
    object-fit: contain;
    display: block;
  }
  .brand-icon--dark {
    display: none;
  }
  :global([data-theme="dark"]) .brand-icon--light {
    display: none;
  }
  :global([data-theme="dark"]) .brand-icon--dark {
    display: block;
  }
  .unknown-tool-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-muted);
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    border-radius: 22%;
  }
  .unknown-tool-icon.has-shadow {
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
  }
</style>
