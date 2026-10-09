import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SquadPlayer } from "@/models/player";
import {
  applyArrangement,
  arrKey,
  loadArr,
  loadSession,
  saveArr,
  saveSession,
  toArrangement,
} from "./store";

const mem = new Map<string, string>();
beforeEach(() => {
  mem.clear();
  vi.stubGlobal("localStorage", {
    getItem: (k: string) => mem.get(k) ?? null,
    setItem: (k: string, v: string) => void mem.set(k, v),
  });
});

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
  it("keys an arrangement by save and club", () => {
    expect(arrKey(club)).toBe("fm:arr:abc:Aston Villa");
  });
  it("round-trips an arrangement", () => {
    const arr = { slots: [], vacant: [{ p: "ST", x: 1, y: 2 }] };
    saveArr(club, arr);
    expect(loadArr(club)).toEqual(arr);
  });
  it("round-trips the session", () => {
    saveSession({ club, arr: { slots: [], vacant: [] } });
    expect(loadSession()?.club).toEqual(club);
  });
  it("returns null when nothing is saved or the data is broken", () => {
    expect(loadArr(club)).toBeNull();
    mem.set(arrKey(club), "{oops");
    expect(loadArr(club)).toBeNull();
  });
  it("does not throw when storage is blocked", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("blocked");
      },
    });
    expect(loadSession()).toBeNull();
    expect(() => saveArr(club, { slots: [], vacant: [] })).not.toThrow();
  });
});
