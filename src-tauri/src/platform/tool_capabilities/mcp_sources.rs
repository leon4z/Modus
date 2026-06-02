// Purpose: Shared MCP source parsing and fragment helpers for declared tool capabilities.

use super::skills::{McpServerActivationState, McpServerInfo};
use super::{
    parse_json_value_for_format, ToolCapability, ToolCapabilityAccess, ToolCapabilityFormat,
    ToolCapabilityKind, ToolCapabilityScope,
};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_MCP_LOCATORS: &[&[&str]] = &[
    &["mcpServers"],
    &["mcp_servers"],
    &["mcp", "servers"],
    &["mcp", "mcpServers"],
    &["mcp"],
];

pub fn concrete_source_path(raw: &str) -> Option<PathBuf> {
    let source = raw.split('#').next().unwrap_or("").trim();
    if source.is_empty() || source.contains('*') {
        return None;
    }
    let expanded = PathBuf::from(shellexpand::tilde(source).to_string());
    if expanded.is_absolute() {
        Some(expanded)
    } else {
        std::env::current_dir().ok().map(|cwd| cwd.join(expanded))
    }
}

fn source_locator(raw: &str) -> Option<String> {
    let (_, locator) = raw.split_once('#')?;
    let locator = locator.trim();
    if locator.is_empty() {
        return None;
    }
    let locator = locator
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(locator)
        .trim();
    if locator.is_empty() {
        None
    } else {
        Some(locator.to_string())
    }
}

fn locator_segments(locator: &str) -> Vec<String> {
    locator
        .split('.')
        .map(|segment| segment.trim())
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_string())
        .collect()
}

fn candidate_paths(raw_source_path: &str) -> Vec<Vec<String>> {
    if let Some(locator) = source_locator(raw_source_path) {
        let segments = locator_segments(&locator);
        if !segments.is_empty() {
            return vec![segments];
        }
    }
    DEFAULT_MCP_LOCATORS
        .iter()
        .map(|path| path.iter().map(|segment| segment.to_string()).collect())
        .collect()
}

fn json_value_at_path<'a>(value: &'a JsonValue, path: &[String]) -> Option<&'a JsonValue> {
    let mut current = value;
    for segment in path {
        current = current.get(segment)?;
    }
    Some(current)
}

fn json_value_at_path_mut<'a>(
    value: &'a mut JsonValue,
    path: &[String],
) -> Option<&'a mut JsonValue> {
    let mut current = value;
    for segment in path {
        current = current.get_mut(segment)?;
    }
    Some(current)
}

fn json_mcp_servers<'a>(
    value: &'a JsonValue,
    raw_source_path: &str,
) -> Option<(&'a JsonMap<String, JsonValue>, Vec<String>)> {
    for path in candidate_paths(raw_source_path) {
        if let Some(servers) = json_value_at_path(value, &path).and_then(|value| value.as_object())
        {
            return Some((servers, path));
        }
    }
    None
}

fn json_mcp_servers_mut<'a>(
    value: &'a mut JsonValue,
    raw_source_path: &str,
) -> Option<(&'a mut JsonMap<String, JsonValue>, Vec<String>)> {
    for path in candidate_paths(raw_source_path) {
        if json_value_at_path(value, &path)
            .and_then(|value| value.as_object())
            .is_some()
        {
            let servers = json_value_at_path_mut(value, &path)?.as_object_mut()?;
            return Some((servers, path));
        }
    }
    None
}

fn parse_mcp_server(name: &str, value: &JsonValue) -> McpServerInfo {
    let server_type = value
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("stdio")
        .to_string();
    let command = value
        .get("command")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let args = value
        .get("args")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|value| value.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let mut env_keys: Vec<String> = value
        .get("env")
        .and_then(|value| value.as_object())
        .map(|env| env.keys().cloned().collect())
        .unwrap_or_default();
    env_keys.sort();
    let url = value
        .get("url")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let description = value
        .get("description")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let enabled = value
        .get("enabled")
        .and_then(|value| value.as_bool())
        .unwrap_or_else(|| {
            !value
                .get("disabled")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        });

    McpServerInfo {
        name: name.to_string(),
        server_type,
        command,
        args,
        env_keys,
        url,
        enabled,
        activation_state: if enabled {
            McpServerActivationState::Enabled
        } else {
            McpServerActivationState::Disabled
        },
        description,
    }
}

