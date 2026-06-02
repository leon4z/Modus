// Purpose: Own Skill install, copy, update, delete, diff, and direct file operation workflows.
use super::*;

fn relative_skill_file_path(relative_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("Invalid skill file path".to_string());
    }
    Ok(relative.to_path_buf())
}

fn normalized_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn canonical_existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut cursor = if path.exists() || path.is_symlink() {
        path.to_path_buf()
    } else {
        path.parent()?.to_path_buf()
    };
    loop {
        if cursor.exists() || cursor.is_symlink() {
            return fs::canonicalize(&cursor).ok();
        }
        if !cursor.pop() {
            return None;
        }
    }
}

fn path_stays_within_root(candidate: &Path, root: &Path) -> bool {
    let normalized_candidate = normalized_path(candidate);
    let normalized_root = normalized_path(root);
    if !normalized_candidate.starts_with(&normalized_root) {
        return false;
    }
    match (
        canonical_existing_ancestor(candidate),
        canonical_existing_ancestor(root),
    ) {
        (Some(candidate_existing), Some(root_existing)) => {
            candidate_existing.starts_with(root_existing)
        }
        _ => true,
    }
}

fn path_is_literal_child_of_root(candidate: &Path, root: &Path) -> bool {
    normalized_path(candidate).starts_with(normalized_path(root))
}

fn path_resolves_within_root(candidate: &Path, root: &Path) -> bool {
    match (fs::canonicalize(candidate), fs::canonicalize(root)) {
        (Ok(candidate), Ok(root)) => candidate.starts_with(root),
        _ => false,
    }
}

fn path_is_or_resolves_within_root(candidate: &Path, root: &Path) -> bool {
    path_is_literal_child_of_root(candidate, root) || path_resolves_within_root(candidate, root)
}

fn find_skill_dir_in_root(root: &Path, skill_name: &str) -> Option<PathBuf> {
    find_skill_dirs_in_root(root, skill_name).into_iter().next()
}

fn detected_tool_for_skill_action(
    registry: &ToolRegistry,
    tool_id: &str,
) -> Result<DetectedTool, String> {
    let canonical = canonical_skill_tool_id(tool_id);
    registry
        .detect_all()
        .into_iter()
        .find(|tool| tool.id == canonical && tool.detected)
        .ok_or_else(|| format!("Tool not found: {}", tool_id))
}

fn detected_tool_for_skill_action_with_config(
    registry: &ToolRegistry,
    tool_id: &str,
    config: &crate::platform::config::AppConfig,
) -> Result<DetectedTool, String> {
    let canonical = canonical_skill_tool_id(tool_id);
    registry
        .detect_all_for_config(config)
        .into_iter()
        .find(|tool| tool.id == canonical && tool.detected)
        .ok_or_else(|| format!("Tool not found: {}", tool_id))
}

fn find_skill_dirs_in_root(root: &Path, skill_name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_skill_dirs_in_root_recursive(root, skill_name, &mut paths);
    paths.sort();
    paths.dedup();
    paths
}

fn collect_skill_dirs_in_root_recursive(dir: &Path, skill_name: &str, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let is_symlink = entry
            .file_type()
            .map(|file_type| file_type.is_symlink())
            .unwrap_or(false);
        let is_broken_symlink = is_symlink && !path.exists();
        if name == skill_name && (is_valid_skill_dir(&path) || is_broken_symlink) {
            paths.push(path);
            continue;
        }
        if !is_symlink && path.is_dir() && !is_valid_skill_dir(&path) {
            collect_skill_dirs_in_root_recursive(&path, skill_name, paths);
        }
    }
}

fn resolved_skill_path_in_root(root: &Path, skill_name: &str) -> PathBuf {
    find_skill_dir_in_root(root, skill_name).unwrap_or_else(|| root.join(skill_name))
}

fn source_path_matches_skill(path: &Path, skill_name: &str) -> bool {
    path.file_name().and_then(|value| value.to_str()) == Some(skill_name)
        && (is_valid_skill_dir(path)
            || fs::symlink_metadata(path)
                .map(|meta| meta.file_type().is_symlink() && !path.exists())
                .unwrap_or(false))
}

fn select_skill_source_path(
    root: &Path,
    skill_name: &str,
    source_path: Option<&str>,
) -> Result<PathBuf, String> {
    let candidates = find_skill_dirs_in_root(root, skill_name);
    if let Some(source_path) = source_path {
        let source = normalized_path(Path::new(source_path));
        let known = candidates
            .iter()
            .any(|candidate| normalized_path(candidate) == source);
        if !known
            || !path_stays_within_root(&source, root)
            || !source_path_matches_skill(&source, skill_name)
        {
            return Err("source_path is not a matching local Skill source".to_string());
        }
        return Ok(source);
    }
    if candidates.len() > 1 {
        return Err("duplicate_skill_source_path_required".to_string());
    }
    Ok(candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| root.join(skill_name)))
}

fn preview_entry_kind_for_existing_path(path: &Path) -> &'static str {
    if fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
    {
        "symlink"
    } else {
        "file"
    }
}

#[cfg(unix)]
fn create_skill_symlink(source: &Path, target: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(source, target).map_err(|e| e.to_string())
}

#[cfg(windows)]
fn create_skill_symlink(source: &Path, target: &Path) -> Result<(), String> {
    std::os::windows::fs::symlink_dir(source, target).map_err(|e| e.to_string())
}

fn writable_skill_roots(registry: &ToolRegistry) -> Vec<PathBuf> {
    let mut roots = vec![crate::platform::env::generic_skills_dir()];
    roots.extend(registry.detect_all().into_iter().filter_map(|tool| {
        resolved_detected_tool_skills_dir_for_action(&tool, &ToolCapabilityAction::Save)
            .or_else(|| {
                resolved_detected_tool_skills_dir_for_action(&tool, &ToolCapabilityAction::Edit)
            })
            .or_else(|| {
                resolved_detected_tool_skills_dir_for_action(&tool, &ToolCapabilityAction::Install)
            })
    }));
    roots
}

fn can_write_skill_path(registry: &ToolRegistry, skill_root: &Path, full_path: &Path) -> bool {
    writable_skill_roots(registry).into_iter().any(|root| {
        let root = normalized_path(&root);
        let root_ok =
            path_stays_within_root(skill_root, &root) && normalized_path(skill_root) != root;
        root_ok && path_stays_within_root(full_path, &root)
    })
}

