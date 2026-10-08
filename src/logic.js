export const TARGET = {
  GK: 2,
  DR: 2,
  DC: 4,
  DL: 2,
  DM: 2,
  AM: 2,
  AMR: 2,
  AML: 2,
  ST: 2,
};
export const need = (squad, pos) =>
  Math.max(0, TARGET[pos] - squad.filter((s) => s.p === pos).length);
export const matchFilter = (p, f) =>
  (!f.pos || p.p === f.pos) &&
  p.a >= f.min &&
  p.a <= f.max &&
  (!f.exp || p.left <= 12) &&
  (!f.val || p.v <= f.val);
// ponytail: placeholder match formula, replace once real profile matching exists
export const asCandidate = (p) => ({
  ...p,
  m: Math.min(99, Math.round((p.ca * 0.6 + p.pa * 0.4) / 1.6)),
  tags: [...(p.left <= 12 ? ["exp"] : []), ...(p.v <= 5 ? ["cheap"] : [])],
});
export const contractLabel = (left, end) =>
  left <= 12 ? `${left} months left` : `to ${end}`;
export const isFmFile = (name) => /\.fm$/i.test(name);
