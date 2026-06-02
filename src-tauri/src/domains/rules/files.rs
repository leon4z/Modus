// Purpose: Own Rules file read/write/copy workflows while adapters handle tool-specific writes.

use crate::adapters::ToolRegistry;
use crate::platform::tool_capabilities::{
    capability_declared_source_path, capability_is_eligible_global_rule_target,
    effective_capabilities, normalized_path,
    rule_sources::{
        rule_directory_capabilities_matching_path_for_adapter_action,
        rule_file_capabilities_matching_path_for_adapter_action,
    },
    ToolCapability, ToolCapabilityAction, ToolCapabilityFormat,
};
use serde::Serialize;
use std::{
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Default)]
pub struct RuleFileChangePreview {
    pub creates: Vec<String>,
    pub deletes: Vec<String>,
    pub overwrites: Vec<String>,
    pub preserves: Vec<String>,
    pub changes: Vec<RuleFileChange>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleFileChange {
    pub action: String,
    #[serde(rename = "changeKind")]
    pub change_kind: String,
    pub subject: String,
    pub path: String,
    #[serde(rename = "entryKind")]
    pub entry_kind: String,
}

fn preview_delete(tool_id: &str, path: &Path, entry_kind: &str) -> RuleFileChange {
    RuleFileChange {
        action: "delete".to_string(),
        change_kind: "delete".to_string(),
        subject: tool_id.to_string(),
        path: path.to_string_lossy().to_string(),
        entry_kind: entry_kind.to_string(),
    }
}

fn expanded_rule_path(path: &str) -> PathBuf {
    normalized_path(Path::new(&shellexpand::tilde(path).to_string()))
}

fn expanded_raw_rule_path(path: &str) -> PathBuf {
    PathBuf::from(shellexpand::tilde(path).to_string())
}

fn effective_rule_capabilities(adapter: &dyn crate::adapters::ToolAdapter) -> Vec<ToolCapability> {
    let config = crate::platform::config::load_config();
    effective_capabilities::resolve_effective_capabilities(
        adapter.id(),
        &adapter.capabilities(),
        &config,
    )
}

fn effective_rule_capabilities_for_tool(
    registry: &ToolRegistry,
    tool_id: &str,
) -> Result<(String, Vec<ToolCapability>), String> {
    if let Some(adapter) = registry.get_adapter(tool_id) {
        return Ok((
            adapter.id().to_string(),
            effective_rule_capabilities(adapter),
        ));
    }
    let config = crate::platform::config::load_config();
    let canonical = crate::platform::tool_catalog::normalization::canonical_tool_id(tool_id);
    registry
        .detect_all_for_config(&config)
        .into_iter()
        .find(|tool| tool.id == canonical && tool.detected)
        .map(|tool| (tool.id, tool.capabilities))
        .ok_or_else(|| format!("Tool not found: {}", tool_id))
}

fn write_rule_content(
    registry: &ToolRegistry,
    tool_id: &str,
    target_path: &Path,
    content: &str,
) -> Result<(), String> {
    if let Some(adapter) = registry.get_adapter(tool_id) {
        return adapter.write_rule(&target_path.to_string_lossy(), content);
    }
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create rule directory: {}", e))?;
    }
    std::fs::write(target_path, content).map_err(|e| format!("Failed to write rule file: {}", e))
}

fn capability_source_is_directory(capability: &ToolCapability, source: &Path) -> bool {
    matches!(capability.format, ToolCapabilityFormat::Directory)
        || source.is_dir()
        || (!source.is_file() && source.extension().is_none())
}

fn mutation_boundary_for_capability(capability: &ToolCapability) -> Option<PathBuf> {
    let source = capability_declared_source_path(capability)?;
    if capability_source_is_directory(capability, &source) {
        Some(source)
    } else {
        source.parent().map(Path::to_path_buf)
    }
}

