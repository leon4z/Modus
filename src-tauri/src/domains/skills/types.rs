// Purpose: Shared Skill management payloads that preserve the frontend command contract.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct SkillPresence {
    pub tool_id: String,
    pub mode: String, // "copy" or "symlink"
    pub path: String, // actual filesystem path for this tool
    pub target_path: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct SkillOverviewItem {
    pub name: String,
    pub description: String,
    pub path: String,
    pub installed_in: Vec<SkillPresence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<crate::adapters::skills::SkillPackageInfo>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillStatus {
    NoVariant,
    VariantNotInstalled,
    VariantInstalledCopy,
    VariantInstalledSymlink,
    VariantDrifted,
    BrokenSymlink,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillSourceEntry {
    pub tool_id: String,
    pub tool_name: String,
    pub status: SkillStatus,
    pub path: String,
    pub path_origin: String,
    pub updated_at: Option<String>,
    pub content_hash: Option<String>,
    pub symlink_target: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolSkillStatus {
    pub tool_id: String,
    pub tool_name: String,
    pub status: SkillStatus,
    pub path: Option<String>,
    pub path_origin: String,
    pub updated_at: Option<String>,
    pub content_hash: Option<String>,
    pub symlink_target: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<SkillSourceEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abnormal_state: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillEntry {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub path: Option<String>,
    pub tool_statuses: Vec<ToolSkillStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<crate::adapters::skills::SkillPackageInfo>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillInventory {
    pub skills: Vec<SkillEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationReceiptItem {
    pub tool_id: Option<String>,
    pub path: String,
    pub action: String,
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RejectReason {
    pub code: String,
    pub message: String,
    pub raw: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StructuredChange {
    pub action: String,
    pub change_kind: String,
    pub skill_name: String,
    pub subject: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_kind: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StructuredBlocked {
    pub action: String,
    pub skill_name: String,
    pub subject: String,
    pub reason: RejectReason,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillChangeConfirmationItem {
    pub operation: String,
    pub location_domain: String,
    pub affected_object_kind: String,
    pub affected_object: String,
    pub real_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_shape: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub risk: Vec<String>,
    pub action: String,
    pub skill_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_context: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationPreview {
    pub creates: Vec<String>,
    pub deletes: Vec<String>,
    pub overwrites: Vec<String>,
    pub preserves: Vec<String>,
    pub requires_force_overwrite: bool,
    pub message: Option<String>,
    pub successes: Vec<OperationReceiptItem>,
    pub failures: Vec<OperationReceiptItem>,
    pub skipped: Vec<OperationReceiptItem>,
    pub changes: Vec<StructuredChange>,
    pub blocked: Vec<StructuredBlocked>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub confirmation_items: Vec<SkillChangeConfirmationItem>,
    pub partial: bool,
}

#[derive(Serialize, Clone)]
pub struct SkillFileEntry {
    pub relative_path: String,
    pub is_dir: bool,
}
