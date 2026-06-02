// Purpose: Own local Skill source path helpers.
use super::*;

pub(crate) fn canonical_skill_tool_id(tool_id: &str) -> String {
    crate::platform::tool_catalog::normalization::canonical_tool_id(tool_id)
}

pub(crate) fn skill_path_resolves_to(candidate: &Path, target: &Path) -> bool {
    if candidate == target {
        return true;
    }
    let canonical_candidate = fs::canonicalize(candidate).ok();
    let canonical_target = fs::canonicalize(target).ok();
    canonical_candidate
        .as_ref()
        .zip(canonical_target.as_ref())
        .map(|(candidate, target)| candidate == target || candidate.starts_with(target))
        .unwrap_or(false)
}
