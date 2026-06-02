// Purpose: Built-in AI coding tool registry and canonical id lookup.

use super::types::{CapabilityDiagnostics, PathTemplate, ToolCapabilityDefinition, ToolDefinition};
use crate::platform::tool_capabilities::{
    ToolCapabilityAccess, ToolCapabilityFormat, ToolCapabilityKind, ToolCapabilityScope,
    ToolCapabilitySourceConfidence,
};
use std::collections::{BTreeMap, BTreeSet};

const CODEX_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const OPENCLAW_STATIC_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const CURSOR_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const CONTINUE_CAPABILITIES: &[ToolCapabilityDefinition] = &[
    ToolCapabilityDefinition {
        id: "rules-config",
        kind: ToolCapabilityKind::Rule,
        scope: ToolCapabilityScope::Global,
        access: ToolCapabilityAccess::ReadOnly,
        format: ToolCapabilityFormat::Yaml,
        source_path: PathTemplate::Home(".continue/config.yaml#rules"),
        label: "Rules",
        diagnostics: CapabilityDiagnostics::File,
        source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
        notes: "Continue rules are configured through config.yaml references.",
    },
    ToolCapabilityDefinition {
        id: "mcp-config",
        kind: ToolCapabilityKind::Mcp,
        scope: ToolCapabilityScope::Global,
        access: ToolCapabilityAccess::ReadOnly,
        format: ToolCapabilityFormat::Yaml,
        source_path: PathTemplate::Home(".continue/config.yaml#mcpServers"),
        label: "MCP configuration",
        diagnostics: CapabilityDiagnostics::File,
        source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
        notes: "Continue MCP servers are configured through config.yaml.",
    },
];

const KIRO_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const OPENHANDS_CAPABILITIES: &[ToolCapabilityDefinition] = &[
    ToolCapabilityDefinition {
        id: "project-agents-md",
        kind: ToolCapabilityKind::Rule,
        scope: ToolCapabilityScope::Project,
        access: ToolCapabilityAccess::ReadOnly,
        format: ToolCapabilityFormat::Markdown,
        source_path: PathTemplate::Project("AGENTS.md"),
        label: "AGENTS.md",
        diagnostics: CapabilityDiagnostics::File,
        source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
        notes: "OpenHands loads project AGENTS.md as always-loaded context.",
    },
    ToolCapabilityDefinition {
        id: "project-microagents",
        kind: ToolCapabilityKind::Rule,
        scope: ToolCapabilityScope::Project,
        access: ToolCapabilityAccess::ReadOnly,
        format: ToolCapabilityFormat::Directory,
        source_path: PathTemplate::Project(".openhands/microagents/*.md"),
        label: "Microagents",
        diagnostics: CapabilityDiagnostics::Directory,
        source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
        notes: "OpenHands repository microagents are official.",
    },
    ToolCapabilityDefinition {
        id: "installed-skills",
        kind: ToolCapabilityKind::Skill,
        scope: ToolCapabilityScope::Tool,
        access: ToolCapabilityAccess::ReadOnly,
        format: ToolCapabilityFormat::SkillDirectory,
        source_path: PathTemplate::Home(".openhands/skills/installed"),
        label: "Installed Skills",
        diagnostics: CapabilityDiagnostics::Directory,
        source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
        notes: "OpenHands installed AgentSkills location is official; writes are not enabled in this foundation.",
    },
    ToolCapabilityDefinition {
        id: "mcp-config",
        kind: ToolCapabilityKind::Mcp,
        scope: ToolCapabilityScope::Global,
        access: ToolCapabilityAccess::ReadOnly,
        format: ToolCapabilityFormat::Toml,
        source_path: PathTemplate::Home(".openhands/config.toml#[mcp]"),
        label: "MCP configuration",
        diagnostics: CapabilityDiagnostics::File,
        source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
        notes: "OpenHands MCP configuration is official.",
    },
];

