// Purpose: Store translation provider secrets outside normal app configuration.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

static SECRETS_WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct AppSecrets {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    translation_api_key: String,
}

fn secrets_path() -> std::path::PathBuf {
    crate::platform::env::data_dir().join("secrets.json")
}

fn load_secrets_from(path: &Path) -> AppSecrets {
    let Ok(content) = fs::read_to_string(path) else {
        return AppSecrets::default();
    };
    serde_json::from_str::<AppSecrets>(&content).unwrap_or_default()
}

fn save_secrets_to(path: &Path, secrets: &AppSecrets) -> Result<(), String> {
    let _guard = SECRETS_WRITE_LOCK
        .lock()
        .map_err(|_| "secrets write lock poisoned".to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(secrets).map_err(|e| e.to_string())?;
    let temp_path = path.with_extension("json.tmp");
    let write_result = (|| -> Result<(), String> {
        let mut file = fs::File::create(&temp_path).map_err(|e| e.to_string())?;
        file.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600))
                .map_err(|e| e.to_string())?;
        }
        fs::rename(&temp_path, path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

pub(super) fn translation_api_key_configured() -> bool {
    !load_translation_api_key().trim().is_empty()
}

pub(super) fn load_translation_api_key() -> String {
    load_translation_api_key_from(&secrets_path())
}

pub(super) fn save_translation_api_key(api_key: &str) -> Result<(), String> {
    save_translation_api_key_to(&secrets_path(), api_key)
}

pub(super) fn clear_translation_api_key() -> Result<(), String> {
    save_translation_api_key("")
}

fn load_translation_api_key_from(path: &Path) -> String {
    load_secrets_from(path).translation_api_key
}

fn save_translation_api_key_to(path: &Path, api_key: &str) -> Result<(), String> {
    let mut secrets = load_secrets_from(path);
    secrets.translation_api_key = api_key.trim().to_string();
    save_secrets_to(path, &secrets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_key_roundtrips_through_separate_secret_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("secrets.json");

        save_translation_api_key_to(&path, "  secret-value  ").unwrap();

        assert_eq!(load_translation_api_key_from(&path), "secret-value");
        let raw = fs::read_to_string(path).unwrap();
        assert!(raw.contains("translationApiKey"));
    }

    #[test]
    fn clearing_translation_key_removes_configured_value() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("secrets.json");

        save_translation_api_key_to(&path, "secret-value").unwrap();
        save_translation_api_key_to(&path, "").unwrap();

        assert_eq!(load_translation_api_key_from(&path), "");
    }
}
