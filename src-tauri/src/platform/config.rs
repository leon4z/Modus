// Purpose: Persist app configuration and provide shared filesystem helpers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static CONFIG_WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub skill_notes: HashMap<String, String>,
    #[serde(default)]
    pub default_rules: Vec<DefaultRule>,
    #[serde(default)]
    pub default_rule_injection_baselines: DefaultRuleInjectionBaselines,
    #[serde(default)]
    pub injection_targets: HashMap<String, String>,
    #[serde(default)]
    pub tool_paths: HashMap<String, ToolPaths>,
    #[serde(default)]
    pub custom_tools: Vec<CustomTool>,
    #[serde(default)]
    pub managed_tools: Vec<String>, // tool IDs the user chose to manage
    #[serde(default)]
    pub handled_new_tool_ids: Vec<String>, // detected tools already handled in post-onboarding prompts
    #[serde(default)]
    pub initialized: bool, // true after first-run onboarding
    #[serde(default)]
    pub skills_page_visited: bool, // true after first entering skills page
    #[serde(default)]
    pub skill_local_inventory: SkillLocalInventoryState,
    #[serde(default)]
    pub skill_keep_local_choices: HashMap<String, Vec<SkillLocalSourceRecord>>,
    #[serde(default)]
    pub tool_capability_overrides: HashMap<String, ToolCapabilityOverrides>,
    #[serde(default = "default_language")]
    pub language: String, // "system", "zh", or "en"
    #[serde(default = "default_theme")]
    pub theme: String, // "light", "dark", or "system"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_performance_diagnostics_enabled: Option<bool>,
    #[serde(default)]
    pub skill_performance_diagnostics_enabled: bool,
    #[serde(default)]
    pub app_update_state: AppUpdateState,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_startup_check_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_successful_check_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_failure_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_failure_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_update: Option<AppAvailableUpdate>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppAvailableUpdate {
    pub version: String,
    pub current_version: String,
    pub channel: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillLocalInventoryState {
    #[serde(default)]
    pub initialized: bool,
    #[serde(default)]
    pub fingerprint: String,
    #[serde(default)]
    pub sources: HashMap<String, Vec<SkillLocalSourceRecord>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillLocalSourceRecord {
    pub tool_id: String,
    pub abs_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolCapabilityOverrides {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_rule_source_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_rule_source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_global_rule_target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_mcp_config_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_tool_config_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared_skill_direct_read: Option<bool>,
}

impl ToolCapabilityOverrides {
    pub fn is_empty(&self) -> bool {
        self.custom_rule_source_type
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
            && self
                .custom_rule_source_path
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            && self
                .custom_global_rule_target
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            && self
                .custom_mcp_config_path
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            && self
                .custom_tool_config_path
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            && self.shared_skill_direct_read.is_none()
    }

    pub fn normalized_rule_source_type(&self) -> Option<&'static str> {
        match self
            .custom_rule_source_type
            .as_deref()?
            .trim()
            .to_ascii_lowercase()
            .replace('-', "_")
            .as_str()
        {
            "single_file" | "file" => Some("single_file"),
            "directory" | "dir" => Some("directory"),
            _ => None,
        }
    }

    pub fn normalized_rule_source_path(&self) -> Option<String> {
        normalized_exact_override_path(self.custom_rule_source_path.as_deref())
    }

    pub fn normalized_global_rule_target(&self) -> Option<String> {
        normalized_exact_override_path(self.custom_global_rule_target.as_deref())
    }

    pub fn normalized_mcp_config_path(&self) -> Option<String> {
        normalized_exact_override_path(self.custom_mcp_config_path.as_deref())
    }

    pub fn normalized_tool_config_path(&self) -> Option<String> {
        normalized_exact_override_path(self.custom_tool_config_path.as_deref())
    }
}

fn normalized_exact_override_path(value: Option<&str>) -> Option<String> {
    let path = value?.trim();
    if path.is_empty() || path.contains('*') {
        return None;
    }
    Some(path.to_string())
}

fn default_language() -> String {
    "zh".to_string()
}

fn default_theme() -> String {
    "system".to_string()
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CustomTool {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub config_dir: String,
    #[serde(default)]
    pub rule_directory: String,
    #[serde(default)]
    pub global_rule_file: String,
    #[serde(default)]
    pub skills_dir: String,
    #[serde(default)]
    pub shared_skill_direct_read: bool,
    #[serde(default)]
    pub mcp_config: String,
    #[serde(default)]
    pub tool_config: String,
    #[serde(default)]
    pub rule_file: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ToolPaths {
    pub config_dir: String,
    pub skills_dir: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DefaultRule {
    pub id: String,
    pub name: String,
    pub content: String,
    pub inject_to: Vec<String>, // tool_ids, empty = all
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_targets: Option<Vec<String>>, // explicit management scope; empty = no managed tools
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DefaultRuleInjectionBaselines {
    pub common_rule: String,
    pub custom_rules: HashMap<String, String>,
    pub custom_rule_pending_targets: HashMap<String, Vec<String>>,
}

fn config_path() -> PathBuf {
    crate::platform::env::config_path()
}

pub fn default_config() -> AppConfig {
    AppConfig {
        skill_notes: HashMap::new(),
        default_rules: vec![],
        default_rule_injection_baselines: DefaultRuleInjectionBaselines::default(),
        injection_targets: HashMap::new(),
        tool_paths: HashMap::new(),
        custom_tools: vec![],
        managed_tools: vec![],
        handled_new_tool_ids: vec![],
        initialized: false,
        skills_page_visited: false,
        skill_local_inventory: SkillLocalInventoryState::default(),
        skill_keep_local_choices: HashMap::new(),
        tool_capability_overrides: HashMap::new(),
        language: "zh".to_string(),
        theme: "system".to_string(),
        module_performance_diagnostics_enabled: Some(false),
        skill_performance_diagnostics_enabled: false,
        app_update_state: AppUpdateState::default(),
    }
}

/// Compute a hash of a skill directory's contents for change detection.
/// Walks all files, collects their relative paths and bytes, and hashes the result.
/// Returns the first 16 hex chars of SHA-256 for stable short comparison.
pub fn compute_skill_hash(skill_path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let mut files = Vec::new();
    if let Ok(entries) = walk_dir_recursive(skill_path) {
        for entry in entries {
            if let Ok(meta) = fs::metadata(&entry) {
                if meta.is_file() {
                    files.push(entry);
                }
            }
        }
    }
    files.sort();

    let mut hasher = Sha256::new();
    for file in files {
        let rel = file.strip_prefix(skill_path).unwrap_or(&file);
        hasher.update(rel.to_string_lossy().as_bytes());
        hasher.update([0u8]);
        if let Ok(content) = fs::read(&file) {
            hasher.update(content);
        }
        hasher.update([0xffu8]);
    }

    let digest = hasher.finalize();
    let mut short = String::with_capacity(16);
    for b in digest[..8].iter() {
        short.push_str(&format!("{:02x}", b));
    }
    short
}

fn walk_dir_recursive(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = vec![];
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_dir_recursive(&path)?);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

/// Public wrapper for use by other modules.
pub fn walk_dir_recursive_pub(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    walk_dir_recursive(dir)
}

pub fn resolved_skills_dir_with_config(
    config: &AppConfig,
    tool_id: &str,
    adapter_skills_dir: &str,
) -> String {
    let raw = if let Some(tp) = config.tool_paths.get(tool_id) {
        let s = tp.skills_dir.trim();
        if !s.is_empty() {
            s.to_string()
        } else {
            adapter_skills_dir.to_string()
        }
    } else {
        adapter_skills_dir.to_string()
    };
    shellexpand::tilde(&raw).to_string()
}

pub fn resolved_config_dir_with_config(
    config: &AppConfig,
    tool_id: &str,
    adapter_config_dir: &str,
) -> String {
    let raw = if let Some(tp) = config.tool_paths.get(tool_id) {
        let s = tp.config_dir.trim();
        if !s.is_empty() {
            s.to_string()
        } else {
            adapter_config_dir.to_string()
        }
    } else {
        adapter_config_dir.to_string()
    };
    shellexpand::tilde(&raw).to_string()
}

/// Effective per-tool skills directory for filesystem scans: uses `tool_paths.skills_dir` when
/// non-empty, otherwise the adapter default from detection. Tilde-expanded.
pub fn resolved_skills_dir(tool_id: &str, adapter_skills_dir: &str) -> String {
    let config = load_config();
    resolved_skills_dir_with_config(&config, tool_id, adapter_skills_dir)
}

/// Load config from a specific path. Returns default config if file is missing or invalid.
pub fn load_config_from(path: &std::path::Path) -> AppConfig {
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(mut config) = serde_json::from_str::<AppConfig>(&content) {
                crate::platform::tool_catalog::normalization::normalize_config(&mut config);
                return config;
            }
        }
    }
    default_config()
}

fn temp_config_path(path: &std::path::Path) -> Result<PathBuf, String> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.json");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    Ok(dir.join(format!(
        ".{}.{}.{}.tmp",
        file_name,
        std::process::id(),
        nonce
    )))
}

