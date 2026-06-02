// Purpose: Shared response types for Rules domain command adapters.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct DiffLine {
    pub tag: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffResult {
    pub left_label: String,
    pub right_label: String,
    pub changes: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRuleTargetClassification {
    InSync,
    RequiresSync,
    Drifted,
    Unresolved,
    Unmanaged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRuleTargetReason {
    InSync,
    PendingSource,
    MissingManagedBlock,
    ManagedBlockDrift,
    StaleManagedBlock,
    MalformedMarkers,
    UnreadableTarget,
    UnconfiguredTarget,
    FileMissing,
    ReadOnlyTarget,
    UnknownSupport,
    UnsupportedTarget,
    NoManagedRelation,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManagedRuleSetState {
    pub rule_id: String,
    pub rule_name: String,
    pub managed_tool_ids: Vec<String>,
    pub source_pending: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManagedRuleTargetState {
    pub tool_id: String,
    pub tool_name: String,
    pub target_path: Option<String>,
    pub rule_set_ids: Vec<String>,
    pub rule_set_names: Vec<String>,
    pub classification: ManagedRuleTargetClassification,
    pub reason: ManagedRuleTargetReason,
    pub can_read: bool,
    pub can_write: bool,
    pub source_pending: bool,
    pub has_managed_block: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_block: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_block: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ManagedRulesSummary {
    pub managed_rule_sets: usize,
    pub managed_targets: usize,
    pub in_sync_targets: usize,
    pub requires_sync_targets: usize,
    pub drifted_targets: usize,
    pub unresolved_targets: usize,
    pub pending_source_rule_sets: usize,
    pub affected_tool_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManagedRulesState {
    pub rule_sets: Vec<ManagedRuleSetState>,
    pub targets: Vec<ManagedRuleTargetState>,
    pub unmanaged_tool_rule_count: usize,
    pub summary: ManagedRulesSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManagedRulesActionFailure {
    pub tool_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManagedRulesActionResult {
    pub requested_tool_ids: Vec<String>,
    pub succeeded_tool_ids: Vec<String>,
    pub failed: Vec<ManagedRulesActionFailure>,
    pub state: ManagedRulesState,
}