fn parse_servers_from_json_value(value: &JsonValue, raw_source_path: &str) -> Vec<McpServerInfo> {
    json_mcp_servers(value, raw_source_path)
        .map(|(servers, _)| {
            servers
                .iter()
                .map(|(name, server)| parse_mcp_server(name, server))
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_mcp_servers_for_capability(
    capability: &ToolCapability,
) -> Result<Vec<McpServerInfo>, String> {
    if capability.kind != ToolCapabilityKind::Mcp
        || capability.scope == ToolCapabilityScope::Project
        || !matches!(
            capability.access,
            ToolCapabilityAccess::Writable
                | ToolCapabilityAccess::ReadOnly
                | ToolCapabilityAccess::Readable
        )
    {
        return Ok(vec![]);
    }
    let Some(path) = concrete_source_path(&capability.source_path) else {
        return Ok(vec![]);
    };
    if !path.exists() {
        return Ok(vec![]);
    }
    parse_mcp_servers_file(&path, &capability.format, &capability.source_path)
}

fn parse_mcp_servers_file(
    path: &Path,
    format: &ToolCapabilityFormat,
    raw_source_path: &str,
) -> Result<Vec<McpServerInfo>, String> {
    let content = fs::read_to_string(path).map_err(|error| format!("unreadable: {}", error))?;
    match format {
        ToolCapabilityFormat::Json | ToolCapabilityFormat::Jsonc => {
            let value = parse_json_value_for_format(format, &content)
                .map_err(|error| format!("malformed: {}", error))?;
            Ok(parse_servers_from_json_value(&value, raw_source_path))
        }
        ToolCapabilityFormat::Unknown => {
            let value: JsonValue =
                serde_json::from_str(&content).map_err(|error| format!("malformed: {}", error))?;
            Ok(parse_servers_from_json_value(&value, raw_source_path))
        }
        ToolCapabilityFormat::Toml => {
            let value: toml::Value = content
                .parse()
                .map_err(|error: toml::de::Error| format!("malformed: {}", error))?;
            let json =
                serde_json::to_value(value).map_err(|error| format!("malformed: {}", error))?;
            Ok(parse_servers_from_json_value(&json, raw_source_path))
        }
        ToolCapabilityFormat::Yaml => {
            let value: serde_yaml::Value =
                serde_yaml::from_str(&content).map_err(|error| format!("malformed: {}", error))?;
            let json =
                serde_json::to_value(value).map_err(|error| format!("malformed: {}", error))?;
            Ok(parse_servers_from_json_value(&json, raw_source_path))
        }
        _ => Ok(vec![]),
    }
}

fn toml_value_at_path<'a>(value: &'a toml::Value, path: &[String]) -> Option<&'a toml::Value> {
    let mut current = value;
    for segment in path {
        current = current.get(segment)?;
    }
    Some(current)
}

fn toml_value_at_path_mut<'a>(
    value: &'a mut toml::Value,
    path: &[String],
) -> Option<&'a mut toml::Value> {
    let mut current = value;
    for segment in path {
        current = current.get_mut(segment)?;
    }
    Some(current)
}

fn toml_mcp_servers<'a>(
    value: &'a toml::Value,
    raw_source_path: &str,
) -> Option<(&'a toml::value::Table, Vec<String>)> {
    for path in candidate_paths(raw_source_path) {
        if let Some(servers) = toml_value_at_path(value, &path).and_then(|value| value.as_table()) {
            return Some((servers, path));
        }
    }
    None
}

fn toml_mcp_servers_mut<'a>(
    value: &'a mut toml::Value,
    raw_source_path: &str,
) -> Option<(&'a mut toml::value::Table, Vec<String>)> {
    for path in candidate_paths(raw_source_path) {
        if toml_value_at_path(value, &path)
            .and_then(|value| value.as_table())
            .is_some()
        {
            let servers = toml_value_at_path_mut(value, &path)?.as_table_mut()?;
            return Some((servers, path));
        }
    }
    None
}

fn nested_toml_fragment(path: &[String], server_name: &str, server: toml::Value) -> toml::Value {
    let mut current = toml::Value::Table({
        let mut table = toml::value::Table::new();
        table.insert(server_name.to_string(), server);
        table
    });
    for segment in path.iter().rev() {
        let mut table = toml::value::Table::new();
        table.insert(segment.clone(), current);
        current = toml::Value::Table(table);
    }
    current
}

