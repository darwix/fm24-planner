import { describe, it, expect } from "vitest";
import { POS_INDEX, nameQuery, scoutQuery, monthsLeft, toPool } from "./player";

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
  it("treats a player with no club and no contract as expired", () => {
    const p = toPool(
      { ...api, club: null, contractEnd: null, wage: null, value: null },
      "2053-02-18",
    );
    expect(p.left).toBe(0);
  });
  it("keeps a clubbed player with no contract unknown", () => {
    expect(
      toPool({ ...api, contractEnd: null }, "2053-02-18").left,
    ).toBeUndefined();
  });
  it("keeps every position he can play", () => {
    expect(toPool(api, "2053-02-18").pos).toEqual(["MC", "AMC"]);
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

describe("scoutQuery", () => {
  const f = { pos: [] as string[], min: 18, max: 23, con: 0, val: "" as const };
  it("sends only age, sort and limit when no extra filters are on", () => {
    expect(scoutQuery(f, "2053-03-01")).toEqual({
      ageMin: 18,
      ageMax: 23,
      sort: "pa",
      limit: 250,
    });
  });
  it("sends main positions as a comma list", () => {
    const q = scoutQuery({ ...f, pos: ["ST", "DC"] }, "2053-03-01");
    expect(q.mainPosition).toBe(`${POS_INDEX.ST},${POS_INDEX.DC}`);
  });
  it("sends the expiry cutoff for N months and max value", () => {
    const q = scoutQuery({ ...f, con: 12, val: 2 }, "2053-03-01");
    expect(q.contractEndMax).toBe("2054-03-31");
    expect(q.valueMax).toBe(2_000_000);
    expect(q.valueUnknown).toBeUndefined();
    expect(scoutQuery({ ...f, con: 6 }, "2053-03-01").contractEndMax).toBe(
      "2053-09-30",
    );
  });
  it("expired means no contract at any club, and a price keeps unknown values", () => {
    const q = scoutQuery({ ...f, con: -1, val: 5 }, "2053-03-01");
    expect(q).not.toHaveProperty("club");
    expect(q.noContract).toBe(1);
    expect(q.contractEndMax).toBeUndefined();
    expect(q.valueUnknown).toBe(1);
  });
});

describe("nameQuery", () => {
  it("searches by trimmed name with no other filters, ranked by potential", () => {
    expect(nameQuery("  Soldi ")).toEqual({
      q: "Soldi",
      sort: "pa",
      limit: 250,
    });
  });
  it("is null for an empty or blank search", () => {
    expect(nameQuery("")).toBeNull();
    expect(nameQuery("   ")).toBeNull();
  });
});

describe("listed filters", () => {
  const f = { pos: [] as string[], min: 18, max: 23, con: 0, val: "" as const };
  it("sends transferListed or loanListed for the listed chip", () => {
    expect(scoutQuery({ ...f, lst: "tl" }, "2053-03-01").transferListed).toBe(
      1,
    );
    expect(scoutQuery({ ...f, lst: "ll" }, "2053-03-01").loanListed).toBe(1);
    expect(scoutQuery(f, "2053-03-01")).not.toHaveProperty("transferListed");
  });
  it("maps the API's listed flags", () => {
    const api = {
      id: 1,
      name: "A",
      age: 20,
      ca: 100,
      pa: 120,
      club: "X",
      nationalities: ["ESP"],
      positions: ["ST"],
      wage: 100,
      contractEnd: "2054-06-30",
      value: 1e6,
      transferListed: true,
      loanListed: false,
    };
    expect(toPool(api, "2053-03-01")).toMatchObject({ tl: true, ll: false });
  });
});
