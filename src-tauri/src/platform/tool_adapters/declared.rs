// Purpose: Declarative adapters for tools whose first support phase is capability discovery.

use super::{
    adapter_can_write_path, command_exists, discover_rule_sources_for_adapter, get_file_modified,
    RuleFormat, RuleSource, ToolAdapter, ToolCapability, ToolCapabilityAccess,
    ToolCapabilityActionEvidence, ToolCapabilityFormat, ToolCapabilityKind, ToolCapabilityScope,
    ToolCapabilitySourceConfidence, ToolCapabilitySourceKind, ToolCapabilitySupportingSource,
    ToolSourceDiagnosticState,
};
use crate::platform::tool_catalog::{capabilities as catalog_capabilities, registry};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RuleFileDeclaration {
    pub label: &'static str,
    pub relative_path: &'static str,
    pub format: RuleFormat,
}

#[derive(Debug, Clone)]
pub struct DeclaredCapability {
    pub id: &'static str,
    pub kind: ToolCapabilityKind,
    pub scope: ToolCapabilityScope,
    pub access: ToolCapabilityAccess,
    pub format: ToolCapabilityFormat,
    pub source_path: String,
    pub label: &'static str,
    pub diagnostics: Vec<ToolSourceDiagnosticState>,
    pub source_confidence: ToolCapabilitySourceConfidence,
    pub notes: &'static str,
    pub source_kind: ToolCapabilitySourceKind,
    pub primary_config_dir: Option<String>,
    pub supporting_sources: Vec<ToolCapabilitySupportingSource>,
    pub action_evidence: Vec<ToolCapabilityActionEvidence>,
}

pub struct DeclaredToolAdapter {
    id: &'static str,
    name: &'static str,
    icon: &'static str,
    config_dir: PathBuf,
    primary_config_dir: Option<PathBuf>,
    detection_paths: Vec<PathBuf>,
    detection_commands: Vec<&'static str>,
    rules: Vec<RuleFileDeclaration>,
    skills_dir: Option<PathBuf>,
    supports_generic_skills: bool,
    allow_external_generic_symlink: bool,
    capabilities: Vec<DeclaredCapability>,
}

impl DeclaredToolAdapter {
    pub fn new(
        id: &'static str,
        name: &'static str,
        icon: &'static str,
        config_dir: PathBuf,
        capabilities: Vec<DeclaredCapability>,
    ) -> Self {
        Self {
            id,
            name,
            icon,
            config_dir,
            primary_config_dir: None,
            detection_paths: vec![],
            detection_commands: vec![],
            rules: vec![],
            skills_dir: None,
            supports_generic_skills: false,
            allow_external_generic_symlink: false,
            capabilities,
        }
    }

    pub fn with_skills_dir(mut self, skills_dir: PathBuf) -> Self {
        self.skills_dir = Some(skills_dir);
        self
    }

    pub fn with_generic_skills(mut self, allow_external_generic_symlink: bool) -> Self {
        self.supports_generic_skills = true;
        self.allow_external_generic_symlink = allow_external_generic_symlink;
        self
    }

    pub fn with_external_generic_symlink(mut self) -> Self {
        self.allow_external_generic_symlink = true;
        self
    }

    pub fn with_detection_commands(mut self, commands: &[&'static str]) -> Self {
        self.detection_commands = commands.to_vec();
        self
    }

    pub fn with_primary_config_dir(mut self, config_dir: PathBuf) -> Self {
        self.primary_config_dir = Some(config_dir);
        self
    }

    pub fn with_detection_paths(mut self, paths: &[PathBuf]) -> Self {
        self.detection_paths = paths.to_vec();
        self
    }
}

pub fn adapter_from_catalog(id: &'static str, home: &Path) -> DeclaredToolAdapter {
    let definition = registry::definition(id).expect("missing catalog definition");
    let mut adapter = DeclaredToolAdapter::new(
        definition.id,
        definition.name,
        definition.icon,
        definition.config_dir.resolve_path(home),
        catalog_capabilities::declared_capabilities_for_tool(definition, home),
    );
    if let Some(skills_dir) = definition.skills_dir {
        adapter = adapter.with_skills_dir(skills_dir.resolve_path(home));
    }
    if definition.supports_generic_skills {
        adapter = adapter.with_generic_skills(definition.allow_external_generic_symlink);
    } else if definition.allow_external_generic_symlink {
        adapter = adapter.with_external_generic_symlink();
    }
    adapter
}

impl ToolAdapter for DeclaredToolAdapter {
    fn id(&self) -> &str {
        self.id
    }

    fn name(&self) -> &str {
        self.name
    }

    fn icon(&self) -> &str {
        self.icon
    }

    fn config_dir(&self) -> PathBuf {
        self.config_dir.clone()
    }

    fn primary_config_dir(&self) -> PathBuf {
        self.primary_config_dir
            .clone()
            .or_else(|| {
                self.capabilities.iter().find_map(|capability| {
                    capability.primary_config_dir.as_ref().map(PathBuf::from)
                })
            })
            .unwrap_or_else(|| self.config_dir())
    }

