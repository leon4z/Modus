// Purpose: Tool adapter registry root and compatibility export surface.

#[path = "adapters/claude_code.rs"]
pub mod claude_code;
#[path = "adapters/codex.rs"]
pub mod codex;
#[path = "adapters/openclaw.rs"]
pub mod openclaw;

#[path = "adapters/codebuddy.rs"]
pub mod codebuddy;
#[path = "adapters/continue.rs"]
pub mod continue_adapter;
#[path = "adapters/cursor.rs"]
pub mod cursor;
#[path = "adapters/github_copilot.rs"]
pub mod github_copilot;
#[path = "adapters/goose.rs"]
pub mod goose;
#[path = "adapters/hermes_agent.rs"]
pub mod hermes_agent;
#[path = "adapters/kiro.rs"]
pub mod kiro;
#[path = "adapters/opencode.rs"]
pub mod opencode;
#[path = "adapters/openhands.rs"]
pub mod openhands;
#[path = "adapters/pi_agent.rs"]
pub mod pi_agent;
#[path = "adapters/qoder.rs"]
pub mod qoder;
#[path = "adapters/trae.rs"]
pub mod trae;
#[path = "adapters/trae_cn.rs"]
pub mod trae_cn;
#[path = "adapters/trae_solo.rs"]
pub mod trae_solo;
#[path = "adapters/trae_solo_cn.rs"]
pub mod trae_solo_cn;
#[path = "adapters/windsurf.rs"]
pub mod windsurf;
#[path = "adapters/workbuddy.rs"]
pub mod workbuddy;

// Compatibility exports keep existing domain imports stable during the shared
// support migration. Remove them once consumers import the platform/scenario
// owners directly.
#[allow(unused_imports)]
pub use crate::platform::tool_adapters::{
    adapter_can_manage_global_rule_target, adapter_can_write_path,
    adapter_has_default_global_rule_file_target, declared as declared_tool, ToolAdapter,
    ToolPresence,
};
pub(crate) use crate::platform::tool_capabilities::capability_matches_eligible_global_rule_target;
#[allow(unused_imports)]
pub use crate::platform::tool_capabilities::rule_sources::{
    can_write_rule_path_for_adapter, discover_rule_sources_for_adapter,
    rule_capabilities_matching_path_for_adapter,
};
#[allow(unused_imports)]
pub use crate::platform::tool_capabilities::{
    capability_declared_source_path, capability_existing_source_file,
    capability_is_eligible_global_rule_target, capability_projections, directory_capability,
    effective_capabilities, get_file_modified, rule_sources, skills, tool_capability, RuleFormat,
    RuleSource, ToolCapability, ToolCapabilityAccess, ToolCapabilityAction,
    ToolCapabilityActionEvidence, ToolCapabilityFormat, ToolCapabilityKind, ToolCapabilityScope,
    ToolCapabilitySourceConfidence, ToolCapabilitySourceKind, ToolCapabilitySupportingSource,
    ToolCapabilitySupportingSourceRole, ToolSourceDiagnosticState,
};
#[cfg(test)]
pub use crate::scenario::dev_tool;

use crate::platform::tool_capabilities::capability_source_is_directory;
use crate::platform::tool_catalog::registry as tool_catalog_registry;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimaryConfigHealth {
    Unknown,
    Ok,
    Missing,
    Unreadable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedTool {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub config_dir: String,
    #[serde(default)]
    pub default_config_dir: String,
    #[serde(default)]
    pub primary_config_dir: String,
    #[serde(default)]
    pub default_primary_config_dir: String,
    #[serde(default = "default_primary_config_health")]
    pub primary_config_health: PrimaryConfigHealth,
    pub skills_dir: String,
    #[serde(default)]
    pub default_skills_dir: String,
    #[serde(default = "default_primary_config_health")]
    pub primary_skills_health: PrimaryConfigHealth,
    #[serde(default = "default_primary_config_health")]
    pub rule_directory_health: PrimaryConfigHealth,
    #[serde(default = "default_primary_config_health")]
    pub global_rule_file_health: PrimaryConfigHealth,
    #[serde(default = "default_primary_config_health")]
    pub mcp_config_health: PrimaryConfigHealth,
    #[serde(default = "default_primary_config_health")]
    pub tool_config_health: PrimaryConfigHealth,
    pub detected: bool,
    #[serde(default)]
    pub presence_app_detected: bool,
    #[serde(default)]
    pub presence_cli_detected: bool,
    #[serde(default)]
    pub presence_label: String,
    pub rule_sources: Vec<RuleSource>,
    pub supports_generic_skills: bool,
    #[serde(default)]
    pub certified_global_rule_target: String,
    #[serde(default)]
    pub certified_shared_skill_direct_read: bool,
    pub read_paths: Vec<String>,
    pub managed_anchor: String,
    pub allow_external_generic_symlink: bool,
    pub capabilities: Vec<ToolCapability>,
}

fn default_primary_config_health() -> PrimaryConfigHealth {
    PrimaryConfigHealth::Unknown
}

pub struct ToolRegistry {
    adapters: Vec<Box<dyn ToolAdapter>>,
}

/// Lightweight (tool_id, skills_dir) pair used by the scenario-runner
/// to enumerate dev tool skill directories without holding a registry borrow.
#[derive(Debug, Clone)]
pub struct ToolSkillsLocation {
    pub tool_id: String,
    pub skills_dir: PathBuf,
}

fn capability_path_string(capability: &ToolCapability) -> Option<String> {
    capability_declared_source_path(capability).map(|path| path.to_string_lossy().to_string())
}

fn skill_capability_projections(
    adapter_id: &str,
    capabilities: &[ToolCapability],
) -> Vec<capability_projections::ToolCapabilityProjection> {
    capability_projections::project_capabilities(
        adapter_id,
        capability_projections::ToolCapabilityModule::Skills,
        capabilities,
    )
}

fn certified_global_rule_target_path(adapter_id: &str, capabilities: &[ToolCapability]) -> String {
    capability_projections::project_capabilities(
        adapter_id,
        capability_projections::ToolCapabilityModule::Rules,
        capabilities,
    )
    .into_iter()
    .find(|projection| {
        if projection.source_role
            != capability_projections::ToolCapabilitySourceRole::RuleGlobalTarget
            || !projection.allows(&capability_projections::ToolCapabilityAction::Inject)
        {
            return false;
        }
        let Some(source) = capability_declared_source_path(&projection.evidence) else {
            return false;
        };
        !capability_source_is_directory(&projection.evidence, &source)
    })
    .and_then(|projection| capability_path_string(&projection.evidence))
    .unwrap_or_default()
}

fn has_user_configured_rule_source(adapter_id: &str, capabilities: &[ToolCapability]) -> bool {
    capability_projections::project_capabilities(
        adapter_id,
        capability_projections::ToolCapabilityModule::Rules,
        capabilities,
    )
    .into_iter()
    .any(|projection| {
        projection.source_role
            == capability_projections::ToolCapabilitySourceRole::RuleNativeFileSource
            && projection.exclusion_reason.is_none()
            && projection.evidence.source_confidence
                == ToolCapabilitySourceConfidence::UserConfigured
    })
}

fn detected_rule_sources(
    adapter: &dyn ToolAdapter,
    base_capabilities: &[ToolCapability],
    effective_capabilities: &[ToolCapability],
) -> Vec<RuleSource> {
    if has_user_configured_rule_source(adapter.id(), effective_capabilities) {
        return rule_sources::discover_rule_sources_for_adapter(
            adapter.id(),
            effective_capabilities,
        )
        .unwrap_or_default();
    }
    adapter.read_rules().unwrap_or_else(|_| {
        rule_sources::discover_rule_sources_for_adapter(adapter.id(), base_capabilities)
            .unwrap_or_default()
    })
}

fn first_skill_projection_path(
    projections: &[capability_projections::ToolCapabilityProjection],
    source_role: capability_projections::ToolCapabilitySourceRole,
    action: capability_projections::ToolCapabilityAction,
) -> Option<String> {
    projections
        .iter()
        .find(|projection| projection.source_role == source_role && projection.allows(&action))
        .and_then(|projection| capability_path_string(&projection.evidence))
}

fn compatibility_skill_read_paths(
    projections: &[capability_projections::ToolCapabilityProjection],
) -> Vec<String> {
    let mut paths = vec![];
    for role in [
        capability_projections::ToolCapabilitySourceRole::SkillToolDirectory,
        capability_projections::ToolCapabilitySourceRole::SkillCompoundSource,
        capability_projections::ToolCapabilitySourceRole::SkillSharedSource,
    ] {
        if let Some(path) = first_skill_projection_path(
            projections,
            role,
            capability_projections::ToolCapabilityAction::View,
        ) {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    paths
}

fn declared_market_adapters(home: &std::path::Path) -> Vec<Box<dyn ToolAdapter>> {
    vec![
        cursor::create(home),
        qoder::create(home),
        opencode::create(home),
        codebuddy::create(home),
        workbuddy::create(home),
        hermes_agent::create(home),
        kiro::create(home),
        pi_agent::create(home),
        github_copilot::create(home),
        windsurf::create(home),
        trae::create(home),
        trae_cn::create(home),
        trae_solo::create(home),
        trae_solo_cn::create(home),
    ]
}

fn primary_config_health_for_path(path: &Path, detected: bool) -> PrimaryConfigHealth {
    if !detected {
        return PrimaryConfigHealth::Unknown;
    }
    if path.as_os_str().is_empty() {
        return PrimaryConfigHealth::Unknown;
    }
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => match std::fs::read_dir(&path) {
            Ok(_) => PrimaryConfigHealth::Ok,
            Err(_) => PrimaryConfigHealth::Unreadable,
        },
        Ok(_) => PrimaryConfigHealth::Unreadable,
        Err(error) if error.kind() == ErrorKind::NotFound => PrimaryConfigHealth::Missing,
        Err(_) => PrimaryConfigHealth::Unreadable,
    }
}

#[derive(Clone, Copy)]
enum SourcePathKind {
    Directory,
    File,
}

fn source_path_health_for_path(
    path: &Path,
    detected: bool,
    kind: SourcePathKind,
) -> PrimaryConfigHealth {
    if !detected || path.as_os_str().is_empty() {
        return PrimaryConfigHealth::Unknown;
    }
    match std::fs::metadata(path) {
        Ok(metadata) => match kind {
            SourcePathKind::Directory if metadata.is_dir() => match std::fs::read_dir(path) {
                Ok(_) => PrimaryConfigHealth::Ok,
                Err(_) => PrimaryConfigHealth::Unreadable,
            },
            SourcePathKind::File if metadata.is_file() => match std::fs::File::open(path) {
                Ok(_) => PrimaryConfigHealth::Ok,
                Err(_) => PrimaryConfigHealth::Unreadable,
            },
            _ => PrimaryConfigHealth::Unreadable,
        },
        Err(error) if error.kind() == ErrorKind::NotFound => PrimaryConfigHealth::Missing,
        Err(_) => PrimaryConfigHealth::Unreadable,
    }
}

fn first_projection_source_health(
    adapter_id: &str,
    capabilities: &[ToolCapability],
    module: capability_projections::ToolCapabilityModule,
    roles: &[capability_projections::ToolCapabilitySourceRole],
    kind: SourcePathKind,
    detected: bool,
) -> PrimaryConfigHealth {
    let projection = capability_projections::project_capabilities(adapter_id, module, capabilities)
        .into_iter()
        .find(|projection| {
            if !roles.contains(&projection.source_role) || projection.exclusion_reason.is_some() {
                return false;
            }
            let Some(source) = capability_declared_source_path(&projection.evidence) else {
                return false;
            };
            match kind {
                SourcePathKind::Directory => {
                    capability_source_is_directory(&projection.evidence, &source)
                }
                SourcePathKind::File => {
                    !capability_source_is_directory(&projection.evidence, &source)
                }
            }
        });
    projection
        .and_then(|projection| capability_declared_source_path(&projection.evidence))
        .map(|path| source_path_health_for_path(&path, detected, kind))
        .unwrap_or(PrimaryConfigHealth::Unknown)
}

fn tool_capability_overrides_for_adapter<'a>(
    config: &'a crate::platform::config::AppConfig,
    adapter_id: &str,
) -> Option<&'a crate::platform::config::ToolCapabilityOverrides> {
    let canonical_tool_id =
        crate::platform::tool_catalog::normalization::canonical_tool_id(adapter_id);
    config
        .tool_capability_overrides
        .get(&canonical_tool_id)
        .or_else(|| config.tool_capability_overrides.get(adapter_id))
}

