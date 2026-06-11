// Purpose: Build and execute configured Markdown translation provider requests.

use crate::platform::config::TranslationProviderConfig;
use serde::{Deserialize, Serialize};

const MAX_MARKDOWN_CHARS: usize = 120_000;
const REQUEST_TIMEOUT_SECONDS: u64 = 90;
const ANTHROPIC_MAX_TOKENS: u32 = 8192;
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslationProviderState {
    pub enabled: bool,
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub api_key_configured: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationProviderConfigInput {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslationResult {
    pub content: String,
    pub target_language: String,
}

#[derive(Debug, Serialize, PartialEq)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Debug, Serialize, PartialEq)]
struct AnthropicMessagesRequest {
    model: String,
    system: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize, PartialEq)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, PartialEq)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessagesResponse {
    content: Vec<AnthropicContentBlock>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    content_type: Option<String>,
    text: Option<String>,
}

pub(super) fn provider_state(
    config: &TranslationProviderConfig,
    api_key_configured: bool,
) -> TranslationProviderState {
    TranslationProviderState {
        enabled: config.enabled,
        provider: normalize_provider(&config.provider),
        base_url: normalize_base_url_or_default(&config.base_url),
        model: config.model.trim().to_string(),
        api_key_configured,
    }
}

pub(super) fn normalize_config_input(
    input: TranslationProviderConfigInput,
) -> Result<TranslationProviderConfig, String> {
    let provider = normalize_provider(&input.provider);
    if !is_supported_provider(&provider) {
        return Err("unsupported translation provider".to_string());
    }
    let base_url = normalize_base_url_or_default(&input.base_url);
    validate_base_url(&base_url)?;
    Ok(TranslationProviderConfig {
        enabled: input.enabled,
        provider,
        base_url,
        model: input.model.trim().to_string(),
    })
}

pub(super) fn normalize_target_language(value: &str) -> Result<String, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "zh" | "zh-cn" | "chinese" | "简体中文" | "中文" => Ok("zh-CN".to_string()),
        "en" | "en-us" | "english" | "英文" => Ok("en".to_string()),
        _ => Err("unsupported target language".to_string()),
    }
}

pub(super) fn validate_markdown(markdown: &str) -> Result<(), String> {
    if markdown.trim().is_empty() {
        return Err("markdown content is empty".to_string());
    }
    if markdown.chars().count() > MAX_MARKDOWN_CHARS {
        return Err("markdown content is too large to translate".to_string());
    }
    Ok(())
}

pub(super) fn validate_provider_ready(
    config: &TranslationProviderConfig,
    api_key: &str,
    require_enabled: bool,
) -> Result<(), String> {
    if require_enabled && !config.enabled {
        return Err("translation provider is disabled".to_string());
    }
    if !is_supported_provider(&normalize_provider(&config.provider)) {
        return Err("unsupported translation provider".to_string());
    }
    validate_base_url(&normalize_base_url_or_default(&config.base_url))?;
    if config.model.trim().is_empty() {
        return Err("translation model is not configured".to_string());
    }
    if api_key.trim().is_empty() {
        return Err("translation API key is not configured".to_string());
    }
    Ok(())
}

pub(super) async fn translate_markdown_with_provider(
    config: TranslationProviderConfig,
    api_key: String,
    markdown: String,
    target_language: String,
) -> Result<TranslationResult, String> {
    validate_provider_ready(&config, &api_key, true)?;
    validate_markdown(&markdown)?;
    let normalized_target = normalize_target_language(&target_language)?;
    let provider = normalize_provider(&config.provider);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
        .build()
        .map_err(|_| "translation client could not be created".to_string())?;
    let content = if provider == "anthropic-messages" {
        let request = build_anthropic_messages_request(&config, &markdown, &normalized_target)?;
        let url = anthropic_messages_url(&config.base_url)?;
        let response = client
            .post(url)
            .bearer_auth(api_key.trim())
            .header("x-api-key", api_key.trim())
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&request)
            .send()
            .await
            .map_err(|_| "translation provider request failed".to_string())?;
        parse_anthropic_messages_response(response).await?
    } else {
        let request = build_chat_request(&config, &markdown, &normalized_target)?;
        let url = chat_completions_url(&config.base_url)?;
        let response = client
            .post(url)
            .bearer_auth(api_key.trim())
            .json(&request)
            .send()
            .await
            .map_err(|_| "translation provider request failed".to_string())?;
        parse_chat_response(response).await?
    };
    if content.is_empty() {
        return Err("translation provider returned empty content".to_string());
    }
    Ok(TranslationResult {
        content,
        target_language: normalized_target,
    })
}

