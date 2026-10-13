import { atom } from "recoil";
import type { PoolPlayer, ScoutList } from "@/models/player";

export const listsState = atom<ScoutList[]>({
  key: "listsState",
  default: [
    {
      name: "Promising youngsters",
      f: { pos: [], min: 18, max: 23, con: 0, val: "" },
      fixed: true,
    },
    {
      name: "Contract expiring",
      f: { pos: [], min: 16, max: 45, con: 12, val: "" },
      fixed: true,
    },
  ],
});
export const listIndexState = atom<number>({
  key: "listIndexState",
  default: 0,
});

// ids on the shortlist (swiped right on)
export const keptState = atom<number[]>({
  key: "keptState",
  default: [],
});