fn configured_source_path_health(
    source_path: Option<String>,
    detected: bool,
    kind: SourcePathKind,
) -> Option<PrimaryConfigHealth> {
    source_path.map(|path| source_path_health_for_path(&PathBuf::from(path), detected, kind))
}

fn configured_rule_directory_health(
    overrides: Option<&crate::platform::config::ToolCapabilityOverrides>,
    detected: bool,
) -> Option<PrimaryConfigHealth> {
    let overrides = overrides?;
    if overrides
        .normalized_rule_source_type()
        .unwrap_or("directory")
        != "directory"
    {
        return None;
    }
    configured_source_path_health(
        overrides.normalized_rule_source_path(),
        detected,
        SourcePathKind::Directory,
    )
}

fn trimmed_custom_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains('*') {
        return None;
    }
    Some(trimmed.to_string())
}

fn custom_tool_is_configured(tool: &crate::platform::config::CustomTool) -> bool {
    [
        tool.rule_directory.as_str(),
        tool.global_rule_file.as_str(),
        tool.rule_file.as_str(),
        tool.skills_dir.as_str(),
        tool.mcp_config.as_str(),
        tool.tool_config.as_str(),
    ]
    .iter()
    .any(|value| trimmed_custom_value(value).is_some())
        || tool.shared_skill_direct_read
}

fn custom_tool_scoped_config(
    config: &crate::platform::config::AppConfig,
    canonical_tool_id: &str,
    tool: &crate::platform::config::CustomTool,
) -> crate::platform::config::AppConfig {
    let mut scoped = config.clone();
    scoped.tool_paths.insert(
        canonical_tool_id.to_string(),
        crate::platform::config::ToolPaths {
            config_dir: tool.config_dir.clone(),
            skills_dir: tool.skills_dir.clone(),
        },
    );
    let global_rule_file = trimmed_custom_value(&tool.global_rule_file)
        .or_else(|| trimmed_custom_value(&tool.rule_file));
    scoped.tool_capability_overrides.insert(
        canonical_tool_id.to_string(),
        crate::platform::config::ToolCapabilityOverrides {
            custom_rule_source_type: trimmed_custom_value(&tool.rule_directory)
                .map(|_| "directory".to_string()),
            custom_rule_source_path: trimmed_custom_value(&tool.rule_directory),
            custom_global_rule_target: global_rule_file,
            custom_mcp_config_path: trimmed_custom_value(&tool.mcp_config),
            custom_tool_config_path: trimmed_custom_value(&tool.tool_config),
            shared_skill_direct_read: tool.shared_skill_direct_read.then_some(true),
        },
    );
    scoped
}

