import axios from "axios";

// the dev server proxies /api to the SaveLens backend (see vite.config.ts)
export const api = axios.create({ baseURL: "" });

const fail = (error: any) =>
  Promise.reject(
    new Error(
      error.response?.data?.error ||
        error.response?.data?.message ||
        error.message,
    ),
  );
api.interceptors.response.use((r) => r, fail);

export const apiFetcher = (resource: string) =>
  api.get(resource).then((res) => res.data);
