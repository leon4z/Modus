// Purpose: Own Modus application update channel, state, and installer workflows.

pub mod commands;

use crate::platform::config::{self, AppAvailableUpdate, AppUpdateState};
use crate::platform::env::{self, RuntimeUpdateChannel};
use crate::platform::logging::{self, LogCategory, LogEvent, LogLevel};
use chrono::{DateTime, Duration, Utc};
use semver::Version;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_updater::{Update, UpdaterExt};

const STABLE_UPDATE_ENDPOINT: &str =
    "https://github.com/leon4z/Modus/releases/latest/download/latest.json";
const TEST_UPDATE_ENDPOINT: &str =
    "https://github.com/leon4z/Modus/releases/download/modus-test/latest.json";
const STARTUP_CHECK_INTERVAL_HOURS: i64 = 24;

pub struct PendingAppUpdate(pub Mutex<Option<Update>>);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateViewState {
    pub status: String,
    pub channel: String,
    pub can_check: bool,
    pub source: Option<String>,
    pub current_version: String,
    pub available_update: Option<AppAvailableUpdate>,
    pub last_startup_check_at: Option<String>,
    pub last_successful_check_at: Option<String>,
    pub last_failure_at: Option<String>,
    pub last_failure_summary: Option<String>,
}

pub fn pending_app_update_state() -> PendingAppUpdate {
    PendingAppUpdate(Mutex::new(None))
}

pub fn get_app_update_state_domain(app_version: String) -> AppUpdateViewState {
    view_state_from_persisted(
        "idle",
        &load_current_app_update_state(&app_version),
        app_version,
    )
}

pub async fn check_app_update_domain(
    app: AppHandle,
    pending_update: State<'_, PendingAppUpdate>,
    reason: String,
) -> Result<AppUpdateViewState, String> {
    run_update_check(app, &pending_update, normalize_check_reason(&reason)).await
}

pub async fn install_app_update_domain(
    app: AppHandle,
    pending_update: State<'_, PendingAppUpdate>,
) -> Result<AppUpdateViewState, String> {
    let app_version = app.package_info().version.to_string();
    if !current_channel().can_check {
        return Ok(get_app_update_state_with_status("disabled", app_version));
    }

    let update = {
        let guard = pending_update
            .0
            .lock()
            .map_err(|_| "pending update lock poisoned".to_string())?;
        guard.clone()
    };

    let update = if let Some(update) = update {
        update
    } else {
        let checked = run_update_check(app, &pending_update, CheckReason::Manual).await?;
        if checked.available_update.is_none() {
            return Ok(checked);
        }
        let guard = pending_update
            .0
            .lock()
            .map_err(|_| "pending update lock poisoned".to_string())?;
        guard
            .clone()
            .ok_or_else(|| "no pending update is available".to_string())?
    };

    write_update_log("app_update_install", "started", None);
    let install_result = update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|error| error.to_string());

    match install_result {
        Ok(()) => {
            write_update_log("app_update_install", "ok", None);
            let persisted = load_current_app_update_state(&app_version);
            Ok(view_state_from_persisted(
                "restart_needed",
                &persisted,
                app_version,
            ))
        }
        Err(error) => {
            write_update_failure("app_update_install", &error)?;
            Ok(get_app_update_state_with_status("failed", app_version))
        }
    }
}

pub fn skip_app_update_domain(
    app: AppHandle,
    pending_update: State<'_, PendingAppUpdate>,
) -> Result<AppUpdateViewState, String> {
    let app_version = app.package_info().version.to_string();
    if !current_channel().can_check {
        clear_pending_update(&pending_update)?;
        return Ok(get_app_update_state_with_status("disabled", app_version));
    }

    let mut persisted = load_current_app_update_state(&app_version);
    if let Some(available) = persisted.available_update.clone() {
        persisted.skipped_update_version = Some(available.version);
        persisted.available_update = None;
        persisted.last_failure_at = None;
        persisted.last_failure_summary = None;
        persist_app_update_state(persisted.clone())?;
        clear_pending_update(&pending_update)?;
        write_update_log("app_update_skip", "ok", None);
    }

    Ok(view_state_from_persisted(
        skip_app_update_status(&persisted),
        &persisted,
        app_version,
    ))
}

pub fn restart_app_for_update_domain(app: AppHandle) {
    app.restart();
}

