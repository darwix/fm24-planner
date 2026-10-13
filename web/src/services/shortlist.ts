import { api } from "./api";

// the shortlist is a table in the snapshot's sqlite; search filters it with `shortlist=1`
const url = (snap: string) => `/api/snapshots/${snap}/shortlist`;

export const getShortlist = async (snap: string): Promise<number[]> =>
  (await api.get(url(snap))).data.ids;
export const addShortlist = (snap: string, id: number) =>
  api.put(`${url(snap)}/${id}`);
export const removeShortlist = (snap: string, id: number) =>
  api.delete(`${url(snap)}/${id}`);
