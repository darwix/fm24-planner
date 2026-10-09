// money is in the save's base currency (£): value in millions, wage in thousands a week
export const money = (m?: number) =>
  m === undefined ? "n/a" : m < 1 ? `£${Math.round(m * 1000)}k` : `£${m}M`;
export const wage = (w?: number) => (w === undefined ? "n/a" : `£${w}k/wk`);
export const sz = (b: number) =>
  b >= 1e9 ? (b / 1e9).toFixed(2) + " GB" : (b / 1e6).toFixed(0) + " MB";
