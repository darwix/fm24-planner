export const money = (m: number) =>
  m < 1 ? `€${Math.round(m * 1000)}k` : `€${m}M`;
export const sz = (b: number) =>
  b >= 1e9 ? (b / 1e9).toFixed(2) + " GB" : (b / 1e6).toFixed(0) + " MB";
