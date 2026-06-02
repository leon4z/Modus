//! Generic dev tool adapter for testing.
//!
//! Provides a minimal ToolAdapter implementation with configurable
//! id, name, icon, and base directory. Used in dev mode to simulate
//! real tools without touching any production directories.

use crate::adapters::{
    adapter_can_write_path, directory_capability, discover_rule_sources_for_adapter,
    tool_capability, RuleSource, ToolAdapter, ToolCapability, ToolCapabilityAccess,
    ToolCapabilityAction, ToolCapabilityActionEvidence, ToolCapabilityFormat, ToolCapabilityKind,
    ToolCapabilityScope, ToolCapabilitySourceConfidence,
};
use std::fs;
use std::path::PathBuf;

pub struct DevToolAdapter {
    tool_id: String,
    tool_name: String,
    tool_icon: String,
    base_dir: PathBuf,
    supports_generic: bool,
}

impl DevToolAdapter {
    pub fn new(
        id: &str,
        name: &str,
        icon: &str,
        base_dir: PathBuf,
        supports_generic: bool,
    ) -> Self {
        Self {
            tool_id: id.to_string(),
            tool_name: name.to_string(),
            tool_icon: icon.to_string(),
            base_dir,
            supports_generic,
        }
    }
}

impl ToolAdapter for DevToolAdapter {
    fn id(&self) -> &str {
        &self.tool_id
    }
    fn name(&self) -> &str {
        &self.tool_name
    }
    fn icon(&self) -> &str {
        &self.tool_icon
    }

    fn config_dir(&self) -> PathBuf {
        self.base_dir.clone()
    }

    fn detect(&self) -> bool {
        self.base_dir.exists()
    }

    fn read_rules(&self) -> Result<Vec<RuleSource>, String> {
        discover_rule_sources_for_adapter(self.id(), &self.capabilities())
    }

