// Purpose: Provide deterministic browser data for frontend visual verification.

export const VISUAL_VERIFICATION_VIEWS = [
  "dashboard",
  "rules",
  "skills",
  "mcp",
  "config",
  "settings",
];

const managedToolIds = ["codex", "claude-code", "dev_tool"];

const fixtureTools = [
  {
    id: "codex",
    name: "Codex",
    detected: true,
    config_dir: "/Users/visual/.codex",
    skills_dir: "/Users/visual/.codex/skills",
	    capabilities: [
	      {
	        id: "global-rule",
	        kind: "rule",
	        scope: "global",
	        access: "writable",
	        format: "markdown",
	        source_path: "/Users/visual/.codex/AGENTS.md",
	        label: "AGENTS.md",
	        source_confidence: "official_docs",
	      },
	      {
	        id: "project-rule",
	        kind: "rule",
	        scope: "project",
	        access: "readable",
	        format: "markdown",
	        source_path: "/Users/visual/workspace/AGENTS.md",
	        label: "Project AGENTS.md",
	        source_confidence: "official_docs",
	      },
	      {
	        id: "tool-skills",
	        kind: "skill",
	        scope: "tool",
	        access: "writable",
	        format: "skill_directory",
	        source_path: "/Users/visual/.codex/skills",
	        label: "Codex Skills",
	      },
	      {
	        id: "shared-skills",
	        kind: "skill",
	        scope: "shared",
	        access: "read_only",
	        format: "skill_directory",
	        source_path: "/Users/visual/.agents/skills",
	        label: "Shared Skills",
	      },
	      {
	        id: "mcp-config",
	        kind: "mcp",
	        scope: "global",
	        access: "writable",
	        format: "json",
	        source_path: "/Users/visual/.codex/mcp.json",
	        label: "MCP Config",
	      },
	      {
	        id: "settings",
	        kind: "ordinary_config",
	        scope: "global",
	        access: "writable",
	        format: "json",
	        source_path: "/Users/visual/.codex/config.json",
	        label: "Config",
	      },
	    ],
	    rule_sources: [
	      {
	        label: "AGENTS.md",
	        path: "/Users/visual/.codex/AGENTS.md",
	        group: "Global",
	        content: "# Visual global rule\n\nKeep visual checks deterministic.",
	      },
	    ],
	  },
	  {
	    id: "claude-code",
    name: "Claude Code",
    detected: true,
    config_dir: "/Users/visual/.claude",
    skills_dir: "/Users/visual/.claude/skills",
	    capabilities: [
	      {
	        id: "global-rule",
	        kind: "rule",
	        scope: "global",
	        access: "writable",
	        format: "markdown",
	        source_path: "/Users/visual/.claude/CLAUDE.md",
	        label: "CLAUDE.md",
	        source_confidence: "official_docs",
	      },
	      {
	        id: "global-rules-directory",
	        kind: "rule",
	        scope: "global",
	        access: "writable",
	        format: "directory",
	        source_path: "/Users/visual/.claude/rules",
	        label: "Rules directory",
	        source_confidence: "official_docs",
	      },
	      {
	        id: "project-rule",
	        kind: "rule",
	        scope: "project",
	        access: "readable",
	        format: "markdown",
	        source_path: "/Users/visual/workspace/CLAUDE.md",
	        label: "Project CLAUDE.md",
	        source_confidence: "official_docs",
	      },
	      {
	        id: "tool-skills",
	        kind: "skill",
	        scope: "tool",
	        access: "writable",
	        format: "skill_directory",
	        source_path: "/Users/visual/.claude/skills",
	        label: "Claude Skills",
	      },
	      {
	        id: "shared-skills",
	        kind: "skill",
	        scope: "shared",
	        access: "read_only",
	        format: "skill_directory",
	        source_path: "/Users/visual/.agents/skills",
	        label: "Shared Skills",
	      },
	      {
	        id: "mcp-support",
	        kind: "mcp",
	        scope: "global",
	        access: "unknown",
	        format: "unknown",
	        source_path: "",
	        label: "MCP",
	      },
	      {
	        id: "settings",
	        kind: "ordinary_config",
	        scope: "global",
	        access: "read_only",
	        format: "json",
	        source_path: "/Users/visual/.claude/settings.json",
	        label: "Settings",
	      },
	    ],
	    rule_sources: [
	      {
	        label: "CLAUDE.md",
	        path: "/Users/visual/.claude/CLAUDE.md",
	        group: "Global",
	        content: "# Visual target rule\n\nTarget file for copy modal evidence.",
	      },
	    ],
	  },
  {
    id: "dev_tool",
    name: "Dev Tool",
    detected: true,
    config_dir: "/Users/visual/.dev-tool",
    skills_dir: "/Users/visual/.dev-tool/skills",
    capabilities: [
      { kind: "rule", scope: "global", access: "unsupported" },
      { kind: "skill", scope: "global", access: "unsupported" },
      { kind: "mcp", scope: "global", access: "unsupported" },
      {
        id: "ordinary-config",
        kind: "ordinary_config",
        scope: "global",
        access: "unknown",
        format: "unknown",
        source_path: "",
        label: "Config",
      },
    ],
  },
];

