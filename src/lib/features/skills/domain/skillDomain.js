// Purpose: Shared frontend interpretation rules for Community Skill file operations.

/**
 * @param {string | null | undefined} status
 */
export function normalizeSkillStatus(status) {
  const raw = String(status || "").trim();
  if (!raw) return "";
  if (raw.includes("_")) {
    const parts = raw.toLowerCase().split("_");
    return parts[0] + parts.slice(1).map((part) => part[0]?.toUpperCase() + part.slice(1)).join("");
  }
  return raw[0].toLowerCase() + raw.slice(1);
}

/**
 * @param {any} preview
 */
export function parseOperationPreview(preview) {
  return {
    creates: preview?.creates || preview?.pathsToCreate || preview?.paths_to_create || [],
    deletes: preview?.deletes || preview?.pathsToDelete || preview?.paths_to_delete || [],
    overwrites: preview?.overwrites || preview?.pathsToOverwrite || preview?.paths_to_overwrite || [],
    preserves: preview?.preserves || preview?.pathsToPreserve || preview?.paths_to_preserve || [],
    requiresForceOverwrite: Boolean(preview?.requiresForceOverwrite ?? preview?.requires_force_overwrite),
    message: preview?.message || null,
    changes: preview?.changes || [],
    blocked: preview?.blocked || [],
    blockedItems: preview?.blockedItems || preview?.blocked_items || [],
    confirmationItems: preview?.confirmationItems || preview?.confirmation_items || [],
    warnings: preview?.warnings || [],
    blockers: preview?.blockers || [],
    partial: Boolean(preview?.partial),
    operationGuideTitle: preview?.operationGuideTitle || preview?.operation_guide_title || "",
    operationGuide: preview?.operationGuide || preview?.operation_guide || [],
  };
}

const confirmationOperations = new Set(["create", "overwrite", "delete", "switch", "preserve", "blocked"]);
const destructiveRisks = new Set(["overwrite", "delete", "irreversible", "blocked", "content_difference"]);

/**
 * @param {string | undefined | null} value
 * @returns {"create"|"overwrite"|"delete"|"switch"|"preserve"|"blocked"}
 */
function normalizeConfirmationOperation(value) {
  const raw = String(value || "").trim();
  if (confirmationOperations.has(raw)) return /** @type {any} */ (raw);
  if (raw === "delete_skill" || raw === "delete_from_tool") return "delete";
  if (raw === "link_shared") return "create";
  return "preserve";
}

/** @param {string | undefined | null} subject */
function normalizeSubject(subject) {
  const value = String(subject || "");
  if (!value || value === "generic") return "__shared__";
  return value;
}

/**
 * @param {{ subject?: string, path?: string }} input
 */
function inferLocationDomain(input = {}) {
  const subject = normalizeSubject(input.subject);
  const path = String(input.path || "");
  if (path.includes("/.agents") || path.includes("/shared/")) return "shared_source";
  if (subject === "__shared__") return "shared_source";
  return "tool_directory";
}

/**
 * @param {string} subject
 * @param {string} locationDomain
 */
function affectedObjectKind(subject, locationDomain) {
  if (locationDomain === "shared_source") return "shared_source";
  if (subject === "__shared__") return "shared_source";
  return "tool";
}

/** @param {string} operation */
function risksForOperation(operation) {
  if (operation === "overwrite") return ["overwrite", "content_difference"];
  if (operation === "delete") return ["delete"];
  if (operation === "blocked") return ["blocked"];
  return [];
}

/**
 * @param {any} raw
 * @returns {any | null}
 */
