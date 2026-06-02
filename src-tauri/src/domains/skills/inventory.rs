// Purpose: Skill inventory scanning, status aggregation, and read-only listing.

use super::*;

pub(crate) fn resolved_detected_tool_skills_dir(tool: &DetectedTool) -> Option<PathBuf> {
    detected_tool_skill_directory_projection(tool, &ToolCapabilityAction::View)
        .and_then(|projection| capability_declared_source_path(&projection.evidence))
}

fn detected_tool_skill_projections(tool: &DetectedTool) -> Vec<ToolCapabilityProjection> {
    project_capabilities(&tool.id, ToolCapabilityModule::Skills, &tool.capabilities)
}

#[cfg(test)]
fn adapter_skill_projections_for_config(
    adapter: &dyn ToolAdapter,
    config: &crate::platform::config::AppConfig,
) -> Vec<ToolCapabilityProjection> {
    let base_capabilities = adapter.capabilities();
    let capabilities = crate::adapters::effective_capabilities::resolve_effective_capabilities(
        adapter.id(),
        &base_capabilities,
        config,
    );
    project_capabilities(adapter.id(), ToolCapabilityModule::Skills, &capabilities)
}

#[cfg(test)]
fn adapter_skill_projections(adapter: &dyn ToolAdapter) -> Vec<ToolCapabilityProjection> {
    let config = crate::platform::config::load_config();
    adapter_skill_projections_for_config(adapter, &config)
}

fn detected_tool_skill_directory_projection(
    tool: &DetectedTool,
    action: &ToolCapabilityAction,
) -> Option<ToolCapabilityProjection> {
    detected_tool_skill_projections(tool)
        .into_iter()
        .find(|projection| {
            is_tool_skill_source_role(&projection.source_role) && projection.allows(action)
        })
}

#[cfg(test)]
fn adapter_skill_directory_projection(
    adapter: &dyn ToolAdapter,
    action: &ToolCapabilityAction,
) -> Option<ToolCapabilityProjection> {
    adapter_skill_projections(adapter)
        .into_iter()
        .find(|projection| {
            is_tool_skill_source_role(&projection.source_role) && projection.allows(action)
        })
}

#[cfg(test)]
fn adapter_deployable_skill_directory_projection(
    adapter: &dyn ToolAdapter,
    action: &ToolCapabilityAction,
) -> Option<ToolCapabilityProjection> {
    adapter_skill_projections(adapter)
        .into_iter()
        .find(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillToolDirectory
                && projection.evidence.is_writable()
                && projection.allows(action)
        })
}

#[cfg(test)]
pub(crate) fn resolved_adapter_skills_dir_for_action(
    adapter: &dyn ToolAdapter,
    action: &ToolCapabilityAction,
) -> Option<PathBuf> {
    adapter_skill_directory_projection(adapter, action)
        .and_then(|projection| capability_declared_source_path(&projection.evidence))
}

#[cfg(test)]
pub(crate) fn resolved_adapter_skills_dir_for_action_with_config(
    adapter: &dyn ToolAdapter,
    action: &ToolCapabilityAction,
    config: &crate::platform::config::AppConfig,
) -> Option<PathBuf> {
    adapter_skill_projections_for_config(adapter, config)
        .into_iter()
        .find(|projection| {
            is_tool_skill_source_role(&projection.source_role) && projection.allows(action)
        })
        .and_then(|projection| capability_declared_source_path(&projection.evidence))
}

pub(crate) fn resolved_detected_tool_skills_dir_for_action(
    tool: &DetectedTool,
    action: &ToolCapabilityAction,
) -> Option<PathBuf> {
    detected_tool_skill_directory_projection(tool, action)
        .and_then(|projection| capability_declared_source_path(&projection.evidence))
}

fn is_tool_skill_source_role(role: &ToolCapabilitySourceRole) -> bool {
    matches!(
        role,
        ToolCapabilitySourceRole::SkillToolDirectory
            | ToolCapabilitySourceRole::SkillCompoundSource
    )
}

#[cfg(test)]
pub(crate) fn resolved_adapter_skills_dir(adapter: &dyn ToolAdapter) -> Option<PathBuf> {
    adapter_skill_directory_projection(adapter, &ToolCapabilityAction::View)
        .and_then(|projection| capability_declared_source_path(&projection.evidence))
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

#[cfg(test)]
fn adapter_can_read_shared_skills_with_override(
    adapter: &dyn ToolAdapter,
    direct_read_override: Option<bool>,
) -> bool {
    if let Some(value) = direct_read_override {
        return value;
    }
    adapter_skill_projections(adapter)
        .into_iter()
        .any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
                && projection.allows(&ToolCapabilityAction::View)
        })
}

#[cfg(test)]
pub(crate) fn adapter_can_read_shared_skills(adapter: &dyn ToolAdapter) -> bool {
    adapter_can_read_shared_skills_with_override(adapter, None)
}

pub(crate) fn detected_tool_can_read_shared_skills(tool: &DetectedTool) -> bool {
    detected_tool_skill_projections(tool)
        .into_iter()
        .any(|projection| {
            projection.source_role == ToolCapabilitySourceRole::SkillSharedSource
                && projection.allows(&ToolCapabilityAction::View)
        })
}

#[cfg(test)]
pub(crate) fn adapter_has_shared_skill_deploy_target(adapter: &dyn ToolAdapter) -> bool {
    if adapter_can_read_shared_skills(adapter) {
        return false;
    }
    adapter_deployable_skill_directory_projection(adapter, &ToolCapabilityAction::Link).is_some()
        || adapter_deployable_skill_directory_projection(adapter, &ToolCapabilityAction::Install)
            .is_some()
}

#[cfg(test)]
pub(crate) fn resolved_adapter_shared_skill_link_dir(adapter: &dyn ToolAdapter) -> Option<PathBuf> {
    if !adapter_has_shared_skill_deploy_target(adapter) {
        return None;
    }
    adapter_deployable_skill_directory_projection(adapter, &ToolCapabilityAction::Link)
        .or_else(|| {
            adapter_deployable_skill_directory_projection(adapter, &ToolCapabilityAction::Install)
        })
        .and_then(|projection| capability_declared_source_path(&projection.evidence))
}

pub(crate) fn registry_can_read_shared_skills(registry: &ToolRegistry) -> bool {
    registry
        .detect_all()
        .into_iter()
        .filter(|tool| tool.detected)
        .any(|tool| detected_tool_can_read_shared_skills(&tool))
}

pub(crate) fn list_skills_domain(registry: &ToolRegistry, tool_id: String) -> Vec<SkillInfo> {
    let tool_id = canonical_skill_tool_id(&tool_id);
    let Some(tool) = registry
        .detect_all()
        .into_iter()
        .find(|tool| tool.id == tool_id && tool.detected)
    else {
        return vec![];
    };
    let mut result = resolved_detected_tool_skills_dir(&tool)
        .map(|dir| scan_skills_dir(&dir, &tool_id))
        .unwrap_or_default();
    if detected_tool_can_read_shared_skills(&tool) {
        let agents_dir = crate::platform::env::generic_skills_dir();
        let generic = scan_skills_dir(&agents_dir, "generic");
        // Only add generic skills that aren't already in the tool's own directory (by name)
        let existing_names: std::collections::HashSet<String> =
            result.iter().map(|s| s.name.clone()).collect();
        for skill in generic {
            if !existing_names.contains(&skill.name) {
                result.push(skill);
            }
        }
    }
    result
}