const toolSelectorManyTools = [
  {
    id: "openclaw",
    name: "OpenClaw",
    detected: true,
    config_dir: "/Users/visual/.openclaw",
    skills_dir: "/Users/visual/.openclaw/skills",
    capabilities: [
      {
        id: "trusted-workspace-agents",
        kind: "rule",
        scope: "global",
        access: "writable",
        format: "markdown",
        source_path: "/Users/visual/.openclaw/workspace/AGENTS.md",
        label: "Workspace AGENTS.md",
        source_confidence: "official_docs",
      },
      {
        id: "tool-skills",
        kind: "skill",
        scope: "tool",
        access: "writable",
        format: "skill_directory",
        source_path: "/Users/visual/.openclaw/skills",
        label: "OpenClaw Skills",
      },
      { kind: "mcp", scope: "global", access: "unsupported" },
      {
        id: "openclaw-config",
        kind: "ordinary_config",
        scope: "global",
        access: "writable",
        format: "json",
        source_path: "/Users/visual/.openclaw/openclaw.json",
        label: "OpenClaw Config",
      },
    ],
  },
  {
    id: "cursor",
    name: "Cursor",
    detected: true,
    config_dir: "/Users/visual/.cursor",
    skills_dir: "/Users/visual/.cursor/skills",
    capabilities: [
      {
        id: "user-rules",
        kind: "rule",
        scope: "global",
        access: "unsupported",
        format: "unknown",
        source_path: "",
        label: "User Rules",
        source_confidence: "official_docs",
        notes: "Cursor User Rules are app-internal settings and do not expose a stable file-backed sync target.",
      },
      {
        id: "tool-skills",
        kind: "skill",
        scope: "tool",
        access: "writable",
        format: "skill_directory",
        source_path: "/Users/visual/.cursor/skills",
        label: "Cursor Skills",
      },
      {
        id: "shared-skills",
        kind: "skill",
        scope: "shared",
        access: "read_only",
        format: "skill_directory",
        source_path: "/Users/visual/.agents/skills",
        label: "Shared Skills",
      },
      {
        id: "mcp-config",
        kind: "mcp",
        scope: "global",
        access: "writable",
        format: "json",
        source_path: "/Users/visual/.cursor/mcp.json",
        label: "MCP Config",
      },
      {
        id: "ordinary-config",
        kind: "ordinary_config",
        scope: "global",
        access: "unknown",
        format: "unknown",
        source_path: "",
        label: "Config",
      },
    ],
  },
  {
    id: "trae",
    name: "Trae",
    detected: true,
    config_dir: "/Users/visual/.trae",
    skills_dir: "",
    capabilities: [
      {
        id: "user-rules",
        kind: "rule",
        scope: "global",
        access: "unsupported",
        format: "unknown",
        source_path: "",
        label: "User Rules",
        source_confidence: "official_community",
        notes: "Trae global rules are not confirmed as a stable external file-backed target for Modus synchronization.",
      },
      {
        id: "project-rules",
        kind: "rule",
        scope: "project",
        access: "read_only",
        format: "markdown",
        source_path: ".trae/rules/project_rules.md",
        label: "Project rules",
        source_confidence: "official_community",
      },
      {
        id: "project-mcp-config",
        kind: "mcp",
        scope: "project",
        access: "read_only",
        format: "json",
        source_path: ".trae/mcp.json",
        label: "Project MCP configuration",
        source_confidence: "official_community",
      },
      {
        id: "skills",
        kind: "skill",
        scope: "tool",
        access: "unknown",
        format: "unknown",
        source_path: "",
        label: "Skills",
        source_confidence: "official_community",
      },
      {
        id: "ordinary-config",
        kind: "ordinary_config",
        scope: "global",
        access: "unknown",
        format: "unknown",
        source_path: "",
        label: "Config",
      },
    ],
  },
];

const toolSelectorManyManagedToolIds = [
  ...managedToolIds,
  ...toolSelectorManyTools.map((tool) => tool.id),
];

const defaultRules = [
  {
    id: "common_rule",
    name: "Visual baseline rule",
    content: "# Visual baseline rule\n\nKeep workspace rules small and explicit.\n\nLong editor verification line: keep-this-single-source-line-associated-with-one-gutter-number-even-when-the-editor-wraps-it-across-several-visual-rows-for-cursor-and-selection-review.",
    inject_to: ["codex"],
  },
];
/** @type {null | { common_rule: string, custom_rules: Record<string, string>, custom_rule_pending_targets: Record<string, string[]> }} */
let visualDefaultRuleInjectionBaselines = null;

const skills = [
  {
    name: "visual-review",
    display_name: "Visual Review",
    description: "Review visible frontend changes with deterministic fixture state.",
    path: "/Users/visual/.codex/skills/visual-review",
    tool_statuses: [
      {
        tool_id: "codex",
        tool_name: "Codex",
        status: "installed",
        path: "/Users/visual/.codex/skills/visual-review",
        path_origin: "tool",
        source_variant: null,
        content_relation: "same",
        content_hash: "visual-review-codex",
      },
      {
        tool_id: "claude-code",
        tool_name: "Claude Code",
        status: "installed",
        path: "/Users/visual/.claude/skills/visual-review",
        path_origin: "tool",
        source_variant: null,
        content_relation: "different",
        content_hash: "visual-review-claude",
      },
    ],
  },
  {
    name: "tool-config",
    display_name: "Tool Configuration",
    description: "Inspect registered configuration files.",
    path: "/Users/visual/.claude/skills/tool-config",
    tool_statuses: [
      {
        tool_id: "claude-code",
        status: "installed",
        path: "/Users/visual/.claude/skills/tool-config",
      },
    ],
  },
  {
    name: "gsd",
    display_name: "GSD",
    description: "Coordinate project work with a package of commands and agents.",
    path: "/Users/visual/.codex/skills/gsd",
    tool_statuses: [
      {
        tool_id: "codex",
        status: "installed",
        path: "/Users/visual/.codex/skills/gsd",
      },
    ],
    package: {
      isPackage: true,
      memberCount: 3,
      members: [
        {
          name: "plan-phase",
          displayName: "Plan Phase",
          description: "Create a phase plan.",
          kind: "command",
          relativePath: "commands/plan-phase",
        },
        {
          name: "executor",
          displayName: "Executor",
          description: "Execute planned work.",
          kind: "agent",
          relativePath: "agents/executor",
        },
        {
          name: "checkpoints",
          displayName: "Checkpoints",
          description: "Checkpoint reference.",
          kind: "reference",
          relativePath: "references/checkpoints",
        },
      ],
    },
  },
];