function normalizeBackendConfirmationItem(raw) {
  const operation = normalizeConfirmationOperation(raw?.operation);
  const subject = normalizeSubject(raw?.affectedObject || raw?.affected_object);
  const locationDomain = raw?.locationDomain || raw?.location_domain || inferLocationDomain({
    subject,
    path: raw?.realPath || raw?.real_path,
  });
  const risk = Array.isArray(raw?.risk) ? raw.risk : [];
  return {
    operation,
    locationDomain,
    affectedObjectKind: raw?.affectedObjectKind || raw?.affected_object_kind || affectedObjectKind(subject, locationDomain),
    affectedObject: subject,
    realPath: String(raw?.realPath || raw?.real_path || ""),
    entryShape: raw?.entryShape || raw?.entry_shape || "",
    risk: risk.length > 0 ? risk : risksForOperation(operation),
    action: raw?.action || "",
    skillName: raw?.skillName || raw?.skill_name || "",
    sourceContext: raw?.sourceContext || raw?.source_context || "",
    targetContext: raw?.targetContext || raw?.target_context || "",
  };
}

/** @param {any} change */
function confirmationItemFromStructuredChange(change) {
  const operation = normalizeConfirmationOperation(change?.changeKind || change?.change_kind || change?.action);
  const subject = normalizeSubject(change?.subject);
  const locationDomain = inferLocationDomain({
    subject,
    path: change?.path,
  });
  return {
    operation,
    locationDomain,
    affectedObjectKind: affectedObjectKind(subject, locationDomain),
    affectedObject: subject,
    realPath: String(change?.path || ""),
    entryShape: change?.entryKind || change?.entry_kind || "",
    risk: risksForOperation(operation),
    action: change?.action || "",
    skillName: change?.skillName || change?.skill_name || "",
    sourceContext: "",
    targetContext: "",
  };
}

/** @param {any} change */
function confirmationItemFromPathChange(change) {
  const operation = normalizeConfirmationOperation(change?.kind);
  const subject = normalizeSubject(change?.toolId || change?.tool_id);
  const locationDomain = inferLocationDomain({
    subject,
    path: change?.absPath || change?.abs_path,
  });
  return {
    operation,
    locationDomain,
    affectedObjectKind: affectedObjectKind(subject, locationDomain),
    affectedObject: subject,
    realPath: String(change?.absPath || change?.abs_path || ""),
    entryShape: change?.entryKind || change?.entry_kind || "",
    risk: risksForOperation(operation),
    action: "",
    skillName: change?.skillName || change?.skill_name || "",
    sourceContext: "",
    targetContext: "",
  };
}

/**
 * @param {"create"|"overwrite"|"delete"|"preserve"} operation
 * @param {string} raw
 */
function confirmationItemFromLegacyPath(operation, raw) {
  const text = String(raw || "");
  const match = text.match(/^(.*?) @ (.*?): (.+)$/) || text.match(/^(.*?): (.+)$/);
  const skillName = match ? String(match[1] || "") : "";
  const subject = normalizeSubject(match && match.length > 3 ? match[2] : "__shared__");
  const path = match ? String(match[match.length - 1] || "") : text;
  const locationDomain = inferLocationDomain({ subject, path });
  return {
    operation,
    locationDomain,
    affectedObjectKind: affectedObjectKind(subject, locationDomain),
    affectedObject: subject,
    realPath: path,
    entryShape: "",
    risk: risksForOperation(operation),
    action: "",
    skillName,
    sourceContext: "",
    targetContext: "",
  };
}

/**
 * @param {"create"|"overwrite"|"delete"|"preserve"} operation
 * @param {unknown[]} paths
 */
function confirmationItemsFromLegacyPaths(operation, paths) {
  return paths.map((path) => confirmationItemFromLegacyPath(operation, String(path)));
}

/** @param {any} item */
function isUsefulConfirmationItem(item) {
  return item && typeof item.realPath === "string" && item.realPath.length > 0;
}

/**
 * Builds the grouped confirmation view model used by Community Skill
 * file-changing confirmation surfaces.
 *
 * @param {any} preview
 */