pub(crate) fn write_skill_file_domain(
    registry: &ToolRegistry,
    skill_path: String,
    relative_path: String,
    content: String,
) -> Result<(), String> {
    let skill_root = normalized_path(&std::path::PathBuf::from(&skill_path));
    let full_path = skill_root.join(relative_skill_file_path(&relative_path)?);
    if !can_write_skill_path(registry, &skill_root, &full_path) {
        return Err("Skill path is not writable for current tool capabilities".to_string());
    }
    if full_path.is_dir() {
        return Err("Cannot write to a directory".to_string());
    }
    // Ensure parent directory exists
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    std::fs::write(&full_path, &content).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod file_write_tests {
    use super::*;
    use crate::adapters::{
        RuleSource, ToolCapability, ToolCapabilityAccess, ToolCapabilityActionEvidence,
        ToolCapabilityFormat, ToolCapabilityKind, ToolCapabilityScope,
        ToolCapabilitySourceConfidence, ToolCapabilitySupportingSource,
        ToolCapabilitySupportingSourceRole, ToolSourceDiagnosticState,
    };

    struct SkillWriteTestAdapter {
        id: String,
        access: ToolCapabilityAccess,
        dir: PathBuf,
        metadata_path: Option<PathBuf>,
        action_evidence: Vec<ToolCapabilityActionEvidence>,
        shared_read: bool,
    }

    impl ToolAdapter for SkillWriteTestAdapter {
        fn id(&self) -> &str {
            &self.id
        }
        fn name(&self) -> &str {
            "Skill Write Test"
        }
        fn icon(&self) -> &str {
            "S"
        }
        fn config_dir(&self) -> PathBuf {
            self.dir.clone()
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
            Some(self.dir.clone())
        }
        fn capabilities(&self) -> Vec<ToolCapability> {
            let mut capabilities = vec![ToolCapability {
                id: "skills".to_string(),
                kind: ToolCapabilityKind::Skill,
                scope: ToolCapabilityScope::Tool,
                access: self.access.clone(),
                format: ToolCapabilityFormat::SkillDirectory,
                source_path: self.dir.to_string_lossy().to_string(),
                label: "Skills".to_string(),
                diagnostics: vec![ToolSourceDiagnosticState::Loaded],
                source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
                notes: String::new(),
                source_kind: crate::adapters::ToolCapabilitySourceKind::FeatureSource,
                primary_config_dir: None,
                supporting_sources: self
                    .metadata_path
                    .as_ref()
                    .map(|path| {
                        vec![ToolCapabilitySupportingSource {
                            id: "skill-metadata".to_string(),
                            role: ToolCapabilitySupportingSourceRole::Metadata,
                            source_path: path.to_string_lossy().to_string(),
                            format: ToolCapabilityFormat::Json,
                            required: true,
                            diagnostics: vec![ToolSourceDiagnosticState::Loaded],
                            notes: "test metadata".to_string(),
                        }]
                    })
                    .unwrap_or_default(),
                action_evidence: self.action_evidence.clone(),
            }];
            if self.shared_read {
                capabilities.push(ToolCapability {
                    id: "shared-skills".to_string(),
                    kind: ToolCapabilityKind::Skill,
                    scope: ToolCapabilityScope::Shared,
                    access: ToolCapabilityAccess::ReadOnly,
                    format: ToolCapabilityFormat::SkillDirectory,
                    source_path: crate::platform::env::generic_skills_dir()
                        .to_string_lossy()
                        .to_string(),
                    label: "Shared Skills".to_string(),
                    diagnostics: vec![ToolSourceDiagnosticState::Loaded],
                    source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
                    notes: String::new(),
                    source_kind: crate::adapters::ToolCapabilitySourceKind::FeatureSource,
                    primary_config_dir: None,
                    supporting_sources: vec![],
                    action_evidence: vec![ToolCapabilityActionEvidence {
                        action: ToolCapabilityAction::View,
                        supported: true,
                        evidence: "certified shared direct read".to_string(),
                        variant: None,
                        version: None,
                        verified_at: Some("2026-05-17".to_string()),
                    }],
                });
            }
            capabilities
        }
    }

    fn registry_for(access: ToolCapabilityAccess, dir: PathBuf) -> ToolRegistry {
        registry_for_with_metadata(access, dir, None)
    }

    fn registry_for_with_metadata(
        access: ToolCapabilityAccess,
        dir: PathBuf,
        metadata_path: Option<PathBuf>,
    ) -> ToolRegistry {
        registry_for_with_metadata_and_actions(access, dir, metadata_path, &[])
    }

    fn registry_for_with_metadata_and_actions(
        access: ToolCapabilityAccess,
        dir: PathBuf,
        metadata_path: Option<PathBuf>,
        actions: &[ToolCapabilityAction],
    ) -> ToolRegistry {
        ToolRegistry::from_adapters_for_tests(vec![Box::new(SkillWriteTestAdapter {
            id: "skill-write-test".to_string(),
            access,
            dir,
            metadata_path,
            action_evidence: action_evidence_for(actions),
            shared_read: false,
        })])
    }

    fn registry_for_with_actions(
        access: ToolCapabilityAccess,
        dir: PathBuf,
        actions: &[ToolCapabilityAction],
    ) -> ToolRegistry {
        ToolRegistry::from_adapters_for_tests(vec![Box::new(SkillWriteTestAdapter {
            id: "skill-write-test".to_string(),
            access,
            dir,
            metadata_path: None,
            action_evidence: action_evidence_for(actions),
            shared_read: false,
        })])
    }

    fn action_evidence_for(actions: &[ToolCapabilityAction]) -> Vec<ToolCapabilityActionEvidence> {
        actions
            .iter()
            .cloned()
            .map(|action| ToolCapabilityActionEvidence {
                action,
                supported: true,
                evidence: "verified".to_string(),
                variant: Some("Trae CN".to_string()),
                version: None,
                verified_at: Some("2026-05-16".to_string()),
            })
            .collect()
    }

    fn registry_for_two_tools_with_actions(
        target_dir: PathBuf,
        target_actions: &[ToolCapabilityAction],
        source_dir: PathBuf,
        source_actions: &[ToolCapabilityAction],
    ) -> ToolRegistry {
        ToolRegistry::from_adapters_for_tests(vec![
            Box::new(SkillWriteTestAdapter {
                id: "skill-write-test".to_string(),
                access: ToolCapabilityAccess::Writable,
                dir: target_dir,
                metadata_path: None,
                action_evidence: action_evidence_for(target_actions),
                shared_read: false,
            }),
            Box::new(SkillWriteTestAdapter {
                id: "source-skill-tool".to_string(),
                access: ToolCapabilityAccess::Writable,
                dir: source_dir,
                metadata_path: None,
                action_evidence: action_evidence_for(source_actions),
                shared_read: false,
            }),
        ])
    }

    fn registry_for_direct_shared_reader(
        access: ToolCapabilityAccess,
        dir: PathBuf,
    ) -> ToolRegistry {
        ToolRegistry::from_adapters_for_tests(vec![Box::new(SkillWriteTestAdapter {
            id: "skill-write-test".to_string(),
            access,
            dir,
            metadata_path: None,
            action_evidence: vec![],
            shared_read: true,
        })])
    }

    #[test]
    fn write_skill_file_rejects_read_only_skill_root() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let registry = registry_for(ToolCapabilityAccess::ReadOnly, skills_dir.clone());
        let skill_root = skills_dir.join("demo");

        let err = write_skill_file_domain(
            &registry,
            skill_root.to_string_lossy().to_string(),
            "SKILL.md".to_string(),
            "# demo".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("not writable"));
        assert!(!skill_root.join("SKILL.md").exists());
    }

    #[test]
    fn write_skill_file_allows_writable_skill_root() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let registry = registry_for(ToolCapabilityAccess::Writable, skills_dir.clone());
        let skill_root = skills_dir.join("demo");

        write_skill_file_domain(
            &registry,
            skill_root.to_string_lossy().to_string(),
            "SKILL.md".to_string(),
            "# demo".to_string(),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(skill_root.join("SKILL.md")).unwrap(),
            "# demo"
        );
    }

    #[test]
    fn write_skill_file_allows_save_action_and_shared_install_deployment() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let generic_dir = tmp.path().join("shared");
        let shared_source = generic_dir.join("demo");
        std::fs::create_dir_all(&shared_source).unwrap();
        std::fs::write(shared_source.join("SKILL.md"), "# shared").unwrap();
        let registry = registry_for_with_actions(
            ToolCapabilityAccess::Writable,
            skills_dir.clone(),
            &[
                ToolCapabilityAction::View,
                ToolCapabilityAction::Save,
                ToolCapabilityAction::Copy,
                ToolCapabilityAction::Delete,
            ],
        );
        let skill_root = skills_dir.join("demo");

        write_skill_file_domain(
            &registry,
            skill_root.to_string_lossy().to_string(),
            "SKILL.md".to_string(),
            "# demo".to_string(),
        )
        .unwrap();
        let preview = install_skill_with_registry_and_generic_dir(
            &registry,
            &generic_dir,
            "demo",
            "skill-write-test",
            "symlink",
            Some(shared_source.as_path()),
            true,
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(skill_root.join("SKILL.md")).unwrap(),
            "# demo"
        );
        assert_eq!(preview.changes[0].entry_kind.as_deref(), Some("file"));
        assert_eq!(preview.changes[1].entry_kind.as_deref(), Some("symlink"));
    }

    #[test]
    fn write_skill_file_rejects_parent_path_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let registry = registry_for(ToolCapabilityAccess::Writable, skills_dir.clone());

        let err = write_skill_file_domain(
            &registry,
            skills_dir.join("demo").to_string_lossy().to_string(),
            "../outside.md".to_string(),
            "escape".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("Invalid skill file path"));
    }

    #[test]
    fn write_skill_file_rejects_skill_root_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let registry = registry_for(ToolCapabilityAccess::Writable, skills_dir.clone());

        let err = write_skill_file_domain(
            &registry,
            skills_dir
                .join("demo")
                .join("..")
                .join("..")
                .join("outside")
                .to_string_lossy()
                .to_string(),
            "SKILL.md".to_string(),
            "# outside".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("not writable"));
        assert!(!tmp.path().join("outside").join("SKILL.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn write_skill_file_rejects_symlink_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, skills_dir.join("linked")).unwrap();
        let registry = registry_for(ToolCapabilityAccess::Writable, skills_dir.clone());

        let err = write_skill_file_domain(
            &registry,
            skills_dir.join("linked").to_string_lossy().to_string(),
            "SKILL.md".to_string(),
            "# outside".to_string(),
        )
        .unwrap_err();

        assert!(err.contains("not writable"));
        assert!(!outside.join("SKILL.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn install_skill_creates_tool_directory_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let generic_dir = tmp.path().join("shared");
        let source = generic_dir.join("demo");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "# shared").unwrap();
        let registry = registry_for_with_actions(
            ToolCapabilityAccess::Writable,
            skills_dir.clone(),
            &[
                ToolCapabilityAction::View,
                ToolCapabilityAction::Install,
                ToolCapabilityAction::Link,
                ToolCapabilityAction::Uninstall,
            ],
        );

        let preview = install_skill_with_registry_and_generic_dir(
            &registry,
            &generic_dir,
            "demo",
            "skill-write-test",
            "symlink",
            Some(source.as_path()),
            false,
        )
        .unwrap();

        let target = skills_dir.join("demo");
        assert_eq!(preview.changes[0].entry_kind.as_deref(), Some("symlink"));
        assert!(std::fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read_link(&target).unwrap(), source);
    }

    #[test]
    fn install_skill_rejects_tool_directory_source() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("target-skills");
        let generic_dir = tmp.path().join("shared");
        let source = tmp.path().join("source-tool").join("demo");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "# tool").unwrap();
        let registry = registry_for_with_actions(
            ToolCapabilityAccess::Writable,
            skills_dir,
            &[ToolCapabilityAction::Install, ToolCapabilityAction::Link],
        );

        let err = install_skill_with_registry_and_generic_dir(
            &registry,
            &generic_dir,
            "demo",
            "skill-write-test",
            "symlink",
            Some(source.as_path()),
            true,
        )
        .unwrap_err();

        assert!(err.contains("shared-directory source"));
    }

    #[cfg(unix)]
    #[test]
    fn copy_skill_rejects_tool_symlink_to_shared_source() {
        let tmp = tempfile::tempdir().unwrap();
        let target_skills_dir = tmp.path().join("target-skills");
        let generic_dir = tmp.path().join("shared");
        let shared_source = generic_dir.join("demo");
        let source_tool_dir = tmp.path().join("source-tool");
        let source_link = source_tool_dir.join("demo");
        std::fs::create_dir_all(&shared_source).unwrap();
        std::fs::write(shared_source.join("SKILL.md"), "# shared").unwrap();
        std::fs::create_dir_all(&source_tool_dir).unwrap();
        std::os::unix::fs::symlink(&shared_source, &source_link).unwrap();
        let registry = registry_for_two_tools_with_actions(
            target_skills_dir,
            &[ToolCapabilityAction::View, ToolCapabilityAction::Copy],
            source_tool_dir,
            &[ToolCapabilityAction::View],
        );

        let err = copy_skill_to_tool_with_generic_dir(
            &registry,
            &generic_dir,
            "demo".to_string(),
            "skill-write-test".to_string(),
            source_link.to_string_lossy().to_string(),
            true,
        )
        .unwrap_err();

        assert!(err.contains("shared-directory sources must use install"));
    }

    #[cfg(unix)]
    #[test]
    fn uninstall_removes_symlink_but_delete_rejects_it() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let source = tmp.path().join("shared").join("demo");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "# shared").unwrap();
        std::fs::create_dir_all(&skills_dir).unwrap();
        let target = skills_dir.join("demo");
        std::os::unix::fs::symlink(&source, &target).unwrap();
        let registry = registry_for_with_actions(
            ToolCapabilityAccess::Writable,
            skills_dir.clone(),
            &[
                ToolCapabilityAction::View,
                ToolCapabilityAction::Install,
                ToolCapabilityAction::Link,
                ToolCapabilityAction::Uninstall,
                ToolCapabilityAction::Delete,
            ],
        );

        let err = delete_skill_from_tool_v2_domain(
            &registry,
            "demo".to_string(),
            "skill-write-test".to_string(),
            None,
            true,
        )
        .unwrap_err();
        assert!(err.contains("use uninstall"));

        uninstall_skill_v2_domain(
            &registry,
            "demo".to_string(),
            "skill-write-test".to_string(),
            false,
        )
        .unwrap();

        assert!(!target.exists());
        assert!(!target.is_symlink());
        assert!(source.join("SKILL.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn link_shared_skill_finds_nested_shared_source() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let generic_dir = tmp.path().join("shared");
        let source = generic_dir.join("devops").join("webhook-subscriptions");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "# nested shared").unwrap();
        let registry = registry_for_with_actions(
            ToolCapabilityAccess::Writable,
            skills_dir.clone(),
            &[ToolCapabilityAction::Install, ToolCapabilityAction::Link],
        );

        link_shared_skill_to_tool_with_paths(
            &registry,
            &generic_dir,
            "webhook-subscriptions".to_string(),
            "skill-write-test".to_string(),
            false,
        )
        .unwrap();

        let target = skills_dir.join("webhook-subscriptions");
        assert!(std::fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read_link(&target).unwrap(), source);
    }

    #[test]
    fn delete_skill_from_tool_finds_nested_tool_source() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let nested_source = skills_dir.join("devops").join("webhook-subscriptions");
        std::fs::create_dir_all(&nested_source).unwrap();
        std::fs::write(nested_source.join("SKILL.md"), "# nested tool").unwrap();
        let registry = registry_for(ToolCapabilityAccess::Writable, skills_dir);

        delete_skill_from_tool_v2_domain(
            &registry,
            "webhook-subscriptions".to_string(),
            "skill-write-test".to_string(),
            None,
            false,
        )
        .unwrap();

        assert!(!nested_source.exists());
    }

    #[test]
    fn delete_skill_from_tool_requires_source_path_for_duplicate_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let top_level = skills_dir.join("executing-plans");
        let nested_source = skills_dir
            .join("superpowers")
            .join("skills")
            .join("executing-plans");
        std::fs::create_dir_all(&top_level).unwrap();
        std::fs::create_dir_all(&nested_source).unwrap();
        std::fs::write(top_level.join("SKILL.md"), "# top").unwrap();
        std::fs::write(nested_source.join("SKILL.md"), "# nested").unwrap();
        let registry = registry_for(ToolCapabilityAccess::Writable, skills_dir);

        let preview = delete_skill_from_tool_v2_domain(
            &registry,
            "executing-plans".to_string(),
            "skill-write-test".to_string(),
            None,
            true,
        )
        .unwrap();

        assert_eq!(preview.blocked.len(), 1);
        assert_eq!(
            preview.blocked[0].reason.raw.as_deref(),
            Some("duplicate_skill_source_path_required")
        );
        assert!(top_level.join("SKILL.md").exists());
        assert!(nested_source.join("SKILL.md").exists());
    }

    #[test]
    fn delete_skill_from_tool_deletes_selected_duplicate_source_only() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let top_level = skills_dir.join("executing-plans");
        let nested_source = skills_dir
            .join("superpowers")
            .join("skills")
            .join("executing-plans");
        std::fs::create_dir_all(&top_level).unwrap();
        std::fs::create_dir_all(&nested_source).unwrap();
        std::fs::write(top_level.join("SKILL.md"), "# top").unwrap();
        std::fs::write(nested_source.join("SKILL.md"), "# nested").unwrap();
        let registry = registry_for(ToolCapabilityAccess::Writable, skills_dir);

        delete_skill_from_tool_v2_domain(
            &registry,
            "executing-plans".to_string(),
            "skill-write-test".to_string(),
            Some(nested_source.to_string_lossy().to_string()),
            false,
        )
        .unwrap();

        assert!(top_level.join("SKILL.md").exists());
        assert!(!nested_source.exists());
    }

    #[test]
    fn cleanup_duplicate_skill_sources_keeps_selected_source() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let keep_source = skills_dir.join("executing-plans");
        let delete_source = skills_dir
            .join("superpowers")
            .join("skills")
            .join("executing-plans");
        std::fs::create_dir_all(&keep_source).unwrap();
        std::fs::create_dir_all(&delete_source).unwrap();
        std::fs::write(keep_source.join("SKILL.md"), "# keep").unwrap();
        std::fs::write(delete_source.join("SKILL.md"), "# delete").unwrap();
        let registry = registry_for(ToolCapabilityAccess::Writable, skills_dir);

        let preview = cleanup_duplicate_skill_sources_domain(
            &registry,
            "executing-plans".to_string(),
            keep_source.to_string_lossy().to_string(),
            vec![delete_source.to_string_lossy().to_string()],
            false,
        )
        .unwrap();

        assert_eq!(preview.preserves.len(), 1);
        assert!(keep_source.join("SKILL.md").exists());
        assert!(!delete_source.exists());
    }

    #[test]
    fn cleanup_duplicate_skill_sources_rejects_read_only_delete_source() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let keep_source = skills_dir.join("executing-plans");
        let delete_source = skills_dir
            .join("superpowers")
            .join("skills")
            .join("executing-plans");
        std::fs::create_dir_all(&keep_source).unwrap();
        std::fs::create_dir_all(&delete_source).unwrap();
        std::fs::write(keep_source.join("SKILL.md"), "# keep").unwrap();
        std::fs::write(delete_source.join("SKILL.md"), "# delete").unwrap();
        let registry = registry_for(ToolCapabilityAccess::ReadOnly, skills_dir);

        let err = cleanup_duplicate_skill_sources_domain(
            &registry,
            "executing-plans".to_string(),
            keep_source.to_string_lossy().to_string(),
            vec![delete_source.to_string_lossy().to_string()],
            true,
        )
        .unwrap_err();

        assert!(err.contains("not deletable"));
        assert!(keep_source.join("SKILL.md").exists());
        assert!(delete_source.join("SKILL.md").exists());
    }

    #[test]
    fn cleanup_duplicate_skill_sources_rejects_cross_tool_source_groups() {
        let tmp = tempfile::tempdir().unwrap();
        let target_skills_dir = tmp.path().join("target-skills");
        let source_skills_dir = tmp.path().join("source-skills");
        let keep_source = target_skills_dir.join("demo");
        let delete_source = source_skills_dir.join("demo");
        std::fs::create_dir_all(&keep_source).unwrap();
        std::fs::create_dir_all(&delete_source).unwrap();
        std::fs::write(keep_source.join("SKILL.md"), "# target").unwrap();
        std::fs::write(delete_source.join("SKILL.md"), "# source").unwrap();
        let registry = registry_for_two_tools_with_actions(
            target_skills_dir,
            &[ToolCapabilityAction::View, ToolCapabilityAction::Delete],
            source_skills_dir,
            &[ToolCapabilityAction::View, ToolCapabilityAction::Delete],
        );

        let err = cleanup_duplicate_skill_sources_with_generic_dir(
            &registry,
            &tmp.path().join("shared"),
            "demo".to_string(),
            keep_source.to_string_lossy().to_string(),
            vec![delete_source.to_string_lossy().to_string()],
            true,
        )
        .unwrap_err();

        assert!(err.contains("multiple tool source groups"));
        assert!(keep_source.join("SKILL.md").exists());
        assert!(delete_source.join("SKILL.md").exists());
    }

    #[test]
    fn rename_skill_source_targets_selected_source_path() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let source = skills_dir.join("executing-plans");
        let target = skills_dir.join("executing-plans-local");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "# source").unwrap();
        let registry = registry_for(ToolCapabilityAccess::Writable, skills_dir);

        let preview = rename_skill_source_domain(
            &registry,
            "executing-plans".to_string(),
            source.to_string_lossy().to_string(),
            "executing-plans-local".to_string(),
            false,
        )
        .unwrap();

        assert_eq!(preview.creates, vec![target.to_string_lossy().to_string()]);
        assert_eq!(preview.deletes, vec![source.to_string_lossy().to_string()]);
        assert!(!source.exists());
        assert_eq!(
            std::fs::read_to_string(target.join("SKILL.md")).unwrap(),
            "# source"
        );
    }

    #[test]
    fn cleanup_duplicate_skill_sources_warns_when_deleting_shared_source() {
        let tmp = tempfile::tempdir().unwrap();
        let generic_dir = tmp.path().join("shared");
        let skills_dir = tmp.path().join("skills");
        let shared_source = generic_dir.join("demo");
        let tool_source = skills_dir.join("demo");
        std::fs::create_dir_all(&shared_source).unwrap();
        std::fs::create_dir_all(&tool_source).unwrap();
        std::fs::write(shared_source.join("SKILL.md"), "# shared").unwrap();
        std::fs::write(tool_source.join("SKILL.md"), "# tool").unwrap();
        let registry = registry_for(ToolCapabilityAccess::Writable, skills_dir);

        let preview = cleanup_duplicate_skill_sources_with_generic_dir(
            &registry,
            &generic_dir,
            "demo".to_string(),
            tool_source.to_string_lossy().to_string(),
            vec![shared_source.to_string_lossy().to_string()],
            true,
        )
        .unwrap();

        assert!(preview
            .message
            .as_deref()
            .unwrap_or("")
            .contains("共享目录"));
        assert!(preview
            .deletes
            .contains(&shared_source.to_string_lossy().to_string()));
    }

    #[test]
    fn shared_install_rejects_direct_shared_reader() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let generic_dir = tmp.path().join("shared");
        let source = generic_dir.join("demo");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "# shared").unwrap();
        let registry =
            registry_for_direct_shared_reader(ToolCapabilityAccess::Writable, skills_dir);

        let err = install_skill_with_registry_and_generic_dir(
            &registry,
            &generic_dir,
            "demo",
            "skill-write-test",
            "symlink",
            Some(source.as_path()),
            true,
        )
        .unwrap_err();

        assert!(err.contains("directly reads shared Skills"));
    }

    #[test]
    fn trae_cn_default_shared_read_rejects_redundant_tool_directory_install() {
        let tmp = tempfile::tempdir().unwrap();
        let generic_dir = tmp.path().join("shared");
        let source = generic_dir.join("demo");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "# shared").unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![crate::adapters::trae_cn::create(
                tmp.path(),
            )]);

        let err = link_shared_skill_to_tool_with_paths(
            &registry,
            &generic_dir,
            "demo".to_string(),
            "trae-cn".to_string(),
            true,
        )
        .unwrap_err();

        assert!(err.contains("directly reads shared Skills"));
    }

    #[test]
    fn windsurf_shared_install_is_disabled_and_tool_copy_targets_global_skill_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let windsurf_root = home.join(".codeium/windsurf");
        let windsurf_skills = windsurf_root.join("skills");
        let workspace_skills = tmp.path().join("workspace/.windsurf/skills");
        let generic_dir = tmp.path().join("shared");
        let shared_source = generic_dir.join("demo");
        let source_skills_dir = tmp.path().join("source-tool");
        let source = source_skills_dir.join("demo");
        for skill in [&shared_source, &source] {
            std::fs::create_dir_all(skill).unwrap();
            std::fs::write(skill.join("SKILL.md"), "# source").unwrap();
        }
        std::fs::create_dir_all(&windsurf_root).unwrap();
        std::fs::create_dir_all(&workspace_skills).unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![
            crate::adapters::windsurf::create(&home),
            Box::new(SkillWriteTestAdapter {
                id: "source-skill-tool".to_string(),
                access: ToolCapabilityAccess::Writable,
                dir: source_skills_dir,
                metadata_path: None,
                action_evidence: action_evidence_for(&[ToolCapabilityAction::View]),
                shared_read: false,
            }),
        ]);

        let install_err = install_skill_with_registry_and_generic_dir(
            &registry,
            &generic_dir,
            "demo",
            "windsurf",
            "symlink",
            Some(shared_source.as_path()),
            true,
        )
        .unwrap_err();
        assert!(install_err.contains("directly reads shared Skills"));

        let app_config = crate::platform::config::default_config();
        copy_skill_to_tool_for_config_with_generic_dir(
            &registry,
            &generic_dir,
            "demo".to_string(),
            "windsurf".to_string(),
            source.to_string_lossy().to_string(),
            false,
            &app_config,
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(windsurf_skills.join("demo").join("SKILL.md")).unwrap(),
            "# source"
        );
        assert!(!workspace_skills.join("demo").exists());
    }

    #[cfg(unix)]
    #[test]
    fn kiro_shared_install_links_into_global_skill_dir_and_tool_copy_still_uses_it() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let kiro_skills = home.join(".kiro/skills");
        let workspace_skills = tmp.path().join("workspace/.kiro/skills");
        let generic_dir = tmp.path().join("shared");
        let shared_source = generic_dir.join("demo");
        let source_skills_dir = tmp.path().join("source-tool");
        let source = source_skills_dir.join("tool-demo");
        for skill in [&shared_source, &source] {
            std::fs::create_dir_all(skill).unwrap();
            std::fs::write(skill.join("SKILL.md"), "# source").unwrap();
        }
        std::fs::create_dir_all(&workspace_skills).unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![
            crate::adapters::kiro::create(&home),
            Box::new(SkillWriteTestAdapter {
                id: "source-skill-tool".to_string(),
                access: ToolCapabilityAccess::Writable,
                dir: source_skills_dir,
                metadata_path: None,
                action_evidence: action_evidence_for(&[ToolCapabilityAction::View]),
                shared_read: false,
            }),
        ]);

        install_skill_with_registry_and_generic_dir(
            &registry,
            &generic_dir,
            "demo",
            "kiro",
            "symlink",
            Some(shared_source.as_path()),
            false,
        )
        .unwrap();

        let linked_target = kiro_skills.join("demo");
        assert!(std::fs::symlink_metadata(&linked_target)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read_link(&linked_target).unwrap(), shared_source);

        copy_skill_to_tool_with_generic_dir(
            &registry,
            &generic_dir,
            "tool-demo".to_string(),
            "kiro".to_string(),
            source.to_string_lossy().to_string(),
            false,
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(kiro_skills.join("tool-demo").join("SKILL.md")).unwrap(),
            "# source"
        );
        assert!(!workspace_skills.join("demo").exists());
        assert!(!workspace_skills.join("tool-demo").exists());
    }

    #[test]
    fn shared_install_uses_writable_tool_directory_without_link_evidence() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let generic_dir = tmp.path().join("shared");
        let source = generic_dir.join("demo");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "# shared").unwrap();
        let registry = registry_for(ToolCapabilityAccess::Writable, skills_dir);

        let preview = install_skill_with_registry_and_generic_dir(
            &registry,
            &generic_dir,
            "demo",
            "skill-write-test",
            "symlink",
            Some(source.as_path()),
            true,
        )
        .unwrap();

        assert_eq!(preview.changes[0].entry_kind.as_deref(), Some("symlink"));
    }

    #[test]
    fn copy_delete_and_shared_install_can_target_tool_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let target_skills_dir = tmp.path().join("target-skills");
        let source_skills_dir = tmp.path().join("source-tool");
        let source = source_skills_dir.join("demo");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "# source").unwrap();
        let registry = registry_for_two_tools_with_actions(
            target_skills_dir.clone(),
            &[
                ToolCapabilityAction::View,
                ToolCapabilityAction::Copy,
                ToolCapabilityAction::Delete,
                ToolCapabilityAction::Save,
            ],
            source_skills_dir,
            &[ToolCapabilityAction::View],
        );

        copy_skill_to_tool_with_generic_dir(
            &registry,
            &tmp.path().join("shared"),
            "demo".to_string(),
            "skill-write-test".to_string(),
            source.to_string_lossy().to_string(),
            false,
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(target_skills_dir.join("demo").join("SKILL.md")).unwrap(),
            "# source"
        );
        let shared = tmp.path().join("shared").join("demo");
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::write(shared.join("SKILL.md"), "# shared").unwrap();
        let preview = link_shared_skill_to_tool_with_paths(
            &registry,
            &tmp.path().join("shared"),
            "demo".to_string(),
            "skill-write-test".to_string(),
            true,
        )
        .unwrap();
        assert!(preview
            .creates
            .contains(&target_skills_dir.join("demo").to_string_lossy().to_string()));

        delete_skill_from_tool_v2_domain(
            &registry,
            "demo".to_string(),
            "skill-write-test".to_string(),
            None,
            false,
        )
        .unwrap();

        assert!(!target_skills_dir.join("demo").exists());
    }

    #[test]
    fn copy_skill_rejects_unknown_tool_source_path() {
        let tmp = tempfile::tempdir().unwrap();
        let target_skills_dir = tmp.path().join("target-skills");
        let source = tmp.path().join("unregistered-source").join("demo");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "# source").unwrap();
        let registry = registry_for_with_actions(
            ToolCapabilityAccess::Writable,
            target_skills_dir,
            &[ToolCapabilityAction::View, ToolCapabilityAction::Copy],
        );

        let err = copy_skill_to_tool_with_generic_dir(
            &registry,
            &tmp.path().join("shared"),
            "demo".to_string(),
            "skill-write-test".to_string(),
            source.to_string_lossy().to_string(),
            true,
        )
        .unwrap_err();

        assert!(err.contains("known tool Skill source"));
    }

    #[test]
    fn delete_skill_from_tool_rejects_metadata_drift() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let metadata_path = tmp.path().join("skill-config.json");
        let source = skills_dir.join("content-only");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("SKILL.md"), "# content").unwrap();
        std::fs::write(&metadata_path, r#"{"managedSkills":{}}"#).unwrap();
        let registry = registry_for_with_metadata_and_actions(
            ToolCapabilityAccess::Writable,
            skills_dir,
            Some(metadata_path),
            &[ToolCapabilityAction::View, ToolCapabilityAction::Delete],
        );

        let err = delete_skill_from_tool_v2_domain(
            &registry,
            "content-only".to_string(),
            "skill-write-test".to_string(),
            None,
            false,
        )
        .unwrap_err();

        assert!(err.contains("元数据不一致"));
        assert!(source.join("SKILL.md").exists());
    }
}

