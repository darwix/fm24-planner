import { describe, it, expect, vi } from "vitest";
import { createApi } from "./api.js";

const reply = (body, ok = true) => ({
  ok,
  status: ok ? 200 : 500,
  json: async () => body,
});

describe("api", () => {
  it("lists saves", async () => {
    const fetch = vi.fn(async () =>
      reply({ saves: [{ id: "a", name: "x.fm" }] }),
    );
    const api = createApi("http://h", fetch);
    expect(await api.listSaves()).toEqual([{ id: "a", name: "x.fm" }]);
    expect(fetch).toHaveBeenCalledWith("http://h/api/saves", undefined);
  });

  it("imports a save and polls until complete, reporting progress", async () => {
    const jobs = [
      { id: "j", status: "running", progress: 40, message: "Parsing" },
      { id: "j", status: "complete", progress: 100, snapshotId: "s1" },
    ];
    const fetch = vi.fn(async (url, init) =>
      init?.method === "POST"
        ? reply({ id: "j", status: "queued", progress: 0 })
        : reply(jobs.shift()),
    );
    const seen = [];
    const api = createApi("http://h", fetch);
    const snap = await api.importSave("a", (j) => seen.push(j.progress), 0);
    expect(snap).toBe("s1");
    expect(seen).toEqual([0, 40, 100]);
  });

  it("throws when the import fails", async () => {
    const fetch = vi.fn(async (url, init) =>
      init?.method === "POST"
        ? reply({ id: "j", status: "running" })
        : reply({ id: "j", status: "failed", message: "bad save" }),
    );
    await expect(
      createApi("http://h", fetch).importSave("a", () => {}, 0),
    ).rejects.toThrow("bad save");
  });

  it("searches players with query params", async () => {
    const fetch = vi.fn(async () => reply({ players: [{ id: 1 }], total: 1 }));
    const r = await createApi("http://h", fetch).searchPlayers("s1", {
      limit: 5,
      page: 2,
    });
    expect(r.players).toEqual([{ id: 1 }]);
    expect(fetch).toHaveBeenCalledWith(
      "http://h/api/snapshots/s1/players?limit=5&page=2",
      undefined,
    );
  });

  it("throws on http errors", async () => {
    const fetch = vi.fn(async () => reply({ message: "nope" }, false));
    await expect(createApi("http://h", fetch).listSaves()).rejects.toThrow(
      "nope",
    );
  });
});
