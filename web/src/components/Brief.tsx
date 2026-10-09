import type { ReactNode } from "react";
import type { ScoutFilter } from "@/models/player";

export const SPOT: Record<string, [number, number]> = {
  ST: [50, 14],
  AML: [18, 30],
  AMC: [50, 30],
  AMR: [82, 30],
  DM: [50, 52],
  DL: [14, 72],
  DC: [50, 72],
  DR: [86, 72],
  GK: [50, 90],
};
const POSITIONS = Object.keys(SPOT);
const PRICES = [0, 1, 5, 10, 20];
const CONTRACTS: [number, string][] = [
  [0, "Any"],
  [-1, "Expired (free agents)"],
  [12, "Expiring ≤ 12 months"],
  [6, "≤ 6 months"],
];

const Chip = ({
  on,
  onClick,
  children,
}: {
  on: boolean;
  onClick: () => void;
  children: ReactNode;
}) => (
  <button className={`chip ${on ? "on" : ""}`} onClick={onClick}>
    {children}
  </button>
);

const Q = ({
  title,
  hint,
  last,
  children,
}: {
  title: string;
  hint: string;
  last?: boolean;
  children: ReactNode;
}) => (
  <div className="q" style={last ? { border: 0 } : undefined}>
    <span>
      <b>{title}</b>
      <small>{hint}</small>
    </span>
    {children}
  </div>
);

// the filter step of scouting: a report-card style brief
export default function Brief({
  f,
  patch,
  total,
  list,
  onList,
  onStart,
}: {
  f: ScoutFilter;
  patch: (next: Partial<ScoutFilter>) => void;
  total?: number;
  list: boolean;
  onList: () => void;
  onStart: () => void;
}) {
  const setMin = (v: number) => patch({ min: Math.min(v, f.max) });
  const setMax = (v: number) => patch({ max: Math.max(v, f.min) });
  return (
    <div className="brief">
      <Q title="Position" hint="pick one or more">
        <div className="pw">
          <div className="pitch">
            {Object.entries(SPOT).map(([p, [x, y]]) => (
              <button
                key={p}
                className={`sp ${f.pos.includes(p) ? "on" : ""}`}
                style={{ left: `${x}%`, top: `${y}%` }}
                onClick={() =>
                  patch({
                    pos: f.pos.includes(p)
                      ? f.pos.filter((q) => q !== p)
                      : [...f.pos, p],
                  })
                }
              >
                {p}
              </button>
            ))}
          </div>
          <div className="hint">
            <span>Click spots on the pitch. Pick as many as you like.</span>
            <div className="chips">
              <Chip on={!f.pos.length} onClick={() => patch({ pos: [] })}>
                Any position
              </Chip>
              <Chip
                on={f.pos.length === POSITIONS.length}
                onClick={() => patch({ pos: POSITIONS })}
              >
                Select all
              </Chip>
            </div>
            <b>{f.pos.length ? f.pos.join(", ") : "Any"}</b>
          </div>
        </div>
      </Q>
      <Q title="Age" hint="years">
        <div className="age">
          <input
            type="number"
            value={f.min}
            onChange={(e) => setMin(+e.target.value)}
          />
          <input
            type="range"
            min={15}
            max={40}
            value={f.min}
            onChange={(e) => setMin(+e.target.value)}
          />
          <input
            type="range"
            min={15}
            max={40}
            value={f.max}
            onChange={(e) => setMax(+e.target.value)}
          />
          <input
            type="number"
            value={f.max}
            onChange={(e) => setMax(+e.target.value)}
          />
        </div>
      </Q>
      <Q title="Max price" hint="market value">
        <div className="chips">
          {PRICES.map((v) => (
            <Chip
              key={v}
              on={f.val === (v || "")}
              onClick={() => patch({ val: v || "" })}
            >
              {v ? `£${v}M` : "Any"}
            </Chip>
          ))}
        </div>
      </Q>
      <Q title="Contract" hint="time left" last>
        <div className="chips">
          {CONTRACTS.map(([v, label]) => (
            <Chip key={v} on={f.con === v} onClick={() => patch({ con: v })}>
              {label}
            </Chip>
          ))}
        </div>
      </Q>
      <div className="foot">
        <div className="count">
          {total === undefined ? "…" : total.toLocaleString()}
        </div>
        <span className="sub">
          {total !== undefined && total > 250
            ? "players match. The 250 with the highest potential are listed."
            : "players match."}
        </span>
        <div className="grow">
          <button className="btn" onClick={onList}>
            {list ? "Hide list" : "Browse as list"}
          </button>
          <button
            className="btn pri"
            disabled={!total}
            onClick={onStart}
            style={{ marginLeft: 8 }}
          >
            Start scouting →
          </button>
        </div>
      </div>
    </div>
  );
}
