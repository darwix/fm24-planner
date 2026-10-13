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
  club: "Test FC",
  nationalities: ["England"],
  wage: null,
  contractEnd: null,
  value: null,
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
  it("keeps every position the player can play", () => {
    const [x] = buildSquad([pl("A", ["MC", "AMC", "AMR"], 150)]);
    expect(x.pos).toEqual(["MC", "AMC", "AMR"]);
  });
  it("keeps the player's real club, so a planned signing can be told apart", () => {
    const [x] = buildSquad([pl("A", ["ST"], 150)]);
    expect(x.cl).toBe("Test FC");
  });
  it("fills the AMC slot", () => {
    const s = buildSquad([pl("A", ["AMC"], 150)]);
    expect(s[0]).toMatchObject({ p: "AMC", xi: 1 });
  });
  it("carries contract, value and wage from the save", () => {
    const [x] = buildSquad(
      [
        {
          ...pl("A", ["ST"], 150),
          wage: 455_280,
          contractEnd: "2056-06-30",
          value: 96_391_200,
        },
      ],
      "2053-02-18",
    );
    expect(x).toMatchObject({ c: 2056, left: 40, v: 96.4, w: 455 });
  });
  it("uses a player once only", () => {
    const s = buildSquad([pl("A", ["DM", "AMC"], 150)]);
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
