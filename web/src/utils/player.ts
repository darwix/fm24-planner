import type { PoolPlayer, ScoutFilter } from "@/models/player";
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
// last contract-end date that still counts as expiring (<= `months` left)
// ponytail: month-end game dates (e.g. 31 Jan) can overshoot by a day or two
const expiryCutoff = (now: string, months: number) => {
  const [y, m, d] = now.split("-").map(Number);
  return new Date(Date.UTC(y, m + months, d - 1)).toISOString().slice(0, 10);
};

// con: 0 any, -1 expired (no contract, at a club or free), N = contract ends within N months
export const scoutQuery = (f: ScoutFilter, gameDate: string) => ({
  ...(f.pos.length && {
    mainPosition: f.pos.map((x) => POS_INDEX[x]).join(","),
  }),
  ageMin: f.min,
  ageMax: f.max,
  ...(f.con === -1 && { noContract: 1 }),
  ...(f.con > 0 && { contractEndMax: expiryCutoff(gameDate, f.con) }),
  ...(f.val && { valueMax: f.val * 1_000_000 }),
  ...(f.val && f.con === -1 && { valueUnknown: 1 }),
  ...(f.lst === "tl" && { transferListed: 1 }),
  ...(f.lst === "ll" && { loanListed: 1 }),
  sort: "pa",
  limit: 250,
});

// name search ignores the list filters so any player can be found and compared
export const nameQuery = (text: string) =>
  text.trim() ? { q: text.trim(), sort: "pa", limit: 250 } : null;

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
  // no club and no contract = free agent
  left: !p.club && !p.contractEnd ? 0 : monthsLeft(p.contractEnd, gameDate),
  v: p.value ? Math.round(p.value / 1e5) / 10 : undefined,
  w: p.wage ? Math.round(p.wage / 1000) : undefined,
  tl: p.transferListed,
  ll: p.loanListed,
});

export const SWIPE_PAGE = 50;

// POST body for one page of the swipe stack: the same filters, minus passed players
export const swipeBody = (
  query: ReturnType<typeof scoutQuery>,
  passed: number[],
  page: number,
) => ({
  ...query,
  limit: SWIPE_PAGE,
  page,
  ...(passed.length && { excludeIds: passed }),
});
