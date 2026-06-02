//! Local application logging with bounded retention and safe-by-default redaction.

use chrono::{Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const LOG_FILE_PREFIX: &str = "app";
const MODULE_PERFORMANCE_LOG_PREFIX: &str = "module-performance";
const RETAINED_LOG_DAYS: i64 = 5;
const MAX_LOG_READ_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogCategory {
    Rules,
    Skills,
    Settings,
    System,
}

impl Default for LogCategory {
    fn default() -> Self {
        Self::System
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    #[serde(default)]
    pub level: LogLevel,
    #[serde(default)]
    pub category: LogCategory,
    pub action: String,
    pub result: Option<String>,
    pub message: Option<String>,
    pub tool_id: Option<String>,
    pub target_role: Option<String>,
    pub target_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEventRecord {
    pub timestamp: String,
    pub level: LogLevel,
    pub category: LogCategory,
    pub action: String,
    pub result: Option<String>,
    pub message: Option<String>,
    pub tool_id: Option<String>,
    pub target_role: Option<String>,
    pub target_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillPerformanceLogEvent {
    pub reason: Option<String>,
    pub status: Option<String>,
    pub total_ms: Option<f64>,
    pub request_count: Option<u64>,
    pub request_counts: Option<std::collections::HashMap<String, u64>>,
    pub milestones: Option<Vec<SkillPerformanceMilestoneLog>>,
    pub requests: Option<Vec<SkillPerformanceRequestLog>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillPerformanceMilestoneLog {
    pub name: Option<String>,
    pub at_ms: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillPerformanceRequestLog {
    pub label: Option<String>,
    pub status: Option<String>,
    pub duration_ms: Option<f64>,
    pub error_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillPerformanceLogRecord {
    pub timestamp: String,
    pub reason: Option<String>,
    pub status: Option<String>,
    pub total_ms: Option<f64>,
    pub request_count: Option<u64>,
    pub request_counts: Option<std::collections::HashMap<String, u64>>,
    pub milestones: Vec<SkillPerformanceMilestoneLog>,
    pub requests: Vec<SkillPerformanceRequestLog>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModulePerformanceLogEvent {
    pub module: Option<String>,
    pub view: Option<String>,
    pub reason: Option<String>,
    pub status: Option<String>,
    pub visible_ms: Option<f64>,
    pub interactive_ms: Option<f64>,
    pub background_complete_ms: Option<f64>,
    pub total_ms: Option<f64>,
    pub request_count: Option<u64>,
    pub request_counts: Option<std::collections::HashMap<String, u64>>,
    pub counters: Option<std::collections::HashMap<String, u64>>,
    pub milestones: Option<Vec<ModulePerformanceMilestoneLog>>,
    pub requests: Option<Vec<ModulePerformanceRequestLog>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModulePerformanceMilestoneLog {
    pub name: Option<String>,
    pub role: Option<String>,
    pub at_ms: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModulePerformanceRequestLog {
    pub label: Option<String>,
    pub status: Option<String>,
    pub duration_ms: Option<f64>,
    pub error_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModulePerformanceLogRecord {
    pub timestamp: String,
    pub module: Option<String>,
    pub view: Option<String>,
    pub reason: Option<String>,
    pub status: Option<String>,
    pub visible_ms: Option<f64>,
    pub interactive_ms: Option<f64>,
    pub background_complete_ms: Option<f64>,
    pub total_ms: Option<f64>,
    pub request_count: Option<u64>,
    pub request_counts: Option<std::collections::HashMap<String, u64>>,
    pub counters: Option<std::collections::HashMap<String, u64>>,
    pub milestones: Vec<ModulePerformanceMilestoneLog>,
    pub requests: Vec<ModulePerformanceRequestLog>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedLogFile {
    pub id: String,
    pub label: String,
    pub path: String,
    pub date: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedLogReadResult {
    pub id: String,
    pub label: String,
    pub content: String,
    pub truncated: bool,
}

pub fn write_log_event(event: LogEvent) -> Result<(), String> {
    write_log_event_to_dir(&crate::platform::env::logs_dir(), event)
}

pub fn write_skill_performance_event(event: SkillPerformanceLogEvent) -> Result<(), String> {
    write_skill_performance_event_to_dir(&crate::platform::env::logs_dir(), event)
}

pub fn write_skill_performance_event_if_enabled(
    event: SkillPerformanceLogEvent,
    enabled: bool,
) -> Result<(), String> {
    write_skill_performance_event_to_dir_if_enabled(
        &crate::platform::env::logs_dir(),
        event,
        enabled,
    )
}

pub fn write_module_performance_event_if_enabled(
    event: ModulePerformanceLogEvent,
    enabled: bool,
) -> Result<(), String> {
    write_module_performance_event_to_dir_if_enabled(
        &crate::platform::env::logs_dir(),
        event,
        enabled,
    )
}

pub fn write_module_performance_event_to_dir_if_enabled(
    dir: &Path,
    event: ModulePerformanceLogEvent,
    enabled: bool,
) -> Result<(), String> {
    if !enabled {
        return Ok(());
    }
    write_module_performance_event_to_dir(dir, event)
}

pub fn write_module_performance_event_to_dir(
    dir: &Path,
    event: ModulePerformanceLogEvent,
) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("create log dir failed: {}", e))?;
    prune_managed_logs(dir, MODULE_PERFORMANCE_LOG_PREFIX)?;

    let record = sanitize_module_performance_event(event);
    let line = serde_json::to_string(&record).map_err(|e| e.to_string())?;
    let path = current_managed_log_path(dir, MODULE_PERFORMANCE_LOG_PREFIX);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("open module performance log file failed: {}", e))?;
    writeln!(file, "{}", line)
        .map_err(|e| format!("write module performance log file failed: {}", e))
}

pub fn write_skill_performance_event_to_dir_if_enabled(
    dir: &Path,
    event: SkillPerformanceLogEvent,
    enabled: bool,
) -> Result<(), String> {
    if !enabled {
        return Ok(());
    }
    write_skill_performance_event_to_dir(dir, event)
}

pub fn write_skill_performance_event_to_dir(
    dir: &Path,
    event: SkillPerformanceLogEvent,
) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("create log dir failed: {}", e))?;
    prune_managed_logs(dir, "skill-performance")?;

    let record = sanitize_skill_performance_event(event);
    let line = serde_json::to_string(&record).map_err(|e| e.to_string())?;
    let path = current_managed_log_path(dir, "skill-performance");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("open skill performance log file failed: {}", e))?;
    writeln!(file, "{}", line)
        .map_err(|e| format!("write skill performance log file failed: {}", e))
}

pub fn write_log_event_to_dir(dir: &Path, event: LogEvent) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("create log dir failed: {}", e))?;
    prune_managed_logs(dir, LOG_FILE_PREFIX)?;

    let record = sanitize_event(event);
    let line = serde_json::to_string(&record).map_err(|e| e.to_string())?;
    let path = current_managed_log_path(dir, LOG_FILE_PREFIX);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("open log file failed: {}", e))?;
    writeln!(file, "{}", line).map_err(|e| format!("write log file failed: {}", e))
}

pub fn ensure_log_file() -> Result<PathBuf, String> {
    ensure_managed_log_file(LOG_FILE_PREFIX)
}

pub fn ensure_skill_performance_log_file() -> Result<PathBuf, String> {
    ensure_managed_log_file("skill-performance")
}

pub fn ensure_module_performance_log_file() -> Result<PathBuf, String> {
    ensure_managed_log_file(MODULE_PERFORMANCE_LOG_PREFIX)
}

fn ensure_managed_log_file(prefix: &str) -> Result<PathBuf, String> {
    let dir = crate::platform::env::logs_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("create log dir failed: {}", e))?;
    prune_managed_logs(&dir, prefix)?;
    let path = current_managed_log_path(&dir, prefix);
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("open log file failed: {}", e))?;
    Ok(path)
}

pub fn list_application_logs() -> Result<Vec<ManagedLogFile>, String> {
    list_managed_logs_for_prefix(&crate::platform::env::logs_dir(), LOG_FILE_PREFIX, true)
}

pub fn list_module_performance_logs() -> Result<Vec<ManagedLogFile>, String> {
    list_managed_logs_for_prefix(
        &crate::platform::env::logs_dir(),
        MODULE_PERFORMANCE_LOG_PREFIX,
        true,
    )
}

pub fn read_application_log(id: String) -> Result<ManagedLogReadResult, String> {
    read_managed_log(&crate::platform::env::logs_dir(), LOG_FILE_PREFIX, &id)
}

pub fn read_module_performance_log(id: String) -> Result<ManagedLogReadResult, String> {
    read_managed_log(
        &crate::platform::env::logs_dir(),
        MODULE_PERFORMANCE_LOG_PREFIX,
        &id,
    )
}

pub fn export_application_logs(ids: Vec<String>, destination: String) -> Result<String, String> {
    export_managed_logs(
        &crate::platform::env::logs_dir(),
        LOG_FILE_PREFIX,
        ids,
        PathBuf::from(destination),
    )
}

pub fn export_module_performance_logs(
    ids: Vec<String>,
    destination: String,
) -> Result<String, String> {
    export_managed_logs(
        &crate::platform::env::logs_dir(),
        MODULE_PERFORMANCE_LOG_PREFIX,
        ids,
        PathBuf::from(destination),
    )
}

fn sanitize_event(event: LogEvent) -> LogEventRecord {
    LogEventRecord {
        timestamp: Utc::now().to_rfc3339(),
        level: event.level,
        category: event.category,
        action: redact_sensitive(&event.action),
        result: event.result.map(|value| redact_sensitive(&value)),
        message: event.message.map(|value| redact_text(&value)),
        tool_id: event.tool_id.map(|value| redact_sensitive(&value)),
        target_role: event.target_role.map(|value| redact_sensitive(&value)),
        target_path: event.target_path.map(|value| redact_path(&value)),
        error: event.error.map(|value| redact_text(&value)),
    }
}

fn sanitize_skill_performance_event(event: SkillPerformanceLogEvent) -> SkillPerformanceLogRecord {
    SkillPerformanceLogRecord {
        timestamp: Utc::now().to_rfc3339(),
        reason: event.reason.map(|value| redact_performance_text(&value)),
        status: event.status.map(|value| redact_performance_text(&value)),
        total_ms: event.total_ms,
        request_count: event.request_count,
        request_counts: event.request_counts.map(|counts| {
            counts
                .into_iter()
                .map(|(key, value)| (redact_performance_text(&key), value))
                .collect()
        }),
        milestones: event
            .milestones
            .unwrap_or_default()
            .into_iter()
            .map(|milestone| SkillPerformanceMilestoneLog {
                name: milestone.name.map(|value| redact_performance_text(&value)),
                at_ms: milestone.at_ms,
            })
            .collect(),
        requests: event
            .requests
            .unwrap_or_default()
            .into_iter()
            .map(|request| SkillPerformanceRequestLog {
                label: request.label.map(|value| redact_performance_text(&value)),
                status: request.status.map(|value| redact_performance_text(&value)),
                duration_ms: request.duration_ms,
                error_kind: request
                    .error_kind
                    .map(|value| redact_performance_text(&value)),
            })
            .collect(),
    }
}

fn sanitize_module_performance_event(
    event: ModulePerformanceLogEvent,
) -> ModulePerformanceLogRecord {
    ModulePerformanceLogRecord {
        timestamp: Utc::now().to_rfc3339(),
        module: event.module.map(|value| redact_performance_text(&value)),
        view: event.view.map(|value| redact_performance_text(&value)),
        reason: event.reason.map(|value| redact_performance_text(&value)),
        status: event.status.map(|value| redact_performance_text(&value)),
        visible_ms: event.visible_ms,
        interactive_ms: event.interactive_ms,
        background_complete_ms: event.background_complete_ms,
        total_ms: event.total_ms,
        request_count: event.request_count,
        request_counts: event.request_counts.map(|counts| {
            counts
                .into_iter()
                .map(|(key, value)| (redact_performance_text(&key), value))
                .collect()
        }),
        counters: event.counters.map(|counts| {
            counts
                .into_iter()
                .map(|(key, value)| (redact_performance_text(&key), value))
                .collect()
        }),
        milestones: event
            .milestones
            .unwrap_or_default()
            .into_iter()
            .map(|milestone| ModulePerformanceMilestoneLog {
                name: milestone.name.map(|value| redact_performance_text(&value)),
                role: milestone.role.map(|value| redact_performance_text(&value)),
                at_ms: milestone.at_ms,
            })
            .collect(),
        requests: event
            .requests
            .unwrap_or_default()
            .into_iter()
            .map(|request| ModulePerformanceRequestLog {
                label: request.label.map(|value| redact_performance_text(&value)),
                status: request.status.map(|value| redact_performance_text(&value)),
                duration_ms: request.duration_ms,
                error_kind: request
                    .error_kind
                    .map(|value| redact_performance_text(&value)),
            })
            .collect(),
    }
}

fn current_managed_log_path(dir: &Path, prefix: &str) -> PathBuf {
    dir.join(managed_log_file_name(prefix, Utc::now().date_naive()))
}

fn managed_log_file_name(prefix: &str, date: NaiveDate) -> String {
    format!("{}-{}.log", prefix, date.format("%Y-%m-%d"))
}

fn parse_managed_log_date(file_name: &str, prefix: &str) -> Option<NaiveDate> {
    let rest = file_name.strip_prefix(&format!("{prefix}-"))?;
    let date = rest.strip_suffix(".log")?;
    NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
}

fn prune_managed_logs(dir: &Path, prefix: &str) -> Result<(), String> {
    let cutoff = Utc::now().date_naive() - Duration::days(RETAINED_LOG_DAYS - 1);
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(date) = parse_managed_log_date(file_name, prefix) else {
            continue;
        };
        if date < cutoff {
            fs::remove_file(&path).map_err(|e| format!("remove old managed log failed: {}", e))?;
        }
    }
    Ok(())
}

fn list_managed_logs_for_prefix(
    dir: &Path,
    prefix: &str,
    ensure_current: bool,
) -> Result<Vec<ManagedLogFile>, String> {
    fs::create_dir_all(dir).map_err(|e| format!("create log dir failed: {}", e))?;
    prune_managed_logs(dir, prefix)?;
    if ensure_current {
        let path = current_managed_log_path(dir, prefix);
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("open log file failed: {}", e))?;
    }

    let mut logs = vec![];
    let entries = fs::read_dir(dir).map_err(|e| format!("read log dir failed: {}", e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(date) = parse_managed_log_date(file_name, prefix) else {
            continue;
        };
        let size_bytes = fs::metadata(&path)
            .map(|meta| meta.len())
            .unwrap_or_default();
        let id = file_name.to_string();
        logs.push(ManagedLogFile {
            id: id.clone(),
            label: date.format("%Y-%m-%d").to_string(),
            path: path.to_string_lossy().to_string(),
            date: date.format("%Y-%m-%d").to_string(),
            size_bytes,
        });
    }
    logs.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.id.cmp(&b.id)));
    Ok(logs)
}