async fn run_update_check(
    app: AppHandle,
    pending_update: &State<'_, PendingAppUpdate>,
    reason: CheckReason,
) -> Result<AppUpdateViewState, String> {
    let app_version = app.package_info().version.to_string();
    let channel = current_channel();
    if !channel.can_check {
        clear_pending_update(pending_update)?;
        return Ok(get_app_update_state_with_status("disabled", app_version));
    }

    let mut persisted = load_current_app_update_state(&app_version);
    if reason == CheckReason::Startup && !startup_check_due(&persisted, Utc::now()) {
        return Ok(view_state_from_persisted("idle", &persisted, app_version));
    }

    let now = Utc::now().to_rfc3339();
    if reason == CheckReason::Startup {
        persisted.last_startup_check_at = Some(now.clone());
        persist_app_update_state(persisted.clone())?;
    }

    write_update_log("app_update_check", reason.as_log_result(), None);

    let endpoint = channel
        .source
        .clone()
        .ok_or_else(|| "update source is unavailable".to_string())?;
    let endpoint = tauri::Url::parse(&endpoint).map_err(|error| error.to_string())?;
    let update = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|error| error.to_string())?
        .build()
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| error.to_string());

    match update {
        Ok(Some(update)) => {
            let available = update_metadata(&update, &channel.channel);
            persisted.last_successful_check_at = Some(Utc::now().to_rfc3339());
            persisted.last_failure_at = None;
            persisted.last_failure_summary = None;
            if skipped_update_matches(&persisted, &available) {
                persisted.available_update = None;
                persist_app_update_state(persisted.clone())?;
                clear_pending_update(pending_update)?;
                write_update_log("app_update_check", "skipped", None);
                Ok(view_state_from_persisted(
                    "skipped",
                    &persisted,
                    app_version,
                ))
            } else {
                persisted.skipped_update_version = None;
                persisted.available_update = Some(available);
                persist_app_update_state(persisted.clone())?;
                replace_pending_update(pending_update, Some(update))?;
                write_update_log("app_update_check", "available", None);
                Ok(view_state_from_persisted(
                    "available",
                    &persisted,
                    app_version,
                ))
            }
        }
        Ok(None) => {
            persisted.available_update = None;
            persisted.last_successful_check_at = Some(Utc::now().to_rfc3339());
            persisted.last_failure_at = None;
            persisted.last_failure_summary = None;
            persist_app_update_state(persisted.clone())?;
            clear_pending_update(pending_update)?;
            write_update_log("app_update_check", "current", None);
            Ok(view_state_from_persisted(
                "current",
                &persisted,
                app_version,
            ))
        }
        Err(error) => {
            persisted.last_failure_at = Some(Utc::now().to_rfc3339());
            persisted.last_failure_summary = Some(error.clone());
            persist_app_update_state(persisted.clone())?;
            write_update_log("app_update_check", "failed", Some(error));
            Ok(view_state_from_persisted("failed", &persisted, app_version))
        }
    }
}

fn current_channel() -> ChannelInfo {
    channel_info(env::runtime_shape().update_channel())
}

fn channel_info(channel: RuntimeUpdateChannel) -> ChannelInfo {
    match channel {
        RuntimeUpdateChannel::Disabled => ChannelInfo {
            channel: "disabled".to_string(),
            can_check: false,
            source: None,
        },
        RuntimeUpdateChannel::Test => ChannelInfo {
            channel: "test".to_string(),
            can_check: true,
            source: Some(TEST_UPDATE_ENDPOINT.to_string()),
        },
        RuntimeUpdateChannel::Stable => ChannelInfo {
            channel: "stable".to_string(),
            can_check: true,
            source: Some(STABLE_UPDATE_ENDPOINT.to_string()),
        },
    }
}

fn view_state_from_persisted(
    status: &str,
    persisted: &AppUpdateState,
    app_version: String,
) -> AppUpdateViewState {
    let channel = current_channel();
    AppUpdateViewState {
        status: status.to_string(),
        channel: channel.channel,
        can_check: channel.can_check,
        source: channel.source,
        current_version: app_version,
        available_update: persisted.available_update.clone(),
        last_startup_check_at: persisted.last_startup_check_at.clone(),
        last_successful_check_at: persisted.last_successful_check_at.clone(),
        last_failure_at: persisted.last_failure_at.clone(),
        last_failure_summary: persisted.last_failure_summary.clone(),
    }
}

fn get_app_update_state_with_status(status: &str, app_version: String) -> AppUpdateViewState {
    view_state_from_persisted(
        status,
        &load_current_app_update_state(&app_version),
        app_version,
    )
}

