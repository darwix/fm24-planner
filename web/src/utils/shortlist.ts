import type { PoolPlayer } from "@/models/player";

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

// filtered and sorted copy; a missing value (unknown price, no contract data) always sorts last
export const shortlistView = (
  list: PoolPlayer[],
  f: ShortlistFilter,
  s: { k: SortKey; dir: 1 | -1 },
) =>
  list
    .filter(
      (p) =>
        (!f.pos.length || f.pos.includes(p.p)) &&
        (!f.tl || p.tl) &&
        (!f.ll || p.ll) &&
        (!f.exp || (p.left !== undefined && p.left <= 12)),
    )
    .sort((a, b) => {
      const x = a[s.k],
        y = b[s.k];
      if (x === undefined || y === undefined)
        return (x === undefined ? 1 : 0) - (y === undefined ? 1 : 0);
      return s.dir * (x > y ? 1 : x < y ? -1 : 0);
    });
