// Purpose: Shared frontend interpretation helpers for tool capability metadata.

const readableAccessStates = new Set(["writable", "read_only", "readable"]);
const trustedRuleConfidence = new Set(["official_docs", "official_repository", "user_configured"]);
const trustedFilelessRuleConfidence = new Set(["official_docs", "official_repository"]);
const supportedTextConfigFormats = new Set(["json", "jsonc", "toml", "yaml"]);
const ruleFormats = new Set(["markdown", "instructions_markdown", "mdc", "directory"]);

/** @typedef {"rules" | "skills" | "mcp" | "ordinary_config"} CapabilityModule */

/** @type {Record<CapabilityModule, string>} */
const moduleKind = {
  rules: "rule",
  skills: "skill",
  mcp: "mcp",
  ordinary_config: "ordinary_config",
};

/**
 * @param {any} tool
 * @returns {any[]}
 */
export function getToolCapabilities(tool) {
  return Array.isArray(tool?.capabilities) ? tool.capabilities : [];
}

/**
 * @param {any} capability
 */
export function canReadCapability(capability) {
  return readableAccessStates.has(capability?.access);
}

/**
 * @param {any} capability
 */
export function canWriteCapability(capability) {
  return capability?.access === "writable";
}

/**
 * @param {any} capability
 */
function canDiagnoseCapability(capability) {
  return Boolean(capability) && capability.access !== "unsupported";
}

/**
 * @param {string} toolId
 * @param {any} capability
 */
function isTrustedGlobalRuleProjection(toolId, capability) {
  return trustedRuleConfidence.has(capability?.source_confidence)
    || (capability?.source_confidence === "certified_local_product_behavior" && hasVerifiedAction(capability, "inject"))
    || (toolId === "openclaw" && capability?.id === "trusted-workspace-agents");
}

/**
 * @param {string} module
 * @param {string} sourceRole
 * @param {any} capability
 * @param {string | null} exclusionReason
 * @param {string[]} actions
 */
function buildProjection(module, sourceRole, capability, exclusionReason, actions = []) {
  return {
    module,
    sourceRole,
    actions,
    exclusionReason,
    evidence: capability,
  };
}

/**
 * @param {any} capability
 */
function evidenceActions(capability) {
  const explicit = explicitSupportedActions(capability);
  if (explicit) return explicit;
  if (!canDiagnoseCapability(capability)) return [];
  return canReadCapability(capability) ? ["view", "read", "diagnose"] : ["diagnose"];
}

/**
 * @param {any} capability
 */
function ruleSourceActions(capability) {
  const explicit = explicitSupportedActions(capability);
  if (explicit) return explicit;
  const actions = evidenceActions(capability);
  if (canWriteCapability(capability)) {
    for (const action of ["create", "edit", "save", "delete"]) {
      if (!actions.includes(action)) actions.push(action);
    }
  }
  return actions;
}

/**
 * @param {any} capability
 */
function ruleGlobalTargetActions(capability) {
  const actions = ruleSourceActions(capability);
  if (!actions.includes("inject")) actions.push("inject");
  return actions;
}

/**
 * @param {any} capability
 */
function ruleNativeSourceActions(capability) {
  return ruleSourceActions(capability).filter((action) => action !== "inject");
}

/**
 * @param {string[]} actions
 * @param {string} action
 */
function pushActionWithReadAliases(actions, action) {
  if (action === "view" || action === "read") {
    for (const alias of ["view", "read"]) {
      if (!actions.includes(alias)) actions.push(alias);
    }
    return;
  }
  if (!actions.includes(action)) actions.push(action);
}

/**
 * @param {any} capability
 * @returns {string[] | null}
 */
function explicitSupportedActions(capability) {
  if (!Array.isArray(capability?.action_evidence) || capability.action_evidence.length === 0) return null;
  /** @type {string[]} */
  const actions = [];
  for (const evidence of capability.action_evidence) {
    if (evidence?.supported === false || !evidence?.action) continue;
    pushActionWithReadAliases(actions, evidence.action);
  }
  return actions;
}