fn detected_custom_tool(
    config: &crate::platform::config::AppConfig,
    tool: &crate::platform::config::CustomTool,
    builtin_ids: &HashSet<String>,
) -> Option<DetectedTool> {
    let canonical_tool_id =
        crate::platform::tool_catalog::normalization::canonical_tool_id(&tool.id);
    if canonical_tool_id.trim().is_empty()
        || builtin_ids.contains(&canonical_tool_id)
        || !custom_tool_is_configured(tool)
    {
        return None;
    }

    let scoped_config = custom_tool_scoped_config(config, &canonical_tool_id, tool);
    let detected = true;
    let capabilities = effective_capabilities::resolve_effective_capabilities(
        &canonical_tool_id,
        &[],
        &scoped_config,
    );
    let rule_sources =
        rule_sources::discover_rule_sources_for_adapter(&canonical_tool_id, &capabilities)
            .unwrap_or_default();
    let skill_projections = skill_capability_projections(&canonical_tool_id, &capabilities);
    let skills_dir = first_skill_projection_path(
        &skill_projections,
        capability_projections::ToolCapabilitySourceRole::SkillToolDirectory,
        capability_projections::ToolCapabilityAction::View,
    )
    .or_else(|| {
        first_skill_projection_path(
            &skill_projections,
            capability_projections::ToolCapabilitySourceRole::SkillCompoundSource,
            capability_projections::ToolCapabilityAction::View,
        )
    })
    .unwrap_or_default();
    let read_paths = compatibility_skill_read_paths(&skill_projections);
    let managed_anchor = first_skill_projection_path(
        &skill_projections,
        capability_projections::ToolCapabilitySourceRole::SkillToolDirectory,
        capability_projections::ToolCapabilityAction::Install,
    )
    .or_else(|| {
        first_skill_projection_path(
            &skill_projections,
            capability_projections::ToolCapabilitySourceRole::SkillCompoundSource,
            capability_projections::ToolCapabilityAction::Install,
        )
    })
    .unwrap_or_default();
    let supports_generic_skills = skill_projections.iter().any(|projection| {
        projection.source_role
            == capability_projections::ToolCapabilitySourceRole::SkillSharedSource
            && projection.allows(&capability_projections::ToolCapabilityAction::View)
    });
    let config_dir = tool.config_dir.trim().to_string();
    let capability_overrides = scoped_config
        .tool_capability_overrides
        .get(&canonical_tool_id);

    Some(DetectedTool {
        id: canonical_tool_id,
        name: if tool.name.trim().is_empty() {
            tool.id.clone()
        } else {
            tool.name.trim().to_string()
        },
        icon: if tool.icon.trim().is_empty() {
            "wrench".to_string()
        } else {
            tool.icon.trim().to_string()
        },
        default_config_dir: config_dir.clone(),
        primary_config_health: primary_config_health_for_path(
            &PathBuf::from(&config_dir),
            detected,
        ),
        config_dir: config_dir.clone(),
        default_primary_config_dir: config_dir.clone(),
        primary_config_dir: config_dir,
        primary_skills_health: primary_config_health_for_path(
            &PathBuf::from(&skills_dir),
            detected,
        ),
        rule_directory_health: configured_rule_directory_health(capability_overrides, detected)
            .unwrap_or(PrimaryConfigHealth::Unknown),
        global_rule_file_health: configured_source_path_health(
            capability_overrides.and_then(|overrides| overrides.normalized_global_rule_target()),
            detected,
            SourcePathKind::File,
        )
        .unwrap_or(PrimaryConfigHealth::Unknown),
        mcp_config_health: configured_source_path_health(
            capability_overrides.and_then(|overrides| overrides.normalized_mcp_config_path()),
            detected,
            SourcePathKind::File,
        )
        .unwrap_or(PrimaryConfigHealth::Unknown),
        tool_config_health: configured_source_path_health(
            capability_overrides.and_then(|overrides| overrides.normalized_tool_config_path()),
            detected,
            SourcePathKind::File,
        )
        .unwrap_or(PrimaryConfigHealth::Unknown),
        skills_dir,
        default_skills_dir: String::new(),
        detected,
        presence_app_detected: false,
        presence_cli_detected: false,
        presence_label: String::new(),
        rule_sources,
        supports_generic_skills,
        certified_global_rule_target: String::new(),
        certified_shared_skill_direct_read: false,
        read_paths,
        managed_anchor,
        allow_external_generic_symlink: true,
        capabilities,
    })
}

fn builtin_tool_adapters(home: &std::path::Path) -> Vec<Box<dyn ToolAdapter>> {
    let mut adapters: Vec<Box<dyn ToolAdapter>> = vec![
        Box::new(claude_code::ClaudeCodeAdapter::new(home.to_path_buf())),
        Box::new(codex::CodexAdapter::new(home.to_path_buf())),
        Box::new(openclaw::OpenClawAdapter::new(home.to_path_buf())),
    ];
    adapters.extend(declared_market_adapters(home));
    adapters
}

fn dev_tool_adapters(home: &std::path::Path) -> Vec<Box<dyn ToolAdapter>> {
    vec![
        Box::new(crate::scenario::dev_tool::DevToolAdapter::new(
            "dev-tool1",
            "Dev Tool 1",
            "🔧",
            home.join(".dev-tool1"),
            true,
        )),
        Box::new(crate::scenario::dev_tool::DevToolAdapter::new(
            "dev-tool2",
            "Dev Tool 2",
            "🔩",
            home.join(".dev-tool2"),
            false,
        )),
        Box::new(crate::scenario::dev_tool::DevToolAdapter::new(
            "dev-tool3",
            "Dev Tool 3",
            "🧪",
            home.join(".dev-tool3"),
            false,
        )),
    ]
}

fn tool_adapters_for_runtime(
    home: &std::path::Path,
    runtime_shape: crate::platform::env::RuntimeShape,
) -> Vec<Box<dyn ToolAdapter>> {
    if runtime_shape.uses_sandbox_tools() {
        dev_tool_adapters(home)
    } else {
        builtin_tool_adapters(home)
    }
}

impl ToolRegistry {
    #[cfg(test)]
    pub(crate) fn from_adapters_for_tests(adapters: Vec<Box<dyn ToolAdapter>>) -> Self {
        Self { adapters }
    }

    pub fn new() -> Self {
        let home = crate::platform::env::home_dir();
        let adapters = tool_adapters_for_runtime(&home, crate::platform::env::runtime_shape());
        Self { adapters }
    }

    pub fn detect_all(&self) -> Vec<DetectedTool> {
        let config = crate::platform::config::load_config();
        self.detect_all_for_config(&config)
    }