    fn detect(&self) -> bool {
        let has_explicit_probe =
            !self.detection_paths.is_empty() || !self.detection_commands.is_empty();
        let explicit_probe_detected = self.detection_paths.iter().any(|path| path.exists())
            || self
                .detection_commands
                .iter()
                .any(|command| command_exists(command));
        if has_explicit_probe {
            explicit_probe_detected
        } else {
            self.config_dir.exists()
        }
    }

    fn read_rules(&self) -> Result<Vec<RuleSource>, String> {
        let mut sources = vec![];
        let mut seen = HashSet::new();
        for declaration in &self.rules {
            let path = self.config_dir.join(declaration.relative_path);
            if !path.exists() {
                continue;
            }
            let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            sources.push(RuleSource {
                path: path.to_string_lossy().to_string(),
                format: declaration.format.clone(),
                content,
                last_modified: get_file_modified(&path),
                label: declaration.label.to_string(),
                group: String::new(),
                diagnostic: None,
            });
            seen.insert(path);
        }

        for source in discover_rule_sources_for_adapter(self.id(), &self.capabilities())? {
            let path = PathBuf::from(&source.path);
            if seen.insert(path) {
                sources.push(source);
            }
        }
        Ok(sources)
    }

    fn write_rule(&self, path: &str, content: &str) -> Result<(), String> {
        let target = PathBuf::from(shellexpand::tilde(path).to_string());
        let can_write = adapter_can_write_path(self, ToolCapabilityKind::Rule, &target);
        if !can_write {
            return Err("Rule writing is not enabled for this declared tool target".to_string());
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(target, content).map_err(|e| e.to_string())
    }

    fn skills_dir(&self) -> Option<PathBuf> {
        self.skills_dir.clone()
    }

    fn supports_generic_skills(&self) -> bool {
        self.supports_generic_skills
    }

    fn allow_external_generic_symlink(&self) -> bool {
        self.allow_external_generic_symlink
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        self.capabilities
            .iter()
            .map(|capability| ToolCapability {
                id: capability.id.to_string(),
                kind: capability.kind.clone(),
                scope: capability.scope.clone(),
                access: capability.access.clone(),
                format: capability.format.clone(),
                source_path: capability.source_path.clone(),
                label: capability.label.to_string(),
                diagnostics: capability.diagnostics.clone(),
                source_confidence: capability.source_confidence.clone(),
                notes: capability.notes.to_string(),
                source_kind: capability.source_kind.clone(),
                primary_config_dir: capability.primary_config_dir.clone(),
                supporting_sources: capability.supporting_sources.clone(),
                action_evidence: capability.action_evidence.clone(),
            })
            .collect()
    }
}

pub fn file_diagnostics() -> Vec<ToolSourceDiagnosticState> {
    vec![
        ToolSourceDiagnosticState::Missing,
        ToolSourceDiagnosticState::Empty,
        ToolSourceDiagnosticState::Loaded,
        ToolSourceDiagnosticState::Unreadable,
        ToolSourceDiagnosticState::Malformed,
    ]
}

pub fn directory_diagnostics() -> Vec<ToolSourceDiagnosticState> {
    vec![
        ToolSourceDiagnosticState::Missing,
        ToolSourceDiagnosticState::Loaded,
        ToolSourceDiagnosticState::Unreadable,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mcp_capability(path: String, format: ToolCapabilityFormat) -> DeclaredCapability {
        mcp_capability_with_scope(path, format, ToolCapabilityScope::Global)
    }

    fn mcp_capability_with_scope(
        path: String,
        format: ToolCapabilityFormat,
        scope: ToolCapabilityScope,
    ) -> DeclaredCapability {
        mcp_capability_with_scope_and_access(path, format, scope, ToolCapabilityAccess::ReadOnly)
    }

    fn mcp_capability_with_scope_and_access(
        path: String,
        format: ToolCapabilityFormat,
        scope: ToolCapabilityScope,
        access: ToolCapabilityAccess,
    ) -> DeclaredCapability {
        DeclaredCapability {
            id: "mcp",
            kind: ToolCapabilityKind::Mcp,
            scope,
            access,
            format,
            source_path: path,
            label: "MCP",
            diagnostics: file_diagnostics(),
            source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
            notes: "",
            source_kind: ToolCapabilitySourceKind::FeatureSource,
            primary_config_dir: None,
            supporting_sources: vec![],
            action_evidence: vec![],
        }
    }

    #[test]
    fn declared_mcp_reader_reports_malformed_json_as_error() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("mcp.json");
        fs::write(&path, "{").unwrap();
        let adapter = DeclaredToolAdapter::new(
            "declared",
            "Declared",
            "D",
            tmp.path().to_path_buf(),
            vec![mcp_capability(
                path.to_string_lossy().to_string(),
                ToolCapabilityFormat::Json,
            )],
        );

        assert!(adapter.read_mcp_servers().is_err());
    }

    #[test]
    fn declared_mcp_reader_loads_servers_without_env_values() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("mcp.json");
        fs::write(
            &path,
            r#"{
              "mcpServers": {
                "demo": {
                  "command": "node",
                  "disabled": true,
                  "env": { "API_KEY": "secret-value" }
                }
              }
            }"#,
        )
        .unwrap();
        let adapter = DeclaredToolAdapter::new(
            "declared",
            "Declared",
            "D",
            tmp.path().to_path_buf(),
            vec![mcp_capability(
                path.to_string_lossy().to_string(),
                ToolCapabilityFormat::Json,
            )],
        );

