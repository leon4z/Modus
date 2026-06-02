// Purpose: Shared Skill execution guards and rollback helpers.

use super::*;

#[derive(Clone)]
enum PathSnapshotKind {
    Missing,
    Directory { backup_path: PathBuf },
    File { backup_path: PathBuf },
    Symlink { target: PathBuf },
}

#[derive(Clone)]
struct PathSnapshot {
    original_path: PathBuf,
    kind: PathSnapshotKind,
}

fn capture_path_snapshot(
    backup_root: &Path,
    original_path: &Path,
    label: &str,
) -> Result<PathSnapshot, String> {
    let metadata = match fs::symlink_metadata(original_path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PathSnapshot {
                original_path: original_path.to_path_buf(),
                kind: PathSnapshotKind::Missing,
            })
        }
        Err(err) => return Err(err.to_string()),
    };

    if metadata.file_type().is_symlink() {
        return Ok(PathSnapshot {
            original_path: original_path.to_path_buf(),
            kind: PathSnapshotKind::Symlink {
                target: fs::read_link(original_path).map_err(|e| e.to_string())?,
            },
        });
    }

    let backup_path = backup_root.join(label);
    remove_if_exists(&backup_path)?;
    if metadata.is_dir() {
        copy_dir_recursive(original_path, &backup_path)?;
        Ok(PathSnapshot {
            original_path: original_path.to_path_buf(),
            kind: PathSnapshotKind::Directory { backup_path },
        })
    } else {
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(original_path, &backup_path).map_err(|e| e.to_string())?;
        Ok(PathSnapshot {
            original_path: original_path.to_path_buf(),
            kind: PathSnapshotKind::File { backup_path },
        })
    }
}

fn restore_path_snapshot(snapshot: &PathSnapshot) -> Result<(), String> {
    remove_if_exists(&snapshot.original_path)?;
    match &snapshot.kind {
        PathSnapshotKind::Missing => Ok(()),
        PathSnapshotKind::Directory { backup_path } => {
            copy_dir_recursive(backup_path, &snapshot.original_path)
        }
        PathSnapshotKind::File { backup_path } => {
            if let Some(parent) = snapshot.original_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::copy(backup_path, &snapshot.original_path)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        PathSnapshotKind::Symlink { target } => {
            if let Some(parent) = snapshot.original_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::os::unix::fs::symlink(target, &snapshot.original_path).map_err(|e| e.to_string())
        }
    }
}

fn rollback_path_snapshots(snapshots: &[PathSnapshot]) -> Result<(), String> {
    let mut errors = Vec::new();
    for snapshot in snapshots.iter().rev() {
        if let Err(err) = restore_path_snapshot(snapshot) {
            errors.push(format!("{}: {}", snapshot.original_path.display(), err));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join(" | "))
    }
}

pub(crate) fn with_path_snapshots<T, F>(paths: Vec<PathBuf>, execute: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    let mut deduped = Vec::new();
    let mut seen = HashSet::new();
    for path in paths {
        if seen.insert(path.clone()) {
            deduped.push(path);
        }
    }

    let backup_root = std::env::temp_dir().join(format!(
        "modus-snapshot-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    ));
    remove_if_exists(&backup_root)?;
    fs::create_dir_all(&backup_root).map_err(|e| e.to_string())?;
    let mut snapshots = Vec::with_capacity(deduped.len());
    for (index, path) in deduped.iter().enumerate() {
        snapshots.push(capture_path_snapshot(
            &backup_root,
            path,
            &format!("snapshot-{}", index),
        )?);
    }

    let result = match execute() {
        Ok(result) => Ok(result),
        Err(error) => {
            if let Err(rollback_error) = rollback_path_snapshots(&snapshots) {
                Err(format!("{}; rollback failed: {}", error, rollback_error))
            } else {
                Err(error)
            }
        }
    };
    let _ = remove_if_exists(&backup_root);
    result
}