    pub(crate) fn detect_all_for_config(
        &self,
        config: &crate::platform::config::AppConfig,
    ) -> Vec<DetectedTool> {
        let mut tools: Vec<DetectedTool> = self
            .adapters
            .iter()
            .map(|adapter| {
                let presence = adapter.presence();
                let detected = presence.detected;
                let base_capabilities = adapter.capabilities();
                let base_skill_projections =
                    skill_capability_projections(adapter.id(), &base_capabilities);
                let certified_global_rule_target =
                    certified_global_rule_target_path(adapter.id(), &base_capabilities);
                let certified_shared_skill_direct_read =
                    base_skill_projections.iter().any(|projection| {
                        projection.source_role
                            == capability_projections::ToolCapabilitySourceRole::SkillSharedSource
                            && projection
                                .allows(&capability_projections::ToolCapabilityAction::View)
                    });
                let capabilities = effective_capabilities::resolve_effective_capabilities(
                    adapter.id(),
                    &base_capabilities,
                    config,
                );
                let rule_sources = if detected {
                    detected_rule_sources(adapter.as_ref(), &base_capabilities, &capabilities)
                } else {
                    vec![]
                };
                let skill_projections = skill_capability_projections(adapter.id(), &capabilities);
                let skills_dir = first_skill_projection_path(
                    &skill_projections,
                    capability_projections::ToolCapabilitySourceRole::SkillToolDirectory,
                    capability_projections::ToolCapabilityAction::View,
                )
                .or_else(|| {
                    first_skill_projection_path(
                        &skill_projections,
                        capability_projections::ToolCapabilitySourceRole::SkillCompoundSource,
                        capability_projections::ToolCapabilityAction::View,
                    )
                })
                .unwrap_or_default();
                let default_skills_dir = first_skill_projection_path(
                    &base_skill_projections,
                    capability_projections::ToolCapabilitySourceRole::SkillToolDirectory,
                    capability_projections::ToolCapabilityAction::View,
                )
                .or_else(|| {
                    first_skill_projection_path(
                        &base_skill_projections,
                        capability_projections::ToolCapabilitySourceRole::SkillCompoundSource,
                        capability_projections::ToolCapabilityAction::View,
                    )
                })
                .unwrap_or_default();
                let read_paths = compatibility_skill_read_paths(&skill_projections);
                let managed_anchor = first_skill_projection_path(
                    &skill_projections,
                    capability_projections::ToolCapabilitySourceRole::SkillToolDirectory,
                    capability_projections::ToolCapabilityAction::Install,
                )
                .or_else(|| {
                    first_skill_projection_path(
                        &skill_projections,
                        capability_projections::ToolCapabilitySourceRole::SkillCompoundSource,
                        capability_projections::ToolCapabilityAction::Install,
                    )
                })
                .unwrap_or_default();
                let supports_generic_skills = skill_projections.iter().any(|projection| {
                    projection.source_role
                        == capability_projections::ToolCapabilitySourceRole::SkillSharedSource
                        && projection.allows(&capability_projections::ToolCapabilityAction::View)
                });
                let adapter_config_dir = adapter.config_dir().to_string_lossy().to_string();
                let adapter_primary_config_dir =
                    adapter.primary_config_dir().to_string_lossy().to_string();
                let config_dir = crate::platform::config::resolved_config_dir_with_config(
                    config,
                    adapter.id(),
                    &adapter_config_dir,
                );
                let primary_config_dir = crate::platform::config::resolved_config_dir_with_config(
                    config,
                    adapter.id(),
                    &adapter_primary_config_dir,
                );
                let capability_overrides =
                    tool_capability_overrides_for_adapter(config, adapter.id());
                DetectedTool {
                    id: adapter.id().to_string(),
                    name: adapter.name().to_string(),
                    icon: adapter.icon().to_string(),
                    default_config_dir: adapter_config_dir,
                    primary_config_health: primary_config_health_for_path(
                        &PathBuf::from(&primary_config_dir),
                        detected,
                    ),
                    config_dir,
                    default_primary_config_dir: adapter_primary_config_dir,
                    primary_config_dir,
                    primary_skills_health: primary_config_health_for_path(
                        &PathBuf::from(&skills_dir),
                        detected,
                    ),
                    rule_directory_health: configured_rule_directory_health(
                        capability_overrides,
                        detected,
                    )
                    .unwrap_or_else(|| {
                        first_projection_source_health(
                            adapter.id(),
                            &capabilities,
                            capability_projections::ToolCapabilityModule::Rules,
                            &[
                                capability_projections::ToolCapabilitySourceRole::RuleNativeFileSource,
                            ],
                            SourcePathKind::Directory,
                            detected,
                        )
                    }),
                    global_rule_file_health: configured_source_path_health(
                        capability_overrides
                            .and_then(|overrides| overrides.normalized_global_rule_target()),
                        detected,
                        SourcePathKind::File,
                    )
                    .unwrap_or_else(|| {
                        first_projection_source_health(
                            adapter.id(),
                            &capabilities,
                            capability_projections::ToolCapabilityModule::Rules,
                            &[capability_projections::ToolCapabilitySourceRole::RuleGlobalTarget],
                            SourcePathKind::File,
                            detected,
                        )
                    }),
                    mcp_config_health: configured_source_path_health(
                        capability_overrides
                            .and_then(|overrides| overrides.normalized_mcp_config_path()),
                        detected,
                        SourcePathKind::File,
                    )
                    .unwrap_or_else(|| {
                        first_projection_source_health(
                            adapter.id(),
                            &capabilities,
                            capability_projections::ToolCapabilityModule::Mcp,
                            &[capability_projections::ToolCapabilitySourceRole::McpGlobalConfig],
                            SourcePathKind::File,
                            detected,
                        )
                    }),
                    tool_config_health: configured_source_path_health(
                        capability_overrides
                            .and_then(|overrides| overrides.normalized_tool_config_path()),
                        detected,
                        SourcePathKind::File,
                    )
                    .unwrap_or_else(|| {
                        first_projection_source_health(
                            adapter.id(),
                            &capabilities,
                            capability_projections::ToolCapabilityModule::OrdinaryConfig,
                            &[
                                capability_projections::ToolCapabilitySourceRole::OrdinaryConfigFile,
                            ],
                            SourcePathKind::File,
                            detected,
                        )
                    }),
                    skills_dir,
                    default_skills_dir,
                    detected,
                    presence_app_detected: presence.app_detected,
                    presence_cli_detected: presence.cli_detected,
                    presence_label: presence.label,
                    rule_sources,
                    supports_generic_skills,
                    certified_global_rule_target,
                    certified_shared_skill_direct_read,
                    read_paths,
                    managed_anchor,
                    allow_external_generic_symlink: adapter.allow_external_generic_symlink(),
                    capabilities,
                }
            })
            .collect();
        let builtin_ids: HashSet<String> = tools.iter().map(|tool| tool.id.clone()).collect();
        tools.extend(
            config
                .custom_tools
                .iter()
                .filter_map(|tool| detected_custom_tool(config, tool, &builtin_ids)),
        );
        tools
    }

    pub fn get_adapter(&self, id: &str) -> Option<&dyn ToolAdapter> {
        let canonical = tool_catalog_registry::canonical_id(id);
        self.adapters
            .iter()
            .find(|a| a.id() == canonical)
            .map(|a| a.as_ref())
    }

    pub fn tool_ids(&self) -> Vec<String> {
        self.adapters.iter().map(|a| a.id().to_string()).collect()
    }

