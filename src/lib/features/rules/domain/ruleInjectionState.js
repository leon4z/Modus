// Shared rule-injection state helpers used by the UI and scenario runner.

/**
 * @param {string} value
 */
export function hashString(value) {
  let hash = 2166136261;
  for (const byte of new TextEncoder().encode(value)) {
    hash ^= byte;
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

/**
 * @param {any} rule
 */
export function defaultRuleFingerprint(rule) {
  const injectTo = Array.isArray(rule?.inject_to) ? [...rule.inject_to].sort() : [];
  const managedTargets = Array.isArray(rule?.managed_targets) ? [...rule.managed_targets].sort() : null;
  return `v1:${hashString(JSON.stringify({
    id: rule?.id || "",
    name: rule?.name || "",
    content: rule?.content || "",
    inject_to: injectTo,
    managed_targets: managedTargets,
  }))}`;
}

/**
 * @param {any} rule
 * @param {string[]} allToolIds
 */
export function scopedDefaultRuleFingerprint(rule, allToolIds) {
  if (!rule) return "";
  if (Array.isArray(rule.managed_targets)) {
    return defaultRuleFingerprint({
      ...rule,
      managed_targets: getRuleTargetIds(rule, allToolIds),
    });
  }
  if (Array.isArray(rule.inject_to) && rule.inject_to.length > 0) {
    return defaultRuleFingerprint({
      ...rule,
      inject_to: getRuleTargetIds(rule, allToolIds),
    });
  }
  return defaultRuleFingerprint(rule);
}

/**
 * @param {any} rule
 * @param {string | undefined | null} baseline
 * @param {string[]} [allToolIds]
 */
export function defaultRuleBaselineMatches(rule, baseline, allToolIds) {
  if (!rule || !baseline) return false;
  if (baseline === defaultRuleFingerprint(rule)) return true;
  if (Array.isArray(allToolIds)) {
    return baseline === scopedDefaultRuleFingerprint(rule, allToolIds);
  }
  return false;
}

/**
 * @param {any} raw
 */
export function normalizeDefaultRuleBaselines(raw) {
  const custom = raw?.custom_rules || raw?.customRules || {};
  const pendingTargets = raw?.custom_rule_pending_targets || raw?.customRulePendingTargets || {};
  return {
    common_rule: raw?.common_rule || raw?.commonRule || "",
    custom_rules: { ...custom },
    custom_rule_pending_targets: Object.fromEntries(
      Object.entries(pendingTargets).map(([id, targets]) => [
        id,
        Array.isArray(targets) ? [...new Set(targets.map(String))] : [],
      ]),
    ),
  };
}

/**
 * @param {any[]} rules
 */
export function buildDefaultRuleBaselines(rules) {
  const common = rules.find((rule) => rule.id === "common_rule");
  /** @type {Record<string,string>} */
  const custom = {};
  for (const rule of rules.filter((item) => item.id !== "common_rule")) {
    custom[rule.id] = defaultRuleFingerprint(rule);
  }
  return {
    common_rule: common ? defaultRuleFingerprint(common) : "",
    custom_rules: custom,
    custom_rule_pending_targets: {},
  };
}

/**
 * @param {any} baselines
 */
export function hasAnyDefaultRuleBaseline(baselines) {
  const normalized = normalizeDefaultRuleBaselines(baselines);
  return Boolean(normalized.common_rule)
    || Object.keys(normalized.custom_rules || {}).length > 0
    || Object.keys(normalized.custom_rule_pending_targets || {}).length > 0;
}

/**
 * @param {any[] | undefined | null} ids
 */
function uniqueStringIds(ids) {
  return [...new Set((ids || []).map(String).filter(Boolean))];
}

/**
 * @param {any[] | undefined | null} ids
 * @param {string[]} allToolIds
 */
function filterToActiveToolIds(ids, allToolIds) {
  const activeToolIds = new Set(uniqueStringIds(allToolIds));
  return uniqueStringIds(ids).filter((id) => activeToolIds.has(id));
}

/**
 * @param {any} rule
 * @param {string[]} allToolIds
 */
export function getRuleTargetIds(rule, allToolIds) {
  if (rule && Array.isArray(rule.managed_targets)) {
    return filterToActiveToolIds(rule.managed_targets, allToolIds);
  }
  if (rule && Array.isArray(rule.inject_to) && rule.inject_to.length > 0) {
    return filterToActiveToolIds(rule.inject_to, allToolIds);
  }
  return uniqueStringIds(allToolIds);
}

/**
 * @param {any[]} rules
 * @param {string[]} allToolIds
 */
export function getMergedRuleTargetIds(rules, allToolIds) {
  const ids = new Set();
  for (const rule of rules) {
    for (const id of getRuleTargetIds(rule, allToolIds)) ids.add(id);
  }
  return Array.from(ids);
}

/**
 * @param {any[]} rules
 * @param {any} baselines
 * @param {string[]} [allToolIds]
 */
export function getCustomPendingRuleIds(rules, baselines, allToolIds) {
  const normalized = normalizeDefaultRuleBaselines(baselines);
  const customRules = rules.filter((rule) => rule.id !== "common_rule");
  const currentIds = new Set(customRules.map((rule) => rule.id));
  const pending = new Set();
  const filterByActiveScope = Array.isArray(allToolIds);
  for (const rule of customRules) {
    if (!defaultRuleBaselineMatches(rule, normalized.custom_rules?.[rule.id], allToolIds)) {
      if (!filterByActiveScope || getStoredOrCurrentPendingTargetIds(rule.id, rule, normalized, allToolIds).length > 0) {
        pending.add(rule.id);
      }
    }
  }
  for (const id of Object.keys(normalized.custom_rules || {})) {
    if (!currentIds.has(id)) {
      const storedTargets = normalized.custom_rule_pending_targets?.[id];
      if (!filterByActiveScope || !Array.isArray(storedTargets) || filterToActiveToolIds(storedTargets, allToolIds).length > 0) {
        pending.add(id);
      }
    }
  }
  return Array.from(pending);
}

/**
 * @param {string} ruleId
 * @param {any} rule
 * @param {any} baselines
 * @param {string[]} allToolIds
 */
export function getStoredOrCurrentPendingTargetIds(ruleId, rule, baselines, allToolIds) {
  const normalized = normalizeDefaultRuleBaselines(baselines);
  const stored = normalized.custom_rule_pending_targets?.[ruleId];
  if (Array.isArray(stored) && stored.length > 0) {
    return filterToActiveToolIds(stored, allToolIds);
  }
  if (rule) return getRuleTargetIds(rule, allToolIds);
  return uniqueStringIds(allToolIds);
}

/**
 * @param {any} baselines
 * @param {string} ruleId
 * @param {string[]} targetIds
 */
export function markCustomRulePendingTargetsState(baselines, ruleId, targetIds) {
  const normalized = normalizeDefaultRuleBaselines(baselines);
  const existing = normalized.custom_rule_pending_targets?.[ruleId] || [];
  return {
    common_rule: normalized.common_rule,
    custom_rules: normalized.custom_rules,
    custom_rule_pending_targets: {
      ...normalized.custom_rule_pending_targets,
      [ruleId]: [...new Set([...existing, ...targetIds.map(String)])],
    },
  };
}

/**
 * @param {any[]} rules
 * @param {any} baselines
 * @param {{ includeCommon?: boolean, includeCustom?: boolean }} options
 */
export function persistInjectedBaselinesState(rules, baselines, options) {
  const normalized = normalizeDefaultRuleBaselines(baselines);
  const next = buildDefaultRuleBaselines(rules);
  return {
    common_rule: options.includeCommon ? next.common_rule : normalized.common_rule,
    custom_rules: options.includeCustom ? next.custom_rules : normalized.custom_rules,
    custom_rule_pending_targets: options.includeCustom ? {} : normalized.custom_rule_pending_targets,
  };
}

/**
 * @param {any[]} rules
 * @param {any} baselines
 * @param {string[]} allToolIds
 */
export function getCurrentCustomPendingTargetIds(rules, baselines, allToolIds) {
  const normalized = normalizeDefaultRuleBaselines(baselines);
  const pendingIds = new Set(getCustomPendingRuleIds(rules, normalized, allToolIds));
  const byId = new Map(
    rules
      .filter((rule) => rule.id !== "common_rule")
      .map((rule) => [rule.id, rule]),
  );
  const ids = new Set();
  for (const id of pendingIds) {
    const rule = byId.get(id);
    const targetIds = getStoredOrCurrentPendingTargetIds(id, rule, normalized, allToolIds);
    for (const toolId of targetIds) ids.add(toolId);
  }
  return Array.from(ids);
}

/**
 * @param {any[]} rules
 * @param {any} baselines
 * @param {string[]} allToolIds
 * @param {string[]} targetIds
 */
export function persistCustomInjectedBaselinesForTargetsState(rules, baselines, allToolIds, targetIds) {
  const normalized = normalizeDefaultRuleBaselines(baselines);
  const injectedTargets = new Set(targetIds.map(String));
  const activeToolIds = new Set(uniqueStringIds(allToolIds));
  const next = buildDefaultRuleBaselines(rules);
  const currentRules = new Map(
    rules
      .filter((rule) => rule.id !== "common_rule")
      .map((rule) => [rule.id, rule]),
  );
  const pendingIds = new Set([
    ...getCustomPendingRuleIds(rules, normalized, allToolIds),
    ...Object.keys(normalized.custom_rule_pending_targets || {}),
  ]);
  const customRules = { ...normalized.custom_rules };
  const pendingTargets = { ...normalized.custom_rule_pending_targets };

  for (const id of pendingIds) {
    const rule = currentRules.get(id);
    const storedTargets = normalized.custom_rule_pending_targets?.[id];
    const inactiveStoredTargets = Array.isArray(storedTargets)
      ? uniqueStringIds(storedTargets).filter((toolId) => !activeToolIds.has(toolId))
      : [];
    const requiredTargets = getStoredOrCurrentPendingTargetIds(id, rule, normalized, allToolIds);
    if (requiredTargets.length === 0) {
      if (inactiveStoredTargets.length > 0) {
        pendingTargets[id] = inactiveStoredTargets;
      }
      continue;
    }
    const remainingTargets = requiredTargets.filter((toolId) => !injectedTargets.has(toolId));
    const nextPendingTargets = [...new Set([...remainingTargets, ...inactiveStoredTargets])];
    if (nextPendingTargets.length === 0) {
      if (rule) {
        customRules[id] = next.custom_rules[id];
      } else {
        delete customRules[id];
      }
      delete pendingTargets[id];
    } else {
      pendingTargets[id] = nextPendingTargets;
    }
  }

  return {
    common_rule: normalized.common_rule,
    custom_rules: customRules,
    custom_rule_pending_targets: pendingTargets,
  };
}
