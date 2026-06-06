// Purpose: Continue adapter entry backed by the shared declared-tool adapter.

use crate::adapters::ToolAdapter;
use crate::platform::tool_adapters::declared::adapter_from_catalog;
use std::path::Path;

#[allow(dead_code)]
pub fn create(home: &Path) -> Box<dyn ToolAdapter> {
    Box::new(adapter_from_catalog("continue", home))
}
