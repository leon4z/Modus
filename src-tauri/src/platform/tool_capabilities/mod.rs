// Purpose: Shared tool capability records, projections, source discovery, and Skill/MCP shapes.

pub mod capability_projections;
pub mod effective_capabilities;
pub mod mcp_sources;
pub mod rule_sources;
pub mod skills;

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleFormat {
    SingleMarkdown,
    DirectoryMarkdown,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSource {
    pub path: String,
    pub format: RuleFormat,
    pub content: String,
    pub last_modified: u64,
    pub label: String,
    pub group: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<ToolSourceDiagnosticState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapabilityKind {
    Rule,
    Skill,
    Mcp,
    OrdinaryConfig,
    ProjectAsset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapabilityScope {
    Global,
    Project,
    Shared,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapabilityAccess {
    Writable,
    ReadOnly,
    Readable,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapabilityFormat {
    Markdown,
    InstructionsMarkdown,
    Mdc,
    Json,
    Jsonc,
    Toml,
    Yaml,
    Directory,
    SkillDirectory,
    Unknown,
}

pub fn parse_json_value_for_format(
    format: &ToolCapabilityFormat,
    content: &str,
) -> Result<serde_json::Value, String> {
    match format {
        ToolCapabilityFormat::Json => {
            serde_json::from_str(content).map_err(|error| format!("Invalid JSON: {}", error))
        }
        ToolCapabilityFormat::Jsonc => parse_jsonc_value(content),
        _ => Err("Unsupported JSON-like configuration format".to_string()),
    }
}

fn parse_jsonc_value(content: &str) -> Result<serde_json::Value, String> {
    let without_comments = strip_jsonc_comments(content)?;
    let normalized = strip_jsonc_trailing_commas(&without_comments);
    serde_json::from_str(&normalized).map_err(|error| format!("Invalid JSONC: {}", error))
}

fn strip_jsonc_comments(content: &str) -> Result<String, String> {
    let chars: Vec<char> = content.chars().collect();
    let mut output = String::with_capacity(content.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;

    while index < chars.len() {
        let ch = chars[index];
        if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            output.push(ch);
            index += 1;
            continue;
        }

        if ch == '/' && index + 1 < chars.len() && chars[index + 1] == '/' {
            index += 2;
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            if index < chars.len() {
                output.push('\n');
                index += 1;
            }
            continue;
        }

        if ch == '/' && index + 1 < chars.len() && chars[index + 1] == '*' {
            index += 2;
            let mut closed = false;
            while index < chars.len() {
                if index + 1 < chars.len() && chars[index] == '*' && chars[index + 1] == '/' {
                    closed = true;
                    index += 2;
                    break;
                }
                if chars[index] == '\n' {
                    output.push('\n');
                }
                index += 1;
            }
            if !closed {
                return Err("Invalid JSONC: unclosed block comment".to_string());
            }
            continue;
        }

        output.push(ch);
        index += 1;
    }

    Ok(output)
}

fn strip_jsonc_trailing_commas(content: &str) -> String {
    let chars: Vec<char> = content.chars().collect();
    let mut output = String::with_capacity(content.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;

    while index < chars.len() {
        let ch = chars[index];
        if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            output.push(ch);
            index += 1;
            continue;
        }

        if ch == ',' {
            let mut lookahead = index + 1;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }
            if lookahead < chars.len() && matches!(chars[lookahead], '}' | ']') {
                index += 1;
                continue;
            }
        }

        output.push(ch);
        index += 1;
    }

    output
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapabilitySourceConfidence {
    OfficialDocs,
    OfficialRepository,
    CertifiedLocalProductBehavior,
    OfficialCommunity,
    ThirdParty,
    LocalObservation,
    UserConfigured,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapabilitySourceKind {
    PrimaryConfigDirectory,
    FeatureSource,
    SupportingSource,
}

impl Default for ToolCapabilitySourceKind {
    fn default() -> Self {
        Self::FeatureSource
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapabilitySupportingSourceRole {
    Content,
    Metadata,
    Registry,
    Index,
    Manifest,
    Settings,
    PrimaryConfig,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCapabilitySupportingSource {
    pub id: String,
    pub role: ToolCapabilitySupportingSourceRole,
    pub source_path: String,
    pub format: ToolCapabilityFormat,
    #[serde(default)]
    pub required: bool,
    pub diagnostics: Vec<ToolSourceDiagnosticState>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapabilityAction {
    View,
    Read,
    Diagnose,
    Create,
    Edit,
    Save,
    Install,
    Copy,
    Uninstall,
    Delete,
    Update,
    Repair,
    Link,
    Sync,
    Inject,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCapabilityActionEvidence {
    pub action: ToolCapabilityAction,
    #[serde(default = "default_action_supported")]
    pub supported: bool,
    pub evidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
}

fn default_action_supported() -> bool {
    true
}

pub fn runtime_action_gates(
    feature: &str,
    source: &str,
    actions: &[ToolCapabilityAction],
) -> Vec<ToolCapabilityActionEvidence> {
    actions
        .iter()
        .map(|action| ToolCapabilityActionEvidence {
            action: action.clone(),
            supported: true,
            evidence: format!("{feature} action {:?} is supported for {source}.", action),
            variant: None,
            version: None,
            verified_at: None,
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolSourceDiagnosticState {
    Missing,
    Empty,
    Loaded,
    Unreadable,
    Malformed,
    Inconsistent,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCapability {
    pub id: String,
    pub kind: ToolCapabilityKind,
    pub scope: ToolCapabilityScope,
    pub access: ToolCapabilityAccess,
    pub format: ToolCapabilityFormat,
    pub source_path: String,
    pub label: String,
    pub diagnostics: Vec<ToolSourceDiagnosticState>,
    pub source_confidence: ToolCapabilitySourceConfidence,
    pub notes: String,
    #[serde(default)]
    pub source_kind: ToolCapabilitySourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supporting_sources: Vec<ToolCapabilitySupportingSource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_evidence: Vec<ToolCapabilityActionEvidence>,
}

impl ToolCapability {
    pub fn is_writable(&self) -> bool {
        self.access == ToolCapabilityAccess::Writable
    }

    pub fn is_readable(&self) -> bool {
        matches!(
            self.access,
            ToolCapabilityAccess::Writable
                | ToolCapabilityAccess::ReadOnly
                | ToolCapabilityAccess::Readable
        )
    }

    pub fn has_action_evidence(&self) -> bool {
        !self.action_evidence.is_empty()
    }

    pub fn has_supported_action(&self, action: &ToolCapabilityAction) -> bool {
        self.action_evidence
            .iter()
            .any(|evidence| evidence.supported && &evidence.action == action)
    }

    pub fn has_compound_supporting_sources(&self) -> bool {
        !self.supporting_sources.is_empty()
    }
}

pub(crate) fn normalized_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

pub fn capability_declared_source_path(capability: &ToolCapability) -> Option<PathBuf> {
    let source = capability
        .source_path
        .split('#')
        .next()
        .unwrap_or("")
        .trim();
    if source.is_empty() || source.contains('*') {
        return None;
    }
    Some(normalized_path(&PathBuf::from(
        shellexpand::tilde(source).to_string(),
    )))
}

pub(crate) fn capability_source_is_directory(capability: &ToolCapability, source: &Path) -> bool {
    matches!(
        capability.format,
        ToolCapabilityFormat::Directory
            | ToolCapabilityFormat::InstructionsMarkdown
            | ToolCapabilityFormat::SkillDirectory
    ) || source.is_dir()
        || (!source.is_file() && source.extension().is_none())
}

pub fn capability_matches_path(capability: &ToolCapability, path: &Path) -> bool {
    let Some(source) = capability_declared_source_path(capability) else {
        return false;
    };
    let target = normalized_path(path);
    if capability_source_is_directory(capability, &source) {
        return target.starts_with(source);
    }
    target == source
}

fn capability_source_is_trusted_for_global_rule_injection(
    adapter_id: &str,
    capability: &ToolCapability,
) -> bool {
    matches!(
        capability.source_confidence,
        ToolCapabilitySourceConfidence::OfficialDocs
            | ToolCapabilitySourceConfidence::OfficialRepository
            | ToolCapabilitySourceConfidence::UserConfigured
    ) || (capability.source_confidence
        == ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior
        && capability.has_supported_action(&ToolCapabilityAction::Inject))
        || (adapter_id == "openclaw" && capability.id == "trusted-workspace-agents")
}

pub fn capability_is_eligible_global_rule_target(
    adapter_id: &str,
    capability: &ToolCapability,
) -> bool {
    let Some(source) = capability_declared_source_path(capability) else {
        return false;
    };
    capability.kind == ToolCapabilityKind::Rule
        && capability.scope == ToolCapabilityScope::Global
        && capability.access == ToolCapabilityAccess::Writable
        && capability_source_is_trusted_for_global_rule_injection(adapter_id, capability)
        && !capability_source_is_directory(capability, &source)
}

pub(crate) fn capability_matches_eligible_global_rule_target(
    adapter_id: &str,
    capability: &ToolCapability,
    path: &Path,
) -> bool {
    if !capability_is_eligible_global_rule_target(adapter_id, capability) {
        return false;
    }
    let Some(source) = capability_declared_source_path(capability) else {
        return false;
    };
    let target = normalized_path(path);
    target == source
}

pub fn tool_capability(
    id: &str,
    kind: ToolCapabilityKind,
    scope: ToolCapabilityScope,
    access: ToolCapabilityAccess,
    format: ToolCapabilityFormat,
    source_path: impl Into<String>,
    label: &str,
    source_confidence: ToolCapabilitySourceConfidence,
    notes: &str,
) -> ToolCapability {
    ToolCapability {
        id: id.to_string(),
        kind,
        scope,
        access,
        format,
        source_path: source_path.into(),
        label: label.to_string(),
        diagnostics: vec![
            ToolSourceDiagnosticState::Missing,
            ToolSourceDiagnosticState::Empty,
            ToolSourceDiagnosticState::Loaded,
            ToolSourceDiagnosticState::Unreadable,
            ToolSourceDiagnosticState::Malformed,
        ],
        source_confidence,
        notes: notes.to_string(),
        source_kind: ToolCapabilitySourceKind::FeatureSource,
        primary_config_dir: None,
        supporting_sources: vec![],
        action_evidence: vec![],
    }
}

pub fn directory_capability(
    id: &str,
    kind: ToolCapabilityKind,
    scope: ToolCapabilityScope,
    access: ToolCapabilityAccess,
    source_path: impl Into<String>,
    label: &str,
    source_confidence: ToolCapabilitySourceConfidence,
    notes: &str,
) -> ToolCapability {
    ToolCapability {
        id: id.to_string(),
        kind,
        scope,
        access,
        format: ToolCapabilityFormat::Directory,
        source_path: source_path.into(),
        label: label.to_string(),
        diagnostics: vec![
            ToolSourceDiagnosticState::Missing,
            ToolSourceDiagnosticState::Loaded,
            ToolSourceDiagnosticState::Unreadable,
        ],
        source_confidence,
        notes: notes.to_string(),
        source_kind: ToolCapabilitySourceKind::FeatureSource,
        primary_config_dir: None,
        supporting_sources: vec![],
        action_evidence: vec![],
    }
}

pub fn capability_existing_source_file(capability: &ToolCapability) -> Option<PathBuf> {
    let source = capability_declared_source_path(capability)?;
    if source.is_file() {
        return Some(source);
    }
    if source.is_dir() {
        return Some(source);
    }
    if source.exists() {
        return Some(source);
    }
    None
}

pub fn get_file_modified(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs())
        .unwrap_or(0)
}