fn toml_fragment_server(
    fragment: &toml::Value,
    raw_source_path: &str,
    server_name: &str,
) -> Option<toml::Value> {
    toml_mcp_servers(fragment, raw_source_path)
        .and_then(|(servers, _)| servers.get(server_name).cloned())
        .or_else(|| {
            candidate_paths("").into_iter().find_map(|path| {
                toml_value_at_path(fragment, &path)
                    .and_then(|value| value.as_table())
                    .and_then(|servers| servers.get(server_name))
                    .cloned()
            })
        })
        .or_else(|| {
            let table = fragment.as_table()?;
            if table.contains_key("command")
                || table.contains_key("type")
                || table.contains_key("url")
            {
                Some(fragment.clone())
            } else {
                None
            }
        })
}

pub fn extract_server_fragment(
    format: &ToolCapabilityFormat,
    raw_source_path: &str,
    content: &str,
    server_name: &str,
) -> Result<String, String> {
    match format {
        ToolCapabilityFormat::Json | ToolCapabilityFormat::Jsonc => {
            let value = parse_json_value_for_format(format, content)?;
            let server = json_mcp_servers(&value, raw_source_path)
                .and_then(|(servers, _)| servers.get(server_name))
                .ok_or("MCP server configuration was not found")?;
            serde_json::to_string_pretty(server).map_err(|error| error.to_string())
        }
        ToolCapabilityFormat::Toml => {
            let value: toml::Value = content
                .parse()
                .map_err(|error: toml::de::Error| format!("Invalid TOML: {}", error))?;
            let (servers, path) = toml_mcp_servers(&value, raw_source_path)
                .ok_or("MCP server collection was not found")?;
            let server = servers
                .get(server_name)
                .ok_or("MCP server configuration was not found")?;
            toml::to_string_pretty(&nested_toml_fragment(&path, server_name, server.clone()))
                .map_err(|error| error.to_string())
        }
        ToolCapabilityFormat::Yaml => {
            let value: JsonValue = serde_yaml::from_str(content)
                .map_err(|error| format!("Invalid YAML: {}", error))?;
            let server = json_mcp_servers(&value, raw_source_path)
                .and_then(|(servers, _)| servers.get(server_name))
                .ok_or("MCP server configuration was not found")?;
            serde_yaml::to_string(server).map_err(|error| error.to_string())
        }
        _ => Err("Unsupported MCP configuration format".to_string()),
    }
}