fn managed_log_path_from_id(dir: &Path, prefix: &str, id: &str) -> Result<PathBuf, String> {
    if parse_managed_log_date(id, prefix).is_none() {
        return Err("invalid managed log id".to_string());
    }
    let path = dir.join(id);
    let canonical_dir = fs::canonicalize(dir).map_err(|e| format!("read log dir failed: {}", e))?;
    if path.parent() != Some(dir) {
        return Err("invalid managed log path".to_string());
    }
    if path.exists() {
        let canonical_path =
            fs::canonicalize(&path).map_err(|e| format!("read log file failed: {}", e))?;
        if !canonical_path.starts_with(&canonical_dir) {
            return Err("managed log path escaped log directory".to_string());
        }
    }
    Ok(path)
}

fn read_managed_log(dir: &Path, prefix: &str, id: &str) -> Result<ManagedLogReadResult, String> {
    fs::create_dir_all(dir).map_err(|e| format!("create log dir failed: {}", e))?;
    prune_managed_logs(dir, prefix)?;
    let path = managed_log_path_from_id(dir, prefix, id)?;
    let date =
        parse_managed_log_date(id, prefix).ok_or_else(|| "invalid managed log id".to_string())?;
    if !path.exists() {
        return Ok(ManagedLogReadResult {
            id: id.to_string(),
            label: date.format("%Y-%m-%d").to_string(),
            content: String::new(),
            truncated: false,
        });
    }

    let mut file = File::open(&path).map_err(|e| format!("open log file failed: {}", e))?;
    let size = file.metadata().map(|meta| meta.len()).unwrap_or_default();
    let truncated = size > MAX_LOG_READ_BYTES;
    if truncated {
        file.seek(SeekFrom::End(-(MAX_LOG_READ_BYTES as i64)))
            .map_err(|e| format!("seek log file failed: {}", e))?;
    }
    let mut bytes = vec![];
    file.read_to_end(&mut bytes)
        .map_err(|e| format!("read log file failed: {}", e))?;
    let content = String::from_utf8_lossy(&bytes).to_string();
    Ok(ManagedLogReadResult {
        id: id.to_string(),
        label: date.format("%Y-%m-%d").to_string(),
        content,
        truncated,
    })
}