pub(crate) fn list_all_skills_domain(registry: &ToolRegistry) -> Vec<SkillInfo> {
    let tools = registry.detect_all();
    let mut all_skills = vec![];
    for tool in &tools {
        if !tool.detected {
            continue;
        }
        if let Some(dir) = resolved_detected_tool_skills_dir(tool) {
            all_skills.extend(scan_skills_dir(&dir, &tool.id));
        }
    }
    if registry_can_read_shared_skills(registry) {
        let agents_dir = crate::platform::env::generic_skills_dir();
        all_skills.extend(scan_skills_dir(&agents_dir, "generic"));
    }
    all_skills
}

pub(crate) fn list_generic_skills_domain(registry: &ToolRegistry) -> Vec<SkillInfo> {
    if !registry_can_read_shared_skills(registry) {
        return vec![];
    }
    let agents_dir = crate::platform::env::generic_skills_dir();
    scan_skills_dir(&agents_dir, "generic")
}

fn collect_skill_paths_recursive(dir: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let is_symlink = entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false);
        let is_broken_symlink = is_symlink && !path.exists();
        if is_broken_symlink {
            paths.push(path);
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        if is_valid_skill_dir(&path) {
            paths.push(path);
            continue;
        }
        if !is_symlink {
            collect_skill_paths_recursive(&path, paths);
        }
    }
}

fn list_skill_paths_in_dir(dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_skill_paths_recursive(dir, &mut paths);
    paths.sort();
    paths.dedup();
    paths
}

fn collect_skill_paths_recursive_for_name(dir: &Path, skill_name: &str, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let is_symlink = entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false);
        let is_broken_symlink = is_symlink && !path.exists();
        if is_broken_symlink {
            if name == skill_name {
                paths.push(path);
            }
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        if is_valid_skill_dir(&path) {
            if name == skill_name {
                paths.push(path);
            }
            continue;
        }
        if !is_symlink {
            collect_skill_paths_recursive_for_name(&path, skill_name, paths);
        }
    }
}

fn list_skill_paths_in_dir_for_name(dir: &Path, skill_name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_skill_paths_recursive_for_name(dir, skill_name, &mut paths);
    paths.sort();
    paths.dedup();
    paths
}

fn concrete_supporting_source_path(raw: &str) -> Option<PathBuf> {
    let source = raw.split('#').next().unwrap_or("").trim();
    if source.is_empty() || source.contains('*') {
        return None;
    }
    Some(PathBuf::from(shellexpand::tilde(source).to_string()))
}

fn read_metadata_skill_names(path: &Path) -> HashSet<String> {
    let mut names = HashSet::new();
    let Ok(content) = fs::read_to_string(path) else {
        return names;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return names;
    };

    if let Some(managed) = value
        .get("managedSkills")
        .and_then(|value| value.as_object())
    {
        names.extend(managed.keys().cloned());
    }
    if let Some(skills) = value.get("skills").and_then(|value| value.as_object()) {
        names.extend(skills.keys().cloned());
    }
    if let Some(skills) = value.get("skills").and_then(|value| value.as_array()) {
        names.extend(
            skills
                .iter()
                .filter_map(|value| value.as_str().map(ToString::to_string)),
        );
    }
    names
}

fn capability_metadata_skill_names(capability: &ToolCapability) -> Option<HashSet<String>> {
    let metadata_paths = capability
        .supporting_sources
        .iter()
        .filter(|source| source.role == ToolCapabilitySupportingSourceRole::Metadata)
        .filter_map(|source| concrete_supporting_source_path(&source.source_path))
        .collect::<Vec<_>>();
    if metadata_paths.is_empty() {
        return None;
    }
    let metadata_names = metadata_paths
        .into_iter()
        .flat_map(|path| read_metadata_skill_names(&path))
        .collect::<HashSet<_>>();
    Some(metadata_names)
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn adapter_metadata_skill_names(adapter: &dyn ToolAdapter) -> Option<HashSet<String>> {
    adapter_skill_projections(adapter)
        .into_iter()
        .find(|projection| projection.source_role == ToolCapabilitySourceRole::SkillCompoundSource)
        .and_then(|projection| capability_metadata_skill_names(&projection.evidence))
}

pub(crate) fn detected_tool_metadata_skill_names(tool: &DetectedTool) -> Option<HashSet<String>> {
    detected_tool_skill_projections(tool)
        .into_iter()
        .find(|projection| projection.source_role == ToolCapabilitySourceRole::SkillCompoundSource)
        .and_then(|projection| capability_metadata_skill_names(&projection.evidence))
}

fn read_skill_display_info(skill_dir: &Path, fallback_name: &str) -> (String, String) {
    let skill_md = skill_dir.join("SKILL.md");
    if let Ok(content) = fs::read_to_string(skill_md) {
        let (name, description) = parse_skill_frontmatter(&content);
        if !name.is_empty() {
            return (name, description);
        }
        return (fallback_name.to_string(), description);
    }
    (fallback_name.to_string(), String::new())
}

pub(crate) fn derive_tool_skill_status(
    skill_name: &str,
    tool_id: &str,
    tool_name: &str,
    tool_skills_dir: &Path,
    supports_generic: bool,
    _allow_external_generic_symlink: bool,
    generic_dir: &Path,
    generic_skill_path: Option<&Path>,
    metadata_skill_names: Option<&HashSet<String>>,
) -> Result<ToolSkillStatus, String> {
    let tool_path = tool_skills_dir.join(skill_name);
    let generic_skill_dir = generic_skill_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| generic_dir.join(skill_name));
    let metadata_has_skill = metadata_skill_names.is_some_and(|names| names.contains(skill_name));
    let mut result = ToolSkillStatus {
        tool_id: tool_id.to_string(),
        tool_name: tool_name.to_string(),
        status: SkillStatus::VariantNotInstalled,
        path: None,
        path_origin: "tool".to_string(),
        updated_at: None,
        content_hash: None,
        symlink_target: None,
        sources: Vec::new(),
        abnormal_state: None,
    };

    if let Ok(meta) = fs::symlink_metadata(&tool_path) {
        result.updated_at = modified_time_to_rfc3339(meta.modified().ok());
        result.path = Some(tool_path.to_string_lossy().to_string());
        if meta.file_type().is_symlink() {
            let symlink_target = fs::read_link(&tool_path).ok();
            result.symlink_target = symlink_target
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());
            if skill_path_resolves_to(&tool_path, &generic_skill_dir)
                || symlink_target
                    .as_ref()
                    .map(|target| {
                        let resolved_target = if target.is_absolute() {
                            target.clone()
                        } else {
                            tool_path.parent().unwrap_or(tool_skills_dir).join(target)
                        };
                        normalized_path(&resolved_target) == normalized_path(&generic_skill_dir)
                    })
                    .unwrap_or(false)
            {
                result.symlink_target = Some(generic_skill_dir.to_string_lossy().to_string());
            }
            if !tool_path.exists() {
                result.status = SkillStatus::BrokenSymlink;
                return Ok(result);
            }

            result.status = SkillStatus::VariantInstalledSymlink;
            return Ok(result);
        }

        if meta.is_dir() {
            result.content_hash = Some(crate::platform::config::compute_skill_hash(&tool_path));
            if metadata_skill_names.is_some() && !metadata_has_skill {
                result.status = SkillStatus::VariantDrifted;
                return Ok(result);
            }
            result.status = SkillStatus::VariantInstalledCopy;
            return Ok(result);
        }
    }

    if metadata_has_skill {
        result.path = Some(tool_path.to_string_lossy().to_string());
        result.status = SkillStatus::VariantDrifted;
        return Ok(result);
    }

    if supports_generic {
        if generic_skill_dir.exists() {
            result.status = SkillStatus::VariantInstalledCopy;
            result.path = Some(generic_skill_dir.to_string_lossy().to_string());
            result.path_origin = "generic".to_string();
            if let Ok(meta) = fs::metadata(&generic_skill_dir) {
                result.updated_at = modified_time_to_rfc3339(meta.modified().ok());
            }
        }
    }

    Ok(result)
}

