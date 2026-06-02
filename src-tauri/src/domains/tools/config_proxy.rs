// Purpose: Own registered ordinary tool configuration file listing, reading, and safe saves.

use crate::adapters::skills::ConfigEntry;
use crate::adapters::{
    capability_declared_source_path,
    capability_projections::{
        project_capabilities, ToolCapabilityAction, ToolCapabilityExclusionReason,
        ToolCapabilityModule, ToolCapabilityProjection, ToolCapabilitySourceRole,
    },
    ToolAdapter, ToolCapability, ToolCapabilityAccess, ToolCapabilityFormat, ToolCapabilityKind,
    ToolRegistry,
};
use crate::platform::config as app_config;
use crate::platform::tool_capabilities::effective_capabilities;
use crate::platform::tool_capabilities::parse_json_value_for_format;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigFileDescriptor {
    pub id: String,
    pub label: String,
    pub path: String,
    pub format: ToolCapabilityFormat,
    pub access: ToolCapabilityAccess,
    pub editable: bool,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub modified_unix: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigFileContent {
    pub id: String,
    pub label: String,
    pub path: String,
    pub format: ToolCapabilityFormat,
    pub editable: bool,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigFileSaveResult {
    pub backup_path: String,
    pub size_bytes: u64,
    pub modified_unix: Option<u64>,
}

fn is_supported_config_file_format(format: &ToolCapabilityFormat) -> bool {
    matches!(
        format,
        ToolCapabilityFormat::Json
            | ToolCapabilityFormat::Jsonc
            | ToolCapabilityFormat::Toml
            | ToolCapabilityFormat::Yaml
    )
}

fn metadata_for_path(path: &Path) -> (bool, Option<u64>, Option<u64>) {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => {
            let modified_unix = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs());
            (true, Some(metadata.len()), modified_unix)
        }
        Ok(_) => (true, None, None),
        Err(_) => (false, None, None),
    }
}

fn ordinary_config_projections_for_capabilities(
    tool_id: &str,
    capabilities: &[ToolCapability],
) -> Vec<ToolCapabilityProjection> {
    project_capabilities(tool_id, ToolCapabilityModule::OrdinaryConfig, capabilities)
        .into_iter()
        .filter(|projection| {
            let has_user_override = capabilities.iter().any(|capability| {
                capability.kind == ToolCapabilityKind::OrdinaryConfig
                    && capability.source_confidence
                        == crate::adapters::ToolCapabilitySourceConfidence::UserConfigured
            });
            !has_user_override
                || projection.evidence.source_confidence
                    == crate::adapters::ToolCapabilitySourceConfidence::UserConfigured
        })
        .filter(|projection| {
            matches!(
                projection.source_role,
                ToolCapabilitySourceRole::OrdinaryConfigFile
                    | ToolCapabilitySourceRole::OrdinaryConfigNonActionableEvidence
            ) && capability_declared_source_path(&projection.evidence).is_some()
                && projection.exclusion_reason != Some(ToolCapabilityExclusionReason::ProjectScoped)
                && projection.exclusion_reason != Some(ToolCapabilityExclusionReason::OutOfScope)
        })
        .collect()
}

fn ordinary_config_projections_for_config(
    adapter: &dyn ToolAdapter,
    config: &app_config::AppConfig,
) -> Vec<ToolCapabilityProjection> {
    let capabilities = effective_capabilities::resolve_effective_capabilities(
        adapter.id(),
        &adapter.capabilities(),
        config,
    );
    ordinary_config_projections_for_capabilities(adapter.id(), &capabilities)
}

