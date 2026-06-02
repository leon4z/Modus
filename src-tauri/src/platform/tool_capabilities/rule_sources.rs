//! Shared discovery for file-backed and directory-backed tool rule sources.

use super::{
    capability_declared_source_path,
    capability_projections::{
        project_capability, ToolCapabilityAction, ToolCapabilityModule, ToolCapabilitySourceRole,
    },
    get_file_modified, normalized_path, RuleFormat, RuleSource, ToolCapability,
    ToolCapabilityFormat, ToolCapabilityKind, ToolCapabilitySourceConfidence,
    ToolSourceDiagnosticState,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct RuleSourceDiscovery {
    pub sources: Vec<RuleSource>,
    pub unreadable_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleDiscoverySourceKind {
    FixedFile,
    Directory,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolRuleSourceShape {
    SingleFile,
    FilePlusDirectory,
    SingleDirectory,
    DirectorySet,
}

#[derive(Debug, Clone)]
pub struct RuleDiscoveryPolicy {
    pub capability: ToolCapability,
    pub source: PathBuf,
    pub source_kind: RuleDiscoverySourceKind,
    pub recursive: bool,
    pub eligible_extensions: Vec<&'static str>,
    pub exclude_roots: Vec<PathBuf>,
    pub root_group: String,
}

#[cfg(test)]
pub fn discover_rule_sources(capabilities: &[ToolCapability]) -> Result<Vec<RuleSource>, String> {
    Ok(discover_rule_sources_with_diagnostics(capabilities)?.sources)
}

pub fn discover_rule_sources_for_adapter(
    adapter_id: &str,
    capabilities: &[ToolCapability],
) -> Result<Vec<RuleSource>, String> {
    Ok(discover_rule_sources_with_diagnostics_for_adapter(adapter_id, capabilities)?.sources)
}

#[cfg(test)]
pub fn discover_rule_sources_with_diagnostics(
    capabilities: &[ToolCapability],
) -> Result<RuleSourceDiscovery, String> {
    discover_rule_sources_with_diagnostics_for_adapter("", capabilities)
}

pub fn discover_rule_sources_with_diagnostics_for_adapter(
    adapter_id: &str,
    capabilities: &[ToolCapability],
) -> Result<RuleSourceDiscovery, String> {
    let mut discovery = RuleSourceDiscovery::default();
    let mut seen = HashSet::new();

    for policy in rule_discovery_policies_for_adapter(adapter_id, capabilities) {
        match policy.source_kind {
            RuleDiscoverySourceKind::Directory => {
                discover_directory(
                    &policy,
                    &policy.source,
                    &policy.source,
                    &mut seen,
                    &mut discovery,
                )?;
            }
            RuleDiscoverySourceKind::FixedFile => {
                discover_file(
                    &policy,
                    &policy.source,
                    "",
                    false,
                    &mut seen,
                    &mut discovery,
                );
            }
        }
    }

    discovery.sources.sort_by(|a, b| {
        a.group
            .cmp(&b.group)
            .then_with(|| a.label.cmp(&b.label))
            .then_with(|| a.path.cmp(&b.path))
    });

    Ok(discovery)
}

pub fn rule_discovery_policies_for_adapter(
    adapter_id: &str,
    capabilities: &[ToolCapability],
) -> Vec<RuleDiscoveryPolicy> {
    rule_policies_for_adapter(adapter_id, capabilities, None)
}

pub fn rule_source_shape_for_adapter(
    adapter_id: &str,
    capabilities: &[ToolCapability],
) -> Option<ToolRuleSourceShape> {
    let policies = rule_discovery_policies_for_adapter(adapter_id, capabilities);
    let file_count = policies
        .iter()
        .filter(|policy| policy.source_kind == RuleDiscoverySourceKind::FixedFile)
        .count();
    let directory_count = policies
        .iter()
        .filter(|policy| policy.source_kind == RuleDiscoverySourceKind::Directory)
        .count();

    match (file_count, directory_count) {
        (1, 0) => Some(ToolRuleSourceShape::SingleFile),
        (0, 1) => Some(ToolRuleSourceShape::SingleDirectory),
        (files, directories) if files >= 1 && directories >= 1 => {
            Some(ToolRuleSourceShape::FilePlusDirectory)
        }
        (0, directories) if directories > 1 => Some(ToolRuleSourceShape::DirectorySet),
        _ => None,
    }
}

fn rule_policies_for_adapter(
    adapter_id: &str,
    capabilities: &[ToolCapability],
    required_action: Option<&ToolCapabilityAction>,
) -> Vec<RuleDiscoveryPolicy> {
    let mut rule_sources: Vec<(ToolCapability, PathBuf, RuleDiscoverySourceKind)> = capabilities
        .iter()
        .filter(|capability| {
            is_rule_policy_capability_for_adapter(adapter_id, capability, required_action)
        })
        .filter_map(|capability| {
            let source = capability_declared_source_path(capability)?;
            let source_kind = if is_directory_rule_capability(capability, &source) {
                RuleDiscoverySourceKind::Directory
            } else {
                RuleDiscoverySourceKind::FixedFile
            };
            Some((capability.clone(), source, source_kind))
        })
        .collect();
    if rule_sources.iter().any(|(capability, _, _)| {
        is_user_configured_rule_source_for_adapter(adapter_id, capability)
    }) {
        rule_sources.retain(|(capability, _, _)| {
            let projection =
                project_capability(adapter_id, ToolCapabilityModule::Rules, capability);
            projection.source_role != ToolCapabilitySourceRole::RuleNativeFileSource
                || is_user_configured_rule_source_for_adapter(adapter_id, capability)
        });
    }
    let directory_count = rule_sources
        .iter()
        .filter(|(_, _, source_kind)| *source_kind == RuleDiscoverySourceKind::Directory)
        .count();
    let non_rule_roots = non_rule_exclusion_roots(capabilities);

    rule_sources
        .into_iter()
        .map(|(capability, source, source_kind)| {
            let exclude_roots = non_rule_roots
                .iter()
                .filter(|root| !source.starts_with(root))
                .cloned()
                .collect();
            let root_group =
                if source_kind == RuleDiscoverySourceKind::Directory && directory_count > 1 {
                    source
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                };
            RuleDiscoveryPolicy {
                eligible_extensions: eligible_extensions(&capability),
                capability,
                source,
                source_kind,
                recursive: true,
                exclude_roots,
                root_group,
            }
        })
        .collect()
}

fn is_user_configured_rule_source_for_adapter(
    adapter_id: &str,
    capability: &ToolCapability,
) -> bool {
    let projection = project_capability(adapter_id, ToolCapabilityModule::Rules, capability);
    capability.source_confidence == ToolCapabilitySourceConfidence::UserConfigured
        && projection.source_role == ToolCapabilitySourceRole::RuleNativeFileSource
        && projection.exclusion_reason.is_none()
}

#[cfg(test)]
pub fn rule_capabilities_matching_path(
    capabilities: &[ToolCapability],
    path: &Path,
) -> Vec<ToolCapability> {
    rule_capabilities_matching_path_for_adapter("", capabilities, path)
}

pub fn rule_capabilities_matching_path_for_adapter(
    adapter_id: &str,
    capabilities: &[ToolCapability],
    path: &Path,
) -> Vec<ToolCapability> {
    rule_file_capabilities_matching_path_for_adapter_action(
        adapter_id,
        capabilities,
        path,
        &ToolCapabilityAction::Create,
    )
}

pub fn rule_file_capabilities_matching_path_for_adapter_action(
    adapter_id: &str,
    capabilities: &[ToolCapability],
    path: &Path,
    action: &ToolCapabilityAction,
) -> Vec<ToolCapability> {
    let target = normalized_path(path);
    rule_policies_for_adapter(adapter_id, capabilities, Some(action))
        .into_iter()
        .filter(|policy| policy_matches_path(policy, &target))
        .map(|policy| policy.capability)
        .collect()
}

pub fn rule_directory_capabilities_matching_path_for_adapter_action(
    adapter_id: &str,
    capabilities: &[ToolCapability],
    path: &Path,
    action: &ToolCapabilityAction,
) -> Vec<ToolCapability> {
    let target = normalized_path(path);
    rule_policies_for_adapter(adapter_id, capabilities, Some(action))
        .into_iter()
        .filter(|policy| {
            policy.source_kind == RuleDiscoverySourceKind::Directory
                && directory_policy_matches_path(policy, &target)
        })
        .map(|policy| policy.capability)
        .collect()
}

#[cfg(test)]
pub fn can_write_rule_path(capabilities: &[ToolCapability], path: &Path) -> bool {
    rule_capabilities_matching_path(capabilities, path)
        .iter()
        .any(|capability| capability.is_writable())
}

pub fn can_write_rule_path_for_adapter(
    adapter_id: &str,
    capabilities: &[ToolCapability],
    path: &Path,
) -> bool {
    !rule_capabilities_matching_path_for_adapter(adapter_id, capabilities, path).is_empty()
}

fn is_rule_policy_capability_for_adapter(
    adapter_id: &str,
    capability: &ToolCapability,
    required_action: Option<&ToolCapabilityAction>,
) -> bool {
    let projection = project_capability(adapter_id, ToolCapabilityModule::Rules, capability);
    if projection.exclusion_reason
        == Some(super::capability_projections::ToolCapabilityExclusionReason::WrongKind)
    {
        return false;
    }
    if !matches!(
        projection.source_role,
        ToolCapabilitySourceRole::RuleGlobalTarget | ToolCapabilitySourceRole::RuleNativeFileSource
    ) {
        return false;
    }
    if projection.source_role == ToolCapabilitySourceRole::RuleGlobalTarget
        && capability.source_confidence == ToolCapabilitySourceConfidence::UserConfigured
    {
        return false;
    }
    if let Some(action) = required_action {
        return projection.allows(action);
    }
    projection.allows(&ToolCapabilityAction::View)
}

fn is_directory_rule_capability(capability: &ToolCapability, source: &Path) -> bool {
    matches!(
        capability.format,
        ToolCapabilityFormat::Directory | ToolCapabilityFormat::InstructionsMarkdown
    ) || source.is_dir()
        || (!source.is_file() && source.extension().is_none())
}

fn non_rule_exclusion_roots(capabilities: &[ToolCapability]) -> Vec<PathBuf> {
    capabilities
        .iter()
        .filter(|capability| capability.kind != ToolCapabilityKind::Rule)
        .filter_map(capability_declared_source_path)
        .map(|path| normalized_path(&path))
        .collect()
}

fn eligible_extensions(capability: &ToolCapability) -> Vec<&'static str> {
    match capability.format {
        ToolCapabilityFormat::Mdc => vec!["mdc"],
        ToolCapabilityFormat::Markdown
        | ToolCapabilityFormat::InstructionsMarkdown
        | ToolCapabilityFormat::Directory => vec!["md"],
        _ => vec![],
    }
}

fn policy_matches_path(policy: &RuleDiscoveryPolicy, path: &Path) -> bool {
    match policy.source_kind {
        RuleDiscoverySourceKind::FixedFile => normalized_path(path) == policy.source,
        RuleDiscoverySourceKind::Directory => {
            path.starts_with(&policy.source)
                && is_eligible_policy_file(policy, path)
                && !is_excluded(policy, path)
        }
    }
}

fn directory_policy_matches_path(policy: &RuleDiscoveryPolicy, path: &Path) -> bool {
    policy.source_kind == RuleDiscoverySourceKind::Directory
        && path.starts_with(&policy.source)
        && !is_excluded(policy, path)
}

fn is_excluded(policy: &RuleDiscoveryPolicy, path: &Path) -> bool {
    policy
        .exclude_roots
        .iter()
        .any(|root| path.starts_with(root))
}

fn discover_directory(
    policy: &RuleDiscoveryPolicy,
    root: &Path,
    dir: &Path,
    seen: &mut HashSet<PathBuf>,
    discovery: &mut RuleSourceDiscovery,
) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    if !dir.is_dir() {
        discover_file(policy, dir, &policy.root_group, true, seen, discovery);
        return Ok(());
    }
    if is_excluded(policy, &normalized_path(dir)) {
        return Ok(());
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => {
            push_unreadable_source(policy, dir, &policy.root_group, true, seen, discovery);
            return Ok(());
        }
    };

    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();

    for path in paths {
        if path.is_dir() {
            if policy.recursive {
                push_directory_source(policy, root, &path, seen, discovery);
                discover_directory(policy, root, &path, seen, discovery)?;
            }
        } else if is_eligible_policy_file(policy, &path) {
            let relative_group = path
                .parent()
                .and_then(|parent| parent.strip_prefix(root).ok())
                .map(|relative| relative.to_string_lossy().to_string())
                .unwrap_or_default();
            let group = match (policy.root_group.is_empty(), relative_group.is_empty()) {
                (true, _) => relative_group,
                (false, true) => policy.root_group.to_string(),
                (false, false) => format!("{}/{}", policy.root_group, relative_group),
            };
            discover_file(policy, &path, &group, true, seen, discovery);
        }
    }
    Ok(())
}

fn push_directory_source(
    policy: &RuleDiscoveryPolicy,
    root: &Path,
    path: &Path,
    seen: &mut HashSet<PathBuf>,
    discovery: &mut RuleSourceDiscovery,
) {
    if path == root || is_excluded(policy, &normalized_path(path)) {
        return;
    }
    let key = normalized_path(path);
    if !seen.insert(key) {
        return;
    }
    let relative_group = path
        .strip_prefix(root)
        .ok()
        .map(|relative| relative.to_string_lossy().to_string())
        .unwrap_or_default();
    let group = match (policy.root_group.is_empty(), relative_group.is_empty()) {
        (true, _) => relative_group,
        (false, true) => policy.root_group.to_string(),
        (false, false) => format!("{}/{}", policy.root_group, relative_group),
    };
    discovery.sources.push(RuleSource {
        path: path.to_string_lossy().to_string(),
        format: RuleFormat::Directory,
        content: String::new(),
        last_modified: get_file_modified(path),
        label: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        group,
        diagnostic: None,
    });
}

fn discover_file(
    policy: &RuleDiscoveryPolicy,
    path: &Path,
    group: &str,
    from_directory: bool,
    seen: &mut HashSet<PathBuf>,
    discovery: &mut RuleSourceDiscovery,
) {
    if !path.exists() || !path.is_file() {
        return;
    }
    if !is_eligible_policy_file(policy, path) {
        return;
    }

    let key = normalized_path(path);
    if seen.contains(&key) {
        return;
    }

    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => {
            push_unreadable_source(policy, path, group, from_directory, seen, discovery);
            return;
        }
    };
    seen.insert(key);

    let label = if !from_directory {
        policy.capability.label.clone()
    } else {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    };

    discovery.sources.push(RuleSource {
        path: path.to_string_lossy().to_string(),
        format: if from_directory {
            RuleFormat::DirectoryMarkdown
        } else {
            RuleFormat::SingleMarkdown
        },
        content,
        last_modified: get_file_modified(path),
        label,
        group: group.to_string(),
        diagnostic: None,
    });
}

