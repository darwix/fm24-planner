import { api } from "./api";

// planner state lives in the snapshot's sqlite on the backend
const url = (snap: string, key: string) =>
  `/api/snapshots/${snap}/planner/${encodeURIComponent(key)}`;

export const getPlanner = async <T>(
  snap: string,
  key: string,
): Promise<T | null> => (await api.get(url(snap, key))).data.value ?? null;

// writes to one key go out one at a time, so a slow request can't land after a newer one
const chains = new Map<string, Promise<unknown>>();
export const putPlanner = (snap: string, key: string, value: unknown) => {
  const k = url(snap, key);
  const next = (chains.get(k) ?? Promise.resolve())
    .catch(() => {})
    .then(() => api.put(k, { value }));
  chains.set(k, next);
  return next;
};

export const getSession = async <T>(): Promise<T | null> =>
  (await api.get("/api/session")).data.value ?? null;
export const putSession = (value: unknown) =>
  api.put("/api/session", { value });