fn path_has_existing_symlink_component_from_boundary(
    boundary: &Path,
    path: &Path,
) -> Result<bool, String> {
    let boundary = normalized_path(boundary);
    let normalized_target = normalized_path(path);
    if !normalized_target.starts_with(&boundary) {
        return Ok(true);
    }
    match std::fs::symlink_metadata(&boundary) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Ok(true);
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "Failed to inspect rule path {}: {}",
                boundary.display(),
                error
            ));
        }
    }

    let Ok(relative) = path.strip_prefix(&boundary) else {
        return Ok(true);
    };
    let mut current = boundary;
    for component in relative.components() {
        match component {
            Component::Normal(part) => current.push(part),
            Component::CurDir => continue,
            Component::ParentDir => return Ok(true),
            Component::Prefix(_) | Component::RootDir => return Ok(true),
        }
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Ok(true);
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
            Err(error) => {
                return Err(format!(
                    "Failed to inspect rule path {}: {}",
                    current.display(),
                    error
                ));
            }
        }
    }
    Ok(false)
}

fn capability_allows_safe_rule_path(capability: &ToolCapability, path: &Path) -> bool {
    let Some(boundary) = mutation_boundary_for_capability(capability) else {
        return false;
    };
    matches!(
        path_has_existing_symlink_component_from_boundary(&boundary, path),
        Ok(false)
    )
}

fn rule_file_allows_action(
    adapter_id: &str,
    capabilities: &[ToolCapability],
    path: &Path,
    action: &ToolCapabilityAction,
) -> bool {
    rule_file_capabilities_matching_path_for_adapter_action(adapter_id, capabilities, path, action)
        .iter()
        .any(|capability| capability_allows_safe_rule_path(capability, path))
}

fn rule_directory_allows_action(
    adapter_id: &str,
    capabilities: &[ToolCapability],
    path: &Path,
    action: &ToolCapabilityAction,
) -> bool {
    rule_directory_capabilities_matching_path_for_adapter_action(
        adapter_id,
        capabilities,
        path,
        action,
    )
    .iter()
    .any(|capability| capability_allows_safe_rule_path(capability, path))
}

fn is_declared_fixed_rule_target(
    adapter_id: &str,
    capabilities: &[ToolCapability],
    path: &Path,
) -> bool {
    let target = normalized_path(path);
    capabilities.iter().any(|capability| {
        capability_is_eligible_global_rule_target(adapter_id, capability)
            && capability_declared_source_path(capability)
                .map(|source| source == target)
                .unwrap_or(false)
    })
}

fn ensure_rule_file_action_path(
    registry: &ToolRegistry,
    tool_id: &str,
    path: &str,
    action: ToolCapabilityAction,
) -> Result<PathBuf, String> {
    let (adapter_id, capabilities) = effective_rule_capabilities_for_tool(registry, tool_id)?;
    let raw_path = expanded_raw_rule_path(path);
    let target_path = normalized_path(&raw_path);
    if rule_file_allows_action(&adapter_id, &capabilities, &raw_path, &action) {
        Ok(target_path)
    } else {
        Err("Rule target is not writable for this tool capability".to_string())
    }
}

fn ensure_rule_directory_action_path(
    registry: &ToolRegistry,
    tool_id: &str,
    path: &str,
    action: ToolCapabilityAction,
) -> Result<PathBuf, String> {
    let (adapter_id, capabilities) = effective_rule_capabilities_for_tool(registry, tool_id)?;
    let raw_path = expanded_raw_rule_path(path);
    let target_path = normalized_path(&raw_path);
    if rule_directory_allows_action(&adapter_id, &capabilities, &raw_path, &action) {
        Ok(target_path)
    } else {
        Err("Rule directory target is not writable for this tool capability".to_string())
    }
}

pub(crate) fn write_rule_domain(
    registry: &ToolRegistry,
    tool_id: String,
    path: String,
    content: String,
) -> Result<(), String> {
    let target_path =
        ensure_rule_file_action_path(registry, &tool_id, &path, ToolCapabilityAction::Save)?;
    write_rule_content(registry, &tool_id, &target_path, &content)
}

pub(crate) fn create_rule_file_domain(
    registry: &ToolRegistry,
    tool_id: String,
    path: String,
    content: String,
) -> Result<(), String> {
    let target_path =
        ensure_rule_file_action_path(registry, &tool_id, &path, ToolCapabilityAction::Create)?;
    if target_path.exists() {
        return Err("Rule file already exists".to_string());
    }
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create rule directory: {}", e))?;
    }

    write_rule_content(registry, &tool_id, &target_path, &content)
}

