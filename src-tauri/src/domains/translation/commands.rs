// Purpose: Tauri command adapters for translation provider settings and Markdown translation.

use super::cache;
use super::provider::{
    normalize_config_input, normalize_provider, normalize_target_language, provider_state,
    test_provider as run_provider_test, translate_markdown_with_provider, validate_markdown,
    validate_provider_ready, TranslationProviderConfigInput, TranslationProviderState,
    TranslationResult,
};
use super::secrets;
use crate::platform::config as app_config;
use crate::platform::config::TranslationProviderConfig;
use crate::platform::logging::{self, LogCategory, LogEvent, LogLevel};
use std::path::Path;

#[tauri::command]
pub fn get_translation_provider_config() -> TranslationProviderState {
    let config = app_config::load_config();
    provider_state(
        &config.translation_provider,
        secrets::translation_api_key_configured(),
    )
}

#[tauri::command]
pub fn set_translation_provider_config(
    config: TranslationProviderConfigInput,
) -> Result<TranslationProviderState, String> {
    let normalized = normalize_config_input(config)?;
    let updated = app_config::update_config(|app_config| {
        app_config.translation_provider = normalized;
        Ok(())
    })?;
    Ok(provider_state(
        &updated.translation_provider,
        secrets::translation_api_key_configured(),
    ))
}

#[tauri::command]
pub fn set_translation_api_key(api_key: String) -> Result<TranslationProviderState, String> {
    secrets::save_translation_api_key(&api_key)?;
    Ok(get_translation_provider_config())
}

#[tauri::command]
pub fn clear_translation_api_key() -> Result<TranslationProviderState, String> {
    secrets::clear_translation_api_key()?;
    Ok(get_translation_provider_config())
}

#[tauri::command]
pub async fn test_translation_provider() -> Result<TranslationResult, String> {
    let config = app_config::load_config().translation_provider;
    let api_key = secrets::load_translation_api_key();
    run_provider_test(config, api_key).await
}

#[tauri::command]
pub async fn translate_markdown(
    markdown: String,
    target_language: String,
) -> Result<TranslationResult, String> {
    let config = app_config::load_config().translation_provider;
    translate_markdown_with_cache_dir(
        config,
        secrets::load_translation_api_key,
        &cache::translation_cache_dir(),
        markdown,
        target_language,
    )
    .await
}

async fn translate_markdown_with_cache_dir<F>(
    config: TranslationProviderConfig,
    load_api_key: F,
    cache_dir: &Path,
    markdown: String,
    target_language: String,
) -> Result<TranslationResult, String>
where
    F: FnOnce() -> String,
{
    let provider_for_log = translation_provider_log_value(&config);
    let target_for_log = normalize_target_language(&target_language).ok();
    let result = translate_markdown_with_cache_dir_unlogged(
        config,
        load_api_key,
        cache_dir,
        markdown,
        target_language,
    )
    .await;

    match &result {
        Ok(result) => write_translation_request_log(
            &provider_for_log,
            Some(&result.target_language),
            "ok",
            None,
        ),
        Err(error) => write_translation_request_log(
            &provider_for_log,
            target_for_log.as_deref(),
            "failed",
            Some(translation_error_class(error)),
        ),
    }

    result
}

async fn translate_markdown_with_cache_dir_unlogged<F>(
    config: TranslationProviderConfig,
    load_api_key: F,
    cache_dir: &Path,
    markdown: String,
    target_language: String,
) -> Result<TranslationResult, String>
where
    F: FnOnce() -> String,
{
    validate_provider_ready(&config, "cache-key", true)?;
    validate_markdown(&markdown)?;
    let normalized_target = normalize_target_language(&target_language)?;
    let cache_key = cache::translation_cache_key(&config, &markdown, &normalized_target);
    if let Some(result) = cache::read_translation_cache(cache_dir, &cache_key) {
        return Ok(result);
    }

    let result =
        translate_markdown_with_provider(config, load_api_key(), markdown, normalized_target)
            .await?;
    let _ = cache::write_translation_cache(cache_dir, &cache_key, &result);
    Ok(result)
}

fn write_translation_request_log(
    provider: &str,
    target_language: Option<&str>,
    result: &str,
    error_class: Option<&str>,
) {
    let _ = logging::write_log_event(translation_request_log_event(
        provider,
        target_language,
        result,
        error_class,
    ));
}

