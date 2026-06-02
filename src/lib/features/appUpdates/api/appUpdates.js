// Purpose: Keep Modus app update command wrappers in one frontend domain API.
import { invoke } from "$lib/shared/api/invoke.js";

export async function getAppUpdateState() {
  return invoke("get_app_update_state");
}

/**
 * @param {"startup" | "manual"} reason
 */
export async function checkAppUpdate(reason) {
  return invoke("check_app_update", { reason });
}

export async function installAppUpdate() {
  return invoke("install_app_update");
}

export async function restartAppForUpdate() {
  return invoke("restart_app_for_update");
}
