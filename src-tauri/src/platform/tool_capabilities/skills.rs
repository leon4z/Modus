// Purpose: Shared skill and configuration data shapes exposed by tool adapters.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillPackageMember {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub kind: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillPackageInfo {
    pub is_package: bool,
    pub member_count: usize,
    pub members: Vec<SkillPackageMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub path: String,
    pub tool_id: String,
    pub has_scripts: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<SkillPackageInfo>,
    pub files: Vec<String>,
    pub skill_md_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
    pub source_path: String,
    pub editable: bool,
    pub category: String,
}

/// Parse YAML frontmatter from SKILL.md content
/// Expected format:
/// ---
/// name: skill-name
/// description: what this skill does
/// ---
pub fn parse_skill_frontmatter(content: &str) -> (String, String) {
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        return (String::new(), String::new());
    }

    let after_first = &trimmed[3..];
    if let Some(end_pos) = after_first.find("---") {
        let yaml_block = &after_first[..end_pos];
        let mut name = String::new();
        let mut description = String::new();
        let mut in_multiline_desc = false;

        for line in yaml_block.lines() {
            let trimmed_line = line.trim();

            if in_multiline_desc {
                if line.starts_with(' ') || line.starts_with('\t') {
                    if !trimmed_line.is_empty() {
                        if !description.is_empty() {
                            description.push(' ');
                        }
                        description.push_str(trimmed_line);
                    }
                    continue;
                } else {
                    in_multiline_desc = false;
                }
            }

            if let Some(val) = trimmed_line.strip_prefix("name:") {
                name = val.trim().trim_matches('"').trim_matches('\'').to_string();
            } else if let Some(val) = trimmed_line.strip_prefix("description:") {
                let v = val.trim();
                if v == ">" || v == ">-" || v == "|" || v == "|-" {
                    in_multiline_desc = true;
                    description = String::new();
                } else {
                    description = v.trim_matches('"').trim_matches('\'').to_string();
                }
            }
        }
        (name, description)
    } else {
        (String::new(), String::new())
    }
}

pub fn parse_skill_frontmatter_field(content: &str, field: &str) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        return String::new();
    }
    let after_first = &trimmed[3..];
    let Some(end_pos) = after_first.find("---") else {
        return String::new();
    };
    let yaml_block = &after_first[..end_pos];
    let prefix = format!("{}:", field);
    for line in yaml_block.lines() {
        let trimmed_line = line.trim();
        if let Some(value) = trimmed_line.strip_prefix(&prefix) {
            return value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
        }
    }
    String::new()
}

fn read_member_display_info(skill_md: &Path, fallback_name: &str) -> (String, String) {
    let Ok(content) = fs::read_to_string(skill_md) else {
        return (fallback_name.to_string(), String::new());
    };
    let (name, description) = parse_skill_frontmatter(&content);
    if name.is_empty() {
        (fallback_name.to_string(), description)
    } else {
        (name, description)
    }
}

fn package_member_kind(relative_path: &Path) -> String {
    let first = relative_path
        .components()
        .next()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .unwrap_or_default();
    match first.as_str() {
        "agents" => "agent",
        "commands" => "command",
        "workflows" => "workflow",
        "references" => "reference",
        _ => "skill",
    }
    .to_string()
}

fn collect_package_member_dirs(root: &Path, dir: &Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with('.'))
        {
            continue;
        }
        if path.join("SKILL.md").exists() && path != root {
            output.push(path.clone());
        }
        collect_package_member_dirs(root, &path, output);
    }
}