fn push_path_by_name(map: &mut HashMap<String, Vec<PathBuf>>, path: PathBuf) {
    let Some(name) = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
    else {
        return;
    };
    map.entry(name).or_default().push(path);
}

fn sorted_paths_by_name(paths: Vec<PathBuf>) -> HashMap<String, Vec<PathBuf>> {
    let mut by_name: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for path in paths {
        push_path_by_name(&mut by_name, path);
    }
    for paths in by_name.values_mut() {
        paths.sort();
        paths.dedup();
    }
    by_name
}

fn source_entry_from_status(status: &ToolSkillStatus) -> Option<SkillSourceEntry> {
    let path = status.path.clone()?;
    Some(SkillSourceEntry {
        tool_id: status.tool_id.clone(),
        tool_name: status.tool_name.clone(),
        status: status.status.clone(),
        path,
        path_origin: status.path_origin.clone(),
        updated_at: status.updated_at.clone(),
        content_hash: status.content_hash.clone(),
        symlink_target: status.symlink_target.clone(),
    })
}

fn with_source_entries(
    mut primary: ToolSkillStatus,
    source_statuses: Vec<ToolSkillStatus>,
) -> ToolSkillStatus {
    primary.sources = source_statuses
        .iter()
        .filter_map(source_entry_from_status)
        .collect();
    if primary.sources.len() > 1 {
        primary.abnormal_state = Some("duplicate_sources".to_string());
    }
    primary
}

fn derive_status_for_tool_path(
    skill_name: &str,
    tool: &DetectedTool,
    tool_path: &Path,
    generic_dir: &Path,
    generic_skill_path: Option<&Path>,
    metadata_skill_names: Option<&HashSet<String>>,
) -> Result<ToolSkillStatus, String> {
    let tool_skill_dir = tool_path.parent().unwrap_or(generic_dir);
    derive_tool_skill_status(
        skill_name,
        &tool.id,
        &tool.name,
        tool_skill_dir,
        detected_tool_can_read_shared_skills(tool),
        tool.allow_external_generic_symlink,
        generic_dir,
        generic_skill_path,
        metadata_skill_names,
    )
}

fn derive_status_for_shared_path(
    _skill_name: &str,
    tool: &DetectedTool,
    generic_skill_path: &Path,
) -> Result<ToolSkillStatus, String> {
    let mut status = ToolSkillStatus {
        tool_id: tool.id.clone(),
        tool_name: tool.name.clone(),
        status: SkillStatus::VariantInstalledCopy,
        path: Some(generic_skill_path.to_string_lossy().to_string()),
        path_origin: "generic".to_string(),
        updated_at: None,
        content_hash: None,
        symlink_target: None,
        sources: Vec::new(),
        abnormal_state: None,
    };
    if let Ok(meta) = fs::metadata(generic_skill_path) {
        status.updated_at = modified_time_to_rfc3339(meta.modified().ok());
    }
    Ok(status)
}

fn modified_time_to_rfc3339(time: Option<std::time::SystemTime>) -> Option<String> {
    let time = time?;
    let datetime: chrono::DateTime<chrono::Utc> = time.into();
    Some(datetime.to_rfc3339())
}

pub(crate) fn scan_skill_inventory_domain(
    registry: &ToolRegistry,
) -> Result<SkillInventory, String> {
    build_skill_inventory_for_current_config(registry)
}

pub(crate) fn scan_skill_inventory_entry_domain(
    registry: &ToolRegistry,
    skill_name: String,
) -> Result<SkillEntry, String> {
    build_skill_inventory_entry_for_current_config(registry, &skill_name)
}

#[cfg(test)]
pub(crate) fn build_skill_inventory(registry: &ToolRegistry) -> Result<SkillInventory, String> {
    let generic_dir = crate::platform::env::generic_skills_dir();
    build_skill_inventory_with_generic_dir(registry, &generic_dir)
}

fn build_skill_inventory_for_current_config(
    registry: &ToolRegistry,
) -> Result<SkillInventory, String> {
    let generic_dir = crate::platform::env::generic_skills_dir();
    let config = crate::platform::config::load_config();
    build_skill_inventory_for_config_with_generic_dir(registry, &generic_dir, &config)
}

#[cfg(test)]
fn build_skill_inventory_with_generic_dir(
    registry: &ToolRegistry,
    generic_dir: &Path,
) -> Result<SkillInventory, String> {
    let config = crate::platform::config::default_config();
    build_skill_inventory_for_config_with_generic_dir(registry, generic_dir, &config)
}

fn build_skill_inventory_for_config_with_generic_dir(
    registry: &ToolRegistry,
    generic_dir: &Path,
    config: &crate::platform::config::AppConfig,
) -> Result<SkillInventory, String> {
    let detected_tools = inventory_detected_tools_for_config(registry, config);
    let can_read_shared_skills = detected_tools
        .iter()
        .any(|tool| detected_tool_can_read_shared_skills(tool));
    let mut all_names: HashSet<String> = HashSet::new();
    let generic_skill_paths = if can_read_shared_skills {
        list_skill_paths_in_dir(generic_dir)
    } else {
        Vec::new()
    };
    let generic_skill_paths_by_name = sorted_paths_by_name(generic_skill_paths.clone());

    for path in &generic_skill_paths {
        if let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
        {
            all_names.insert(name.to_string());
        }
    }

    let metadata_names_by_tool = metadata_names_by_detected_tool(&detected_tools);
    let skill_paths_by_tool = skill_paths_by_detected_tool(&detected_tools);

    for tool in &detected_tools {
        let Some(paths_by_name) = skill_paths_by_tool.get(&tool.id) else {
            continue;
        };
        for name in paths_by_name.keys() {
            all_names.insert(name.to_string());
        }
        if let Some(metadata_names) = metadata_names_by_tool.get(&tool.id) {
            all_names.extend(metadata_names.iter().cloned());
        }
    }

    let mut skill_names: Vec<String> = all_names.into_iter().collect();
    skill_names.sort();

    let mut skills = Vec::with_capacity(skill_names.len());
    for skill_name in skill_names {
        skills.push(build_skill_entry_from_sources(
            &skill_name,
            &detected_tools,
            generic_dir,
            generic_skill_paths_by_name
                .get(&skill_name)
                .cloned()
                .unwrap_or_default(),
            &skill_paths_by_tool,
            &metadata_names_by_tool,
        )?);
    }

    Ok(SkillInventory { skills })
}