fn install_skill_with_registry_and_generic_dir(
    registry: &ToolRegistry,
    generic_dir: &Path,
    skill_name: &str,
    tool_id: &str,
    mode: &str,
    source_path: Option<&Path>,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let tool_id = canonical_skill_tool_id(tool_id);
    let subject = tool_id.to_string();
    if !matches!(mode, "symlink" | "link") {
        return preview_reject_or_error(
            dry_run,
            ACTION_INSTALL,
            skill_name,
            &subject,
            format!("Unsupported Community install mode: {}", mode),
        );
    }

    let tool = detected_tool_for_skill_action(registry, &tool_id)?;
    if detected_tool_can_read_shared_skills(&tool) {
        return preview_reject_or_error(
            dry_run,
            ACTION_INSTALL,
            skill_name,
            &subject,
            format!(
                "Tool directly reads shared Skills; shared Skill install is unavailable: {}",
                tool_id
            ),
        );
    }
    let tool_skills_dir =
        resolved_detected_tool_skills_dir_for_action(&tool, &ToolCapabilityAction::Link)
            .or_else(|| {
                resolved_detected_tool_skills_dir_for_action(&tool, &ToolCapabilityAction::Install)
            })
            .ok_or_else(|| {
                format!(
                    "Tool does not have a deployable Skill directory: {}",
                    tool_id
                )
            })?;
    let target_path = resolved_skill_path_in_root(&tool_skills_dir, skill_name);

    let mut preview = OperationPreview::default();

    let Some(source_dir) = source_path.map(Path::to_path_buf) else {
        return preview_reject_or_error(
            dry_run,
            ACTION_INSTALL,
            skill_name,
            &subject,
            "source_path is required for Community Skill install".to_string(),
        );
    };
    if !is_valid_skill_dir(&source_dir) {
        return preview_reject_or_error(
            dry_run,
            ACTION_INSTALL,
            skill_name,
            &subject,
            "source_path must be an existing skill directory containing SKILL.md".to_string(),
        );
    }
    if !path_is_literal_child_of_root(&source_dir, generic_dir) {
        return preview_reject_or_error(
            dry_run,
            ACTION_INSTALL,
            skill_name,
            &subject,
            "source_path must be a shared-directory source".to_string(),
        );
    }

    if target_path.exists() || target_path.is_symlink() {
        push_preview_change_with_entry_kind(
            &mut preview,
            PreviewChangeKind::Delete,
            ACTION_INSTALL,
            skill_name,
            subject.clone(),
            &target_path,
            Some(preview_entry_kind_for_existing_path(&target_path)),
        );
    }
    push_preview_change_with_entry_kind(
        &mut preview,
        PreviewChangeKind::Create,
        ACTION_INSTALL,
        skill_name,
        subject.clone(),
        &target_path,
        Some("symlink"),
    );

    if dry_run {
        return Ok(preview);
    }

    remove_if_exists(&target_path)?;
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    create_skill_symlink(&source_dir, &target_path)?;

    preview.successes.push(OperationReceiptItem {
        tool_id: Some(tool_id.to_string()),
        path: target_path.to_string_lossy().to_string(),
        action: ACTION_INSTALL.to_string(),
        reason: None,
    });

    Ok(preview)
}

