// Purpose: Own tool discovery and refresh behavior for the Tools domain.

use crate::adapters::{DetectedTool, ToolRegistry};

pub(crate) fn list_tools_domain(registry: &ToolRegistry) -> Vec<DetectedTool> {
    registry.detect_all()
}

pub(crate) fn refresh_tool_domain(
    registry: &ToolRegistry,
    tool_id: String,
) -> Option<DetectedTool> {
    let tools = registry.detect_all();
    tools.into_iter().find(|tool| tool.id == tool_id)
}
