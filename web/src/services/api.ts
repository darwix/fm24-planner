import axios from "axios";

// the dev server proxies /api to the backend (see vite.config.ts)
export const api = axios.create({ baseURL: "" });

const fail = (error: any) =>
  Promise.reject(
    Object.assign(
      new Error(
        error.response?.data?.error ||
          error.response?.data?.message ||
          error.message,
      ),
      { status: error.response?.status },
    ),
  );
api.interceptors.response.use((r) => r, fail);

export const apiFetcher = (resource: string) =>
  api.get(resource).then((res) => res.data);
