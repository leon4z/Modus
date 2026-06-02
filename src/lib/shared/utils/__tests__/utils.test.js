import { describe, expect, it } from "vitest";
import { formatPath } from "$lib/shared/utils/utils.js";

describe("formatPath", () => {
  it("将 /Users/xxx 替换为 ~", () => {
    expect(formatPath("/Users/leon/projects/foo")).toBe("~/projects/foo");
  });

  it("保留用户目录后的路径结构", () => {
    expect(formatPath("/Users/leon/.codex/skills/doc/SKILL.md")).toBe("~/.codex/skills/doc/SKILL.md");
  });

  it("非用户目录路径保持原样", () => {
    expect(formatPath("/opt/homebrew/bin/foo")).toBe("/opt/homebrew/bin/foo");
  });

  it("空路径返回空字符串", () => {
    expect(formatPath("")).toBe("");
    expect(formatPath(null)).toBe("");
  });
});
