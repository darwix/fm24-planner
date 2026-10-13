import { describe, it, expect } from "vitest";
import { POS_INDEX, scoutQuery, swipeBody } from "./player";
import { shortlistQuery, stars } from "./shortlist";

describe("stars", () => {
  it("rounds CA and PA to half stars, 200 = 5", () => {
    expect(stars(148, 175)).toEqual({ ca: 3.5, pa: 4.5 });
    expect(stars(200, 200)).toEqual({ ca: 5, pa: 5 });
    expect(stars(0, 4)).toEqual({ ca: 0, pa: 0 });
  });
});

describe("shortlistQuery", () => {
  const none = { pos: [], tl: false, ll: false, exp: false };
  const asc = { k: "v", dir: 1 } as const;
  it("asks for the shortlist only, leaving out your club", () => {
    expect(shortlistQuery(none, asc, "2053-03-08", 305)).toMatchObject({
      shortlist: 1,
      excludeClub: 305,
      sort: "value",
      direction: "asc",
      limit: 250,
    });
  });
  it("maps each column to the backend sort", () => {
    const sort = (k: any, dir: 1 | -1) =>
      shortlistQuery(none, { k, dir }, "2053-03-08", 1);
    expect(sort("n", 1)).toMatchObject({ sort: "name", direction: "asc" });
    expect(sort("p", 1).sort).toBe("position");
    expect(sort("ca", -1)).toMatchObject({ sort: "ca", direction: "desc" });
    expect(sort("left", 1).sort).toBe("contractEnd");
    expect(sort("w", 1).sort).toBe("wage");
  });
  it("filters by any picked position he can play", () => {
    const q = shortlistQuery(
      { ...none, pos: ["DC", "DM"] },
      asc,
      "2053-03-08",
      1,
    );
    expect(q).toMatchObject({
      position: `${POS_INDEX.DC},${POS_INDEX.DM}`,
      positionMatch: "or",
    });
    expect(shortlistQuery(none, asc, "2053-03-08", 1).position).toBeUndefined();
  });
  it("status chips become backend filters; expiring keeps free agents", () => {
    const q = shortlistQuery(
      { pos: [], tl: true, ll: true, exp: true },
      asc,
      "2053-03-08",
      1,
    );
    expect(q).toMatchObject({
      transferListed: 1,
      loanListed: 1,
      contractEndMax: "2054-04-07",
      orNoContract: 1,
    });
    const off = shortlistQuery(none, asc, "2053-03-08", 1);
    expect(off.transferListed).toBeUndefined();
    expect(off.contractEndMax).toBeUndefined();
  });
});

describe("swipeBody", () => {
  it("pages the scouting query, nothing excluded", () => {
    const q = scoutQuery(
      { pos: ["ST"], min: 18, max: 23, con: 0, val: "" },
      "2053-03-01",
    );
    const b = swipeBody(q, 2);
    expect(b).toMatchObject({ page: 2, limit: 50, ageMin: 18 });
    expect("excludeIds" in b).toBe(false);
  });
});
