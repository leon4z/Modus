import { describe, expect, it } from "vitest";

import {
  buildSkillChangeConfirmation,
  normalizeSkillStatus,
  parseOperationPreview,
  statusToInstallMode,
} from "$lib/features/skills/domain/skillDomain.js";

describe("skillDomain", () => {
  it("归一化 Skill 状态", () => {
    expect(normalizeSkillStatus("variant_installed_symlink")).toBe("variantInstalledSymlink");
    expect(normalizeSkillStatus("NotInstalled")).toBe("notInstalled");
    expect(normalizeSkillStatus("")).toBe("");
  });

  it("解析新旧字段形态的操作预览", () => {
    const result = parseOperationPreview({
      paths_to_create: ["/create"],
      pathsToDelete: ["/delete"],
      paths_to_overwrite: ["/overwrite"],
      pathsToPreserve: ["/preserve"],
      requires_force_overwrite: true,
      blocked_items: [{ path: "/blocked" }],
      operation_guide_title: "guide",
      operation_guide: ["step"],
      partial: true,
    });

    expect(result.creates).toEqual(["/create"]);
    expect(result.deletes).toEqual(["/delete"]);
    expect(result.overwrites).toEqual(["/overwrite"]);
    expect(result.preserves).toEqual(["/preserve"]);
    expect(result.requiresForceOverwrite).toBe(true);
    expect(result.blockedItems).toEqual([{ path: "/blocked" }]);
    expect(result.operationGuideTitle).toBe("guide");
    expect(result.operationGuide).toEqual(["step"]);
    expect(result.partial).toBe(true);
  });

  it("把后端确认项按本地落点分组", () => {
    const model = buildSkillChangeConfirmation({
      confirmationItems: [
        {
          operation: "create",
          locationDomain: "tool_directory",
          affectedObjectKind: "tool",
          affectedObject: "dev-tool1",
          realPath: "/tool/demo",
          entryShape: "file",
          risk: [],
          action: "copy",
          skillName: "demo",
        },
        {
          operation: "delete",
          locationDomain: "shared_source",
          affectedObjectKind: "shared_source",
          affectedObject: "__shared__",
          realPath: "/shared/demo",
          entryShape: "file",
          risk: ["delete"],
          action: "delete",
          skillName: "demo",
        },
      ],
    });

    expect(model.overview.map((item) => item.locationDomain)).toEqual([
      "tool_directory",
      "shared_source",
    ]);
    expect(model.groups.map((group) => group.key)).toEqual([
      "create::tool_directory",
      "delete::shared_source",
    ]);
    expect(model.groups[0].items[0].affectedObject).toBe("dev-tool1");
    expect(model.groups[1].defaultExpanded).toBe(true);
  });

  it("兼容旧 changes 预览并补齐覆盖风险", () => {
    const model = buildSkillChangeConfirmation({
      changes: [
        {
          action: "copy",
          changeKind: "overwrite",
          skillName: "demo",
          subject: "dev-tool2",
          path: "/tool/demo",
          entryKind: "symlink",
        },
      ],
    });

    expect(model.groups).toHaveLength(1);
    expect(model.groups[0].operation).toBe("overwrite");
    expect(model.groups[0].locationDomain).toBe("tool_directory");
    expect(model.groups[0].items[0].risk).toEqual(["overwrite", "content_difference"]);
    expect(model.groups[0].items[0].entryShape).toBe("symlink");
  });

  it("从状态推导安装方式", () => {
    expect(statusToInstallMode("variant_installed_symlink")).toBe("symlink");
    expect(statusToInstallMode("broken_symlink")).toBe("symlink");
    expect(statusToInstallMode("variant_installed_copy")).toBe("copy");
  });
});
