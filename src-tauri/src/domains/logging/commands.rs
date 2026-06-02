// Purpose: Tauri commands for local application log access.

use crate::platform::logging::{
    self, LogEvent, ModulePerformanceLogEvent, ModulePerformanceMilestoneLog,
    ModulePerformanceRequestLog, SkillPerformanceLogEvent,
};

pub fn write_application_log(event: LogEvent) -> Result<(), String> {
    logging::write_log_event(event)
}

pub fn get_application_log_path() -> Result<String, String> {
    logging::ensure_log_file().map(|path| path.to_string_lossy().to_string())
}

pub fn list_application_logs() -> Result<Vec<logging::ManagedLogFile>, String> {
    logging::list_application_logs()
}

pub fn read_application_log(id: String) -> Result<logging::ManagedLogReadResult, String> {
    logging::read_application_log(id)
}

pub fn export_application_logs(ids: Vec<String>, destination: String) -> Result<String, String> {
    logging::export_application_logs(ids, destination)
}

pub fn write_skill_performance_log(event: SkillPerformanceLogEvent) -> Result<(), String> {
    let config = crate::platform::config::load_config();
    let enabled = config
        .module_performance_diagnostics_enabled
        .unwrap_or(config.skill_performance_diagnostics_enabled);
    logging::write_module_performance_event_if_enabled(
        skill_performance_event_as_module_event(event),
        enabled,
    )
}

pub fn get_skill_performance_log_path() -> Result<String, String> {
    logging::ensure_module_performance_log_file().map(|path| path.to_string_lossy().to_string())
}

pub fn write_module_performance_log(event: ModulePerformanceLogEvent) -> Result<(), String> {
    let config = crate::platform::config::load_config();
    let enabled = config
        .module_performance_diagnostics_enabled
        .unwrap_or(config.skill_performance_diagnostics_enabled);
    logging::write_module_performance_event_if_enabled(event, enabled)
}

pub fn get_module_performance_log_path() -> Result<String, String> {
    logging::ensure_module_performance_log_file().map(|path| path.to_string_lossy().to_string())
}

pub fn list_module_performance_logs() -> Result<Vec<logging::ManagedLogFile>, String> {
    logging::list_module_performance_logs()
}

pub fn read_module_performance_log(id: String) -> Result<logging::ManagedLogReadResult, String> {
    logging::read_module_performance_log(id)
}

pub fn export_module_performance_logs(
    ids: Vec<String>,
    destination: String,
) -> Result<String, String> {
    logging::export_module_performance_logs(ids, destination)
}

fn skill_performance_event_as_module_event(
    event: SkillPerformanceLogEvent,
) -> ModulePerformanceLogEvent {
    ModulePerformanceLogEvent {
        module: Some("skills".to_string()),
        view: None,
        reason: event.reason,
        status: event.status,
        visible_ms: None,
        interactive_ms: None,
        background_complete_ms: event.total_ms,
        total_ms: event.total_ms,
        request_count: event.request_count,
        request_counts: event.request_counts,
        counters: None,
        milestones: event.milestones.map(|milestones| {
            milestones
                .into_iter()
                .map(|milestone| ModulePerformanceMilestoneLog {
                    name: milestone.name,
                    role: None,
                    at_ms: milestone.at_ms,
                })
                .collect()
        }),
        requests: event.requests.map(|requests| {
            requests
                .into_iter()
                .map(|request| ModulePerformanceRequestLog {
                    label: request.label,
                    status: request.status,
                    duration_ms: request.duration_ms,
                    error_kind: request.error_kind,
                })
                .collect()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::logging::{SkillPerformanceMilestoneLog, SkillPerformanceRequestLog};

    #[test]
    fn maps_legacy_skill_performance_event_to_skills_module_event() {
        let event = SkillPerformanceLogEvent {
            reason: Some("entry".to_string()),
            status: Some("success".to_string()),
            total_ms: Some(42.0),
            request_count: Some(1),
            request_counts: Some(std::collections::HashMap::from([(
                "inventory".to_string(),
                1,
            )])),
            milestones: Some(vec![SkillPerformanceMilestoneLog {
                name: Some("visible".to_string()),
                at_ms: Some(2.0),
            }]),
            requests: Some(vec![SkillPerformanceRequestLog {
                label: Some("inventory".to_string()),
                status: Some("success".to_string()),
                duration_ms: Some(40.0),
                error_kind: None,
            }]),
        };

        let mapped = skill_performance_event_as_module_event(event);

        assert_eq!(mapped.module.as_deref(), Some("skills"));
        assert_eq!(mapped.reason.as_deref(), Some("entry"));
        assert_eq!(mapped.background_complete_ms, Some(42.0));
        assert_eq!(mapped.total_ms, Some(42.0));
        assert_eq!(
            mapped.milestones.unwrap()[0].name.as_deref(),
            Some("visible")
        );
        assert_eq!(
            mapped.requests.unwrap()[0].label.as_deref(),
            Some("inventory")
        );
    }
}
