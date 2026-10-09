import type {
  Candidate,
  PoolPlayer,
  ScoutFilter,
  SquadPlayer,
  Tag,
} from "@/models/player";

export const TARGET: Record<string, number> = {
  GK: 2,
  DR: 2,
  DC: 4,
  DL: 2,
  DM: 2,
  AMC: 2,
  AMR: 2,
  AML: 2,
  ST: 2,
};

export const need = (squad: Pick<SquadPlayer, "p">[], pos: string) =>
  Math.max(0, TARGET[pos] - squad.filter((s) => s.p === pos).length);

export const matchFilter = (
  p: Pick<PoolPlayer, "p" | "a" | "left" | "v">,
  f: ScoutFilter,
) =>
  (!f.pos || p.p === f.pos) &&
  p.a >= f.min &&
  p.a <= f.max &&
  (!f.exp || (p.left !== undefined && p.left <= 12)) &&
  (!f.val || (p.v !== undefined && p.v <= f.val));

// ponytail: match = how close CA is to the player being replaced; attribute-profile matching comes later
export const asCandidate = (
  p: Pick<PoolPlayer, "ca" | "pa" | "left" | "v"> & Partial<PoolPlayer>,
  targetCa?: number,
): Candidate => {
  const tags: Tag[] = [
    ...(p.left !== undefined && p.left <= 12 ? (["exp"] as Tag[]) : []),
    ...(p.v !== undefined && p.v <= 5 ? (["cheap"] as Tag[]) : []),
  ];
  return {
    ...(p as PoolPlayer),
    m:
      targetCa === undefined
        ? Math.min(99, Math.round((p.ca * 0.6 + p.pa * 0.4) / 1.6))
        : Math.max(1, Math.min(99, 100 - 2 * Math.abs(p.ca - targetCa))),
    tags,
  };
};

export const contractLabel = (left?: number, end?: number) =>
  left === undefined ? "n/a" : left <= 12 ? `${left} months left` : `to ${end}`;

export const isFmFile = (name: string) => /\.fm$/i.test(name);