/// Detect package membership for display context only. This intentionally does
/// not validate package health; unusual child layout simply yields fewer
/// display members.
pub fn detect_skill_package_info(skill_root: &Path) -> Option<SkillPackageInfo> {
    if !skill_root.is_dir() || !skill_root.join("SKILL.md").exists() {
        return None;
    }

    let mut member_dirs = Vec::new();
    for dir_name in ["commands", "agents", "workflows", "references"] {
        let member_root = skill_root.join(dir_name);
        if member_root.is_dir() {
            collect_package_member_dirs(skill_root, &member_root, &mut member_dirs);
        }
    }

    let Ok(entries) = fs::read_dir(skill_root) else {
        return None;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.')
            || matches!(
                name.as_str(),
                "commands" | "agents" | "workflows" | "references" | "scripts"
            )
        {
            continue;
        }
        if path.join("SKILL.md").exists() {
            member_dirs.push(path);
        }
    }

    member_dirs.sort();
    member_dirs.dedup();
    if member_dirs.is_empty() {
        return None;
    }

    let mut members = member_dirs
        .into_iter()
        .map(|path| {
            let relative_path = path
                .strip_prefix(skill_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            let fallback = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| relative_path.clone());
            let (display_name, description) =
                read_member_display_info(&path.join("SKILL.md"), &fallback);
            SkillPackageMember {
                name: fallback,
                display_name,
                description,
                kind: package_member_kind(Path::new(&relative_path)),
                relative_path,
            }
        })
        .collect::<Vec<_>>();
    members.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });

    Some(SkillPackageInfo {
        is_package: true,
        member_count: members.len(),
        members,
    })
}

fn collect_skill_infos(dir: &Path, tool_id: &str, skills: &mut Vec<SkillInfo>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let skill_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Skip hidden directories (e.g. .git, .venv)
        if skill_name.starts_with('.') {
            continue;
        }

        let is_symlink = entry
            .file_type()
            .map(|file_type| file_type.is_symlink())
            .unwrap_or(false);
        let skill_md = path.join("SKILL.md");
        if skill_md.exists() {
            // Only directories with their own SKILL.md are skills. Once a skill
            // root is found, its nested directories stay inside that skill.
            let (parsed_name, description) = read_frontmatter_fast(&skill_md);
            let display_name = if parsed_name.is_empty() {
                skill_name.clone()
            } else {
                parsed_name
            };
            let has_scripts = path.join("scripts").is_dir();
            let package = detect_skill_package_info(&path);

            skills.push(SkillInfo {
                name: skill_name,
                display_name,
                description,
                path: path.to_string_lossy().to_string(),
                tool_id: tool_id.to_string(),
                has_scripts,
                package,
                files: vec![],                   // loaded on demand
                skill_md_content: String::new(), // loaded on demand
            });
            continue;
        }

        if !is_symlink {
            collect_skill_infos(&path, tool_id, skills);
        }
    }
}

/// Scan a skills directory and return lightweight SkillInfo for each skill found
/// Only reads name/description from frontmatter — skips full content and file listing for speed
pub fn scan_skills_dir(dir: &Path, tool_id: &str) -> Vec<SkillInfo> {
    let mut skills = vec![];
    if !dir.exists() {
        return skills;
    }
    collect_skill_infos(dir, tool_id, &mut skills);

    skills.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    skills
}

/// Read only YAML frontmatter from SKILL.md without loading entire file
fn read_frontmatter_fast(path: &Path) -> (String, String) {
    use std::io::{BufRead, BufReader};
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return (String::new(), String::new()),
    };
    let reader = BufReader::new(file);
    let mut in_frontmatter = false;
    let mut in_multiline_desc = false;
    let mut name = String::new();
    let mut description = String::new();

    for line in reader.lines().take(30) {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed == "---" {
            if in_frontmatter {
                break; // end of frontmatter
            }
            in_frontmatter = true;
            continue;
        }
        if !in_frontmatter {
            continue;
        }

        // If we're reading continuation lines of a multi-line description
        if in_multiline_desc {
            // Continuation lines must start with whitespace (indented)
            if line.starts_with(' ') || line.starts_with('\t') {
                if !trimmed.is_empty() {
                    if !description.is_empty() {
                        description.push(' ');
                    }
                    description.push_str(trimmed);
                }
                continue;
            } else {
                // Non-indented line means multi-line block ended
                in_multiline_desc = false;
                // Fall through to parse this line as a normal key
            }
        }

        if let Some(val) = trimmed.strip_prefix("name:") {
            name = val.trim().trim_matches('"').trim_matches('\'').to_string();
        } else if let Some(val) = trimmed.strip_prefix("description:") {
            let v = val.trim();
            // Check for YAML block scalar indicators
            if v == ">" || v == ">-" || v == "|" || v == "|-" {
                in_multiline_desc = true;
                description = String::new();
            } else {
                description = v.trim_matches('"').trim_matches('\'').to_string();
            }
        }
    }
    (name, description)
}