        let servers = adapter.read_mcp_servers().unwrap();

        assert_eq!(servers.len(), 1);
        assert!(!servers[0].enabled);
        assert_eq!(
            servers[0].activation_state,
            crate::platform::tool_capabilities::skills::McpServerActivationState::Disabled
        );
        assert_eq!(servers[0].env_keys, vec!["API_KEY".to_string()]);
        assert!(!format!("{servers:?}").contains("secret-value"));
    }

    #[test]
    fn declared_mcp_reader_loads_writable_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("writable-mcp.json");
        fs::write(
            &path,
            r#"{"mcpServers":{"writable-demo":{"command":"node"}}}"#,
        )
        .unwrap();
        let adapter = DeclaredToolAdapter::new(
            "declared",
            "Declared",
            "D",
            tmp.path().to_path_buf(),
            vec![mcp_capability_with_scope_and_access(
                path.to_string_lossy().to_string(),
                ToolCapabilityFormat::Json,
                ToolCapabilityScope::Global,
                ToolCapabilityAccess::Writable,
            )],
        );

        let servers = adapter.read_mcp_servers().unwrap();

        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "writable-demo");
    }

    #[test]
    fn declared_mcp_reader_loads_top_level_mcp_server_map_without_env_values() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("opencode.json");
        fs::write(
            &path,
            r#"{
              "mcp": {
                "opencode-demo": {
                  "command": "node",
                  "env": { "API_KEY": "secret-value" }
                }
              }
            }"#,
        )
        .unwrap();
        let adapter = DeclaredToolAdapter::new(
            "declared",
            "Declared",
            "D",
            tmp.path().to_path_buf(),
            vec![mcp_capability_with_scope_and_access(
                format!("{}#mcp", path.to_string_lossy()),
                ToolCapabilityFormat::Json,
                ToolCapabilityScope::Global,
                ToolCapabilityAccess::Writable,
            )],
        );

        let servers = adapter.read_mcp_servers().unwrap();

        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "opencode-demo");
        assert_eq!(servers[0].env_keys, vec!["API_KEY".to_string()]);
        assert!(!format!("{servers:?}").contains("secret-value"));
    }

    #[test]
    fn declared_mcp_reader_ignores_project_scope_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let global_path = tmp.path().join("global-mcp.json");
        let project_path = tmp.path().join("project-mcp.json");
        fs::write(&global_path, r#"{"mcpServers":{}}"#).unwrap();
        fs::write(
            &project_path,
            r#"{"mcpServers":{"project-only":{"command":"node"}}}"#,
        )
        .unwrap();
        let adapter = DeclaredToolAdapter::new(
            "declared",
            "Declared",
            "D",
            tmp.path().to_path_buf(),
            vec![
                mcp_capability(
                    global_path.to_string_lossy().to_string(),
                    ToolCapabilityFormat::Json,
                ),
                mcp_capability_with_scope(
                    project_path.to_string_lossy().to_string(),
                    ToolCapabilityFormat::Json,
                    ToolCapabilityScope::Project,
                ),
            ],
        );

        let servers = adapter.read_mcp_servers().unwrap();

        assert!(servers.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn command_detection_requires_executable_file() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let command_path = tmp.path().join("opencode");
        fs::write(&command_path, "").unwrap();

        let mut permissions = fs::metadata(&command_path).unwrap().permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&command_path, permissions).unwrap();
        assert!(!crate::platform::tool_adapters::is_executable_file(
            &command_path
        ));

        let mut permissions = fs::metadata(&command_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&command_path, permissions).unwrap();
        assert!(crate::platform::tool_adapters::is_executable_file(
            &command_path
        ));
    }

    #[test]
    fn declared_detection_accepts_explicit_identity_path() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join("missing-config");
        let app_dir = tmp.path().join("Trae Solo.app");
        fs::create_dir_all(&app_dir).unwrap();
        let adapter = DeclaredToolAdapter::new("declared", "Declared", "D", config_dir, vec![])
            .with_detection_paths(&[app_dir]);

        assert!(adapter.detect());
    }

    #[test]
    fn declared_detection_ignores_config_dir_when_explicit_probe_is_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join("leftover-config");
        let app_dir = tmp.path().join("Missing.app");
        fs::create_dir_all(&config_dir).unwrap();
        let adapter = DeclaredToolAdapter::new("declared", "Declared", "D", config_dir, vec![])
            .with_detection_paths(&[app_dir]);

        assert!(!adapter.detect());
    }
}
