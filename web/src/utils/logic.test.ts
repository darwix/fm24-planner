import { describe, it, expect } from "vitest";
import { TARGET, need, asCandidate, contractLabel } from "./logic";

describe("need with extra cover groups", () => {
  it("counts a player in every group he covers", () => {
    const squad = [{ p: "AMR", cover: ["ST"] }, { p: "ST" }];
    expect(need(squad, "ST")).toBe(TARGET.ST - 2);
    expect(need(squad, "AMR")).toBe(TARGET.AMR - 1);
  });
});

describe("need", () => {
  it("counts missing cover against the position target", () => {
    const squad = [{ p: "GK" }, { p: "DC" }, { p: "DC" }];
    expect(need(squad, "GK")).toBe(TARGET.GK - 1);
    expect(need(squad, "DC")).toBe(TARGET.DC - 2);
  });
  it("never goes negative", () => {
    expect(need([{ p: "GK" }, { p: "GK" }, { p: "GK" }], "GK")).toBe(0);
  });
});

describe("unknown contract data", () => {
  it("is never cheap or expiring", () => {
    expect(asCandidate({ ca: 100, pa: 100 }).tags).toEqual([]);
  });
});

describe("asCandidate", () => {
  it("tags expiring (<=12m) and cheap (<=5M) players", () => {
    const c = asCandidate({ ca: 100, pa: 100, left: 12, v: 5 });
    expect(c.tags).toEqual(["exp", "cheap"]);
    expect(asCandidate({ ca: 100, pa: 100, left: 13, v: 6 }).tags).toEqual([]);
  });
  it("scores higher CA above the target, lower CA below it", () => {
    expect(asCandidate({ ca: 150, pa: 160 }, 150).m).toBe(50);
    expect(asCandidate({ ca: 160, pa: 160 }, 150).m).toBe(70);
    expect(asCandidate({ ca: 140, pa: 160 }, 150).m).toBe(30);
    expect(asCandidate({ ca: 20, pa: 160 }, 150).m).toBe(1);
  });
  it("keeps the score at 99 for a big upgrade", () => {
    expect(asCandidate({ ca: 190, pa: 190 }, 150).m).toBe(99);
  });
  it("caps the match score at 99", () => {
    expect(asCandidate({ ca: 200, pa: 200, left: 1, v: 1 }).m).toBe(99);
  });
});

describe("contractLabel", () => {
  it("shows months when within a year, else the end year", () => {
    expect(contractLabel(7, 2053)).toBe("7 months left");
    expect(contractLabel(31, 2055)).toBe("to 2055");
  });
});