pub(crate) fn install_skill_with_registry(
    registry: &ToolRegistry,
    skill_name: &str,
    tool_id: &str,
    mode: &str,
    source_path: Option<&Path>,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let generic_dir = crate::platform::env::generic_skills_dir();
    install_skill_with_registry_and_generic_dir(
        registry,
        &generic_dir,
        skill_name,
        tool_id,
        mode,
        source_path,
        dry_run,
    )
}

pub(crate) fn install_skill_v2_domain(
    registry: &ToolRegistry,
    skill_name: String,
    tool_id: String,
    mode: String,
    source_path: Option<String>,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let tool_id = canonical_skill_tool_id(&tool_id);
    install_skill_with_registry(
        registry,
        &skill_name,
        &tool_id,
        &mode,
        source_path.as_deref().map(Path::new),
        dry_run,
    )
}

// Pure file-level copy from one tool's Skill directory into another tool's
// Skill directory. Shared-directory sources install through the link command.
pub(crate) fn copy_skill_to_tool_domain(
    registry: &ToolRegistry,
    skill_name: String,
    tool_id: String,
    source_path: String,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let generic_dir = crate::platform::env::generic_skills_dir();
    copy_skill_to_tool_with_generic_dir(
        registry,
        &generic_dir,
        skill_name,
        tool_id,
        source_path,
        dry_run,
    )
}