    pub fn iter_tool_skills_dirs(&self) -> Vec<ToolSkillsLocation> {
        self.adapters
            .iter()
            .filter_map(|a| {
                a.skills_dir().map(|dir| ToolSkillsLocation {
                    tool_id: a.id().to_string(),
                    skills_dir: dir,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability(id: &str, access: ToolCapabilityAccess) -> ToolCapability {
        ToolCapability {
            id: id.to_string(),
            kind: ToolCapabilityKind::Rule,
            scope: ToolCapabilityScope::Project,
            access,
            format: ToolCapabilityFormat::Markdown,
            source_path: "AGENTS.md".to_string(),
            label: "AGENTS.md".to_string(),
            diagnostics: vec![ToolSourceDiagnosticState::Missing],
            source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
            notes: String::new(),
            source_kind: ToolCapabilitySourceKind::FeatureSource,
            primary_config_dir: None,
            supporting_sources: vec![],
            action_evidence: vec![],
        }
    }

    fn tool_ids_for_runtime(runtime_shape: crate::platform::env::RuntimeShape) -> Vec<String> {
        let registry = ToolRegistry {
            adapters: tool_adapters_for_runtime(Path::new("/tmp/modus-home"), runtime_shape),
        };
        registry.tool_ids()
    }

    fn deferred_research_tool_ids() -> std::collections::BTreeSet<String> {
        ["continue", "goose", "openhands"]
            .into_iter()
            .map(String::from)
            .collect()
    }

    #[test]
    fn development_sandbox_uses_only_dev_tool_adapters() {
        let ids = tool_ids_for_runtime(crate::platform::env::RuntimeShape::DevelopmentSandbox);
        assert_eq!(ids, vec!["dev-tool1", "dev-tool2", "dev-tool3"]);
    }

    #[test]
    fn pre_release_and_release_use_real_tool_adapters() {
        let pre_release = tool_ids_for_runtime(crate::platform::env::RuntimeShape::PreRelease);
        let release = tool_ids_for_runtime(crate::platform::env::RuntimeShape::Release);

        for ids in [&pre_release, &release] {
            assert!(ids.iter().any(|id| id == "codex"));
            assert!(ids.iter().any(|id| id == "openclaw"));
            assert!(!ids.iter().any(|id| id.starts_with("dev-tool")));
        }
        assert_eq!(pre_release, release);
    }

    #[test]
    fn runtime_builtin_adapters_exclude_deferred_research_only_tools() {
        let ids = tool_ids_for_runtime(crate::platform::env::RuntimeShape::Release);

        for tool_id in deferred_research_tool_ids() {
            assert!(
                !ids.contains(&tool_id),
                "{tool_id} is research-only and must not enter current runtime detection"
            );
        }
    }

    #[test]
    fn capability_writable_gate_only_allows_writable() {
        assert!(capability("a", ToolCapabilityAccess::Writable).is_writable());
        assert!(!capability("b", ToolCapabilityAccess::ReadOnly).is_writable());
        assert!(!capability("c", ToolCapabilityAccess::Readable).is_writable());
        assert!(!capability("d", ToolCapabilityAccess::Unknown).is_writable());
        assert!(!capability("e", ToolCapabilityAccess::Unsupported).is_writable());
    }

    #[test]
    fn default_capabilities_do_not_assume_write_for_shared_skills() {
        let adapter = dev_tool::DevToolAdapter::new(
            "dev",
            "Dev",
            "D",
            PathBuf::from("/tmp/modus-missing-dev-tool"),
            true,
        );
        let capabilities = adapter.capabilities();
        let shared = capabilities
            .iter()
            .find(|capability| capability.id == "shared-skills")
            .expect("expected shared skill capability");

        assert_eq!(shared.access, ToolCapabilityAccess::ReadOnly);
        assert!(!shared.is_writable());
    }

    struct DefaultSkillAdapter;

    impl ToolAdapter for DefaultSkillAdapter {
        fn id(&self) -> &str {
            "default-skill"
        }
        fn name(&self) -> &str {
            "Default Skill"
        }
        fn icon(&self) -> &str {
            "D"
        }
        fn config_dir(&self) -> PathBuf {
            PathBuf::from("/tmp/default-skill")
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
        fn skills_dir(&self) -> Option<PathBuf> {
            Some(PathBuf::from("/tmp/default-skill/skills"))
        }
    }

    #[test]
    fn default_dedicated_skill_capability_is_not_writable_without_confidence() {
        let adapter = DefaultSkillAdapter;
        let capabilities = adapter.capabilities();
        let dedicated = capabilities
            .iter()
            .find(|capability| capability.id == "dedicated-skills")
            .expect("expected dedicated skill capability");

        assert_eq!(dedicated.access, ToolCapabilityAccess::ReadOnly);
        assert_eq!(
            dedicated.source_confidence,
            ToolCapabilitySourceConfidence::Unknown
        );
        assert!(!dedicated.is_writable());
    }

    #[test]
    fn custom_tool_records_become_detected_capability_rows() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let skills_dir = tmp.path().join("skills");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::create_dir_all(&skills_dir).unwrap();
        let global_rule = tmp.path().join("RULES.md");
        let mcp_config = tmp.path().join("mcp.json");
        let tool_config = tmp.path().join("settings.json");
        std::fs::write(&global_rule, "# rules").unwrap();
        std::fs::write(&mcp_config, r#"{"mcpServers":{}}"#).unwrap();
        std::fs::write(&tool_config, "{}").unwrap();

        let mut config = crate::platform::config::default_config();
        config
            .custom_tools
            .push(crate::platform::config::CustomTool {
                id: "custom-tool".to_string(),
                name: "Custom Tool".to_string(),
                icon: "wrench".to_string(),
                config_dir: tmp.path().to_string_lossy().to_string(),
                rule_directory: rules_dir.to_string_lossy().to_string(),
                global_rule_file: global_rule.to_string_lossy().to_string(),
                skills_dir: skills_dir.to_string_lossy().to_string(),
                shared_skill_direct_read: true,
                mcp_config: mcp_config.to_string_lossy().to_string(),
                tool_config: tool_config.to_string_lossy().to_string(),
                rule_file: String::new(),
            });
        let registry = ToolRegistry::from_adapters_for_tests(vec![]);
        let tools = registry.detect_all_for_config(&config);
        let tool = tools
            .iter()
            .find(|tool| tool.id == "custom-tool")
            .expect("custom tool row");

        assert!(tool.detected);
        assert_eq!(tool.name, "Custom Tool");
        assert_eq!(tool.skills_dir, skills_dir.to_string_lossy().to_string());
        assert!(tool.supports_generic_skills);
        assert!(!tool.presence_app_detected);
        assert!(!tool.presence_cli_detected);
        assert_eq!(tool.presence_label, "");
        assert!(tool.capabilities.iter().any(|capability| {
            capability.kind == ToolCapabilityKind::Rule
                && capability.source_confidence == ToolCapabilitySourceConfidence::UserConfigured
                && capability.source_path == rules_dir.to_string_lossy().to_string()
        }));
        assert!(capability_projections::project_capabilities(
            &tool.id,
            capability_projections::ToolCapabilityModule::Mcp,
            &tool.capabilities,
        )
        .iter()
        .any(|projection| projection.source_role
            == capability_projections::ToolCapabilitySourceRole::McpGlobalConfig));
        assert!(capability_projections::project_capabilities(
            &tool.id,
            capability_projections::ToolCapabilityModule::OrdinaryConfig,
            &tool.capabilities,
        )
        .iter()
        .any(|projection| projection.source_role
            == capability_projections::ToolCapabilitySourceRole::OrdinaryConfigFile));
    }

    #[test]
    fn custom_tool_does_not_override_builtin_adapter_row() {
        let adapter = DefaultSkillAdapter;
        let mut config = crate::platform::config::default_config();
        config
            .custom_tools
            .push(crate::platform::config::CustomTool {
                id: "default-skill".to_string(),
                name: "Custom Collision".to_string(),
                icon: "wrench".to_string(),
                config_dir: "/tmp/custom-collision".to_string(),
                rule_directory: "/tmp/custom-collision/rules".to_string(),
                global_rule_file: String::new(),
                skills_dir: "/tmp/custom-collision/skills".to_string(),
                shared_skill_direct_read: false,
                mcp_config: String::new(),
                tool_config: String::new(),
                rule_file: String::new(),
            });
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(adapter)]);
        let tools = registry.detect_all_for_config(&config);

        assert_eq!(
            tools
                .iter()
                .filter(|tool| tool.id == "default-skill")
                .count(),
            1
        );
        assert_eq!(
            tools
                .iter()
                .find(|tool| tool.id == "default-skill")
                .map(|tool| tool.name.as_str()),
            Some("Default Skill")
        );
    }

    #[test]
    fn custom_tool_without_feature_sources_stays_out_of_discovery() {
        let mut config = crate::platform::config::default_config();
        config
            .custom_tools
            .push(crate::platform::config::CustomTool {
                id: "custom-draft".to_string(),
                name: "Custom Draft".to_string(),
                icon: "wrench".to_string(),
                config_dir: "/tmp/custom-draft".to_string(),
                rule_directory: String::new(),
                global_rule_file: String::new(),
                skills_dir: String::new(),
                shared_skill_direct_read: false,
                mcp_config: String::new(),
                tool_config: String::new(),
                rule_file: String::new(),
            });
        let registry = ToolRegistry::from_adapters_for_tests(vec![]);
        let tools = registry.detect_all_for_config(&config);

        assert!(tools.iter().all(|tool| tool.id != "custom-draft"));
    }

    struct ProjectionSkillAdapter {
        capabilities: Vec<ToolCapability>,
    }

    impl ToolAdapter for ProjectionSkillAdapter {
        fn id(&self) -> &str {
            "projection-skill"
        }
        fn name(&self) -> &str {
            "Projection Skill"
        }
        fn icon(&self) -> &str {
            "P"
        }
        fn config_dir(&self) -> PathBuf {
            PathBuf::from("/tmp/projection-skill")
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
            self.capabilities.clone()
        }
    }

    struct HealthAdapter {
        config_dir: PathBuf,
        detected: bool,
    }

    impl ToolAdapter for HealthAdapter {
        fn id(&self) -> &str {
            "health-test"
        }
        fn name(&self) -> &str {
            "Health Test"
        }
        fn icon(&self) -> &str {
            "H"
        }
        fn config_dir(&self) -> PathBuf {
            self.config_dir.clone()
        }
        fn detect(&self) -> bool {
            self.detected
        }
        fn read_rules(&self) -> Result<Vec<RuleSource>, String> {
            Ok(vec![])
        }
        fn write_rule(&self, _path: &str, _content: &str) -> Result<(), String> {
            Ok(())
        }
    }

    struct PresenceAdapter {
        app_detected: bool,
        cli_detected: bool,
    }

    impl ToolAdapter for PresenceAdapter {
        fn id(&self) -> &str {
            "presence-test"
        }
        fn name(&self) -> &str {
            "Presence Test"
        }
        fn icon(&self) -> &str {
            "P"
        }
        fn config_dir(&self) -> PathBuf {
            PathBuf::from("/tmp/presence-test")
        }
        fn detect(&self) -> bool {
            self.presence().detected
        }
        fn presence(&self) -> ToolPresence {
            ToolPresence::from_presence(self.app_detected, self.cli_detected)
        }
        fn read_rules(&self) -> Result<Vec<RuleSource>, String> {
            Ok(vec![])
        }
        fn write_rule(&self, _path: &str, _content: &str) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn detected_tool_payload_includes_structured_presence_fields() {
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(PresenceAdapter {
            app_detected: true,
            cli_detected: true,
        })]);

        let tool = registry.detect_all().pop().expect("expected tool");

        assert!(tool.detected);
        assert!(tool.presence_app_detected);
        assert!(tool.presence_cli_detected);
        assert_eq!(tool.presence_label, "APP+CLI");
    }

    #[test]
    fn present_tool_reports_missing_primary_config_health() {
        let tmp = tempfile::tempdir().unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(HealthAdapter {
            config_dir: tmp.path().join("missing-config"),
            detected: true,
        })]);

        let tool = registry.detect_all().pop().expect("expected detected tool");

        assert!(tool.detected);
        assert_eq!(tool.primary_config_health, PrimaryConfigHealth::Missing);
    }

    #[test]
    fn absent_tool_ignores_leftover_primary_config_health() {
        let tmp = tempfile::tempdir().unwrap();
        let leftover = tmp.path().join("leftover-config");
        std::fs::create_dir_all(&leftover).unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(HealthAdapter {
            config_dir: leftover,
            detected: false,
        })]);

        let tool = registry.detect_all().pop().expect("expected tool");

