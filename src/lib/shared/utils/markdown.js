// Purpose: render markdown for UI preview while blocking unsafe raw HTML and dangerous URLs.
import { Marked } from "marked";

const SAFE_SCHEMES = new Set(["http:", "https:", "mailto:", "tel:"]);

/**
 * @param {unknown} value
 */
function toText(value) {
  return typeof value === "string" ? value : value == null ? "" : String(value);
}

/**
 * @param {unknown} value
 */
function escapeHtml(value) {
  return toText(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

/**
 * @param {string} href
 */
function sanitizeUrl(href) {
  const value = toText(href).trim();
  if (!value) return null;
  if (value.startsWith("#")) return value;
  if (value.startsWith("/")) return value;
  if (value.startsWith("./") || value.startsWith("../")) return value;
  if (value.startsWith("?")) return value;

  try {
    const parsed = new URL(value, "https://local.invalid");
    if (!SAFE_SCHEMES.has(parsed.protocol)) return null;
    return value;
  } catch {
    return null;
  }
}

/**
 * @this {any}
 * @param {import("marked").Tokens.Link} token
 */
function renderSafeLink(token) {
  const text = token.tokens?.length ? this.parser.parseInline(token.tokens) : escapeHtml(token.href || "");
  const href = sanitizeUrl(token.href);
  if (!href) return text;
  const safeHref = escapeHtml(href);
  const safeTitle = token.title ? ` title="${escapeHtml(token.title)}"` : "";
  const external = href.startsWith("http://") || href.startsWith("https://");
  const rel = external ? ' target="_blank" rel="noopener noreferrer"' : "";
  return `<a href="${safeHref}"${safeTitle}${rel}>${text}</a>`;
}

/**
 * @this {any}
 * @param {import("marked").Tokens.Image} token
 */
function renderSafeImage(token) {
  const src = sanitizeUrl(token.href);
  if (!src) return escapeHtml(token.text || "");
  const safeSrc = escapeHtml(src);
  const safeAlt = escapeHtml(token.text || "");
  const safeTitle = token.title ? ` title="${escapeHtml(token.title)}"` : "";
  return `<img src="${safeSrc}" alt="${safeAlt}"${safeTitle}>`;
}

const safeRenderer = {
  /**
   * Keep raw HTML as text so markdown preview cannot inject executable DOM.
   * @param {import("marked").Tokens.HTML | import("marked").Tokens.Tag} token
   */
  html(token) {
    return escapeHtml(token.text);
  },
  link: renderSafeLink,
  image: renderSafeImage,
};

const safeMarkdown = new Marked({
  gfm: true,
  breaks: true,
  renderer: safeRenderer,
});

/**
 * @param {unknown} markdown
 */
export function renderSafeMarkdown(markdown) {
  return String(safeMarkdown.parse(toText(markdown)));
}
