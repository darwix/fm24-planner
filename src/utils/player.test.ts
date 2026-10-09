import { describe, it, expect } from "vitest";
import { monthsLeft, toPool } from "./player";

describe("monthsLeft", () => {
  it("counts whole months from the game date to the contract end", () => {
    expect(monthsLeft("2053-06-30", "2053-02-18")).toBe(4);
    expect(monthsLeft("2054-06-30", "2053-02-18")).toBe(16);
  });
  it("drops a month when the end day is before the game day", () => {
    expect(monthsLeft("2053-03-10", "2053-02-18")).toBe(0);
  });
  it("never goes below zero", () => {
    expect(monthsLeft("2052-12-01", "2053-02-18")).toBe(0);
  });
  it("is undefined when the contract is unknown", () => {
    expect(monthsLeft(null, "2053-02-18")).toBeUndefined();
    expect(monthsLeft(undefined, "2053-02-18")).toBeUndefined();
  });
});

describe("toPool", () => {
  const api = {
    id: 7,
    name: "Andrea Pietrangeli",
    club: "Aston Villa",
    nationalities: ["Italy"],
    positions: ["MC", "AMC"],
    age: 28,
    ca: 184,
    pa: 184,
    wage: 455_280,
    contractEnd: "2056-06-30",
    value: 96_391_200,
  };
  it("converts value to millions and wage to thousands a week", () => {
    const p = toPool(api, "2053-02-18");
    expect(p).toMatchObject({
      id: 7,
      n: "Andrea Pietrangeli",
      cl: "Aston Villa",
      nat: "Italy",
      a: 28,
      v: 96.4,
      w: 455,
      left: 40,
    });
  });
  it("uses the first position as the backend names it", () => {
    expect(toPool({ ...api, positions: ["AMC"] }, "2053-02-18").p).toBe("AMC");
  });
  it("leaves contract, value and wage undefined when the save has none", () => {
    const p = toPool(
      { ...api, wage: null, contractEnd: null, value: 0 },
      "2053-02-18",
    );
    expect(p.left).toBeUndefined();
    expect(p.w).toBeUndefined();
    expect(p.v).toBeUndefined();
  });
});
