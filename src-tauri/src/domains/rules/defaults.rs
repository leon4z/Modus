// Purpose: Own default rule and injection baseline persistence for the Rules domain.

use super::*;

pub(crate) fn list_default_rules_domain() -> Vec<app_config::DefaultRule> {
    let config = app_config::load_config();
    config
        .default_rules
        .into_iter()
        .filter(|rule| rule.id == "common_rule")
        .collect()
}

pub(crate) fn get_default_rule_injection_baselines_domain(
) -> app_config::DefaultRuleInjectionBaselines {
    let config = app_config::load_config();
    config.default_rule_injection_baselines
}

pub(crate) fn set_default_rule_injection_baselines_domain(
    baselines: app_config::DefaultRuleInjectionBaselines,
) -> Result<(), String> {
    app_config::update_config(|config| {
        config.default_rule_injection_baselines = baselines;
        Ok(())
    })
    .map(|_| ())
}

pub(crate) fn save_default_rule_domain(rule: app_config::DefaultRule) -> Result<(), String> {
    if rule.id != "common_rule" {
        return Err("Custom Rules are not available".to_string());
    }
    app_config::update_config(|config| {
        if let Some(existing) = config.default_rules.iter_mut().find(|r| r.id == rule.id) {
            let mut next = rule;
            if next.managed_targets.is_none() {
                next.managed_targets = existing.managed_targets.clone();
            }
            *existing = next;
        } else {
            config.default_rules.push(rule);
        }
        Ok(())
    })
    .map(|_| ())
}

pub(crate) fn delete_default_rule_domain(rule_id: String) -> Result<(), String> {
    if rule_id != "common_rule" {
        return Err("Custom Rules are not available".to_string());
    }
    app_config::update_config(|config| {
        config.default_rules.retain(|r| r.id != rule_id);
        Ok(())
    })
    .map(|_| ())
}
