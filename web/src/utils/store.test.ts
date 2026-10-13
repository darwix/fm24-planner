import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SquadPlayer } from "@/models/player";
import { api } from "@/services/api";
import {
  applyArrangement,
  arrKey,
  loadArr,
  loadSession,
  saveArr,
  saveSession,
  toArrangement,
} from "./store";

let server: Record<string, unknown>;
beforeEach(() => {
  server = {};
  vi.spyOn(api, "get").mockImplementation(async (url: string) => ({
    data: { value: server[url] ?? null },
  }));
  vi.spyOn(api, "put").mockImplementation(async (url: string, body: any) => {
    server[url] = body.value;
    return { data: {} };
  });
});
afterEach(() => vi.restoreAllMocks());

const club = { name: "Aston Villa", snapshotId: "abc", gameDate: "2053-03-01" };
const pl = (id: number, extra: Partial<SquadPlayer> = {}): SquadPlayer => ({
  id,
  n: `P${id}`,
  p: "ST",
  a: 20,
  ca: 100,
  pa: 120,
  at: {},
  ...extra,
});

describe("arrangement", () => {
  it("keeps only who is where, never the player data", () => {
    const a = toArrangement(
      [pl(1, { xi: 1, x: 5, y: 6, c: 2055, v: 9, cover: ["AMR"] })],
      [],
    );
    expect(a.slots[0]).toEqual({
      id: 1,
      p: "ST",
      xi: 1,
      x: 5,
      y: 6,
      cover: ["AMR"],
    });
    expect(JSON.stringify(a)).not.toContain("2055");
  });
  it("puts saved slots back on fresh player data", () => {
    const a = toArrangement(
      [pl(1, { p: "DL", xi: 1, x: 5, y: 6 }), pl(2, { cover: ["ST"] })],
      [],
    );
    const fresh = new Map([
      [1, pl(1, { c: 2060, ca: 150 })],
      [2, pl(2, { c: 2061 })],
    ]);
    const { squad } = applyArrangement(a, fresh);
    expect(squad[0]).toMatchObject({ p: "DL", xi: 1, x: 5, ca: 150, c: 2060 });
    expect(squad[1]).toMatchObject({ cover: ["ST"], c: 2061 });
  });
  it("drops players the save no longer has", () => {
    const a = toArrangement([pl(1), pl(2)], []);
    expect(applyArrangement(a, new Map([[2, pl(2)]])).squad).toHaveLength(1);
  });
});

describe("store", () => {
  it("keys an arrangement by club inside the snapshot", () => {
    expect(arrKey(club)).toBe("squad:Aston Villa");
  });
  it("round-trips an arrangement through the snapshot's planner", async () => {
    const arr = { slots: [], vacant: [{ p: "ST", x: 1, y: 2 }] };
    await saveArr(club, arr);
    expect(server["/api/snapshots/abc/planner/squad%3AAston%20Villa"]).toEqual(
      arr,
    );
    expect(await loadArr(club)).toEqual(arr);
  });
  it("round-trips the session", async () => {
    await saveSession({ club });
    expect((await loadSession())?.club).toEqual(club);
  });
  it("returns null when nothing is saved", async () => {
    expect(await loadArr(club)).toBeNull();
    expect(await loadSession()).toBeNull();
  });
});
