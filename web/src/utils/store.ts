import type { SquadPlayer } from "@/models/player";
import {
  getPlanner,
  getSession,
  putPlanner,
  putSession,
} from "@/services/planner";

type Club = { name: string; snapshotId: string; gameDate: string };
type Vacant = { p: string; x: number; y: number };
// who sits where; player data (contract, value, positions) is read fresh from the save
export type Slot = Pick<SquadPlayer, "id" | "p" | "xi" | "x" | "y" | "cover">;
export type Arrangement = { slots: Slot[]; vacant: Vacant[] };
// which save and club the app has open; the arrangement is stored per club
export type Session = { club: Club };

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

// one arrangement per club, stored inside the save's snapshot, so a new save version starts fresh
export const arrKey = (c: Pick<Club, "name">) => `squad:${c.name}`;
export const loadArr = (c: Club) =>
  getPlanner<Arrangement>(c.snapshotId, arrKey(c));
export const saveArr = (c: Club, a: Arrangement) =>
  putPlanner(c.snapshotId, arrKey(c), a);
export const loadSession = () => getSession<Session>();
export const saveSession = (s: Session) => putSession(s);