/**
 * @param {any} capability
 * @param {string} action
 */
function hasVerifiedAction(capability, action) {
  const actions = explicitSupportedActions(capability);
  if (!actions) return false;
  if (action === "view" || action === "read") {
    return actions.includes("view") || actions.includes("read");
  }
  return actions.includes(action);
}

/**
 * @param {any} capability
 */
function hasCompoundSupportingSources(capability) {
  return Array.isArray(capability?.supporting_sources) && capability.supporting_sources.length > 0;
}

/**
 * @param {any} capability
 */
function hasVerifiedManagedSkillAction(capability) {
  return ["install", "copy", "uninstall", "delete", "update", "repair", "sync"].some((action) => hasVerifiedAction(capability, action));
}

/**
 * @param {any} capability
 * @param {string} sourceRole
 */
function skillToolDirectoryActions(capability, sourceRole) {
  const actions = explicitSupportedActions(capability) || [
    "view",
    "read",
    "diagnose",
    "install",
    "copy",
    "uninstall",
    "delete",
    "update",
    "repair",
    "link",
    "sync",
  ];
  if (sourceRole === "tool_directory" && canWriteCapability(capability)) {
    for (const action of ["install", "link", "uninstall"]) {
      if (!actions.includes(action)) actions.push(action);
    }
  }
  return actions;
}

/**
 * @param {string} raw
 */
function stripSourcePath(raw) {
  return String(raw || "").split("#")[0].trim();
}

/**
 * @param {string} path
 */
function baseName(path) {
  return stripSourcePath(path).replace(/\/+$/, "").split("/").filter(Boolean).pop() || "";
}

/**
 * @param {any} capability
 */
function sourceLooksLikeDirectory(capability) {
  const name = baseName(capability?.source_path || "");
  return capability?.format === "directory"
    || capability?.format === "instructions_markdown"
    || capability?.format === "skill_directory"
    || !name
    || !name.includes(".");
}

/**
 * @param {any} capability
 */
function hasExactDeclaredSourceFile(capability) {
  const source = stripSourcePath(capability?.source_path || "");
  return Boolean(source)
    && !source.includes("*")
    && !sourceLooksLikeDirectory(capability);
}

/**
 * @param {string} module
 * @param {any} capability
 */
function wrongKindProjection(module, capability) {
  return buildProjection(module, "non_actionable_evidence", capability, "wrong_kind", []);
}

/**
 * @param {string} toolId
 * @param {string} module
 * @param {any} capability
 */
export function projectCapability(toolId, module, capability) {
  const expectedKind = Object.prototype.hasOwnProperty.call(moduleKind, module)
    ? moduleKind[/** @type {CapabilityModule} */ (module)]
    : null;
  if (!expectedKind || !capability || capability.kind !== expectedKind) {
    return wrongKindProjection(module, capability);
  }
  if (module === "rules") return projectRuleCapability(toolId, capability);
  if (module === "skills") return projectSkillCapability(capability);
  if (module === "mcp") return projectMcpCapability(capability);
  if (module === "ordinary_config") return projectOrdinaryConfigCapability(capability);
  return wrongKindProjection(module, capability);
}

/**
 * @param {any} tool
 * @param {string} module
 */
export function listCapabilityProjections(tool, module) {
  return getToolCapabilities(tool)
    .map((capability) => projectCapability(tool?.id || "", module, capability))
    .filter((projection) => projection.exclusionReason !== "wrong_kind");
}

/**
 * @param {any} projection
 * @param {string} action
 */
export function projectionAllowsAction(projection, action) {
  return Array.isArray(projection?.actions) && projection.actions.includes(action);
}

/**
 * @param {any} tool
 * @param {string} module
 * @param {string} action
 */
