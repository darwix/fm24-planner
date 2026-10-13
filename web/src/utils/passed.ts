import { getPlanner, putPlanner } from "@/services/planner";

// ids of players swiped left on, stored in the snapshot; cached so quick swipes build on each other
const cache = new Map<string, Promise<number[]>>();

export const loadPassed = (snap: string): Promise<number[]> => {
  if (!cache.has(snap))
    cache.set(
      snap,
      getPlanner<number[]>(snap, "passed").then((v) => v ?? []),
    );
  return cache.get(snap)!;
};

const update = async (snap: string, f: (ids: number[]) => number[]) => {
  // chain on the cached list synchronously so each change sees the one before
  const next = loadPassed(snap).then(f);
  cache.set(snap, next);
  await putPlanner(snap, "passed", await next);
};

export const addPassed = (snap: string, id: number) =>
  update(snap, (ids) => (ids.includes(id) ? ids : [...ids, id]));
export const removePassed = (snap: string, id: number) =>
  update(snap, (ids) => ids.filter((x) => x !== id));
export const clearPassed = (snap: string) => update(snap, () => []);
