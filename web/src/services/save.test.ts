import { afterEach, describe, expect, it, vi } from "vitest";
import { api } from "./api";
import { importSave, listSaves, searchPlayers } from "./save";

afterEach(() => vi.restoreAllMocks());

describe("save service", () => {
  it("lists saves", async () => {
    vi.spyOn(api, "get").mockResolvedValue({ data: { saves: [{ id: "a" }] } });
    expect(await listSaves()).toEqual([{ id: "a" }]);
  });

  it("imports a save and polls until complete, reporting progress", async () => {
    vi.spyOn(api, "post").mockResolvedValue({
      data: { id: "j", status: "queued", progress: 0 },
    });
    const jobs = [
      { id: "j", status: "running", progress: 40 },
      { id: "j", status: "complete", progress: 100, snapshotId: "s1" },
    ];
    vi.spyOn(api, "get").mockImplementation(async () => ({
      data: jobs.shift(),
    }));
    const seen: number[] = [];
    expect(await importSave("a", (j) => seen.push(j.progress), 0)).toBe("s1");
    expect(seen).toEqual([0, 40, 100]);
  });

  it("throws when the import fails", async () => {
    vi.spyOn(api, "post").mockResolvedValue({
      data: { id: "j", status: "running", progress: 1 },
    });
    vi.spyOn(api, "get").mockResolvedValue({
      data: { id: "j", status: "failed", message: "bad save" },
    });
    await expect(importSave("a", () => {}, 0)).rejects.toThrow("bad save");
  });

  it("searches players with query params", async () => {
    const get = vi
      .spyOn(api, "get")
      .mockResolvedValue({ data: { players: [{ id: 1 }], total: 1 } });
    const r = await searchPlayers("s1", { limit: 5, page: 2 });
    expect(r.players).toEqual([{ id: 1 }]);
    expect(get).toHaveBeenCalledWith("/api/snapshots/s1/players", {
      params: { limit: 5, page: 2 },
    });
  });
});