fn ordinary_config_projections_for_tool_config(
    registry: &ToolRegistry,
    tool_id: &str,
    config: &app_config::AppConfig,
) -> Result<Vec<ToolCapabilityProjection>, String> {
    if let Some(adapter) = registry.get_adapter(tool_id) {
        return Ok(ordinary_config_projections_for_config(adapter, config));
    }
    let canonical = crate::platform::tool_catalog::normalization::canonical_tool_id(tool_id);
    let tool = registry
        .detect_all_for_config(config)
        .into_iter()
        .find(|tool| tool.id == canonical && tool.detected)
        .ok_or_else(|| format!("Tool not found: {}", tool_id))?;
    Ok(ordinary_config_projections_for_capabilities(
        &tool.id,
        &tool.capabilities,
    ))
}

fn descriptor_from_projection(
    projection: &ToolCapabilityProjection,
) -> Option<ConfigFileDescriptor> {
    let capability = &projection.evidence;
    let path = capability_declared_source_path(capability)?;
    let (exists, size_bytes, modified_unix) = metadata_for_path(&path);
    let is_file = path.is_file();
    let supported_format = is_supported_config_file_format(&capability.format);
    Some(ConfigFileDescriptor {
        id: capability.id.clone(),
        label: capability.label.clone(),
        path: path.to_string_lossy().to_string(),
        format: capability.format.clone(),
        access: capability.access.clone(),
        editable: projection.allows(&ToolCapabilityAction::Save)
            && exists
            && is_file
            && supported_format,
        exists,
        size_bytes,
        modified_unix,
    })
}

pub(crate) fn list_config_files_domain(
    registry: &ToolRegistry,
    tool_id: String,
) -> Vec<ConfigFileDescriptor> {
    let config = app_config::load_config();
    list_config_files_for_config_domain(registry, tool_id, &config)
}

pub(crate) fn list_config_files_for_config_domain(
    registry: &ToolRegistry,
    tool_id: String,
    config: &app_config::AppConfig,
) -> Vec<ConfigFileDescriptor> {
    let Ok(projections) = ordinary_config_projections_for_tool_config(registry, &tool_id, config)
    else {
        return vec![];
    };
    let mut files: Vec<_> = projections
        .iter()
        .filter_map(descriptor_from_projection)
        .collect();
    files.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.id.cmp(&b.id)));
    files
}

fn registered_config_capability(
    registry: &ToolRegistry,
    tool_id: &str,
    file_id: &str,
) -> Result<(ToolCapability, ToolCapabilityProjection, PathBuf), String> {
    let config = app_config::load_config();
    registered_config_capability_for_config(registry, tool_id, file_id, &config)
}

fn registered_config_capability_for_config(
    registry: &ToolRegistry,
    tool_id: &str,
    file_id: &str,
    config: &app_config::AppConfig,
) -> Result<(ToolCapability, ToolCapabilityProjection, PathBuf), String> {
    let projections = ordinary_config_projections_for_tool_config(registry, tool_id, config)?;
    let projection = projections
        .into_iter()
        .find(|projection| projection.evidence.id == file_id)
        .ok_or("Config file is not registered for this tool")?;
    let capability = projection.evidence.clone();
    let path = capability_declared_source_path(&capability)
        .ok_or("Config file registration has no concrete path")?;
    Ok((capability, projection, path))
}