fn inventory_detected_tools_for_config(
    registry: &ToolRegistry,
    config: &crate::platform::config::AppConfig,
) -> Vec<DetectedTool> {
    let active_tool_ids: Option<HashSet<String>> = if config.initialized {
        Some(
            crate::domains::tools::canonical_managed_tool_ids(&config.managed_tools)
                .into_iter()
                .collect(),
        )
    } else {
        None
    };
    registry
        .detect_all_for_config(config)
        .into_iter()
        .filter(|tool| {
            tool.detected
                && active_tool_ids.as_ref().map_or(true, |ids| {
                    ids.contains(
                        &crate::platform::tool_catalog::normalization::canonical_tool_id(&tool.id),
                    )
                })
                && (resolved_detected_tool_skills_dir(tool).is_some()
                    || detected_tool_can_read_shared_skills(tool))
        })
        .collect()
}

fn metadata_names_by_detected_tool(
    detected_tools: &[DetectedTool],
) -> HashMap<String, HashSet<String>> {
    detected_tools
        .iter()
        .filter_map(|tool| {
            detected_tool_metadata_skill_names(tool).map(|names| (tool.id.clone(), names))
        })
        .collect()
}

fn skill_paths_by_detected_tool(
    detected_tools: &[DetectedTool],
) -> HashMap<String, HashMap<String, Vec<PathBuf>>> {
    detected_tools
        .iter()
        .filter_map(|tool| {
            resolved_detected_tool_skills_dir(tool).map(|dir| {
                let paths = list_skill_paths_in_dir(&dir);
                (tool.id.clone(), sorted_paths_by_name(paths))
            })
        })
        .collect()
}

fn targeted_skill_paths_by_detected_tool(
    detected_tools: &[DetectedTool],
    skill_name: &str,
) -> HashMap<String, HashMap<String, Vec<PathBuf>>> {
    detected_tools
        .iter()
        .filter_map(|tool| {
            resolved_detected_tool_skills_dir(tool).map(|dir| {
                let paths = list_skill_paths_in_dir_for_name(&dir, skill_name);
                let by_name = if paths.is_empty() {
                    HashMap::new()
                } else {
                    sorted_paths_by_name(paths)
                };
                (tool.id.clone(), by_name)
            })
        })
        .collect()
}

fn normalize_inventory_entry_skill_name(skill_name: &str) -> Result<String, String> {
    let normalized = skill_name.trim();
    if normalized.is_empty()
        || normalized == "."
        || normalized == ".."
        || normalized.contains('/')
        || normalized.contains('\\')
    {
        return Err("Skill name must be a single visible directory name".to_string());
    }
    Ok(normalized.to_string())
}

fn build_skill_inventory_entry_for_current_config(
    registry: &ToolRegistry,
    skill_name: &str,
) -> Result<SkillEntry, String> {
    let generic_dir = crate::platform::env::generic_skills_dir();
    let config = crate::platform::config::load_config();
    build_skill_inventory_entry_for_config_with_generic_dir(
        registry,
        &generic_dir,
        &config,
        skill_name,
    )
}

fn build_skill_inventory_entry_for_config_with_generic_dir(
    registry: &ToolRegistry,
    generic_dir: &Path,
    config: &crate::platform::config::AppConfig,
    skill_name: &str,
) -> Result<SkillEntry, String> {
    let skill_name = normalize_inventory_entry_skill_name(skill_name)?;
    let detected_tools = inventory_detected_tools_for_config(registry, config);
    let can_read_shared_skills = detected_tools
        .iter()
        .any(|tool| detected_tool_can_read_shared_skills(tool));
    let generic_skills = if can_read_shared_skills {
        list_skill_paths_in_dir_for_name(generic_dir, &skill_name)
    } else {
        Vec::new()
    };
    let metadata_names_by_tool = metadata_names_by_detected_tool(&detected_tools);
    let skill_paths_by_tool = targeted_skill_paths_by_detected_tool(&detected_tools, &skill_name);
    build_skill_entry_from_sources(
        &skill_name,
        &detected_tools,
        generic_dir,
        generic_skills,
        &skill_paths_by_tool,
        &metadata_names_by_tool,
    )
}

fn build_skill_entry_from_sources(
    skill_name: &str,
    detected_tools: &[DetectedTool],
    generic_dir: &Path,
    generic_skills: Vec<PathBuf>,
    skill_paths_by_tool: &HashMap<String, HashMap<String, Vec<PathBuf>>>,
    metadata_names_by_tool: &HashMap<String, HashSet<String>>,
) -> Result<SkillEntry, String> {
    let generic_skill = generic_skills.first();
    let (display_name, description) = if let Some(generic_skill) = generic_skill {
        read_skill_display_info(generic_skill, skill_name)
    } else {
        let mut fallback = (skill_name.to_string(), String::new());
        for tool in detected_tools {
            let Some(paths_by_name) = skill_paths_by_tool.get(&tool.id) else {
                continue;
            };
            let Some(tool_skill) = paths_by_name
                .get(skill_name)
                .and_then(|paths| paths.first())
            else {
                continue;
            };
            fallback = read_skill_display_info(tool_skill, skill_name);
            break;
        }
        fallback
    };

    let mut tool_statuses = Vec::with_capacity(detected_tools.len());
    for tool in detected_tools {
        let status_root = resolved_detected_tool_skills_dir(tool)
            .unwrap_or_else(|| generic_dir.join(".direct-read").join(&tool.id));
        let tool_skill_paths = skill_paths_by_tool
            .get(&tool.id)
            .and_then(|paths_by_name| paths_by_name.get(skill_name))
            .cloned()
            .unwrap_or_default();
        let shared_paths = if detected_tool_can_read_shared_skills(tool) {
            generic_skills.clone()
        } else {
            Vec::new()
        };
        let metadata_names = metadata_names_by_tool.get(&tool.id);
        let mut source_statuses = Vec::new();
        for tool_path in &tool_skill_paths {
            source_statuses.push(derive_status_for_tool_path(
                skill_name,
                tool,
                tool_path,
                generic_dir,
                generic_skill.map(|path| path.as_path()),
                metadata_names,
            )?);
        }
        for shared_path in &shared_paths {
            source_statuses.push(derive_status_for_shared_path(
                skill_name,
                tool,
                shared_path,
            )?);
        }

        if let Some(primary) = source_statuses.first().cloned() {
            tool_statuses.push(with_source_entries(primary, source_statuses));
        } else {
            tool_statuses.push(derive_tool_skill_status(
                skill_name,
                &tool.id,
                &tool.name,
                status_root.as_path(),
                detected_tool_can_read_shared_skills(tool),
                tool.allow_external_generic_symlink,
                generic_dir,
                generic_skill.map(|path| path.as_path()),
                metadata_names,
            )?);
        }
    }

    let path = if let Some(generic_skill) = generic_skill {
        Some(generic_skill.to_string_lossy().to_string())
    } else {
        tool_statuses.iter().find_map(|status| status.path.clone())
    };

    let package = if let Some(generic_skill) = generic_skill {
        crate::adapters::skills::detect_skill_package_info(generic_skill)
    } else {
        detected_tools.iter().find_map(|tool| {
            let skill_path = skill_paths_by_tool
                .get(&tool.id)
                .and_then(|paths_by_name| paths_by_name.get(skill_name))
                .and_then(|paths| paths.first())?;
            crate::adapters::skills::detect_skill_package_info(skill_path)
        })
    };

    Ok(SkillEntry {
        name: skill_name.to_string(),
        display_name,
        description,
        path,
        tool_statuses,
        package,
    })
}