const localToolSkill = {
  name: "local-helper",
  display_name: "Local Helper",
  description: "Local tool directory sources available for direct use.",
  path: "/Users/visual/.codex/skills/local-helper",
  tool_statuses: [
    {
      tool_id: "codex",
      tool_name: "Codex",
      status: "installed",
      path: "/Users/visual/.codex/skills/local-helper",
      content_hash: "visual-local-helper-codex",
      updated_at: "2026-04-30T08:00:00Z",
    },
    {
      tool_id: "claude-code",
      tool_name: "Claude Code",
      status: "installed",
      path: "/Users/visual/.claude/skills/local-helper",
      content_hash: "visual-local-helper-claude",
      updated_at: "2026-04-30T08:03:00Z",
    },
  ],
};

const metadataDriftSkill = {
  name: "metadata-drift",
  display_name: "Metadata Drift",
  description: "Skill metadata exists, but the corresponding content source is missing or inconsistent.",
  path: "/Users/visual/.codex/skills/metadata-drift",
  tool_statuses: [
    {
      tool_id: "codex",
      tool_name: "Codex",
      status: "variant_drifted",
      path: "/Users/visual/.codex/skills/metadata-drift",
      path_origin: "tool",
      content_hash: "visual-metadata-drift",
      updated_at: "2026-05-15T08:00:00Z",
    },
  ],
};

const sharedLinkSkill = {
  name: "shared-design",
  display_name: "Shared Design",
  description: "Shared source used directly by some tools and copied by others.",
  path: "/Users/visual/.agents/skills/shared-design",
  has_variants: [],
  variant_paths: {
    generic: "/Users/visual/.agents/skills/shared-design",
  },
  tool_statuses: [
    {
      tool_id: "codex",
      tool_name: "Codex",
      status: "variant_installed_copy",
      path: "/Users/visual/.agents/skills/shared-design",
      path_origin: "generic",
      content_hash: "visual-shared-design",
      updated_at: "2026-05-04T08:00:00Z",
    },
    {
      tool_id: "claude-code",
      tool_name: "Claude Code",
      status: "notInstalled",
      path: "",
    },
  ],
};

const sharedPickerSkill = {
  ...sharedLinkSkill,
  name: "shared-picker",
  display_name: "Shared Picker",
  path: "/Users/visual/.agents/skills/shared-picker",
  variant_paths: {
    generic: "/Users/visual/.agents/skills/shared-picker",
  },
  tool_statuses: [
    {
      tool_id: "codex",
      tool_name: "Codex",
      status: "variant_installed_copy",
      path: "/Users/visual/.agents/skills/shared-picker",
      path_origin: "generic",
      content_hash: "visual-shared-picker",
      updated_at: "2026-05-04T08:00:00Z",
    },
    {
      tool_id: "claude-code",
      tool_name: "Claude Code",
      status: "variant_installed_copy",
      path: "/Users/visual/.agents/skills/shared-picker",
      path_origin: "generic",
      content_hash: "visual-shared-picker",
      updated_at: "2026-05-04T08:00:00Z",
    },
    {
      tool_id: "openclaw",
      tool_name: "OpenClaw",
      status: "localOnly",
      path: "/Users/visual/.openclaw/skills/shared-picker",
      path_origin: "tool",
      content_hash: "visual-shared-picker-openclaw",
      updated_at: "2026-05-04T08:08:00Z",
    },
    {
      tool_id: "dev_tool",
      tool_name: "Dev Tool",
      status: "notInstalled",
      path: "",
    },
  ],
};

function skillInventoryFixture() {
  const state = getVisualVerificationState();
  const shouldShowUnmanaged = state === "skill-detail-unconnected-connect-action"
    || state === "skill-detail-shared-link"
    || state === "skill-detail-shared-picker";
  const unmanagedSkills = state === "skill-detail-shared-link"
    ? [sharedLinkSkill]
    : state === "skill-detail-shared-picker"
      ? [sharedPickerSkill]
      : [localToolSkill];
  const list = shouldShowUnmanaged
    ? [...unmanagedSkills, ...skills]
    : state === "skill-metadata-drift"
      ? [metadataDriftSkill, ...skills]
    : skills;
  const cloned = clone(list);
  if (state === "skill-detail-content-source-selector") {
    const toolConfig = cloned.find((/** @type {any} */ item) => item.name === "tool-config");
    if (toolConfig) {
      toolConfig.path = "/Users/visual/.claude/skills/tool-config";
      toolConfig.has_variants = ["claude-code", "codex"];
      toolConfig.variant_paths = {
        "claude-code": "/Users/visual/.claude/skills/tool-config",
        codex: "/Users/visual/.codex/skills/tool-config",
      };
      toolConfig.tool_statuses = [
        {
          tool_id: "claude-code",
          tool_name: "Claude Code",
          status: "installed",
          path: "/Users/visual/.claude/skills/tool-config",
        },
        {
          tool_id: "codex",
          tool_name: "Codex",
          status: "installed",
          path: "/Users/visual/.codex/skills/tool-config",
        },
      ];
    }
  }
  return cloned;
}

