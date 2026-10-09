// ids of players swiped left on, kept per save snapshot
const key = (snap: string) => `fm:passed:${snap}`;

export const loadPassed = (snap: string): number[] => {
  try {
    const v = JSON.parse(localStorage.getItem(key(snap)) ?? "[]");
    return Array.isArray(v) ? v : [];
  } catch {
    return [];
  }
};

const save = (snap: string, ids: number[]) => {
  try {
    localStorage.setItem(key(snap), JSON.stringify(ids));
  } catch {
    // storage blocked: passes only last for this page view
  }
};

export const addPassed = (snap: string, id: number) => {
  const ids = loadPassed(snap);
  if (!ids.includes(id)) save(snap, [...ids, id]);
};

export const clearPassed = (snap: string) => save(snap, []);

export const removePassed = (snap: string, id: number) =>
  save(
    snap,
    loadPassed(snap).filter((x) => x !== id),
  );
