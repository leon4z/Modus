// Purpose: Tauri command adapters for Modus application updates.

use super::*;

#[tauri::command]
pub fn get_app_update_state(app: AppHandle) -> AppUpdateViewState {
    get_app_update_state_domain(app.package_info().version.to_string())
}

#[tauri::command]
pub async fn check_app_update(
    app: AppHandle,
    pending_update: State<'_, PendingAppUpdate>,
    reason: String,
) -> Result<AppUpdateViewState, String> {
    check_app_update_domain(app, pending_update, reason).await
}

#[tauri::command]
pub async fn install_app_update(
    app: AppHandle,
    pending_update: State<'_, PendingAppUpdate>,
) -> Result<AppUpdateViewState, String> {
    install_app_update_domain(app, pending_update).await
}

#[tauri::command]
pub fn skip_app_update(
    app: AppHandle,
    pending_update: State<'_, PendingAppUpdate>,
) -> Result<AppUpdateViewState, String> {
    skip_app_update_domain(app, pending_update)
}

#[tauri::command]
pub fn restart_app_for_update(app: AppHandle) {
    restart_app_for_update_domain(app);
}
