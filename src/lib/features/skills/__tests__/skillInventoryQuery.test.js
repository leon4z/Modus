import { beforeEach, describe, expect, it, vi } from "vitest";

const apiMocks = vi.hoisted(() => ({
  scanSkillInventory: vi.fn(),
}));

vi.mock("$lib/features/skills/api/skills.js", () => apiMocks);

async function loadModule() {
  vi.resetModules();
  return import("$lib/features/skills/queries/skillInventoryQuery.js");
}

function createDeferred() {
  /** @type {(value: any) => void} */
  let resolve = () => {};
  /** @type {(reason?: any) => void} */
  let reject = () => {};
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("skillInventoryQuery", () => {
  beforeEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("共享并发库存请求", async () => {
    const { getSkillInventory } = await loadModule();
    const result = { skills: [{ name: "demo" }] };
    apiMocks.scanSkillInventory.mockResolvedValue(result);

    const [first, second] = await Promise.all([
      getSkillInventory(),
      getSkillInventory(),
    ]);

    expect(first).toBe(result);
    expect(second).toBe(result);
    expect(apiMocks.scanSkillInventory).toHaveBeenCalledTimes(1);
  });

  it("在 TTL 内复用缓存，失效后重新扫描", async () => {
    vi.useFakeTimers();
    const { getSkillInventory, invalidateSkillInventory } = await loadModule();
    apiMocks.scanSkillInventory
      .mockResolvedValueOnce({ skills: [{ name: "first" }] })
      .mockResolvedValueOnce({ skills: [{ name: "second" }] });

    await expect(getSkillInventory({ ttl: 1500 })).resolves.toEqual({ skills: [{ name: "first" }] });
    await expect(getSkillInventory({ ttl: 1500 })).resolves.toEqual({ skills: [{ name: "first" }] });
    expect(apiMocks.scanSkillInventory).toHaveBeenCalledTimes(1);

    invalidateSkillInventory();
    await expect(getSkillInventory({ ttl: 1500 })).resolves.toEqual({ skills: [{ name: "second" }] });
    expect(apiMocks.scanSkillInventory).toHaveBeenCalledTimes(2);
  });

  it("强制刷新绕过缓存", async () => {
    const { getSkillInventory } = await loadModule();
    apiMocks.scanSkillInventory
      .mockResolvedValueOnce({ skills: [{ name: "cached" }] })
      .mockResolvedValueOnce({ skills: [{ name: "fresh" }] });

    await expect(getSkillInventory()).resolves.toEqual({ skills: [{ name: "cached" }] });
    await expect(getSkillInventory({ force: true })).resolves.toEqual({ skills: [{ name: "fresh" }] });
    expect(apiMocks.scanSkillInventory).toHaveBeenCalledTimes(2);
  });

  it("TTL 失效后仍可读取上一份快照，显式失效会清空快照", async () => {
    vi.useFakeTimers();
    const { getCachedSkillInventory, getSkillInventory, invalidateSkillInventory } = await loadModule();
    const result = { skills: [{ name: "cached" }] };
    apiMocks.scanSkillInventory.mockResolvedValue(result);

    await expect(getSkillInventory({ ttl: 1500 })).resolves.toBe(result);
    vi.advanceTimersByTime(2000);

    expect(getCachedSkillInventory({ ttl: 1500 })).toBeNull();
    expect(getCachedSkillInventory()).toBe(result);

    invalidateSkillInventory();
    expect(getCachedSkillInventory()).toBeNull();
  });

  it("预热会写入库存缓存供后续页面复用", async () => {
    const { getSkillInventory, prewarmSkillInventory } = await loadModule();
    const result = { skills: [{ name: "prewarmed" }] };
    apiMocks.scanSkillInventory.mockResolvedValue(result);

    await expect(prewarmSkillInventory()).resolves.toBe(result);
    await expect(getSkillInventory()).resolves.toBe(result);
    expect(apiMocks.scanSkillInventory).toHaveBeenCalledTimes(1);
  });

  it("预热失败不影响调用方", async () => {
    const { prewarmSkillInventory } = await loadModule();
    apiMocks.scanSkillInventory.mockRejectedValueOnce(new Error("scan failed"));

    await expect(prewarmSkillInventory()).resolves.toBeNull();
    expect(apiMocks.scanSkillInventory).toHaveBeenCalledTimes(1);
  });

  it("失效后忽略旧的并发请求缓存回写", async () => {
    const { getSkillInventory, invalidateSkillInventory } = await loadModule();
    const stale = createDeferred();
    const fresh = createDeferred();
    apiMocks.scanSkillInventory
      .mockReturnValueOnce(stale.promise)
      .mockReturnValueOnce(fresh.promise)
      .mockResolvedValueOnce({ skills: [{ name: "next" }] });

    const staleRequest = getSkillInventory();
    invalidateSkillInventory();
    const freshRequest = getSkillInventory();

    fresh.resolve({ skills: [{ name: "fresh" }] });
    await expect(freshRequest).resolves.toEqual({ skills: [{ name: "fresh" }] });

    stale.resolve({ skills: [{ name: "stale" }] });
    await expect(staleRequest).resolves.toEqual({ skills: [{ name: "stale" }] });

    await expect(getSkillInventory()).resolves.toEqual({ skills: [{ name: "fresh" }] });
    expect(apiMocks.scanSkillInventory).toHaveBeenCalledTimes(2);
  });
});