export function hasProjectionAction(tool, module, action) {
  return listCapabilityProjections(tool, module).some((projection) => projectionAllowsAction(projection, action));
}

/**
 * @param {Record<string, any>} overrides
 * @param {string} toolId
 */
export function getToolCapabilityOverride(overrides, toolId) {
  if (!overrides || !toolId) return null;
  return overrides[toolId] || overrides[toolId.replaceAll("-", "_")] || overrides[toolId.replaceAll("_", "-")] || null;
}

/**
 * @param {any} tool
 */
export function getCertifiedGlobalRuleTarget(tool) {
  const declaredDefault = tool?.certified_global_rule_target ?? tool?.certifiedGlobalRuleTarget;
  if (typeof declaredDefault === "string" && declaredDefault.trim()) return declaredDefault.trim();
  const projection = listCapabilityProjections(tool, "rules").find((candidate) => (
    candidate.sourceRole === "global_target"
    && !candidate.exclusionReason
    && candidate.evidence?.source_confidence !== "user_configured"
    && candidate.evidence?.format !== "directory"
    && Boolean(candidate.evidence?.source_path)
  ));
  return projection?.evidence?.source_path || "";
}

/**
 * @param {any} tool
 */
export function getCertifiedSharedSkillDirectReadDefault(tool) {
  const declaredDefault = tool?.certified_shared_skill_direct_read ?? tool?.certifiedSharedSkillDirectRead;
  if (typeof declaredDefault === "boolean") return declaredDefault;
  const skillProjections = listCapabilityProjections(tool, "skills")
    .filter((projection) => projection.evidence?.source_confidence !== "user_configured");
  const hasDirectReadProjection = skillProjections.some((projection) => (
    projection.sourceRole === "shared_source"
    && !projection.exclusionReason
    && projectionAllowsAction(projection, "view")
  ));
  if (hasDirectReadProjection) return true;
  if (typeof tool?.supports_generic_skills === "boolean") return tool.supports_generic_skills;
  if (typeof tool?.supportsGenericSkills === "boolean") return tool.supportsGenericSkills;
  return false;
}

/**
 * @param {string | null | undefined} value
 */
function normalizedOptionalPath(value) {
  const trimmed = String(value || "").trim();
  return trimmed || "";
}

/**
 * @param {string} path
 */
function normalizedComparablePath(path) {
  const raw = stripSourcePath(path).replace(/\/+$/, "");
  const absolute = raw.startsWith("/");
  /** @type {string[]} */
  const parts = [];
  for (const part of raw.split("/")) {
    if (!part || part === ".") continue;
    if (part === "..") {
      parts.pop();
    } else {
      parts.push(part);
    }
  }
  return `${absolute ? "/" : ""}${parts.join("/")}`;
}

/**
 * @param {string} path
 * @param {string} directory
 */
export function pathIsInsideDirectory(path, directory) {
  const target = normalizedComparablePath(path);
  const root = normalizedComparablePath(directory);
  return Boolean(target && root && target !== root && target.startsWith(`${root}/`));
}

/**
 * @param {any} tool
 */
function getUserConfiguredGlobalRuleTarget(tool) {
  const projection = listCapabilityProjections(tool, "rules").find((candidate) => (
    candidate.sourceRole === "global_target"
    && !candidate.exclusionReason
    && candidate.evidence?.source_confidence === "user_configured"
    && Boolean(candidate.evidence?.source_path)
  ));
  return projection?.evidence?.source_path || "";
}

/**
 * @param {any} tool
 */
function getCertifiedRuleDirectory(tool) {
  const nativeProjection = listCapabilityProjections(tool, "rules").find((candidate) => (
    candidate.sourceRole === "native_file_source"
    && !candidate.exclusionReason
    && candidate.evidence?.source_confidence !== "user_configured"
    && Boolean(candidate.evidence?.source_path)
    && projectionIsDirectorySource(candidate)
  ));
  if (nativeProjection) {
    return {
      type: "directory",
      path: stripSourcePath(nativeProjection.evidence?.source_path || ""),
      projection: nativeProjection,
    };
  }
  return null;
}

