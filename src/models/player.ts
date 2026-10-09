export type Tag = "exp" | "cheap";

export interface SquadPlayer {
  id?: number | string;
  n: string;
  p: string;
  a: number;
  ca: number;
  pa: number;
  c?: number;
  left?: number;
  v?: number;
  w?: number;
  xi?: 1;
  x?: number;
  y?: number;
  at: Record<string, number>;
}

// scouting / shortlist candidate before it is tagged
export interface PoolPlayer {
  id: number;
  n: string;
  cl: string;
  nat: string;
  p: string;
  a: number;
  ca: number;
  pa: number;
  c?: number;
  left?: number;
  v?: number;
  w?: number;
}

export interface Candidate extends Omit<PoolPlayer, "nat"> {
  nat?: string;
  m: number;
  tags: Tag[];
  at?: Record<string, number>;
}

export interface ScoutFilter {
  pos: string;
  min: number;
  max: number;
  exp: boolean;
  val: number | "";
}

export interface ScoutList {
  name: string;
  f: ScoutFilter;
  fixed?: boolean;
}

export const TAG: Record<Tag, string> = {
  exp: "Contract expiring",
  cheap: "Cheap",
};