/** @type {Record<string, { files: Array<{ relative_path: string, is_dir: boolean }>, contents: Record<string, string> }>} */
const skillFilesByPath = {
  "/Users/visual/.agents/skills/shared-design": {
    files: [
      { relative_path: "SKILL.md", is_dir: false },
    ],
    contents: {
      "SKILL.md": "---\nname: shared-design\ndescription: Shared source used by direct readers and copied tools.\n---\n\n# Shared Design\n\nReview shared-source copy management with deterministic fixture state.",
    },
  },
  "/Users/visual/.agents/skills/shared-picker": {
    files: [
      { relative_path: "SKILL.md", is_dir: false },
    ],
    contents: {
      "SKILL.md": "---\nname: shared-picker\ndescription: Shared source picker dedupe fixture.\n---\n\n# Shared Picker\n\nReview source selection when multiple tools consume the same shared path.",
    },
  },
  "/Users/visual/.openclaw/skills/shared-picker": {
    files: [
      { relative_path: "SKILL.md", is_dir: false },
    ],
    contents: {
      "SKILL.md": "---\nname: shared-picker\ndescription: Tool-local alternative for source picker fixture.\n---\n\n# Shared Picker Local\n\nReview source picker alongside a tool-local alternative.",
    },
  },
  "/Users/visual/.codex/skills/visual-review": {
    files: [
      { relative_path: "SKILL.md", is_dir: false },
      { relative_path: "references", is_dir: true },
      { relative_path: "references/checklist.md", is_dir: false },
    ],
    contents: {
      "SKILL.md": "---\nname: visual-review\ndescription: Review visible frontend changes with deterministic fixture state.\n---\n\n# Visual Review\n\nUse this fixture to inspect modal layout, content mode switching, and source selectors.\n\nLong editor verification line: this-single-skill-source-line-should-soft-wrap-without-creating-extra-file-line-numbers-or-moving-the-editing-cursor-away-from-visible-text.",
      "references/checklist.md": "# Checklist\n\n- Header rhythm\n- Capsule treatment\n- Content toolbar spacing",
    },
  },
  "/Users/visual/.claude/skills/tool-config": {
    files: [
      { relative_path: "SKILL.md", is_dir: false },
    ],
    contents: {
      "SKILL.md": "---\nname: tool-config\ndescription: Inspect registered configuration files.\n---\n\n# Tool Configuration\n\nReview configuration surfaces with deterministic fixture state.",
    },
  },
  "/Users/visual/.codex/skills/tool-config": {
    files: [
      { relative_path: "SKILL.md", is_dir: false },
    ],
    contents: {
      "SKILL.md": "---\nname: tool-config\ndescription: Inspect Codex configuration files.\n---\n\n# Tool Configuration\n\nCodex-specific configuration review.",
    },
  },
  "/Users/visual/.codex/skills/gsd": {
    files: [
      { relative_path: "SKILL.md", is_dir: false },
      { relative_path: "commands", is_dir: true },
      { relative_path: "commands/plan-phase", is_dir: true },
      { relative_path: "commands/plan-phase/SKILL.md", is_dir: false },
      { relative_path: "agents", is_dir: true },
      { relative_path: "agents/executor", is_dir: true },
      { relative_path: "agents/executor/SKILL.md", is_dir: false },
      { relative_path: "references", is_dir: true },
      { relative_path: "references/checkpoints", is_dir: true },
      { relative_path: "references/checkpoints/SKILL.md", is_dir: false },
    ],
    contents: {
      "SKILL.md": "---\nname: gsd\ndescription: Coordinate project work with a package of commands and agents.\nversion: 1.4.0\n---\n\n# GSD\n\nPackage root for visual verification.",
      "commands/plan-phase/SKILL.md": "---\nname: plan-phase\n---\n\n# Plan Phase",
      "agents/executor/SKILL.md": "---\nname: executor\n---\n\n# Executor",
      "references/checkpoints/SKILL.md": "---\nname: checkpoints\n---\n\n# Checkpoints",
    },
  },
  "/Users/visual/.codex/skills/local-helper": {
    files: [
      { relative_path: "SKILL.md", is_dir: false },
    ],
    contents: {
      "SKILL.md": "---\nname: local-helper\ndescription: Local helper from Codex.\n---\n\n# Local Helper\n\nCodex-local copy used to verify local source layout.",
    },
  },
  "/Users/visual/.claude/skills/local-helper": {
    files: [
      { relative_path: "SKILL.md", is_dir: false },
    ],
    contents: {
      "SKILL.md": "---\nname: local-helper\ndescription: Local helper from Claude Code.\n---\n\n# Local Helper\n\nClaude-local copy used to verify local source layout.",
    },
  },
};

/** @param {unknown} skillPath */
function getSkillFileFixture(skillPath) {
  return skillFilesByPath[typeof skillPath === "string" ? skillPath : ""] || null;
}

