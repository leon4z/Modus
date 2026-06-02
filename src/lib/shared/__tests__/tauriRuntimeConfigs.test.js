// @ts-nocheck
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");

function readJson(path) {
  return JSON.parse(readFileSync(resolve(repoRoot, path), "utf8"));
}

describe("Tauri runtime flavor configs", () => {
  it("keeps visible product name stable while separating native identifiers", () => {
    const releaseConfig = readJson("src-tauri/tauri.conf.json");
    const devConfig = readJson("src-tauri/tauri.dev.conf.json");
    const preReleaseConfig = readJson("src-tauri/tauri.pre-release.conf.json");

    expect(releaseConfig.productName).toBe("Modus");
    expect(devConfig.productName).toBe("Modus");
    expect(preReleaseConfig.productName).toBe("Modus");

    expect(releaseConfig.identifier).toBe("com.leon4z.modus");
    expect(devConfig.identifier).toBe("com.leon4z.modus.dev");
    expect(preReleaseConfig.identifier).toBe("com.leon4z.modus.pre-release");
    expect(new Set([
      releaseConfig.identifier,
      devConfig.identifier,
      preReleaseConfig.identifier,
    ]).size).toBe(3);
  });

  it("launch scripts bind runtime state to the matching native identifier", () => {
    const packageJson = readJson("package.json");

    expect(packageJson.scripts["tauri:dev"]).toContain("MODUS_RUNTIME=development-sandbox");
    expect(packageJson.scripts["tauri:dev"]).toContain("--config src-tauri/tauri.dev.conf.json");
    expect(packageJson.scripts["tauri:pre-release"]).toContain("MODUS_RUNTIME=pre-release");
    expect(packageJson.scripts["tauri:pre-release"]).toContain("--config src-tauri/tauri.pre-release.conf.json");
  });
});
