import type { SquadPlayer } from "@/models/player";
import type { ApiPlayer } from "@/models/save";

// 4-2-3-1, same coordinates as the mock pitch
export const SLOTS = [
  { p: "GK", x: 50, y: 90 },
  { p: "DR", x: 86, y: 70 },
  { p: "DC", x: 63, y: 74 },
  { p: "DC", x: 37, y: 74 },
  { p: "DL", x: 14, y: 70 },
  { p: "DM", x: 34, y: 55 },
  { p: "DM", x: 66, y: 55 },
  { p: "AM", x: 50, y: 38 },
  { p: "AMR", x: 84, y: 20 },
  { p: "AML", x: 16, y: 20 },
  { p: "ST", x: 50, y: 12 },
];

// ponytail: contract/value/wage unavailable until the backend exposes them
export function buildSquad(players: ApiPlayer[]): SquadPlayer[] {
  const byCa = [...players].sort((a, b) => b.ca - a.ca);
  const used = new Set<number>();
  const slotFor = new Map<number, (typeof SLOTS)[number]>();
  for (const slot of SLOTS) {
    const pick = byCa.find(
      (p) => !used.has(p.id) && p.positions.includes(slot.p),
    );
    if (pick) {
      used.add(pick.id);
      slotFor.set(pick.id, slot);
    }
  }
  return byCa.map((p) => {
    const slot = slotFor.get(p.id);
    return {
      id: p.id,
      n: p.name,
      p: slot ? slot.p : p.positions[0],
      a: p.age,
      ca: p.ca,
      pa: p.pa,
      at: {},
      ...(slot && { xi: 1 as const, x: slot.x, y: slot.y }),
    };
  });
}