/**
 * @param {any} tool
 * @param {Record<string, any>} overrides
 */
export function summarizeRuleSourceState(tool, overrides = {}) {
  const override = getToolCapabilityOverride(overrides, tool?.id || "") || {};
  const customRuleSourcePath = normalizedOptionalPath(
    override.customRuleSourcePath ?? override.custom_rule_source_path
  );
  const overrideGlobalRuleFile = normalizedOptionalPath(
    override.customGlobalRuleTarget ?? override.custom_global_rule_target
  ) || getUserConfiguredGlobalRuleTarget(tool);
  const certifiedRuleSource = getCertifiedRuleDirectory(tool);
  const certifiedGlobalRuleFile = getCertifiedGlobalRuleTarget(tool);
  const sourceType = customRuleSourcePath || certifiedRuleSource?.path ? "directory" : "";
  const sourcePath = customRuleSourcePath || certifiedRuleSource?.path || "";
  const globalRuleFile = overrideGlobalRuleFile || certifiedGlobalRuleFile;
  const hasUserOverride = Boolean(customRuleSourcePath || overrideGlobalRuleFile);

  return {
    supported: Boolean(sourcePath || certifiedGlobalRuleFile),
    sourceType,
    sourcePath,
    globalRuleFile,
    overrideRuleSourcePath: customRuleSourcePath,
    overrideRuleSourceType: customRuleSourcePath ? "directory" : "",
    overrideGlobalRuleFile,
    certifiedRuleSourcePath: certifiedRuleSource?.path || "",
    certifiedRuleSourceType: certifiedRuleSource?.type || "",
    certifiedGlobalRuleFile,
    source: hasUserOverride ? "user_override" : sourcePath || certifiedGlobalRuleFile ? "certified_default" : "missing",
    missingGlobalRuleTarget: !globalRuleFile,
    directoryTargetOutsideSource: false,
  };
}

/**
 * @param {any} tool
 */
function getUserConfiguredSharedSkillDirectRead(tool) {
  const projection = listCapabilityProjections(tool, "skills").find((candidate) => (
    candidate.evidence?.source_confidence === "user_configured"
    && candidate.evidence?.kind === "skill"
    && candidate.evidence?.scope === "shared"
  ));
  if (!projection) return null;
  return projection.sourceRole === "shared_source"
    && !projection.exclusionReason
    && projectionAllowsAction(projection, "view");
}

/**
 * @param {any} tool
 * @param {Record<string, any>} overrides
 */
export function summarizeEffectiveToolCapabilities(tool, overrides = {}) {
  const override = getToolCapabilityOverride(overrides, tool?.id || "") || {};
  const ruleState = summarizeRuleSourceState(tool, overrides);
  const overrideTarget = ruleState.overrideGlobalRuleFile;
  const certifiedDefaultTarget = ruleState.certifiedGlobalRuleFile;
  const effectiveTarget = ruleState.globalRuleFile || certifiedDefaultTarget;
  const explicitDirectReadOverride = override.sharedSkillDirectRead ?? override.shared_skill_direct_read;
  const inferredDirectReadOverride = getUserConfiguredSharedSkillDirectRead(tool);
  const hasDirectReadOverride = typeof explicitDirectReadOverride === "boolean"
    || typeof inferredDirectReadOverride === "boolean";
  const certifiedDirectReadDefault = getCertifiedSharedSkillDirectReadDefault(tool);
  const effectiveDirectRead = hasDirectReadOverride
    ? Boolean(typeof explicitDirectReadOverride === "boolean" ? explicitDirectReadOverride : inferredDirectReadOverride)
    : certifiedDirectReadDefault;

  return {
    rules: {
      certifiedDefaultTarget,
      effectiveTarget,
      overrideTarget,
      source: ruleState.source,
      missingGlobalRuleTarget: !effectiveTarget,
      ruleSourceType: ruleState.sourceType,
      ruleSourcePath: ruleState.sourcePath,
      directoryTargetOutsideSource: ruleState.directoryTargetOutsideSource,
    },
    skills: {
      certifiedDirectReadDefault,
      effectiveDirectRead,
      overrideValue: hasDirectReadOverride ? effectiveDirectRead : null,
      source: hasDirectReadOverride ? "user_override" : "certified_default",
    },
  };
}