pub(crate) fn read_config_file_domain(
    registry: &ToolRegistry,
    tool_id: String,
    file_id: String,
) -> Result<ConfigFileContent, String> {
    let (capability, projection, path) =
        registered_config_capability(registry, &tool_id, &file_id)?;
    if !projection.allows(&ToolCapabilityAction::View) {
        return Err("Config file is not readable for this tool capability".to_string());
    }
    if !path.exists() {
        return Err("Config file is missing".to_string());
    }
    if path.is_dir() {
        return Err("Config file registration points to a directory".to_string());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;
    let editable = projection.allows(&ToolCapabilityAction::Save)
        && path.is_file()
        && is_supported_config_file_format(&capability.format);
    Ok(ConfigFileContent {
        id: capability.id,
        label: capability.label,
        path: path.to_string_lossy().to_string(),
        format: capability.format,
        editable,
        content,
    })
}

fn validate_config_file_content(
    format: &ToolCapabilityFormat,
    content: &str,
) -> Result<(), String> {
    match format {
        ToolCapabilityFormat::Json | ToolCapabilityFormat::Jsonc => {
            parse_json_value_for_format(format, content).map(|_| ())
        }
        ToolCapabilityFormat::Toml => content
            .parse::<toml::Value>()
            .map(|_| ())
            .map_err(|e: toml::de::Error| format!("Invalid TOML: {}", e)),
        ToolCapabilityFormat::Yaml => serde_yaml::from_str::<serde_yaml::Value>(content)
            .map(|_| ())
            .map_err(|e| format!("Invalid YAML: {}", e)),
        ToolCapabilityFormat::Markdown
        | ToolCapabilityFormat::InstructionsMarkdown
        | ToolCapabilityFormat::Mdc
        | ToolCapabilityFormat::Directory
        | ToolCapabilityFormat::SkillDirectory
        | ToolCapabilityFormat::Unknown => {
            Err("Unsupported configuration file format for saving".to_string())
        }
    }
}

fn sanitize_backup_segment(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "config".to_string()
    } else {
        sanitized
    }
}

fn create_config_file_backup(tool_id: &str, file_id: &str, path: &Path) -> Result<PathBuf, String> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f").to_string();
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "config".to_string());
    let backup_dir = crate::platform::env::backups_dir()
        .join("tool-config")
        .join(sanitize_backup_segment(tool_id))
        .join(sanitize_backup_segment(file_id));
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    let backup_path = backup_dir.join(format!("{}-{}", timestamp, file_name));
    fs::copy(path, &backup_path).map_err(|e| format!("Failed to create backup: {}", e))?;
    Ok(backup_path)
}

pub(crate) fn save_config_file_domain(
    registry: &ToolRegistry,
    tool_id: String,
    file_id: String,
    content: String,
) -> Result<ConfigFileSaveResult, String> {
    let (capability, projection, path) =
        registered_config_capability(registry, &tool_id, &file_id)?;
    if !path.exists() {
        return Err(
            "Config file is missing; creation is not supported in this version".to_string(),
        );
    }
    if path.is_dir() {
        return Err("Config file registration points to a directory".to_string());
    }
    if !is_supported_config_file_format(&capability.format) {
        return Err("Config file format is not supported for saving".to_string());
    }
    if !projection.allows(&ToolCapabilityAction::Save) {
        return Err("Config file is not writable for this tool capability".to_string());
    }
    validate_config_file_content(&capability.format, &content)?;
    let backup_path = create_config_file_backup(&tool_id, &file_id, &path)?;
    fs::write(&path, content).map_err(|e| format!("Failed to save file: {}", e))?;
    let (_, size_bytes, modified_unix) = metadata_for_path(&path);
    Ok(ConfigFileSaveResult {
        backup_path: backup_path.to_string_lossy().to_string(),
        size_bytes: size_bytes.unwrap_or(0),
        modified_unix,
    })
}

pub(crate) fn list_configs_domain(registry: &ToolRegistry, tool_id: String) -> Vec<ConfigEntry> {
    let _ = registry;
    let _ = tool_id;
    vec![]
}