/// Load full skill details (content + file list) — called on demand
pub fn load_skill_full(skill_path: &Path) -> (String, Vec<String>) {
    let skill_md = skill_path.join("SKILL.md");
    let content = if skill_md.exists() {
        fs::read_to_string(&skill_md).unwrap_or_default()
    } else {
        let readme = skill_path.join("README.md");
        if readme.exists() {
            fs::read_to_string(&readme).unwrap_or_default()
        } else {
            String::new()
        }
    };
    let files = list_files_recursive(skill_path, skill_path);
    (content, files)
}

fn list_files_recursive(dir: &Path, base: &Path) -> Vec<String> {
    let mut files = vec![];
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let relative = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            if path.is_dir() {
                files.extend(list_files_recursive(&path, base));
            } else {
                files.push(relative);
            }
        }
    }
    files.sort();
    files
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpServerActivationState {
    Enabled,
    Disabled,
    Unknown,
}

impl Default for McpServerActivationState {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub server_type: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env_keys: Vec<String>,
    pub url: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub activation_state: McpServerActivationState,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // === parse_skill_frontmatter tests ===

    #[test]
    fn standard_frontmatter() {
        let (name, desc) =
            parse_skill_frontmatter("---\nname: foo\ndescription: bar\n---\n# Content");
        assert_eq!(name, "foo");
        assert_eq!(desc, "bar");
    }

    #[test]
    fn quoted_name() {
        let (name, _) = parse_skill_frontmatter("---\nname: \"my skill\"\n---");
        assert_eq!(name, "my skill");
    }

    #[test]
    fn single_quoted_name() {
        let (name, _) = parse_skill_frontmatter("---\nname: 'my skill'\n---");
        assert_eq!(name, "my skill");
    }

    #[test]
    fn block_scalar_folded() {
        let (_, desc) = parse_skill_frontmatter("---\ndescription: >\n  line1\n  line2\n---");
        assert_eq!(desc, "line1 line2");
    }

    #[test]
    fn block_scalar_folded_strip() {
        let (_, desc) = parse_skill_frontmatter("---\ndescription: >-\n  line1\n  line2\n---");
        assert_eq!(desc, "line1 line2");
    }

    #[test]
    fn block_scalar_literal() {
        let (_, desc) = parse_skill_frontmatter("---\ndescription: |\n  line1\n  line2\n---");
        assert_eq!(desc, "line1 line2");
    }

    #[test]
    fn no_frontmatter() {
        let (name, desc) = parse_skill_frontmatter("# Hello World");
        assert_eq!(name, "");
        assert_eq!(desc, "");
    }

    #[test]
    fn empty_content() {
        let (name, desc) = parse_skill_frontmatter("");
        assert_eq!(name, "");
        assert_eq!(desc, "");
    }

    #[test]
    fn unclosed_frontmatter() {
        let (name, desc) = parse_skill_frontmatter("---\nname: foo\n");
        assert_eq!(name, "");
        assert_eq!(desc, "");
    }

    #[test]
    fn only_name_no_desc() {
        let (name, desc) = parse_skill_frontmatter("---\nname: foo\n---");
        assert_eq!(name, "foo");
        assert_eq!(desc, "");
    }

    #[test]
    fn multiline_desc_then_name() {
        let (name, desc) =
            parse_skill_frontmatter("---\ndescription: >\n  some text\nname: bar\n---");
        assert_eq!(name, "bar");
        assert_eq!(desc, "some text");
    }

    // === scan_skills_dir tests ===

