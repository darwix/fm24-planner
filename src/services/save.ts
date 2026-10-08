import useSWR from "swr";
import type { Candidate, PoolPlayer } from "@/models/player";
import type { ApiPlayer, ImportJob, SaveFile, Snapshot } from "@/models/save";
import { api, apiFetcher, mockFetcher } from "./api";

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export const listSaves = async (): Promise<SaveFile[]> =>
  (await api.get("/api/saves")).data.saves;

// resolves with the snapshot id once the import job completes
export async function importSave(
  saveId: string,
  onProgress: (job: ImportJob) => void,
  pollMs = 500,
): Promise<string> {
  let job: ImportJob = (await api.post("/api/imports", { saveId })).data;
  onProgress(job);
  while (job.status !== "complete") {
    if (job.status === "failed")
      throw new Error(job.message || "Import failed");
    await sleep(pollMs);
    job = (await api.get(`/api/imports/${job.id}`)).data;
    onProgress(job);
  }
  return job.snapshotId as string;
}

export const getSnapshot = async (snap: string): Promise<Snapshot> =>
  (await api.get(`/api/snapshots/${snap}`)).data;

export const searchPlayers = async (
  snap: string,
  params: Record<string, string | number> = {},
): Promise<{ players: ApiPlayer[]; total: number }> =>
  (await api.get(`/api/snapshots/${snap}/players`, { params })).data;

export function useMock<T>(name: "cands" | "pool") {
  const { data } = useSWR<T[]>(`/${name}.json`, mockFetcher);
  return data;
}
export const useCands = () => useMock<Candidate>("cands");
export const usePool = () => useMock<PoolPlayer>("pool");

export interface RealPlayer {
  clubId: number;
  club: string;
  nationalities: string[];
  attributes: Record<string, number>;
  positions: string[];
}
export const useRealPlayer = (snap?: string, id?: number | string) =>
  useSWR<RealPlayer>(
    snap && id !== undefined ? `/api/snapshots/${snap}/players/${id}` : null,
    apiFetcher,
  );
export const useAttributeCatalog = () =>
  useSWR<{ attributes: { group: string; key: string; label: string }[] }>(
    "/api/attributes",
    apiFetcher,
  );
