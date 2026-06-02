// Purpose: Verify that release updater configuration keeps stable and test channels separated.
import { readFileSync } from "node:fs";
import { join } from "node:path";

const repoRoot = new URL("..", import.meta.url).pathname;
const stableEndpoint = "https://github.com/leon4z/Modus/releases/latest/download/latest.json";
const testEndpoint = "https://github.com/leon4z/Modus/releases/download/modus-test/latest.json";

function fail(message) {
  console.error(`[updater-config] ${message}`);
  process.exit(1);
}

function readText(path) {
  return readFileSync(join(repoRoot, path), "utf8");
}

const tauriConfig = JSON.parse(readText("src-tauri/tauri.conf.json"));
const preReleaseConfig = JSON.parse(readText("src-tauri/tauri.pre-release.conf.json"));
const packageManifest = JSON.parse(readText("package.json"));
const updaterConfig = tauriConfig.plugins?.updater || {};
const configuredEndpoints = updaterConfig.endpoints || [];
const preReleaseUpdaterConfig = preReleaseConfig.plugins?.updater || {};
const preReleaseEndpoints = preReleaseUpdaterConfig.endpoints || [];

if (tauriConfig.bundle?.createUpdaterArtifacts !== true) {
  fail("bundle.createUpdaterArtifacts must be true");
}

if (typeof updaterConfig.pubkey !== "string" || updaterConfig.pubkey.trim().length < 32) {
  fail("plugins.updater.pubkey must contain the public update signing key");
}

if (updaterConfig.pubkey.includes("/Users/") || updaterConfig.pubkey.includes("PRIVATE")) {
  fail("plugins.updater.pubkey must not contain local paths or private key material");
}

if (!configuredEndpoints.includes(stableEndpoint)) {
  fail(`stable updater endpoint must include ${stableEndpoint}`);
}

if (!preReleaseEndpoints.includes(testEndpoint)) {
  fail(`pre-release updater endpoint must include ${testEndpoint}`);
}

if (preReleaseEndpoints.includes(stableEndpoint)) {
  fail("pre-release updater endpoint must not include the stable manifest");
}

if (preReleaseUpdaterConfig.pubkey !== updaterConfig.pubkey) {
  fail("pre-release updater public key must match the stable updater public key");
}

const domainSource = readText("src-tauri/src/domains/app_updates/mod.rs");

if (!domainSource.includes(`const STABLE_UPDATE_ENDPOINT: &str =\n    "${stableEndpoint}";`)) {
  fail("stable update source constant is missing or changed");
}

if (!domainSource.includes(`const TEST_UPDATE_ENDPOINT: &str =\n    "${testEndpoint}";`)) {
  fail("test update source constant is missing or changed");
}

if (stableEndpoint === testEndpoint) {
  fail("stable and test updater endpoints must be different");
}

const preReleaseBuildScript = packageManifest.scripts?.["tauri:build:pre-release"] || "";
if (!preReleaseBuildScript.includes("MODUS_RUNTIME=pre-release")) {
  fail("tauri:build:pre-release must pin MODUS_RUNTIME=pre-release at build time");
}

if (!preReleaseBuildScript.includes("src-tauri/tauri.pre-release.conf.json")) {
  fail("tauri:build:pre-release must use the pre-release Tauri config");
}

if (!packageManifest.scripts?.["updater:manifest"]?.includes("create-updater-manifest.js")) {
  fail("package.json must expose the updater manifest generator");
}

console.log("[updater-config] ok: stable and test updater channels are separated");