fn load_current_app_update_state(app_version: &str) -> AppUpdateState {
    let mut state = config::load_config().app_update_state;
    if prune_installed_available_update(&mut state, app_version) {
        let _ = persist_app_update_state(state.clone());
    }
    state
}

fn prune_installed_available_update(state: &mut AppUpdateState, app_version: &str) -> bool {
    let mut changed = false;

    if let Some(available) = state.available_update.as_ref() {
        if available_update_is_installed_or_older(available, app_version) {
            state.available_update = None;
            changed = true;
        }
    }

    if let Some(skipped_version) = state.skipped_update_version.as_deref() {
        if version_is_installed_or_older(skipped_version, app_version) {
            state.skipped_update_version = None;
            changed = true;
        }
    }

    changed
}

fn available_update_is_installed_or_older(
    available: &AppAvailableUpdate,
    app_version: &str,
) -> bool {
    version_is_installed_or_older(&available.version, app_version)
}

fn version_is_installed_or_older(candidate_version: &str, app_version: &str) -> bool {
    if candidate_version == app_version {
        return true;
    }

    let Ok(current_version) = Version::parse(app_version) else {
        return false;
    };
    let Ok(candidate_version) = Version::parse(candidate_version) else {
        return false;
    };

    current_version >= candidate_version
}

fn skipped_update_matches(state: &AppUpdateState, available: &AppAvailableUpdate) -> bool {
    state.skipped_update_version.as_deref() == Some(available.version.as_str())
}

fn skip_app_update_status(state: &AppUpdateState) -> &'static str {
    if state.skipped_update_version.is_some() {
        "skipped"
    } else {
        "current"
    }
}

fn persist_app_update_state(next: AppUpdateState) -> Result<(), String> {
    config::update_config(|config| {
        config.app_update_state = next;
        Ok(())
    })
    .map(|_| ())
}

fn update_metadata(update: &Update, channel: &str) -> AppAvailableUpdate {
    AppAvailableUpdate {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        channel: channel.to_string(),
        date: update.date.map(|date| date.to_string()),
        body: update.body.clone(),
    }
}

fn replace_pending_update(
    pending_update: &State<'_, PendingAppUpdate>,
    update: Option<Update>,
) -> Result<(), String> {
    let mut guard = pending_update
        .0
        .lock()
        .map_err(|_| "pending update lock poisoned".to_string())?;
    *guard = update;
    Ok(())
}

fn clear_pending_update(pending_update: &State<'_, PendingAppUpdate>) -> Result<(), String> {
    replace_pending_update(pending_update, None)
}

fn startup_check_due(state: &AppUpdateState, now: DateTime<Utc>) -> bool {
    let Some(last) = state.last_startup_check_at.as_deref() else {
        return true;
    };
    let Ok(last) = DateTime::parse_from_rfc3339(last) else {
        return true;
    };
    now.signed_duration_since(last.with_timezone(&Utc))
        >= Duration::hours(STARTUP_CHECK_INTERVAL_HOURS)
}

fn write_update_failure(action: &str, error: &str) -> Result<(), String> {
    let mut state = config::load_config().app_update_state;
    state.last_failure_at = Some(Utc::now().to_rfc3339());
    state.last_failure_summary = Some(error.to_string());
    persist_app_update_state(state)?;
    write_update_log(action, "failed", Some(error.to_string()));
    Ok(())
}

