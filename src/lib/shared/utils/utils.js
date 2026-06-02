/**
 * Format an absolute path for UI display.
 * 1. Replaces the user's home directory with `~`
 * 2. Optionally trims the middle of very long paths if max length is provided
 */
/** @param {any} pathStr */
export function formatPath(pathStr, maxLength = 80) {
    if (!pathStr) return "";
    
    let formatted = String(pathStr);
    
    // Keep the path structure intact while folding the user-home prefix.
    formatted = formatted.replace(/^(\/Users\/[^\/]+|\/home\/[^\/]+)/, "~");

    // Replace Windows home dir "C:\Users\username" -> "~"
    formatted = formatted.replace(/^[A-Z]:\\Users\\[^\\]+/i, "~");

    // Normalize slashes (windows to unix style for display if wanted, optional)
    formatted = formatted.replace(/\\/g, "/");

    // Let CSS text-overflow handle all truncation visually

    return formatted;
}

/**
 * Convert YAML frontmatter to a markdown code block so it renders nicely in reading mode.
 */
/** @param {any} text */
export function formatFrontmatterForMarkdown(text) {
    if (!text || !text.startsWith("---")) return text || "";
    // Match closing --- on its own line, compatible with LF and CRLF
    const match = text.match(/\r?\n---[ \t]*(?:\r?\n|$)/);
    if (!match) return text;
    
    const frontmatterLines = String(text
        .slice(3, match.index)
        .trim())
        .split(/\r?\n/);
    const frontmatter = frontmatterLines
        .filter((/** @type {string} */ line) => !/^\s*version\s*:/i.test(line))
        .join("\n")
        .trim();
    const rest = text.slice(match.index + match[0].length);

    // Version is update-entry metadata; the normal Skill detail read view keeps it out of sight.
    if (!frontmatter) return rest;
    return "```yaml\n" + frontmatter + "\n```\n\n" + rest;
}