const PI_AGENT_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const GITHUB_COPILOT_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const WINDSURF_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const GOOSE_CAPABILITIES: &[ToolCapabilityDefinition] = &[
    ToolCapabilityDefinition {
        id: "goosehints",
        kind: ToolCapabilityKind::Rule,
        scope: ToolCapabilityScope::Global,
        access: ToolCapabilityAccess::ReadOnly,
        format: ToolCapabilityFormat::Markdown,
        source_path: PathTemplate::Home(".config/goose/.goosehints"),
        label: "Hints",
        diagnostics: CapabilityDiagnostics::File,
        source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
        notes: "Goose hints are official persistent instruction input.",
    },
    ToolCapabilityDefinition {
        id: "recipes",
        kind: ToolCapabilityKind::ProjectAsset,
        scope: ToolCapabilityScope::Project,
        access: ToolCapabilityAccess::ReadOnly,
        format: ToolCapabilityFormat::Directory,
        source_path: PathTemplate::Project("*.yaml"),
        label: "Recipes",
        diagnostics: CapabilityDiagnostics::Directory,
        source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
        notes: "Goose recipes are workflow assets, not managed Skills.",
    },
    ToolCapabilityDefinition {
        id: "extensions",
        kind: ToolCapabilityKind::Mcp,
        scope: ToolCapabilityScope::Global,
        access: ToolCapabilityAccess::Unknown,
        format: ToolCapabilityFormat::Unknown,
        source_path: PathTemplate::Empty,
        label: "Extensions",
        diagnostics: CapabilityDiagnostics::Unknown,
        source_confidence: ToolCapabilitySourceConfidence::OfficialDocs,
        notes: "Goose extensions are MCP-based; source-state diagnostics wait for a verified config path.",
    },
];

const QODER_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const OPENCODE_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const CODEBUDDY_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const WORKBUDDY_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const HERMES_AGENT_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const TRAE_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const TRAE_CN_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const TRAE_SOLO_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const TRAE_SOLO_CN_CAPABILITIES: &[ToolCapabilityDefinition] = &[];