fn export_managed_logs(
    dir: &Path,
    prefix: &str,
    ids: Vec<String>,
    destination: PathBuf,
) -> Result<String, String> {
    let logs = list_managed_logs_for_prefix(dir, prefix, true)?;
    let selected_ids = if ids.is_empty() {
        logs.iter().map(|file| file.id.clone()).collect::<Vec<_>>()
    } else {
        ids
    };
    let mut output = String::new();
    for id in selected_ids {
        let path = managed_log_path_from_id(dir, prefix, &id)?;
        let date = parse_managed_log_date(&id, prefix)
            .ok_or_else(|| "invalid managed log id".to_string())?;
        output.push_str(&format!(
            "===== {} {} =====\n",
            prefix,
            date.format("%Y-%m-%d")
        ));
        if path.exists() {
            output.push_str(
                &fs::read_to_string(&path).map_err(|e| format!("read log file failed: {}", e))?,
            );
        }
        if !output.ends_with('\n') {
            output.push('\n');
        }
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create export dir failed: {}", e))?;
    }
    fs::write(&destination, output).map_err(|e| format!("write log export failed: {}", e))?;
    Ok(destination.to_string_lossy().to_string())
}

pub fn redact_path(value: &str) -> String {
    let value = redact_text(value);
    value
}

