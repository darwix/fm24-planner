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
  AM: 2,
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
  (!f.exp || p.left <= 12) &&
  (!f.val || p.v <= f.val);

// ponytail: placeholder match formula, replace once real profile matching exists
export const asCandidate = (
  p: Pick<PoolPlayer, "ca" | "pa" | "left" | "v"> & Partial<PoolPlayer>,
): Candidate => {
  const tags: Tag[] = [
    ...(p.left <= 12 ? (["exp"] as Tag[]) : []),
    ...(p.v <= 5 ? (["cheap"] as Tag[]) : []),
  ];
  return {
    ...(p as PoolPlayer),
    m: Math.min(99, Math.round((p.ca * 0.6 + p.pa * 0.4) / 1.6)),
    tags,
  };
};

export const contractLabel = (left: number, end: number) =>
  left <= 12 ? `${left} months left` : `to ${end}`;

export const isFmFile = (name: string) => /\.fm$/i.test(name);