const DEFINITIONS: &[ToolDefinition] = &[
    ToolDefinition {
        id: "cursor",
        aliases: &[],
        name: "Cursor",
        icon: "cursor",
        config_dir: PathTemplate::Home(".cursor"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: CURSOR_CAPABILITIES,
    },
    ToolDefinition {
        id: "continue",
        aliases: &[],
        name: "Continue",
        icon: "Co",
        config_dir: PathTemplate::Home(".continue"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: CONTINUE_CAPABILITIES,
    },
    ToolDefinition {
        id: "claude-code",
        aliases: &["claude_code"],
        name: "Claude Code",
        icon: "claude-code",
        config_dir: PathTemplate::Home(".claude"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: &[],
    },
    ToolDefinition {
        id: "codex",
        aliases: &[],
        name: "Codex",
        icon: "codex",
        config_dir: PathTemplate::Home(".codex"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: CODEX_CAPABILITIES,
    },
    ToolDefinition {
        id: "openclaw",
        aliases: &[],
        name: "OpenClaw",
        icon: "openclaw",
        config_dir: PathTemplate::Home(".openclaw"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: OPENCLAW_STATIC_CAPABILITIES,
    },
    ToolDefinition {
        id: "qoder",
        aliases: &["qodercli", "qoder-cli"],
        name: "Qoder",
        icon: "qoder",
        config_dir: PathTemplate::Home(".qoder"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: QODER_CAPABILITIES,
    },
    ToolDefinition {
        id: "opencode",
        aliases: &["open-code", "open_code"],
        name: "OpenCode",
        icon: "opencode",
        config_dir: PathTemplate::Home(".config/opencode"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: OPENCODE_CAPABILITIES,
    },
    ToolDefinition {
        id: "codebuddy",
        aliases: &["codebuddy-code", "codebuddy_code", "cbc"],
        name: "CodeBuddy",
        icon: "codebuddy",
        config_dir: PathTemplate::Home(".codebuddy"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: CODEBUDDY_CAPABILITIES,
    },
    ToolDefinition {
        id: "workbuddy",
        aliases: &["workbuddy-desktop", "workbuddy_desktop"],
        name: "WorkBuddy",
        icon: "workbuddy",
        config_dir: PathTemplate::Home(".workbuddy"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: WORKBUDDY_CAPABILITIES,
    },
    ToolDefinition {
        id: "hermes-agent",
        aliases: &["hermes_agent", "hermes"],
        name: "Hermes Agent",
        icon: "hermes-agent",
        config_dir: PathTemplate::Home(".hermes"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: HERMES_AGENT_CAPABILITIES,
    },
    ToolDefinition {
        id: "github-copilot",
        aliases: &["copilot"],
        name: "GitHub Copilot",
        icon: "github-copilot",
        config_dir: PathTemplate::Home(".copilot"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: GITHUB_COPILOT_CAPABILITIES,
    },
    ToolDefinition {
        id: "goose",
        aliases: &[],
        name: "Goose",
        icon: "Go",
        config_dir: PathTemplate::Home(".config/goose"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: GOOSE_CAPABILITIES,
    },
    ToolDefinition {
        id: "kiro",
        aliases: &[],
        name: "Kiro",
        icon: "kiro",
        config_dir: PathTemplate::Home(".kiro"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: KIRO_CAPABILITIES,
    },
    ToolDefinition {
        id: "openhands",
        aliases: &[],
        name: "OpenHands",
        icon: "Oh",
        config_dir: PathTemplate::Home(".openhands"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: OPENHANDS_CAPABILITIES,
    },
    ToolDefinition {
        id: "pi-agent",
        aliases: &["pi_agent", "pi"],
        name: "Pi Agent",
        icon: "pi-agent",
        config_dir: PathTemplate::Home(".pi/agent"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: PI_AGENT_CAPABILITIES,
    },
    ToolDefinition {
        id: "trae",
        aliases: &[],
        name: "Trae",
        icon: "trae",
        config_dir: PathTemplate::Home(".trae"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: TRAE_CAPABILITIES,
    },
    ToolDefinition {
        id: "trae-cn",
        aliases: &["trae_cn"],
        name: "Trae CN",
        icon: "trae",
        config_dir: PathTemplate::Home(".trae-cn"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: TRAE_CN_CAPABILITIES,
    },
    ToolDefinition {
        id: "trae-solo",
        aliases: &["trae_solo"],
        name: "Trae Solo",
        icon: "trae",
        config_dir: PathTemplate::Home(".trae"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: TRAE_SOLO_CAPABILITIES,
    },
    ToolDefinition {
        id: "trae-solo-cn",
        aliases: &["trae_solo_cn"],
        name: "Trae Solo CN",
        icon: "trae",
        config_dir: PathTemplate::Home(".trae-cn"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: TRAE_SOLO_CN_CAPABILITIES,
    },
    ToolDefinition {
        id: "windsurf",
        aliases: &[],
        name: "Windsurf",
        icon: "windsurf",
        config_dir: PathTemplate::Home(".codeium/windsurf"),
        skills_dir: None,
        supports_generic_skills: false,
        allow_external_generic_symlink: false,
        capabilities: WINDSURF_CAPABILITIES,
    },
];

#[allow(dead_code)]
pub(crate) fn definitions() -> &'static [ToolDefinition] {
    DEFINITIONS
}

pub(crate) fn definition(id_or_alias: &str) -> Option<&'static ToolDefinition> {
    let canonical = canonical_id(id_or_alias);
    DEFINITIONS
        .iter()
        .find(|definition| definition.id == canonical)
}

#[allow(dead_code)]
pub(crate) fn known_ids() -> Vec<String> {
    DEFINITIONS
        .iter()
        .map(|definition| definition.id.to_string())
        .collect()
}

pub(crate) fn is_retired_builtin_id(id_or_alias: &str) -> bool {
    matches!(
        id_or_alias.trim(),
        "antigravity" | "gemini-cli" | "gemini_cli"
    )
}

pub(crate) fn canonical_id(id_or_alias: &str) -> String {
    let trimmed = id_or_alias.trim();
    for definition in DEFINITIONS {
        if definition.id == trimmed || definition.aliases.iter().any(|alias| *alias == trimmed) {
            return definition.id.to_string();
        }
    }
    trimmed.to_string()
}

#[allow(dead_code)]
pub(crate) fn identity_candidates(id_or_alias: &str) -> Vec<String> {
    let canonical = canonical_id(id_or_alias);
    let mut candidates = vec![canonical.clone()];
    if let Some(definition) = DEFINITIONS
        .iter()
        .find(|definition| definition.id == canonical)
    {
        candidates.extend(definition.aliases.iter().map(|alias| (*alias).to_string()));
    }
    candidates
}

#[allow(dead_code)]
pub(crate) fn validate_registry() -> Result<(), String> {
    let mut ids = BTreeSet::new();
    let mut aliases = BTreeMap::new();
    for definition in DEFINITIONS {
        if !ids.insert(definition.id) {
            return Err(format!("duplicate tool id: {}", definition.id));
        }
        if definition
            .aliases
            .iter()
            .any(|alias| *alias == definition.id)
        {
            return Err(format!("alias repeats canonical id: {}", definition.id));
        }
        for alias in definition.aliases {
            if ids.contains(alias) {
                return Err(format!(
                    "alias {} collides with canonical tool id {}",
                    alias, definition.id
                ));
            }
            if let Some(owner) = aliases.insert(*alias, definition.id) {
                return Err(format!(
                    "alias {} is shared by {} and {}",
                    alias, owner, definition.id
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_registered_aliases() {
        assert_eq!(canonical_id("claude_code"), "claude-code");
        assert_eq!(canonical_id("qodercli"), "qoder");
        assert_eq!(canonical_id("open_code"), "opencode");
        assert_eq!(canonical_id("codebuddy-code"), "codebuddy");
        assert_eq!(canonical_id("workbuddy_desktop"), "workbuddy");
        assert_eq!(canonical_id("hermes"), "hermes-agent");
        assert_eq!(canonical_id("pi_agent"), "pi-agent");
        assert_eq!(canonical_id("pi"), "pi-agent");
        assert_eq!(canonical_id("trae_solo"), "trae-solo");
        assert_eq!(canonical_id("trae_solo_cn"), "trae-solo-cn");
        assert_eq!(canonical_id("codex"), "codex");
    }

    #[test]
    fn recognizes_retired_google_tool_ids_without_registering_them() {
        assert!(is_retired_builtin_id("antigravity"));
        assert!(is_retired_builtin_id("gemini-cli"));
        assert!(is_retired_builtin_id("gemini_cli"));
        assert!(definition("antigravity").is_none());
        assert!(definition("gemini-cli").is_none());
        assert_eq!(canonical_id("gemini_cli"), "gemini_cli");
    }

    #[test]
    fn preserves_unknown_custom_ids() {
        assert_eq!(canonical_id("custom_tool"), "custom_tool");
    }

    #[test]
    fn registry_has_unique_ids_and_aliases() {
        validate_registry().unwrap();
    }

    #[test]
    fn default_path_expansion_uses_home() {
        let home = std::path::PathBuf::from("/Users/example");
        let qoder = definition("qoder").unwrap();
        assert_eq!(
            qoder.config_dir.resolve_string(&home),
            "/Users/example/.qoder"
        );
    }

    #[test]
    fn global_rule_capabilities_separate_writable_external_targets_from_non_targets() {
        let writable_global_rule_tools: [&str; 0] = [];
        for tool_id in writable_global_rule_tools {
            let tool = definition(tool_id).unwrap();
            assert!(
                tool.capabilities.iter().any(|capability| {
                    capability.kind == ToolCapabilityKind::Rule
                        && capability.scope == ToolCapabilityScope::Global
                        && capability.access == ToolCapabilityAccess::Writable
                        && matches!(
                            capability.source_confidence,
                            ToolCapabilitySourceConfidence::OfficialDocs
                                | ToolCapabilitySourceConfidence::OfficialRepository
                                | ToolCapabilitySourceConfidence::CertifiedLocalProductBehavior
                        )
                }),
                "{tool_id} should expose a certified writable global rule target"
            );
        }

        for tool_id in [
            "cursor",
            "qoder",
            "opencode",
            "codebuddy",
            "hermes-agent",
            "workbuddy",
            "trae",
            "trae-cn",
            "trae-solo",
            "trae-solo-cn",
        ] {
            let tool = definition(tool_id).unwrap();
            assert!(
                !tool.capabilities.iter().any(|capability| {
                    capability.kind == ToolCapabilityKind::Rule
                        && capability.scope == ToolCapabilityScope::Global
                        && capability.access == ToolCapabilityAccess::Writable
                }),
                "{tool_id} must not expose a writable global rule target"
            );
        }
    }

    #[test]
    fn trae_entries_match_certified_capability_model() {
        let home = std::path::PathBuf::from("/Users/example");
        let trae = definition("trae").unwrap();
        assert_eq!(trae.icon, "trae");
        assert_eq!(
            trae.config_dir.resolve_string(&home),
            "/Users/example/.trae"
        );
        assert!(trae.skills_dir.is_none());
        assert!(!trae.supports_generic_skills);
        assert!(!trae.allow_external_generic_symlink);
        assert!(
            trae.capabilities.is_empty(),
            "Trae runtime capabilities are owned by the adapter boundary"
        );

        let trae_cn = definition("trae_cn").unwrap();
        assert_eq!(trae_cn.id, "trae-cn");
        assert_eq!(
            trae_cn.config_dir.resolve_string(&home),
            "/Users/example/.trae-cn"
        );
        assert!(trae_cn.skills_dir.is_none());
        assert!(!trae_cn.supports_generic_skills);
        assert!(!trae_cn.allow_external_generic_symlink);
        assert!(
            trae_cn.capabilities.is_empty(),
            "Trae CN runtime capabilities are owned by the adapter boundary"
        );
        let trae_solo = definition("trae_solo").unwrap();
        assert_eq!(trae_solo.id, "trae-solo");
        assert_eq!(
            trae_solo.config_dir.resolve_string(&home),
            "/Users/example/.trae"
        );
        assert!(trae_solo.skills_dir.is_none());
        assert!(!trae_solo.supports_generic_skills);
        assert!(!trae_solo.allow_external_generic_symlink);
        assert!(
            trae_solo.capabilities.is_empty(),
            "Trae Solo runtime capabilities are owned by the adapter boundary"
        );

        let trae_solo_cn = definition("trae-solo-cn").unwrap();
        assert_eq!(trae_solo_cn.id, "trae-solo-cn");
        assert_eq!(
            trae_solo_cn.config_dir.resolve_string(&home),
            "/Users/example/.trae-cn"
        );
        assert!(trae_solo_cn.skills_dir.is_none());
        assert!(!trae_solo_cn.supports_generic_skills);
        assert!(!trae_solo_cn.allow_external_generic_symlink);
        assert!(
            trae_solo_cn.capabilities.is_empty(),
            "Trae Solo CN runtime capabilities are owned by the adapter boundary"
        );
    }

    #[test]
    fn new_cli_tools_match_official_capability_model() {
        let home = std::path::PathBuf::from("/Users/example");

        let qoder = definition("qoder-cli").unwrap();
        assert_eq!(qoder.id, "qoder");
        assert_eq!(qoder.icon, "qoder");
        assert_eq!(
            qoder.config_dir.resolve_string(&home),
            "/Users/example/.qoder"
        );
        assert!(qoder.skills_dir.is_none());
        assert!(!qoder.supports_generic_skills);
        assert!(!qoder.allow_external_generic_symlink);
        assert!(
            qoder.capabilities.is_empty(),
            "Qoder runtime capabilities are owned by the adapter boundary"
        );

        let opencode = definition("open-code").unwrap();
        assert_eq!(opencode.id, "opencode");
        assert_eq!(opencode.icon, "opencode");
        assert_eq!(
            opencode.config_dir.resolve_string(&home),
            "/Users/example/.config/opencode"
        );
        assert!(opencode.skills_dir.is_none());
        assert!(!opencode.supports_generic_skills);
        assert!(!opencode.allow_external_generic_symlink);
        assert!(
            opencode.capabilities.is_empty(),
            "OpenCode runtime capabilities are owned by the adapter boundary"
        );

        let codebuddy = definition("cbc").unwrap();
        assert_eq!(codebuddy.id, "codebuddy");
        assert_eq!(codebuddy.icon, "codebuddy");
        assert_eq!(
            codebuddy.config_dir.resolve_string(&home),
            "/Users/example/.codebuddy"
        );
        assert!(codebuddy.skills_dir.is_none());
        assert!(!codebuddy.supports_generic_skills);
        assert!(!codebuddy.allow_external_generic_symlink);
        assert!(
            codebuddy.capabilities.is_empty(),
            "CodeBuddy runtime capabilities are owned by the adapter boundary"
        );

        let claude_code = definition("claude_code").unwrap();
        assert_eq!(claude_code.id, "claude-code");
        assert_eq!(claude_code.icon, "claude-code");
        assert_eq!(
            claude_code.config_dir.resolve_string(&home),
            "/Users/example/.claude"
        );
        assert!(claude_code.skills_dir.is_none());
        assert!(!claude_code.supports_generic_skills);
        assert!(!claude_code.allow_external_generic_symlink);
        assert!(
            claude_code.capabilities.is_empty(),
            "Claude Code runtime capabilities are owned by the adapter boundary"
        );

        let codex = definition("codex").unwrap();
        assert_eq!(codex.id, "codex");
        assert_eq!(codex.icon, "codex");
        assert_eq!(
            codex.config_dir.resolve_string(&home),
            "/Users/example/.codex"
        );
        assert!(codex.skills_dir.is_none());
        assert!(!codex.supports_generic_skills);
        assert!(!codex.allow_external_generic_symlink);
        assert!(
            codex.capabilities.is_empty(),
            "Codex runtime capabilities are owned by the adapter boundary"
        );

        let workbuddy = definition("workbuddy_desktop").unwrap();
        assert_eq!(workbuddy.id, "workbuddy");
        assert_eq!(workbuddy.icon, "workbuddy");
        assert_eq!(
            workbuddy.config_dir.resolve_string(&home),
            "/Users/example/.workbuddy"
        );
        assert!(workbuddy.skills_dir.is_none());
        assert!(!workbuddy.supports_generic_skills);
        assert!(!workbuddy.allow_external_generic_symlink);
        assert!(
            workbuddy.capabilities.is_empty(),
            "WorkBuddy runtime capabilities are owned by the adapter boundary"
        );

        let hermes = definition("hermes").unwrap();
        assert_eq!(hermes.id, "hermes-agent");
        assert_eq!(hermes.icon, "hermes-agent");
        assert_eq!(
            hermes.config_dir.resolve_string(&home),
            "/Users/example/.hermes"
        );
        assert!(hermes.skills_dir.is_none());
        assert!(!hermes.supports_generic_skills);
        assert!(!hermes.allow_external_generic_symlink);
        assert!(
            hermes.capabilities.is_empty(),
            "Hermes Agent runtime capabilities are owned by the adapter boundary"
        );

        let copilot = definition("copilot").unwrap();
        assert_eq!(copilot.id, "github-copilot");
        assert_eq!(copilot.icon, "github-copilot");
        assert_eq!(
            copilot.config_dir.resolve_string(&home),
            "/Users/example/.copilot"
        );
        assert!(copilot.skills_dir.is_none());
        assert!(!copilot.supports_generic_skills);
        assert!(!copilot.allow_external_generic_symlink);
        assert!(
            copilot.capabilities.is_empty(),
            "GitHub Copilot runtime capabilities are owned by the adapter boundary"
        );

        let kiro = definition("kiro").unwrap();
        assert_eq!(kiro.id, "kiro");
        assert_eq!(kiro.icon, "kiro");
        assert_eq!(
            kiro.config_dir.resolve_string(&home),
            "/Users/example/.kiro"
        );
        assert!(kiro.skills_dir.is_none());
        assert!(!kiro.supports_generic_skills);
        assert!(!kiro.allow_external_generic_symlink);
        assert!(
            kiro.capabilities.is_empty(),
            "Kiro runtime capabilities are owned by the adapter boundary"
        );

        let windsurf = definition("windsurf").unwrap();
        assert_eq!(windsurf.id, "windsurf");
        assert_eq!(windsurf.icon, "windsurf");
        assert_eq!(
            windsurf.config_dir.resolve_string(&home),
            "/Users/example/.codeium/windsurf"
        );
        assert!(windsurf.skills_dir.is_none());
        assert!(!windsurf.supports_generic_skills);
        assert!(!windsurf.allow_external_generic_symlink);
        assert!(
            windsurf.capabilities.is_empty(),
            "Windsurf runtime capabilities are owned by the adapter boundary"
        );
    }

    #[test]
    fn cursor_registry_is_identity_only_after_adapter_migration() {
        let cursor = definition("cursor").unwrap();

        assert_eq!(cursor.icon, "cursor");
        assert!(cursor.skills_dir.is_none());
        assert!(!cursor.supports_generic_skills);
        assert!(!cursor.allow_external_generic_symlink);
        assert!(cursor.capabilities.is_empty());
    }

    #[test]
    fn openclaw_registry_is_identity_only_after_adapter_migration() {
        let openclaw = definition("openclaw").unwrap();

        assert!(openclaw.skills_dir.is_none());
        assert!(!openclaw.supports_generic_skills);
        assert!(!openclaw.allow_external_generic_symlink);
        assert!(openclaw.capabilities.is_empty());
    }
}