async fn parse_chat_response(response: reqwest::Response) -> Result<String, String> {
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "translation provider returned HTTP {}",
            status.as_u16()
        ));
    }
    let body = response
        .json::<ChatResponse>()
        .await
        .map_err(|_| "translation provider returned an unreadable response".to_string())?;
    Ok(body
        .choices
        .into_iter()
        .find_map(|choice| choice.message.content)
        .unwrap_or_default()
        .trim()
        .to_string())
}

async fn parse_anthropic_messages_response(response: reqwest::Response) -> Result<String, String> {
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "translation provider returned HTTP {}",
            status.as_u16()
        ));
    }
    let body = response
        .json::<AnthropicMessagesResponse>()
        .await
        .map_err(|_| "translation provider returned an unreadable response".to_string())?;
    Ok(body
        .content
        .into_iter()
        .filter(|block| block.content_type.as_deref().unwrap_or("text") == "text")
        .filter_map(|block| block.text)
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string())
}

pub(super) async fn test_provider(
    config: TranslationProviderConfig,
    api_key: String,
) -> Result<TranslationResult, String> {
    validate_provider_ready(&config, &api_key, false)?;
    let mut enabled_config = config;
    enabled_config.enabled = true;
    translate_markdown_with_provider(
        enabled_config,
        api_key,
        "Hello".to_string(),
        "zh-CN".to_string(),
    )
    .await
}

pub(super) fn normalize_provider(value: &str) -> String {
    let provider = value.trim().to_ascii_lowercase();
    match provider.as_str() {
        "" => "openai-compatible".to_string(),
        "cc-router" | "anthropic" | "anthropic-compatible" => "anthropic-messages".to_string(),
        _ => provider,
    }
}

fn is_supported_provider(provider: &str) -> bool {
    matches!(provider, "openai-compatible" | "anthropic-messages")
}

pub(super) fn normalize_base_url_or_default(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        "https://api.openai.com/v1".to_string()
    } else {
        trimmed.to_string()
    }
}

fn validate_base_url(value: &str) -> Result<(), String> {
    let parsed =
        reqwest::Url::parse(value).map_err(|_| "translation base URL is invalid".to_string())?;
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        _ => Err("translation base URL must use http or https".to_string()),
    }
}

fn chat_completions_url(base_url: &str) -> Result<String, String> {
    validate_base_url(base_url)?;
    Ok(format!(
        "{}/chat/completions",
        normalize_base_url_or_default(base_url)
    ))
}

fn anthropic_messages_url(base_url: &str) -> Result<String, String> {
    validate_base_url(base_url)?;
    let base = normalize_base_url_or_default(base_url);
    if base.ends_with("/v1") {
        Ok(format!("{base}/messages"))
    } else {
        Ok(format!("{base}/v1/messages"))
    }
}

fn translation_system_prompt() -> String {
    [
        "You translate Markdown skill documentation.",
        "Preserve Markdown structure exactly where possible.",
        "Do not translate fenced code blocks, inline code, file paths, command names, URLs, frontmatter keys, or YAML/TOML/JSON keys.",
        "Translate natural-language prose, headings, table text, and list text into the requested target language.",
        "Return only translated Markdown, with no commentary.",
    ]
    .join(" ")
}

fn translation_user_prompt(markdown: &str, target_language: &str) -> String {
    format!(
        "Target language: {}\n\nMarkdown:\n{}",
        target_language, markdown
    )
}

fn build_chat_request(
    config: &TranslationProviderConfig,
    markdown: &str,
    target_language: &str,
) -> Result<ChatRequest, String> {
    if config.model.trim().is_empty() {
        return Err("translation model is not configured".to_string());
    }
    Ok(ChatRequest {
        model: config.model.trim().to_string(),
        temperature: 0.2,
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: translation_system_prompt(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: translation_user_prompt(markdown, target_language),
            },
        ],
    })
}

