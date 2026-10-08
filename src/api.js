const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

export function createApi(base = "", fetchFn = (...a) => fetch(...a)) {
  const call = async (path, init) => {
    const res = await fetchFn(base + path, init);
    const body = await res.json();
    if (!res.ok) throw new Error(body.message || `HTTP ${res.status}`);
    return body;
  };
  return {
    listSaves: async () => (await call("/api/saves")).saves,
    // resolves with the snapshot id once the import job completes
    async importSave(saveId, onProgress, pollMs = 500) {
      const post = {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ saveId }),
      };
      let job = await call("/api/imports", post);
      onProgress(job);
      while (job.status !== "complete") {
        if (job.status === "failed")
          throw new Error(job.message || "Import failed");
        await sleep(pollMs);
        job = await call(`/api/imports/${job.id}`);
        onProgress(job);
      }
      return job.snapshotId;
    },
    searchPlayers: (snap, params = {}) =>
      call(`/api/snapshots/${snap}/players?${new URLSearchParams(params)}`),
    player: (snap, id) => call(`/api/snapshots/${snap}/players/${id}`),
  };
}
