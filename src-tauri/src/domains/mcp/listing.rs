// Purpose: Own adapter-backed MCP configuration source listing, reading, and safe saves.

use crate::adapters::skills::McpServerInfo;
use crate::adapters::{
    capability_declared_source_path, capability_existing_source_file,
    capability_projections::{
        project_capabilities, ToolCapabilityAction, ToolCapabilityModule, ToolCapabilityProjection,
        ToolCapabilitySourceRole,
    },
    ToolCapability, ToolCapabilityAccess, ToolCapabilityFormat, ToolCapabilityKind,
    ToolCapabilitySourceConfidence, ToolRegistry,
};
use crate::platform::config as app_config;
use crate::platform::tool_capabilities::{effective_capabilities, mcp_sources};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpDiagnosticState {
    Loaded,
    Missing,
    Empty,
    Unreadable,
    Malformed,
    Error,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpDiagnosticResult {
    pub tool_id: String,
    pub state: McpDiagnosticState,
    pub source_paths: Vec<String>,
    pub message: String,
    pub servers: Vec<McpServerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpConfigSourceKind {
    File,
    MixedFile,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfigSourceInfo {
    pub id: String,
    pub tool_id: String,
    pub label: String,
    pub path: Option<String>,
    pub format: ToolCapabilityFormat,
    pub access: ToolCapabilityAccess,
    pub source_kind: McpConfigSourceKind,
    pub state: McpDiagnosticState,
    pub editable: bool,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub modified_unix: Option<u64>,
    pub server_count: Option<usize>,
    pub message: String,
    pub servers: Vec<McpServerInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_adapter: Option<McpAdapterRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpAdapterRequirement {
    pub tool_name: String,
    pub package_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfigFragment {
    pub tool_id: String,
    pub source_id: String,
    pub server_name: String,
    pub label: String,
    pub path: String,
    pub format: ToolCapabilityFormat,
    pub editable: bool,
    pub content: String,
    pub server: McpServerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfigSaveResult {
    pub backup_path: String,
    pub size_bytes: u64,
    pub modified_unix: Option<u64>,
    pub servers: Vec<McpServerInfo>,
}

fn sanitize_error_message(_error: String) -> String {
    "MCP configuration could not be parsed or read. Source content is hidden for safety."
        .to_string()
}

fn classify_mcp_error(error: &str) -> McpDiagnosticState {
    let lower = error.to_ascii_lowercase();
    if lower.starts_with("unreadable:")
        || lower.contains("permission denied")
        || lower.contains("operation not permitted")
        || lower.contains("is a directory")
    {
        return McpDiagnosticState::Unreadable;
    }
    if lower.starts_with("malformed:")
        || lower.contains("parse")
        || lower.contains("expected")
        || lower.contains("invalid")
        || lower.contains("json")
        || lower.contains("toml")
        || lower.contains("yaml")
    {
        return McpDiagnosticState::Malformed;
    }
    McpDiagnosticState::Error
}

fn mcp_capability_projections_for_config(
    adapter: &dyn crate::adapters::ToolAdapter,
    config: &app_config::AppConfig,
) -> Vec<ToolCapabilityProjection> {
    let capabilities = effective_capabilities::resolve_effective_capabilities(
        adapter.id(),
        &adapter.capabilities(),
        config,
    );
    let has_user_override = capabilities.iter().any(|capability| {
        capability.kind == ToolCapabilityKind::Mcp
            && capability.source_confidence == ToolCapabilitySourceConfidence::UserConfigured
    });
    project_capabilities(adapter.id(), ToolCapabilityModule::Mcp, &capabilities)
        .into_iter()
        .filter(|projection| {
            !has_user_override
                || projection.evidence.source_confidence
                    == ToolCapabilitySourceConfidence::UserConfigured
        })
        .filter(|projection| {
            matches!(
                projection.source_role,
                ToolCapabilitySourceRole::McpGlobalConfig
                    | ToolCapabilitySourceRole::McpNonActionableEvidence
            )
        })
        .collect()
}

fn mcp_capability_projections_for_capabilities(
    tool_id: &str,
    capabilities: &[ToolCapability],
) -> Vec<ToolCapabilityProjection> {
    let has_user_override = capabilities.iter().any(|capability| {
        capability.kind == ToolCapabilityKind::Mcp
            && capability.source_confidence == ToolCapabilitySourceConfidence::UserConfigured
    });
    project_capabilities(tool_id, ToolCapabilityModule::Mcp, capabilities)
        .into_iter()
        .filter(|projection| {
            !has_user_override
                || projection.evidence.source_confidence
                    == ToolCapabilitySourceConfidence::UserConfigured
        })
        .filter(|projection| {
            matches!(
                projection.source_role,
                ToolCapabilitySourceRole::McpGlobalConfig
                    | ToolCapabilitySourceRole::McpNonActionableEvidence
            )
        })
        .collect()
}

fn is_supported_config_format(format: &ToolCapabilityFormat) -> bool {
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

fn source_kind_for_capability(capability: &ToolCapability) -> McpConfigSourceKind {
    if capability.source_path.trim().is_empty() {
        return McpConfigSourceKind::Unknown;
    }
    if capability.source_path.contains('#') {
        return McpConfigSourceKind::MixedFile;
    }
    McpConfigSourceKind::File
}

fn state_message(state: &McpDiagnosticState) -> String {
    match state {
        McpDiagnosticState::Loaded => "MCP configuration loaded".to_string(),
        McpDiagnosticState::Missing => "MCP configuration source is missing".to_string(),
        McpDiagnosticState::Empty => "MCP configuration exists but has no servers".to_string(),
        McpDiagnosticState::Unreadable => "MCP configuration source cannot be read".to_string(),
        McpDiagnosticState::Malformed => "MCP configuration source is malformed".to_string(),
        McpDiagnosticState::Error => "MCP configuration could not be read".to_string(),
        McpDiagnosticState::Unsupported => {
            "MCP configuration is not supported for this tool".to_string()
        }
        McpDiagnosticState::Unknown => {
            "MCP configuration support is unknown for this tool".to_string()
        }
    }
}

fn adapter_requirement(
    tool_name: &str,
    capability: &ToolCapability,
) -> Option<McpAdapterRequirement> {
    if capability.access == ToolCapabilityAccess::Unsupported
        && capability.id == "mcp-adapter"
        && !capability.label.trim().is_empty()
    {
        return Some(McpAdapterRequirement {
            tool_name: tool_name.to_string(),
            package_name: capability.label.clone(),
        });
    }
    None
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
        "mcp".to_string()
    } else {
        sanitized
    }
}

fn create_mcp_config_backup(
    tool_id: &str,
    source_id: &str,
    path: &Path,
) -> Result<PathBuf, String> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f").to_string();
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "mcp-config".to_string());
    let backup_dir = crate::platform::env::backups_dir()
        .join("mcp-config")
        .join(sanitize_backup_segment(tool_id))
        .join(sanitize_backup_segment(source_id));
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    let backup_path = backup_dir.join(format!("{}-{}", timestamp, file_name));
    fs::copy(path, &backup_path).map_err(|e| format!("Failed to create backup: {}", e))?;
    Ok(backup_path)
}

fn source_info_from_projection(
    tool_id: &str,
    tool_name: &str,
    projection: &ToolCapabilityProjection,
    servers_result: &Result<Vec<McpServerInfo>, String>,
) -> McpConfigSourceInfo {
    let capability = &projection.evidence;
    let path = capability_declared_source_path(capability);
    let (exists, size_bytes, modified_unix) = path
        .as_ref()
        .map(|path| metadata_for_path(path))
        .unwrap_or((false, None, None));
    let is_file = path.as_ref().is_some_and(|path| path.is_file());
    let source_kind = source_kind_for_capability(capability);
    let supported_format = is_supported_config_format(&capability.format);

    let (state, servers) = if capability.access == ToolCapabilityAccess::Unsupported {
        (McpDiagnosticState::Unsupported, vec![])
    } else if capability.access == ToolCapabilityAccess::Unknown || path.is_none() {
        (McpDiagnosticState::Unknown, vec![])
    } else if !exists {
        (McpDiagnosticState::Missing, vec![])
    } else if !is_file {
        (McpDiagnosticState::Unreadable, vec![])
    } else {
        match servers_result {
            Ok(servers) if !servers.is_empty() => (McpDiagnosticState::Loaded, servers.clone()),
            Ok(servers) => (McpDiagnosticState::Empty, servers.clone()),
            Err(error) => (classify_mcp_error(error), vec![]),
        }
    };

    McpConfigSourceInfo {
        id: capability.id.clone(),
        tool_id: tool_id.to_string(),
        label: capability.label.clone(),
        path: path.map(|path| path.to_string_lossy().to_string()),
        format: capability.format.clone(),
        access: capability.access.clone(),
        source_kind,
        state: state.clone(),
        editable: projection.allows(&ToolCapabilityAction::Save)
            && exists
            && is_file
            && supported_format,
        exists,
        size_bytes,
        modified_unix,
        server_count: if matches!(
            state,
            McpDiagnosticState::Loaded | McpDiagnosticState::Empty
        ) {
            Some(servers.len())
        } else {
            None
        },
        message: state_message(&state),
        servers,
        required_adapter: adapter_requirement(tool_name, capability),
    }
}

pub(crate) fn get_mcp_diagnostics_domain(
    registry: &ToolRegistry,
    tool_id: String,
) -> McpDiagnosticResult {
    let config = app_config::load_config();
    get_mcp_diagnostics_for_config_domain(registry, tool_id, &config)
}

pub(crate) fn get_mcp_diagnostics_for_config_domain(
    registry: &ToolRegistry,
    tool_id: String,
    config: &app_config::AppConfig,
) -> McpDiagnosticResult {
    let canonical = crate::platform::tool_catalog::normalization::canonical_tool_id(&tool_id);
    let (adapter, mcp_projections) = if let Some(adapter) = registry.get_adapter(&tool_id) {
        (
            Some(adapter),
            mcp_capability_projections_for_config(adapter, config),
        )
    } else {
        let Some(tool) = registry
            .detect_all_for_config(config)
            .into_iter()
            .find(|tool| tool.id == canonical && tool.detected)
        else {
            return McpDiagnosticResult {
                tool_id,
                state: McpDiagnosticState::Unknown,
                source_paths: vec![],
                message: "Tool is not registered".to_string(),
                servers: vec![],
            };
        };
        (
            None,
            mcp_capability_projections_for_capabilities(&tool.id, &tool.capabilities),
        )
    };

    if mcp_projections.is_empty() && adapter.is_none() {
        return McpDiagnosticResult {
            tool_id,
            state: McpDiagnosticState::Unsupported,
            source_paths: vec![],
            message: "MCP configuration is not supported for this tool".to_string(),
            servers: vec![],
        };
    };

    let mcp_capabilities: Vec<&ToolCapability> = mcp_projections
        .iter()
        .map(|projection| &projection.evidence)
        .collect();
    let source_paths: Vec<String> = mcp_capabilities
        .iter()
        .filter_map(|capability| {
            let path = capability.source_path.trim();
            if path.is_empty() {
                None
            } else {
                Some(path.to_string())
            }
        })
        .collect();

    if mcp_capabilities.is_empty()
        || mcp_capabilities
            .iter()
            .all(|capability| capability.access == ToolCapabilityAccess::Unsupported)
    {
        return McpDiagnosticResult {
            tool_id,
            state: McpDiagnosticState::Unsupported,
            source_paths,
            message: "MCP configuration is not supported for this tool".to_string(),
            servers: vec![],
        };
    }

    if mcp_capabilities
        .iter()
        .all(|capability| capability.access == ToolCapabilityAccess::Unknown)
    {
        return McpDiagnosticResult {
            tool_id,
            state: McpDiagnosticState::Unknown,
            source_paths,
            message: "MCP configuration support is unknown for this tool".to_string(),
            servers: vec![],
        };
    }

    let servers_result = if mcp_capabilities.iter().any(|capability| {
        capability.source_confidence == ToolCapabilitySourceConfidence::UserConfigured
    }) || adapter.is_none()
    {
        let mut servers = vec![];
        let mut first_error: Option<String> = None;
        for capability in &mcp_capabilities {
            match mcp_sources::parse_mcp_servers_for_capability(capability) {
                Ok(mut capability_servers) => servers.append(&mut capability_servers),
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        if servers.is_empty() {
            if let Some(error) = first_error {
                Err(error)
            } else {
                Ok(servers)
            }
        } else {
            Ok(servers)
        }
    } else {
        adapter
            .expect("adapter is present when no user-configured MCP source is selected")
            .read_mcp_servers()
    };

    match servers_result {
        Ok(servers) if !servers.is_empty() => McpDiagnosticResult {
            tool_id,
            state: McpDiagnosticState::Loaded,
            source_paths,
            message: "MCP configuration loaded".to_string(),
            servers,
        },
        Ok(servers) => {
            let has_source = mcp_capabilities
                .iter()
                .any(|capability| capability_existing_source_file(capability).is_some());
            McpDiagnosticResult {
                tool_id,
                state: if has_source {
                    McpDiagnosticState::Empty
                } else {
                    McpDiagnosticState::Missing
                },
                source_paths,
                message: if has_source {
                    "MCP configuration exists but has no servers".to_string()
                } else {
                    "MCP configuration source is missing".to_string()
                },
                servers,
            }
        }
        Err(error) => {
            let state = classify_mcp_error(&error);
            McpDiagnosticResult {
                tool_id,
                state,
                source_paths,
                message: sanitize_error_message(error),
                servers: vec![],
            }
        }
    }
}

pub(crate) fn list_mcp_servers_domain(
    registry: &ToolRegistry,
    tool_id: String,
) -> Vec<McpServerInfo> {
    get_mcp_diagnostics_domain(registry, tool_id).servers
}

pub(crate) fn list_mcp_config_sources_domain(
    registry: &ToolRegistry,
    tool_id: String,
) -> Vec<McpConfigSourceInfo> {
    let config = app_config::load_config();
    list_mcp_config_sources_for_config_domain(registry, tool_id, &config)
}

fn list_mcp_config_sources_for_config_domain(
    registry: &ToolRegistry,
    tool_id: String,
    config: &app_config::AppConfig,
) -> Vec<McpConfigSourceInfo> {
    let (tool_name, projections) = if let Some(adapter) = registry.get_adapter(&tool_id) {
        (
            adapter.name().to_string(),
            mcp_capability_projections_for_config(adapter, config),
        )
    } else {
        let canonical = crate::platform::tool_catalog::normalization::canonical_tool_id(&tool_id);
        let Some(tool) = registry
            .detect_all_for_config(config)
            .into_iter()
            .find(|tool| tool.id == canonical && tool.detected)
        else {
            return vec![];
        };
        (
            tool.name.clone(),
            mcp_capability_projections_for_capabilities(&tool.id, &tool.capabilities),
        )
    };
    let mut sources: Vec<_> = projections
        .iter()
        .map(|projection| {
            let servers_result =
                mcp_sources::parse_mcp_servers_for_capability(&projection.evidence);
            source_info_from_projection(&tool_id, &tool_name, projection, &servers_result)
        })
        .collect();
    sources.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.id.cmp(&b.id)));
    sources
}

fn get_registered_mcp_capability<'a>(
    registry: &'a ToolRegistry,
    tool_id: &str,
    source_id: &str,
) -> Result<
    (
        Option<&'a dyn crate::adapters::ToolAdapter>,
        ToolCapability,
        ToolCapabilityProjection,
        PathBuf,
    ),
    String,
> {
    let config = app_config::load_config();
    get_registered_mcp_capability_for_config(registry, tool_id, source_id, &config)
}

fn get_registered_mcp_capability_for_config<'a>(
    registry: &'a ToolRegistry,
    tool_id: &str,
    source_id: &str,
    config: &app_config::AppConfig,
) -> Result<
    (
        Option<&'a dyn crate::adapters::ToolAdapter>,
        ToolCapability,
        ToolCapabilityProjection,
        PathBuf,
    ),
    String,
> {
    let canonical = crate::platform::tool_catalog::normalization::canonical_tool_id(tool_id);
    let (adapter, projections) = if let Some(adapter) = registry.get_adapter(tool_id) {
        (
            Some(adapter),
            mcp_capability_projections_for_config(adapter, config),
        )
    } else {
        let tool = registry
            .detect_all_for_config(config)
            .into_iter()
            .find(|tool| tool.id == canonical && tool.detected)
            .ok_or("Tool not found")?;
        (
            None,
            mcp_capability_projections_for_capabilities(&tool.id, &tool.capabilities),
        )
    };
    let projection = projections
        .into_iter()
        .find(|projection| projection.evidence.id == source_id)
        .ok_or("MCP configuration source is not registered for this tool")?;
    let capability = projection.evidence.clone();
    let path = capability_declared_source_path(&capability)
        .ok_or("MCP configuration source has no concrete path")?;
    Ok((adapter, capability, projection, path))
}

#[cfg(test)]
fn read_mcp_server_config_fragment_for_config_domain(
    registry: &ToolRegistry,
    tool_id: String,
    source_id: String,
    server_name: String,
    config: &app_config::AppConfig,
) -> Result<McpServerConfigFragment, String> {
    let (_adapter, capability, projection, path) =
        get_registered_mcp_capability_for_config(registry, &tool_id, &source_id, config)?;
    read_mcp_server_config_fragment_for_capability(
        tool_id,
        source_id,
        server_name,
        capability,
        projection,
        path,
    )
}

pub(crate) fn read_mcp_server_config_fragment_domain(
    registry: &ToolRegistry,
    tool_id: String,
    source_id: String,
    server_name: String,
) -> Result<McpServerConfigFragment, String> {
    let (_adapter, capability, projection, path) =
        get_registered_mcp_capability(registry, &tool_id, &source_id)?;
    read_mcp_server_config_fragment_for_capability(
        tool_id,
        source_id,
        server_name,
        capability,
        projection,
        path,
    )
}

fn read_mcp_server_config_fragment_for_capability(
    tool_id: String,
    source_id: String,
    server_name: String,
    capability: ToolCapability,
    projection: ToolCapabilityProjection,
    path: PathBuf,
) -> Result<McpServerConfigFragment, String> {
    if !projection.allows(&ToolCapabilityAction::View) {
        return Err("MCP configuration source is not readable".to_string());
    }
    if !path.exists() {
        return Err("MCP configuration source is missing".to_string());
    }
    if !path.is_file() {
        return Err("MCP configuration source is not a file".to_string());
    }
    let server = mcp_sources::parse_mcp_servers_for_capability(&capability)
        .map_err(sanitize_error_message)?
        .into_iter()
        .find(|server| server.name == server_name)
        .ok_or("MCP server configuration was not found")?;
    let source_content = fs::read_to_string(&path).map_err(|_| {
        "MCP configuration source cannot be read. Source content is hidden for safety.".to_string()
    })?;
    let content = mcp_sources::extract_server_fragment(
        &capability.format,
        &capability.source_path,
        &source_content,
        &server_name,
    )
    .map_err(|_| {
        "MCP server configuration could not be read. Source content is hidden for safety."
            .to_string()
    })?;
    Ok(McpServerConfigFragment {
        tool_id,
        source_id,
        server_name: server_name.clone(),
        label: server_name,
        path: path.to_string_lossy().to_string(),
        format: capability.format.clone(),
        editable: projection.allows(&ToolCapabilityAction::Save)
            && is_supported_config_format(&capability.format),
        content,
        server,
    })
}

#[cfg(test)]
fn save_mcp_server_config_fragment_for_config_domain(
    registry: &ToolRegistry,
    tool_id: String,
    source_id: String,
    server_name: String,
    content: String,
    config: &app_config::AppConfig,
) -> Result<McpServerConfigSaveResult, String> {
    let (_adapter, capability, projection, path) =
        get_registered_mcp_capability_for_config(registry, &tool_id, &source_id, config)?;
    save_mcp_server_config_fragment_for_capability(
        tool_id,
        source_id,
        capability,
        projection,
        path,
        server_name,
        content,
    )
}

pub(crate) fn save_mcp_server_config_fragment_domain(
    registry: &ToolRegistry,
    tool_id: String,
    source_id: String,
    server_name: String,
    content: String,
) -> Result<McpServerConfigSaveResult, String> {
    let (_adapter, capability, projection, path) =
        get_registered_mcp_capability(registry, &tool_id, &source_id)?;
    save_mcp_server_config_fragment_for_capability(
        tool_id,
        source_id,
        capability,
        projection,
        path,
        server_name,
        content,
    )
}

fn save_mcp_server_config_fragment_for_capability(
    tool_id: String,
    source_id: String,
    capability: ToolCapability,
    projection: ToolCapabilityProjection,
    path: PathBuf,
    server_name: String,
    content: String,
) -> Result<McpServerConfigSaveResult, String> {
    if !projection.allows(&ToolCapabilityAction::Save) {
        return Err("MCP configuration source is not writable".to_string());
    }
    if !path.exists() {
        return Err("MCP configuration source is missing; creation is not supported".to_string());
    }
    if !path.is_file() {
        return Err("MCP configuration source is not a file".to_string());
    }
    if !is_supported_config_format(&capability.format) {
        return Err("MCP configuration format is not supported for saving".to_string());
    }
    let source_content = fs::read_to_string(&path).map_err(|_| {
        "MCP configuration source cannot be read. Source content is hidden for safety.".to_string()
    })?;
    let next_source = mcp_sources::replace_server_fragment(
        &capability.format,
        &capability.source_path,
        &source_content,
        &server_name,
        &content,
    )?;
    let backup_path = create_mcp_config_backup(&tool_id, &source_id, &path)?;
    fs::write(&path, next_source).map_err(|_| {
        "MCP configuration source could not be saved. Source content is hidden for safety."
            .to_string()
    })?;
    let (_, size_bytes, modified_unix) = metadata_for_path(&path);
    let servers = mcp_sources::parse_mcp_servers_for_capability(&capability).unwrap_or_default();
    Ok(McpServerConfigSaveResult {
        backup_path: backup_path.to_string_lossy().to_string(),
        size_bytes: size_bytes.unwrap_or(0),
        modified_unix,
        servers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{
        RuleSource, ToolAdapter, ToolCapability, ToolCapabilityFormat, ToolCapabilityKind,
        ToolCapabilityScope, ToolCapabilitySourceConfidence, ToolSourceDiagnosticState,
    };
    use std::path::PathBuf;

    struct McpTestAdapter {
        access: ToolCapabilityAccess,
        source_path: String,
        format: ToolCapabilityFormat,
        result: Result<Vec<McpServerInfo>, String>,
    }

    impl ToolAdapter for McpTestAdapter {
        fn id(&self) -> &str {
            "mcp-test"
        }
        fn name(&self) -> &str {
            "MCP Test"
        }
        fn icon(&self) -> &str {
            "M"
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
        fn read_mcp_servers(&self) -> Result<Vec<McpServerInfo>, String> {
            self.result.clone()
        }
        fn capabilities(&self) -> Vec<ToolCapability> {
            vec![ToolCapability {
                id: "mcp".to_string(),
                kind: ToolCapabilityKind::Mcp,
                scope: ToolCapabilityScope::Global,
                access: self.access.clone(),
                format: self.format.clone(),
                source_path: self.source_path.clone(),
                label: "MCP".to_string(),
                diagnostics: vec![ToolSourceDiagnosticState::Loaded],
                source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
                notes: String::new(),
                source_kind: crate::adapters::ToolCapabilitySourceKind::FeatureSource,
                primary_config_dir: None,
                supporting_sources: vec![],
                action_evidence: vec![],
            }]
        }
    }

    struct MultiScopeMcpTestAdapter {
        global_path: String,
        project_path: String,
    }

    impl ToolAdapter for MultiScopeMcpTestAdapter {
        fn id(&self) -> &str {
            "mcp-multi-scope-test"
        }
        fn name(&self) -> &str {
            "MCP Multi Scope Test"
        }
        fn icon(&self) -> &str {
            "M"
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
        fn read_mcp_servers(&self) -> Result<Vec<McpServerInfo>, String> {
            Ok(vec![])
        }
        fn capabilities(&self) -> Vec<ToolCapability> {
            vec![
                ToolCapability {
                    id: "global-mcp".to_string(),
                    kind: ToolCapabilityKind::Mcp,
                    scope: ToolCapabilityScope::Global,
                    access: ToolCapabilityAccess::ReadOnly,
                    format: ToolCapabilityFormat::Json,
                    source_path: self.global_path.clone(),
                    label: "Global MCP".to_string(),
                    diagnostics: vec![ToolSourceDiagnosticState::Loaded],
                    source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
                    notes: String::new(),
                    source_kind: crate::adapters::ToolCapabilitySourceKind::FeatureSource,
                    primary_config_dir: None,
                    supporting_sources: vec![],
                    action_evidence: vec![],
                },
                ToolCapability {
                    id: "project-mcp".to_string(),
                    kind: ToolCapabilityKind::Mcp,
                    scope: ToolCapabilityScope::Project,
                    access: ToolCapabilityAccess::ReadOnly,
                    format: ToolCapabilityFormat::Json,
                    source_path: self.project_path.clone(),
                    label: "Project MCP".to_string(),
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

    fn test_registry(adapter: McpTestAdapter) -> ToolRegistry {
        ToolRegistry::from_adapters_for_tests(vec![Box::new(adapter)])
    }

    fn server() -> McpServerInfo {
        McpServerInfo {
            name: "demo".to_string(),
            server_type: "stdio".to_string(),
            command: Some("node".to_string()),
            args: vec![],
            env_keys: vec!["API_KEY".to_string()],
            url: None,
            enabled: true,
            activation_state: crate::adapters::skills::McpServerActivationState::Enabled,
            description: None,
        }
    }

    #[test]
    fn diagnostics_reports_loaded_servers() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join("mcp.json");
        std::fs::write(&config, "{}").unwrap();
        let registry = test_registry(McpTestAdapter {
            access: ToolCapabilityAccess::ReadOnly,
            source_path: config.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Json,
            result: Ok(vec![server()]),
        });

        let diagnostic = get_mcp_diagnostics_domain(&registry, "mcp-test".to_string());

        assert_eq!(diagnostic.state, McpDiagnosticState::Loaded);
        assert_eq!(diagnostic.servers.len(), 1);
        assert_eq!(diagnostic.servers[0].env_keys, vec!["API_KEY".to_string()]);
    }

    #[test]
    fn diagnostics_distinguishes_missing_and_empty_config() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("missing.json");
        let registry = test_registry(McpTestAdapter {
            access: ToolCapabilityAccess::ReadOnly,
            source_path: missing.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Json,
            result: Ok(vec![]),
        });
        assert_eq!(
            get_mcp_diagnostics_domain(&registry, "mcp-test".to_string()).state,
            McpDiagnosticState::Missing
        );

        let config = tmp.path().join("mcp.json");
        std::fs::write(&config, "{}").unwrap();
        let registry = test_registry(McpTestAdapter {
            access: ToolCapabilityAccess::ReadOnly,
            source_path: config.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Json,
            result: Ok(vec![]),
        });
        assert_eq!(
            get_mcp_diagnostics_domain(&registry, "mcp-test".to_string()).state,
            McpDiagnosticState::Empty
        );
    }

    #[test]
    fn diagnostics_reports_malformed_unreadable_unknown_and_unsupported() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join("mcp.json");
        std::fs::write(&config, "{}").unwrap();
        let registry = test_registry(McpTestAdapter {
            access: ToolCapabilityAccess::ReadOnly,
            source_path: config.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Json,
            result: Err("malformed: parse failed with API_KEY=secret".to_string()),
        });
        let diagnostic = get_mcp_diagnostics_domain(&registry, "mcp-test".to_string());
        assert_eq!(diagnostic.state, McpDiagnosticState::Malformed);
        assert!(!diagnostic.message.contains("API_KEY"));
        assert!(!diagnostic.message.contains("secret"));
        assert!(!diagnostic.message.contains("parse failed"));

        let registry = test_registry(McpTestAdapter {
            access: ToolCapabilityAccess::ReadOnly,
            source_path: config.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Json,
            result: Err("unreadable: permission denied with API_KEY=secret".to_string()),
        });
        let diagnostic = get_mcp_diagnostics_domain(&registry, "mcp-test".to_string());
        assert_eq!(diagnostic.state, McpDiagnosticState::Unreadable);
        assert!(!diagnostic.message.contains("API_KEY"));
        assert!(!diagnostic.message.contains("secret"));

        let registry = test_registry(McpTestAdapter {
            access: ToolCapabilityAccess::Unknown,
            source_path: String::new(),
            format: ToolCapabilityFormat::Unknown,
            result: Ok(vec![]),
        });
        assert_eq!(
            get_mcp_diagnostics_domain(&registry, "mcp-test".to_string()).state,
            McpDiagnosticState::Unknown
        );

        let registry = test_registry(McpTestAdapter {
            access: ToolCapabilityAccess::Unsupported,
            source_path: String::new(),
            format: ToolCapabilityFormat::Unknown,
            result: Ok(vec![]),
        });
        assert_eq!(
            get_mcp_diagnostics_domain(&registry, "mcp-test".to_string()).state,
            McpDiagnosticState::Unsupported
        );
    }

    #[test]
    fn config_sources_report_editability_and_summary_without_secret_values() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join("mcp.json");
        std::fs::write(
            &config,
            r#"{"mcpServers":{"demo":{"env":{"API_KEY":"secret"}}}}"#,
        )
        .unwrap();
        let registry = test_registry(McpTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: config.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Json,
            result: Ok(vec![server()]),
        });

        let sources = list_mcp_config_sources_domain(&registry, "mcp-test".to_string());

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].state, McpDiagnosticState::Loaded);
        assert!(sources[0].editable);
        assert_eq!(sources[0].server_count, Some(1));
        assert_eq!(sources[0].servers[0].env_keys, vec!["API_KEY".to_string()]);
        assert!(!format!("{sources:?}").contains("secret"));
    }

    #[test]
    fn custom_tool_mcp_config_projects_to_editable_source() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("custom-mcp.json");
        std::fs::write(
            &config_path,
            r#"{"mcpServers":{"demo":{"command":"node","env":{"API_KEY":"secret"}}}}"#,
        )
        .unwrap();
        let mut app_config = app_config::default_config();
        app_config
            .custom_tools
            .push(crate::platform::config::CustomTool {
                id: "custom-mcp".to_string(),
                name: "Custom MCP".to_string(),
                icon: "wrench".to_string(),
                config_dir: String::new(),
                rule_directory: String::new(),
                global_rule_file: String::new(),
                skills_dir: String::new(),
                shared_skill_direct_read: false,
                mcp_config: config_path.to_string_lossy().to_string(),
                tool_config: String::new(),
                rule_file: String::new(),
            });
        let registry = ToolRegistry::from_adapters_for_tests(vec![]);

        let sources = list_mcp_config_sources_for_config_domain(
            &registry,
            "custom-mcp".to_string(),
            &app_config,
        );

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, "user-configured-mcp-config");
        assert!(sources[0].editable);
        assert_eq!(sources[0].state, McpDiagnosticState::Loaded);
        assert_eq!(sources[0].server_count, Some(1));
        assert_eq!(
            sources[0].path.as_deref(),
            Some(config_path.to_string_lossy().as_ref())
        );
        assert!(!format!("{sources:?}").contains("secret"));
    }

    #[test]
    fn config_sources_hide_project_scope_sources_for_now() {
        let tmp = tempfile::tempdir().unwrap();
        let global_config = tmp.path().join("global-mcp.json");
        let project_config = tmp.path().join("project-mcp.json");
        std::fs::write(&global_config, "{}").unwrap();
        std::fs::write(&project_config, "{}").unwrap();
        let global_path = global_config.to_string_lossy().to_string();
        let project_path = project_config.to_string_lossy().to_string();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(MultiScopeMcpTestAdapter {
                global_path: global_path.clone(),
                project_path,
            })]);

        let sources = list_mcp_config_sources_domain(&registry, "mcp-multi-scope-test".to_string());

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, "global-mcp");
        assert_eq!(sources[0].path.as_deref(), Some(global_path.as_str()));
    }

    #[test]
    fn trae_cn_declared_mcp_source_lists_reads_saves_and_excludes_project_evidence() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp
            .path()
            .join("Library/Application Support/Trae CN/User/mcp.json");
        std::fs::create_dir_all(config.parent().unwrap()).unwrap();
        std::fs::write(
            &config,
            r#"{"mcpServers":{"demo":{"command":"node","env":{"API_KEY":"secret"}},"other":{"command":"python"}}}"#,
        )
        .unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![crate::adapters::trae_cn::create(
                tmp.path(),
            )]);

        let sources = list_mcp_config_sources_domain(&registry, "trae-cn".to_string());
        let source = sources
            .iter()
            .find(|source| source.id == "mcp-config")
            .expect("expected Trae CN MCP source");

        assert_eq!(source.state, McpDiagnosticState::Loaded);
        assert!(source.editable);
        assert_eq!(source.server_count, Some(2));
        assert_eq!(
            source.path.as_deref(),
            Some(config.to_string_lossy().as_ref())
        );
        assert!(!format!("{sources:?}").contains("secret"));
        assert!(sources
            .iter()
            .all(|source| source.id != "project-mcp-config"));

        let fragment = read_mcp_server_config_fragment_domain(
            &registry,
            "trae-cn".to_string(),
            "mcp-config".to_string(),
            "demo".to_string(),
        )
        .unwrap();
        assert!(fragment.editable);
        assert!(fragment.content.contains("\"command\": \"node\""));
        assert!(!fragment.content.contains("python"));

        let invalid = save_mcp_server_config_fragment_domain(
            &registry,
            "trae-cn".to_string(),
            "mcp-config".to_string(),
            "demo".to_string(),
            "{".to_string(),
        );
        assert!(invalid.is_err());

        let result = save_mcp_server_config_fragment_domain(
            &registry,
            "trae-cn".to_string(),
            "mcp-config".to_string(),
            "demo".to_string(),
            r#"{"command":"deno"}"#.to_string(),
        )
        .unwrap();
        let saved = std::fs::read_to_string(&config).unwrap();

        assert!(result.backup_path.contains("trae-cn"));
        assert!(saved.contains("\"command\": \"deno\""));
        assert!(saved.contains("\"command\": \"python\""));
        assert!(!saved.contains("\"command\": \"node\""));
    }

    #[test]
    fn trae_solo_declared_mcp_source_uses_servers_locator() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp
            .path()
            .join("Library/Application Support/TRAE SOLO/User/mcp.json");
        std::fs::create_dir_all(config.parent().unwrap()).unwrap();
        std::fs::write(
            &config,
            r#"{"servers":{"demo":{"command":"node","env":{"API_KEY":"secret"}},"other":{"command":"python"}},"inputs":[]}"#,
        )
        .unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![crate::adapters::trae_solo::create(
                tmp.path(),
            )]);

        let sources = list_mcp_config_sources_domain(&registry, "trae-solo".to_string());
        let source = sources
            .iter()
            .find(|source| source.id == "mcp-config")
            .expect("expected Trae Solo MCP source");

        assert_eq!(source.state, McpDiagnosticState::Loaded);
        assert!(source.editable);
        assert_eq!(source.server_count, Some(2));
        assert_eq!(
            source.path.as_deref(),
            Some(config.to_string_lossy().as_ref())
        );
        assert!(!format!("{sources:?}").contains("secret"));

        let fragment = read_mcp_server_config_fragment_domain(
            &registry,
            "trae-solo".to_string(),
            "mcp-config".to_string(),
            "demo".to_string(),
        )
        .unwrap();
        assert!(fragment.editable);
        assert!(fragment.content.contains("\"command\": \"node\""));
        assert!(!fragment.content.contains("python"));

        let result = save_mcp_server_config_fragment_domain(
            &registry,
            "trae-solo".to_string(),
            "mcp-config".to_string(),
            "demo".to_string(),
            r#"{"command":"deno"}"#.to_string(),
        )
        .unwrap();
        let saved = std::fs::read_to_string(&config).unwrap();

        assert!(result.backup_path.contains("trae-solo"));
        assert!(saved.contains("\"servers\""));
        assert!(saved.contains("\"command\": \"deno\""));
        assert!(saved.contains("\"command\": \"python\""));
        assert!(!saved.contains("\"command\": \"node\""));
    }

    #[test]
    fn windsurf_declared_mcp_source_lists_reads_and_saves_without_lifecycle_actions() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join(".codeium/windsurf/mcp_config.json");
        std::fs::create_dir_all(config.parent().unwrap()).unwrap();
        std::fs::write(
            &config,
            r#"{"mcpServers":{"demo":{"command":"node","disabled":true,"env":{"API_KEY":"secret"}},"other":{"command":"python"}}}"#,
        )
        .unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![crate::adapters::windsurf::create(
                tmp.path(),
            )]);
        let app_config = app_config::default_config();

        let sources = list_mcp_config_sources_for_config_domain(
            &registry,
            "windsurf".to_string(),
            &app_config,
        );
        let source = sources
            .iter()
            .find(|source| source.id == "mcp-config")
            .expect("expected Windsurf MCP source");

        assert_eq!(source.state, McpDiagnosticState::Loaded);
        assert!(source.editable);
        assert_eq!(source.server_count, Some(2));
        assert_eq!(
            source.path.as_deref(),
            Some(config.to_string_lossy().as_ref())
        );
        assert_eq!(source.servers[0].env_keys, vec!["API_KEY".to_string()]);
        assert!(!format!("{sources:?}").contains("secret"));

        let fragment = read_mcp_server_config_fragment_for_config_domain(
            &registry,
            "windsurf".to_string(),
            "mcp-config".to_string(),
            "demo".to_string(),
            &app_config,
        )
        .unwrap();
        assert!(fragment.editable);
        assert!(fragment.content.contains("\"command\": \"node\""));
        assert!(fragment.content.contains("\"disabled\": true"));
        assert!(!fragment.content.contains("python"));

        let invalid = save_mcp_server_config_fragment_for_config_domain(
            &registry,
            "windsurf".to_string(),
            "mcp-config".to_string(),
            "demo".to_string(),
            "{".to_string(),
            &app_config,
        );
        assert!(invalid.is_err());

        let result = save_mcp_server_config_fragment_for_config_domain(
            &registry,
            "windsurf".to_string(),
            "mcp-config".to_string(),
            "demo".to_string(),
            r#"{"command":"deno"}"#.to_string(),
            &app_config,
        )
        .unwrap();
        let saved = std::fs::read_to_string(&config).unwrap();

        assert!(result.backup_path.contains("windsurf"));
        assert!(saved.contains("\"command\": \"deno\""));
        assert!(saved.contains("\"command\": \"python\""));
        assert!(!saved.contains("\"command\": \"node\""));
    }

    #[test]
    fn kiro_declared_mcp_source_uses_agent_config_not_ide_shell_path() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join(".kiro/settings/mcp.json");
        let shell_config = tmp
            .path()
            .join("Library/Application Support/Kiro/User/mcp.json");
        std::fs::create_dir_all(config.parent().unwrap()).unwrap();
        std::fs::create_dir_all(shell_config.parent().unwrap()).unwrap();
        std::fs::write(
            &config,
            r#"{"mcpServers":{"demo":{"command":"node","env":{"API_KEY":"secret"}},"other":{"command":"python"}}}"#,
        )
        .unwrap();
        std::fs::write(
            &shell_config,
            r#"{"mcpServers":{"shell-only":{"command":"ruby"}}}"#,
        )
        .unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![crate::adapters::kiro::create(tmp.path())]);

        let sources = list_mcp_config_sources_domain(&registry, "kiro".to_string());
        let source = sources
            .iter()
            .find(|source| source.id == "mcp-config")
            .expect("expected Kiro MCP source");

        assert_eq!(sources.len(), 1);
        assert_eq!(source.state, McpDiagnosticState::Loaded);
        assert!(source.editable);
        assert_eq!(source.server_count, Some(2));
        assert_eq!(
            source.path.as_deref(),
            Some(config.to_string_lossy().as_ref())
        );
        assert_eq!(source.servers[0].env_keys, vec!["API_KEY".to_string()]);
        assert!(!format!("{sources:?}").contains("secret"));
        assert!(!format!("{sources:?}").contains("shell-only"));
        assert!(!format!("{sources:?}").contains("Application Support"));

        let fragment = read_mcp_server_config_fragment_domain(
            &registry,
            "kiro".to_string(),
            "mcp-config".to_string(),
            "demo".to_string(),
        )
        .unwrap();
        assert!(fragment.editable);
        assert!(fragment.content.contains("\"command\": \"node\""));
        assert!(!fragment.content.contains("python"));
        assert!(!fragment.content.contains("shell-only"));

        let result = save_mcp_server_config_fragment_domain(
            &registry,
            "kiro".to_string(),
            "mcp-config".to_string(),
            "demo".to_string(),
            r#"{"command":"deno"}"#.to_string(),
        )
        .unwrap();
        let saved = std::fs::read_to_string(&config).unwrap();
        let shell_saved = std::fs::read_to_string(&shell_config).unwrap();

        assert!(result.backup_path.contains("kiro"));
        assert!(saved.contains("\"command\": \"deno\""));
        assert!(saved.contains("\"command\": \"python\""));
        assert!(!saved.contains("\"command\": \"node\""));
        assert!(shell_saved.contains("\"command\":\"ruby\""));
        assert!(!shell_saved.contains("deno"));
    }

    #[test]
    fn github_copilot_declared_mcp_source_lists_reads_and_saves() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join(".copilot/mcp-config.json");
        std::fs::create_dir_all(config.parent().unwrap()).unwrap();
        std::fs::write(
            &config,
            r#"{"mcpServers":{"demo":{"command":"node","args":["server"],"env":{"API_KEY":"secret"}},"other":{"command":"python"}}}"#,
        )
        .unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![crate::adapters::github_copilot::create(
                tmp.path(),
            )]);

        let sources = list_mcp_config_sources_domain(&registry, "github-copilot".to_string());
        let source = sources
            .iter()
            .find(|source| source.id == "mcp-config")
            .expect("expected GitHub Copilot MCP source");

        assert_eq!(source.state, McpDiagnosticState::Loaded);
        assert!(source.editable);
        assert_eq!(source.server_count, Some(2));
        assert_eq!(
            source.path.as_deref(),
            Some(config.to_string_lossy().as_ref())
        );
        assert_eq!(source.servers[0].env_keys, vec!["API_KEY".to_string()]);
        assert!(!format!("{sources:?}").contains("secret"));

        let fragment = read_mcp_server_config_fragment_domain(
            &registry,
            "github-copilot".to_string(),
            "mcp-config".to_string(),
            "demo".to_string(),
        )
        .unwrap();
        assert!(fragment.editable);
        assert!(fragment.content.contains("\"command\": \"node\""));
        assert!(fragment.content.contains("\"server\""));
        assert!(!fragment.content.contains("python"));

        let result = save_mcp_server_config_fragment_domain(
            &registry,
            "github-copilot".to_string(),
            "mcp-config".to_string(),
            "demo".to_string(),
            r#"{"command":"deno"}"#.to_string(),
        )
        .unwrap();
        let saved = std::fs::read_to_string(&config).unwrap();

        assert!(result.backup_path.contains("github-copilot"));
        assert!(saved.contains("\"command\": \"deno\""));
        assert!(saved.contains("\"command\": \"python\""));
        assert!(!saved.contains("\"command\": \"node\""));
    }

    #[test]
    fn pi_agent_without_mcp_adapter_reports_adapter_requirement() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join(".pi/agent/settings.json");
        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        std::fs::write(&settings, r#"{"packages":[]}"#).unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![crate::adapters::pi_agent::create(
                tmp.path(),
            )]);

        let sources = list_mcp_config_sources_domain(&registry, "pi-agent".to_string());

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, "mcp-adapter");
        assert_eq!(sources[0].label, "pi-mcp-adapter");
        assert_eq!(sources[0].state, McpDiagnosticState::Unsupported);
        assert!(!sources[0].editable);
        assert!(sources[0].path.is_none());
        assert_eq!(
            sources[0].required_adapter.as_ref().unwrap(),
            &McpAdapterRequirement {
                tool_name: "Pi Agent".to_string(),
                package_name: "pi-mcp-adapter".to_string(),
            }
        );
        assert_eq!(
            sources[0].message,
            "MCP configuration is not supported for this tool"
        );
    }

    #[test]
    fn pi_agent_with_mcp_adapter_lists_sources_independently() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join(".pi/agent/settings.json");
        let shared_config = tmp.path().join(".config/mcp/mcp.json");
        let pi_config = tmp.path().join(".pi/agent/mcp.json");
        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        std::fs::create_dir_all(shared_config.parent().unwrap()).unwrap();
        std::fs::create_dir_all(pi_config.parent().unwrap()).unwrap();
        std::fs::write(
            &settings,
            r#"{"packages":[{"source":"npm:pi-mcp-adapter"}]}"#,
        )
        .unwrap();
        std::fs::write(
            &shared_config,
            r#"{"mcpServers":{"shared-demo":{"command":"node-shared"}}}"#,
        )
        .unwrap();
        std::fs::write(
            &pi_config,
            r#"{"mcpServers":{"pi-demo":{"command":"node-pi"}}}"#,
        )
        .unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![crate::adapters::pi_agent::create(
                tmp.path(),
            )]);

        let sources = list_mcp_config_sources_domain(&registry, "pi-agent".to_string());
        let shared_source = sources
            .iter()
            .find(|source| source.id == "shared-mcp-config")
            .expect("expected shared Pi MCP source");
        let pi_source = sources
            .iter()
            .find(|source| source.id == "pi-mcp-config")
            .expect("expected Pi-specific MCP source");

        assert_eq!(sources.len(), 2);
        assert_eq!(shared_source.state, McpDiagnosticState::Loaded);
        assert_eq!(shared_source.server_count, Some(1));
        assert_eq!(shared_source.servers[0].name, "shared-demo");
        assert_eq!(pi_source.state, McpDiagnosticState::Loaded);
        assert_eq!(pi_source.server_count, Some(1));
        assert_eq!(pi_source.servers[0].name, "pi-demo");
        assert!(!shared_source.editable);
        assert!(!pi_source.editable);

        let fragment = read_mcp_server_config_fragment_domain(
            &registry,
            "pi-agent".to_string(),
            "pi-mcp-config".to_string(),
            "pi-demo".to_string(),
        )
        .unwrap();
        assert!(!fragment.editable);
        assert!(fragment.content.contains("\"command\": \"node-pi\""));
        assert!(!fragment.content.contains("node-shared"));

        let save_attempt = save_mcp_server_config_fragment_domain(
            &registry,
            "pi-agent".to_string(),
            "pi-mcp-config".to_string(),
            "pi-demo".to_string(),
            r#"{"command":"deno"}"#.to_string(),
        );
        assert!(save_attempt.is_err());
    }

    #[test]
    fn read_server_fragment_returns_selected_server_only() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join("mcp.json");
        std::fs::write(
            &config,
            r#"{"mcpServers":{"demo":{"command":"node"},"other":{"command":"python"}}}"#,
        )
        .unwrap();
        let registry = test_registry(McpTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: config.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Json,
            result: Ok(vec![server()]),
        });

        let fragment = read_mcp_server_config_fragment_domain(
            &registry,
            "mcp-test".to_string(),
            "mcp".to_string(),
            "demo".to_string(),
        )
        .unwrap();

        assert_eq!(fragment.server_name, "demo");
        assert!(fragment.content.contains("\"command\": \"node\""));
        assert!(!fragment.content.contains("mcpServers"));
        assert!(!fragment.content.contains("python"));
    }

    #[test]
    fn read_server_fragment_supports_top_level_mcp_hash_source() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join("opencode.json");
        std::fs::write(
            &config,
            r#"{"mcp":{"demo":{"command":"node"},"other":{"command":"python"}}}"#,
        )
        .unwrap();
        let registry = test_registry(McpTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: format!("{}#mcp", config.to_string_lossy()),
            format: ToolCapabilityFormat::Json,
            result: Ok(vec![server()]),
        });

        let fragment = read_mcp_server_config_fragment_domain(
            &registry,
            "mcp-test".to_string(),
            "mcp".to_string(),
            "demo".to_string(),
        )
        .unwrap();

        assert_eq!(fragment.server_name, "demo");
        assert!(fragment.content.contains("\"command\": \"node\""));
        assert!(!fragment.content.contains("\"mcp\""));
        assert!(!fragment.content.contains("python"));
    }

    #[test]
    fn save_server_fragment_validates_backs_up_and_replaces_only_selected_server() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join("mcp.json");
        std::fs::write(
            &config,
            r#"{"mcpServers":{"demo":{"command":"node"},"other":{"command":"python"}}}"#,
        )
        .unwrap();
        let registry = test_registry(McpTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: config.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Json,
            result: Ok(vec![server()]),
        });

        let result = save_mcp_server_config_fragment_domain(
            &registry,
            "mcp-test".to_string(),
            "mcp".to_string(),
            "demo".to_string(),
            r#"{"command":"deno"}"#.to_string(),
        )
        .unwrap();
        let saved = std::fs::read_to_string(&config).unwrap();

        assert!(result.backup_path.contains("mcp-config"));
        assert!(std::path::Path::new(&result.backup_path).exists());
        assert!(saved.contains("\"command\": \"deno\""));
        assert!(saved.contains("\"command\": \"python\""));
        assert!(!saved.contains("\"command\": \"node\""));
    }
}