fn push_unreadable_source(
    policy: &RuleDiscoveryPolicy,
    path: &Path,
    group: &str,
    from_directory: bool,
    seen: &mut HashSet<PathBuf>,
    discovery: &mut RuleSourceDiscovery,
) {
    let key = normalized_path(path);
    if !seen.insert(key) {
        return;
    }

    let path_string = path.to_string_lossy().to_string();
    discovery.unreadable_paths.push(path_string.clone());
    let label = if !from_directory {
        policy.capability.label.clone()
    } else {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    };

    discovery.sources.push(RuleSource {
        path: path_string,
        format: if from_directory {
            RuleFormat::DirectoryMarkdown
        } else {
            RuleFormat::SingleMarkdown
        },
        content: String::new(),
        last_modified: get_file_modified(path),
        label,
        group: group.to_string(),
        diagnostic: Some(ToolSourceDiagnosticState::Unreadable),
    });
}

fn is_eligible_policy_file(policy: &RuleDiscoveryPolicy, path: &Path) -> bool {
    if matches!(
        policy.capability.format,
        ToolCapabilityFormat::InstructionsMarkdown
    ) {
        return path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(".instructions.md"))
            .unwrap_or(false);
    }

    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    policy
        .eligible_extensions
        .iter()
        .any(|eligible| ext.eq_ignore_ascii_case(eligible))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::tool_capabilities::{
        directory_capability, tool_capability, ToolCapabilityAccess, ToolCapabilityActionEvidence,
        ToolCapabilityFormat, ToolCapabilityKind, ToolCapabilityScope,
        ToolCapabilitySourceConfidence,
    };

    fn file_capability(path: &Path) -> ToolCapability {
        tool_capability(
            "rule-file",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Markdown,
            path.to_string_lossy().to_string(),
            "RULES.md",
            ToolCapabilitySourceConfidence::OfficialRepository,
            "test",
        )
    }

    fn dir_capability(path: &Path) -> ToolCapability {
        let mut capability = directory_capability(
            "rule-dir",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            path.to_string_lossy().to_string(),
            "Rule directory",
            ToolCapabilitySourceConfidence::OfficialRepository,
            "test",
        );
        capability.format = ToolCapabilityFormat::Markdown;
        capability
    }

    fn skill_capability(path: &Path) -> ToolCapability {
        let mut capability = directory_capability(
            "skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            path.to_string_lossy().to_string(),
            "Skills",
            ToolCapabilitySourceConfidence::OfficialRepository,
            "test",
        );
        capability.format = ToolCapabilityFormat::SkillDirectory;
        capability
    }

    fn trae_cn_rule_capability(path: &Path) -> ToolCapability {
        let mut capability = directory_capability(
            "user-rules",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            path.to_string_lossy().to_string(),
            "User Rules",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            "Trae CN user rules",
        );
        capability.format = ToolCapabilityFormat::Directory;
        capability.action_evidence = [
            ToolCapabilityAction::View,
            ToolCapabilityAction::Create,
            ToolCapabilityAction::Save,
            ToolCapabilityAction::Delete,
        ]
        .into_iter()
        .map(|action| ToolCapabilityActionEvidence {
            action,
            supported: true,
            evidence: "verified".to_string(),
            variant: Some("Trae CN".to_string()),
            version: None,
            verified_at: Some("2026-05-16".to_string()),
        })
        .collect();
        capability
    }

    #[test]
    fn discovers_one_file_backed_rule_source() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("RULES.md");
        fs::write(&path, "one").unwrap();

        let sources = discover_rule_sources(&[file_capability(&path)]).unwrap();

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].label, "RULES.md");
        assert_eq!(sources[0].content, "one");
        assert!(sources[0].group.is_empty());
    }

    #[test]
    fn discovers_multiple_files_in_rule_directory() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.md"), "a").unwrap();
        fs::write(tmp.path().join("b.md"), "b").unwrap();
        fs::write(tmp.path().join("ignored.txt"), "ignored").unwrap();

        let sources = discover_rule_sources(&[dir_capability(tmp.path())]).unwrap();

        let labels: Vec<_> = sources.iter().map(|source| source.label.as_str()).collect();
        assert_eq!(labels, vec!["a.md", "b.md"]);
    }

    #[test]
    fn discovers_nested_directory_rule_files_with_groups() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("team").join("frontend");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("rules.md"), "nested").unwrap();

        let sources = discover_rule_sources(&[dir_capability(tmp.path())]).unwrap();

        let directory_sources: Vec<_> = sources
            .iter()
            .filter(|source| matches!(source.format, RuleFormat::Directory))
            .collect();
        let file_sources: Vec<_> = sources
            .iter()
            .filter(|source| matches!(source.format, RuleFormat::DirectoryMarkdown))
            .collect();

        assert_eq!(
            directory_sources
                .iter()
                .map(|source| (source.label.as_str(), source.group.as_str()))
                .collect::<Vec<_>>(),
            vec![("team", "team"), ("frontend", "team/frontend")]
        );
        assert_eq!(file_sources.len(), 1);
        assert_eq!(file_sources[0].label, "rules.md");
        assert_eq!(file_sources[0].group, "team/frontend");
    }

    #[test]
    fn deduplicates_mixed_file_and_directory_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("RULES.md");
        fs::write(&file, "same").unwrap();
        fs::write(tmp.path().join("other.md"), "other").unwrap();

        let sources =
            discover_rule_sources(&[file_capability(&file), dir_capability(tmp.path())]).unwrap();

        let labels: Vec<_> = sources.iter().map(|source| source.label.as_str()).collect();
        assert_eq!(labels, vec!["RULES.md", "other.md"]);
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn excludes_non_rule_capability_roots_from_rule_directory_scan() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        fs::create_dir_all(skills_dir.join("demo")).unwrap();
        fs::write(tmp.path().join("RULES.md"), "rules").unwrap();
        fs::write(skills_dir.join("demo").join("SKILL.md"), "skill").unwrap();

        let sources =
            discover_rule_sources(&[dir_capability(tmp.path()), skill_capability(&skills_dir)])
                .unwrap();

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].label, "RULES.md");
    }

    #[test]
    fn explicit_rule_source_under_excluded_root_still_wins() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();
        let rule_path = skills_dir.join("SKILL.md");
        fs::write(&rule_path, "explicit").unwrap();

        let sources =
            discover_rule_sources(&[file_capability(&rule_path), skill_capability(&skills_dir)])
                .unwrap();

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].path, rule_path.to_string_lossy());
        assert_eq!(sources[0].content, "explicit");
    }

    #[test]
    fn writable_rule_policy_rejects_excluded_skill_files() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let rule_path = tmp.path().join("new.md");
        let skill_path = skills_dir.join("demo").join("SKILL.md");
        let capabilities = vec![dir_capability(tmp.path()), skill_capability(&skills_dir)];

        assert!(can_write_rule_path(&capabilities, &rule_path));
        assert!(!can_write_rule_path(&capabilities, &skill_path));
    }

    #[test]
    fn trae_cn_rule_policy_discovers_only_user_rule_directory_and_allows_native_writes() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("user_rules");
        let skills_dir = tmp.path().join("skills");
        let mcp_file = tmp.path().join("mcp.json");
        fs::create_dir_all(skills_dir.join("demo")).unwrap();
        fs::create_dir_all(&rules_dir).unwrap();
        fs::write(rules_dir.join("rule.md"), "rule").unwrap();
        fs::write(skills_dir.join("demo").join("SKILL.md"), "skill").unwrap();
        fs::write(&mcp_file, r#"{"mcpServers":{}}"#).unwrap();
        let mcp = tool_capability(
            "mcp-config",
            ToolCapabilityKind::Mcp,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::Writable,
            ToolCapabilityFormat::Json,
            mcp_file.to_string_lossy().to_string(),
            "MCP",
            ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior,
            "mcp",
        );
        let capabilities = vec![
            trae_cn_rule_capability(&rules_dir),
            skill_capability(&skills_dir),
            mcp,
        ];

        let sources = discover_rule_sources_for_adapter("trae-cn", &capabilities).unwrap();

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].label, "rule.md");
        assert!(can_write_rule_path_for_adapter(
            "trae-cn",
            &capabilities,
            &rules_dir.join("created.md")
        ));
        assert!(!can_write_rule_path_for_adapter(
            "trae-cn",
            &capabilities,
            &skills_dir.join("demo").join("SKILL.md")
        ));
        assert!(!can_write_rule_path_for_adapter(
            "trae-cn",
            &capabilities,
            &tmp.path().join(".trae").join("project_rules.md")
        ));
    }

    #[test]
    fn user_configured_rule_source_takes_precedence_over_certified_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let certified = tmp.path().join("certified");
        let configured = tmp.path().join("configured");
        fs::create_dir_all(&certified).unwrap();
        fs::create_dir_all(&configured).unwrap();
        fs::write(certified.join("default.md"), "default").unwrap();
        fs::write(configured.join("custom.md"), "custom").unwrap();
        let mut configured_capability = dir_capability(&configured);
        configured_capability.id = "user-configured-rule-source-directory".to_string();
        configured_capability.source_confidence = ToolCapabilitySourceConfidence::UserConfigured;
        configured_capability.action_evidence = [
            ToolCapabilityAction::View,
            ToolCapabilityAction::Create,
            ToolCapabilityAction::Save,
            ToolCapabilityAction::Delete,
        ]
        .into_iter()
        .map(|action| ToolCapabilityActionEvidence {
            action,
            supported: true,
            evidence: "user configured".to_string(),
            variant: None,
            version: None,
            verified_at: None,
        })
        .collect();

        let capabilities = vec![dir_capability(&certified), configured_capability];
        let sources = discover_rule_sources_for_adapter("codex", &capabilities).unwrap();

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].label, "custom.md");
        assert_eq!(sources[0].content, "custom");
        assert!(!can_write_rule_path_for_adapter(
            "codex",
            &capabilities,
            &certified.join("new.md")
        ));
        assert!(can_write_rule_path_for_adapter(
            "codex",
            &capabilities,
            &configured.join("new.md")
        ));
    }

    #[test]
    fn rule_discovery_ignores_project_and_structured_config_rule_facts() {
        let tmp = tempfile::tempdir().unwrap();
        let global_file = tmp.path().join("GLOBAL.md");
        let project_file = tmp.path().join("project.mdc");
        let config_file = tmp.path().join("config.yaml");
        fs::write(&global_file, "global").unwrap();
        fs::write(&project_file, "project").unwrap();
        fs::write(&config_file, "rules: []").unwrap();
        let mut project_rule = file_capability(&project_file);
        project_rule.id = "project-rule".to_string();
        project_rule.scope = ToolCapabilityScope::Project;
        project_rule.format = ToolCapabilityFormat::Mdc;
        let structured_rule = tool_capability(
            "rules-config",
            ToolCapabilityKind::Rule,
            ToolCapabilityScope::Global,
            ToolCapabilityAccess::ReadOnly,
            ToolCapabilityFormat::Yaml,
            format!("{}#rules", config_file.to_string_lossy()),
            "Rules config",
            ToolCapabilitySourceConfidence::OfficialDocs,
            "structured config",
        );

        let sources =
            discover_rule_sources(&[file_capability(&global_file), project_rule, structured_rule])
                .unwrap();

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].path, global_file.to_string_lossy());
    }

    #[test]
    fn rule_discovery_ignores_user_configured_global_injection_target() {
        let tmp = tempfile::tempdir().unwrap();
        let custom_file = tmp.path().join("CUSTOM.md");
        let mut custom_target = file_capability(&custom_file);
        custom_target.id = "user-configured-global-rule-target".to_string();
        custom_target.source_confidence = ToolCapabilitySourceConfidence::UserConfigured;
        custom_target.action_evidence = [
            ToolCapabilityAction::View,
            ToolCapabilityAction::Create,
            ToolCapabilityAction::Inject,
        ]
        .into_iter()
        .map(|action| ToolCapabilityActionEvidence {
            action,
            supported: true,
            evidence: "user configured".to_string(),
            variant: None,
            version: None,
            verified_at: None,
        })
        .collect();

        assert!(
            discover_rule_sources_for_adapter("codex", &[custom_target.clone()])
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            rule_source_shape_for_adapter("codex", &[custom_target.clone()]),
            None
        );
        assert!(!can_write_rule_path_for_adapter(
            "codex",
            &[custom_target],
            &custom_file
        ));
    }

    #[test]
    fn derives_shared_rule_source_shapes_from_discovery_policies() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("GLOBAL.md");
        let dir_one = tmp.path().join("rules");
        let dir_two = tmp.path().join("workspace");

        assert_eq!(
            rule_source_shape_for_adapter("test", &[file_capability(&file)]),
            Some(ToolRuleSourceShape::SingleFile)
        );
        assert_eq!(
            rule_source_shape_for_adapter(
                "test",
                &[file_capability(&file), dir_capability(&dir_one)]
            ),
            Some(ToolRuleSourceShape::FilePlusDirectory)
        );
        assert_eq!(
            rule_source_shape_for_adapter("test", &[dir_capability(&dir_one)]),
            Some(ToolRuleSourceShape::SingleDirectory)
        );
        assert_eq!(
            rule_source_shape_for_adapter(
                "test",
                &[dir_capability(&dir_one), dir_capability(&dir_two)]
            ),
            Some(ToolRuleSourceShape::DirectorySet)
        );
    }

    #[test]
    fn discovery_keeps_unreadable_rule_sources_visible() {
        let tmp = tempfile::tempdir().unwrap();
        let readable = tmp.path().join("readable.md");
        let unreadable = tmp.path().join("unreadable.md");
        fs::write(&readable, "ok").unwrap();
        fs::write(&unreadable, [0xff, 0xfe, 0xfd]).unwrap();

        let discovery =
            discover_rule_sources_with_diagnostics(&[dir_capability(tmp.path())]).unwrap();

        assert_eq!(discovery.sources.len(), 2);
        assert_eq!(
            discovery.unreadable_paths,
            vec![unreadable.to_string_lossy().to_string()]
        );
        let unreadable_source = discovery
            .sources
            .iter()
            .find(|source| source.path == unreadable.to_string_lossy())
            .unwrap();
        assert_eq!(
            unreadable_source.diagnostic,
            Some(ToolSourceDiagnosticState::Unreadable)
        );
        assert_eq!(unreadable_source.content, "");
        assert!(discovery
            .sources
            .iter()
            .any(|source| source.path == readable.to_string_lossy() && source.content == "ok"));
    }
}
