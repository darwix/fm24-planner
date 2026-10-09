import type { SquadPlayer } from "@/models/player";

type Club = { name: string; snapshotId: string; gameDate: string };
type Vacant = { p: string; x: number; y: number };
// who sits where; player data (contract, value, positions) is read fresh from the save
export type Slot = Pick<SquadPlayer, "id" | "p" | "xi" | "x" | "y" | "cover">;
export type Arrangement = { slots: Slot[]; vacant: Vacant[] };
export type Session = { club: Club; arr: Arrangement };

export const toArrangement = (
  squad: SquadPlayer[],
  vacant: Vacant[],
): Arrangement => ({
  slots: squad.map(({ id, p, xi, x, y, cover }) => ({
    id,
    p,
    xi,
    x,
    y,
    cover,
  })),
  vacant,
});

export const applyArrangement = (
  arr: Arrangement,
  fresh: Map<SquadPlayer["id"], SquadPlayer>,
) => ({
  squad: arr.slots.flatMap((sl) => {
    const f = fresh.get(sl.id);
    return f ? [{ ...f, ...sl }] : [];
  }),
  vacant: arr.vacant,
});

// every call is guarded: storage can be blocked or hold broken data
const get = <T>(k: string): T | null => {
  try {
    const v = localStorage.getItem(k);
    return v ? (JSON.parse(v) as T) : null;
  } catch {
    return null;
  }
};
const put = (k: string, v: unknown) => {
  try {
    localStorage.setItem(k, JSON.stringify(v));
  } catch {
    // ignore: the planner still works without saving
  }
};

// one arrangement per save and club, so a new save version starts fresh
export const arrKey = (c: Pick<Club, "snapshotId" | "name">) =>
  `fm:arr:${c.snapshotId}:${c.name}`;
export const loadArr = (c: Club) => get<Arrangement>(arrKey(c));
export const saveArr = (c: Club, a: Arrangement) => put(arrKey(c), a);
export const loadSession = () => get<Session>("fm:session");
export const saveSession = (s: Session) => put("fm:session", s);
