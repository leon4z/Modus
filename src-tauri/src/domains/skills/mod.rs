// Purpose: Skill management domain boundary and workflow exports.

use crate::adapters::skills::{parse_skill_frontmatter, scan_skills_dir, SkillInfo};
#[cfg(test)]
use crate::adapters::ToolAdapter;
use crate::adapters::{
    capability_declared_source_path,
    capability_projections::{
        project_capabilities, ToolCapabilityAction, ToolCapabilityModule, ToolCapabilityProjection,
        ToolCapabilitySourceRole,
    },
    DetectedTool, ToolCapability, ToolCapabilitySupportingSourceRole, ToolRegistry,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

mod app_settings;
mod commands;
mod execution;
mod inventory;
mod operations;
mod preview;
pub(crate) mod repositories;
mod sources;
mod types;

pub use app_settings::*;
pub use commands::*;
pub(crate) use execution::*;
pub(crate) use inventory::*;
pub(crate) use operations::*;
use preview::*;
use repositories::*;
pub(crate) use sources::*;
pub use types::*;