/**
 * @param {any} capability
 */
function isTrustedNonFileEditableGlobalRule(capability) {
  if (!capability) return false;
  const notes = String(capability.notes || "").toLowerCase();
  return capability.kind === "rule"
    && capability.scope === "global"
    && capability.access === "unsupported"
    && !capability.source_path
    && trustedFilelessRuleConfidence.has(capability.source_confidence)
    && (
      notes.includes("app-internal")
      || notes.includes("file-backed sync target")
      || notes.includes("file-synchronizable")
    );
}

/**
 * @param {any} tool
 */
export function getNonFileEditableGlobalRuleCapability(tool) {
  const projection = listCapabilityProjections(tool, "rules").find((candidate) => (
    candidate.exclusionReason === "unsupported"
    && isTrustedNonFileEditableGlobalRule(candidate.evidence)
  ));
  return projection?.evidence || null;
}

/**
 * @param {any} tool
 */
export function hasNonFileEditableGlobalRuleModel(tool) {
  return Boolean(getNonFileEditableGlobalRuleCapability(tool));
}

/**
 * @typedef {"tool_rules" | "global_rule_settings" | "global_rules"} RulePromptContext
 * @typedef {"none" | "context" | "exception"} RulePromptKind
 * @typedef {{
 *   context?: RulePromptContext,
 *   hasDisplayableToolRules?: boolean,
 *   hasToolRuleSourceRoot?: boolean,
 *   hasToolRuleCreateOptions?: boolean,
 *   hasMissingGlobalRuleFile?: boolean,
 *   hasUnavailableCustomGlobalRuleTarget?: boolean,
 *   effectiveGlobalRuleTarget?: string,
 * }} RulePromptDecisionInput
 * @typedef {{
 *   kind: RulePromptKind,
 *   reason: string,
 *   capability?: any
 * }} RulePromptDecision
 */

/** @type {RulePromptDecision} */
const noRulePrompt = { kind: "none", reason: "normal" };

/**
 * @param {any} capability
 */
function capabilityPath(capability) {
  return stripSourcePath(capability?.source_path || "");
}

/**
 * @param {any} projection
 */
function projectionHasRuleSourcePath(projection) {
  const sourcePath = capabilityPath(projection?.evidence);
  return Boolean(sourcePath) && !String(projection?.evidence?.source_path || "").includes("*");
}

/**
 * @param {any} projection
 */
function projectionIsDirectorySource(projection) {
  return projection?.evidence?.format === "directory" || sourceLooksLikeDirectory(projection?.evidence);
}

/**
 * @param {any} projection
 */
function projectionHasTrustedRuleManagementEvidence(projection) {
  const capability = projection?.evidence;
  if (!capability) return false;
  if (projection.sourceRole === "global_target") return true;
  if (capability.source_confidence === "user_configured") return true;
  if (trustedRuleConfidence.has(capability.source_confidence)) return true;
  if (capability.source_confidence === "certified_local_product_behavior") {
    return ["view", "read", "create", "edit", "save", "delete"].some((action) => hasVerifiedAction(capability, action));
  }
  return false;
}

/**
 * @param {any} tool
 * @param {RulePromptDecisionInput} input
 * @returns {RulePromptDecision}
 */