fn copy_skill_to_tool_with_generic_dir(
    registry: &ToolRegistry,
    generic_dir: &Path,
    skill_name: String,
    tool_id: String,
    source_path: String,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let config = crate::platform::config::load_config();
    copy_skill_to_tool_for_config_with_generic_dir(
        registry,
        generic_dir,
        skill_name,
        tool_id,
        source_path,
        dry_run,
        &config,
    )
}

fn copy_skill_to_tool_for_config_with_generic_dir(
    registry: &ToolRegistry,
    generic_dir: &Path,
    skill_name: String,
    tool_id: String,
    source_path: String,
    dry_run: bool,
    config: &crate::platform::config::AppConfig,
) -> Result<OperationPreview, String> {
    let tool_id = canonical_skill_tool_id(&tool_id);
    let subject = tool_id.clone();
    let tool = detected_tool_for_skill_action_with_config(registry, &tool_id, config)?;
    let tool_skills_dir =
        resolved_detected_tool_skills_dir_for_action(&tool, &ToolCapabilityAction::Copy)
            .ok_or_else(|| format!("Tool does not support Skill copy: {}", tool_id))?;
    let target_path = resolved_skill_path_in_root(&tool_skills_dir, &skill_name);

    let source = PathBuf::from(&source_path);
    if path_is_or_resolves_within_root(&source, generic_dir) {
        return preview_reject_or_error(
            dry_run,
            ACTION_COPY,
            &skill_name,
            &subject,
            "shared-directory sources must use install".to_string(),
        );
    }
    if !is_valid_skill_dir(&source) {
        return preview_reject_or_error(
            dry_run,
            ACTION_COPY,
            &skill_name,
            &subject,
            "source_path must be an existing skill directory containing SKILL.md".to_string(),
        );
    }
    let normalized_source = normalized_path(&source);
    let known_tool_sources =
        known_tool_skill_source_paths_with_generic_dir(registry, generic_dir, &skill_name)
            .into_iter()
            .map(|path| normalized_path(&path))
            .collect::<HashSet<_>>();
    if !known_tool_sources.contains(&normalized_source)
        || !source_path_matches_skill(&normalized_source, &skill_name)
    {
        return preview_reject_or_error(
            dry_run,
            ACTION_COPY,
            &skill_name,
            &subject,
            "source_path must be a known tool Skill source".to_string(),
        );
    }

    let target_is_symlink = fs::symlink_metadata(&target_path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false);
    let canonical_source = fs::canonicalize(&source).ok();
    let canonical_target = fs::canonicalize(&target_path).ok();
    let copy_source = if target_is_symlink {
        canonical_source.clone().unwrap_or_else(|| source.clone())
    } else {
        source.clone()
    };
    if let (Some(src), Some(dst)) = (canonical_source.as_ref(), canonical_target.as_ref()) {
        if src == dst && !target_is_symlink {
            return preview_reject_or_error(
                dry_run,
                ACTION_COPY,
                &skill_name,
                &subject,
                "source_path and target resolve to the same directory".to_string(),
            );
        }
    }

    let mut preview = OperationPreview::default();
    if target_path.exists() || target_path.is_symlink() {
        push_preview_change_with_entry_kind(
            &mut preview,
            PreviewChangeKind::Delete,
            ACTION_COPY,
            &skill_name,
            subject.clone(),
            &target_path,
            Some(preview_entry_kind_for_existing_path(&target_path)),
        );
    }
    push_preview_change_with_entry_kind(
        &mut preview,
        PreviewChangeKind::Create,
        ACTION_COPY,
        &skill_name,
        subject.clone(),
        &target_path,
        Some("file"),
    );

    if dry_run {
        return Ok(preview);
    }

    remove_if_exists(&target_path)?;
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    copy_dir_recursive(&copy_source, &target_path)?;

    preview.successes.push(OperationReceiptItem {
        tool_id: Some(tool_id),
        path: target_path.to_string_lossy().to_string(),
        action: ACTION_COPY.to_string(),
        reason: None,
    });

    Ok(preview)
}

