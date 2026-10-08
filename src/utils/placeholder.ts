import type { PoolPlayer } from "@/models/player";

export const GROUPS: Record<string, string[]> = {
  Technical: [
    "Crossing",
    "Dribbling",
    "Finishing",
    "First touch",
    "Heading",
    "Marking",
    "Passing",
    "Tackling",
    "Technique",
  ],
  Mental: [
    "Anticipation",
    "Composure",
    "Decisions",
    "Determination",
    "Positioning",
    "Teamwork",
    "Vision",
    "Work rate",
  ],
  Physical: [
    "Acceleration",
    "Agility",
    "Balance",
    "Jumping",
    "Pace",
    "Stamina",
    "Strength",
  ],
};

const hash = (s: string) =>
  [...s].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);

// scouting-pool players have no parsed attributes yet, so derive stable fakes from CA
export const fakeAttr = (p: Pick<PoolPlayer, "n" | "ca">, a: string) =>
  Math.max(
    3,
    Math.min(20, Math.round(p.ca / 9.5 + ((hash(p.n + a) % 7) - 3) * 0.9)),
  );