export function getRulePromptDecision(tool, input = {}) {
  if (!tool) return { kind: "exception", reason: "tool_missing" };
  const context = input.context || "tool_rules";
  const projections = listCapabilityProjections(tool, "rules");
  const normalSources = projections.filter((projection) => (
    !projection.exclusionReason
    && (projection.sourceRole === "global_target" || projection.sourceRole === "native_file_source")
    && !(projection.sourceRole === "global_target" && projection.evidence?.source_confidence === "user_configured")
    && projectionHasRuleSourcePath(projection)
    && projectionHasTrustedRuleManagementEvidence(projection)
  ));
  const hasSingleFileSource = normalSources.some((projection) => !projectionIsDirectorySource(projection));
  const hasDirectorySource = normalSources.some(projectionIsDirectorySource);
  const filelessCapability = getNonFileEditableGlobalRuleCapability(tool);
  const unknownProjection = projections.find((projection) => projection.exclusionReason === "unknown");
  const unsupportedProjection = projections.find((projection) => (
    projection.exclusionReason === "unsupported"
    && projection.evidence !== filelessCapability
  ));
  const unsupportedFormatProjection = projections.find((projection) => projection.exclusionReason === "unsupported_format");
  const hasEffectiveTargetInput = Object.prototype.hasOwnProperty.call(input, "effectiveGlobalRuleTarget");
  const effectiveTarget = hasEffectiveTargetInput
    ? normalizedOptionalPath(input.effectiveGlobalRuleTarget)
    : summarizeEffectiveToolCapabilities(tool, {}).rules.effectiveTarget;

  if (context === "global_rule_settings" || context === "global_rules") {
    if (input.hasUnavailableCustomGlobalRuleTarget) {
      return { kind: "exception", reason: "custom_global_rule_target_unavailable" };
    }
    if (effectiveTarget) return noRulePrompt;
    if (filelessCapability) return { kind: "exception", reason: "unsupported_rule_management", capability: filelessCapability };
    if (hasDirectorySource && !hasSingleFileSource) return { kind: "context", reason: "global_rule_file_unsupported" };
    if (unknownProjection) return { kind: "exception", reason: "unsupported_rule_management", capability: unknownProjection.evidence };
    if (unsupportedFormatProjection) return { kind: "exception", reason: "unsupported_rule_format", capability: unsupportedFormatProjection.evidence };
    if (unsupportedProjection) return { kind: "exception", reason: "unsupported_rule_management", capability: unsupportedProjection.evidence };
    return { kind: "exception", reason: "unsupported_rule_management" };
  }

  if (input.hasDisplayableToolRules) return noRulePrompt;
  if (input.hasMissingGlobalRuleFile) return { kind: "exception", reason: "global_rule_file_missing" };
  if (input.hasUnavailableCustomGlobalRuleTarget) {
    return { kind: "exception", reason: "custom_global_rule_target_unavailable" };
  }
  if (input.hasToolRuleCreateOptions || input.hasToolRuleSourceRoot) return noRulePrompt;
  if (filelessCapability) return { kind: "exception", reason: "unsupported_rule_management", capability: filelessCapability };
  if (hasSingleFileSource && !hasDirectorySource) return { kind: "context", reason: "single_global_rule_only" };
  if (unknownProjection) return { kind: "exception", reason: "unsupported_rule_management", capability: unknownProjection.evidence };
  if (unsupportedFormatProjection) return { kind: "exception", reason: "unsupported_rule_format", capability: unsupportedFormatProjection.evidence };
  if (unsupportedProjection) return { kind: "exception", reason: "unsupported_rule_management", capability: unsupportedProjection.evidence };
  return { kind: "exception", reason: "no_rule_source" };
}

/**
 * @param {string} toolId
 * @param {any} capability
 */
