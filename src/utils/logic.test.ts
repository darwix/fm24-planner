import { describe, it, expect } from "vitest";
import {
  TARGET,
  need,
  matchFilter,
  asCandidate,
  contractLabel,
  isFmFile,
} from "./logic";

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

describe("matchFilter", () => {
  const p = { p: "ST", a: 20, left: 6, v: 3 };
  const f = { pos: "", min: 18, max: 23, exp: false, val: "" as const };
  it("matches any position inside the age range", () => {
    expect(matchFilter(p, f)).toBe(true);
    expect(matchFilter({ ...p, a: 24 }, f)).toBe(false);
  });
  it("filters by position, expiry and max value", () => {
    expect(matchFilter(p, { ...f, pos: "GK" })).toBe(false);
    expect(matchFilter({ ...p, left: 13 }, { ...f, exp: true })).toBe(false);
    expect(matchFilter(p, { ...f, exp: true })).toBe(true);
    expect(matchFilter(p, { ...f, val: 2 })).toBe(false);
  });
});

describe("unknown contract data", () => {
  const f = { pos: "", min: 16, max: 40, exp: false, val: "" as const };
  it("is never expiring and never cheap", () => {
    expect(matchFilter({ p: "ST", a: 20 }, { ...f, exp: true })).toBe(false);
    expect(matchFilter({ p: "ST", a: 20 }, { ...f, val: 5 })).toBe(false);
    expect(matchFilter({ p: "ST", a: 20 }, f)).toBe(true);
    expect(asCandidate({ ca: 100, pa: 100 }).tags).toEqual([]);
  });
});

describe("asCandidate", () => {
  it("tags expiring (<=12m) and cheap (<=5M) players", () => {
    const c = asCandidate({ ca: 100, pa: 100, left: 12, v: 5 });
    expect(c.tags).toEqual(["exp", "cheap"]);
    expect(asCandidate({ ca: 100, pa: 100, left: 13, v: 6 }).tags).toEqual([]);
  });
  it("scores by CA closeness to the player being replaced", () => {
    expect(asCandidate({ ca: 150, pa: 160 }, 150).m).toBe(99);
    expect(asCandidate({ ca: 140, pa: 160 }, 150).m).toBe(80);
    expect(asCandidate({ ca: 20, pa: 160 }, 150).m).toBe(1);
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

describe("isFmFile", () => {
  it("accepts only .fm", () => {
    expect(isFmFile("a.FM")).toBe(true);
    expect(isFmFile("a.txt")).toBe(false);
  });
});