pub(crate) fn list_skills_overview_domain(
    registry: &ToolRegistry,
) -> Result<Vec<SkillOverviewItem>, String> {
    let inventory = build_skill_inventory_for_current_config(registry)?;

    let mut overview: Vec<SkillOverviewItem> = inventory
        .skills
        .into_iter()
        .map(|entry| {
            let mut installed_in = Vec::new();
            for ts in &entry.tool_statuses {
                if matches!(
                    ts.status,
                    SkillStatus::VariantNotInstalled | SkillStatus::NoVariant
                ) {
                    continue;
                }
                let Some(path) = ts.path.clone() else {
                    continue;
                };

                let mode = if ts.symlink_target.is_some()
                    || matches!(
                        ts.status,
                        SkillStatus::VariantInstalledSymlink | SkillStatus::BrokenSymlink
                    ) {
                    "symlink".to_string()
                } else {
                    "copy".to_string()
                };

                installed_in.push(SkillPresence {
                    tool_id: ts.tool_id.clone(),
                    mode,
                    path,
                    target_path: ts.symlink_target.clone(),
                });
            }
            let path = if let Some(path) = entry.path.clone() {
                path
            } else if let Some(first) = installed_in.first() {
                first.path.clone()
            } else {
                String::new()
            };

            SkillOverviewItem {
                name: entry.name,
                description: entry.description,
                path,
                installed_in,
                package: entry.package,
            }
        })
        .collect();
    overview.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(overview)
}

pub(crate) fn read_skill_content_domain(skill_path: String) -> Option<SkillInfo> {
    let p = std::path::PathBuf::from(&skill_path);
    if !p.exists() || !p.is_dir() {
        return None;
    }
    let (content, files) = crate::adapters::skills::load_skill_full(&p);
    let skill_name = p.file_name()?.to_string_lossy().to_string();
    let has_scripts = p.join("scripts").is_dir();
    Some(SkillInfo {
        name: skill_name.clone(),
        display_name: skill_name,
        description: String::new(),
        path: skill_path,
        tool_id: String::new(),
        has_scripts,
        package: crate::adapters::skills::detect_skill_package_info(&p),
        files,
        skill_md_content: content,
    })
}

pub(crate) fn list_skill_files_domain(skill_path: String) -> Vec<SkillFileEntry> {
    let root = std::path::PathBuf::from(&skill_path);
    if !root.exists() || !root.is_dir() {
        return vec![];
    }
    let mut entries = Vec::new();
    collect_files_recursive(&root, &root, &mut entries);
    // Sort: directories first, then by path
    entries.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            return if a.is_dir {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        a.relative_path.cmp(&b.relative_path)
    });
    entries
}