function projectRuleCapability(toolId, capability) {
  if (capability.access === "unsupported") {
    return buildProjection("rules", "non_actionable_evidence", capability, "unsupported", []);
  }
  if (capability.access === "unknown") {
    return buildProjection("rules", "non_actionable_evidence", capability, "unknown", evidenceActions(capability));
  }
  if (capability.scope === "project") {
    return buildProjection("rules", "project_source", capability, "project_scoped", evidenceActions(capability));
  }
  if (!ruleFormats.has(capability.format)) {
    return buildProjection("rules", "non_actionable_evidence", capability, "unsupported_format", evidenceActions(capability));
  }
  if (!stripSourcePath(capability.source_path) || String(capability.source_path || "").includes("*")) {
    return buildProjection("rules", "non_actionable_evidence", capability, "missing_path", evidenceActions(capability));
  }
  const explicitActions = explicitSupportedActions(capability);
  const actions = ruleSourceActions(capability);
  if (explicitActions) {
    const hasRuleWriteAction = actions.some((action) => ["create", "edit", "save", "delete", "inject"].includes(action));
    if (!hasRuleWriteAction) {
      return buildProjection("rules", "native_file_source", capability, "action_not_verified", explicitActions);
    }
    if (!hasVerifiedAction(capability, "inject")) {
      return buildProjection("rules", "native_file_source", capability, null, explicitActions);
    }
  }
  if (
    canWriteCapability(capability)
    && hasExactDeclaredSourceFile(capability)
    && isTrustedGlobalRuleProjection(toolId, capability)
    && (!explicitActions || hasVerifiedAction(capability, "inject"))
  ) {
    return buildProjection("rules", "global_target", capability, null, ruleGlobalTargetActions(capability));
  }
  return buildProjection("rules", "native_file_source", capability, null, ruleNativeSourceActions(capability));
}

/**
 * @param {any} capability
 */
function projectSkillCapability(capability) {
  if (capability.access === "unsupported") {
    return buildProjection("skills", "non_actionable_evidence", capability, "unsupported", []);
  }
  if (capability.access === "unknown") {
    return buildProjection("skills", "non_actionable_evidence", capability, "unknown", evidenceActions(capability));
  }
  if (capability.scope === "project") {
    return buildProjection("skills", "non_actionable_evidence", capability, "project_scoped", evidenceActions(capability));
  }
  if (capability.format !== "skill_directory") {
    return buildProjection("skills", "non_actionable_evidence", capability, "unsupported_format", evidenceActions(capability));
  }
  if (!stripSourcePath(capability.source_path)) {
    return buildProjection("skills", "non_actionable_evidence", capability, "missing_path", evidenceActions(capability));
  }
  if (capability.scope === "shared") {
    return buildProjection("skills", "shared_source", capability, null, evidenceActions(capability));
  }
  const sourceRole = hasCompoundSupportingSources(capability) ? "compound_source" : "tool_directory";
  if (capability.scope === "tool" && hasCompoundSupportingSources(capability) && !hasVerifiedManagedSkillAction(capability)) {
    return buildProjection("skills", sourceRole, capability, "compound_source_requires_verified_action", evidenceActions(capability));
  }
  if (capability.scope === "tool" && (canWriteCapability(capability) || hasVerifiedManagedSkillAction(capability))) {
    return buildProjection("skills", sourceRole, capability, null, skillToolDirectoryActions(capability, sourceRole));
  }
  return buildProjection("skills", sourceRole, capability, "read_only", evidenceActions(capability));
}

/**
 * @param {any} capability
 */
