// Purpose: Keep dashboard command wrappers in one frontend domain API.
import { invoke } from "$lib/shared/api/invoke.js";

export async function getDashboard() {
  return invoke("get_dashboard");
}
