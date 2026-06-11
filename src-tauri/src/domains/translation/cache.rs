// Purpose: App-owned cache for model-backed Markdown translation results.

use super::provider::{normalize_base_url_or_default, normalize_provider, TranslationResult};
use crate::platform::{config::TranslationProviderConfig, env};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const CACHE_VERSION: &str = "skill-markdown-translation-v1";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranslationCacheEntry {
    version: String,
    content: String,
    target_language: String,
}

pub(super) fn translation_cache_dir() -> PathBuf {
    env::data_dir().join("translation-cache")
}

pub(super) fn translation_cache_key(
    config: &TranslationProviderConfig,
    markdown: &str,
    target_language: &str,
) -> String {
    let provider = normalize_provider(&config.provider);
    let base_url = normalize_base_url_or_default(&config.base_url);
    let mut hasher = Sha256::new();
    for part in [
        CACHE_VERSION,
        provider.as_str(),
        base_url.as_str(),
        config.model.trim(),
        target_language.trim(),
        markdown,
    ] {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    hex_lower(&hasher.finalize())
}

pub(super) fn read_translation_cache(
    cache_dir: &Path,
    cache_key: &str,
) -> Option<TranslationResult> {
    let path = cache_path(cache_dir, cache_key);
    let raw = fs::read_to_string(path).ok()?;
    let entry = serde_json::from_str::<TranslationCacheEntry>(&raw).ok()?;
    if entry.version != CACHE_VERSION || entry.content.trim().is_empty() {
        return None;
    }
    Some(TranslationResult {
        content: entry.content,
        target_language: entry.target_language,
    })
}

pub(super) fn write_translation_cache(
    cache_dir: &Path,
    cache_key: &str,
    result: &TranslationResult,
) -> Result<(), String> {
    fs::create_dir_all(cache_dir)
        .map_err(|_| "translation cache directory could not be created".to_string())?;
    let entry = TranslationCacheEntry {
        version: CACHE_VERSION.to_string(),
        content: result.content.clone(),
        target_language: result.target_language.clone(),
    };
    let raw = serde_json::to_vec_pretty(&entry)
        .map_err(|_| "translation cache entry could not be encoded".to_string())?;
    fs::write(cache_path(cache_dir, cache_key), raw)
        .map_err(|_| "translation cache entry could not be written".to_string())
}

fn cache_path(cache_dir: &Path, cache_key: &str) -> PathBuf {
    cache_dir.join(format!("{cache_key}.json"))
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
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
    fn translation_cache_roundtrips_result() {
        let dir = tempfile::tempdir().unwrap();
        let key = translation_cache_key(&config(), "# Title", "zh-CN");
        let result = TranslationResult {
            content: "# 标题".to_string(),
            target_language: "zh-CN".to_string(),
        };

        write_translation_cache(dir.path(), &key, &result).unwrap();

        assert_eq!(read_translation_cache(dir.path(), &key), Some(result));
    }

    #[test]
    fn translation_cache_key_changes_with_content_target_and_model() {
        let base = config();
        let mut other_model = base.clone();
        other_model.model = "model-b".to_string();

        let key = translation_cache_key(&base, "# Title", "zh-CN");

        assert_ne!(key, translation_cache_key(&base, "# Different", "zh-CN"));
        assert_ne!(key, translation_cache_key(&base, "# Title", "en"));
        assert_ne!(key, translation_cache_key(&other_model, "# Title", "zh-CN"));
    }
}