fn translation_request_log_event(
    provider: &str,
    target_language: Option<&str>,
    result: &str,
    error_class: Option<&str>,
) -> LogEvent {
    LogEvent {
        level: if result == "failed" {
            LogLevel::Warn
        } else {
            LogLevel::Info
        },
        category: LogCategory::Skills,
        action: "translation_translate_markdown".to_string(),
        result: Some(result.to_string()),
        message: None,
        tool_id: None,
        target_role: Some(provider.to_string()),
        target_path: target_language.map(|value| value.to_string()),
        error: error_class.map(|value| value.to_string()),
    }
}

fn translation_provider_log_value(config: &TranslationProviderConfig) -> String {
    match normalize_provider(&config.provider).as_str() {
        "openai-compatible" => "openai-compatible".to_string(),
        "anthropic-messages" => "anthropic-messages".to_string(),
        _ => "unsupported".to_string(),
    }
}

fn translation_error_class(error: &str) -> &'static str {
    if error.contains("disabled") {
        "provider_disabled"
    } else if error.contains("unsupported translation provider") {
        "unsupported_provider"
    } else if error.contains("base URL") {
        "invalid_base_url"
    } else if error.contains("model is not configured") {
        "missing_model"
    } else if error.contains("API key") {
        "missing_api_key"
    } else if error.contains("markdown content") {
        "invalid_markdown"
    } else if error.contains("unsupported target language") {
        "unsupported_target_language"
    } else if error.contains("provider request failed") {
        "provider_request_failed"
    } else if error.contains("returned HTTP") {
        "provider_http_status"
    } else if error.contains("unreadable response") {
        "provider_unreadable_response"
    } else if error.contains("empty content") {
        "provider_empty_response"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> TranslationProviderConfig {
        TranslationProviderConfig {
            enabled: true,
            provider: "openai-compatible".to_string(),
            base_url: "https://example.test/v1".to_string(),
            model: "model-a".to_string(),
        }
    }

    #[test]
    fn translate_markdown_uses_cache_before_loading_api_key() {
        let dir = tempfile::tempdir().unwrap();
        let markdown = "# Title".to_string();
        let target_language = "zh-CN".to_string();
        let key = cache::translation_cache_key(&config(), &markdown, &target_language);
        let cached = TranslationResult {
            content: "# 标题".to_string(),
            target_language: target_language.clone(),
        };
        cache::write_translation_cache(dir.path(), &key, &cached).unwrap();

        let result = tauri::async_runtime::block_on(translate_markdown_with_cache_dir(
            config(),
            || panic!("API key should not be loaded on cache hit"),
            dir.path(),
            markdown,
            target_language,
        ))
        .unwrap();

        assert_eq!(result, cached);
    }

    #[test]
    fn translate_markdown_cache_miss_requires_api_key() {
        let dir = tempfile::tempdir().unwrap();

        let result = tauri::async_runtime::block_on(translate_markdown_with_cache_dir(
            config(),
            || "".to_string(),
            dir.path(),
            "# Title".to_string(),
            "zh-CN".to_string(),
        ));

        assert_eq!(
            result.err(),
            Some("translation API key is not configured".to_string())
        );
    }

    #[test]
    fn translation_request_log_event_contains_metadata_only() {
        let event = translation_request_log_event(
            "openai-compatible",
            Some("zh-CN"),
            "failed",
            Some("provider_http_status"),
        );

        assert_eq!(event.category, LogCategory::Skills);
        assert_eq!(event.level, LogLevel::Warn);
        assert_eq!(event.action, "translation_translate_markdown");
        assert_eq!(event.result.as_deref(), Some("failed"));
        assert_eq!(event.target_role.as_deref(), Some("openai-compatible"));
        assert_eq!(event.target_path.as_deref(), Some("zh-CN"));
        assert_eq!(event.error.as_deref(), Some("provider_http_status"));
        assert!(event.message.is_none());
        assert!(event.tool_id.is_none());
    }

    #[test]
    fn translation_error_class_does_not_expose_raw_error() {
        assert_eq!(
            translation_error_class("translation provider returned HTTP 404"),
            "provider_http_status"
        );
        assert_eq!(
            translation_error_class("translation API key is not configured"),
            "missing_api_key"
        );
        assert_eq!(
            translation_error_class("translation provider returned unreadable secret payload"),
            "unknown"
        );
    }
}
