// Preserve the Settings page sub-tab across view switches while the module owns the rest of the page state.
import { writable } from "svelte/store";

export const activeSettingsTab = writable("general");
export const focusedSettingsToolId = writable(null);
