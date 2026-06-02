import { describe, expect, it } from "vitest";
import { normalizeLogEvent, redactLogValue } from "$lib/shared/logging/appLogger.js";

describe("appLogger", () => {
  it("redacts common secret-like key values", () => {
    const value = redactLogValue('token=abc123 password: hunter2 api_key="sk-test"');

    expect(value).not.toContain("abc123");
    expect(value).not.toContain("hunter2");
    expect(value).not.toContain("sk-test");
    expect(value).toContain("token=[REDACTED]");
  });

  it("normalizes invalid level/category without preserving sensitive values", () => {
    const event = normalizeLogEvent({
      level: "verbose",
      category: "unknown",
      action: "inject",
      error: "secret=my-secret failed",
    });

    expect(event.level).toBe("info");
    expect(event.category).toBe("system");
    expect(event.error).not.toContain("my-secret");
  });
});