fn build_anthropic_messages_request(
    config: &TranslationProviderConfig,
    markdown: &str,
    target_language: &str,
) -> Result<AnthropicMessagesRequest, String> {
    if config.model.trim().is_empty() {
        return Err("translation model is not configured".to_string());
    }
    Ok(AnthropicMessagesRequest {
        model: config.model.trim().to_string(),
        system: translation_system_prompt(),
        max_tokens: ANTHROPIC_MAX_TOKENS,
        temperature: 0.2,
        messages: vec![AnthropicMessage {
            role: "user".to_string(),
            content: translation_user_prompt(markdown, target_language),
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> TranslationProviderConfig {
        TranslationProviderConfig {
            enabled: true,
            provider: "openai-compatible".to_string(),
            base_url: "https://example.test/v1/".to_string(),
            model: "model-a".to_string(),
        }
    }

    #[test]
    fn normalizes_config_without_storing_secret() {
        let normalized = normalize_config_input(TranslationProviderConfigInput {
            enabled: true,
            provider: "".to_string(),
            base_url: " https://example.test/v1/ ".to_string(),
            model: " model-a ".to_string(),
        })
        .unwrap();

        assert_eq!(normalized.provider, "openai-compatible");
        assert_eq!(normalized.base_url, "https://example.test/v1");
        assert_eq!(normalized.model, "model-a");
    }

    #[test]
    fn rejects_unsupported_provider() {
        let result = normalize_config_input(TranslationProviderConfigInput {
            enabled: true,
            provider: "unknown-provider".to_string(),
            base_url: "https://example.test".to_string(),
            model: "model-a".to_string(),
        });

        assert_eq!(
            result.err(),
            Some("unsupported translation provider".to_string())
        );
    }

    #[test]
    fn normalizes_cc_router_provider_alias() {
        let normalized = normalize_config_input(TranslationProviderConfigInput {
            enabled: true,
            provider: "cc-router".to_string(),
            base_url: " http://127.0.0.1:23456 ".to_string(),
            model: " model-opus ".to_string(),
        })
        .unwrap();

        assert_eq!(normalized.provider, "anthropic-messages");
        assert_eq!(normalized.base_url, "http://127.0.0.1:23456");
        assert_eq!(normalized.model, "model-opus");
    }

    #[test]
    fn anthropic_messages_url_accepts_root_or_v1_base() {
        assert_eq!(
            anthropic_messages_url("http://127.0.0.1:23456").unwrap(),
            "http://127.0.0.1:23456/v1/messages"
        );
        assert_eq!(
            anthropic_messages_url("https://api.anthropic.com/v1").unwrap(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn normalizes_explicit_target_language() {
        assert_eq!(normalize_target_language("中文").unwrap(), "zh-CN");
        assert_eq!(normalize_target_language("en").unwrap(), "en");
        assert!(normalize_target_language("fr").is_err());
    }

    #[test]
    fn request_instructs_model_to_preserve_markdown_structure() {
        let request = build_chat_request(&config(), "# Title\n\n`code`\n", "zh-CN").unwrap();
        let system = &request.messages[0].content;

        assert!(system.contains("Preserve Markdown structure"));
        assert!(system.contains("Do not translate fenced code blocks"));
        assert!(request.messages[1]
            .content
            .contains("Target language: zh-CN"));
        assert!(request.messages[1].content.contains("# Title"));
    }

    #[test]
    fn anthropic_request_uses_system_prompt_and_user_message() {
        let mut config = config();
        config.provider = "anthropic-messages".to_string();
        config.base_url = "http://127.0.0.1:23456".to_string();

        let request =
            build_anthropic_messages_request(&config, "# Title\n\n`code`\n", "zh-CN").unwrap();

        assert_eq!(request.model, "model-a");
        assert_eq!(request.max_tokens, ANTHROPIC_MAX_TOKENS);
        assert!(request.system.contains("Preserve Markdown structure"));
        assert!(request
            .system
            .contains("Do not translate fenced code blocks"));
        assert_eq!(request.messages[0].role, "user");
        assert!(request.messages[0]
            .content
            .contains("Target language: zh-CN"));
        assert!(request.messages[0].content.contains("# Title"));
    }

    #[test]
    fn provider_ready_requires_enabled_for_real_translation() {
        let mut disabled = config();
        disabled.enabled = false;

        assert_eq!(
            validate_provider_ready(&disabled, "key", true).err(),
            Some("translation provider is disabled".to_string())
        );
        assert!(validate_provider_ready(&disabled, "key", false).is_ok());
    }

    #[test]
    fn translation_http_client_can_be_built() {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
            .build()
            .expect("translation HTTP client should build with configured TLS features");
    }
}