function projectMcpCapability(capability) {
  if (capability.access === "unsupported") {
    return buildProjection("mcp", "non_actionable_evidence", capability, "unsupported", []);
  }
  if (capability.access === "unknown") {
    return buildProjection("mcp", "non_actionable_evidence", capability, "unknown", evidenceActions(capability));
  }
  if (capability.scope === "project") {
    return buildProjection("mcp", "project_config", capability, "project_scoped", evidenceActions(capability));
  }
  if (!supportedTextConfigFormats.has(capability.format)) {
    return buildProjection("mcp", "global_config", capability, "unsupported_format", evidenceActions(capability));
  }
  if (canWriteCapability(capability) || hasVerifiedAction(capability, "save")) {
    return buildProjection("mcp", "global_config", capability, null, explicitSupportedActions(capability) || ["view", "read", "diagnose", "edit", "save", "sync"]);
  }
  return buildProjection("mcp", "global_config", capability, "read_only", evidenceActions(capability));
}

/**
 * @param {any} capability
 */
function projectOrdinaryConfigCapability(capability) {
  if (capability.source_kind === "primary_config_directory") {
    return buildProjection("ordinary_config", "non_actionable_evidence", capability, "out_of_scope", evidenceActions(capability));
  }
  if (capability.access === "unsupported") {
    return buildProjection("ordinary_config", "non_actionable_evidence", capability, "unsupported", []);
  }
  if (capability.access === "unknown") {
    return buildProjection("ordinary_config", "non_actionable_evidence", capability, "unknown", evidenceActions(capability));
  }
  if (capability.scope === "project") {
    return buildProjection("ordinary_config", "non_actionable_evidence", capability, "project_scoped", evidenceActions(capability));
  }
  if (!supportedTextConfigFormats.has(capability.format)) {
    return buildProjection("ordinary_config", "config_file", capability, "unsupported_format", evidenceActions(capability));
  }
  if (canWriteCapability(capability) || hasVerifiedAction(capability, "save")) {
    return buildProjection("ordinary_config", "config_file", capability, null, explicitSupportedActions(capability) || ["view", "read", "diagnose", "edit", "save", "sync"]);
  }
  return buildProjection("ordinary_config", "config_file", capability, "read_only", evidenceActions(capability));
}

/**
 * @param {any} tool
 * @param {{ kind?: string, scope?: string, includeUnsupported?: boolean }} options
 */
export function listToolCapabilities(tool, options = {}) {
  const includeUnsupported = options.includeUnsupported !== false;
  return getToolCapabilities(tool).filter((capability) => {
    if (options.kind && capability.kind !== options.kind) return false;
    if (options.scope && capability.scope !== options.scope) return false;
    if (!includeUnsupported && capability.access === "unsupported") return false;
    return true;
  });
}

/**
 * Ordinary configuration files are listed separately from Rules, Skills, and MCP.
 *
 * @param {any} tool
 */
export function listOrdinaryConfigCapabilities(tool) {
  return listCapabilityProjections(tool, "ordinary_config")
    .filter((projection) => !["unsupported", "out_of_scope"].includes(String(projection.exclusionReason || "")))
    .map((projection) => projection.evidence);
}

/**
 * @param {any} tool
 * @param {string} kind
 */
export function hasWritableCapability(tool, kind) {
  const module = {
    rule: "rules",
    skill: "skills",
    mcp: "mcp",
    ordinary_config: "ordinary_config",
  }[kind];
  const action = {
    rule: "inject",
    skill: "install",
    mcp: "save",
    ordinary_config: "save",
  }[kind];
  if (!module || !action) {
    return listToolCapabilities(tool, { kind, includeUnsupported: false }).some(canWriteCapability);
  }
  return hasProjectionAction(tool, module, action);
}

/**
 * @param {any} capability
 */
export function getCapabilityActionState(capability) {
  if (!capability) {
    return { canRead: false, canWrite: false, unavailableReason: "unknown" };
  }
  if (capability.access === "unsupported") {
    return { canRead: false, canWrite: false, unavailableReason: "unsupported" };
  }
  if (capability.access === "unknown") {
    return { canRead: false, canWrite: false, unavailableReason: "unknown" };
  }
  return {
    canRead: canReadCapability(capability),
    canWrite: canWriteCapability(capability),
    unavailableReason: null,
  };
}