pub fn replace_server_fragment(
    format: &ToolCapabilityFormat,
    raw_source_path: &str,
    full_content: &str,
    server_name: &str,
    fragment_content: &str,
) -> Result<String, String> {
    match format {
        ToolCapabilityFormat::Json | ToolCapabilityFormat::Jsonc => {
            let mut full = parse_json_value_for_format(format, full_content)
                .map_err(|error| format!("Invalid source: {}", error))?;
            let next = parse_json_value_for_format(format, fragment_content)?;
            let (servers, _) = json_mcp_servers_mut(&mut full, raw_source_path)
                .ok_or("MCP server collection was not found")?;
            if !servers.contains_key(server_name) {
                return Err("MCP server configuration was not found".to_string());
            }
            servers.insert(server_name.to_string(), next);
            serde_json::to_string_pretty(&full).map_err(|error| error.to_string())
        }
        ToolCapabilityFormat::Toml => {
            let mut full: toml::Value = full_content
                .parse()
                .map_err(|error: toml::de::Error| format!("Invalid source TOML: {}", error))?;
            let fragment: toml::Value = fragment_content
                .parse()
                .map_err(|error: toml::de::Error| format!("Invalid TOML: {}", error))?;
            let next = toml_fragment_server(&fragment, raw_source_path, server_name)
                .ok_or("Edited TOML must include the selected MCP server table")?;
            let (servers, _) = toml_mcp_servers_mut(&mut full, raw_source_path)
                .ok_or("MCP server collection was not found")?;
            if !servers.contains_key(server_name) {
                return Err("MCP server configuration was not found".to_string());
            }
            servers.insert(server_name.to_string(), next);
            toml::to_string_pretty(&full).map_err(|error| error.to_string())
        }
        ToolCapabilityFormat::Yaml => {
            let mut full: JsonValue = serde_yaml::from_str(full_content)
                .map_err(|error| format!("Invalid source YAML: {}", error))?;
            let next: JsonValue = serde_yaml::from_str(fragment_content)
                .map_err(|error| format!("Invalid YAML: {}", error))?;
            let (servers, _) = json_mcp_servers_mut(&mut full, raw_source_path)
                .ok_or("MCP server collection was not found")?;
            if !servers.contains_key(server_name) {
                return Err("MCP server configuration was not found".to_string());
            }
            servers.insert(server_name.to_string(), next);
            serde_yaml::to_string(&full).map_err(|error| error.to_string())
        }
        _ => Err("Unsupported MCP configuration format for saving".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::tool_capabilities::{
        ToolCapabilitySourceConfidence, ToolCapabilitySourceKind, ToolSourceDiagnosticState,
    };

    fn mcp_capability(path: String, format: ToolCapabilityFormat) -> ToolCapability {
        ToolCapability {
            id: "mcp".to_string(),
            kind: ToolCapabilityKind::Mcp,
            scope: ToolCapabilityScope::Global,
            access: ToolCapabilityAccess::ReadOnly,
            format,
            source_path: path,
            label: "MCP".to_string(),
            diagnostics: vec![ToolSourceDiagnosticState::Loaded],
            source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
            notes: String::new(),
            source_kind: ToolCapabilitySourceKind::FeatureSource,
            primary_config_dir: None,
            supporting_sources: vec![],
            action_evidence: vec![],
        }
    }

    #[test]
    fn parses_json_locator_without_env_values() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        fs::write(
            &path,
            r#"{"mcpServers":{"demo":{"command":"node","env":{"API_KEY":"secret"}}}}"#,
        )
        .unwrap();
        let servers = parse_mcp_servers_for_capability(&mcp_capability(
            format!("{}#mcpServers", path.to_string_lossy()),
            ToolCapabilityFormat::Json,
        ))
        .unwrap();

        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "demo");
        assert_eq!(servers[0].env_keys, vec!["API_KEY".to_string()]);
        assert!(!format!("{servers:?}").contains("secret"));
    }

    #[test]
    fn parses_jsonc_locator_and_fragment_roundtrip() {
        let content = r#"{
  // OpenCode user config
  "mcp": {
    "demo": {
      "command": "node",
      "args": [
        "server.js",
      ],
    },
    "other": {
      "command": "python"
    },
  },
}"#;
        let servers = {
            let tmp = tempfile::tempdir().unwrap();
            let path = tmp.path().join("opencode.jsonc");
            fs::write(&path, content).unwrap();
            parse_mcp_servers_for_capability(&mcp_capability(
                format!("{}#mcp", path.to_string_lossy()),
                ToolCapabilityFormat::Jsonc,
            ))
            .unwrap()
        };

        assert_eq!(servers.len(), 2);
        assert!(servers.iter().any(|server| server.name == "demo"));

        let fragment =
            extract_server_fragment(&ToolCapabilityFormat::Jsonc, "#mcp", content, "demo").unwrap();
        assert!(fragment.contains("\"server.js\""));
        assert!(!fragment.contains("python"));

        let saved = replace_server_fragment(
            &ToolCapabilityFormat::Jsonc,
            "#mcp",
            content,
            "demo",
            r#"{
  // edited fragment
  "command": "deno",
}"#,
        )
        .unwrap();
        assert!(saved.contains("\"command\": \"deno\""));
        assert!(saved.contains("\"command\": \"python\""));
        assert!(!saved.contains("\"command\": \"node\""));
    }

    #[test]
    fn parses_toml_bracket_locator_and_fragment_roundtrip() {
        let content = r#"
[mcp.demo]
command = "node"

[mcp.other]
command = "python"
"#;
        let servers = {
            let tmp = tempfile::tempdir().unwrap();
            let path = tmp.path().join("config.toml");
            fs::write(&path, content).unwrap();
            parse_mcp_servers_for_capability(&mcp_capability(
                format!("{}#[mcp]", path.to_string_lossy()),
                ToolCapabilityFormat::Toml,
            ))
            .unwrap()
        };

        assert_eq!(servers.len(), 2);
        assert!(servers.iter().any(|server| server.name == "demo"));

        let fragment =
            extract_server_fragment(&ToolCapabilityFormat::Toml, "#[mcp]", content, "demo")
                .unwrap();
        assert!(fragment.contains("[mcp.demo]"));
        assert!(!fragment.contains("python"));

        let saved = replace_server_fragment(
            &ToolCapabilityFormat::Toml,
            "#[mcp]",
            content,
            "demo",
            r#"[mcp.demo]
command = "deno"
"#,
        )
        .unwrap();
        assert!(saved.contains("command = \"deno\""));
        assert!(saved.contains("command = \"python\""));
        assert!(!saved.contains("command = \"node\""));
    }

    #[test]
    fn concrete_source_path_keeps_project_relative_paths() {
        let path = concrete_source_path(".cursor/mcp.json").expect("expected relative path");

        assert!(path.ends_with(".cursor/mcp.json"));
    }
}
