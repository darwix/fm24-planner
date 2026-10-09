import type { SquadPlayer } from "@/models/player";

interface Props {
  squad: SquadPlayer[];
  sel: number;
  onPick: (i: number) => void;
}

export default function Pitch({ squad, sel, onPick }: Props) {
  return (
    <div className="pitch">
      {squad.map(
        (s, i) =>
          s.xi && (
            <div
              key={i}
              className={`pl ${i === sel ? "sel" : ""} ${s.left !== undefined && s.left <= 12 ? "bad" : ""}`}
              style={{ left: `${s.x}%`, top: `${s.y}%` }}
              onClick={() => onPick(i)}
            >
              <i>{s.ca}</i>
              <small>{s.n.split(" ").pop()}</small>
              <em>{s.p}</em>
            </div>
          ),
      )}
    </div>
  );
}
