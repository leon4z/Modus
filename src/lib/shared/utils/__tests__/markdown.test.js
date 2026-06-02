import { describe, expect, it } from "vitest";
import { renderSafeMarkdown } from "../markdown.js";

describe("renderSafeMarkdown", () => {
  it("escapes raw html blocks", () => {
    const html = renderSafeMarkdown("<script>alert(1)</script>");
    expect(html).not.toContain("<script>");
    expect(html).toContain("&lt;script&gt;alert(1)&lt;/script&gt;");
  });

  it("drops unsafe javascript links", () => {
    const html = renderSafeMarkdown("[click](javascript:alert(1))");
    expect(html).toContain(">click<");
    expect(html).not.toContain("javascript:alert");
    expect(html).not.toContain("<a ");
  });

  it("keeps safe https links with hardened target attrs", () => {
    const html = renderSafeMarkdown("[docs](https://example.com)");
    expect(html).toContain('href="https://example.com"');
    expect(html).toContain('target="_blank"');
    expect(html).toContain('rel="noopener noreferrer"');
  });
});
