import { atom } from "recoil";
import type { SquadPlayer } from "@/models/player";

export const squadState = atom<SquadPlayer[]>({
  key: "squadState",
  default: [],
});
export const selectedState = atom<number>({ key: "selectedState", default: 0 });
export const clubState = atom<{ name: string; snapshotId: string } | null>({
  key: "clubState",
  default: null,
});
