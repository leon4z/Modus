import { describe, expect, it } from "vitest";
import {
  canReadCapability,
  canWriteCapability,
  getCapabilityActionState,
  getCertifiedGlobalRuleTarget,
  getCertifiedSharedSkillDirectReadDefault,
  getNonFileEditableGlobalRuleCapability,
  getRulePromptDecision,
  hasNonFileEditableGlobalRuleModel,
  hasWritableCapability,
  hasProjectionAction,
  listOrdinaryConfigCapabilities,
  listCapabilityProjections,
  listToolCapabilities,
  projectCapability,
  projectionAllowsAction,
  summarizeEffectiveToolCapabilities,
  summarizeRuleSourceState,
  pathIsInsideDirectory,
} from "../domain/capabilities.js";

const tool = {
  id: "codex",
  capabilities: [
    {
      id: "rule",
      kind: "rule",
      scope: "project",
      access: "writable",
      format: "markdown",
      source_path: ".cursor/rules/project.mdc",
      source_confidence: "official_docs",
    },
    {
      id: "global-rule",
      kind: "rule",
      scope: "global",
      access: "writable",
      format: "markdown",
      source_path: "/Users/visual/.codex/AGENTS.md",
      source_confidence: "official_docs",
    },
    {
      id: "skill",
      kind: "skill",
      scope: "tool",
      access: "read_only",
      format: "skill_directory",
      source_path: "/Users/visual/.codex/skills",
    },
    {
      id: "shared-skill",
      kind: "skill",
      scope: "shared",
      access: "read_only",
      format: "skill_directory",
      source_path: "/Users/visual/.agents/skills",
    },
    {
      id: "mcp",
      kind: "mcp",
      scope: "global",
      access: "read_only",
      format: "json",
    },
    {
      id: "config",
      kind: "ordinary_config",
      scope: "global",
      access: "readable",
      format: "json",
    },
    {
      id: "unknown-config",
      kind: "ordinary_config",
      scope: "global",
      access: "unknown",
      format: "unknown",
    },
    {
      id: "unsupported-config",
      kind: "ordinary_config",
      scope: "global",
      access: "unsupported",
      format: "unknown",
    },
  ],
};