pub fn redact_text(value: &str) -> String {
    let value = redact_sensitive(value);
    if let Some(home) = dirs::home_dir() {
        let home_s = home.to_string_lossy().to_string();
        if value == home_s {
            return "~".to_string();
        }
        return value.replace(&(home_s + "/"), "~/");
    }
    value
}

pub fn redact_sensitive(value: &str) -> String {
    let mut redacted = value.to_string();
    redacted = redact_key_value_like(&redacted, "password");
    redacted = redact_key_value_like(&redacted, "passwd");
    redacted = redact_key_value_like(&redacted, "secret");
    redacted = redact_key_value_like(&redacted, "token");
    redacted = redact_key_value_like(&redacted, "api_key");
    redacted = redact_key_value_like(&redacted, "apikey");
    redacted = redact_key_value_like(&redacted, "access_key");
    redacted
}

fn redact_performance_text(value: &str) -> String {
    let redacted = redact_text(value);
    if redacted.contains('/') || redacted.contains('\\') {
        return "[REDACTED_PATH]".to_string();
    }
    redacted
}

fn redact_key_value_like(input: &str, key: &str) -> String {
    let mut out = String::new();
    let mut cursor = 0;

    while let Some(start) = find_ascii_case_insensitive(input, key, cursor) {
        let key_end = start + key.len();
        out.push_str(&input[cursor..key_end]);

        let rest = &input[key_end..];
        let mut suffix_start = None;
        for (idx, ch) in rest.char_indices() {
            if ch == ':' || ch == '=' {
                out.push(ch);
                suffix_start = Some(idx + ch.len_utf8());
                break;
            }
            if !ch.is_whitespace() && ch != '"' && ch != '\'' {
                break;
            }
            out.push(ch);
        }

        let Some(suffix_start) = suffix_start else {
            cursor = key_end;
            continue;
        };

        let after_sep_start = key_end + suffix_start;
        let after_sep = &input[after_sep_start..];
        let trimmed =
            after_sep.trim_start_matches(|c: char| c.is_whitespace() || c == '"' || c == '\'');
        let skipped = after_sep.len() - trimmed.len();
        out.push_str(&after_sep[..skipped]);

        let end_offset = trimmed
            .find(|c: char| {
                c.is_whitespace() || c == ',' || c == '&' || c == ';' || c == '"' || c == '\''
            })
            .unwrap_or(trimmed.len());

        out.push_str("[REDACTED]");
        cursor = after_sep_start + skipped + end_offset;
    }

    out.push_str(&input[cursor..]);
    out
}

