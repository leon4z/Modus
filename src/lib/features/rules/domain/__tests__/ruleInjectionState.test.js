import { describe, expect, it } from "vitest";

import {
  defaultRuleBaselineMatches,
  defaultRuleFingerprint,
  getCurrentCustomPendingTargetIds,
  getCustomPendingRuleIds,
  getRuleTargetIds,
  getStoredOrCurrentPendingTargetIds,
  persistCustomInjectedBaselinesForTargetsState,
  scopedDefaultRuleFingerprint,
} from "$lib/features/rules/domain/ruleInjectionState.js";

describe("rule injection state", () => {
  it("matches backend UTF-8 fingerprints for localized rule metadata", () => {
    expect(defaultRuleFingerprint({
      id: "common_rule",
      name: "全局规则",
      content: "123",
      inject_to: [],
    })).toBe("v1:e1cfc463");
  });

  it("clears pending targets after canonical tool injection", () => {
    const rules = [
      {
        id: "custom_rule",
        name: "Custom Rule",
        content: "content",
        inject_to: [],
        managed_targets: ["claude-code"],
      },
    ];
    const baselines = {
      common_rule: "",
      custom_rules: {},
      custom_rule_pending_targets: {
        custom_rule: ["claude-code"],
      },
    };

    const next = persistCustomInjectedBaselinesForTargetsState(
      rules,
      baselines,
      ["claude-code"],
      ["claude-code"],
    );

    expect(next.custom_rule_pending_targets).toEqual({});
    expect(next.custom_rules.custom_rule).toBe(defaultRuleFingerprint(rules[0]));
  });

  it("filters explicit rule targets to enabled tools", () => {
    const rule = {
      id: "custom_rule",
      name: "Custom Rule",
      content: "content",
      inject_to: ["codex", "disabled-tool"],
      managed_targets: ["codex", "disabled-tool"],
    };

    expect(getRuleTargetIds(rule, ["codex"])).toEqual(["codex"]);
  });

  it("accepts scoped fingerprints when disabled historical targets remain", () => {
    const rule = {
      id: "custom_rule",
      name: "Custom Rule",
      content: "content",
      inject_to: [],
      managed_targets: ["codex", "disabled-tool"],
    };
    const scoped = scopedDefaultRuleFingerprint(rule, ["codex"]);
    const baselines = {
      common_rule: "",
      custom_rules: {
        custom_rule: scoped,
      },
      custom_rule_pending_targets: {},
    };

    expect(scoped).not.toBe(defaultRuleFingerprint(rule));
    expect(defaultRuleBaselineMatches(rule, scoped, ["codex"])).toBe(true);
    expect(getCustomPendingRuleIds([rule], baselines, ["codex"])).toEqual([]);
  });

  it("ignores stored pending targets for disabled tools in current pending state", () => {
    const rules = [
      {
        id: "custom_rule",
        name: "Custom Rule",
        content: "content",
        inject_to: [],
        managed_targets: ["disabled-tool"],
      },
    ];
    const baselines = {
      common_rule: "",
      custom_rules: {
        custom_rule: "v1:old",
      },
      custom_rule_pending_targets: {
        custom_rule: ["disabled-tool"],
      },
    };

    expect(getStoredOrCurrentPendingTargetIds("custom_rule", rules[0], baselines, ["codex"])).toEqual([]);
    expect(getCurrentCustomPendingTargetIds(rules, baselines, ["codex"])).toEqual([]);
    expect(getCustomPendingRuleIds(rules, baselines, ["codex"])).toEqual([]);
  });

  it("preserves inactive stored pending targets while clearing injected enabled targets", () => {
    const rules = [
      {
        id: "custom_rule",
        name: "Custom Rule",
        content: "content",
        inject_to: [],
        managed_targets: ["codex", "disabled-tool"],
      },
    ];
    const baselines = {
      common_rule: "",
      custom_rules: {
        custom_rule: "v1:old",
      },
      custom_rule_pending_targets: {
        custom_rule: ["codex", "disabled-tool"],
      },
    };

    const next = persistCustomInjectedBaselinesForTargetsState(
      rules,
      baselines,
      ["codex"],
      ["codex"],
    );

    expect(next.custom_rule_pending_targets).toEqual({ custom_rule: ["disabled-tool"] });
    expect(next.custom_rules.custom_rule).toBe("v1:old");
  });
});
