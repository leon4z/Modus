import { cleanup, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";

import ToolIcon from "$lib/shared/components/ToolIcon.svelte";
import toolIconSource from "$lib/shared/components/ToolIcon.svelte?raw";

describe("ToolIcon", () => {
  afterEach(() => {
    document.documentElement.removeAttribute("data-theme");
    cleanup();
  });

  /**
   * @param {HTMLImageElement} image
   */
  function decodedSrc(image) {
    return decodeURIComponent(image.getAttribute("src") || "");
  }

  /**
   * @param {HTMLImageElement} lightImage
   * @param {HTMLImageElement} darkImage
   * @param {string} lightName
   * @param {string} darkName
   * @param {string} title
   * @param {string} [fixedColor]
   */
  function expectLightDarkAssets(lightImage, darkImage, lightName, darkName, title, fixedColor = undefined) {
    const lightSrc = decodedSrc(lightImage);
    const darkSrc = decodedSrc(darkImage);
    if (lightSrc.startsWith("data:image/svg+xml")) {
      expect(lightSrc).toContain(`<title>${title}</title>`);
      expect(darkSrc).toContain(`<title>${title}</title>`);
      expect(lightSrc).toContain("fill='#000'");
      expect(darkSrc).toContain("fill='#fff'");
      if (fixedColor) {
        expect(lightSrc).toContain(fixedColor);
        expect(darkSrc).toContain(fixedColor);
      }
    } else {
      expect(lightSrc).toContain(lightName);
      expect(darkSrc).toContain(darkName);
    }
  }

  /**
   * @param {HTMLImageElement} lightImage
   * @param {HTMLImageElement} darkImage
   */
  function expectThemeSwitchContract(lightImage, darkImage) {
    expect(lightImage).toHaveClass("brand-icon--light");
    expect(darkImage).toHaveClass("brand-icon--dark");
    expect(toolIconSource).toContain('.brand-icon--dark');
    expect(toolIconSource).toContain(':global([data-theme="dark"]) .brand-icon--light');
    expect(toolIconSource).toContain(':global([data-theme="dark"]) .brand-icon--dark');
  }

  it("renders Cursor as a theme-paired mono brand asset", () => {
    const { container } = render(ToolIcon, {
      props: {
        toolId: "cursor",
        size: 16,
      },
    });

    const icons = /** @type {NodeListOf<HTMLImageElement>} */ (
      container.querySelectorAll('img[alt="Cursor Logo"]')
    );
    expect(icons).toHaveLength(2);
    expect(icons[0]).toHaveClass("brand-icon--light");
    expect(icons[1]).toHaveClass("brand-icon--dark");
    expectLightDarkAssets(icons[0], icons[1], "cursor-light", "cursor-dark", "Cursor");
    expectThemeSwitchContract(icons[0], icons[1]);
    expect(icons[0]).toHaveStyle({ width: "16px", height: "16px" });
    expect(container.querySelector(".unknown-tool-icon")).not.toBeInTheDocument();
    expect(container.querySelector(".synthetic-box")).not.toBeInTheDocument();
    expect(container.querySelector("svg")).not.toBeInTheDocument();
  });

  it.each([
    ["codex", "Codex Logo"],
    ["claude-code", "Claude Code Logo"],
    ["trae", "Trae Logo"],
    ["trae-cn", "Trae Logo"],
    ["trae-solo", "Trae Solo Logo"],
    ["trae-solo-cn", "Trae Solo CN Logo"],
    ["codebuddy", "CodeBuddy Logo"],
    ["workbuddy", "WorkBuddy Logo"],
    ["kiro", "Kiro Logo"],
  ])("renders %s with its original brand asset and no decorative wrapper", (toolId, alt) => {
    const { container } = render(ToolIcon, {
      props: {
        toolId,
        size: 18,
        shadow: true,
      },
    });

    const icon = container.querySelector(`img[alt="${alt}"]`);
    expect(icon).toBeInTheDocument();
    expect(icon).toHaveClass("brand-icon");
    expect(icon).toHaveClass("brand-icon--single");
    expect(icon).toHaveStyle({ width: "18px", height: "18px" });
    expect(container.querySelector(".unknown-tool-icon")).not.toBeInTheDocument();
    expect(container.querySelector(".synthetic-box")).not.toBeInTheDocument();
  });

  it.each([
    ["opencode", "OpenCode Logo", "opencode-light", "opencode-dark", "opencode", undefined],
    ["hermes-agent", "Hermes Agent Logo", "hermes-agent-light", "hermes-agent-dark", "Hermes Agent", undefined],
    ["github-copilot", "GitHub Copilot Logo", "github-copilot-light", "github-copilot-dark", "GithubCopilot", undefined],
    ["qoder", "Qoder Logo", "qoder-color-light", "qoder-color-dark", "Qoder", "#2ADB5C"],
    ["windsurf", "Windsurf Logo", "windsurf-light", "windsurf-dark", "Windsurf", undefined],
  ])("renders %s with light and dark brand assets", (toolId, alt, lightAsset, darkAsset, title, fixedColor) => {
    const { container } = render(ToolIcon, {
      props: {
        toolId,
        size: 18,
        shadow: true,
      },
    });

    const icons = /** @type {NodeListOf<HTMLImageElement>} */ (
      container.querySelectorAll(`img[alt="${alt}"]`)
    );
    expect(icons).toHaveLength(2);
    expect(icons[0]).toHaveClass("brand-icon", "brand-icon--light");
    expect(icons[1]).toHaveClass("brand-icon", "brand-icon--dark");
    expectLightDarkAssets(icons[0], icons[1], lightAsset, darkAsset, title, fixedColor);
    expectThemeSwitchContract(icons[0], icons[1]);
    expect(container.querySelector(".unknown-tool-icon")).not.toBeInTheDocument();
    expect(container.querySelector(".synthetic-box")).not.toBeInTheDocument();
  });

  it("uses the neutral unknown-tool fallback for unrecognized tools", () => {
    const { container } = render(ToolIcon, {
      props: {
        toolId: "local-experimental-tool",
        size: 16,
      },
    });

    const icon = container.querySelector(".unknown-tool-icon");
    expect(icon).toBeInTheDocument();
    expect(icon).toHaveStyle({ width: "16px", height: "16px" });
  });
});
