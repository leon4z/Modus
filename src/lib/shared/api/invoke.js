// Purpose: Provide the shared Tauri invoke boundary for domain API modules.
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { getVisualInvokeResponse, isVisualVerificationMode } from "$lib/dev/visualVerification/fixtures.js";

/**
 * @param {string} command
 * @param {Record<string, any>} [args]
 */
export async function invoke(command, args = {}) {
  if (isVisualVerificationMode()) {
    return getVisualInvokeResponse(command, args);
  }
  return tauriInvoke(command, args);
}