describe("tool capability helpers", () => {
  it("lists ordinary config without mixing rules skills or MCP sources", () => {
    const primaryDirectoryTool = {
      id: "trae-solo-cn",
      capabilities: [
        ...tool.capabilities,
        {
          id: "primary-config-directory",
          kind: "ordinary_config",
          scope: "global",
          access: "readable",
          format: "directory",
          source_kind: "primary_config_directory",
          source_path: "/Users/visual/.trae-cn",
        },
      ],
    };

    expect(listOrdinaryConfigCapabilities(primaryDirectoryTool).map((capability) => capability.id)).toEqual([
      "config",
      "unknown-config",
    ]);
  });

  it("gates write actions to writable capabilities only", () => {
    expect(canWriteCapability({ access: "writable" })).toBe(true);
    expect(canWriteCapability({ access: "read_only" })).toBe(false);
    expect(canWriteCapability({ access: "readable" })).toBe(false);
    expect(canWriteCapability({ access: "unknown" })).toBe(false);
    expect(canWriteCapability({ access: "unsupported" })).toBe(false);
    expect(hasWritableCapability(tool, "rule")).toBe(true);
    expect(hasWritableCapability(tool, "skill")).toBe(false);
    expect(hasWritableCapability(tool, "mcp")).toBe(false);
  });

  it("projects raw rule capabilities before granting injection actions", () => {
    const projectRule = projectCapability("codex", "rules", tool.capabilities[0]);
    const globalRule = projectCapability("codex", "rules", tool.capabilities[1]);

    expect(projectRule).toMatchObject({
      sourceRole: "project_source",
      exclusionReason: "project_scoped",
    });
    expect(projectionAllowsAction(projectRule, "inject")).toBe(false);
    expect(globalRule).toMatchObject({
      sourceRole: "global_target",
      exclusionReason: null,
    });
    expect(projectionAllowsAction(globalRule, "inject")).toBe(true);
  });

  it("allows certified local global rules only when inject evidence exists", () => {
    const projection = projectCapability("opencode", "rules", {
      id: "global-rule",
      kind: "rule",
      scope: "global",
      access: "writable",
      format: "markdown",
      source_path: "/Users/visual/.config/opencode/AGENTS.md",
      source_confidence: "certified_local_product_behavior",
      action_evidence: [{ action: "inject", supported: true, evidence: "verified" }],
    });

    expect(projection.sourceRole).toBe("global_target");
    expect(projectionAllowsAction(projection, "inject")).toBe(true);
  });

  it("keeps writable rule directories as tool rule sources instead of injection targets", () => {
    const projection = projectCapability("claude-code", "rules", {
      id: "global-rules-directory",
      kind: "rule",
      scope: "global",
      access: "writable",
      format: "directory",
      source_path: "/Users/visual/.claude/rules",
      source_confidence: "official_docs",
    });

    expect(projection.sourceRole).toBe("native_file_source");
    expect(projection.exclusionReason).toBe(null);
    expect(projectionAllowsAction(projection, "create")).toBe(true);
    expect(projectionAllowsAction(projection, "inject")).toBe(false);
  });

  it("keeps shared skill consumption distinct from writable tool directories", () => {
    const projections = listCapabilityProjections(tool, "skills");

    expect(projections.map((projection) => projection.sourceRole)).toEqual([
      "tool_directory",
      "shared_source",
    ]);
    expect(projections[0].exclusionReason).toBe("read_only");
    expect(projectionAllowsAction(projections[1], "delete")).toBe(false);
  });

  it("keeps metadata-backed skill sources read-only unless managed actions are verified", () => {
    const traeSoloCn = {
      id: "trae-solo-cn",
      capabilities: [
        {
          id: "skills",
          kind: "skill",
          scope: "tool",
          access: "readable",
          format: "skill_directory",
          source_path: "/Users/visual/.trae-cn/skills",
          supporting_sources: [
            {
              id: "skill-metadata",
              role: "metadata",
              source_path: "/Users/visual/.trae-cn/skill-config.json",
              format: "json",
              required: true,
            },
          ],
          action_evidence: [
            { action: "read", supported: true, evidence: "verified" },
            { action: "diagnose", supported: true, evidence: "verified" },
          ],
        },
      ],
    };

    const projection = listCapabilityProjections(traeSoloCn, "skills")[0];

    expect(projection.sourceRole).toBe("compound_source");
    expect(projection.exclusionReason).toBe("compound_source_requires_verified_action");
    expect(projectionAllowsAction(projection, "read")).toBe(true);
    expect(projectionAllowsAction(projection, "install")).toBe(false);
    expect(projectionAllowsAction(projection, "delete")).toBe(false);
    expect(hasWritableCapability(traeSoloCn, "skill")).toBe(false);
  });

  it("does not infer managed actions for compound Skill sources without evidence", () => {
    const toolWithCompoundSource = {
      id: "compound-tool",
      capabilities: [
        {
          id: "skills",
          kind: "skill",
          scope: "tool",
          access: "writable",
          format: "skill_directory",
          source_path: "/Users/visual/.compound/skills",
          supporting_sources: [
            {
              id: "skill-metadata",
              role: "metadata",
              source_path: "/Users/visual/.compound/skill-config.json",
              format: "json",
              required: true,
            },
          ],
        },
      ],
    };

    const projection = listCapabilityProjections(toolWithCompoundSource, "skills")[0];

    expect(projection.sourceRole).toBe("compound_source");
    expect(projection.exclusionReason).toBe("compound_source_requires_verified_action");
    expect(projectionAllowsAction(projection, "install")).toBe(false);
    expect(projectionAllowsAction(projection, "delete")).toBe(false);
    expect(hasProjectionAction(toolWithCompoundSource, "skills", "install")).toBe(false);
  });

  it("projects writable tool-directory Skills with shared deployment actions", () => {
    const traeCn = {
      id: "trae-cn",
      capabilities: [
        {
          id: "skills",
          kind: "skill",
          scope: "tool",
          access: "writable",
          format: "skill_directory",
          source_path: "/Users/visual/.trae-cn/skills",
          source_confidence: "certified_local_product_behavior",
          action_evidence: [
            { action: "view", supported: true, evidence: "verified" },
            { action: "read", supported: true, evidence: "verified" },
            { action: "install", supported: true, evidence: "verified" },
            { action: "copy", supported: true, evidence: "verified" },
            { action: "uninstall", supported: true, evidence: "verified" },
            { action: "delete", supported: true, evidence: "verified" },
            { action: "save", supported: true, evidence: "verified" },
          ],
        },
      ],
    };

    const projection = listCapabilityProjections(traeCn, "skills")[0];

    expect(projection.sourceRole).toBe("tool_directory");
    expect(projection.exclusionReason).toBe(null);
    expect(projectionAllowsAction(projection, "copy")).toBe(true);
    expect(projectionAllowsAction(projection, "delete")).toBe(true);
    expect(projectionAllowsAction(projection, "save")).toBe(true);
    expect(projectionAllowsAction(projection, "install")).toBe(true);
    expect(projectionAllowsAction(projection, "uninstall")).toBe(true);
    expect(hasProjectionAction(traeCn, "skills", "copy")).toBe(true);
    expect(hasWritableCapability(traeCn, "skill")).toBe(true);
  });

  it("separates native rule read evidence from default injection eligibility", () => {
    const traeSoloCn = {
      id: "trae-solo-cn",
      capabilities: [
        {
          id: "user-rules",
          kind: "rule",
          scope: "global",
          access: "readable",
          format: "directory",
          source_path: "/Users/visual/.trae-cn/user_rules",
          source_confidence: "official_community",
          action_evidence: [
            { action: "read", supported: true, evidence: "verified" },
            { action: "diagnose", supported: true, evidence: "verified" },
          ],
        },
      ],
    };

    const projection = listCapabilityProjections(traeSoloCn, "rules")[0];

    expect(projection.sourceRole).toBe("native_file_source");
    expect(projectionAllowsAction(projection, "read")).toBe(true);
    expect(projectionAllowsAction(projection, "inject")).toBe(false);
    expect(hasWritableCapability(traeSoloCn, "rule")).toBe(false);
  });

  it("projects Trae CN native Rules and MCP while keeping dedicated sources out of ordinary config", () => {
    const traeCn = {
      id: "trae-cn",
      capabilities: [
        {
          id: "primary-config-directory",
          kind: "ordinary_config",
          scope: "global",
          access: "readable",
          format: "directory",
          source_kind: "primary_config_directory",
          source_path: "/Users/visual/.trae-cn",
        },
        {
          id: "user-rules",
          kind: "rule",
          scope: "global",
          access: "writable",
          format: "directory",
          source_path: "/Users/visual/.trae-cn/user_rules",
          source_confidence: "certified_local_product_behavior",
          action_evidence: [
            { action: "view", supported: true, evidence: "verified" },
            { action: "read", supported: true, evidence: "verified" },
            { action: "create", supported: true, evidence: "verified" },
            { action: "save", supported: true, evidence: "verified" },
            { action: "delete", supported: true, evidence: "verified" },
          ],
        },
        {
          id: "mcp-config",
          kind: "mcp",
          scope: "global",
          access: "writable",
          format: "json",
          source_path: "/Users/visual/Library/Application Support/Trae CN/User/mcp.json",
          source_confidence: "certified_local_product_behavior",
          action_evidence: [
            { action: "view", supported: true, evidence: "verified" },
            { action: "read", supported: true, evidence: "verified" },
            { action: "edit", supported: true, evidence: "verified" },
            { action: "save", supported: true, evidence: "verified" },
          ],
        },
      ],
    };

    const ruleProjections = listCapabilityProjections(traeCn, "rules");
    const nativeRuleProjection = ruleProjections.find((projection) => projection.evidence.id === "user-rules");
    const mcpProjection = listCapabilityProjections(traeCn, "mcp")[0];

    if (!nativeRuleProjection || !mcpProjection) {
      throw new Error("Expected Trae CN rule and MCP projections");
    }

    expect(nativeRuleProjection.sourceRole).toBe("native_file_source");
    expect(nativeRuleProjection.exclusionReason).toBe(null);
    expect(projectionAllowsAction(nativeRuleProjection, "create")).toBe(true);
    expect(projectionAllowsAction(nativeRuleProjection, "inject")).toBe(false);
    expect(ruleProjections.some((projection) => projection.sourceRole === "global_target")).toBe(false);
    expect(hasWritableCapability(traeCn, "rule")).toBe(false);
    expect(mcpProjection.sourceRole).toBe("global_config");
    expect(mcpProjection.exclusionReason).toBe(null);
    expect(projectionAllowsAction(mcpProjection, "save")).toBe(true);
    expect(listOrdinaryConfigCapabilities(traeCn)).toEqual([]);
  });

  it("projects migrated adapter capabilities without snapshot fields", () => {
    /**
     * @param {...string} items
     * @returns {Array<{ action: string, supported: boolean, evidence: string }>}
     */
    const actions = (...items) => items.map((action) => ({
      action,
      supported: true,
      evidence: `${action} is supported`,
    }));
    const codebuddy = {
      id: "codebuddy",
      capabilities: [
        {
          id: "global-rule",
          kind: "rule",
          scope: "global",
          access: "writable",
          format: "markdown",
          source_path: "/Users/visual/.codebuddy/CODEBUDDY.md",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "create", "edit", "save", "inject"),
        },
        {
          id: "user-rules-directory",
          kind: "rule",
          scope: "global",
          access: "writable",
          format: "mdc",
          source_path: "/Users/visual/.codebuddy/rules",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "create", "edit", "save", "delete"),
        },
        {
          id: "agent-skills",
          kind: "skill",
          scope: "tool",
          access: "writable",
          format: "skill_directory",
          source_path: "/Users/visual/.codebuddy/skills",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "install", "copy", "uninstall", "delete"),
        },
        {
          id: "shared-skills",
          kind: "skill",
          scope: "shared",
          access: "read_only",
          format: "skill_directory",
          source_path: "/Users/visual/.agents/skills",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose"),
        },
        {
          id: "mcp-config",
          kind: "mcp",
          scope: "global",
          access: "writable",
          format: "json",
          source_path: "/Users/visual/.codebuddy/mcp.json",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "edit", "save"),
        },
        {
          id: "ordinary-settings",
          kind: "ordinary_config",
          scope: "global",
          access: "writable",
          format: "json",
          source_path: "/Users/visual/.codebuddy/settings.json",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "edit", "save"),
        },
      ],
    };
    const workbuddy = {
      id: "workbuddy",
      capabilities: [
        {
          id: "global-rule",
          kind: "rule",
          scope: "global",
          access: "writable",
          format: "markdown",
          source_path: "/Users/visual/.workbuddy/SOUL.md",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "create", "edit", "save", "inject"),
        },
        {
          id: "agent-skills",
          kind: "skill",
          scope: "tool",
          access: "writable",
          format: "skill_directory",
          source_path: "/Users/visual/.workbuddy/skills",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "install", "copy", "uninstall", "delete"),
        },
        {
          id: "mcp-config",
          kind: "mcp",
          scope: "global",
          access: "writable",
          format: "json",
          source_path: "/Users/visual/.workbuddy/mcp.json#mcpServers",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "edit", "save"),
        },
        {
          id: "ordinary-settings",
          kind: "ordinary_config",
          scope: "global",
          access: "writable",
          format: "json",
          source_path: "/Users/visual/.workbuddy/settings.json",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "edit", "save"),
        },
      ],
    };
    const hermes = {
      id: "hermes-agent",
      capabilities: [
        {
          id: "global-rule",
          kind: "rule",
          scope: "global",
          access: "writable",
          format: "markdown",
          source_path: "/Users/visual/.hermes/SOUL.md",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "create", "edit", "save", "inject"),
        },
        {
          id: "rule-directory",
          kind: "rule",
          scope: "global",
          access: "unsupported",
          format: "unknown",
          source_path: "",
          source_confidence: "certified_local_product_behavior",
        },
        {
          id: "agent-skills",
          kind: "skill",
          scope: "tool",
          access: "writable",
          format: "skill_directory",
          source_path: "/Users/visual/.hermes/skills",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "install", "copy", "uninstall", "delete"),
        },
        {
          id: "shared-skills",
          kind: "skill",
          scope: "shared",
          access: "unknown",
          format: "skill_directory",
          source_path: "/Users/visual/.agents/skills",
          source_confidence: "certified_local_product_behavior",
        },
        {
          id: "mcp-config",
          kind: "mcp",
          scope: "global",
          access: "writable",
          format: "yaml",
          source_path: "/Users/visual/.hermes/config.yaml#mcp_servers",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "edit", "save"),
        },
        {
          id: "ordinary-config",
          kind: "ordinary_config",
          scope: "global",
          access: "writable",
          format: "yaml",
          source_path: "/Users/visual/.hermes/config.yaml",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "edit", "save"),
        },
      ],
    };
    const kiro = {
      id: "kiro",
      capabilities: [
        {
          id: "global-steering",
          kind: "rule",
          scope: "global",
          access: "writable",
          format: "directory",
          source_path: "/Users/visual/.kiro/steering",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "create", "edit", "save", "delete"),
        },
        {
          id: "agent-skills",
          kind: "skill",
          scope: "tool",
          access: "writable",
          format: "skill_directory",
          source_path: "/Users/visual/.kiro/skills",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "install", "copy", "uninstall", "delete"),
        },
        {
          id: "shared-skills",
          kind: "skill",
          scope: "shared",
          access: "unsupported",
          format: "unknown",
          source_path: "",
          source_confidence: "certified_local_product_behavior",
        },
        {
          id: "mcp-config",
          kind: "mcp",
          scope: "global",
          access: "writable",
          format: "json",
          source_path: "/Users/visual/.kiro/settings/mcp.json",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "edit", "save"),
        },
        {
          id: "ordinary-config",
          kind: "ordinary_config",
          scope: "global",
          access: "unsupported",
          format: "unknown",
          source_path: "",
          source_confidence: "certified_local_product_behavior",
        },
      ],
    };
    const windsurf = {
      id: "windsurf",
      capabilities: [
        {
          id: "global-rules",
          kind: "rule",
          scope: "global",
          access: "read_only",
          format: "markdown",
          source_path: "/Users/visual/.codeium/windsurf/memories/global_rules.md",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose"),
        },
        {
          id: "skills",
          kind: "skill",
          scope: "tool",
          access: "writable",
          format: "skill_directory",
          source_path: "/Users/visual/.codeium/windsurf/skills",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "install", "copy", "uninstall", "delete"),
        },
        {
          id: "shared-skills",
          kind: "skill",
          scope: "shared",
          access: "read_only",
          format: "skill_directory",
          source_path: "/Users/visual/.agents/skills",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose"),
        },
        {
          id: "mcp-config",
          kind: "mcp",
          scope: "global",
          access: "writable",
          format: "json",
          source_path: "/Users/visual/.codeium/windsurf/mcp_config.json",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "edit", "save"),
        },
        {
          id: "ordinary-config",
          kind: "ordinary_config",
          scope: "global",
          access: "unsupported",
          format: "unknown",
          source_path: "",
          source_confidence: "certified_local_product_behavior",
        },
      ],
    };
    const copilot = {
      id: "github-copilot",
      capabilities: [
        {
          id: "global-rule",
          kind: "rule",
          scope: "global",
          access: "writable",
          format: "markdown",
          source_path: "/Users/visual/.copilot/copilot-instructions.md",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "create", "edit", "save", "inject"),
        },
        {
          id: "rule-directory",
          kind: "rule",
          scope: "global",
          access: "writable",
          format: "instructions_markdown",
          source_path: "/Users/visual/.copilot/instructions",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "create", "edit", "save", "delete"),
        },
        {
          id: "shared-skills",
          kind: "skill",
          scope: "shared",
          access: "read_only",
          format: "skill_directory",
          source_path: "/Users/visual/.agents/skills",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose"),
        },
        {
          id: "agent-skills",
          kind: "skill",
          scope: "tool",
          access: "writable",
          format: "skill_directory",
          source_path: "/Users/visual/.copilot/skills",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "install", "copy", "uninstall", "delete"),
        },
        {
          id: "mcp-config",
          kind: "mcp",
          scope: "global",
          access: "writable",
          format: "json",
          source_path: "/Users/visual/.copilot/mcp-config.json",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "edit", "save"),
        },
        {
          id: "ordinary-settings",
          kind: "ordinary_config",
          scope: "global",
          access: "writable",
          format: "json",
          source_path: "/Users/visual/.copilot/settings.json",
          source_confidence: "certified_local_product_behavior",
          action_evidence: actions("view", "read", "diagnose", "edit", "save"),
        },
      ],
    };

    expect(getCertifiedGlobalRuleTarget(codebuddy)).toBe("/Users/visual/.codebuddy/CODEBUDDY.md");
    expect(getCertifiedSharedSkillDirectReadDefault(codebuddy)).toBe(true);
    expect(hasProjectionAction(codebuddy, "rules", "inject")).toBe(true);
    expect(listCapabilityProjections(codebuddy, "rules").some((projection) => (
      projection.sourceRole === "native_file_source"
      && projection.evidence.id === "user-rules-directory"
      && projectionAllowsAction(projection, "create")
      && !projectionAllowsAction(projection, "inject")
    ))).toBe(true);
    expect(hasProjectionAction(codebuddy, "skills", "install")).toBe(true);
    expect(hasProjectionAction(codebuddy, "mcp", "save")).toBe(true);
    expect(hasProjectionAction(codebuddy, "ordinary_config", "save")).toBe(true);

    expect(getCertifiedGlobalRuleTarget(workbuddy)).toBe("/Users/visual/.workbuddy/SOUL.md");
    expect(getCertifiedSharedSkillDirectReadDefault(workbuddy)).toBe(false);
    expect(hasProjectionAction(workbuddy, "rules", "inject")).toBe(true);
    expect(hasProjectionAction(workbuddy, "skills", "install")).toBe(true);
    expect(hasProjectionAction(workbuddy, "mcp", "save")).toBe(true);
    expect(hasProjectionAction(workbuddy, "ordinary_config", "save")).toBe(true);
    expect(listCapabilityProjections(workbuddy, "skills").some((projection) => projection.sourceRole === "shared_source")).toBe(false);
    expect(getCertifiedGlobalRuleTarget(hermes)).toBe("/Users/visual/.hermes/SOUL.md");
    expect(getCertifiedSharedSkillDirectReadDefault(hermes)).toBe(false);
    expect(hasProjectionAction(hermes, "rules", "inject")).toBe(true);
    expect(listCapabilityProjections(hermes, "rules").some((projection) => (
      projection.evidence.id === "rule-directory"
      && projection.sourceRole === "non_actionable_evidence"
      && projection.exclusionReason === "unsupported"
    ))).toBe(true);
    expect(hasProjectionAction(hermes, "skills", "install")).toBe(true);
    expect(listCapabilityProjections(hermes, "skills").some((projection) => (
      projection.evidence.id === "shared-skills"
      && projection.sourceRole === "non_actionable_evidence"
      && projection.exclusionReason === "unknown"
      && projectionAllowsAction(projection, "diagnose")
      && !projectionAllowsAction(projection, "view")
    ))).toBe(true);
    expect(hasProjectionAction(hermes, "mcp", "save")).toBe(true);
    expect(hasProjectionAction(hermes, "ordinary_config", "save")).toBe(true);
    expect(getCertifiedGlobalRuleTarget(kiro)).toBe("");
    expect(getCertifiedSharedSkillDirectReadDefault(kiro)).toBe(false);
    expect(listCapabilityProjections(kiro, "rules").some((projection) => (
      projection.evidence.id === "global-steering"
      && projection.sourceRole === "native_file_source"
      && projectionAllowsAction(projection, "create")
      && !projectionAllowsAction(projection, "inject")
    ))).toBe(true);
    expect(hasProjectionAction(kiro, "skills", "install")).toBe(true);
    expect(listCapabilityProjections(kiro, "skills").some((projection) => (
      projection.evidence.id === "shared-skills"
      && projection.sourceRole === "non_actionable_evidence"
      && projection.exclusionReason === "unsupported"
    ))).toBe(true);
    expect(hasProjectionAction(kiro, "mcp", "save")).toBe(true);
    expect(hasProjectionAction(kiro, "ordinary_config", "save")).toBe(false);
    expect(getCertifiedGlobalRuleTarget(windsurf)).toBe("");
    expect(getCertifiedSharedSkillDirectReadDefault(windsurf)).toBe(true);
    expect(hasProjectionAction(windsurf, "rules", "read")).toBe(true);
    expect(hasProjectionAction(windsurf, "rules", "inject")).toBe(false);
    expect(hasProjectionAction(windsurf, "skills", "install")).toBe(true);
    expect(hasProjectionAction(windsurf, "mcp", "save")).toBe(true);
    expect(hasProjectionAction(windsurf, "ordinary_config", "save")).toBe(false);
    expect(getCertifiedGlobalRuleTarget(copilot)).toBe("/Users/visual/.copilot/copilot-instructions.md");
    expect(getCertifiedSharedSkillDirectReadDefault(copilot)).toBe(true);
    expect(hasProjectionAction(copilot, "rules", "inject")).toBe(true);
    expect(listCapabilityProjections(copilot, "rules").some((projection) => (
      projection.evidence.id === "rule-directory"
      && projection.sourceRole === "native_file_source"
      && projectionAllowsAction(projection, "create")
      && !projectionAllowsAction(projection, "inject")
    ))).toBe(true);
    expect(hasProjectionAction(copilot, "skills", "install")).toBe(true);
    expect(hasProjectionAction(copilot, "mcp", "save")).toBe(true);
    expect(hasProjectionAction(copilot, "ordinary_config", "save")).toBe(true);
    expect(codebuddy.capabilities.flatMap((capability) => capability.action_evidence || []).some((evidence) => "version" in evidence || "verified_at" in evidence)).toBe(false);
    expect(workbuddy.capabilities.flatMap((capability) => capability.action_evidence || []).some((evidence) => "version" in evidence || "verified_at" in evidence)).toBe(false);
    expect(hermes.capabilities.flatMap((capability) => capability.action_evidence || []).some((evidence) => "version" in evidence || "verified_at" in evidence)).toBe(false);
    expect(kiro.capabilities.flatMap((capability) => capability.action_evidence || []).some((evidence) => "version" in evidence || "verified_at" in evidence)).toBe(false);
    expect(windsurf.capabilities.flatMap((capability) => capability.action_evidence || []).some((evidence) => "version" in evidence || "verified_at" in evidence)).toBe(false);
    expect(copilot.capabilities.flatMap((capability) => capability.action_evidence || []).some((evidence) => "version" in evidence || "verified_at" in evidence)).toBe(false);
  });

  it("summarizes module actions from projections instead of raw writable fields", () => {
    expect(hasProjectionAction(tool, "rules", "inject")).toBe(true);
    expect(hasProjectionAction(tool, "skills", "delete")).toBe(false);
    expect(hasProjectionAction(tool, "mcp", "save")).toBe(false);
    expect(hasProjectionAction(tool, "ordinary_config", "save")).toBe(false);
  });

  it("treats JSONC as a supported editable config format", () => {
    const jsoncTool = {
      id: "opencode",
      capabilities: [
        {
          id: "mcp-config",
          kind: "mcp",
          scope: "global",
          access: "writable",
          format: "jsonc",
          source_path: "/Users/visual/.config/opencode/opencode.jsonc#mcp",
        },
        {
          id: "ordinary-config",
          kind: "ordinary_config",
          scope: "global",
          access: "writable",
          format: "jsonc",
          source_path: "/Users/visual/.config/opencode/opencode.jsonc",
        },
      ],
    };

    expect(hasProjectionAction(jsoncTool, "mcp", "save")).toBe(true);
    expect(hasProjectionAction(jsoncTool, "ordinary_config", "save")).toBe(true);
  });

  it("keeps read-only and readable states available for read paths", () => {
    expect(canReadCapability({ access: "writable" })).toBe(true);
    expect(canReadCapability({ access: "read_only" })).toBe(true);
    expect(canReadCapability({ access: "readable" })).toBe(true);
    expect(canReadCapability({ access: "unknown" })).toBe(false);
    expect(canReadCapability({ access: "unsupported" })).toBe(false);
  });

  it("preserves unknown and unsupported unavailable reasons for UI state", () => {
    expect(getCapabilityActionState({ access: "unknown" })).toEqual({
      canRead: false,
      canWrite: false,
      unavailableReason: "unknown",
    });
    expect(getCapabilityActionState({ access: "unsupported" })).toEqual({
      canRead: false,
      canWrite: false,
      unavailableReason: "unsupported",
    });
    expect(getCapabilityActionState(null)).toEqual({
      canRead: false,
      canWrite: false,
      unavailableReason: "unknown",
    });
  });

  it("can retain unsupported capabilities for diagnostics when requested", () => {
    expect(listToolCapabilities(tool, { kind: "ordinary_config" }).map((capability) => capability.id)).toEqual([
      "config",
      "unknown-config",
      "unsupported-config",
    ]);
  });

  it("identifies trusted non-file-editable global user rules separately from unresolved evidence", () => {
    const cursor = {
      id: "cursor",
      capabilities: [
        {
          id: "user-rules",
          kind: "rule",
          scope: "global",
          access: "unsupported",
          format: "unknown",
          source_path: "",
          source_confidence: "official_docs",
          notes: "Cursor User Rules are app-internal settings and do not expose a stable file-backed sync target.",
        },
      ],
    };
    const trae = {
      id: "trae",
      capabilities: [
        {
          id: "user-rules",
          kind: "rule",
          scope: "global",
          access: "unsupported",
          format: "unknown",
          source_path: "",
          source_confidence: "official_community",
          notes: "Trae global rules are not confirmed as a stable external file-backed target.",
        },
      ],
    };

    expect(hasNonFileEditableGlobalRuleModel(cursor)).toBe(true);
    expect(getNonFileEditableGlobalRuleCapability(cursor)?.id).toBe("user-rules");
    expect(hasNonFileEditableGlobalRuleModel(trae)).toBe(false);
    expect(getNonFileEditableGlobalRuleCapability(trae)).toBeNull();
  });

  it("summarizes effective Rules and Skills capability override sources", () => {
    const summary = summarizeEffectiveToolCapabilities(
      { ...tool, supports_generic_skills: true },
      {
        codex: {
          customGlobalRuleTarget: "/Users/visual/.codex/CUSTOM.md",
          sharedSkillDirectRead: false,
        },
      }
    );

    expect(getCertifiedGlobalRuleTarget(tool)).toBe("/Users/visual/.codex/AGENTS.md");
    expect(getCertifiedSharedSkillDirectReadDefault({ ...tool, supports_generic_skills: true })).toBe(true);
    expect(summary.rules).toMatchObject({
      certifiedDefaultTarget: "/Users/visual/.codex/AGENTS.md",
      effectiveTarget: "/Users/visual/.codex/CUSTOM.md",
      source: "user_override",
      missingGlobalRuleTarget: false,
    });
    expect(summary.skills).toMatchObject({
      certifiedDirectReadDefault: true,
      effectiveDirectRead: false,
      overrideValue: false,
      source: "user_override",
    });
  });

  it("summarizes directory-backed rule source overrides separately from Global Rule files", () => {
    const directoryTool = {
      id: "trae-cn",
      capabilities: [
        {
          id: "user-rules",
          kind: "rule",
          scope: "global",
          access: "writable",
          format: "directory",
          source_path: "/Users/visual/.trae-cn/user_rules",
          source_confidence: "certified_local_product_behavior",
          action_evidence: [
            { action: "read", supported: true, evidence: "verified" },
            { action: "create", supported: true, evidence: "verified" },
            { action: "save", supported: true, evidence: "verified" },
          ],
        },
      ],
    };

	    const state = summarizeRuleSourceState(directoryTool, {
	      "trae-cn": {
	        customRuleSourceType: "directory",
	        customRuleSourcePath: "/Users/visual/rules",
	        customGlobalRuleTarget: "/Users/visual/AGENTS.md",
	      },
	    });

    expect(state).toMatchObject({
	      sourceType: "directory",
	      sourcePath: "/Users/visual/rules",
	      globalRuleFile: "/Users/visual/AGENTS.md",
	      source: "user_override",
	      directoryTargetOutsideSource: false,
	    });
    expect(pathIsInsideDirectory("/Users/visual/rules/AGENTS.md", "/Users/visual/rules")).toBe(true);
    expect(pathIsInsideDirectory("/Users/visual/rules-outside/AGENTS.md", "/Users/visual/rules")).toBe(false);
  });

	  it("keeps directory-backed rule sources decoupled from independent Global Rule files", () => {
	    const overrides = {
	      codex: {
	        customRuleSourceType: "directory",
	        customRuleSourcePath: "/Users/visual/.codex/rules",
        customGlobalRuleTarget: "/Users/visual/.codex-outside/AGENTS.md",
      },
    };
    const state = summarizeRuleSourceState({ id: "codex", capabilities: [] }, overrides);
    const summary = summarizeEffectiveToolCapabilities({ id: "codex", capabilities: [] }, overrides);

    expect(state).toMatchObject({
	      sourceType: "directory",
	      sourcePath: "/Users/visual/.codex/rules",
	      globalRuleFile: "/Users/visual/.codex-outside/AGENTS.md",
	      directoryTargetOutsideSource: false,
	      missingGlobalRuleTarget: false,
	    });
	    expect(summary.rules.effectiveTarget).toBe("/Users/visual/.codex-outside/AGENTS.md");
	    expect(summary.rules.directoryTargetOutsideSource).toBe(false);
	  });

  it("treats OpenClaw shared Skill source as certified direct read", () => {
    const openclaw = {
      id: "openclaw",
      supports_generic_skills: true,
      capabilities: [
        {
          id: "dedicated-skills",
          kind: "skill",
          scope: "tool",
          access: "writable",
          format: "skill_directory",
          source_path: "/Users/visual/.openclaw/skills",
        },
        {
          id: "shared-skills",
          kind: "skill",
          scope: "shared",
          access: "read_only",
          format: "skill_directory",
          source_path: "/Users/visual/.agents/skills",
          source_confidence: "certified_local_product_behavior",
        },
      ],
    };

    const sharedProjection = listCapabilityProjections(openclaw, "skills")
      .find((projection) => projection.sourceRole === "shared_source");

    expect(sharedProjection?.exclusionReason).toBe(null);
    expect(projectionAllowsAction(sharedProjection, "view")).toBe(true);
    expect(getCertifiedSharedSkillDirectReadDefault(openclaw)).toBe(true);
  });

  it("keeps certified defaults distinct from user-configured effective capabilities", () => {
    const configuredTool = {
      id: "codex",
      certified_global_rule_target: "/Users/visual/.codex/AGENTS.md",
      certified_shared_skill_direct_read: true,
      supports_generic_skills: false,
      capabilities: [
        {
          id: "user-configured-global-rule-target",
          kind: "rule",
          scope: "global",
          access: "writable",
          format: "markdown",
          source_path: "/Users/visual/.codex/CUSTOM.md",
          source_confidence: "user_configured",
          action_evidence: [
            { action: "inject", supported: true, evidence: "user configured" },
          ],
        },
        {
          id: "shared-skills",
          kind: "skill",
          scope: "shared",
          access: "unsupported",
          format: "skill_directory",
          source_path: "/Users/visual/.agents/skills",
          source_confidence: "user_configured",
        },
      ],
    };

    const customProjection = listCapabilityProjections(configuredTool, "rules")
      .find((projection) => projection.evidence?.id === "user-configured-global-rule-target");

    expect(hasProjectionAction(configuredTool, "rules", "inject")).toBe(true);
    expect(projectionAllowsAction(customProjection, "inject")).toBe(true);
    expect(projectionAllowsAction(customProjection, "create")).toBe(false);
    expect(projectionAllowsAction(customProjection, "save")).toBe(false);
    expect(projectionAllowsAction(customProjection, "view")).toBe(false);
    expect(getCertifiedGlobalRuleTarget(configuredTool)).toBe("/Users/visual/.codex/AGENTS.md");
    expect(getCertifiedSharedSkillDirectReadDefault(configuredTool)).toBe(true);
  });

  it("keeps missing Global Rule targets explanatory without granting actions", () => {
    const directoryOnlyTool = {
      id: "trae",
      capabilities: [
        {
          id: "user-rules",
          kind: "rule",
          scope: "global",
          access: "readable",
          format: "directory",
          source_path: "/Users/visual/.trae/user_rules",
          source_confidence: "official_community",
          action_evidence: [
            { action: "read", supported: true, evidence: "verified" },
          ],
        },
      ],
    };

    const summary = summarizeEffectiveToolCapabilities(directoryOnlyTool, {});
    const projection = listCapabilityProjections(directoryOnlyTool, "rules")[0];

    expect(summary.rules).toMatchObject({
      effectiveTarget: "",
      source: "missing",
      missingGlobalRuleTarget: true,
    });
    expect(projectionAllowsAction(projection, "inject")).toBe(false);
  });

  it("keeps normal dual rule support quiet", () => {
    const decision = getRulePromptDecision(tool, {
      context: "tool_rules",
      hasDisplayableToolRules: true,
      hasToolRuleSourceRoot: true,
      hasToolRuleCreateOptions: true,
    });

    expect(decision).toEqual({ kind: "none", reason: "normal" });
    expect(getRulePromptDecision(tool, {
      context: "global_rule_settings",
      effectiveGlobalRuleTarget: "/Users/visual/.codex/AGENTS.md",
    })).toEqual({ kind: "none", reason: "normal" });
  });

  it("classifies single-file-only and directory-only rule contexts separately", () => {
    const singleFileTool = {
      id: "codex",
      capabilities: [
        {
          id: "global-rule",
          kind: "rule",
          scope: "global",
          access: "writable",
          format: "markdown",
          source_path: "/Users/visual/.codex/AGENTS.md",
          source_confidence: "official_docs",
        },
      ],
    };
    const directoryOnlyTool = {
      id: "trae-cn",
      capabilities: [
        {
          id: "user-rules",
          kind: "rule",
          scope: "global",
          access: "writable",
          format: "directory",
          source_path: "/Users/visual/.trae-cn/user_rules",
          source_confidence: "certified_local_product_behavior",
          action_evidence: [
            { action: "create", supported: true, evidence: "verified" },
            { action: "read", supported: true, evidence: "verified" },
          ],
        },
      ],
    };

    expect(getRulePromptDecision(singleFileTool, { context: "tool_rules" })).toMatchObject({
      kind: "context",
      reason: "single_global_rule_only",
    });
    expect(getRulePromptDecision(directoryOnlyTool, { context: "global_rule_settings" })).toMatchObject({
      kind: "context",
      reason: "global_rule_file_unsupported",
    });
    expect(getRulePromptDecision(directoryOnlyTool, {
      context: "tool_rules",
      hasToolRuleSourceRoot: true,
      hasToolRuleCreateOptions: true,
    })).toEqual({ kind: "none", reason: "normal" });
    expect(getRulePromptDecision(directoryOnlyTool, {
      context: "tool_rules",
      hasToolRuleSourceRoot: true,
      hasToolRuleCreateOptions: false,
    })).toEqual({ kind: "none", reason: "normal" });
  });

  it("classifies missing, unavailable, unknown, uncertified, and fileless rule prompt states", () => {
    const noRuleTool = { id: "empty", capabilities: [] };
    const unknownRuleTool = {
      id: "unknown",
      capabilities: [
        {
          id: "rules",
          kind: "rule",
          scope: "global",
          access: "unknown",
          format: "unknown",
          source_path: "",
        },
      ],
    };
    const uncertifiedRuleTool = {
      id: "community-only",
      capabilities: [
        {
          id: "rules",
          kind: "rule",
          scope: "global",
          access: "readable",
          format: "directory",
          source_path: "/Users/visual/.community/rules",
          source_confidence: "official_community",
        },
      ],
    };
    const filelessTool = {
      id: "cursor",
      capabilities: [
        {
          id: "user-rules",
          kind: "rule",
          scope: "global",
          access: "unsupported",
          format: "unknown",
          source_path: "",
          source_confidence: "official_docs",
          notes: "Cursor User Rules are app-internal settings and do not expose a stable file-backed sync target.",
        },
      ],
    };

    expect(getRulePromptDecision(noRuleTool, { context: "tool_rules" })).toMatchObject({
      kind: "exception",
      reason: "no_rule_source",
    });
    expect(getRulePromptDecision(noRuleTool, { context: "global_rule_settings" })).toMatchObject({
      kind: "exception",
      reason: "unsupported_rule_management",
    });
    expect(getRulePromptDecision(tool, {
      context: "global_rule_settings",
      hasUnavailableCustomGlobalRuleTarget: true,
      effectiveGlobalRuleTarget: "/Users/visual/.codex/CUSTOM.md",
    })).toMatchObject({
      kind: "exception",
      reason: "custom_global_rule_target_unavailable",
    });
    expect(getRulePromptDecision(unknownRuleTool, { context: "tool_rules" })).toMatchObject({
      kind: "exception",
      reason: "unsupported_rule_management",
    });
    expect(getRulePromptDecision(uncertifiedRuleTool, { context: "global_rule_settings" })).toMatchObject({
      kind: "exception",
      reason: "unsupported_rule_management",
    });
    expect(getRulePromptDecision(filelessTool, { context: "tool_rules" })).toMatchObject({
      kind: "exception",
      reason: "unsupported_rule_management",
    });
  });
});
