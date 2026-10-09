import type { MouseEvent } from "react";
import type { SquadPlayer } from "@/models/player";
import { TARGET, inGroup, need } from "@/utils/logic";

const GROUPS: [string, string][] = [
  ["Goalkeeper", "GK"],
  ["Right back", "DR"],
  ["Centre back", "DC"],
  ["Left back", "DL"],
  ["Defensive mid", "DM"],
  ["Attacking mid", "AMC"],
  ["Right wing", "AMR"],
  ["Left wing", "AML"],
  ["Striker", "ST"],
];

interface Props {
  squad: SquadPlayer[];
  sel: number;
  pos: string;
  onSelect: (i: number) => void;
  onDetails: (i: number) => void;
  onReplace: () => void;
  onRemove: () => void;
  onAddCover: (pos: string) => void;
  onClose: () => void;
}

export default function DepthPanel({
  squad,
  sel,
  pos,
  onSelect,
  onDetails,
  onReplace,
  onRemove,
  onAddCover,
  onClose,
}: Props) {
  const group = GROUPS.find(([, p]) => p === pos);
  if (!group) return null;
  const [title, p] = group;
  const rows = squad
    .map((s, i) => ({ s, i }))
    .filter(({ s }) => inGroup(s, p))
    .sort((a, b) => b.s.ca - a.s.ca);
  const missing = need(squad, p);
  const t = TARGET[p];
  const stop = (fn: () => void) => (e: MouseEvent) => {
    e.stopPropagation();
    fn();
  };
  return (
    <div>
      <div style={{ display: "flex", alignItems: "start" }}>
        <div>
          <h2>Position depth</h2>
          <div className="sub">Starter and cover</div>
        </div>
        <button
          className="btn"
          style={{ marginLeft: "auto" }}
          onClick={onClose}
        >
          Close
        </button>
      </div>
      <div className="bn">
        {title.toUpperCase()}{" "}
        <span className={`cv ${missing ? "lo" : ""}`}>
          {rows.length}/{t}
        </span>
      </div>
      <div className="list">
        {rows.map(({ s, i }) => (
          <div
            key={i}
            className={`row ${i === sel ? "sel" : ""}`}
            onClick={() => onSelect(i)}
          >
            <span className="p">{s.p === p ? s.p : `${s.p} → ${p}`}</span>
            <b style={{ cursor: "pointer" }} onClick={stop(() => onDetails(i))}>
              {s.n}
            </b>
            <span className="ca">{s.ca}</span>
            {i === sel && (
              <>
                <button className="btn" onClick={stop(() => onDetails(i))}>
                  Details
                </button>
                <button className="btn" onClick={stop(onReplace)}>
                  Find replacement
                </button>
                <button className="btn del" onClick={stop(onRemove)}>
                  Remove
                </button>
              </>
            )}
          </div>
        ))}
        {Array.from({ length: Math.max(1, missing) }, (_, k) => (
          <div key={`a${k}`} className="row ad" onClick={() => onAddCover(p)}>
            + Add cover · {p}
          </div>
        ))}
      </div>
    </div>
  );
}