pub(crate) fn link_shared_skill_to_tool_with_paths(
    registry: &ToolRegistry,
    generic_dir: &Path,
    skill_name: String,
    tool_id: String,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let tool_id = canonical_skill_tool_id(&tool_id);
    let subject = tool_id.clone();
    let Some(shared_source) = find_skill_dir_in_root(generic_dir, &skill_name) else {
        let reason = reject_reason(
            REASON_SHARED_SOURCE_MISSING,
            "共享目录缺少可用的 Skill 文件",
            Some(
                "shared directory must be an existing skill directory containing SKILL.md"
                    .to_string(),
            ),
        );
        if dry_run {
            return Ok(blocked_preview(ACTION_COPY, &skill_name, &subject, reason));
        }
        return Err(reason.message);
    };

    install_skill_with_registry_and_generic_dir(
        registry,
        generic_dir,
        &skill_name,
        &tool_id,
        "symlink",
        Some(shared_source.as_path()),
        dry_run,
    )
}

pub(crate) fn link_shared_skill_to_tool_domain(
    registry: &ToolRegistry,
    skill_name: String,
    tool_id: String,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let generic_dir = crate::platform::env::generic_skills_dir();
    link_shared_skill_to_tool_with_paths(registry, &generic_dir, skill_name, tool_id, dry_run)
}

fn sanitize_skill_dir_name(skill_name: &str) -> Result<String, String> {
    let normalized = skill_name.trim().to_string();
    if normalized.is_empty() {
        return Err("Skill name is required".to_string());
    }
    if normalized == "."
        || normalized == ".."
        || normalized.starts_with('.')
        || normalized.contains('/')
        || normalized.contains('\\')
    {
        return Err("Skill name must be a single visible directory name".to_string());
    }
    Ok(normalized)
}

fn local_source_owner_for_path(registry: &ToolRegistry, source_path: &Path) -> Option<String> {
    let source = normalized_path(source_path);
    for tool in registry.detect_all() {
        if !tool.detected {
            continue;
        }
        let Some(root) =
            resolved_detected_tool_skills_dir_for_action(&tool, &ToolCapabilityAction::Save)
                .or_else(|| {
                    resolved_detected_tool_skills_dir_for_action(
                        &tool,
                        &ToolCapabilityAction::Delete,
                    )
                })
        else {
            continue;
        };
        if path_stays_within_root(&source, &root) && source != normalized_path(&root) {
            return Some(tool.id);
        }
    }

    let generic_root = crate::platform::env::generic_skills_dir();
    if path_stays_within_root(&source, &generic_root) && source != normalized_path(&generic_root) {
        return Some("generic".to_string());
    }

    None
}

pub(crate) fn rename_skill_source_domain(
    registry: &ToolRegistry,
    skill_name: String,
    source_path: String,
    new_skill_name: String,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let old_name = sanitize_skill_dir_name(&skill_name)?;
    let new_name = sanitize_skill_dir_name(&new_skill_name)?;
    if old_name == new_name {
        return preview_reject_or_error(
            dry_run,
            ACTION_RENAME_SOURCE,
            &old_name,
            "local",
            "新名称必须不同于当前 Skill 名称".to_string(),
        );
    }

    let source = normalized_path(Path::new(&source_path));
    if !is_valid_skill_dir(&source) {
        return preview_reject_or_error(
            dry_run,
            ACTION_RENAME_SOURCE,
            &old_name,
            "local",
            "source_path must be an existing skill directory containing SKILL.md".to_string(),
        );
    }
    if source.file_name().and_then(|v| v.to_str()) != Some(old_name.as_str()) {
        return preview_reject_or_error(
            dry_run,
            ACTION_RENAME_SOURCE,
            &old_name,
            "local",
            "source_path must point at the Skill being renamed".to_string(),
        );
    }

    let Some(owner_tool_id) = local_source_owner_for_path(registry, &source) else {
        return preview_reject_or_error(
            dry_run,
            ACTION_RENAME_SOURCE,
            &old_name,
            "local",
            "source_path is not a writable local Skill source".to_string(),
        );
    };

    let Some(parent) = source.parent() else {
        return preview_reject_or_error(
            dry_run,
            ACTION_RENAME_SOURCE,
            &old_name,
            &owner_tool_id,
            "source_path has no parent directory".to_string(),
        );
    };
    let target = parent.join(&new_name);
    if target.exists() || target.is_symlink() {
        return preview_reject_or_error(
            dry_run,
            ACTION_RENAME_SOURCE,
            &old_name,
            &owner_tool_id,
            "目标名称已存在，请选择另一个名称".to_string(),
        );
    }
    if !can_write_skill_path(registry, &source, &source)
        || !can_write_skill_path(registry, &target, &target)
    {
        return preview_reject_or_error(
            dry_run,
            ACTION_RENAME_SOURCE,
            &old_name,
            &owner_tool_id,
            "Skill source is not writable for current tool capabilities".to_string(),
        );
    }

    let mut preview = OperationPreview::default();
    push_preview_change_with_entry_kind(
        &mut preview,
        PreviewChangeKind::Create,
        ACTION_RENAME_SOURCE,
        &new_name,
        owner_tool_id.clone(),
        &target,
        Some("file"),
    );
    push_preview_change_with_entry_kind(
        &mut preview,
        PreviewChangeKind::Delete,
        ACTION_RENAME_SOURCE,
        &old_name,
        owner_tool_id.clone(),
        &source,
        Some(preview_entry_kind_for_existing_path(&source)),
    );

    if dry_run {
        return Ok(preview);
    }

    with_path_snapshots(vec![source.clone(), target.clone()], || {
        copy_dir_recursive(&source, &target)?;
        remove_skill_path(&source)?;
        crate::platform::config::update_config(|config| {
            if let Some(note) = config.skill_notes.remove(&old_name) {
                config.skill_notes.entry(new_name.clone()).or_insert(note);
            }
            Ok(())
        })?;
        Ok(())
    })?;
    preview.successes.push(OperationReceiptItem {
        tool_id: Some(owner_tool_id),
        path: target.to_string_lossy().to_string(),
        action: ACTION_RENAME_SOURCE.to_string(),
        reason: None,
    });
    Ok(preview)
}

pub(crate) fn uninstall_skill_v2_domain(
    registry: &ToolRegistry,
    skill_name: String,
    tool_id: String,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let tool_id = canonical_skill_tool_id(&tool_id);
    let subject = tool_id.clone();
    let tool = detected_tool_for_skill_action(registry, &tool_id)?;
    if detected_tool_can_read_shared_skills(&tool) {
        return preview_reject_or_error(
            dry_run,
            ACTION_UNINSTALL,
            &skill_name,
            &subject,
            format!(
                "Tool directly reads shared Skills; shared Skill uninstall is unavailable: {}",
                tool_id
            ),
        );
    }
    let tool_skills_dir =
        resolved_detected_tool_skills_dir_for_action(&tool, &ToolCapabilityAction::Uninstall)
            .ok_or_else(|| format!("Tool does not support Skill uninstall: {}", tool_id))?;
    let target_path = resolved_skill_path_in_root(&tool_skills_dir, &skill_name);

    let target_is_symlink = fs::symlink_metadata(&target_path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false);
    let mut preview = OperationPreview::default();
    if target_is_symlink {
        push_preview_change_with_entry_kind(
            &mut preview,
            PreviewChangeKind::Delete,
            ACTION_UNINSTALL,
            &skill_name,
            subject.clone(),
            &target_path,
            Some(preview_entry_kind_for_existing_path(&target_path)),
        );
    } else if target_path.exists() {
        return preview_reject_or_error(
            dry_run,
            ACTION_UNINSTALL,
            &skill_name,
            &subject,
            "target tool has a real Skill file; use delete".to_string(),
        );
    } else {
        preview.message = Some("目标工具未安装该 skill".to_string());
    }

    if !dry_run && target_is_symlink {
        remove_skill_path(&target_path)?;
        preview.successes.push(OperationReceiptItem {
            tool_id: Some(tool_id),
            path: target_path.to_string_lossy().to_string(),
            action: ACTION_UNINSTALL.to_string(),
            reason: None,
        });
    }

    Ok(preview)
}

