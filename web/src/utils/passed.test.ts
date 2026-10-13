import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "@/services/api";
import { addPassed, clearPassed, loadPassed, removePassed } from "./passed";
import { scoutQuery, swipeBody } from "./player";

let server: Record<string, unknown>;
let snapN = 0;
let snap: string;
beforeEach(() => {
  server = {};
  // a fresh snapshot id per test: passed.ts caches per snapshot
  snap = `s${++snapN}`;
  vi.spyOn(api, "get").mockImplementation(async (url: string) => ({
    data: { value: server[url] ?? null },
  }));
  vi.spyOn(api, "put").mockImplementation(async (url: string, body: any) => {
    server[url] = body.value;
    return { data: {} };
  });
});
afterEach(() => vi.restoreAllMocks());

const stored = () => server[`/api/snapshots/${snap}/planner/passed`];

describe("passed players", () => {
  it("starts empty and stores ids on the server", async () => {
    expect(await loadPassed(snap)).toEqual([]);
    await addPassed(snap, 5);
    await addPassed(snap, 7);
    expect(await loadPassed(snap)).toEqual([5, 7]);
    expect(stored()).toEqual([5, 7]);
  });
  it("reads what the server already has", async () => {
    server[`/api/snapshots/${snap}/planner/passed`] = [3];
    expect(await loadPassed(snap)).toEqual([3]);
  });
  it("does not store an id twice", async () => {
    await addPassed(snap, 5);
    await addPassed(snap, 5);
    expect(stored()).toEqual([5]);
  });
  it("removes one and clears all", async () => {
    await addPassed(snap, 5);
    await addPassed(snap, 6);
    await removePassed(snap, 5);
    expect(stored()).toEqual([6]);
    await clearPassed(snap);
    expect(await loadPassed(snap)).toEqual([]);
  });
  it("keeps quick swipes that were not awaited", async () => {
    await Promise.all([addPassed(snap, 1), addPassed(snap, 2)]);
    expect(stored()).toEqual([1, 2]);
  });
});

describe("swipeBody", () => {
  const q = scoutQuery(
    { pos: ["ST"], min: 18, max: 23, con: 0, val: "" },
    "2053-03-01",
  );
  it("pages the scouting query and excludes passed ids", () => {
    const b = swipeBody(q, [3, 4], 2);
    expect(b.page).toBe(2);
    expect(b.limit).toBe(50);
    expect(b.excludeIds).toEqual([3, 4]);
    expect(b.ageMin).toBe(18);
  });
  it("leaves excludeIds out when nothing was passed", () => {
    expect(swipeBody(q, [], 1).excludeIds).toBeUndefined();
  });
});