    fn write_rule(&self, path: &str, content: &str) -> Result<(), String> {
        let target = PathBuf::from(shellexpand::tilde(path).to_string());
        if !adapter_can_write_path(self, ToolCapabilityKind::Rule, &target) {
            return Err("Rule writing is not enabled for this Dev Tool target".to_string());
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(target, content).map_err(|e| e.to_string())
    }

    fn skills_dir(&self) -> Option<PathBuf> {
        Some(self.base_dir.join("skills"))
    }

    fn supports_generic_skills(&self) -> bool {
        self.supports_generic
    }

    fn allow_external_generic_symlink(&self) -> bool {
        true
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        let mut capabilities = vec![
            tool_capability(
                "default-rule-file",
                ToolCapabilityKind::Rule,
                ToolCapabilityScope::Global,
                ToolCapabilityAccess::Writable,
                ToolCapabilityFormat::Markdown,
                self.base_dir.join("RULES.md").to_string_lossy().to_string(),
                "RULES.md",
                ToolCapabilitySourceConfidence::OfficialRepository,
                "Development adapter fixed rule file used by scenario tests.",
            ),
            directory_capability(
                "rules-directory",
                ToolCapabilityKind::Rule,
                ToolCapabilityScope::Global,
                ToolCapabilityAccess::Writable,
                self.base_dir.join("rules").to_string_lossy().to_string(),
                "Rule directory",
                ToolCapabilitySourceConfidence::OfficialRepository,
                "Development adapter explicit rule directory used by scenario tests.",
            ),
        ];
        let mut skills = directory_capability(
            "dedicated-skills",
            ToolCapabilityKind::Skill,
            ToolCapabilityScope::Tool,
            ToolCapabilityAccess::Writable,
            self.base_dir.join("skills").to_string_lossy().to_string(),
            "Dedicated Skills",
            ToolCapabilitySourceConfidence::OfficialRepository,
            "Development adapter Skill directory used by scenario tests.",
        );
        skills.format = ToolCapabilityFormat::SkillDirectory;
        skills.action_evidence = [
            ToolCapabilityAction::View,
            ToolCapabilityAction::Read,
            ToolCapabilityAction::Install,
            ToolCapabilityAction::Link,
            ToolCapabilityAction::Copy,
            ToolCapabilityAction::Uninstall,
            ToolCapabilityAction::Delete,
            ToolCapabilityAction::Save,
        ]
        .into_iter()
        .map(|action| ToolCapabilityActionEvidence {
            action,
            supported: true,
            evidence: "Development adapter verifies reversible Skill directory actions."
                .to_string(),
            variant: None,
            version: None,
            verified_at: Some("2026-05-17".to_string()),
        })
        .collect();
        capabilities.push(skills);
        if self.supports_generic {
            let mut shared = directory_capability(
                "shared-skills",
                ToolCapabilityKind::Skill,
                ToolCapabilityScope::Shared,
                ToolCapabilityAccess::ReadOnly,
                crate::platform::env::generic_skills_dir()
                    .to_string_lossy()
                    .to_string(),
                "Shared Skills",
                ToolCapabilitySourceConfidence::OfficialRepository,
                "Development adapter consumes shared Skills; Skill workflows own writes.",
            );
            shared.format = ToolCapabilityFormat::SkillDirectory;
            capabilities.push(shared);
        }
        capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_tool_discovers_fixed_file_and_explicit_rule_directory_only() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = DevToolAdapter::new(
            "dev-tool-test",
            "Dev Tool Test",
            "D",
            tmp.path().to_path_buf(),
            false,
        );

        fs::write(tmp.path().join("RULES.md"), "default").unwrap();
        fs::write(
            tmp.path().join("test.md"),
            "root markdown should not be a rule",
        )
        .unwrap();
        fs::create_dir_all(tmp.path().join("rules").join("team")).unwrap();
        fs::write(tmp.path().join("rules").join("test.md"), "created").unwrap();
        fs::write(
            tmp.path().join("rules").join("team").join("nested.md"),
            "nested",
        )
        .unwrap();
        fs::create_dir_all(tmp.path().join("skills").join("demo")).unwrap();
        fs::write(
            tmp.path().join("skills").join("demo").join("SKILL.md"),
            "skill",
        )
        .unwrap();
        fs::create_dir_all(tmp.path().join("test-wild")).unwrap();
        fs::write(tmp.path().join("test-wild").join("SKILL.md"), "wild skill").unwrap();

        let rules = adapter.read_rules().unwrap();
        let labels: Vec<_> = rules.iter().map(|source| source.label.as_str()).collect();

        assert_eq!(labels, vec!["RULES.md", "test.md", "nested.md", "team"]);
        assert!(rules.iter().any(|source| source.group == "team"));
        assert!(!rules.iter().any(|source| source.path.contains("test-wild")));
        assert!(!rules.iter().any(|source| source.path.contains("skills")));
        assert!(!rules
            .iter()
            .any(|source| source.path == tmp.path().join("test.md").to_string_lossy()));
    }

    #[test]
    fn dev_tool_write_uses_rule_policy_boundaries() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = DevToolAdapter::new(
            "dev-tool-test",
            "Dev Tool Test",
            "D",
            tmp.path().to_path_buf(),
            false,
        );

        let rules_file = tmp.path().join("RULES.md");
        let child_rule = tmp.path().join("rules").join("new.md");
        let root_markdown = tmp.path().join("loose.md");

        adapter
            .write_rule(&rules_file.to_string_lossy(), "default")
            .unwrap();
        adapter
            .write_rule(&child_rule.to_string_lossy(), "child")
            .unwrap();
        assert!(adapter
            .write_rule(&root_markdown.to_string_lossy(), "loose")
            .is_err());

        assert_eq!(fs::read_to_string(rules_file).unwrap(), "default");
        assert_eq!(fs::read_to_string(child_rule).unwrap(), "child");
        assert!(!root_markdown.exists());
    }
}
