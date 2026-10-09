import useSWR from "swr";
import type { ApiPlayer, ImportJob, SaveFile, Snapshot } from "@/models/save";
import { toSquadPlayer } from "@/utils/squad";
import { applyArrangement, type Arrangement } from "@/utils/store";
import { api, apiFetcher } from "./api";

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

// null params = do not fetch
export function useSearch(
  snap: string | undefined,
  params: Record<string, string | number> | null,
) {
  const { data } = useSWR<ApiPlayer[]>(
    snap && params ? [`/api/snapshots/${snap}/players`, params] : null,
    ([url, p]: [string, Record<string, string | number>]) =>
      api.get(url, { params: p }).then((r) => r.data.players),
  );
  return data;
}

export interface RealPlayer extends ApiPlayer {
  clubId: number;
  attributes: Record<string, number>;
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

// rebuild a saved arrangement on player data read from the save now
export async function loadSquad(
  snap: string,
  gameDate: string,
  arr: Arrangement,
) {
  const fresh = new Map<
    Arrangement["slots"][number]["id"],
    ReturnType<typeof toSquadPlayer>
  >();
  await Promise.all(
    arr.slots.map(async (s) => {
      try {
        const { data } = await api.get(
          `/api/snapshots/${snap}/players/${s.id}`,
        );
        fresh.set(s.id, toSquadPlayer(data, gameDate));
      } catch (e: any) {
        // only a real 404 means he left the save; anything else must not shrink the squad
        if (e?.status !== 404) throw e;
      }
    }),
  );
  return applyArrangement(arr, fresh);
}