fn find_ascii_case_insensitive(input: &str, needle: &str, start: usize) -> Option<usize> {
    let input = input.as_bytes();
    let needle = needle.as_bytes();
    if needle.is_empty() || start >= input.len() || needle.len() > input.len() {
        return None;
    }

    (start..=input.len().saturating_sub(needle.len()))
        .find(|&idx| input[idx..idx + needle.len()].eq_ignore_ascii_case(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn today_file(prefix: &str) -> String {
        managed_log_file_name(prefix, Utc::now().date_naive())
    }

    #[test]
    fn redacts_secret_like_values() {
        let input = "token=abc123 password: hunter2 api_key=\"sk-test\" safe";
        let output = redact_sensitive(input);
        assert!(!output.contains("abc123"));
        assert!(!output.contains("hunter2"));
        assert!(!output.contains("sk-test"));
        assert!(output.contains("token=[REDACTED]"));
    }

    #[test]
    fn redacts_repeated_secret_like_values() {
        let input = "token=abc123 safe token=def456 password=hunter2 password=again";
        let output = redact_sensitive(input);

        assert!(!output.contains("abc123"));
        assert!(!output.contains("def456"));
        assert!(!output.contains("hunter2"));
        assert!(!output.contains("again"));
        assert_eq!(output.matches("[REDACTED]").count(), 4);
    }

    #[test]
    fn writes_json_event_without_raw_secret() {
        let dir = tempfile::tempdir().unwrap();
        write_log_event_to_dir(
            dir.path(),
            LogEvent {
                level: LogLevel::Info,
                category: LogCategory::Rules,
                action: "inject".to_string(),
                result: Some("ok".to_string()),
                message: Some("token=abc123".to_string()),
                tool_id: Some("dev-tool1".to_string()),
                target_role: Some("tool-rule".to_string()),
                target_path: Some("/tmp/example".to_string()),
                error: None,
            },
        )
        .unwrap();
        let content = fs::read_to_string(dir.path().join(today_file(LOG_FILE_PREFIX))).unwrap();
        assert!(content.contains("\"category\":\"rules\""));
        assert!(content.contains("\"action\":\"inject\""));
        assert!(!content.contains("abc123"));
    }

    #[test]
    fn folds_home_paths_inside_messages() {
        let home = dirs::home_dir().unwrap();
        let input = format!(
            "failed to write {}/.modus/default-rules/RULES.md token=abc123",
            home.to_string_lossy()
        );
        let output = redact_text(&input);

        assert!(output.contains("~/.modus/default-rules/RULES.md"));
        assert!(!output.contains(&home.to_string_lossy().to_string()));
        assert!(!output.contains("abc123"));
    }

    #[test]
    fn prunes_only_managed_daily_log_files() {
        let dir = tempfile::tempdir().unwrap();
        let old = Utc::now().date_naive() - Duration::days(RETAINED_LOG_DAYS + 1);
        fs::write(
            dir.path().join(managed_log_file_name(LOG_FILE_PREFIX, old)),
            "old",
        )
        .unwrap();
        fs::write(dir.path().join("keep.txt"), "not a log").unwrap();

        prune_managed_logs(dir.path(), LOG_FILE_PREFIX).unwrap();

        assert!(dir.path().join("keep.txt").exists());
        assert!(!dir
            .path()
            .join(managed_log_file_name(LOG_FILE_PREFIX, old))
            .exists());
    }

    #[test]
    fn writes_skill_performance_event_to_dedicated_file() {
        let dir = tempfile::tempdir().unwrap();
        write_skill_performance_event_to_dir(
            dir.path(),
            SkillPerformanceLogEvent {
                reason: Some("entry".to_string()),
                status: Some("success".to_string()),
                total_ms: Some(120.4),
                request_count: Some(1),
                request_counts: Some(std::collections::HashMap::from([(
                    "inventory".to_string(),
                    1,
                )])),
                milestones: Some(vec![SkillPerformanceMilestoneLog {
                    name: Some("visible-lists-ready".to_string()),
                    at_ms: Some(80.2),
                }]),
                requests: Some(vec![SkillPerformanceRequestLog {
                    label: Some("inventory".to_string()),
                    status: Some("success".to_string()),
                    duration_ms: Some(77.7),
                    error_kind: None,
                }]),
            },
        )
        .unwrap();

        assert!(dir.path().join(today_file("skill-performance")).exists());
        assert!(!dir.path().join(today_file(LOG_FILE_PREFIX)).exists());
        let content = fs::read_to_string(dir.path().join(today_file("skill-performance"))).unwrap();
        assert!(content.contains("\"reason\":\"entry\""));
        assert!(content.contains("\"label\":\"inventory\""));
    }

    #[test]
    fn writes_module_performance_event_to_dedicated_file() {
        let dir = tempfile::tempdir().unwrap();
        write_module_performance_event_to_dir(
            dir.path(),
            ModulePerformanceLogEvent {
                module: Some("skills".to_string()),
                view: Some("overview".to_string()),
                reason: Some("entry".to_string()),
                status: Some("success".to_string()),
                visible_ms: Some(2.0),
                interactive_ms: Some(5.0),
                background_complete_ms: Some(120.4),
                total_ms: Some(120.4),
                request_count: Some(1),
                request_counts: Some(std::collections::HashMap::from([(
                    "inventory".to_string(),
                    1,
                )])),
                counters: Some(std::collections::HashMap::from([("items".to_string(), 3)])),
                milestones: Some(vec![ModulePerformanceMilestoneLog {
                    name: Some("visible-lists-ready".to_string()),
                    role: Some("visible".to_string()),
                    at_ms: Some(2.0),
                }]),
                requests: Some(vec![ModulePerformanceRequestLog {
                    label: Some("inventory".to_string()),
                    status: Some("success".to_string()),
                    duration_ms: Some(77.7),
                    error_kind: None,
                }]),
            },
        )
        .unwrap();

        assert!(dir
            .path()
            .join(today_file(MODULE_PERFORMANCE_LOG_PREFIX))
            .exists());
        assert!(!dir.path().join(today_file(LOG_FILE_PREFIX)).exists());
        assert!(!dir.path().join(today_file("skill-performance")).exists());
        let content =
            fs::read_to_string(dir.path().join(today_file(MODULE_PERFORMANCE_LOG_PREFIX))).unwrap();
        assert!(content.contains("\"module\":\"skills\""));
        assert!(content.contains("\"visibleMs\":2.0"));
        assert!(content.contains("\"backgroundCompleteMs\":120.4"));
        assert!(content.contains("\"label\":\"inventory\""));
    }

    #[test]
    fn skips_skill_performance_log_when_disabled() {
        let dir = tempfile::tempdir().unwrap();
        write_skill_performance_event_to_dir_if_enabled(
            dir.path(),
            SkillPerformanceLogEvent {
                reason: Some("entry".to_string()),
                ..SkillPerformanceLogEvent::default()
            },
            false,
        )
        .unwrap();

        assert!(!dir.path().join(today_file("skill-performance")).exists());
    }

    #[test]
    fn skips_module_performance_log_when_disabled() {
        let dir = tempfile::tempdir().unwrap();
        write_module_performance_event_to_dir_if_enabled(
            dir.path(),
            ModulePerformanceLogEvent {
                module: Some("skills".to_string()),
                reason: Some("entry".to_string()),
                ..ModulePerformanceLogEvent::default()
            },
            false,
        )
        .unwrap();

        assert!(!dir
            .path()
            .join(today_file(MODULE_PERFORMANCE_LOG_PREFIX))
            .exists());
    }

    #[test]
    fn skill_performance_log_redacts_paths_and_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let home = dirs::home_dir().unwrap();
        let home_path = format!("{}/.agents/skills/demo", home.to_string_lossy());
        write_skill_performance_event_to_dir_if_enabled(
            dir.path(),
            SkillPerformanceLogEvent {
                reason: Some(home_path.clone()),
                status: Some("success token=abc123".to_string()),
                request_counts: Some(std::collections::HashMap::from([(home_path.clone(), 1)])),
                milestones: Some(vec![SkillPerformanceMilestoneLog {
                    name: Some(home_path.clone()),
                    at_ms: Some(1.0),
                }]),
                requests: Some(vec![SkillPerformanceRequestLog {
                    label: Some(home_path),
                    status: Some("success".to_string()),
                    duration_ms: Some(2.0),
                    error_kind: Some("Error token=abc123".to_string()),
                }]),
                ..SkillPerformanceLogEvent::default()
            },
            true,
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join(today_file("skill-performance"))).unwrap();
        assert!(content.contains("[REDACTED_PATH]"));
        assert!(!content.contains(&home.to_string_lossy().to_string()));
        assert!(!content.contains("abc123"));
    }

    #[test]
    fn module_performance_log_redacts_paths_and_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let home = dirs::home_dir().unwrap();
        let home_path = format!("{}/.agents/skills/demo", home.to_string_lossy());
        write_module_performance_event_to_dir_if_enabled(
            dir.path(),
            ModulePerformanceLogEvent {
                module: Some(home_path.clone()),
                view: Some(home_path.clone()),
                reason: Some(home_path.clone()),
                status: Some("success token=abc123".to_string()),
                request_counts: Some(std::collections::HashMap::from([(home_path.clone(), 1)])),
                counters: Some(std::collections::HashMap::from([(home_path.clone(), 1)])),
                milestones: Some(vec![ModulePerformanceMilestoneLog {
                    name: Some(home_path.clone()),
                    role: Some("visible token=abc123".to_string()),
                    at_ms: Some(1.0),
                }]),
                requests: Some(vec![ModulePerformanceRequestLog {
                    label: Some(home_path),
                    status: Some("success".to_string()),
                    duration_ms: Some(2.0),
                    error_kind: Some("Error token=abc123".to_string()),
                }]),
                ..ModulePerformanceLogEvent::default()
            },
            true,
        )
        .unwrap();

        let content =
            fs::read_to_string(dir.path().join(today_file(MODULE_PERFORMANCE_LOG_PREFIX))).unwrap();
        assert!(content.contains("[REDACTED_PATH]"));
        assert!(!content.contains(&home.to_string_lossy().to_string()));
        assert!(!content.contains("abc123"));
    }

    #[test]
    fn retains_recent_skill_performance_logs_separately() {
        let dir = tempfile::tempdir().unwrap();
        let old = Utc::now().date_naive() - Duration::days(RETAINED_LOG_DAYS + 1);
        fs::write(
            dir.path()
                .join(managed_log_file_name("skill-performance", old)),
            "old",
        )
        .unwrap();
        fs::write(dir.path().join(today_file(LOG_FILE_PREFIX)), "ordinary log").unwrap();

        write_skill_performance_event_to_dir(
            dir.path(),
            SkillPerformanceLogEvent {
                reason: Some("manual-refresh".to_string()),
                ..SkillPerformanceLogEvent::default()
            },
        )
        .unwrap();

        assert!(!dir
            .path()
            .join(managed_log_file_name("skill-performance", old))
            .exists());
        assert!(dir.path().join(today_file("skill-performance")).exists());
        assert!(dir.path().join(today_file(LOG_FILE_PREFIX)).exists());
    }

    #[test]
    fn retains_recent_module_performance_logs_separately() {
        let dir = tempfile::tempdir().unwrap();
        let old = Utc::now().date_naive() - Duration::days(RETAINED_LOG_DAYS + 1);
        fs::write(
            dir.path()
                .join(managed_log_file_name(MODULE_PERFORMANCE_LOG_PREFIX, old)),
            "old",
        )
        .unwrap();
        fs::write(dir.path().join(today_file(LOG_FILE_PREFIX)), "ordinary log").unwrap();
        fs::write(
            dir.path().join(today_file("skill-performance")),
            "old skill log",
        )
        .unwrap();

        write_module_performance_event_to_dir(
            dir.path(),
            ModulePerformanceLogEvent {
                module: Some("settings".to_string()),
                reason: Some("manual-refresh".to_string()),
                ..ModulePerformanceLogEvent::default()
            },
        )
        .unwrap();

        assert!(!dir
            .path()
            .join(managed_log_file_name(MODULE_PERFORMANCE_LOG_PREFIX, old))
            .exists());
        assert!(dir
            .path()
            .join(today_file(MODULE_PERFORMANCE_LOG_PREFIX))
            .exists());
        assert!(dir.path().join(today_file(LOG_FILE_PREFIX)).exists());
        assert!(dir.path().join(today_file("skill-performance")).exists());
    }

    #[test]
    fn lists_and_reads_managed_log_files_without_unknown_files() {
        let dir = tempfile::tempdir().unwrap();
        let today = today_file(LOG_FILE_PREFIX);
        fs::write(dir.path().join(&today), "hello").unwrap();
        fs::write(dir.path().join("app.log"), "legacy single file").unwrap();

        let logs = list_managed_logs_for_prefix(dir.path(), LOG_FILE_PREFIX, false).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].id, today);

        let read = read_managed_log(dir.path(), LOG_FILE_PREFIX, &logs[0].id).unwrap();
        assert_eq!(read.content, "hello");
        assert!(!read.truncated);
    }
}