fn save_config_to_unlocked(config: &AppConfig, path: &std::path::Path) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut normalized = config.clone();
    crate::platform::tool_catalog::normalization::normalize_config(&mut normalized);
    let mut value = serde_json::to_value(&normalized).map_err(|e| e.to_string())?;
    preserve_unknown_top_level_fields(path, &mut value);
    let json = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    let temp_path = temp_config_path(path)?;
    let write_result = (|| -> Result<(), String> {
        let mut temp_file = fs::File::create(&temp_path).map_err(|e| e.to_string())?;
        temp_file
            .write_all(json.as_bytes())
            .map_err(|e| e.to_string())?;
        temp_file.sync_all().map_err(|e| e.to_string())?;
        drop(temp_file);
        fs::rename(&temp_path, path).map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            if let Ok(dir_file) = fs::File::open(dir) {
                let _ = dir_file.sync_all();
            }
        }
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

fn preserve_unknown_top_level_fields(path: &std::path::Path, next: &mut serde_json::Value) {
    let Ok(existing_content) = fs::read_to_string(path) else {
        return;
    };
    let Ok(serde_json::Value::Object(existing)) =
        serde_json::from_str::<serde_json::Value>(&existing_content)
    else {
        return;
    };
    let Some(next_object) = next.as_object_mut() else {
        return;
    };
    for (key, value) in existing {
        next_object.entry(key).or_insert(value);
    }
}

/// Save config to a specific path.
pub fn save_config_to(config: &AppConfig, path: &std::path::Path) -> Result<(), String> {
    let _guard = CONFIG_WRITE_LOCK
        .lock()
        .map_err(|_| "config write lock poisoned".to_string())?;
    save_config_to_unlocked(config, path)?;
    Ok(())
}

pub fn load_config() -> AppConfig {
    load_config_from(&config_path())
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    save_config_to(config, &config_path())
}

pub fn update_config<F>(apply: F) -> Result<AppConfig, String>
where
    F: FnOnce(&mut AppConfig) -> Result<(), String>,
{
    update_config_at(&config_path(), apply)
}

pub fn update_config_at<F>(path: &std::path::Path, apply: F) -> Result<AppConfig, String>
where
    F: FnOnce(&mut AppConfig) -> Result<(), String>,
{
    let _guard = CONFIG_WRITE_LOCK
        .lock()
        .map_err(|_| "config write lock poisoned".to_string())?;
    let mut config = load_config_from(path);
    apply(&mut config)?;
    crate::platform::tool_catalog::normalization::normalize_config(&mut config);
    save_config_to_unlocked(&config, path)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_does_not_persist_injection_targets() {
        let config = default_config();
        assert!(config.injection_targets.is_empty());
    }

    #[test]
    fn default_config_not_initialized() {
        let config = default_config();
        assert!(!config.initialized);
        assert_eq!(config.language, "zh");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let mut config = default_config();
        config.initialized = true;
        config.skills_page_visited = true;
        config.skill_local_inventory.initialized = true;
        config.skill_local_inventory.fingerprint = "baseline-hash".to_string();
        config.skill_local_inventory.sources.insert(
            "skill-a".to_string(),
            vec![SkillLocalSourceRecord {
                tool_id: "codex".to_string(),
                abs_path: "/tmp/codex/skill-a".to_string(),
                content_hash: Some("hash-a".to_string()),
            }],
        );
        config.skill_keep_local_choices.insert(
            "skill-b".to_string(),
            vec![SkillLocalSourceRecord {
                tool_id: "openclaw".to_string(),
                abs_path: "/tmp/openclaw/skill-b".to_string(),
                content_hash: Some("hash-b".to_string()),
            }],
        );
        config.default_rule_injection_baselines.common_rule = "common-hash".to_string();
        config
            .default_rule_injection_baselines
            .custom_rules
            .insert("rule_a".to_string(), "rule-hash".to_string());
        config
            .default_rule_injection_baselines
            .custom_rule_pending_targets
            .insert(
                "rule_a".to_string(),
                vec!["codex".to_string(), "cursor".to_string()],
            );
        config
            .skill_notes
            .insert("test-skill".to_string(), "my note".to_string());
        config.handled_new_tool_ids = vec!["cursor".to_string(), "claude_code".to_string()];
        config.tool_capability_overrides.insert(
            "claude_code".to_string(),
            ToolCapabilityOverrides {
                custom_rule_source_type: Some("single_file".to_string()),
                custom_rule_source_path: Some("/tmp/CLAUDE.md".to_string()),
                custom_global_rule_target: Some("/tmp/CLAUDE.md".to_string()),
                custom_mcp_config_path: Some("/tmp/mcp.json".to_string()),
                custom_tool_config_path: Some("/tmp/settings.json".to_string()),
                shared_skill_direct_read: Some(false),
            },
        );

        save_config_to(&config, &path).unwrap();
        let loaded = load_config_from(&path);

        assert!(loaded.initialized);
        assert!(loaded.skills_page_visited);
        assert!(loaded.skill_local_inventory.initialized);
        assert_eq!(loaded.skill_local_inventory.fingerprint, "baseline-hash");
        assert_eq!(
            loaded.skill_local_inventory.sources.get("skill-a").unwrap()[0].tool_id,
            "codex"
        );
        assert_eq!(
            loaded.skill_keep_local_choices.get("skill-b").unwrap()[0].tool_id,
            "openclaw"
        );
        assert_eq!(
            loaded.default_rule_injection_baselines.common_rule,
            "common-hash"
        );
        assert_eq!(
            loaded
                .default_rule_injection_baselines
                .custom_rules
                .get("rule_a"),
            Some(&"rule-hash".to_string())
        );
        assert_eq!(
            loaded
                .default_rule_injection_baselines
                .custom_rule_pending_targets
                .get("rule_a"),
            Some(&vec!["codex".to_string(), "cursor".to_string()])
        );
        assert_eq!(loaded.skill_notes.get("test-skill").unwrap(), "my note");
        assert_eq!(
            loaded.handled_new_tool_ids,
            vec!["claude-code".to_string(), "cursor".to_string()]
        );
        assert_eq!(
            loaded
                .tool_capability_overrides
                .get("claude-code")
                .and_then(|overrides| overrides.custom_rule_source_type.as_deref()),
            Some("single_file")
        );
        assert_eq!(
            loaded
                .tool_capability_overrides
                .get("claude-code")
                .and_then(|overrides| overrides.custom_rule_source_path.as_deref()),
            Some("/tmp/CLAUDE.md")
        );
        assert_eq!(
            loaded
                .tool_capability_overrides
                .get("claude-code")
                .and_then(|overrides| overrides.custom_global_rule_target.as_deref()),
            Some("/tmp/CLAUDE.md")
        );
        assert_eq!(
            loaded
                .tool_capability_overrides
                .get("claude-code")
                .and_then(|overrides| overrides.custom_mcp_config_path.as_deref()),
            Some("/tmp/mcp.json")
        );
        assert_eq!(
            loaded
                .tool_capability_overrides
                .get("claude-code")
                .and_then(|overrides| overrides.custom_tool_config_path.as_deref()),
            Some("/tmp/settings.json")
        );
        assert_eq!(
            loaded
                .tool_capability_overrides
                .get("claude-code")
                .and_then(|overrides| overrides.shared_skill_direct_read),
            Some(false)
        );
        assert!(loaded.injection_targets.is_empty());
        assert_eq!(loaded.language, "zh");
    }

    #[test]
    fn system_language_preference_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut config = default_config();
        config.language = "system".to_string();

        save_config_to(&config, &path).unwrap();
        let loaded = load_config_from(&path);

        assert_eq!(loaded.language, "system");
    }

    #[test]
    fn handled_new_tool_ids_roundtrip_and_normalize() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut config = default_config();
        config.handled_new_tool_ids = vec![
            "cursor".to_string(),
            "claude_code".to_string(),
            "claude-code".to_string(),
        ];

        save_config_to(&config, &path).unwrap();
        let loaded = load_config_from(&path);

        assert_eq!(
            loaded.handled_new_tool_ids,
            vec!["claude-code".to_string(), "cursor".to_string()]
        );
    }

    #[test]
    fn saving_capability_override_does_not_create_configured_target() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let target = dir.path().join("missing").join("CUSTOM.md");
        let mut config = default_config();
        config.tool_capability_overrides.insert(
            "codex".to_string(),
            ToolCapabilityOverrides {
                custom_rule_source_type: Some("directory".to_string()),
                custom_rule_source_path: Some(
                    target.parent().unwrap().to_string_lossy().to_string(),
                ),
                custom_global_rule_target: Some(target.to_string_lossy().to_string()),
                shared_skill_direct_read: None,
                ..Default::default()
            },
        );

        save_config_to(&config, &path).unwrap();

        assert!(!target.exists());
        assert!(!target.parent().unwrap().exists());
    }

    #[test]
    fn update_config_at_preserves_defaults_and_partial_updates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let updated = update_config_at(&path, |config| {
            config.language = "en".to_string();
            config.theme = "dark".to_string();
            Ok(())
        })
        .unwrap();

        assert_eq!(updated.language, "en");
        assert_eq!(updated.theme, "dark");
        assert!(updated.injection_targets.is_empty());

        let loaded = load_config_from(&path);
        assert_eq!(loaded.language, "en");
        assert_eq!(loaded.theme, "dark");
        assert!(loaded.injection_targets.is_empty());
    }

    #[test]
    fn update_config_at_returns_mutation_error_without_saving() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut config = default_config();
        config.language = "zh".to_string();
        save_config_to(&config, &path).unwrap();

        let result = update_config_at(&path, |config| {
            config.language = "en".to_string();
            Err("boom".to_string())
        });

        assert_eq!(result.err(), Some("boom".to_string()));
        let loaded = load_config_from(&path);
        assert_eq!(loaded.language, "zh");
    }

    #[test]
    fn update_config_at_serializes_concurrent_partial_updates() {
        let dir = tempfile::tempdir().unwrap();
        let path = std::sync::Arc::new(dir.path().join("config.json"));
        save_config_to(&default_config(), path.as_path()).unwrap();

        let update_count = 12usize;
        let start = std::sync::Arc::new(std::sync::Barrier::new(update_count));
        let handles: Vec<_> = (0..update_count)
            .map(|idx| {
                let path = std::sync::Arc::clone(&path);
                let start = std::sync::Arc::clone(&start);
                std::thread::spawn(move || {
                    start.wait();
                    update_config_at(path.as_path(), |config| {
                        config
                            .skill_notes
                            .insert(format!("skill-{idx}"), format!("note-{idx}"));
                        Ok(())
                    })
                    .unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let mut note_keys: Vec<String> = load_config_from(path.as_path())
            .skill_notes
            .keys()
            .cloned()
            .collect();
        note_keys.sort();
        let mut expected: Vec<String> = (0..update_count)
            .map(|idx| format!("skill-{idx}"))
            .collect();
        expected.sort();
        assert_eq!(note_keys, expected);
    }

    #[test]
    fn save_config_to_replaces_config_without_temp_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut config = default_config();
        config.language = "en".to_string();

        save_config_to(&config, &path).unwrap();

        let loaded = load_config_from(&path);
        assert_eq!(loaded.language, "en");
        let temp_files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".config.json.")
            })
            .collect();
        assert!(temp_files.is_empty());
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        let config = load_config_from(&path);
        assert!(!config.initialized);
        assert!(!config.skills_page_visited);
        assert!(!config.skill_local_inventory.initialized);
        assert!(config.skill_keep_local_choices.is_empty());
        assert!(config.injection_targets.is_empty());
    }

    #[test]
    fn load_corrupt_json_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "not valid json {{{").unwrap();
        let config = load_config_from(&path);
        assert!(!config.initialized);
    }

    #[test]
    fn partial_config_fills_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        // Only set initialized, all other fields should get defaults
        std::fs::write(&path, r#"{"initialized": true}"#).unwrap();
        let config = load_config_from(&path);
        assert!(config.initialized);
        assert_eq!(config.language, "zh");
    }

    #[test]
    fn stale_skill_governance_config_fields_are_ignored_but_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(
            &path,
            r#"{
              "initialized": true,
              "skill_mode": "symlink",
              "skills_adopt_skip_fingerprint": "old",
              "skills_adopt_skip_at": "2026-04-17T10:00:00Z",
              "skills_adopt_ignored": ["demo"],
              "skill_governance_modes": { "demo": "unified" }
            }"#,
        )
        .unwrap();

        let loaded = load_config_from(&path);
        assert!(loaded.initialized);
        assert!(loaded.skill_local_inventory.sources.is_empty());
        save_config_to(&loaded, &path).unwrap();
        let serialized = std::fs::read_to_string(&path).unwrap();
        assert!(serialized.contains("\"skill_mode\""));
        assert!(serialized.contains("\"skills_adopt_skip_fingerprint\""));
        assert!(serialized.contains("\"skills_adopt_skip_at\""));
        assert!(serialized.contains("\"skills_adopt_ignored\""));
        assert!(serialized.contains("\"skill_governance_modes\""));
    }

    #[test]
    fn compute_skill_hash_consistent() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("test-skill");
        std::fs::create_dir(&skill).unwrap();
        std::fs::write(skill.join("SKILL.md"), "---\nname: test\n---\nContent").unwrap();
        std::fs::write(skill.join("helper.sh"), "#!/bin/bash\necho hi").unwrap();

        let hash1 = compute_skill_hash(&skill);
        let hash2 = compute_skill_hash(&skill);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16); // 16 hex chars

        // Changing content changes hash
        std::fs::write(
            skill.join("SKILL.md"),
            "---\nname: changed\n---\nNew content",
        )
        .unwrap();
        let hash3 = compute_skill_hash(&skill);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn compute_skill_hash_changes_with_same_size_content() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("same-size");
        std::fs::create_dir(&skill).unwrap();
        std::fs::write(skill.join("SKILL.md"), "abc").unwrap();
        let h1 = compute_skill_hash(&skill);

        // Same file size (3 bytes), different content.
        std::fs::write(skill.join("SKILL.md"), "xyz").unwrap();
        let h2 = compute_skill_hash(&skill);

        assert_ne!(h1, h2);
    }

    #[test]
    fn resolved_skills_dir_with_config_prefers_override_when_present() {
        let mut config = default_config();
        config.tool_paths.insert(
            "codex".to_string(),
            ToolPaths {
                config_dir: String::new(),
                skills_dir: "~/custom-codex-skills".to_string(),
            },
        );

        let resolved = resolved_skills_dir_with_config(&config, "codex", "/default/codex-skills");

        assert!(resolved.ends_with("/custom-codex-skills"));
        assert_ne!(resolved, "/default/codex-skills");
    }

    #[test]
    fn resolved_skills_dir_with_config_falls_back_to_adapter_default_when_override_blank() {
        let mut config = default_config();
        config.tool_paths.insert(
            "codex".to_string(),
            ToolPaths {
                config_dir: String::new(),
                skills_dir: "   ".to_string(),
            },
        );

        let resolved = resolved_skills_dir_with_config(&config, "codex", "/default/codex-skills");

        assert_eq!(resolved, "/default/codex-skills");
    }

    #[test]
    fn resolved_config_dir_with_config_prefers_override_when_present() {
        let mut config = default_config();
        config.tool_paths.insert(
            "codex".to_string(),
            ToolPaths {
                config_dir: "~/custom-codex-config".to_string(),
                skills_dir: String::new(),
            },
        );

        let resolved = resolved_config_dir_with_config(&config, "codex", "/default/codex");

        assert!(resolved.ends_with("/custom-codex-config"));
        assert_ne!(resolved, "/default/codex");
    }

    #[test]
    fn resolved_config_dir_with_config_falls_back_to_adapter_default_when_override_blank() {
        let mut config = default_config();
        config.tool_paths.insert(
            "codex".to_string(),
            ToolPaths {
                config_dir: "   ".to_string(),
                skills_dir: String::new(),
            },
        );

        let resolved = resolved_config_dir_with_config(&config, "codex", "/default/codex");

        assert_eq!(resolved, "/default/codex");
    }
}
