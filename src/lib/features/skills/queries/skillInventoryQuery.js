// Purpose: Share Skill inventory reads across frontend views without changing backend scan semantics.

import { scanSkillInventory } from "$lib/features/skills/api/skills.js";

/** @type {Promise<any> | null} */
let inventoryInflight = null;
/** @type {any | null} */
let inventoryCache = null;
let inventoryCacheAt = 0;
let inventoryVersion = 0;

export const DEFAULT_SKILL_INVENTORY_TTL_MS = 60_000;

/**
 * @param {{ ttl?: number, force?: boolean }} [options]
 */
export async function getSkillInventory(options = {}) {
  const { ttl = DEFAULT_SKILL_INVENTORY_TTL_MS, force = false } = options;
  const now = Date.now();

  if (force) {
    inventoryCache = null;
    inventoryCacheAt = 0;
    inventoryVersion += 1;
  }

  if (!force && inventoryCache && now - inventoryCacheAt < ttl) {
    return inventoryCache;
  }

  if (!force && inventoryInflight) {
    return inventoryInflight;
  }

  const requestVersion = inventoryVersion;
  const request = scanSkillInventory()
    .then((result) => {
      if (requestVersion === inventoryVersion) {
        inventoryCache = result;
        inventoryCacheAt = Date.now();
      }
      return result;
    })
    .finally(() => {
      if (inventoryInflight === request) {
        inventoryInflight = null;
      }
    });

  inventoryInflight = request;
  return inventoryInflight;
}

/**
 * @param {{ ttl?: number }} [options]
 */
export function getCachedSkillInventory(options = {}) {
  const { ttl = Number.POSITIVE_INFINITY } = options;
  const now = Date.now();
  if (inventoryCache && now - inventoryCacheAt < ttl) return inventoryCache;
  return null;
}

/**
 * @param {{ ttl?: number }} [options]
 */
export async function prewarmSkillInventory(options = {}) {
  try {
    return await getSkillInventory({ ttl: options.ttl ?? DEFAULT_SKILL_INVENTORY_TTL_MS });
  } catch {
    return null;
  }
}

export function invalidateSkillInventory() {
  inventoryCache = null;
  inventoryCacheAt = 0;
  inventoryVersion += 1;
  inventoryInflight = null;
}