pub(crate) fn update_config_domain(
    registry: &ToolRegistry,
    tool_id: String,
    key: String,
    value: String,
) -> Result<(), String> {
    let _ = registry;
    let _ = tool_id;
    let _ = key;
    let _ = value;
    Err("Config key editing has been replaced by registered file editing".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{
        RuleSource, ToolCapabilityKind, ToolCapabilityScope, ToolCapabilitySourceConfidence,
        ToolSourceDiagnosticState,
    };

    struct ConfigTestAdapter {
        access: ToolCapabilityAccess,
        format: ToolCapabilityFormat,
        source_path: PathBuf,
    }

    impl ConfigTestAdapter {
        fn new(access: ToolCapabilityAccess) -> Self {
            Self {
                access,
                format: ToolCapabilityFormat::Json,
                source_path: PathBuf::from("/settings.json"),
            }
        }

        fn with_path(access: ToolCapabilityAccess, source_path: PathBuf) -> Self {
            Self {
                access,
                format: ToolCapabilityFormat::Json,
                source_path,
            }
        }

        fn with_path_and_format(
            access: ToolCapabilityAccess,
            source_path: PathBuf,
            format: ToolCapabilityFormat,
        ) -> Self {
            Self {
                access,
                format,
                source_path,
            }
        }
    }

    impl ToolAdapter for ConfigTestAdapter {
        fn id(&self) -> &str {
            "config-test"
        }
        fn name(&self) -> &str {
            "Config Test"
        }
        fn icon(&self) -> &str {
            "C"
        }
        fn config_dir(&self) -> PathBuf {
            PathBuf::new()
        }
        fn detect(&self) -> bool {
            true
        }
        fn read_rules(&self) -> Result<Vec<RuleSource>, String> {
            Ok(vec![])
        }
        fn write_rule(&self, _path: &str, _content: &str) -> Result<(), String> {
            Ok(())
        }
        fn capabilities(&self) -> Vec<ToolCapability> {
            vec![
                ToolCapability {
                    id: "settings".to_string(),
                    kind: ToolCapabilityKind::OrdinaryConfig,
                    scope: ToolCapabilityScope::Global,
                    access: self.access.clone(),
                    format: self.format.clone(),
                    source_path: self.source_path.to_string_lossy().to_string(),
                    label: "Settings".to_string(),
                    diagnostics: vec![ToolSourceDiagnosticState::Loaded],
                    source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
                    notes: String::new(),
                    source_kind: crate::adapters::ToolCapabilitySourceKind::FeatureSource,
                    primary_config_dir: None,
                    supporting_sources: vec![],
                    action_evidence: vec![],
                },
                ToolCapability {
                    id: "rules".to_string(),
                    kind: ToolCapabilityKind::Rule,
                    scope: ToolCapabilityScope::Global,
                    access: ToolCapabilityAccess::Writable,
                    format: ToolCapabilityFormat::Markdown,
                    source_path: "/RULES.md".to_string(),
                    label: "Rules".to_string(),
                    diagnostics: vec![ToolSourceDiagnosticState::Loaded],
                    source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
                    notes: String::new(),
                    source_kind: crate::adapters::ToolCapabilitySourceKind::FeatureSource,
                    primary_config_dir: None,
                    supporting_sources: vec![],
                    action_evidence: vec![],
                },
                ToolCapability {
                    id: "mcp".to_string(),
                    kind: ToolCapabilityKind::Mcp,
                    scope: ToolCapabilityScope::Global,
                    access: ToolCapabilityAccess::ReadOnly,
                    format: ToolCapabilityFormat::Json,
                    source_path: "/mcp.json".to_string(),
                    label: "MCP".to_string(),
                    diagnostics: vec![ToolSourceDiagnosticState::Loaded],
                    source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
                    notes: String::new(),
                    source_kind: crate::adapters::ToolCapabilitySourceKind::FeatureSource,
                    primary_config_dir: None,
                    supporting_sources: vec![],
                    action_evidence: vec![],
                },
            ]
        }
    }

    fn test_registry(adapter: ConfigTestAdapter) -> ToolRegistry {
        ToolRegistry::from_adapters_for_tests(vec![Box::new(adapter)])
    }

    #[test]
    fn list_config_files_returns_registered_ordinary_files_only() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, "{\"model\":\"gpt\"}").unwrap();
        let registry = test_registry(ConfigTestAdapter::with_path(
            ToolCapabilityAccess::Writable,
            settings.clone(),
        ));

        let files = list_config_files_domain(&registry, "config-test".to_string());

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].id, "settings");
        assert_eq!(files[0].label, "Settings");
        assert_eq!(files[0].format, ToolCapabilityFormat::Json);
        assert!(files[0].exists);
        assert!(files[0].editable);
        assert!(files[0].size_bytes.unwrap() > 0);
        assert!(!files
            .iter()
            .any(|file| file.id == "rules" || file.id == "mcp"));
    }

    #[test]
    fn custom_tool_ordinary_config_projects_to_editable_registered_file() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("custom-settings.json");
        fs::write(&settings, "{\"model\":\"gpt\"}").unwrap();
        let mut config = app_config::default_config();
        config
            .custom_tools
            .push(crate::platform::config::CustomTool {
                id: "custom-config".to_string(),
                name: "Custom Config".to_string(),
                icon: "wrench".to_string(),
                config_dir: String::new(),
                rule_directory: String::new(),
                global_rule_file: String::new(),
                skills_dir: String::new(),
                shared_skill_direct_read: false,
                mcp_config: String::new(),
                tool_config: settings.to_string_lossy().to_string(),
                rule_file: String::new(),
            });
        let registry = ToolRegistry::from_adapters_for_tests(vec![]);
        let projections =
            ordinary_config_projections_for_tool_config(&registry, "custom-config", &config)
                .unwrap();
        let files = projections
            .iter()
            .filter_map(descriptor_from_projection)
            .collect::<Vec<_>>();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].id, "user-configured-tool-config");
        assert!(files[0].editable);
        let (_capability, projection, path) = registered_config_capability_for_config(
            &registry,
            "custom-config",
            "user-configured-tool-config",
            &config,
        )
        .unwrap();
        assert_eq!(path, settings);
        assert!(projection.allows(&ToolCapabilityAction::Save));
    }

    #[test]
    fn list_config_files_marks_missing_and_read_only_files_uneditable() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("settings.json");
        let registry = test_registry(ConfigTestAdapter::with_path(
            ToolCapabilityAccess::ReadOnly,
            missing,
        ));

        let files = list_config_files_domain(&registry, "config-test".to_string());

        assert_eq!(files.len(), 1);
        assert!(!files[0].exists);
        assert!(!files[0].editable);
        assert_eq!(files[0].access, ToolCapabilityAccess::ReadOnly);
    }

    #[test]
    fn list_config_files_marks_non_writable_access_uneditable() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, "{}").unwrap();
        for access in [
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityAccess::Readable,
            ToolCapabilityAccess::Unknown,
            ToolCapabilityAccess::Unsupported,
        ] {
            let registry = test_registry(ConfigTestAdapter::with_path(
                access.clone(),
                settings.clone(),
            ));

            let files = list_config_files_domain(&registry, "config-test".to_string());

            assert_eq!(files.len(), 1);
            assert!(files[0].exists);
            assert!(!files[0].editable);
            assert_eq!(files[0].access, access);
        }
    }

    #[test]
    fn windsurf_lists_no_ordinary_config_files() {
        let tmp = tempfile::tempdir().unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![crate::adapters::windsurf::create(
                tmp.path(),
            )]);

        let files = list_config_files_domain(&registry, "windsurf".to_string());

        assert!(files.is_empty());
    }

    #[test]
    fn github_copilot_lists_settings_not_tool_managed_config_state() {
        let tmp = tempfile::tempdir().unwrap();
        let copilot_dir = tmp.path().join(".copilot");
        fs::create_dir_all(&copilot_dir).unwrap();
        fs::write(copilot_dir.join("settings.json"), "{\"model\":\"gpt-4.1\"}").unwrap();
        fs::write(
            copilot_dir.join("config.json"),
            "// User settings belong in settings.json.\n{\"tool\":\"state\"}",
        )
        .unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![crate::adapters::github_copilot::create(
                tmp.path(),
            )]);

        let files = list_config_files_domain(&registry, "github-copilot".to_string());

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].id, "ordinary-settings");
        assert_eq!(files[0].label, "Settings");
        assert_eq!(files[0].format, ToolCapabilityFormat::Json);
        assert!(files[0].editable);
        assert_eq!(
            files[0].path,
            copilot_dir
                .join("settings.json")
                .to_string_lossy()
                .to_string()
        );
    }

    #[test]
    fn read_config_file_returns_raw_content_for_registered_file() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, "{\"model\":\"gpt\"}").unwrap();
        let registry = test_registry(ConfigTestAdapter::with_path(
            ToolCapabilityAccess::Readable,
            settings,
        ));

        let content =
            read_config_file_domain(&registry, "config-test".to_string(), "settings".to_string())
                .unwrap();

        assert_eq!(content.content, "{\"model\":\"gpt\"}");
        assert!(!content.editable);
    }

    #[test]
    fn read_config_file_keeps_unsupported_formats_uneditable() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("settings.conf");
        fs::write(&settings, "model = old").unwrap();
        let registry = test_registry(ConfigTestAdapter::with_path_and_format(
            ToolCapabilityAccess::Writable,
            settings,
            ToolCapabilityFormat::Unknown,
        ));

        let content =
            read_config_file_domain(&registry, "config-test".to_string(), "settings".to_string())
                .unwrap();

        assert_eq!(content.content, "model = old");
        assert!(!content.editable);
    }

    #[test]
    fn save_config_file_validates_json_creates_backup_and_writes() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, "{\"model\":\"old\"}").unwrap();
        let registry = test_registry(ConfigTestAdapter::with_path(
            ToolCapabilityAccess::Writable,
            settings.clone(),
        ));

        let result = save_config_file_domain(
            &registry,
            "config-test".to_string(),
            "settings".to_string(),
            "{\"model\":\"new\"}".to_string(),
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(&settings).unwrap(),
            "{\"model\":\"new\"}"
        );
        assert!(PathBuf::from(&result.backup_path).exists());
        assert_eq!(
            fs::read_to_string(&result.backup_path).unwrap(),
            "{\"model\":\"old\"}"
        );
    }

    #[test]
    fn save_config_file_accepts_jsonc_comments_and_trailing_commas() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("opencode.jsonc");
        fs::write(&settings, "{\n  \"$schema\": \"old\"\n}\n").unwrap();
        let registry = test_registry(ConfigTestAdapter::with_path_and_format(
            ToolCapabilityAccess::Writable,
            settings.clone(),
            ToolCapabilityFormat::Jsonc,
        ));

        let next = "{\n  // OpenCode config\n  \"$schema\": \"new\",\n}\n";
        let result = save_config_file_domain(
            &registry,
            "config-test".to_string(),
            "settings".to_string(),
            next.to_string(),
        )
        .unwrap();

        assert_eq!(fs::read_to_string(&settings).unwrap(), next);
        assert!(PathBuf::from(&result.backup_path).exists());
        assert_eq!(
            fs::read_to_string(&result.backup_path).unwrap(),
            "{\n  \"$schema\": \"old\"\n}\n"
        );
    }

    #[test]
    fn save_config_file_rejects_invalid_json_without_writing() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, "{\"model\":\"old\"}").unwrap();
        let registry = test_registry(ConfigTestAdapter::with_path(
            ToolCapabilityAccess::Writable,
            settings.clone(),
        ));

        let err = save_config_file_domain(
            &registry,
            "config-test".to_string(),
            "settings".to_string(),
            "{invalid".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("Invalid JSON"));
        assert_eq!(
            fs::read_to_string(&settings).unwrap(),
            "{\"model\":\"old\"}"
        );
    }

    #[test]
    fn save_config_file_rejects_invalid_toml_without_writing() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("config.toml");
        fs::write(&settings, "model = \"old\"").unwrap();
        let registry = test_registry(ConfigTestAdapter::with_path_and_format(
            ToolCapabilityAccess::Writable,
            settings.clone(),
            ToolCapabilityFormat::Toml,
        ));

        let err = save_config_file_domain(
            &registry,
            "config-test".to_string(),
            "settings".to_string(),
            "model = [".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("Invalid TOML"));
        assert_eq!(fs::read_to_string(&settings).unwrap(), "model = \"old\"");
    }

    #[test]
    fn save_config_file_validates_yaml_creates_backup_and_writes() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("config.yaml");
        fs::write(&settings, "model: old\n").unwrap();
        let registry = test_registry(ConfigTestAdapter::with_path_and_format(
            ToolCapabilityAccess::Writable,
            settings.clone(),
            ToolCapabilityFormat::Yaml,
        ));

        let result = save_config_file_domain(
            &registry,
            "config-test".to_string(),
            "settings".to_string(),
            "model: new\n".to_string(),
        )
        .unwrap();

        assert_eq!(fs::read_to_string(&settings).unwrap(), "model: new\n");
        assert!(PathBuf::from(&result.backup_path).exists());
        assert_eq!(
            fs::read_to_string(&result.backup_path).unwrap(),
            "model: old\n"
        );
    }

    #[test]
    fn save_config_file_rejects_unknown_and_directory_formats() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("settings.conf");
        fs::write(&settings, "model = old").unwrap();
        let registry = test_registry(ConfigTestAdapter::with_path_and_format(
            ToolCapabilityAccess::Writable,
            settings.clone(),
            ToolCapabilityFormat::Unknown,
        ));

        let files = list_config_files_domain(&registry, "config-test".to_string());
        assert_eq!(files.len(), 1);
        assert!(!files[0].editable);

        let err = save_config_file_domain(
            &registry,
            "config-test".to_string(),
            "settings".to_string(),
            "model = new".to_string(),
        )
        .unwrap_err();
        assert!(err.contains("format is not supported"));
        assert_eq!(fs::read_to_string(&settings).unwrap(), "model = old");

        let dir = tmp.path().join("settings-dir");
        fs::create_dir(&dir).unwrap();
        let registry = test_registry(ConfigTestAdapter::with_path_and_format(
            ToolCapabilityAccess::Writable,
            dir,
            ToolCapabilityFormat::Directory,
        ));
        let files = list_config_files_domain(&registry, "config-test".to_string());
        assert_eq!(files.len(), 1);
        assert!(!files[0].editable);
    }

    #[test]
    fn save_config_file_rejects_missing_read_only_and_unregistered_files() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("settings.json");
        let registry = test_registry(ConfigTestAdapter::with_path(
            ToolCapabilityAccess::Writable,
            missing,
        ));
        let err = save_config_file_domain(
            &registry,
            "config-test".to_string(),
            "settings".to_string(),
            "{}".to_string(),
        )
        .unwrap_err();
        assert!(err.contains("missing"));

        let settings = tmp.path().join("readonly.json");
        fs::write(&settings, "{}").unwrap();
        let registry = test_registry(ConfigTestAdapter::with_path(
            ToolCapabilityAccess::ReadOnly,
            settings,
        ));
        let err = save_config_file_domain(
            &registry,
            "config-test".to_string(),
            "settings".to_string(),
            "{}".to_string(),
        )
        .unwrap_err();
        assert!(err.contains("not writable"));

        let registry = test_registry(ConfigTestAdapter::new(ToolCapabilityAccess::Writable));
        let err = save_config_file_domain(
            &registry,
            "config-test".to_string(),
            "unknown".to_string(),
            "{}".to_string(),
        )
        .unwrap_err();
        assert!(err.contains("not registered"));
    }

    #[test]
    fn list_configs_legacy_key_value_view_returns_empty() {
        let registry = test_registry(ConfigTestAdapter::new(ToolCapabilityAccess::Readable));

        let entries = list_configs_domain(&registry, "config-test".to_string());

        assert!(entries.is_empty());
    }

    #[test]
    fn update_config_legacy_key_value_write_is_disabled() {
        let registry = test_registry(ConfigTestAdapter::new(ToolCapabilityAccess::Writable));

        let err = update_config_domain(
            &registry,
            "config-test".to_string(),
            "model".to_string(),
            "\"gpt\"".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("registered file editing"));
    }
}