pub(crate) fn read_skill_file_domain(
    skill_path: String,
    relative_path: String,
) -> Result<String, String> {
    let full_path = std::path::PathBuf::from(&skill_path).join(&relative_path);
    if !full_path.exists() || full_path.is_dir() {
        return Err("File not found".to_string());
    }
    std::fs::read_to_string(&full_path).map_err(|e| format!("Failed to read file: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{
        RuleSource, ToolCapability, ToolCapabilityAccess, ToolCapabilityActionEvidence,
        ToolCapabilityFormat, ToolCapabilityKind, ToolCapabilityScope,
        ToolCapabilitySourceConfidence, ToolCapabilitySupportingSource,
        ToolCapabilitySupportingSourceRole, ToolSourceDiagnosticState,
    };

    struct SkillCapabilityTestAdapter {
        access: ToolCapabilityAccess,
        dir: PathBuf,
        metadata_path: Option<PathBuf>,
    }

    impl ToolAdapter for SkillCapabilityTestAdapter {
        fn id(&self) -> &str {
            "skill-test"
        }
        fn name(&self) -> &str {
            "Skill Test"
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
            vec![ToolCapability {
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
                action_evidence: vec![],
            }]
        }
    }

    struct SharedSkillReadTestAdapter {
        shared_dir: PathBuf,
        tool_dir: Option<PathBuf>,
        shared_read: bool,
    }

    impl ToolAdapter for SharedSkillReadTestAdapter {
        fn id(&self) -> &str {
            "shared-read-test"
        }
        fn name(&self) -> &str {
            "Shared Read Test"
        }
        fn icon(&self) -> &str {
            "S"
        }
        fn config_dir(&self) -> PathBuf {
            self.tool_dir
                .clone()
                .unwrap_or_else(|| self.shared_dir.clone())
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
            self.tool_dir.clone()
        }
        fn capabilities(&self) -> Vec<ToolCapability> {
            let mut capabilities = vec![];
            if let Some(tool_dir) = &self.tool_dir {
                capabilities.push(ToolCapability {
                    id: "skills".to_string(),
                    kind: ToolCapabilityKind::Skill,
                    scope: ToolCapabilityScope::Tool,
                    access: ToolCapabilityAccess::Writable,
                    format: ToolCapabilityFormat::SkillDirectory,
                    source_path: tool_dir.to_string_lossy().to_string(),
                    label: "Skills".to_string(),
                    diagnostics: vec![ToolSourceDiagnosticState::Loaded],
                    source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
                    notes: String::new(),
                    source_kind: crate::adapters::ToolCapabilitySourceKind::FeatureSource,
                    primary_config_dir: None,
                    supporting_sources: vec![],
                    action_evidence: vec![],
                });
            }
            if self.shared_read {
                capabilities.push(ToolCapability {
                    id: "shared-skills".to_string(),
                    kind: ToolCapabilityKind::Skill,
                    scope: ToolCapabilityScope::Shared,
                    access: ToolCapabilityAccess::ReadOnly,
                    format: ToolCapabilityFormat::SkillDirectory,
                    source_path: self.shared_dir.to_string_lossy().to_string(),
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

    struct NamedSkillCapabilityTestAdapter {
        id: String,
        name: String,
        dir: PathBuf,
    }

    impl ToolAdapter for NamedSkillCapabilityTestAdapter {
        fn id(&self) -> &str {
            &self.id
        }
        fn name(&self) -> &str {
            &self.name
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
            vec![ToolCapability {
                id: "skills".to_string(),
                kind: ToolCapabilityKind::Skill,
                scope: ToolCapabilityScope::Tool,
                access: ToolCapabilityAccess::Readable,
                format: ToolCapabilityFormat::SkillDirectory,
                source_path: self.dir.to_string_lossy().to_string(),
                label: "Skills".to_string(),
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

    #[test]
    fn read_only_skill_capability_can_be_read_but_not_written() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = SkillCapabilityTestAdapter {
            access: ToolCapabilityAccess::ReadOnly,
            dir: tmp.path().join("skills"),
            metadata_path: None,
        };

        assert!(resolved_adapter_skills_dir(&adapter).is_some());
        assert!(
            resolved_adapter_skills_dir_for_action(&adapter, &ToolCapabilityAction::Install)
                .is_none()
        );
    }

    #[test]
    fn unsupported_skill_capability_is_not_read_or_written() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = SkillCapabilityTestAdapter {
            access: ToolCapabilityAccess::Unsupported,
            dir: tmp.path().join("skills"),
            metadata_path: None,
        };

        assert!(resolved_adapter_skills_dir(&adapter).is_none());
        assert!(
            resolved_adapter_skills_dir_for_action(&adapter, &ToolCapabilityAction::Install)
                .is_none()
        );
    }

    #[test]
    fn shared_direct_read_default_lists_shared_skill_without_tool_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let shared_dir = tmp.path().join("shared");
        let shared_skill = shared_dir.join("demo");
        std::fs::create_dir_all(&shared_skill).unwrap();
        std::fs::write(shared_skill.join("SKILL.md"), "# shared").unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(SharedSkillReadTestAdapter {
                shared_dir: shared_dir.clone(),
                tool_dir: None,
                shared_read: true,
            })]);

        let inventory = build_skill_inventory_with_generic_dir(&registry, &shared_dir).unwrap();
        let demo = inventory
            .skills
            .iter()
            .find(|entry| entry.name == "demo")
            .expect("shared skill should be visible");

        assert_eq!(demo.tool_statuses.len(), 1);
        assert_eq!(
            demo.tool_statuses[0].status,
            SkillStatus::VariantInstalledCopy
        );
        assert_eq!(demo.tool_statuses[0].path_origin, "generic");
        let expected_path = shared_skill.to_string_lossy().to_string();
        assert_eq!(
            demo.tool_statuses[0].path.as_deref(),
            Some(expected_path.as_str())
        );
    }

    #[test]
    fn windsurf_inventory_reads_shared_and_tool_skills_but_not_workspace_skills() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let windsurf_root = home.join(".codeium/windsurf");
        let windsurf_skills = windsurf_root.join("skills");
        let shared_dir = tmp.path().join("shared");
        let workspace_skills = tmp.path().join("workspace/.windsurf/skills");
        let shared_skill = shared_dir.join("shared-demo");
        let tool_skill = windsurf_skills.join("tool-demo");
        let workspace_skill = workspace_skills.join("workspace-demo");
        for skill in [&shared_skill, &tool_skill, &workspace_skill] {
            std::fs::create_dir_all(skill).unwrap();
            std::fs::write(skill.join("SKILL.md"), "# demo").unwrap();
        }
        std::fs::create_dir_all(&windsurf_root).unwrap();
        let adapter = crate::adapters::windsurf::create(&home);
        assert!(adapter_can_read_shared_skills(&*adapter));
        let config = crate::platform::config::default_config();
        assert_eq!(
            resolved_adapter_skills_dir_for_action_with_config(
                &*adapter,
                &ToolCapabilityAction::Copy,
                &config
            )
            .as_deref(),
            Some(windsurf_skills.as_path())
        );
        let registry = ToolRegistry::from_adapters_for_tests(vec![adapter]);

        let inventory = build_skill_inventory_with_generic_dir(&registry, &shared_dir).unwrap();
        let names: HashSet<_> = inventory
            .skills
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();

        assert!(names.contains("shared-demo"));
        assert!(names.contains("tool-demo"));
        assert!(!names.contains("workspace-demo"));
        let shared = inventory
            .skills
            .iter()
            .find(|entry| entry.name == "shared-demo")
            .expect("shared Skill should be visible");
        assert_eq!(shared.tool_statuses[0].tool_id, "windsurf");
        assert_eq!(shared.tool_statuses[0].path_origin, "generic");
        assert_eq!(
            shared.tool_statuses[0].path.as_deref(),
            Some(shared_skill.to_string_lossy().as_ref())
        );
        let tool = inventory
            .skills
            .iter()
            .find(|entry| entry.name == "tool-demo")
            .expect("tool Skill should be visible");
        assert_eq!(tool.tool_statuses[0].path_origin, "tool");
        assert_eq!(
            tool.tool_statuses[0].path.as_deref(),
            Some(tool_skill.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn shared_direct_read_override_and_reset_follow_effective_state() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter_without_default = SharedSkillReadTestAdapter {
            shared_dir: tmp.path().join("shared"),
            tool_dir: Some(tmp.path().join("skills")),
            shared_read: false,
        };
        let adapter_with_default = SharedSkillReadTestAdapter {
            shared_dir: tmp.path().join("shared"),
            tool_dir: Some(tmp.path().join("skills")),
            shared_read: true,
        };

        assert!(!adapter_can_read_shared_skills_with_override(
            &adapter_without_default,
            None
        ));
        assert!(adapter_can_read_shared_skills_with_override(
            &adapter_without_default,
            Some(true)
        ));
        assert!(adapter_can_read_shared_skills_with_override(
            &adapter_with_default,
            None
        ));
        assert!(!adapter_can_read_shared_skills_with_override(
            &adapter_with_default,
            Some(false)
        ));
    }

    #[test]
    fn initialized_inventory_excludes_disabled_tool_skills() {
        let tmp = tempfile::tempdir().unwrap();
        let enabled_dir = tmp.path().join("enabled-skills");
        let disabled_dir = tmp.path().join("disabled-skills");
        std::fs::create_dir_all(enabled_dir.join("enabled-skill")).unwrap();
        std::fs::write(
            enabled_dir.join("enabled-skill").join("SKILL.md"),
            "# enabled",
        )
        .unwrap();
        std::fs::create_dir_all(disabled_dir.join("disabled-skill")).unwrap();
        std::fs::write(
            disabled_dir.join("disabled-skill").join("SKILL.md"),
            "# disabled",
        )
        .unwrap();
        let registry = ToolRegistry::from_adapters_for_tests(vec![
            Box::new(NamedSkillCapabilityTestAdapter {
                id: "tool-a".to_string(),
                name: "Tool A".to_string(),
                dir: enabled_dir,
            }),
            Box::new(NamedSkillCapabilityTestAdapter {
                id: "tool-b".to_string(),
                name: "Tool B".to_string(),
                dir: disabled_dir,
            }),
        ]);
        let mut config = crate::platform::config::default_config();
        config.initialized = true;
        config.managed_tools = vec!["tool-a".to_string()];

        let inventory =
            build_skill_inventory_for_config_with_generic_dir(&registry, tmp.path(), &config)
                .unwrap();
        let names: HashSet<_> = inventory
            .skills
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();

        assert!(names.contains("enabled-skill"));
        assert!(!names.contains("disabled-skill"));
    }

    #[test]
    fn metadata_backed_skill_inventory_reports_content_metadata_drift() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let metadata_path = tmp.path().join("skill-config.json");
        std::fs::create_dir_all(skills_dir.join("content-only")).unwrap();
        std::fs::write(
            skills_dir.join("content-only").join("SKILL.md"),
            "# content",
        )
        .unwrap();
        std::fs::write(
            &metadata_path,
            r#"{"managedSkills":{"metadata-only":"marketplace"}}"#,
        )
        .unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(SkillCapabilityTestAdapter {
                access: ToolCapabilityAccess::Readable,
                dir: skills_dir.clone(),
                metadata_path: Some(metadata_path),
            })]);

        let inventory = build_skill_inventory(&registry).unwrap();
        let content_only = inventory
            .skills
            .iter()
            .find(|entry| entry.name == "content-only")
            .expect("content-only skill should be listed");
        let metadata_only = inventory
            .skills
            .iter()
            .find(|entry| entry.name == "metadata-only")
            .expect("metadata-only skill should be listed");

        assert_eq!(
            content_only.tool_statuses[0].status,
            SkillStatus::VariantDrifted
        );
        assert_eq!(
            metadata_only.tool_statuses[0].status,
            SkillStatus::VariantDrifted
        );
    }

    #[test]
    fn compound_skill_source_is_not_shared_install_deploy_target() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = SkillCapabilityTestAdapter {
            access: ToolCapabilityAccess::Writable,
            dir: tmp.path().join("skills"),
            metadata_path: Some(tmp.path().join("skill-config.json")),
        };

        assert!(!adapter_has_shared_skill_deploy_target(&adapter));
        assert!(resolved_adapter_shared_skill_link_dir(&adapter).is_none());
    }

    #[test]
    fn unsupported_shared_skill_direct_read_still_allows_tool_directory_deploy() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = crate::adapters::kiro::create(tmp.path());

        assert!(!adapter_can_read_shared_skills(&*adapter));
        assert!(adapter_has_shared_skill_deploy_target(&*adapter));
        assert_eq!(
            resolved_adapter_shared_skill_link_dir(&*adapter).as_deref(),
            Some(tmp.path().join(".kiro/skills").as_path())
        );
        assert_eq!(
            resolved_adapter_skills_dir_for_action(&*adapter, &ToolCapabilityAction::Copy)
                .as_deref(),
            Some(tmp.path().join(".kiro/skills").as_path())
        );
    }

    #[test]
    fn trae_cn_default_shared_direct_read_matches_installed_tool_default() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = crate::adapters::trae_cn::create(tmp.path());

        assert!(adapter_can_read_shared_skills(&*adapter));
        assert!(!adapter_has_shared_skill_deploy_target(&*adapter));
        assert!(!adapter_can_read_shared_skills_with_override(
            &*adapter,
            Some(false)
        ));
    }

    #[test]
    fn build_skill_inventory_keeps_nested_skill_paths_and_skips_skill_subtrees() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        std::fs::create_dir_all(skills_dir.join("devops").join("webhook-subscriptions")).unwrap();
        std::fs::create_dir_all(skills_dir.join("packaged").join("inner")).unwrap();
        std::fs::write(
            skills_dir
                .join("devops")
                .join("webhook-subscriptions")
                .join("SKILL.md"),
            "---\nname: Webhook Subscriptions\n---",
        )
        .unwrap();
        std::fs::write(
            skills_dir.join("packaged").join("SKILL.md"),
            "---\nname: Packaged\n---",
        )
        .unwrap();
        std::fs::write(
            skills_dir.join("packaged").join("inner").join("SKILL.md"),
            "---\nname: Inner\n---",
        )
        .unwrap();

        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(SkillCapabilityTestAdapter {
                access: ToolCapabilityAccess::Readable,
                dir: skills_dir.clone(),
                metadata_path: None,
            })]);

        let inventory = build_skill_inventory(&registry).unwrap();
        let names: HashSet<_> = inventory
            .skills
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();

        assert!(names.contains("webhook-subscriptions"));
        assert!(names.contains("packaged"));
        assert!(!names.contains("devops"));
        assert!(!names.contains("inner"));

        let webhook = inventory
            .skills
            .iter()
            .find(|entry| entry.name == "webhook-subscriptions")
            .expect("nested skill should be listed");
        assert!(webhook
            .path
            .as_deref()
            .unwrap_or("")
            .ends_with("devops/webhook-subscriptions"));
    }

    #[test]
    fn build_skill_inventory_marks_same_tool_duplicate_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let top_level = skills_dir.join("executing-plans");
        let nested = skills_dir
            .join("superpowers")
            .join("skills")
            .join("executing-plans");
        std::fs::create_dir_all(&top_level).unwrap();
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(top_level.join("SKILL.md"), "# top").unwrap();
        std::fs::write(nested.join("SKILL.md"), "# nested").unwrap();

        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(SkillCapabilityTestAdapter {
                access: ToolCapabilityAccess::Readable,
                dir: skills_dir,
                metadata_path: None,
            })]);

        let inventory = build_skill_inventory(&registry).unwrap();
        let skill = inventory
            .skills
            .iter()
            .find(|entry| entry.name == "executing-plans")
            .expect("duplicate skill should be listed once");

        assert_eq!(skill.tool_statuses.len(), 1);
        assert_eq!(
            skill.tool_statuses[0].abnormal_state.as_deref(),
            Some("duplicate_sources")
        );
        assert_eq!(skill.tool_statuses[0].sources.len(), 2);
        let source_paths: HashSet<_> = skill.tool_statuses[0]
            .sources
            .iter()
            .map(|source| source.path.as_str())
            .collect();
        assert!(source_paths.contains(top_level.to_string_lossy().as_ref()));
        assert!(source_paths.contains(nested.to_string_lossy().as_ref()));
    }

    #[test]
    fn build_single_skill_inventory_entry_matches_full_projection_for_one_skill() {
        let tmp = tempfile::tempdir().unwrap();
        let shared_dir = tmp.path().join("shared");
        let tool_dir = tmp.path().join("tool-skills");
        let shared_skill = shared_dir.join("demo");
        let tool_skill = tool_dir.join("nested").join("demo");
        let unrelated_skill = tool_dir.join("unrelated");
        std::fs::create_dir_all(&shared_skill).unwrap();
        std::fs::create_dir_all(&tool_skill).unwrap();
        std::fs::create_dir_all(&unrelated_skill).unwrap();
        std::fs::write(shared_skill.join("SKILL.md"), "# shared").unwrap();
        std::fs::write(tool_skill.join("SKILL.md"), "# tool").unwrap();
        std::fs::write(unrelated_skill.join("SKILL.md"), "# unrelated").unwrap();

        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(SkillCapabilityTestAdapter {
                access: ToolCapabilityAccess::Readable,
                dir: tool_dir,
                metadata_path: None,
            })]);
        let config = crate::platform::config::default_config();
        let full =
            build_skill_inventory_for_config_with_generic_dir(&registry, &shared_dir, &config)
                .unwrap();
        let full_demo = full
            .skills
            .iter()
            .find(|entry| entry.name == "demo")
            .expect("full inventory should include demo");
        let targeted = build_skill_inventory_entry_for_config_with_generic_dir(
            &registry,
            &shared_dir,
            &config,
            "demo",
        )
        .unwrap();

        assert_eq!(targeted.name, full_demo.name);
        assert_eq!(targeted.path, full_demo.path);
        assert_eq!(targeted.tool_statuses.len(), full_demo.tool_statuses.len());
        assert_eq!(
            targeted.tool_statuses[0].abnormal_state,
            full_demo.tool_statuses[0].abnormal_state
        );
        assert_eq!(
            targeted.tool_statuses[0].sources.len(),
            full_demo.tool_statuses[0].sources.len()
        );
        assert!(full.skills.iter().any(|entry| entry.name == "unrelated"));
        assert_ne!(targeted.name, "unrelated");
    }

    #[test]
    fn build_skill_inventory_preserves_shared_and_tool_duplicate_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let shared_dir = tmp.path().join("shared");
        let tool_dir = tmp.path().join("tool-skills");
        let shared_skill = shared_dir.join("demo");
        let tool_skill = tool_dir.join("demo");
        std::fs::create_dir_all(&shared_skill).unwrap();
        std::fs::create_dir_all(&tool_skill).unwrap();
        std::fs::write(shared_skill.join("SKILL.md"), "# shared").unwrap();
        std::fs::write(tool_skill.join("SKILL.md"), "# tool").unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(SharedSkillReadTestAdapter {
                shared_dir: shared_dir.clone(),
                tool_dir: Some(tool_dir),
                shared_read: true,
            })]);

        let inventory = build_skill_inventory_with_generic_dir(&registry, &shared_dir).unwrap();
        let skill = inventory
            .skills
            .iter()
            .find(|entry| entry.name == "demo")
            .expect("skill should be listed once");
        let status = &skill.tool_statuses[0];

        assert_eq!(status.abnormal_state.as_deref(), Some("duplicate_sources"));
        assert_eq!(status.sources.len(), 2);
        assert!(status
            .sources
            .iter()
            .any(|source| source.path_origin == "generic"
                && source.path == shared_skill.to_string_lossy().as_ref()));
        assert!(status
            .sources
            .iter()
            .any(|source| source.path_origin == "tool"
                && source.path == tool_skill.to_string_lossy().as_ref()));
    }

    #[test]
    fn non_compound_tool_skill_inventory_ignores_adjacent_metadata_file() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let metadata_path = tmp.path().join("skill-config.json");
        std::fs::create_dir_all(skills_dir.join("real-skill")).unwrap();
        std::fs::write(skills_dir.join("real-skill").join("SKILL.md"), "# real").unwrap();
        std::fs::write(
            &metadata_path,
            r#"{"managedSkills":{"metadata-only":"marketplace"}}"#,
        )
        .unwrap();
        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(SkillCapabilityTestAdapter {
                access: ToolCapabilityAccess::Readable,
                dir: skills_dir.clone(),
                metadata_path: None,
            })]);

        let inventory = build_skill_inventory(&registry).unwrap();
        let names: HashSet<_> = inventory
            .skills
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();

        assert!(names.contains("real-skill"));
        assert!(!names.contains("metadata-only"));
    }

    #[cfg(unix)]
    #[test]
    fn build_skill_inventory_keeps_broken_symlink_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::os::unix::fs::symlink(
            tmp.path().join("missing-target"),
            skills_dir.join("broken-link"),
        )
        .unwrap();

        let registry =
            ToolRegistry::from_adapters_for_tests(vec![Box::new(SkillCapabilityTestAdapter {
                access: ToolCapabilityAccess::Readable,
                dir: skills_dir,
                metadata_path: None,
            })]);

        let inventory = build_skill_inventory(&registry).unwrap();
        let broken = inventory
            .skills
            .iter()
            .find(|entry| entry.name == "broken-link")
            .expect("broken symlink should remain visible");

        assert_eq!(broken.tool_statuses[0].status, SkillStatus::BrokenSymlink);
        assert!(broken.tool_statuses[0]
            .path
            .as_deref()
            .unwrap_or("")
            .ends_with("skills/broken-link"));
    }

    #[test]
    fn derive_tool_skill_status_uses_nested_generic_skill_path() {
        let tmp = tempfile::tempdir().unwrap();
        let tool_skills_dir = tmp.path().join("tool-skills");
        let generic_dir = tmp.path().join("shared");
        let nested_source = generic_dir.join("devops").join("webhook-subscriptions");
        std::fs::create_dir_all(&tool_skills_dir).unwrap();
        std::fs::create_dir_all(&nested_source).unwrap();
        std::fs::write(nested_source.join("SKILL.md"), "# nested").unwrap();

        let status = derive_tool_skill_status(
            "webhook-subscriptions",
            "skill-test",
            "Skill Test",
            &tool_skills_dir,
            true,
            false,
            &generic_dir,
            Some(nested_source.as_path()),
            None,
        )
        .unwrap();

        assert_eq!(status.status, SkillStatus::VariantInstalledCopy);
        assert_eq!(status.path_origin, "generic");
        let expected_path = nested_source.to_string_lossy().to_string();
        assert_eq!(status.path.as_deref(), Some(expected_path.as_str()));
    }

    #[test]
    fn derive_tool_skill_status_keeps_shared_link_as_tool_source() {
        let tmp = tempfile::tempdir().unwrap();
        let tool_skills_dir = tmp.path().join("tool-skills");
        let generic_dir = tmp.path().join("shared");
        let generic_skill = generic_dir.join("demo");
        let tool_link = tool_skills_dir.join("demo");
        std::fs::create_dir_all(&tool_skills_dir).unwrap();
        std::fs::create_dir_all(&generic_skill).unwrap();
        std::fs::write(generic_skill.join("SKILL.md"), "# shared").unwrap();
        std::os::unix::fs::symlink(&generic_skill, &tool_link).unwrap();

        let status = derive_tool_skill_status(
            "demo",
            "skill-test",
            "Skill Test",
            &tool_skills_dir,
            true,
            false,
            &generic_dir,
            Some(generic_skill.as_path()),
            None,
        )
        .unwrap();

        assert_eq!(status.status, SkillStatus::VariantInstalledSymlink);
        assert_eq!(status.path_origin, "tool");
        assert_eq!(
            status.path.as_deref(),
            Some(tool_link.to_string_lossy().as_ref())
        );
        assert_eq!(
            status.symlink_target.as_deref(),
            Some(generic_skill.to_string_lossy().as_ref())
        );
    }
}
