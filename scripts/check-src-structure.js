#!/usr/bin/env node

// Purpose: enforce frontend and backend source ownership boundaries.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";

function trackedFiles(root) {
  return execFileSync("git", ["ls-files", "--cached", "--others", "--exclude-standard", root], {
    encoding: "utf8",
  })
  .split("\n")
  .map((line) => line.trim())
  .filter((file) => file && existsSync(file));
}

const frontendSourceFiles = trackedFiles("src");
const backendSourceFiles = trackedFiles("src-tauri/src");

const forbiddenLegacyFiles = new Set(["src/App.svelte", "src/lib/api.js"]);
const legacyRoots = ["src/lib/components/", "src/lib/api/", "src/lib/stores/", "src/lib/__tests__/", "src/routes/__tests__/"];
const sourceExtensions = /\.(js|svelte)$/;
const violations = [];

for (const file of frontendSourceFiles) {
  if (forbiddenLegacyFiles.has(file) || legacyRoots.some((root) => file.startsWith(root))) {
    violations.push(
      `${file}: new frontend business code must live under src/lib/app, src/lib/features, src/lib/shared, or src/lib/dev`
    );
  }
}

const featureImportPattern =
  /(?:from\s+["']|import\s*\(\s*["'])\$lib\/features\/([^/"']+)\/([^"']+)["']/g;
const internalSegments = new Set(["api", "components", "domain", "queries", "stores"]);

for (const file of frontendSourceFiles.filter((path) => sourceExtensions.test(path))) {
  let content = "";
  try {
    content = readFileSync(file, "utf8");
  } catch {
    continue;
  }

  const importerMatch = file.match(/^src\/lib\/features\/([^/]+)\//);
  const importerFeature = importerMatch ? importerMatch[1] : null;
  let match;
  while ((match = featureImportPattern.exec(content)) !== null) {
    const [, targetFeature, rest] = match;
    const firstSegment = rest.split("/")[0];
    if (
      importerFeature &&
      importerFeature !== targetFeature &&
      internalSegments.has(firstSegment)
    ) {
      violations.push(
        `${file}: cross-feature import reaches ${targetFeature}/${firstSegment}; import from the feature public entry or shared layer`
      );
    }
  }
}

const visualShellBypassRules = [
  {
    pattern: /\b(?:config-tabs|tool-tabs)\b/,
    message: "top-level tool/configuration selectors must use the shared level2 selector treatment, not legacy underline tabs",
  },
  {
    pattern: /\.tab\.active\s*\{[^}]*#f59e0b/s,
    message: "top-level active selector color must come from shared shell tokens, not a module-private hard-coded accent",
  },
  {
    pattern: /\.level1-tabs[^{]*\{[^}]*min-height\s*:\s*(?:3[3-9]|[4-9]\d)px/s,
    message: "L1 capsule height must stay on shared shell tokens",
  },
  {
    pattern: /\.l2-tab[^{]*\{[^}]*border-bottom\s*:\s*2px\s+solid/s,
    message: "L2 selectors must stay rectangular neutral controls, not underline tabs",
  },
  {
    pattern: /\.view-header[^{]*\{[^}]*(?:padding-(?:top|bottom)\s*:|padding\s*:)/s,
    message: "top-level feature headers must use the shared primary header band instead of local title-band padding",
  },
  {
    pattern: /\.(?:dashboard-header-bar|view-header)[^{]*\{[^}]*min-height\s*:\s*(?!var\(--view-header-band-min-height\))/s,
    message: "top-level feature headers must use the shared primary header band height",
  },
  {
    pattern: /\.[\w-]*header-actions[^{]*\{[^}]*position\s*:\s*absolute/s,
    message: "top-level feature header actions must fit the shared header rail instead of local absolute positioning",
  },
];

for (const file of frontendSourceFiles.filter((path) => path.startsWith("src/lib/features/") && path.endsWith(".svelte"))) {
  let content = "";
  try {
    content = readFileSync(file, "utf8");
  } catch {
    continue;
  }
  for (const rule of visualShellBypassRules) {
    if (rule.pattern.test(content)) {
      violations.push(`${file}: ${rule.message}`);
    }
  }
}

const allowedBackendRoots = new Set([
  "src-tauri/src/adapters",
  "src-tauri/src/adapters.rs",
  "src-tauri/src/app",
  "src-tauri/src/bin",
  "src-tauri/src/commands",
  "src-tauri/src/domains",
  "src-tauri/src/platform",
  "src-tauri/src/scenario",
  "src-tauri/src/lib.rs",
  "src-tauri/src/main.rs",
]);

for (const file of backendSourceFiles) {
  const parts = file.split("/");
  const ownershipRoot = parts.length >= 4 ? parts.slice(0, 3).join("/") : file;
  if (!allowedBackendRoots.has(file) && !allowedBackendRoots.has(ownershipRoot)) {
    violations.push(
      `${file}: backend runtime code must live under app, commands, domains, platform, adapters, scenario, or bin`
    );
  }
}

for (const file of backendSourceFiles.filter((path) => path.endsWith(".rs"))) {
  let content = "";
  try {
    content = readFileSync(file, "utf8");
  } catch {
    continue;
  }
  const firstMeaningfulLine = content
    .split(/\r?\n/)
    .find((line) => line.trim().length > 0);
  if (!firstMeaningfulLine || !/^\s*(?:\/\/\s*Purpose:|\/\/!)/.test(firstMeaningfulLine)) {
    violations.push(`${file}: backend Rust modules must start with a Purpose comment or module doc`);
  }
}

for (const file of backendSourceFiles.filter((path) => path.startsWith("src-tauri/src/platform/") && path.endsWith(".rs"))) {
  let content = "";
  try {
    content = readFileSync(file, "utf8");
  } catch {
    continue;
  }
  if (/\bcrate::commands\b/.test(content) || /\bcrate::domains\b/.test(content)) {
    violations.push(`${file}: platform code must not depend on commands or product domains`);
  }
}

if (violations.length > 0) {
  console.error("Source structure check failed:");
  for (const violation of violations) console.error(`- ${violation}`);
  process.exit(1);
}

console.log("Source structure checks passed.");