pub(crate) fn delete_skill_from_tool_v2_domain(
    registry: &ToolRegistry,
    skill_name: String,
    tool_id: String,
    source_path: Option<String>,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let tool_id = canonical_skill_tool_id(&tool_id);
    let subject = tool_id.clone();
    let tool = detected_tool_for_skill_action(registry, &tool_id)?;
    let tool_skills_dir =
        resolved_detected_tool_skills_dir_for_action(&tool, &ToolCapabilityAction::Delete)
            .ok_or_else(|| format!("Tool does not support Skill delete: {}", tool_id))?;
    let target_path =
        match select_skill_source_path(&tool_skills_dir, &skill_name, source_path.as_deref()) {
            Ok(path) => path,
            Err(err) => {
                return preview_reject_or_error(
                    dry_run,
                    ACTION_DELETE_FROM_TOOL,
                    &skill_name,
                    &subject,
                    err,
                )
            }
        };
    let generic_dir = crate::platform::env::generic_skills_dir();
    let generic_skill_path = find_skill_dir_in_root(&generic_dir, &skill_name);
    let tool_status_parent = target_path.parent().unwrap_or(tool_skills_dir.as_path());
    let metadata_skill_names = detected_tool_metadata_skill_names(&tool);
    let runtime_status = derive_tool_skill_status(
        &skill_name,
        &tool_id,
        &tool.name,
        tool_status_parent,
        detected_tool_can_read_shared_skills(&tool),
        tool.allow_external_generic_symlink,
        &generic_dir,
        generic_skill_path.as_deref(),
        metadata_skill_names.as_ref(),
    )?;
    let target_is_symlink = fs::symlink_metadata(&target_path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false);
    if target_is_symlink {
        return preview_reject_or_error(
            dry_run,
            ACTION_DELETE_FROM_TOOL,
            &skill_name,
            &subject,
            "target tool has a linked Skill install; use uninstall".to_string(),
        );
    }
    if runtime_status.path_origin == "generic" && !target_is_symlink {
        let reason = reject_reason(
            REASON_POLICY_MISMATCH,
            "当前工具正在直接使用共享目录，请在共享入口或全局删除里处理",
            Some("shared_source_tool_delete_not_allowed".to_string()),
        );
        if dry_run {
            return Ok(blocked_preview(
                ACTION_DELETE_FROM_TOOL,
                &skill_name,
                &subject,
                reason,
            ));
        }
        return Err("当前工具正在直接使用共享目录，请在共享入口或全局删除里处理".to_string());
    }
    if runtime_status.status == SkillStatus::VariantDrifted {
        let reason = reject_reason(
            REASON_POLICY_MISMATCH,
            "Skill 内容与元数据不一致，请先在工具内修复后再删除",
            Some("metadata_drift_delete_not_allowed".to_string()),
        );
        if dry_run {
            return Ok(blocked_preview(
                ACTION_DELETE_FROM_TOOL,
                &skill_name,
                &subject,
                reason,
            ));
        }
        return Err("Skill 内容与元数据不一致，请先在工具内修复后再删除".to_string());
    }

    let mut preview = OperationPreview::default();
    if target_path.exists() || target_path.is_symlink() {
        push_preview_change_with_entry_kind(
            &mut preview,
            PreviewChangeKind::Delete,
            ACTION_DELETE_FROM_TOOL,
            &skill_name,
            subject.clone(),
            &target_path,
            Some(preview_entry_kind_for_existing_path(&target_path)),
        );
    } else {
        preview.message = Some("目标工具未检测到可删除副本".to_string());
    }

    if !dry_run && (target_path.exists() || target_path.is_symlink()) {
        remove_skill_path(&target_path)?;
        preview.successes.push(OperationReceiptItem {
            tool_id: Some(tool_id),
            path: target_path.to_string_lossy().to_string(),
            action: ACTION_DELETE_FROM_TOOL.to_string(),
            reason: None,
        });
    }

    Ok(preview)
}

#[derive(Clone, Debug)]
struct KnownSkillSourcePath {
    path: PathBuf,
    owner: String,
}

fn known_local_skill_sources_with_generic_dir(
    registry: &ToolRegistry,
    generic_dir: &Path,
    skill_name: &str,
) -> Vec<KnownSkillSourcePath> {
    let mut sources = find_skill_dirs_in_root(generic_dir, skill_name)
        .into_iter()
        .map(|path| KnownSkillSourcePath {
            path,
            owner: SUBJECT_SHARED.to_string(),
        })
        .collect::<Vec<_>>();
    for tool in registry.detect_all() {
        if !tool.detected {
            continue;
        }
        if let Some(root) = resolved_detected_tool_skills_dir(&tool) {
            sources.extend(
                find_skill_dirs_in_root(&root, skill_name)
                    .into_iter()
                    .map(|path| KnownSkillSourcePath {
                        path,
                        owner: tool.id.clone(),
                    }),
            );
        }
    }
    sources.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.owner.cmp(&b.owner)));
    sources.dedup_by(|a, b| a.path == b.path && a.owner == b.owner);
    sources
}

