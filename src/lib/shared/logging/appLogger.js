import { writeApplicationLog } from "$lib/shared/logging/api.js";

const VALID_LEVELS = new Set(["debug", "info", "warn", "error"]);
const VALID_CATEGORIES = new Set(["rules", "skills", "settings", "system"]);

/**
 * @param {string} value
 */
export function redactLogValue(value) {
  return String(value ?? "")
    .replace(/(password|passwd|secret|token|api[_-]?key|access[_-]?key)\s*[:=]\s*["']?[^"',\s;&]+/gi, "$1=[REDACTED]");
}

/**
 * @param {any} event
 */
export function normalizeLogEvent(event) {
  const input = event && typeof event === "object" ? event : {};
  const level = VALID_LEVELS.has(input.level) ? input.level : "info";
  const category = VALID_CATEGORIES.has(input.category) ? input.category : "system";
  const action = String(input.action || "unknown");

  /** @param {any} value */
  const optionalString = (value) => value == null ? undefined : redactLogValue(String(value));

  return {
    level,
    category,
    action: redactLogValue(action),
    result: optionalString(input.result),
    message: optionalString(input.message),
    toolId: optionalString(input.toolId),
    targetRole: optionalString(input.targetRole),
    targetPath: optionalString(input.targetPath),
    error: optionalString(input.error),
  };
}

/**
 * Fire-and-forget application logging. Logging failure must not block the user operation.
 * @param {any} event
 */
export async function logAppEvent(event) {
  try {
    await writeApplicationLog(normalizeLogEvent(event));
  } catch (_) {
    // Logging is intentionally non-blocking; the original user action owns its result.
  }
}
