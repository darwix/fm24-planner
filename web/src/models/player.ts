export type Tag = "exp" | "cheap" | "tl";

export interface SquadPlayer {
  id?: number | string;
  n: string;
  p: string;
  pos?: string[];
  // extra groups he is counted as cover in, besides p
  cover?: string[];
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
  // every position he can play (missing on players kept before it was stored)
  pos?: string[];
  a: number;
  ca: number;
  pa: number;
  c?: number;
  left?: number;
  v?: number;
  w?: number;
  tl?: boolean;
  ll?: boolean;
}

export interface Candidate extends Omit<PoolPlayer, "nat"> {
  nat?: string;
  m: number;
  tags: Tag[];
  at?: Record<string, number>;
}

export interface ScoutFilter {
  pos: string[];
  min: number;
  max: number;
  con: number;
  val: number | "";
  // transfer or loan listed only
  lst?: "tl" | "ll";
}

export interface ScoutList {
  name: string;
  f: ScoutFilter;
  fixed?: boolean;
}

export const TAG: Record<Tag, string> = {
  exp: "Contract expiring",
  cheap: "Cheap",
  tl: "Transfer listed",
};
