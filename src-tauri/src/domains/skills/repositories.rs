// Purpose: Low-level filesystem primitives reused across Skill directory workflows.
//
// Repository helpers own the actual mutation against directories, files, and symlinks.
// They contain no policy decisions and no plan/preview construction.

use super::types::SkillFileEntry;
use std::path::Path;

/// Recursive directory copy. Creates `dst` if missing and copies every entry under
/// `src` into it. Returns the first IO error stringified.
pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    let entries = std::fs::read_dir(src).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Delete `path` whether it is a regular directory, file, or symlink. Returns a
/// human-readable error string on failure.
pub(crate) fn remove_skill_path(path: &Path) -> Result<(), String> {
    if path.is_dir() && !path.is_symlink() {
        std::fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to delete {}: {}", path.display(), e))
    } else {
        std::fs::remove_file(path)
            .map_err(|e| format!("Failed to delete {}: {}", path.display(), e))
    }
}

/// Delete `path` only if it currently exists (as a regular entry or symlink).
pub(crate) fn remove_if_exists(path: &Path) -> Result<(), String> {
    if path.exists() || path.is_symlink() {
        remove_skill_path(path)?;
    }
    Ok(())
}

/// Heuristic check for a usable Skill directory: must exist as a directory and
/// contain a `SKILL.md` file at its root.
pub(crate) fn is_valid_skill_dir(path: &Path) -> bool {
    path.exists() && path.is_dir() && path.join("SKILL.md").exists()
}

/// Walk the directory tree rooted at `root` and append a `SkillFileEntry` for
/// each entry encountered. Paths in the result are relative to `root`.
pub(crate) fn collect_files_recursive(root: &Path, dir: &Path, entries: &mut Vec<SkillFileEntry>) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let is_dir = path.is_dir();
        entries.push(SkillFileEntry {
            relative_path: rel.clone(),
            is_dir,
        });
        if is_dir {
            collect_files_recursive(root, &path, entries);
        }
    }
}
