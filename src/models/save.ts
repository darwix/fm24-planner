export interface SaveFile {
  id: string;
  name: string;
  size: number;
  cached: boolean;
  snapshotId?: string;
}

export interface ImportJob {
  id: string;
  status: "queued" | "running" | "complete" | "failed";
  progress: number;
  message?: string;
  snapshotId?: string;
}

export interface Club {
  id: number;
  name: string;
}

export interface Snapshot {
  clubs: Club[];
  gameDate: string;
  name: string;
}

export interface ApiPlayer {
  id: number;
  name: string;
  age: number;
  ca: number;
  pa: number;
  positions: string[];
}
