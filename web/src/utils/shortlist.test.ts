import { describe, it, expect } from "vitest";
import type { PoolPlayer } from "@/models/player";
import { canPlay, stars, shortlistView } from "./shortlist";

describe("stars", () => {
  it("rounds CA and PA to half stars, 200 = 5", () => {
    expect(stars(148, 175)).toEqual({ ca: 3.5, pa: 4.5 });
    expect(stars(200, 200)).toEqual({ ca: 5, pa: 5 });
    expect(stars(0, 4)).toEqual({ ca: 0, pa: 0 });
  });
});

const pl = (id: number, o: Partial<PoolPlayer>): PoolPlayer => ({
  id,
  n: `P${id}`,
  cl: "",
  nat: "",
  p: "ST",
  a: 20,
  ca: 100,
  pa: 120,
  ...o,
});
const list = [
  pl(1, { p: "ST", v: 9.5, left: 34, tl: true }),
  pl(2, { p: "DC", v: 2, left: 4, ll: true }),
  pl(3, { p: "GK", v: undefined, left: 4, ca: 150 }),
  pl(4, { p: "DC", v: 6, left: 20 }),
];
const ids = (r: PoolPlayer[]) => r.map((p) => p.id);
const none = { pos: [], tl: false, ll: false, exp: false };

describe("shortlistView", () => {
  it("sorts by price, unknown price last either way", () => {
    expect(ids(shortlistView(list, none, { k: "v", dir: 1 }))).toEqual([
      2, 4, 1, 3,
    ]);
    expect(ids(shortlistView(list, none, { k: "v", dir: -1 }))).toEqual([
      1, 4, 2, 3,
    ]);
  });
  it("sorts by name and ability", () => {
    expect(ids(shortlistView(list, none, { k: "ca", dir: -1 }))[0]).toBe(3);
    expect(ids(shortlistView(list, none, { k: "n", dir: 1 }))).toEqual([
      1, 2, 3, 4,
    ]);
  });
  it("filters by any of the picked positions", () => {
    const r = shortlistView(
      list,
      { ...none, pos: ["DC", "GK"] },
      {
        k: "v",
        dir: 1,
      },
    );
    expect(ids(r).sort()).toEqual([2, 3, 4]);
  });
  it("status chips all have to hold", () => {
    const s = { k: "v", dir: 1 } as const;
    expect(ids(shortlistView(list, { ...none, tl: true }, s))).toEqual([1]);
    expect(ids(shortlistView(list, { ...none, ll: true }, s))).toEqual([2]);
    expect(ids(shortlistView(list, { ...none, exp: true }, s))).toEqual([2, 3]);
    expect(
      ids(shortlistView(list, { ...none, exp: true, ll: true }, s)),
    ).toEqual([2]);
  });
  it("does not change the list it is given", () => {
    shortlistView(list, none, { k: "v", dir: -1 });
    expect(ids(list)).toEqual([1, 2, 3, 4]);
  });
});

describe("canPlay", () => {
  it("uses every position he can play", () => {
    expect(canPlay(pl(1, { p: "DC", pos: ["DC", "DM"] }), "DM")).toBe(true);
    expect(canPlay(pl(1, { p: "DC", pos: ["DC"] }), "DM")).toBe(false);
  });
  it("falls back to the main position for players kept before positions were stored", () => {
    expect(canPlay(pl(1, { p: "DM" }), "DM")).toBe(true);
    expect(canPlay(pl(1, { p: "DC" }), "DM")).toBe(false);
  });
});
