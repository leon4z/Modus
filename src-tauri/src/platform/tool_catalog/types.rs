// Purpose: Shared static definition types for the internal tool catalog.

use crate::platform::tool_capabilities::{
    ToolCapabilityAccess, ToolCapabilityFormat, ToolCapabilityKind, ToolCapabilityScope,
    ToolCapabilitySourceConfidence,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum PathTemplate {
    Home(&'static str),
    SharedSkills,
    Project(&'static str),
    Empty,
}

impl PathTemplate {
    pub(crate) fn resolve_string(&self, home: &Path) -> String {
        match self {
            PathTemplate::Home(relative) => home.join(relative).to_string_lossy().to_string(),
            PathTemplate::SharedSkills => crate::platform::env::generic_skills_dir()
                .to_string_lossy()
                .to_string(),
            PathTemplate::Project(relative) => (*relative).to_string(),
            PathTemplate::Empty => String::new(),
        }
    }

    pub(crate) fn resolve_path(&self, home: &Path) -> PathBuf {
        match self {
            PathTemplate::Home(relative) => home.join(relative),
            PathTemplate::SharedSkills => crate::platform::env::generic_skills_dir(),
            PathTemplate::Project(relative) => PathBuf::from(relative),
            PathTemplate::Empty => PathBuf::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum CapabilityDiagnostics {
    File,
    Directory,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolCapabilityDefinition {
    pub id: &'static str,
    pub kind: ToolCapabilityKind,
    pub scope: ToolCapabilityScope,
    pub access: ToolCapabilityAccess,
    pub format: ToolCapabilityFormat,
    pub source_path: PathTemplate,
    pub label: &'static str,
    pub diagnostics: CapabilityDiagnostics,
    pub source_confidence: ToolCapabilitySourceConfidence,
    pub notes: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolDefinition {
    pub id: &'static str,
    pub aliases: &'static [&'static str],
    pub name: &'static str,
    pub icon: &'static str,
    pub config_dir: PathTemplate,
    pub skills_dir: Option<PathTemplate>,
    pub supports_generic_skills: bool,
    pub allow_external_generic_symlink: bool,
    pub capabilities: &'static [ToolCapabilityDefinition],
}