fn write_update_log(action: &str, result: &str, error: Option<String>) {
    let _ = logging::write_log_event(LogEvent {
        level: if result == "failed" {
            LogLevel::Warn
        } else {
            LogLevel::Info
        },
        category: LogCategory::System,
        action: action.to_string(),
        result: Some(result.to_string()),
        message: None,
        tool_id: None,
        target_role: Some("app_update".to_string()),
        target_path: None,
        error,
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChannelInfo {
    channel: String,
    can_check: bool,
    source: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckReason {
    Startup,
    Manual,
}

impl CheckReason {
    fn as_log_result(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Manual => "manual",
        }
    }
}

fn normalize_check_reason(reason: &str) -> CheckReason {
    if reason.trim().eq_ignore_ascii_case("startup") {
        CheckReason::Startup
    } else {
        CheckReason::Manual
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_resolve_to_separate_sources() {
        let stable = channel_info(RuntimeUpdateChannel::Stable);
        let test = channel_info(RuntimeUpdateChannel::Test);
        let disabled = channel_info(RuntimeUpdateChannel::Disabled);

        assert!(stable.can_check);
        assert!(test.can_check);
        assert!(!disabled.can_check);
        assert_eq!(stable.channel, "stable");
        assert_eq!(test.channel, "test");
        assert_ne!(stable.source, test.source);
        assert!(stable.source.unwrap().contains("releases/latest/download"));
        assert!(test
            .source
            .unwrap()
            .contains("releases/download/modus-test"));
        assert!(disabled.source.is_none());
    }

    #[test]
    fn startup_cadence_uses_runtime_local_timestamp() {
        let now = DateTime::parse_from_rfc3339("2026-06-01T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut state = AppUpdateState::default();

        assert!(startup_check_due(&state, now));

        state.last_startup_check_at = Some("2026-06-01T09:30:00Z".to_string());
        assert!(!startup_check_due(&state, now));

        state.last_startup_check_at = Some("2026-05-31T09:00:00Z".to_string());
        assert!(startup_check_due(&state, now));

        state.last_startup_check_at = Some("not-a-date".to_string());
        assert!(startup_check_due(&state, now));
    }

    #[test]
    fn view_state_retains_available_update_on_failure() {
        let state = AppUpdateState {
            available_update: Some(AppAvailableUpdate {
                version: "0.2.0".to_string(),
                current_version: "0.1.0".to_string(),
                channel: "stable".to_string(),
                date: None,
                body: None,
            }),
            last_failure_summary: Some("network failed".to_string()),
            ..AppUpdateState::default()
        };

        let view = view_state_from_persisted("failed", &state, "0.1.0".to_string());

        assert_eq!(view.status, "failed");
        assert_eq!(view.available_update.unwrap().version, "0.2.0");
        assert_eq!(view.last_failure_summary.as_deref(), Some("network failed"));
    }

    #[test]
    fn installed_available_update_is_pruned_from_view_state() {
        let mut state = AppUpdateState {
            available_update: Some(AppAvailableUpdate {
                version: "0.1.1-test.1".to_string(),
                current_version: "0.1.0".to_string(),
                channel: "test".to_string(),
                date: None,
                body: None,
            }),
            ..AppUpdateState::default()
        };

        assert!(prune_installed_available_update(&mut state, "0.1.1-test.1"));
        assert!(state.available_update.is_none());
    }

    #[test]
    fn installed_skipped_update_version_is_pruned_from_view_state() {
        let mut state = AppUpdateState {
            skipped_update_version: Some("0.1.1-test.1".to_string()),
            ..AppUpdateState::default()
        };

        assert!(prune_installed_available_update(&mut state, "0.1.1-test.1"));
        assert!(state.skipped_update_version.is_none());
    }

    #[test]
    fn newer_available_update_is_retained_in_view_state() {
        let mut state = AppUpdateState {
            available_update: Some(AppAvailableUpdate {
                version: "0.1.2".to_string(),
                current_version: "0.1.1".to_string(),
                channel: "stable".to_string(),
                date: None,
                body: None,
            }),
            ..AppUpdateState::default()
        };

        assert!(!prune_installed_available_update(&mut state, "0.1.1"));
        assert_eq!(state.available_update.unwrap().version, "0.1.2");
    }

    #[test]
    fn skipped_update_matches_only_the_same_available_version() {
        let mut state = AppUpdateState {
            skipped_update_version: Some("0.1.2".to_string()),
            ..AppUpdateState::default()
        };
        let skipped = AppAvailableUpdate {
            version: "0.1.2".to_string(),
            current_version: "0.1.1".to_string(),
            channel: "stable".to_string(),
            date: None,
            body: None,
        };
        let newer = AppAvailableUpdate {
            version: "0.1.3".to_string(),
            current_version: "0.1.1".to_string(),
            channel: "stable".to_string(),
            date: None,
            body: None,
        };

        assert!(skipped_update_matches(&state, &skipped));
        assert!(!skipped_update_matches(&state, &newer));
        assert!(!prune_installed_available_update(&mut state, "0.1.1"));
        assert_eq!(state.skipped_update_version.as_deref(), Some("0.1.2"));
    }

    #[test]
    fn repeated_skip_status_stays_skipped_while_version_is_remembered() {
        let skipped = AppUpdateState {
            skipped_update_version: Some("0.1.2".to_string()),
            ..AppUpdateState::default()
        };
        let current = AppUpdateState::default();

        assert_eq!(skip_app_update_status(&skipped), "skipped");
        assert_eq!(skip_app_update_status(&current), "current");
    }
}