        assert!(!tool.detected);
        assert_eq!(tool.primary_config_health, PrimaryConfigHealth::Unknown);
    }

    #[test]
    fn present_tool_uses_user_config_dir_override_for_primary_config_health() {
        let tmp = tempfile::tempdir().unwrap();
        let override_config = tmp.path().join("override-config");
        std::fs::create_dir_all(&override_config).unwrap();
        let mut config = crate::platform::config::default_config();
        config.tool_paths.insert(
            "health-test".to_string(),
            crate::platform::config::ToolPaths {
                config_dir: override_config.to_string_lossy().to_string(),
                skills_dir: String::new(),
            },
        );
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(HealthAdapter {
            config_dir: tmp.path().join("default-missing-config"),
            detected: true,
        })]);

        let tool = registry
            .detect_all_for_config(&config)
            .pop()
            .expect("expected tool");

        assert!(tool.detected);
        assert_eq!(
            tool.default_config_dir,
            tmp.path()
                .join("default-missing-config")
                .to_string_lossy()
                .to_string()
        );
        assert_eq!(
            tool.primary_config_dir,
            override_config.to_string_lossy().to_string()
        );
        assert_eq!(
            tool.default_primary_config_dir,
            tmp.path()
                .join("default-missing-config")
                .to_string_lossy()
                .to_string()
        );
        assert_eq!(tool.primary_config_health, PrimaryConfigHealth::Ok);
    }

    #[test]
    fn detected_tool_compatibility_skill_fields_are_projection_derived() {
        let adapter = ProjectionSkillAdapter {
            capabilities: vec![
                tool_capability(
                    "tool-skills",
                    ToolCapabilityKind::Skill,
                    ToolCapabilityScope::Tool,
                    ToolCapabilityAccess::Writable,
                    ToolCapabilityFormat::SkillDirectory,
                    "/tmp/projection-skill/skills",
                    "Tool Skills",
                    ToolCapabilitySourceConfidence::OfficialDocs,
                    "test",
                ),
                tool_capability(
                    "shared-skills",
                    ToolCapabilityKind::Skill,
                    ToolCapabilityScope::Shared,
                    ToolCapabilityAccess::ReadOnly,
                    ToolCapabilityFormat::SkillDirectory,
                    "/tmp/shared-skills",
                    "Shared Skills",
                    ToolCapabilitySourceConfidence::OfficialDocs,
                    "test",
                ),
            ],
        };
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(adapter)]);

        let tool = registry.detect_all().pop().expect("expected detected tool");

        assert_eq!(tool.default_config_dir, "/tmp/projection-skill");
        assert_eq!(tool.primary_config_dir, "/tmp/projection-skill");
        assert_eq!(tool.default_primary_config_dir, "/tmp/projection-skill");
        assert_eq!(tool.skills_dir, "/tmp/projection-skill/skills");
        assert_eq!(tool.default_skills_dir, "/tmp/projection-skill/skills");
        assert_eq!(
            tool.read_paths,
            vec![
                "/tmp/projection-skill/skills".to_string(),
                "/tmp/shared-skills".to_string()
            ]
        );
        assert_eq!(tool.managed_anchor, "/tmp/projection-skill/skills");
        assert!(tool.supports_generic_skills);
    }

    #[test]
    fn present_tool_uses_user_skill_dir_override_for_skill_health_and_default_path() {
        let tmp = tempfile::tempdir().unwrap();
        let default_skills = tmp.path().join("default-missing-skills");
        let override_skills = tmp.path().join("override-skills");
        std::fs::create_dir_all(&override_skills).unwrap();
        let mut config = crate::platform::config::default_config();
        config.tool_paths.insert(
            "projection-skill".to_string(),
            crate::platform::config::ToolPaths {
                config_dir: String::new(),
                skills_dir: override_skills.to_string_lossy().to_string(),
            },
        );
        let adapter = ProjectionSkillAdapter {
            capabilities: vec![tool_capability(
                "tool-skills",
                ToolCapabilityKind::Skill,
                ToolCapabilityScope::Tool,
                ToolCapabilityAccess::Writable,
                ToolCapabilityFormat::SkillDirectory,
                default_skills.to_string_lossy().as_ref(),
                "Tool Skills",
                ToolCapabilitySourceConfidence::OfficialDocs,
                "test",
            )],
        };
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(adapter)]);

        let tool = registry
            .detect_all_for_config(&config)
            .pop()
            .expect("expected detected tool");

        assert_eq!(
            tool.skills_dir,
            override_skills.to_string_lossy().to_string()
        );
        assert_eq!(
            tool.default_skills_dir,
            default_skills.to_string_lossy().to_string()
        );
        assert_eq!(tool.primary_skills_health, PrimaryConfigHealth::Ok);
    }

    #[test]
    fn present_tool_reports_visible_source_path_health() {
        let tmp = tempfile::tempdir().unwrap();
        let missing_rule_dir = tmp.path().join("missing-rules");
        let missing_global_rule = tmp.path().join("missing-global.md");
        let missing_mcp = tmp.path().join("missing-mcp.json");
        let existing_config = tmp.path().join("settings.json");
        std::fs::write(&existing_config, "{}").unwrap();
        let adapter = ProjectionSkillAdapter {
            capabilities: vec![
                tool_capability(
                    "rules",
                    ToolCapabilityKind::Rule,
                    ToolCapabilityScope::Global,
                    ToolCapabilityAccess::Writable,
                    ToolCapabilityFormat::Directory,
                    missing_rule_dir.to_string_lossy().as_ref(),
                    "Rules",
                    ToolCapabilitySourceConfidence::OfficialDocs,
                    "test",
                ),
                tool_capability(
                    "global-rule",
                    ToolCapabilityKind::Rule,
                    ToolCapabilityScope::Global,
                    ToolCapabilityAccess::Writable,
                    ToolCapabilityFormat::Markdown,
                    missing_global_rule.to_string_lossy().as_ref(),
                    "Global Rule",
                    ToolCapabilitySourceConfidence::OfficialDocs,
                    "test",
                ),
                tool_capability(
                    "mcp",
                    ToolCapabilityKind::Mcp,
                    ToolCapabilityScope::Global,
                    ToolCapabilityAccess::Writable,
                    ToolCapabilityFormat::Json,
                    missing_mcp.to_string_lossy().as_ref(),
                    "MCP",
                    ToolCapabilitySourceConfidence::OfficialDocs,
                    "test",
                ),
                tool_capability(
                    "settings",
                    ToolCapabilityKind::OrdinaryConfig,
                    ToolCapabilityScope::Global,
                    ToolCapabilityAccess::Writable,
                    ToolCapabilityFormat::Json,
                    existing_config.to_string_lossy().as_ref(),
                    "Settings",
                    ToolCapabilitySourceConfidence::OfficialDocs,
                    "test",
                ),
            ],
        };
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(adapter)]);

        let tool = registry.detect_all().pop().expect("expected detected tool");

        assert_eq!(tool.rule_directory_health, PrimaryConfigHealth::Missing);
        assert_eq!(tool.global_rule_file_health, PrimaryConfigHealth::Missing);
        assert_eq!(tool.mcp_config_health, PrimaryConfigHealth::Missing);
        assert_eq!(tool.tool_config_health, PrimaryConfigHealth::Ok);
    }

    #[test]
    fn present_tool_prefers_custom_source_path_health_over_certified_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let default_rule_dir = tmp.path().join("rules");
        let default_global_rule = tmp.path().join("CLAUDE.md");
        let default_mcp = tmp.path().join("mcp.json");
        let default_config = tmp.path().join("settings.json");
        std::fs::create_dir_all(&default_rule_dir).unwrap();
        std::fs::write(&default_global_rule, "").unwrap();
        std::fs::write(&default_mcp, "{}").unwrap();
        std::fs::write(&default_config, "{}").unwrap();
        let custom_rule_dir = tmp.path().join("rules1");
        let custom_global_rule = tmp.path().join("CLAUDE.md1");
        let custom_mcp = tmp.path().join("mcp.json1");
        let custom_config = tmp.path().join("settings.json1");
        let mut config = crate::platform::config::default_config();
        config.tool_capability_overrides.insert(
            "projection-skill".to_string(),
            crate::platform::config::ToolCapabilityOverrides {
                custom_rule_source_type: Some("directory".to_string()),
                custom_rule_source_path: Some(custom_rule_dir.to_string_lossy().to_string()),
                custom_global_rule_target: Some(custom_global_rule.to_string_lossy().to_string()),
                custom_mcp_config_path: Some(custom_mcp.to_string_lossy().to_string()),
                custom_tool_config_path: Some(custom_config.to_string_lossy().to_string()),
                shared_skill_direct_read: None,
            },
        );
        let adapter = ProjectionSkillAdapter {
            capabilities: vec![
                tool_capability(
                    "rules",
                    ToolCapabilityKind::Rule,
                    ToolCapabilityScope::Global,
                    ToolCapabilityAccess::Writable,
                    ToolCapabilityFormat::Directory,
                    default_rule_dir.to_string_lossy().as_ref(),
                    "Rules",
                    ToolCapabilitySourceConfidence::OfficialDocs,
                    "test",
                ),
                tool_capability(
                    "global-rule",
                    ToolCapabilityKind::Rule,
                    ToolCapabilityScope::Global,
                    ToolCapabilityAccess::Writable,
                    ToolCapabilityFormat::Markdown,
                    default_global_rule.to_string_lossy().as_ref(),
                    "Global Rule",
                    ToolCapabilitySourceConfidence::OfficialDocs,
                    "test",
                ),
                tool_capability(
                    "mcp",
                    ToolCapabilityKind::Mcp,
                    ToolCapabilityScope::Global,
                    ToolCapabilityAccess::Writable,
                    ToolCapabilityFormat::Json,
                    default_mcp.to_string_lossy().as_ref(),
                    "MCP",
                    ToolCapabilitySourceConfidence::OfficialDocs,
                    "test",
                ),
                tool_capability(
                    "settings",
                    ToolCapabilityKind::OrdinaryConfig,
                    ToolCapabilityScope::Global,
                    ToolCapabilityAccess::Writable,
                    ToolCapabilityFormat::Json,
                    default_config.to_string_lossy().as_ref(),
                    "Settings",
                    ToolCapabilitySourceConfidence::OfficialDocs,
                    "test",
                ),
            ],
        };
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(adapter)]);

        let tool = registry
            .detect_all_for_config(&config)
            .pop()
            .expect("expected detected tool");

        assert_eq!(tool.rule_directory_health, PrimaryConfigHealth::Missing);
        assert_eq!(tool.global_rule_file_health, PrimaryConfigHealth::Missing);
        assert_eq!(tool.mcp_config_health, PrimaryConfigHealth::Missing);
        assert_eq!(tool.tool_config_health, PrimaryConfigHealth::Missing);
    }

    #[test]
    fn detected_tool_managed_anchor_requires_writable_skill_projection() {
        let adapter = ProjectionSkillAdapter {
            capabilities: vec![tool_capability(
                "tool-skills",
                ToolCapabilityKind::Skill,
                ToolCapabilityScope::Tool,
                ToolCapabilityAccess::ReadOnly,
                ToolCapabilityFormat::SkillDirectory,
                "/tmp/projection-skill/skills",
                "Tool Skills",
                ToolCapabilitySourceConfidence::OfficialDocs,
                "test",
            )],
        };
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(adapter)]);

        let tool = registry.detect_all().pop().expect("expected detected tool");

        assert_eq!(tool.skills_dir, "/tmp/projection-skill/skills");
        assert_eq!(tool.managed_anchor, "");
        assert!(!tool.supports_generic_skills);
    }

    #[test]
    fn declared_market_capabilities_represent_uncertain_and_unsupported_states() {
        let home = std::path::Path::new("/tmp/modus-home");
        let mut adapters = declared_market_adapters(home);
        adapters.push(continue_adapter::create(home));
        let find_adapter = |id: &str| {
            adapters
                .iter()
                .find(|adapter| adapter.id() == id)
                .expect("expected declared adapter")
        };

        let continue_capabilities = find_adapter("continue").capabilities();
        assert!(!continue_capabilities
            .iter()
            .any(|capability| capability.kind == ToolCapabilityKind::OrdinaryConfig));
        let continue_mcp = continue_capabilities
            .iter()
            .find(|capability| capability.id == "mcp-config")
            .expect("expected Continue MCP capability");
        assert_eq!(continue_mcp.access, ToolCapabilityAccess::ReadOnly);

        let windsurf_capabilities = find_adapter("windsurf").capabilities();
        let mcp_support = windsurf_capabilities
            .iter()
            .find(|capability| capability.id == "mcp-config")
            .expect("expected Windsurf MCP capability");
        assert_eq!(mcp_support.access, ToolCapabilityAccess::Writable);
        assert_eq!(mcp_support.format, ToolCapabilityFormat::Json);
        assert_eq!(
            mcp_support.source_path,
            "/tmp/modus-home/.codeium/windsurf/mcp_config.json"
        );
        let mcp_projection = capability_projections::project_capability(
            "windsurf",
            capability_projections::ToolCapabilityModule::Mcp,
            mcp_support,
        );
        assert!(mcp_projection.allows(&capability_projections::ToolCapabilityAction::Save));
        assert!(!mcp_projection.allows(&capability_projections::ToolCapabilityAction::Sync));

        let skills = windsurf_capabilities
            .iter()
            .find(|capability| capability.id == "skills")
            .expect("expected Windsurf Skill capability");
        assert_eq!(skills.access, ToolCapabilityAccess::Writable);
        assert_eq!(
            skills.source_path,
            "/tmp/modus-home/.codeium/windsurf/skills"
        );
        let shared_skills = windsurf_capabilities
            .iter()
            .find(|capability| capability.id == "shared-skills")
            .expect("expected Windsurf shared Skill capability");
        assert_eq!(shared_skills.access, ToolCapabilityAccess::ReadOnly);
        assert_eq!(
            shared_skills.source_confidence,
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior
        );
        let ordinary_config = windsurf_capabilities
            .iter()
            .find(|capability| capability.id == "ordinary-config")
            .expect("expected Windsurf ordinary config boundary");
        assert_eq!(ordinary_config.access, ToolCapabilityAccess::Unsupported);
        assert_eq!(
            ordinary_config.diagnostics,
            vec![ToolSourceDiagnosticState::Unsupported]
        );
    }

    #[test]
    fn declared_market_capabilities_keep_cursor_rules_out_of_global_runtime_scope() {
        let adapters = declared_market_adapters(std::path::Path::new("/tmp/modus-home"));
        let cursor = adapters
            .iter()
            .find(|adapter| adapter.id() == "cursor")
            .expect("expected Cursor declared adapter");
        let capabilities = cursor.capabilities();

        assert!(!capabilities
            .iter()
            .any(|capability| capability.kind == ToolCapabilityKind::Rule));
        assert!(!capabilities.iter().any(|capability| {
            capability.source_path.contains(".cursor/rules")
                || capability.source_path.contains(".cursorrules")
                || capability.source_path.contains("AGENTS.md")
        }));
        assert!(capabilities.iter().any(|capability| {
            capability.id == "ordinary-config"
                && capability.source_path == "/tmp/modus-home/.cursor/cli-config.json"
        }));
    }

    #[test]
    fn trae_variant_adapters_use_verified_runtime_capability_boundaries() {
        let adapters = declared_market_adapters(std::path::Path::new("/tmp/modus-home"));
        let trae = adapters
            .iter()
            .find(|adapter| adapter.id() == "trae")
            .expect("expected Trae declared adapter");
        assert_eq!(trae.icon(), "trae");
        let trae_capabilities = trae.capabilities();
        let trae_user_rules = trae_capabilities
            .iter()
            .find(|capability| capability.id == "user-rules")
            .expect("expected Trae user rules capability");
        assert_eq!(trae_user_rules.access, ToolCapabilityAccess::Writable);
        assert_eq!(
            trae_user_rules.source_confidence,
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior
        );
        assert!(!capability_projections::project_capability(
            "trae",
            capability_projections::ToolCapabilityModule::Rules,
            trae_user_rules,
        )
        .allows(&capability_projections::ToolCapabilityAction::Inject));
        let trae_skills = trae_capabilities
            .iter()
            .find(|capability| capability.id == "skills")
            .expect("expected Trae Skill capability");
        assert_eq!(trae_skills.access, ToolCapabilityAccess::Writable);
        let trae_skill_projection = capability_projections::project_capability(
            "trae",
            capability_projections::ToolCapabilityModule::Skills,
            trae_skills,
        );
        assert_eq!(
            trae_skill_projection.source_role,
            capability_projections::ToolCapabilitySourceRole::SkillToolDirectory
        );
        assert_eq!(
            trae_skill_projection.evidence.source_path,
            "/tmp/modus-home/.trae/skills"
        );
        assert!(
            trae_skill_projection.allows(&capability_projections::ToolCapabilityAction::Install)
        );
        let trae_shared_skills = trae_capabilities
            .iter()
            .find(|capability| capability.id == "shared-skills")
            .expect("expected Trae shared Skill evidence");
        assert_eq!(trae_shared_skills.access, ToolCapabilityAccess::ReadOnly);
        assert!(capability_projections::project_capability(
            "trae",
            capability_projections::ToolCapabilityModule::Skills,
            trae_shared_skills,
        )
        .allows(&capability_projections::ToolCapabilityAction::View));

        let trae_cn = adapters
            .iter()
            .find(|adapter| adapter.id() == "trae-cn")
            .expect("expected Trae CN declared adapter");
        assert!(trae_cn.supports_generic_skills());
        assert!(trae_cn.allow_external_generic_symlink());
        let trae_cn_capabilities = trae_cn.capabilities();
        let trae_cn_rules = trae_cn_capabilities
            .iter()
            .find(|capability| capability.id == "user-rules")
            .expect("expected Trae CN Rules capability");
        assert_eq!(trae_cn_rules.access, ToolCapabilityAccess::Writable);
        assert!(!capability_projections::project_capability(
            "trae-cn",
            capability_projections::ToolCapabilityModule::Rules,
            trae_cn_rules,
        )
        .allows(&capability_projections::ToolCapabilityAction::Inject));
        let trae_cn_shared_skills = trae_cn_capabilities
            .iter()
            .find(|capability| capability.id == "shared-skills")
            .expect("expected Trae CN shared Skill evidence");
        assert_eq!(trae_cn_shared_skills.access, ToolCapabilityAccess::ReadOnly);
        assert_eq!(
            trae_cn_shared_skills.source_confidence,
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior
        );
        assert!(capability_projections::project_capability(
            "trae-cn",
            capability_projections::ToolCapabilityModule::Skills,
            trae_cn_shared_skills,
        )
        .allows(&capability_projections::ToolCapabilityAction::View));
        let trae_cn_mcp = trae_cn_capabilities
            .iter()
            .find(|capability| capability.id == "mcp-config")
            .expect("expected Trae CN MCP capability");
        assert!(capability_projections::project_capability(
            "trae-cn",
            capability_projections::ToolCapabilityModule::Mcp,
            trae_cn_mcp,
        )
        .allows(&capability_projections::ToolCapabilityAction::Save));

        let trae_solo = adapters
            .iter()
            .find(|adapter| adapter.id() == "trae-solo")
            .expect("expected Trae Solo declared adapter");
        assert_eq!(
            trae_solo.config_dir(),
            std::path::Path::new("/tmp/modus-home").join(".trae")
        );
        assert_eq!(
            trae_solo.primary_config_dir(),
            std::path::Path::new("/tmp/modus-home").join(".trae")
        );
        let trae_solo_capabilities = trae_solo.capabilities();
        let trae_solo_skills = trae_solo_capabilities
            .iter()
            .find(|capability| capability.id == "skills")
            .expect("expected Trae Solo Skills capability");
        assert_eq!(trae_solo_skills.access, ToolCapabilityAccess::Writable);
        let trae_solo_skill_projection = capability_projections::project_capability(
            "trae-solo",
            capability_projections::ToolCapabilityModule::Skills,
            trae_solo_skills,
        );
        assert_eq!(
            trae_solo_skill_projection.source_role,
            capability_projections::ToolCapabilitySourceRole::SkillToolDirectory
        );
        assert_eq!(
            trae_solo_skill_projection.evidence.source_path,
            "/tmp/modus-home/.trae/skills"
        );
        assert!(trae_solo_skill_projection
            .allows(&capability_projections::ToolCapabilityAction::Install));
        assert!(!trae_solo_capabilities
            .iter()
            .any(|capability| capability.id == "shared-skills"));

        let trae_solo_cn = adapters
            .iter()
            .find(|adapter| adapter.id() == "trae-solo-cn")
            .expect("expected Trae Solo CN declared adapter");
        assert_eq!(
            trae_solo_cn.config_dir(),
            std::path::Path::new("/tmp/modus-home").join(".trae-cn")
        );
        assert_eq!(
            trae_solo_cn.primary_config_dir(),
            std::path::Path::new("/tmp/modus-home").join(".trae-cn")
        );
        let trae_solo_cn_capabilities = trae_solo_cn.capabilities();
        let trae_rules = trae_solo_cn_capabilities
            .iter()
            .find(|capability| capability.id == "user-rules")
            .expect("expected Trae Solo CN Rules capability");
        assert_eq!(trae_rules.access, ToolCapabilityAccess::Writable);
        assert_eq!(
            trae_rules.source_confidence,
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior
        );
        assert_eq!(
            trae_rules.source_path,
            "/tmp/modus-home/.trae-cn/user_rules"
        );
        assert!(!capability_projections::project_capability(
            "trae-solo-cn",
            capability_projections::ToolCapabilityModule::Rules,
            trae_rules,
        )
        .allows(&capability_projections::ToolCapabilityAction::Inject));

        let trae_skills = trae_solo_cn_capabilities
            .iter()
            .find(|capability| capability.id == "skills")
            .expect("expected Trae Solo CN Skills capability");
        assert_eq!(trae_skills.access, ToolCapabilityAccess::Writable);
        assert!(trae_skills.supporting_sources.is_empty());
        let skill_projection = capability_projections::project_capability(
            "trae-solo-cn",
            capability_projections::ToolCapabilityModule::Skills,
            trae_skills,
        );
        assert_eq!(
            skill_projection.source_role,
            capability_projections::ToolCapabilitySourceRole::SkillToolDirectory
        );
        assert!(skill_projection.allows(&capability_projections::ToolCapabilityAction::Install));
        assert!(skill_projection.allows(&capability_projections::ToolCapabilityAction::Delete));

        let trae_mcp = trae_solo_cn_capabilities
            .iter()
            .find(|capability| capability.id == "mcp-config")
            .expect("expected Trae Solo CN MCP capability");
        assert_eq!(trae_mcp.access, ToolCapabilityAccess::Writable);
        assert!(capability_projections::project_capability(
            "trae-solo-cn",
            capability_projections::ToolCapabilityModule::Mcp,
            trae_mcp,
        )
        .allows(&capability_projections::ToolCapabilityAction::Save));
        assert!(trae_mcp
            .source_path
            .ends_with("Library/Application Support/TRAE SOLO CN/User/mcp.json"));
    }

    fn adapter_file_stem_for_tool_id(tool_id: &str) -> String {
        tool_id.replace('-', "_")
    }

    fn adapter_entry_file_stems() -> Vec<String> {
        let adapter_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("adapters");
        let mut entries = std::fs::read_dir(adapter_dir)
            .unwrap()
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                    path.file_stem()
                        .map(|stem| stem.to_string_lossy().to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        entries.sort();
        entries
    }

    #[test]
    fn every_builtin_tool_has_visible_adapter_entry_file() {
        let mut expected = crate::platform::tool_catalog::registry::known_ids()
            .into_iter()
            .map(|id| adapter_file_stem_for_tool_id(&id))
            .collect::<Vec<_>>();
        expected.sort();

        assert_eq!(adapter_entry_file_stems(), expected);
    }

    #[test]
    fn adapter_directory_contains_only_tool_entry_files() {
        let expected = adapter_entry_file_stems();
        let supported = crate::platform::tool_catalog::registry::known_ids()
            .into_iter()
            .map(|id| adapter_file_stem_for_tool_id(&id))
            .collect::<std::collections::BTreeSet<_>>();

        assert!(expected.iter().all(|entry| supported.contains(entry)));
    }

    #[test]
    fn runtime_builtin_registry_matches_current_support_catalog_ids() {
        let mut registered = builtin_tool_adapters(std::path::Path::new("/tmp/modus-home"))
            .into_iter()
            .map(|adapter| adapter.id().to_string())
            .collect::<Vec<_>>();
        registered.sort();

        let deferred = deferred_research_tool_ids();
        let mut expected = crate::platform::tool_catalog::registry::known_ids()
            .into_iter()
            .filter(|id| !deferred.contains(id))
            .collect::<Vec<_>>();
        expected.sort();

        assert_eq!(registered, expected);
    }
}