pub(crate) fn create_rule_directory_domain(
    registry: &ToolRegistry,
    tool_id: String,
    path: String,
) -> Result<(), String> {
    let target_path =
        ensure_rule_directory_action_path(registry, &tool_id, &path, ToolCapabilityAction::Create)?;
    if target_path.exists() {
        return Err("Rule directory already exists".to_string());
    }
    std::fs::create_dir_all(&target_path)
        .map_err(|e| format!("Failed to create rule directory: {}", e))
}

fn validate_plain_segment(name: &str) -> Result<(), String> {
    let value = name.trim();
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains('\0')
    {
        return Err("Invalid rule entry name".to_string());
    }
    Ok(())
}

fn collect_delete_preview_for_directory(
    adapter_id: &str,
    capabilities: &[ToolCapability],
    dir: &Path,
) -> Result<(Vec<PathBuf>, Vec<PathBuf>), String> {
    let mut files = vec![];
    let mut dirs = vec![dir.to_path_buf()];
    collect_delete_preview_for_directory_inner(
        adapter_id,
        capabilities,
        dir,
        &mut files,
        &mut dirs,
    )?;
    dirs.sort_by(|a, b| b.components().count().cmp(&a.components().count()));
    files.sort();
    Ok((files, dirs))
}

fn collect_delete_preview_for_directory_inner(
    adapter_id: &str,
    capabilities: &[ToolCapability],
    dir: &Path,
    files: &mut Vec<PathBuf>,
    dirs: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("Failed to inspect rule directory: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to inspect rule directory entry: {}", e))?;
        let path = normalized_path(&entry.path());
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to inspect rule directory entry: {}", e))?;
        if file_type.is_symlink() {
            return Err(format!(
                "Rule directory contains unsupported path: {}",
                path.display()
            ));
        }
        if file_type.is_dir() {
            if !rule_directory_allows_action(
                adapter_id,
                capabilities,
                &path,
                &ToolCapabilityAction::Delete,
            ) {
                return Err(format!(
                    "Rule directory contains non-rule directory: {}",
                    path.display()
                ));
            }
            dirs.push(path.clone());
            collect_delete_preview_for_directory_inner(
                adapter_id,
                capabilities,
                &path,
                files,
                dirs,
            )?;
        } else if file_type.is_file() {
            if !rule_file_allows_action(
                adapter_id,
                capabilities,
                &path,
                &ToolCapabilityAction::Delete,
            ) {
                return Err(format!(
                    "Rule directory contains non-rule file: {}",
                    path.display()
                ));
            }
            files.push(path);
        } else {
            return Err(format!(
                "Rule directory contains unsupported path: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

pub(crate) fn delete_rule_entry_domain(
    registry: &ToolRegistry,
    tool_id: String,
    path: String,
    dry_run: bool,
) -> Result<RuleFileChangePreview, String> {
    let (adapter_id, capabilities) = effective_rule_capabilities_for_tool(registry, &tool_id)?;
    let raw_path = expanded_raw_rule_path(&path);
    let target_path = expanded_rule_path(&path);
    if !target_path.exists() {
        return Err("Rule entry does not exist".to_string());
    }

    let mut preview = RuleFileChangePreview::default();
    if target_path.is_dir() {
        if !rule_directory_allows_action(
            &adapter_id,
            &capabilities,
            &raw_path,
            &ToolCapabilityAction::Delete,
        ) {
            return Err(
                "Rule directory target is not writable for this tool capability".to_string(),
            );
        }
        let (files, dirs) =
            collect_delete_preview_for_directory(&adapter_id, &capabilities, &target_path)?;
        preview
            .deletes
            .push(target_path.to_string_lossy().to_string());
        preview
            .changes
            .push(preview_delete(&tool_id, &target_path, "directory"));
        for file in &files {
            preview.deletes.push(file.to_string_lossy().to_string());
            preview.changes.push(preview_delete(&tool_id, file, "file"));
        }
        if !dry_run {
            for file in files {
                std::fs::remove_file(&file)
                    .map_err(|e| format!("Failed to delete {}: {}", file.display(), e))?;
            }
            for dir in dirs {
                std::fs::remove_dir(&dir)
                    .map_err(|e| format!("Failed to delete {}: {}", dir.display(), e))?;
            }
        }
        return Ok(preview);
    }

    if !rule_file_allows_action(
        &adapter_id,
        &capabilities,
        &raw_path,
        &ToolCapabilityAction::Delete,
    ) {
        return Err("Rule file target is not writable for this tool capability".to_string());
    }
    preview
        .deletes
        .push(target_path.to_string_lossy().to_string());
    preview
        .changes
        .push(preview_delete(&tool_id, &target_path, "file"));
    if !dry_run {
        std::fs::remove_file(&target_path)
            .map_err(|e| format!("Failed to delete {}: {}", target_path.display(), e))?;
    }
    Ok(preview)
}

pub(crate) fn rename_rule_entry_domain(
    registry: &ToolRegistry,
    tool_id: String,
    path: String,
    new_name: String,
) -> Result<String, String> {
    validate_plain_segment(&new_name)?;
    let (adapter_id, capabilities) = effective_rule_capabilities_for_tool(registry, &tool_id)?;
    let raw_source_path = expanded_raw_rule_path(&path);
    let source_path = expanded_rule_path(&path);
    if !source_path.exists() {
        return Err("Rule entry does not exist".to_string());
    }
    if source_path.is_file()
        && is_declared_fixed_rule_target(&adapter_id, &capabilities, &source_path)
    {
        return Err("Rule entry name is fixed for this tool capability".to_string());
    }
    let parent = source_path
        .parent()
        .ok_or_else(|| "Rule entry cannot be renamed".to_string())?;
    let target_path = normalized_path(&parent.join(new_name.trim()));
    if target_path.exists() {
        return Err("Rule rename target already exists".to_string());
    }

    if source_path.is_dir() {
        if !rule_directory_allows_action(
            &adapter_id,
            &capabilities,
            &raw_source_path,
            &ToolCapabilityAction::Delete,
        ) || !rule_directory_allows_action(
            &adapter_id,
            &capabilities,
            &target_path,
            &ToolCapabilityAction::Create,
        ) {
            return Err(
                "Rule directory target is not writable for this tool capability".to_string(),
            );
        }
    } else if !rule_file_allows_action(
        &adapter_id,
        &capabilities,
        &raw_source_path,
        &ToolCapabilityAction::Delete,
    ) || !rule_file_allows_action(
        &adapter_id,
        &capabilities,
        &target_path,
        &ToolCapabilityAction::Create,
    ) {
        return Err("Rule file target is not writable for this tool capability".to_string());
    }

    std::fs::rename(&source_path, &target_path).map_err(|e| {
        format!(
            "Failed to rename {} to {}: {}",
            source_path.display(),
            target_path.display(),
            e
        )
    })?;
    Ok(target_path.to_string_lossy().to_string())
}

pub(crate) fn copy_rule_domain(
    registry: &ToolRegistry,
    target_tool_id: String,
    target_path: String,
    content: String,
    append: bool,
) -> Result<(), String> {
    let target_path = ensure_rule_file_action_path(
        registry,
        &target_tool_id,
        &target_path,
        ToolCapabilityAction::Save,
    )?;

    if append {
        let existing = std::fs::read_to_string(&target_path).unwrap_or_default();
        let combined = format!("{}\n\n{}", existing.trim_end(), content);
        write_rule_content(registry, &target_tool_id, &target_path, &combined)
    } else {
        write_rule_content(registry, &target_tool_id, &target_path, &content)
    }
}

pub(crate) fn read_rule_content_domain(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{
        openclaw::OpenClawAdapter, RuleSource, ToolAdapter, ToolCapability, ToolCapabilityAccess,
        ToolCapabilityAction, ToolCapabilityActionEvidence, ToolCapabilityFormat,
        ToolCapabilityKind, ToolCapabilityScope, ToolCapabilitySourceConfidence,
        ToolSourceDiagnosticState,
    };
    use std::path::PathBuf;

    struct RuleTestAdapter {
        access: ToolCapabilityAccess,
        source_path: String,
        format: ToolCapabilityFormat,
        action_evidence: Vec<ToolCapabilityActionEvidence>,
    }

    impl ToolAdapter for RuleTestAdapter {
        fn id(&self) -> &str {
            "rule-test"
        }
        fn name(&self) -> &str {
            "Rule Test"
        }
        fn icon(&self) -> &str {
            "R"
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
        fn write_rule(&self, path: &str, content: &str) -> Result<(), String> {
            std::fs::write(path, content).map_err(|e| e.to_string())
        }
        fn capabilities(&self) -> Vec<ToolCapability> {
            vec![ToolCapability {
                id: "rule".to_string(),
                kind: ToolCapabilityKind::Rule,
                scope: ToolCapabilityScope::Global,
                access: self.access.clone(),
                format: self.format.clone(),
                source_path: self.source_path.clone(),
                label: "Rule".to_string(),
                diagnostics: vec![ToolSourceDiagnosticState::Loaded],
                source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
                notes: String::new(),
                source_kind: crate::adapters::ToolCapabilitySourceKind::FeatureSource,
                primary_config_dir: None,
                supporting_sources: vec![],
                action_evidence: self.action_evidence.clone(),
            }]
        }
    }

    struct MultiRuleTestAdapter {
        capabilities: Vec<ToolCapability>,
    }

    impl ToolAdapter for MultiRuleTestAdapter {
        fn id(&self) -> &str {
            "rule-test"
        }
        fn name(&self) -> &str {
            "Rule Test"
        }
        fn icon(&self) -> &str {
            "R"
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
        fn write_rule(&self, path: &str, content: &str) -> Result<(), String> {
            std::fs::write(path, content).map_err(|e| e.to_string())
        }
        fn capabilities(&self) -> Vec<ToolCapability> {
            self.capabilities.clone()
        }
    }

    fn rule_capability(
        id: &str,
        source_path: String,
        format: ToolCapabilityFormat,
    ) -> ToolCapability {
        ToolCapability {
            id: id.to_string(),
            kind: ToolCapabilityKind::Rule,
            scope: ToolCapabilityScope::Global,
            access: ToolCapabilityAccess::Writable,
            format,
            source_path,
            label: "Rule".to_string(),
            diagnostics: vec![ToolSourceDiagnosticState::Loaded],
            source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
            notes: String::new(),
            source_kind: crate::adapters::ToolCapabilitySourceKind::FeatureSource,
            primary_config_dir: None,
            supporting_sources: vec![],
            action_evidence: vec![],
        }
    }

    #[test]
    fn write_rule_rejects_read_only_capability() {
        let tmp = tempfile::tempdir().unwrap();
        let rule_path = tmp.path().join("RULES.md");
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::ReadOnly,
            source_path: rule_path.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Markdown,
            action_evidence: vec![],
        })]);

        let err = write_rule_domain(
            &registry,
            "rule-test".to_string(),
            rule_path.to_string_lossy().to_string(),
            "new".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("not writable"));
        assert!(!rule_path.exists());
    }

    #[test]
    fn write_rule_rejects_path_outside_writable_capability() {
        let tmp = tempfile::tempdir().unwrap();
        let allowed_path = tmp.path().join("RULES.md");
        let outside_path = tmp.path().join("OTHER.md");
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: allowed_path.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Markdown,
            action_evidence: vec![],
        })]);

        let err = write_rule_domain(
            &registry,
            "rule-test".to_string(),
            outside_path.to_string_lossy().to_string(),
            "new".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("not writable"));
        assert!(!outside_path.exists());
    }

    #[test]
    fn write_rule_allows_matching_writable_capability() {
        let tmp = tempfile::tempdir().unwrap();
        let rule_path = tmp.path().join("RULES.md");
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rule_path.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Markdown,
            action_evidence: vec![],
        })]);

        write_rule_domain(
            &registry,
            "rule-test".to_string(),
            rule_path.to_string_lossy().to_string(),
            "new".to_string(),
        )
        .unwrap();

        assert_eq!(std::fs::read_to_string(rule_path).unwrap(), "new");
    }

    #[test]
    fn create_rule_file_creates_parent_and_plain_content() {
        let tmp = tempfile::tempdir().unwrap();
        let rule_path = tmp.path().join("nested").join("RULES.md");
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rule_path.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Markdown,
            action_evidence: vec![],
        })]);

        create_rule_file_domain(
            &registry,
            "rule-test".to_string(),
            rule_path.to_string_lossy().to_string(),
            "".to_string(),
        )
        .unwrap();

        assert_eq!(std::fs::read_to_string(rule_path).unwrap(), "");
    }

    #[test]
    fn create_rule_file_rejects_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let rule_path = tmp.path().join("RULES.md");
        std::fs::write(&rule_path, "keep").unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rule_path.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Markdown,
            action_evidence: vec![],
        })]);

        let err = create_rule_file_domain(
            &registry,
            "rule-test".to_string(),
            rule_path.to_string_lossy().to_string(),
            "replace".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("already exists"));
        assert_eq!(std::fs::read_to_string(rule_path).unwrap(), "keep");
    }

    #[test]
    fn create_rule_file_allows_writable_directory_child() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let rule_path = rules_dir.join("CUSTOM.md");
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![],
        })]);

        create_rule_file_domain(
            &registry,
            "rule-test".to_string(),
            rule_path.to_string_lossy().to_string(),
            "plain".to_string(),
        )
        .unwrap();

        assert_eq!(std::fs::read_to_string(rule_path).unwrap(), "plain");
    }

    #[test]
    fn github_copilot_rule_directory_accepts_only_instruction_markdown_children() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join(".copilot/instructions");
        let ordinary_markdown_path = rules_dir.join("README.md");
        let instruction_path = rules_dir.join("team.instructions.md");
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![crate::adapters::github_copilot::create(
                tmp.path(),
            )]);

        let err = create_rule_file_domain(
            &registry,
            "github-copilot".to_string(),
            ordinary_markdown_path.to_string_lossy().to_string(),
            "ignored".to_string(),
        )
        .unwrap_err();
        assert!(err.contains("not writable"));
        assert!(!ordinary_markdown_path.exists());

        create_rule_file_domain(
            &registry,
            "github-copilot".to_string(),
            instruction_path.to_string_lossy().to_string(),
            "verified".to_string(),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(instruction_path).unwrap(),
            "verified"
        );
    }

    #[test]
    fn create_rule_file_rejects_outside_writable_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let outside_path = tmp.path().join("OTHER.md");
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![],
        })]);

        let err = create_rule_file_domain(
            &registry,
            "rule-test".to_string(),
            outside_path.to_string_lossy().to_string(),
            "plain".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("not writable"));
        assert!(!outside_path.exists());
    }

    #[test]
    fn create_rule_file_rejects_directory_traversal_outside_writable_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let outside_path = rules_dir.join("..").join("ESCAPE.md");
        let escaped_path = tmp.path().join("ESCAPE.md");
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![],
        })]);

        let err = create_rule_file_domain(
            &registry,
            "rule-test".to_string(),
            outside_path.to_string_lossy().to_string(),
            "plain".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("not writable"));
        assert!(!escaped_path.exists());
    }

    fn supported_action(action: ToolCapabilityAction) -> ToolCapabilityActionEvidence {
        ToolCapabilityActionEvidence {
            action,
            supported: true,
            evidence: "test".to_string(),
            variant: None,
            version: None,
            verified_at: None,
        }
    }

    #[test]
    fn create_rule_directory_allows_writable_directory_child() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let child_dir = rules_dir.join("team");
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![],
        })]);

        create_rule_directory_domain(
            &registry,
            "rule-test".to_string(),
            child_dir.to_string_lossy().to_string(),
        )
        .unwrap();

        assert!(child_dir.is_dir());
    }

    #[test]
    fn delete_only_rule_source_rejects_create_but_allows_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        std::fs::create_dir_all(&rules_dir).unwrap();
        let rule_path = rules_dir.join("old.md");
        std::fs::write(&rule_path, "old").unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![
                supported_action(ToolCapabilityAction::View),
                supported_action(ToolCapabilityAction::Delete),
            ],
        })]);

        let create_err = create_rule_file_domain(
            &registry,
            "rule-test".to_string(),
            rules_dir.join("new.md").to_string_lossy().to_string(),
            "new".to_string(),
        )
        .unwrap_err();
        assert!(create_err.contains("not writable"));

        let preview = delete_rule_entry_domain(
            &registry,
            "rule-test".to_string(),
            rule_path.to_string_lossy().to_string(),
            true,
        )
        .unwrap();
        assert_eq!(preview.deletes.len(), 1);
        assert!(rule_path.exists());

        delete_rule_entry_domain(
            &registry,
            "rule-test".to_string(),
            rule_path.to_string_lossy().to_string(),
            false,
        )
        .unwrap();
        assert!(!rule_path.exists());
    }

    #[test]
    fn delete_rule_directory_blocks_non_rule_files() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let team_dir = rules_dir.join("team");
        std::fs::create_dir_all(&team_dir).unwrap();
        std::fs::write(team_dir.join("rule.md"), "rule").unwrap();
        std::fs::write(team_dir.join("note.txt"), "note").unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![],
        })]);

        let err = delete_rule_entry_domain(
            &registry,
            "rule-test".to_string(),
            team_dir.to_string_lossy().to_string(),
            true,
        )
        .unwrap_err();

        assert!(err.contains("non-rule file"));
        assert!(team_dir.join("rule.md").exists());
        assert!(team_dir.join("note.txt").exists());
    }

    #[test]
    fn rename_rule_file_rejects_escape_and_preserves_original() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        std::fs::create_dir_all(&rules_dir).unwrap();
        let rule_path = rules_dir.join("old.md");
        std::fs::write(&rule_path, "old").unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![],
        })]);

        let err = rename_rule_entry_domain(
            &registry,
            "rule-test".to_string(),
            rule_path.to_string_lossy().to_string(),
            "../escape.md".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("Invalid"));
        assert!(rule_path.exists());
        assert!(!tmp.path().join("escape.md").exists());
    }

    #[test]
    fn rename_rule_file_moves_within_directory_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        std::fs::create_dir_all(&rules_dir).unwrap();
        let rule_path = rules_dir.join("old.md");
        let new_path = rules_dir.join("new.md");
        std::fs::write(&rule_path, "old").unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![],
        })]);

        let renamed = rename_rule_entry_domain(
            &registry,
            "rule-test".to_string(),
            rule_path.to_string_lossy().to_string(),
            "new.md".to_string(),
        )
        .unwrap();

        assert_eq!(renamed, new_path.to_string_lossy());
        assert!(!rule_path.exists());
        assert_eq!(std::fs::read_to_string(new_path).unwrap(), "old");
    }

    #[test]
    fn rename_rule_file_rejects_declared_fixed_rule_target_inside_directory_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        std::fs::create_dir_all(&rules_dir).unwrap();
        let fixed_path = rules_dir.join("AGENTS.md");
        let ordinary_path = rules_dir.join("notes.md");
        let ordinary_new_path = rules_dir.join("renamed.md");
        std::fs::write(&fixed_path, "fixed").unwrap();
        std::fs::write(&ordinary_path, "ordinary").unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(MultiRuleTestAdapter {
                capabilities: vec![
                    rule_capability(
                        "rules-directory",
                        rules_dir.to_string_lossy().to_string(),
                        ToolCapabilityFormat::Directory,
                    ),
                    rule_capability(
                        "fixed-agents",
                        fixed_path.to_string_lossy().to_string(),
                        ToolCapabilityFormat::Markdown,
                    ),
                ],
            })]);

        let err = rename_rule_entry_domain(
            &registry,
            "rule-test".to_string(),
            fixed_path.to_string_lossy().to_string(),
            "RENAMED.md".to_string(),
        )
        .unwrap_err();
        assert!(err.contains("fixed"));
        assert!(fixed_path.exists());

        let renamed = rename_rule_entry_domain(
            &registry,
            "rule-test".to_string(),
            ordinary_path.to_string_lossy().to_string(),
            "renamed.md".to_string(),
        )
        .unwrap();
        assert_eq!(renamed, ordinary_new_path.to_string_lossy());
    }

    #[test]
    fn rename_openclaw_workspace_agents_rejects_fixed_entry_but_allows_other_workspace_rules() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join(".openclaw").join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let agents_path = workspace.join("AGENTS.md");
        let notes_path = workspace.join("NOTES.md");
        let notes_new_path = workspace.join("RENAMED.md");
        std::fs::write(&agents_path, "agents").unwrap();
        std::fs::write(&notes_path, "notes").unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(OpenClawAdapter::new(
            tmp.path().to_path_buf(),
        ))]);

        let err = rename_rule_entry_domain(
            &registry,
            "openclaw".to_string(),
            agents_path.to_string_lossy().to_string(),
            "RENAMED.md".to_string(),
        )
        .unwrap_err();
        assert!(err.contains("fixed"));
        assert!(agents_path.exists());

        let renamed = rename_rule_entry_domain(
            &registry,
            "openclaw".to_string(),
            notes_path.to_string_lossy().to_string(),
            "RENAMED.md".to_string(),
        )
        .unwrap();
        assert_eq!(renamed, notes_new_path.to_string_lossy());
        assert_eq!(std::fs::read_to_string(notes_new_path).unwrap(), "notes");
    }

    #[cfg(unix)]
    #[test]
    fn create_rule_directory_rejects_symlink_parent_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let outside_dir = tmp.path().join("outside");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::create_dir_all(&outside_dir).unwrap();
        std::os::unix::fs::symlink(&outside_dir, rules_dir.join("link")).unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![],
        })]);

        let err = create_rule_directory_domain(
            &registry,
            "rule-test".to_string(),
            rules_dir.join("link/child").to_string_lossy().to_string(),
        )
        .unwrap_err();

        assert!(err.contains("not writable"));
        assert!(!outside_dir.join("child").exists());
    }

    #[cfg(unix)]
    #[test]
    fn create_rule_file_rejects_symlink_parent_component_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let outside_dir = tmp.path().join("outside");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::create_dir_all(&outside_dir).unwrap();
        std::os::unix::fs::symlink(&outside_dir, rules_dir.join("link")).unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![],
        })]);

        let err = create_rule_file_domain(
            &registry,
            "rule-test".to_string(),
            rules_dir
                .join("link/../escape.md")
                .to_string_lossy()
                .to_string(),
            "escape".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("not writable"));
        assert!(!tmp.path().join("escape.md").exists());
        assert!(!rules_dir.join("escape.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn write_rule_rejects_symlink_parent_component_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let outside_dir = tmp.path().join("outside");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::create_dir_all(&outside_dir).unwrap();
        std::os::unix::fs::symlink(&outside_dir, rules_dir.join("link")).unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![],
        })]);

        let err = write_rule_domain(
            &registry,
            "rule-test".to_string(),
            rules_dir
                .join("link/../escape.md")
                .to_string_lossy()
                .to_string(),
            "escape".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("not writable"));
        assert!(!tmp.path().join("escape.md").exists());
        assert!(!rules_dir.join("escape.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn copy_rule_rejects_symlink_parent_component_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let outside_dir = tmp.path().join("outside");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::create_dir_all(&outside_dir).unwrap();
        std::os::unix::fs::symlink(&outside_dir, rules_dir.join("link")).unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![],
        })]);

        let err = copy_rule_domain(
            &registry,
            "rule-test".to_string(),
            rules_dir
                .join("link/../escape.md")
                .to_string_lossy()
                .to_string(),
            "escape".to_string(),
            false,
        )
        .unwrap_err();

        assert!(err.contains("not writable"));
        assert!(!tmp.path().join("escape.md").exists());
        assert!(!rules_dir.join("escape.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn delete_rule_directory_rejects_symlink_target_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let outside_dir = tmp.path().join("outside");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::create_dir_all(&outside_dir).unwrap();
        std::fs::write(outside_dir.join("rule.md"), "outside").unwrap();
        std::os::unix::fs::symlink(&outside_dir, rules_dir.join("link")).unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![],
        })]);

        let err = delete_rule_entry_domain(
            &registry,
            "rule-test".to_string(),
            rules_dir.join("link").to_string_lossy().to_string(),
            true,
        )
        .unwrap_err();

        assert!(err.contains("not writable"));
        assert!(outside_dir.join("rule.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn rename_rule_file_rejects_source_through_symlink_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_dir = tmp.path().join("rules");
        let outside_dir = tmp.path().join("outside");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::create_dir_all(&outside_dir).unwrap();
        let outside_rule = outside_dir.join("rule.md");
        std::fs::write(&outside_rule, "outside").unwrap();
        std::os::unix::fs::symlink(&outside_dir, rules_dir.join("link")).unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![Box::new(RuleTestAdapter {
            access: ToolCapabilityAccess::Writable,
            source_path: rules_dir.to_string_lossy().to_string(),
            format: ToolCapabilityFormat::Directory,
            action_evidence: vec![],
        })]);

        let err = rename_rule_entry_domain(
            &registry,
            "rule-test".to_string(),
            rules_dir.join("link/rule.md").to_string_lossy().to_string(),
            "renamed.md".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("not writable"));
        assert!(outside_rule.exists());
        assert!(!outside_dir.join("renamed.md").exists());
    }
}
