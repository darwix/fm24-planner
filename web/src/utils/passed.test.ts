import { beforeEach, describe, expect, it } from "vitest";
import { addPassed, clearPassed, loadPassed } from "./passed";
import { scoutQuery, swipeBody } from "./player";

beforeEach(() => {
  localStorage.clear();
});

describe("passed players", () => {
  it("starts empty and keeps ids per snapshot", () => {
    expect(loadPassed("a")).toEqual([]);
    addPassed("a", 5);
    addPassed("a", 7);
    addPassed("b", 9);
    expect(loadPassed("a")).toEqual([5, 7]);
    expect(loadPassed("b")).toEqual([9]);
  });
  it("does not store an id twice", () => {
    addPassed("a", 5);
    addPassed("a", 5);
    expect(loadPassed("a")).toEqual([5]);
  });
  it("clears one snapshot", () => {
    addPassed("a", 5);
    clearPassed("a");
    expect(loadPassed("a")).toEqual([]);
  });
  it("survives broken storage", () => {
    localStorage.setItem("fm:passed:a", "{nope");
    expect(loadPassed("a")).toEqual([]);
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
