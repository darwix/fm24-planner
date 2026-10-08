import { describe, it, expect } from "vitest";
import { buildSquad, SLOTS } from "./squad";

let n = 0;
const pl = (name: string, positions: string[], ca: number) => ({
  id: ++n,
  name,
  positions,
  ca,
  pa: ca + 5,
  age: 25,
});

describe("buildSquad", () => {
  it("puts the best player for each slot on the pitch", () => {
    const s = buildSquad([
      pl("A", ["GK"], 120),
      pl("B", ["GK"], 140),
      pl("C", ["ST"], 130),
    ]);
    expect(s.filter((x) => x.xi).map((x) => x.n)).toEqual(["B", "C"]);
  });
  it("uses a player once only", () => {
    const s = buildSquad([pl("A", ["DM", "AM"], 150)]);
    expect(s.filter((x) => x.xi)).toHaveLength(1);
  });
  it("maps main position and keeps everyone, subs off the pitch", () => {
    const s = buildSquad([
      pl("A", ["DC"], 100),
      pl("B", ["DC"], 110),
      pl("C", ["DC"], 120),
      pl("D", ["DC"], 130),
      pl("E", ["DC"], 90),
    ]);
    expect(s).toHaveLength(5);
    expect(s.every((x) => x.p === "DC")).toBe(true);
    expect(s.filter((x) => x.xi)).toHaveLength(2);
    expect(SLOTS).toHaveLength(11);
  });
});
