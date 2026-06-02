// Purpose: Shared response types for Tools domain command adapters.

use serde::Serialize;

use crate::adapters::PrimaryConfigHealth;

#[derive(Serialize)]
pub struct ToolStats {
    pub tool_id: String,
    pub tool_name: String,
    pub icon: String,
    pub detected: bool,
    pub primary_config_health: PrimaryConfigHealth,
    pub rule_count: usize,
    pub skill_count: usize,
    pub config_count: usize,
    pub mcp_count: usize,
}

#[derive(Serialize)]
pub struct DashboardData {
    pub tools: Vec<ToolStats>,
    pub total_rules: usize,
    pub total_skills: usize,
    pub total_configs: usize,
    pub total_mcp: usize,
    pub detected_count: usize,
}