const configFiles = [
  {
    id: "settings",
    label: "Settings",
    path: "/Users/visual/.codex/config.toml",
    format: "toml",
    access: "writable",
    editable: true,
    exists: true,
    size_bytes: 128,
    modified_unix: 1_777_000_000,
  },
  {
    id: "readonly",
    label: "Read Only Settings",
    path: "/Users/visual/.claude/settings.json",
    format: "json",
    access: "read_only",
    editable: false,
    exists: true,
    size_bytes: 64,
    modified_unix: 1_777_000_100,
  },
  {
    id: "unknown",
    label: "Unknown Support",
    path: "/Users/visual/.dev-tool/settings.json",
    format: "json",
    access: "unknown",
    editable: false,
    exists: true,
    size_bytes: 42,
    modified_unix: null,
  },
  {
    id: "unsupported",
    label: "Unsupported Settings",
    path: "/Users/visual/.dev-tool/unsupported.json",
    format: "json",
    access: "unsupported",
    editable: false,
    exists: true,
    size_bytes: 42,
    modified_unix: null,
  },
];

function browserSearchParams() {
  if (typeof window === "undefined") return new URLSearchParams();
  return new URLSearchParams(window.location.search);
}

export function isVisualVerificationMode() {
  const params = browserSearchParams();
  return params.get("visual") === "1" || params.get("visual") === "true";
}

export function getVisualVerificationView() {
  const requested = browserSearchParams().get("view") || "dashboard";
  return VISUAL_VERIFICATION_VIEWS.includes(requested) ? requested : "dashboard";
}

export function getVisualVerificationState() {
  return browserSearchParams().get("state") || "populated";
}

function isFirstRunOnboardingState() {
  return getVisualVerificationState() === "first-run-onboarding";
}

function usesExtendedToolSet() {
  const state = getVisualVerificationState();
  return state === "tool-selector-many"
    || state === "settings-tools"
    || state === "rules-tool-selector-many"
    || isFirstRunOnboardingState();
}

export function getVisualVerificationToolId() {
  const requested = browserSearchParams().get("tool") || "codex";
  const selectableToolIds = usesExtendedToolSet() ? toolSelectorManyManagedToolIds : managedToolIds;
  return selectableToolIds.includes(requested) ? requested : "codex";
}

function pendingVisualState() {
  return new Promise(() => {});
}

/**
 * @param {any} value
 * @returns {any}
 */
function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

/**
 * @param {string} skillName
 * @returns {any}
 */
function packageFixture(skillName) {
  const skill = /** @type {any[]} */ (skillInventoryFixture()).find((item) => item.name === skillName);
  return clone(skill?.package || null);
}

/**
 * @param {any} value
 * @returns {boolean}
 */
function isThenable(value) {
  return Boolean(value && typeof value.then === "function");
}

function dashboardFixture() {
  return {
    detected_count: 3,
    total_skills: 2,
    total_rules: 3,
    tools: fixtureTools.map((tool, index) => ({
      tool_id: tool.id,
      tool_name: tool.name,
      detected: tool.detected,
      rule_count: index === 2 ? 0 : 2,
      skill_count: index === 2 ? 0 : 1,
      mcp_count: index === 0 ? 2 : 0,
      config_count: index === 2 ? 0 : 2,
    })),
  };
}

function configFilesFixture() {
  const state = getVisualVerificationState();
  if (state === "loading") return pendingVisualState();
  if (state === "error") throw new Error("Visual fixture configuration error");
  if (state === "empty") return [];
  if (state === "config-single-file") return clone(configFiles.slice(0, 1));
  return clone(configFiles);
}

/**
 * @param {string} toolId
 * @returns {any}
 */
function mcpDiagnosticsFixture(toolId) {
  const state = getVisualVerificationState();
  if (state === "loading") return pendingVisualState();
  if (["missing", "empty", "unreadable", "malformed", "unsupported", "unknown"].includes(state)) {
    return {
      state,
      config_path: `/Users/visual/.${toolId}/mcp.json`,
      servers: [],
    };
  }
  if (state === "error") {
    return {
      state: "error",
      message: "Visual fixture MCP diagnostic error",
      servers: [],
    };
  }
  return {
    state: "loaded",
    config_path: `/Users/visual/.${toolId}/mcp.json`,
    servers: [
      {
        name: "browser-tools",
        description: "Fixture browser automation server",
        server_type: "stdio",
        command: "node",
        enabled: true,
        activation_state: "enabled",
      },
      {
        name: "readonly-docs",
        description: "Fixture documentation server",
        server_type: "http",
        url: "https://docs.example.invalid",
        enabled: false,
        activation_state: "disabled",
      },
    ],
  };
}

/**
 * @param {string} toolId
 * @returns {any}
 */
function mcpConfigSourcesFixture(toolId) {
  const diagnostic = mcpDiagnosticsFixture(toolId);
  if (diagnostic && typeof diagnostic.then === "function") return diagnostic;
  const sourcePath = `/Users/visual/.${toolId}/mcp.json`;
  if (["missing", "unreadable", "malformed", "unsupported", "unknown", "error"].includes(diagnostic.state)) {
    return [{
      id: "mcp-config",
      tool_id: toolId,
      label: "MCP configuration",
      path: diagnostic.state === "unsupported" || diagnostic.state === "unknown" ? null : sourcePath,
      format: "json",
      access: diagnostic.state === "unsupported" ? "unsupported" : diagnostic.state === "unknown" ? "unknown" : "writable",
      source_kind: "file",
      state: diagnostic.state,
      editable: false,
      exists: !["missing", "unsupported", "unknown"].includes(diagnostic.state),
      size_bytes: null,
      modified_unix: null,
      server_count: null,
      message: diagnostic.message || "Visual fixture MCP source state",
      servers: [],
    }];
  }
  return [{
    id: "mcp-config",
    tool_id: toolId,
    label: "MCP configuration",
    path: sourcePath,
    format: "json",
    access: "writable",
    source_kind: "file",
    state: diagnostic.servers?.length ? "loaded" : "empty",
    editable: true,
    exists: true,
    size_bytes: 248,
    modified_unix: 1770000000,
    server_count: diagnostic.servers?.length || 0,
    message: "MCP configuration loaded",
    servers: diagnostic.servers || [],
  }];
}

