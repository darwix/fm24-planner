import type { PoolPlayer } from "@/models/player";
import type { ApiPlayer } from "@/models/save";

// whole months from the game date to the contract end, undefined when the save has no contract
export const monthsLeft = (
  end: string | null | undefined,
  now: string,
): number | undefined => {
  if (!end) return undefined;
  const [ey, em, ed] = end.split("-").map(Number);
  const [ny, nm, nd] = now.split("-").map(Number);
  return Math.max(0, (ey - ny) * 12 + (em - nm) - (ed < nd ? 1 : 0));
};

// index of each UI position in the backend's catalog (GET /api/attributes -> positions)
export const POS_INDEX: Record<string, number> = {
  GK: 0,
  DL: 2,
  DC: 3,
  DR: 4,
  DM: 5,
  AML: 9,
  AMC: 10,
  AMR: 11,
  ST: 12,
};

// value in £M, wage in £k a week (the save's base currency)
export const toPool = (p: ApiPlayer, gameDate: string): PoolPlayer => ({
  id: p.id,
  n: p.name,
  cl: p.club ?? "",
  nat: p.nationalities[0] ?? "",
  p: p.positions[0],
  a: p.age,
  ca: p.ca,
  pa: p.pa,
  c: p.contractEnd ? Number(p.contractEnd.slice(0, 4)) : undefined,
  left: monthsLeft(p.contractEnd, gameDate),
  v: p.value ? Math.round(p.value / 1e5) / 10 : undefined,
  w: p.wage ? Math.round(p.wage / 1000) : undefined,
});
