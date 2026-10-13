import { afterEach, describe, expect, it, vi } from "vitest";
import { api } from "./api";
import { getPlanner, getSession, putPlanner, putSession } from "./planner";

afterEach(() => vi.restoreAllMocks());

describe("planner service", () => {
  it("reads a stored value, url-encoding the key", async () => {
    const get = vi
      .spyOn(api, "get")
      .mockResolvedValue({ data: { value: [1, 2] } });
    expect(await getPlanner("abc", "squad:Aston Villa")).toEqual([1, 2]);
    expect(get).toHaveBeenCalledWith(
      "/api/snapshots/abc/planner/squad%3AAston%20Villa",
    );
  });
  it("returns null when nothing is stored", async () => {
    vi.spyOn(api, "get").mockResolvedValue({ data: { value: null } });
    expect(await getPlanner("abc", "kept")).toBeNull();
  });
  it("sends writes to one key in order, even after a failure", async () => {
    const sent: number[] = [];
    let release!: () => void;
    const first = new Promise<void>((r) => (release = r));
    vi.spyOn(api, "put").mockImplementation(async (_url, body: any) => {
      if (body.value === 1) {
        await first;
        sent.push(1);
        throw new Error("down");
      }
      sent.push(body.value);
      return { data: {} };
    });
    const a = putPlanner("abc", "passed", 1);
    const b = putPlanner("abc", "passed", 2);
    release();
    await expect(a).rejects.toThrow("down");
    await b;
    expect(sent).toEqual([1, 2]);
  });
  it("reads and writes the session", async () => {
    vi.spyOn(api, "get").mockResolvedValue({
      data: { value: { club: { name: "X" } } },
    });
    const put = vi.spyOn(api, "put").mockResolvedValue({ data: {} });
    expect(await getSession()).toEqual({ club: { name: "X" } });
    await putSession({ club: { name: "Y", snapshotId: "s", gameDate: "d" } });
    expect(put).toHaveBeenCalledWith("/api/session", {
      value: { club: { name: "Y", snapshotId: "s", gameDate: "d" } },
    });
  });
});
