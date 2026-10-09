import { afterEach, describe, expect, it, vi } from "vitest";
import { api } from "./api";
import { loadSquad } from "./save";

afterEach(() => vi.restoreAllMocks());

const player = (id: number) => ({
  id,
  name: `P${id}`,
  age: 20,
  ca: 100,
  pa: 110,
  positions: ["ST"],
  club: "X",
  nationalities: [],
  wage: null,
  contractEnd: null,
  value: null,
});
const arr = {
  slots: [
    { id: 1, p: "ST" },
    { id: 2, p: "DC" },
  ],
  vacant: [],
};
const idOf = (url: string) => Number(url.split("/").pop());

describe("loadSquad", () => {
  it("rebuilds every saved player", async () => {
    vi.spyOn(api, "get").mockImplementation(async (url: string) => ({
      data: player(idOf(url)),
    }));
    const r = await loadSquad("s", "2053-03-01", arr);
    expect(r.squad.map((s) => s.id)).toEqual([1, 2]);
  });
  it("drops a player the save no longer has (404)", async () => {
    vi.spyOn(api, "get").mockImplementation(async (url: string) => {
      if (idOf(url) === 2)
        throw Object.assign(new Error("gone"), { status: 404 });
      return { data: player(1) };
    });
    const r = await loadSquad("s", "2053-03-01", arr);
    expect(r.squad.map((s) => s.id)).toEqual([1]);
  });
  it("fails instead of returning a smaller squad on other errors", async () => {
    vi.spyOn(api, "get").mockImplementation(async (url: string) => {
      if (idOf(url) === 2) throw new Error("Network Error");
      return { data: player(1) };
    });
    await expect(loadSquad("s", "2053-03-01", arr)).rejects.toThrow();
  });
});
