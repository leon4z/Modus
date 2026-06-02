// Purpose: Persist General settings preferences through the shared configuration update boundary.

use super::*;

pub(crate) fn is_initialized_domain() -> bool {
    let config = app_config::load_config();
    config.initialized
}

pub(crate) fn get_language_domain() -> String {
    let config = app_config::load_config();
    config.language
}

fn normalize_language_preference(lang: &str) -> Result<String, String> {
    match lang {
        "system" | "zh" | "en" => Ok(lang.to_string()),
        _ => Err("unsupported language preference".to_string()),
    }
}

pub(crate) fn set_language_domain(lang: String) -> Result<(), String> {
    let normalized = normalize_language_preference(&lang)?;
    app_config::update_config(|config| {
        config.language = normalized;
        Ok(())
    })
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::normalize_language_preference;

    #[test]
    fn normalize_language_preference_accepts_supported_values() {
        assert_eq!(normalize_language_preference("system").unwrap(), "system");
        assert_eq!(normalize_language_preference("zh").unwrap(), "zh");
        assert_eq!(normalize_language_preference("en").unwrap(), "en");
    }

    #[test]
    fn normalize_language_preference_rejects_unknown_values() {
        assert_eq!(
            normalize_language_preference("fr").err(),
            Some("unsupported language preference".to_string())
        );
    }
}

pub(crate) fn get_theme_domain() -> String {
    let config = app_config::load_config();
    config.theme
}

pub(crate) fn set_theme_domain(theme: String) -> Result<(), String> {
    app_config::update_config(|config| {
        config.theme = theme;
        Ok(())
    })
    .map(|_| ())
}

pub(crate) fn get_skill_performance_diagnostics_enabled_domain() -> bool {
    get_module_performance_diagnostics_enabled_domain()
}

pub(crate) fn get_module_performance_diagnostics_enabled_domain() -> bool {
    let config = app_config::load_config();
    config
        .module_performance_diagnostics_enabled
        .unwrap_or(config.skill_performance_diagnostics_enabled)
}

pub(crate) fn set_skill_performance_diagnostics_enabled_domain(
    enabled: bool,
) -> Result<(), String> {
    set_module_performance_diagnostics_enabled_domain(enabled)
}

pub(crate) fn set_module_performance_diagnostics_enabled_domain(
    enabled: bool,
) -> Result<(), String> {
    app_config::update_config(|config| {
        config.module_performance_diagnostics_enabled = Some(enabled);
        config.skill_performance_diagnostics_enabled = enabled;
        Ok(())
    })
    .map(|_| ())
}
