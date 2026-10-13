import { afterEach, describe, expect, it, vi } from "vitest";
import { api } from "./api";
import { addShortlist, getShortlist, removeShortlist } from "./shortlist";

afterEach(() => vi.restoreAllMocks());

describe("shortlist service", () => {
  it("reads the ids on the shortlist", async () => {
    const get = vi
      .spyOn(api, "get")
      .mockResolvedValue({ data: { ids: [2, 3] } });
    expect(await getShortlist("abc")).toEqual([2, 3]);
    expect(get).toHaveBeenCalledWith("/api/snapshots/abc/shortlist");
  });
  it("adds and removes one player", async () => {
    const put = vi.spyOn(api, "put").mockResolvedValue({ data: {} });
    const del = vi.spyOn(api, "delete").mockResolvedValue({ data: {} });
    await addShortlist("abc", 7);
    await removeShortlist("abc", 7);
    expect(put).toHaveBeenCalledWith("/api/snapshots/abc/shortlist/7");
    expect(del).toHaveBeenCalledWith("/api/snapshots/abc/shortlist/7");
  });
});
