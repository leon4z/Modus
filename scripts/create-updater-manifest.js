// Purpose: Create a static Tauri updater manifest for GitHub Release assets.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { mkdirSync } from "node:fs";

const repo = "leon4z/Modus";
const channelTags = {
  stable: "latest",
  test: "modus-test",
};

function fail(message) {
  console.error(`[updater-manifest] ${message}`);
  process.exit(1);
}

function argValue(name) {
  const index = process.argv.indexOf(`--${name}`);
  if (index === -1) return "";
  return String(process.argv[index + 1] || "");
}

function currentPlatformKey() {
  if (process.platform === "darwin" && process.arch === "arm64") return "darwin-aarch64";
  if (process.platform === "darwin" && process.arch === "x64") return "darwin-x86_64";
  if (process.platform === "win32" && process.arch === "x64") return "windows-x86_64";
  if (process.platform === "linux" && process.arch === "x64") return "linux-x86_64";
  fail(`unsupported updater platform ${process.platform}/${process.arch}`);
}

const channel = argValue("channel") || "test";
const version = argValue("version");
const artifactPath = argValue("artifact");
const signaturePath = argValue("signature");
const outPath = argValue("out") || "latest.json";
const platform = argValue("platform") || currentPlatformKey();

if (!channelTags[channel]) fail(`unknown channel ${channel}`);
if (!version) fail("--version is required");
if (!artifactPath) fail("--artifact is required");
if (!signaturePath) fail("--signature is required");

const artifact = resolve(artifactPath);
const signature = resolve(signaturePath);
if (!existsSync(artifact)) fail(`artifact not found: ${artifactPath}`);
if (!existsSync(signature)) fail(`signature not found: ${signaturePath}`);

const releaseTag = channelTags[channel];
const artifactName = basename(artifact);
const manifest = {
  version,
  notes: channel === "test"
    ? "Pre-release update manifest for the Modus test channel."
    : "Modus stable update manifest.",
  pub_date: new Date().toISOString(),
  platforms: {
    [platform]: {
      signature: readFileSync(signature, "utf8").trim(),
      url: channel === "stable"
        ? `https://github.com/${repo}/releases/latest/download/${artifactName}`
        : `https://github.com/${repo}/releases/download/${releaseTag}/${artifactName}`,
    },
  },
};

const output = resolve(outPath);
mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`[updater-manifest] wrote ${output}`);
