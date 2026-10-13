import { POS_INDEX, expiryCutoff } from "./player";

// half stars like the game; ponytail: fixed scale (200 = 5), the game rates against your squad
export const stars = (ca: number, pa: number) => ({
  ca: Math.round(ca / 20) / 2,
  pa: Math.round(pa / 20) / 2,
});

export type SortKey = "n" | "p" | "ca" | "v" | "left" | "w";
export interface ShortlistFilter {
  pos: string[];
  tl: boolean;
  ll: boolean;
  exp: boolean;
}

const SORT: Record<SortKey, string> = {
  n: "name",
  p: "position",
  ca: "ca",
  v: "value",
  left: "contractEnd",
  w: "wage",
};

// search params for the shortlist page: the backend filters and sorts (unknowns last)
export const shortlistQuery = (
  f: ShortlistFilter,
  s: { k: SortKey; dir: 1 | -1 },
  gameDate: string,
  clubId: number,
) => ({
  shortlist: 1,
  excludeClub: clubId,
  ...(f.pos.length && {
    position: f.pos.map((x) => POS_INDEX[x]).join(","),
    positionMatch: "or",
  }),
  ...(f.tl && { transferListed: 1 }),
  ...(f.ll && { loanListed: 1 }),
  ...(f.exp && {
    contractEndMax: expiryCutoff(gameDate, 12),
    orNoContract: 1,
  }),
  sort: SORT[s.k],
  direction: s.dir === 1 ? "asc" : "desc",
  limit: 250,
});
