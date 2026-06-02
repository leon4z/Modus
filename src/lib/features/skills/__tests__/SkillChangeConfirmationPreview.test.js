import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";

import SkillChangeConfirmationPreview from "$lib/features/skills/components/SkillChangeConfirmationPreview.svelte";
import { tools } from "$lib/features/tools/index.js";

describe("SkillChangeConfirmationPreview", () => {
  afterEach(() => {
    cleanup();
    tools.set([]);
  });

  it("展示统一确认项的概览、风险、形态、阻塞和说明", () => {
    tools.set([{ id: "dev-tool2", name: "Dev Tool 2", detected: true, capabilities: [] }]);

    render(SkillChangeConfirmationPreview, {
      preview: {
        warnings: ["共享目录需要先解除只读状态", "demo 将覆盖已有共享目录"],
        confirmationItems: [
          {
            operation: "overwrite",
            locationDomain: "tool_directory",
            affectedObjectKind: "tool",
            affectedObject: "dev-tool2",
            realPath: "/tmp/dev-tool2/skills/demo",
            entryShape: "symlink",
            risk: ["overwrite", "content_difference"],
            action: "copy",
            skillName: "demo",
          },
        ],
        blockedItems: [
          {
            skillName: "blocked-skill",
            toolName: "Dev Tool 2",
            reason: "目标目录不可写",
          },
        ],
        message: "执行前请确认路径",
      },
    });

    expect(screen.getByText("共享目录需要先解除只读状态")).toBeInTheDocument();
    expect(screen.getByText("demo 将覆盖已有共享目录")).toBeInTheDocument();
    expect(screen.getByText("变更概览")).toBeInTheDocument();
    expect(screen.getByText("工具目录")).toBeInTheDocument();
    expect(screen.getByText("覆盖 1 项")).toBeInTheDocument();
    expect(screen.getByText("覆盖 · 工具目录")).toBeInTheDocument();
    expect(screen.getByText("Dev Tool 2")).toBeInTheDocument();
    expect(screen.getByText("/tmp/dev-tool2/skills/demo")).toBeInTheDocument();
    expect(screen.getByLabelText("软链接")).toBeInTheDocument();
    expect([...document.querySelectorAll('[role="tooltip"]')].map((node) => node.textContent)).toContain("软链接");
    expect(screen.queryByText("内容不同")).not.toBeInTheDocument();
    expect(screen.getByText("不可执行项")).toBeInTheDocument();
    expect(screen.getByText("目标目录不可写")).toBeInTheDocument();
    expect(screen.getByText("执行前请确认路径")).toBeInTheDocument();
  });

  it("兼容旧变更载荷并展示默认展开的覆盖明细", () => {
    render(SkillChangeConfirmationPreview, {
      preview: {
        changes: [
          {
            action: "install",
            changeKind: "overwrite",
            skillName: "demo",
            subject: "dev-tool3",
            path: "/tmp/dev-tool3/skills/demo",
            entryKind: "symlink",
          },
        ],
      },
    });

    expect(screen.getByText("覆盖 · 工具目录")).toBeInTheDocument();
    expect(screen.getByText("dev-tool3")).toBeInTheDocument();
    expect(screen.getByText("/tmp/dev-tool3/skills/demo")).toBeInTheDocument();
    expect(screen.getByLabelText("软链接")).toBeInTheDocument();
    expect([...document.querySelectorAll('[role="tooltip"]')].map((node) => node.textContent)).toContain("软链接");
  });

  it("删除分组内不重复显示会删除标签", () => {
    render(SkillChangeConfirmationPreview, {
      preview: {
        confirmationItems: [
          {
            operation: "delete",
            locationDomain: "shared_source",
            affectedObjectKind: "shared_source",
            affectedObject: "__shared__",
            realPath: "/tmp/shared/demo",
            entryShape: "file",
            risk: ["delete"],
            action: "delete",
            skillName: "demo",
          },
        ],
      },
    });

    expect(screen.getByText("删除 · 共享目录")).toBeInTheDocument();
    expect(screen.getByText("/tmp/shared/demo")).toBeInTheDocument();
    expect(screen.queryByText("会删除")).not.toBeInTheDocument();
  });

  it("无内容时展示空态", () => {
    render(SkillChangeConfirmationPreview, { preview: null });

    expect(screen.getByText("无文件变更")).toBeInTheDocument();
  });
});