function managedRulesStateFixture() {
  if (getVisualVerificationState() === "rule-missing-file") {
    return {
      summary: {
        managed_targets: 1,
        requires_sync_targets: 1,
        drifted_targets: 0,
        unresolved_targets: 0,
        pending_source_rule_sets: 0,
        affected_tool_ids: ["codex"],
      },
      rule_sets: [
        {
          rule_id: "common_rule",
          rule_name: "Visual baseline rule",
          managed_tool_ids: ["codex"],
          source_pending: false,
        },
      ],
      targets: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          target_path: "/Users/visual/.codex/AGENTS.md",
          rule_set_ids: ["common_rule"],
          rule_set_names: ["Visual baseline rule"],
          classification: "requires_sync",
          reason: "file_missing",
          can_read: true,
          can_write: true,
          source_pending: false,
          has_managed_block: false,
          expected_block: "<!-- ACC:DEFAULT:START -->\n## Visual baseline rule\n\n# Visual baseline rule\n\nKeep workspace rules small and explicit.\n<!-- ACC:DEFAULT:END -->",
          current_block: "",
          message: "Rule target file will be created during injection",
        },
      ],
    };
  }
  if (getVisualVerificationState() === "rule-issue") {
    return {
      summary: {
        managed_targets: 1,
        requires_sync_targets: 0,
        drifted_targets: 1,
        unresolved_targets: 0,
        pending_source_rule_sets: 1,
        affected_tool_ids: ["codex"],
      },
      rule_sets: [
        {
          rule_id: "common_rule",
          rule_name: "Visual baseline rule",
          managed_tool_ids: ["codex"],
          source_pending: true,
        },
      ],
      targets: [
        {
          tool_id: "codex",
          tool_name: "Codex",
          target_path: "/Users/visual/.codex/AGENTS.md",
          rule_set_ids: ["common_rule"],
          rule_set_names: ["Visual baseline rule"],
          classification: "drifted",
          reason: "pending_source",
          can_read: true,
          can_write: true,
          source_pending: true,
          has_managed_block: true,
          expected_block: "<!-- ACC:DEFAULT:START -->\n## Visual baseline rule\n\n# Visual baseline rule\n\nKeep workspace rules small and explicit.\n<!-- ACC:DEFAULT:END -->",
          current_block: "<!-- ACC:DEFAULT:START -->\n## Visual baseline rule\n\n# Visual baseline rule\n\nKeep the older tool-local copy for comparison.\n<!-- ACC:DEFAULT:END -->",
        },
      ],
    };
  }
  return {
    summary: {
      managed_targets: 1,
      requires_sync_targets: 0,
      drifted_targets: 0,
      unresolved_targets: 0,
      pending_source_rule_sets: 0,
    },
    targets: [],
  };
}

/**
 * @param {{
 *   action?: string,
 *   changeKind?: string,
 *   entryKind?: string,
 *   skillName?: string,
 *   toolId?: string,
 *   path?: string,
 * }} args
 */
function operationPreviewFixture({
  action = "delete",
  changeKind = action,
  entryKind = action === "create" ? "file" : "",
  skillName = "visual-review",
  toolId = "codex",
  path = "/Users/visual/.codex/skills/visual-review",
} = {}) {
  return {
    changes: [
      {
        action,
        changeKind,
        entryKind,
        skillName,
        subject: toolId,
        toolName: toolId === "claude-code" ? "Claude Code" : toolId === "dev_tool" ? "Dev Tool" : "Codex",
        path,
      },
    ],
    creates: action === "create" ? [path] : [],
    deletes: action === "delete" ? [path] : [],
    overwrites: action === "overwrite" ? [path] : [],
    preserves: [],
    message: "Visual verification preview only.",
  };
}