export function buildSkillChangeConfirmation(preview) {
  const normalized = parseOperationPreview(preview);
  /** @type {any[]} */
  let items = [];

  if (Array.isArray(normalized.confirmationItems) && normalized.confirmationItems.length > 0) {
    items = normalized.confirmationItems.map(normalizeBackendConfirmationItem);
  } else if (Array.isArray(normalized.changes) && normalized.changes.length > 0) {
    items = normalized.changes.map((change) => {
      if ("changeKind" in change || "change_kind" in change || "path" in change) {
        return confirmationItemFromStructuredChange(change);
      }
      return confirmationItemFromPathChange(change);
    });
  } else {
    items = [
      ...confirmationItemsFromLegacyPaths("create", normalized.creates),
      ...confirmationItemsFromLegacyPaths("overwrite", normalized.overwrites),
      ...confirmationItemsFromLegacyPaths("delete", normalized.deletes),
      ...confirmationItemsFromLegacyPaths("preserve", normalized.preserves),
    ];
  }

  const dedupedItems = new Map();
  for (const item of items.filter(isUsefulConfirmationItem)) {
    const key = [
      item.operation,
      item.locationDomain,
      item.affectedObject,
      item.realPath,
      item.entryShape,
      (item.risk || []).join(","),
    ].join("\u0000");
    if (!dedupedItems.has(key)) dedupedItems.set(key, item);
  }
  items = Array.from(dedupedItems.values());

  const overviewMap = new Map();
  const groupMap = new Map();
  for (const item of items) {
    const overviewKey = item.locationDomain;
    if (!overviewMap.has(overviewKey)) {
      overviewMap.set(overviewKey, { locationDomain: item.locationDomain, counts: {} });
    }
    const overview = overviewMap.get(overviewKey);
    overview.counts[item.operation] = (overview.counts[item.operation] || 0) + 1;

    const groupKey = `${item.operation}::${item.locationDomain}`;
    if (!groupMap.has(groupKey)) {
      groupMap.set(groupKey, {
        key: groupKey,
        operation: item.operation,
        locationDomain: item.locationDomain,
        items: [],
        defaultExpanded: item.operation === "create" || item.operation === "overwrite" || item.operation === "delete" || item.operation === "blocked",
      });
    }
    const group = groupMap.get(groupKey);
    group.items.push(item);
    const risks = /** @type {string[]} */ (Array.isArray(item.risk) ? item.risk : []);
    if (risks.some((risk) => destructiveRisks.has(risk))) {
      group.defaultExpanded = true;
    }
    if (item.entryShape && item.entryShape !== "file" && item.entryShape !== "directory") {
      group.defaultExpanded = true;
    }
  }

  const blockers = [
    ...(Array.isArray(normalized.blocked) ? normalized.blocked.map((item) => ({
      skillName: item?.skillName || item?.skill_name || "",
      affectedObject: normalizeSubject(item?.subject),
      reason: item?.reason?.message || "",
    })) : []),
    ...(Array.isArray(normalized.blockers) ? normalized.blockers.map((item) => ({
      skillName: item?.skillName || item?.skill_name || "",
      affectedObject: "__shared__",
      reason: item?.message || "",
    })) : []),
    ...(Array.isArray(normalized.blockedItems) ? normalized.blockedItems.map((item) => ({
      skillName: item?.skillName || item?.skill_name || "",
      affectedObject: item?.toolName || item?.tool_name || "",
      reason: item?.reason || "",
    })) : []),
  ].filter((item) => item.reason || item.skillName);

  return {
    items,
    overview: Array.from(overviewMap.values()),
    groups: Array.from(groupMap.values()),
    warnings: Array.isArray(normalized.warnings) ? normalized.warnings : [],
    blockers,
    message: normalized.message,
    hasContent: items.length > 0 || blockers.length > 0 || normalized.message,
  };
}

/**
 * @param {string | null | undefined} status
 */
export function statusToInstallMode(status) {
  const symlinkStatuses = new Set(["variantInstalledSymlink", "brokenSymlink"]);
  return symlinkStatuses.has(normalizeSkillStatus(status)) ? "symlink" : "copy";
}