    #[test]
    fn scan_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = scan_skills_dir(dir.path(), "test");
        assert!(result.is_empty());
    }

    #[test]
    fn scan_nonexistent_dir() {
        let result = scan_skills_dir(Path::new("/nonexistent/path/xyz"), "test");
        assert!(result.is_empty());
    }

    #[test]
    fn scan_with_skills() {
        let dir = tempfile::tempdir().unwrap();
        // Create two skill dirs
        let skill_a = dir.path().join("skill-a");
        let skill_b = dir.path().join("skill-b");
        fs::create_dir(&skill_a).unwrap();
        fs::create_dir(&skill_b).unwrap();
        fs::write(
            skill_a.join("SKILL.md"),
            "---\nname: Alpha\ndescription: first\n---",
        )
        .unwrap();
        fs::write(
            skill_b.join("SKILL.md"),
            "---\nname: Beta\ndescription: second\n---",
        )
        .unwrap();

        let result = scan_skills_dir(dir.path(), "test-tool");
        assert_eq!(result.len(), 2);
        // name is directory id; display_name comes from frontmatter
        assert_eq!(result[0].name, "skill-a");
        assert_eq!(result[0].display_name, "Alpha");
        assert_eq!(result[1].name, "skill-b");
        assert_eq!(result[1].display_name, "Beta");
        assert_eq!(result[0].tool_id, "test-tool");
    }

    #[test]
    fn scan_skips_files() {
        let dir = tempfile::tempdir().unwrap();
        // Create a regular file (not a dir) — should be skipped
        fs::write(dir.path().join("not-a-skill.txt"), "hello").unwrap();
        let result = scan_skills_dir(dir.path(), "test");
        assert!(result.is_empty());
    }

    #[test]
    fn scan_detects_scripts_dir() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("my-skill");
        fs::create_dir(&skill).unwrap();
        fs::create_dir(skill.join("scripts")).unwrap();
        fs::write(skill.join("SKILL.md"), "---\nname: test\n---").unwrap();

        let result = scan_skills_dir(dir.path(), "t");
        assert_eq!(result.len(), 1);
        assert!(result[0].has_scripts);
    }

    #[test]
    fn scan_detects_package_members_without_flattening_children() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("gsd");
        fs::create_dir_all(skill.join("commands/plan")).unwrap();
        fs::create_dir_all(skill.join("agents/executor")).unwrap();
        fs::write(skill.join("SKILL.md"), "---\nname: GSD\n---").unwrap();
        fs::write(
            skill.join("commands/plan/SKILL.md"),
            "---\nname: Plan\ndescription: plan work\n---",
        )
        .unwrap();
        fs::write(
            skill.join("agents/executor/SKILL.md"),
            "---\nname: Executor\n---",
        )
        .unwrap();

        let result = scan_skills_dir(dir.path(), "test");
        assert_eq!(result.len(), 1);
        let package = result[0].package.as_ref().expect("package info");
        assert!(package.is_package);
        assert_eq!(package.member_count, 2);
        assert!(package
            .members
            .iter()
            .any(|member| member.kind == "command" && member.display_name == "Plan"));
        assert!(package
            .members
            .iter()
            .any(|member| member.kind == "agent" && member.display_name == "Executor"));
    }

    #[test]
    fn scan_no_skill_md_is_not_reported() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("fallback-name");
        fs::create_dir(&skill).unwrap();
        // No SKILL.md

        let result = scan_skills_dir(dir.path(), "t");
        assert!(result.is_empty());
    }

    #[test]
    fn scan_finds_nested_skills_and_stops_at_skill_roots() {
        let dir = tempfile::tempdir().unwrap();

        let nested_skill = dir.path().join("devops").join("webhook-subscriptions");
        fs::create_dir_all(&nested_skill).unwrap();
        fs::write(
            nested_skill.join("SKILL.md"),
            "---\nname: Webhook Subscriptions\ndescription: nested\n---",
        )
        .unwrap();

        let package_root = dir.path().join("packaged");
        fs::create_dir_all(package_root.join("inner")).unwrap();
        fs::write(package_root.join("SKILL.md"), "---\nname: Packaged\n---").unwrap();
        fs::write(
            package_root.join("inner").join("SKILL.md"),
            "---\nname: Inner\n---",
        )
        .unwrap();

        let result = scan_skills_dir(dir.path(), "test");
        let names: std::collections::HashSet<_> =
            result.iter().map(|skill| skill.name.as_str()).collect();

        assert!(names.contains("webhook-subscriptions"));
        assert!(names.contains("packaged"));
        assert!(!names.contains("devops"));
        assert!(!names.contains("inner"));
    }
}