fn known_tool_skill_source_paths_with_generic_dir(
    registry: &ToolRegistry,
    generic_dir: &Path,
    skill_name: &str,
) -> Vec<PathBuf> {
    let mut paths = known_local_skill_sources_with_generic_dir(registry, generic_dir, skill_name)
        .into_iter()
        .filter(|source| source.owner != SUBJECT_SHARED)
        .map(|source| source.path)
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

fn source_path_is_deletable_with_generic_dir(
    registry: &ToolRegistry,
    generic_dir: &Path,
    path: &Path,
) -> bool {
    if path_stays_within_root(path, generic_dir) {
        return true;
    }
    registry.detect_all().into_iter().any(|tool| {
        tool.detected
            && resolved_detected_tool_skills_dir_for_action(&tool, &ToolCapabilityAction::Delete)
                .is_some_and(|root| path_stays_within_root(path, &root))
    })
}

fn subject_for_source_path_with_generic_dir(
    registry: &ToolRegistry,
    generic_dir: &Path,
    path: &Path,
) -> String {
    if path.starts_with(&generic_dir) {
        return SUBJECT_SHARED.to_string();
    }
    registry
        .detect_all()
        .into_iter()
        .find(|tool| {
            resolved_detected_tool_skills_dir_for_action(tool, &ToolCapabilityAction::Delete)
                .is_some_and(|dir| path.starts_with(&dir))
        })
        .map(|tool| tool.id)
        .unwrap_or_else(|| SUBJECT_SHARED.to_string())
}

fn selected_duplicate_paths_with_generic_dir(
    registry: &ToolRegistry,
    generic_dir: &Path,
    skill_name: &str,
    keep_source_path: &str,
    delete_source_paths: &[String],
) -> Result<(PathBuf, Vec<PathBuf>), String> {
    let known_sources =
        known_local_skill_sources_with_generic_dir(registry, generic_dir, skill_name);
    let known_paths = known_sources
        .iter()
        .map(|source| (normalized_path(&source.path), source.owner.clone()))
        .collect::<HashMap<_, _>>();
    let known_path_set = known_paths.keys().cloned().collect::<HashSet<_>>();
    let normalized_sources = known_sources
        .into_iter()
        .map(|source| KnownSkillSourcePath {
            path: normalized_path(&source.path),
            owner: source.owner,
        })
        .collect::<Vec<_>>();
    let keep = normalized_path(Path::new(keep_source_path));
    if !known_path_set.contains(&keep) || !source_path_matches_skill(&keep, skill_name) {
        return Err("keep_source_path is not a known duplicate Skill source".to_string());
    }
    let mut delete_paths = Vec::new();
    for raw in delete_source_paths {
        let path = normalized_path(Path::new(raw));
        if path == keep {
            return Err("delete_source_paths cannot include the kept source".to_string());
        }
        if !known_path_set.contains(&path) || !source_path_matches_skill(&path, skill_name) {
            return Err(
                "delete_source_paths contains an unknown duplicate Skill source".to_string(),
            );
        }
        if !source_path_is_deletable_with_generic_dir(registry, generic_dir, &path) {
            return Err("delete_source_paths contains a source that is not deletable".to_string());
        }
        delete_paths.push(path);
    }
    delete_paths.sort();
    delete_paths.dedup();
    if delete_paths.is_empty() {
        return Err("duplicate cleanup requires at least one source to delete".to_string());
    }
    let selected_paths = std::iter::once(keep.clone())
        .chain(delete_paths.iter().cloned())
        .collect::<Vec<_>>();
    let selected_tool_owners = selected_paths
        .iter()
        .filter_map(|path| known_paths.get(path))
        .filter(|owner| owner.as_str() != SUBJECT_SHARED)
        .cloned()
        .collect::<HashSet<_>>();
    if selected_tool_owners.len() > 1 {
        return Err("duplicate cleanup cannot span multiple tool source groups".to_string());
    }
    let group_owner = selected_tool_owners
        .iter()
        .next()
        .cloned()
        .unwrap_or_else(|| SUBJECT_SHARED.to_string());
    let group_source_count = normalized_sources
        .iter()
        .filter(|source| {
            source.owner == group_owner
                || (group_owner != SUBJECT_SHARED && source.owner == SUBJECT_SHARED)
        })
        .count();
    if group_source_count < 2 {
        return Err(
            "duplicate cleanup requires multiple sources in the same source group".to_string(),
        );
    }
    Ok((keep, delete_paths))
}

fn cleanup_duplicate_skill_sources_with_generic_dir(
    registry: &ToolRegistry,
    generic_dir: &Path,
    skill_name: String,
    keep_source_path: String,
    delete_source_paths: Vec<String>,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let (keep_path, delete_paths) = selected_duplicate_paths_with_generic_dir(
        registry,
        generic_dir,
        &skill_name,
        &keep_source_path,
        &delete_source_paths,
    )?;
    let mut preview = OperationPreview::default();
    push_preview_change_with_entry_kind(
        &mut preview,
        PreviewChangeKind::Preserve,
        ACTION_DELETE_SKILL,
        &skill_name,
        subject_for_source_path_with_generic_dir(registry, generic_dir, &keep_path),
        &keep_path,
        Some(preview_entry_kind_for_existing_path(&keep_path)),
    );
    let deletes_shared_source = delete_paths
        .iter()
        .any(|path| path.starts_with(&generic_dir));
    if deletes_shared_source {
        preview.message = Some("删除共享目录来源可能影响其他使用共享目录的工具。".to_string());
    }
    for path in &delete_paths {
        push_preview_change_with_entry_kind(
            &mut preview,
            PreviewChangeKind::Delete,
            ACTION_DELETE_SKILL,
            &skill_name,
            subject_for_source_path_with_generic_dir(registry, generic_dir, path),
            path,
            Some(preview_entry_kind_for_existing_path(path)),
        );
    }

    if dry_run {
        return Ok(preview);
    }

    for path in &delete_paths {
        remove_skill_path(path)?;
        preview.successes.push(OperationReceiptItem {
            tool_id: registry
                .detect_all()
                .into_iter()
                .find(|tool| {
                    resolved_detected_tool_skills_dir_for_action(
                        tool,
                        &ToolCapabilityAction::Delete,
                    )
                    .is_some_and(|dir| path.starts_with(&dir))
                })
                .map(|tool| tool.id),
            path: path.to_string_lossy().to_string(),
            action: ACTION_DELETE_SKILL.to_string(),
            reason: None,
        });
    }
    crate::platform::config::update_config(|config| {
        maybe_cleanup_deleted_skill_metadata(config, &skill_name, &delete_paths);
        Ok(())
    })?;
    Ok(preview)
}

pub(crate) fn cleanup_duplicate_skill_sources_domain(
    registry: &ToolRegistry,
    skill_name: String,
    keep_source_path: String,
    delete_source_paths: Vec<String>,
    dry_run: bool,
) -> Result<OperationPreview, String> {
    let generic_dir = crate::platform::env::generic_skills_dir();
    cleanup_duplicate_skill_sources_with_generic_dir(
        registry,
        &generic_dir,
        skill_name,
        keep_source_path,
        delete_source_paths,
        dry_run,
    )
}

pub(crate) fn delete_skill_v2_domain(
    registry: &ToolRegistry,
    skill_name: String,
    dry_run: bool,
    allow_generic_write: Option<bool>,
) -> Result<OperationPreview, String> {
    let allow_generic_write = allow_generic_write.unwrap_or(false);
    let mut preview = OperationPreview::default();
    let mut delete_paths: HashSet<PathBuf> = HashSet::new();
    let generic_dir = crate::platform::env::generic_skills_dir();

    let generic_path = find_skill_dir_in_root(&generic_dir, &skill_name);

    let mut generic_reader_tool_ids = Vec::new();
    for tool in registry.detect_all() {
        if !tool.detected {
            continue;
        }
        let Some(tool_skills_dir) = resolved_detected_tool_skills_dir(&tool) else {
            continue;
        };
        let tool_path = resolved_skill_path_in_root(&tool_skills_dir, &skill_name);
        let tool_status_parent = tool_path.parent().unwrap_or(tool_skills_dir.as_path());
        let metadata_skill_names = detected_tool_metadata_skill_names(&tool);
        let runtime_status = derive_tool_skill_status(
            &skill_name,
            &tool.id,
            &tool.name,
            tool_status_parent,
            detected_tool_can_read_shared_skills(&tool),
            tool.allow_external_generic_symlink,
            &generic_dir,
            generic_path.as_deref(),
            metadata_skill_names.as_ref(),
        )?;
        if runtime_status.status == SkillStatus::VariantDrifted {
            let reason = reject_reason(
                REASON_POLICY_MISMATCH,
                "Skill 内容与元数据不一致，请先在工具内修复后再删除",
                Some("metadata_drift_delete_not_allowed".to_string()),
            );
            push_preview_blocked(
                &mut preview,
                ACTION_DELETE_SKILL,
                &skill_name,
                tool.id.clone(),
                reason,
            );
            if let Some(path) = runtime_status.path {
                preview.skipped.push(OperationReceiptItem {
                    tool_id: Some(tool.id.clone()),
                    path,
                    action: ACTION_DELETE_SKILL.to_string(),
                    reason: Some("metadata_drift_delete_not_allowed".to_string()),
                });
            }
            continue;
        }
        if runtime_status.path_origin == "generic" {
            generic_reader_tool_ids.push(tool.id);
        }
    }
    generic_reader_tool_ids.sort();
    generic_reader_tool_ids.dedup();

    if let Some(generic_path) = generic_path.as_ref() {
        if !allow_generic_write {
            push_preview_change_with_entry_kind(
                &mut preview,
                PreviewChangeKind::Preserve,
                ACTION_DELETE_SKILL,
                &skill_name,
                SUBJECT_SHARED,
                generic_path,
                Some(preview_entry_kind_for_existing_path(generic_path)),
            );
            preview.skipped.push(OperationReceiptItem {
                tool_id: None,
                path: generic_path.to_string_lossy().to_string(),
                action: ACTION_DELETE_SKILL.to_string(),
                reason: Some(REASON_GENERIC_WRITE_NOT_ALLOWED.to_string()),
            });
        } else {
            delete_paths.insert(generic_path.clone());
            if !generic_reader_tool_ids.is_empty() {
                preview.message = Some(format!(
                    "删除共享目录会影响这些工具：{}",
                    generic_reader_tool_ids.join("、")
                ));
            }
        }
    }

    let detected_tools = registry.detect_all();
    for tool in &detected_tools {
        if let Some(skills_dir) =
            resolved_detected_tool_skills_dir_for_action(tool, &ToolCapabilityAction::Delete)
        {
            let tool_path = resolved_skill_path_in_root(&skills_dir, &skill_name);
            if tool_path.exists() || tool_path.is_symlink() {
                delete_paths.insert(tool_path);
            }
        }
    }

    let mut delete_vec: Vec<PathBuf> = delete_paths.into_iter().collect();
    delete_vec.sort();
    for path in &delete_vec {
        let subject = if path.starts_with(&generic_dir) {
            SUBJECT_SHARED.to_string()
        } else {
            registry
                .detect_all()
                .into_iter()
                .find(|tool| {
                    resolved_detected_tool_skills_dir_for_action(
                        tool,
                        &ToolCapabilityAction::Delete,
                    )
                    .is_some_and(|dir| path.starts_with(&dir))
                })
                .map(|tool| tool.id)
                .unwrap_or_else(|| SUBJECT_SHARED.to_string())
        };
        push_preview_change_with_entry_kind(
            &mut preview,
            PreviewChangeKind::Delete,
            ACTION_DELETE_SKILL,
            &skill_name,
            subject,
            path,
            Some(preview_entry_kind_for_existing_path(path)),
        );
    }

    if dry_run {
        return Ok(preview);
    }

    if !preview.blocked.is_empty() {
        return Err(preview
            .message
            .clone()
            .unwrap_or_else(|| "Skill 删除被阻止".to_string()));
    }

    for path in &delete_vec {
        remove_skill_path(path)?;
        preview.successes.push(OperationReceiptItem {
            tool_id: registry
                .detect_all()
                .into_iter()
                .find(|tool| {
                    resolved_detected_tool_skills_dir_for_action(
                        tool,
                        &ToolCapabilityAction::Delete,
                    )
                    .is_some_and(|dir| path.starts_with(&dir))
                })
                .map(|tool| tool.id),
            path: path.to_string_lossy().to_string(),
            action: ACTION_DELETE_SKILL.to_string(),
            reason: None,
        });
    }

    crate::platform::config::update_config(|config| {
        maybe_cleanup_deleted_skill_metadata(config, &skill_name, &delete_vec);
        Ok(())
    })?;

    Ok(preview)
}

pub(crate) fn maybe_cleanup_deleted_skill_metadata(
    config: &mut crate::platform::config::AppConfig,
    skill_name: &str,
    delete_paths: &[PathBuf],
) -> bool {
    if delete_paths.is_empty() {
        return false;
    }
    config.skill_notes.remove(skill_name);
    true
}