/** @type {Record<string, (args?: any) => any>} */
const commandFixtures = {
  list_tools: () => {
    const state = getVisualVerificationState();
    const tools = clone(fixtureTools);
    if (usesExtendedToolSet()) {
      tools.push(...clone(toolSelectorManyTools));
    }
    if (state === "rules-tool-multiple") {
      const codex = tools.find((/** @type {any} */ tool) => tool.id === "codex");
      if (codex) {
        codex.rule_sources = [
          ...(codex.rule_sources || []),
          {
            label: "TEAM.md",
            path: "/Users/visual/.codex/TEAM.md",
            group: "Global",
            content: "# Team rule\n\nUse this second file to review collection detail navigation.",
          },
        ];
      }
    }
    if (state === "rules-tool-directory") {
      const codex = tools.find((/** @type {any} */ tool) => tool.id === "codex");
      if (codex) {
        codex.rule_sources = [
          {
            label: "AGENTS.md",
            path: "/Users/visual/.codex/workspace/AGENTS.md",
            group: "workspace",
            content: "# Workspace Rule\n\nDefault workspace guidance for directory visual evidence.",
          },
          {
            label: "TEAM.md",
            path: "/Users/visual/.codex/workspace/team/TEAM.md",
            group: "workspace/team",
            content: "# Team Rule\n\nNested team guidance for directory visual evidence.",
          },
          {
            label: "LOCAL.md",
            path: "/Users/visual/.codex/local/LOCAL.md",
            group: "local",
            content: "# Local Rule\n\nSecond root for tree navigation evidence.",
          },
        ];
      }
    }
    if (state === "skill-detail-shared-link" || state === "skill-detail-shared-picker") {
      const claude = tools.find((/** @type {any} */ tool) => tool.id === "claude-code");
      if (claude) {
        claude.allow_external_generic_symlink = true;
        claude.capabilities = (claude.capabilities || []).map((/** @type {any} */ cap) => cap.kind === "skill"
          ? { ...cap, access: "writable" }
          : cap);
      }
      const devTool = tools.find((/** @type {any} */ tool) => tool.id === "dev_tool");
      if (devTool) {
        devTool.allow_external_generic_symlink = true;
        devTool.capabilities = (devTool.capabilities || []).map((/** @type {any} */ cap) => cap.kind === "skill"
          ? { ...cap, access: "writable" }
          : cap);
      }
      if (state === "skill-detail-shared-picker") {
        tools.push({
          id: "openclaw",
          name: "OpenClaw",
          detected: true,
          config_dir: "/Users/visual/.openclaw",
          skills_dir: "/Users/visual/.openclaw/skills",
          capabilities: [
            { kind: "skill", scope: "global", access: "writable" },
          ],
        });
      }
    }
    return tools;
  },
  is_initialized: () => !isFirstRunOnboardingState(),
  get_language: () => "en",
  get_theme: () => isFirstRunOnboardingState() ? "dark" : "light",
  get_managed_tools: () => isFirstRunOnboardingState()
    ? []
    : usesExtendedToolSet()
    ? clone(toolSelectorManyManagedToolIds)
    : clone(managedToolIds),
  set_managed_tools: () => true,
  get_handled_new_tool_ids: () => [],
  set_handled_new_tool_ids: () => true,
	  get_dashboard: () => dashboardFixture(),
	  list_default_rules: () => clone(defaultRules),
	  save_default_rule: () => true,
	  delete_default_rule: () => true,
	  inject_default_rules: () => true,
	  copy_rule: () => true,
	  read_rule_content: ({ path } = {}) => {
	    const sources = fixtureTools.flatMap((tool) => tool.rule_sources || []);
	    return sources.find((source) => source.path === path)?.content || "# Visual rule\n";
	  },
	  get_default_rule_injection_baselines: () => {
	    if (getVisualVerificationState() === "rule-issue") {
	      return {
	        common_rule: "old-visual-baseline",
	        custom_rules: {},
	        custom_rule_pending_targets: {},
	      };
	    }
	    return clone(visualDefaultRuleInjectionBaselines || {
	      common_rule: "",
	      custom_rules: {},
	      custom_rule_pending_targets: {},
	    });
	  },
  set_default_rule_injection_baselines: ({ baselines } = {}) => {
    visualDefaultRuleInjectionBaselines = clone(baselines || {
      common_rule: "",
      custom_rules: {},
      custom_rule_pending_targets: {},
    });
    return true;
  },
  get_managed_rules_state: () => managedRulesStateFixture(),
  diff_rules: ({ leftContent = "", leftLabel = "Left", rightContent = "", rightLabel = "Right" } = {}) => ({
    left_label: leftLabel,
    right_label: rightLabel,
    changes: [
      { tag: "equal", content: "<!-- ACC:DEFAULT:START -->" },
      { tag: "delete", content: leftContent.split("\n").find((/** @type {string} */ line) => line.includes("Keep project")) || leftContent },
      { tag: "insert", content: rightContent.split("\n").find((/** @type {string} */ line) => line.includes("Keep the older")) || rightContent },
      { tag: "equal", content: "<!-- ACC:DEFAULT:END -->" },
    ],
  }),
  write_application_log: () => true,
  scan_skill_inventory: () => ({ skills: skillInventoryFixture() }),
  scan_skill_inventory_entry: ({ skillName } = {}) => {
    const skills = skillInventoryFixture();
    return clone(skills.find((/** @type {any} */ skill) => skill.name === skillName) || skills[0]);
  },
  list_generic_skills: () => skillInventoryFixture().slice(0, 1),
  list_skills: () => skillInventoryFixture(),
  list_skill_files: ({ skillPath } = {}) => clone(getSkillFileFixture(skillPath)?.files || []),
  read_skill_content: ({ skillPath } = {}) => ({
    skill_md_content: getSkillFileFixture(skillPath)?.contents["SKILL.md"] || "",
  }),
  read_skill_file: ({ skillPath, relativePath } = {}) => {
    const fixture = getSkillFileFixture(skillPath);
    const key = typeof relativePath === "string" ? relativePath : "SKILL.md";
    return fixture?.contents[key] || "";
  },
  write_skill_file: () => true,
  get_mcp_diagnostics: ({ toolId } = {}) => mcpDiagnosticsFixture(toolId || getVisualVerificationToolId()),
  list_mcp_config_sources: ({ toolId } = {}) => mcpConfigSourcesFixture(toolId || getVisualVerificationToolId()),
  read_mcp_server_config_fragment: ({ toolId, sourceId, serverName } = {}) => {
    const targetToolId = toolId || getVisualVerificationToolId();
    const source = mcpConfigSourcesFixture(targetToolId)[0];
    const server = (source.servers || [])[0] || { name: serverName || "browser-tools", server_type: "stdio" };
    return {
      tool_id: targetToolId,
      source_id: sourceId || source.id,
      server_name: serverName || server.name,
      label: serverName || server.name,
      path: source.path,
      format: source.format,
      editable: source.editable,
      content: "{\n  \"command\": \"node\",\n  \"args\": [\"server.js\"],\n  \"env\": {\n    \"BROWSER_TOKEN\": \"visual-secret\"\n  }\n}\n",
      server,
    };
  },
  save_mcp_server_config_fragment: () => ({
    backup_path: "/Users/visual/.modus/backups/mcp-config/codex/mcp-config/mcp.json",
    size_bytes: 256,
    modified_unix: 1770000100,
    servers: [],
  }),
  list_mcp_servers: ({ toolId } = {}) => {
    const diagnostic = mcpDiagnosticsFixture(toolId || getVisualVerificationToolId());
    if (diagnostic && typeof diagnostic.then === "function") return diagnostic;
    return diagnostic.servers || [];
  },
  list_config_files: () => configFilesFixture(),
  read_config_file: ({ fileId } = {}) => {
    const file = configFiles.find((item) => item.id === fileId) || configFiles[0];
    return {
      ...clone(file),
      content: file.format === "toml"
        ? "model = \"gpt-5\"\n[plugins.\"build-web-apps@openai-curated-with-an-extra-long-name-for-visual-wrap-check\"]\nenabled = true\n[profiles.community]\ntrust_level = \"trusted\"\n"
        : "{\n  \"model\": \"gpt-5\"\n}\n",
    };
  },
  save_config_file: () => ({
    backup_path: "/Users/visual/.backup/config",
    size_bytes: 160,
    modified_unix: 1_777_000_200,
  }),
  get_tool_paths: () => ({
    codex: { config_dir: "/Users/visual/.codex", skills_dir: "/Users/visual/.codex/skills" },
    "claude-code": { config_dir: "/Users/visual/.claude", skills_dir: "/Users/visual/.claude/skills" },
  }),
  get_injection_targets: () => ({
    codex: "/Users/visual/.codex/AGENTS.md",
    "claude-code": "/Users/visual/.claude/CLAUDE.md",
  }),
  get_tool_capability_overrides: () => getVisualVerificationState() === "settings-tools"
    ? {
      cursor: {
        customGlobalRuleTarget: "/Users/visual/.cursor/CUSTOM-RULES.md",
        sharedSkillDirectRead: true,
      },
      "claude-code": {
        sharedSkillDirectRead: false,
      },
    }
    : {},
  set_tool_capability_overrides: () => true,
  get_custom_tools: () => [],
  install_skill_v2: (args = {}) => args.dryRun === false
    ? true
    : operationPreviewFixture({
      action: "create",
      entryKind: "symlink",
      skillName: args.skillName || "visual-review",
      toolId: args.toolId || "codex",
      path: `/Users/visual/.${args.toolId || "codex"}/skills/${args.skillName || "visual-review"}`,
    }),
  copy_skill_to_tool: (args = {}) => args.dryRun === false
    ? true
    : operationPreviewFixture({
      action: "copy",
      changeKind: "create",
      entryKind: "file",
      skillName: args.skillName || "shared-design",
      toolId: args.toolId || "claude-code",
      path: `/Users/visual/.${args.toolId || "claude-code"}/skills/${args.skillName || "shared-design"}`,
    }),
  link_shared_skill_to_tool: (args = {}) => args.dryRun === false
    ? true
    : operationPreviewFixture({
      action: "install",
      changeKind: "create",
      entryKind: "symlink",
      skillName: args.skillName || "shared-design",
      toolId: args.toolId || "claude-code",
      path: `/Users/visual/.${args.toolId || "claude-code"}/skills/${args.skillName || "shared-design"}`,
    }),
  uninstall_skill_v2: (args = {}) => args.dryRun === false
    ? true
    : operationPreviewFixture({
        action: "delete",
        skillName: args.skillName || "visual-review",
        toolId: args.toolId || "codex",
        path: `/Users/visual/.${args.toolId || "codex"}/skills/${args.skillName || "visual-review"}`,
      }),
  delete_skill_from_tool_v2: (args = {}) => args.dryRun === false
    ? true
    : operationPreviewFixture({
      action: "delete",
      skillName: args.skillName || "visual-review",
      toolId: args.toolId || "codex",
      path: `/Users/visual/.${args.toolId || "codex"}/skills/${args.skillName || "visual-review"}`,
    }),
  delete_skill_v2: (args = {}) => args.dryRun === false
    ? true
    : operationPreviewFixture({
      action: "delete",
      skillName: args.skillName || "visual-review",
      toolId: "codex",
      path: `/Users/visual/.codex/skills/${args.skillName || "visual-review"}`,
    }),
  get_application_log_path: () => "/Users/visual/Library/Application Support/Modus/logs/app.log",
};

/**
 * @param {string} command
 * @param {Record<string, any>} [args]
 */
export async function getVisualInvokeResponse(command, args = {}) {
  const handler = commandFixtures[command];
  if (!handler) {
    throw new Error(`No visual verification fixture registered for command: ${command}`);
  }
  return handler(args || {});
}

/**
 * @param {string} view
 * @param {{ state?: string, tool?: string }} [options]
 */
export function getVisualVerificationUrl(view, options = {}) {
  const params = new URLSearchParams({
    visual: "1",
    view: VISUAL_VERIFICATION_VIEWS.includes(view) ? view : "dashboard",
  });
  if (options.state) params.set("state", options.state);
  if (options.tool) params.set("tool", options.tool);
  return `/?${params.toString()}`;
}
