import "@/pages/squad/squad.css";
import type { SquadPlayer } from "@/models/player";

interface Props {
  squad: SquadPlayer[];
  sel: number;
  vacant: { p: string; x: number; y: number }[];
  pos: string;
  onPick: (i: number) => void;
  onPickVacant: (p: string) => void;
  onDrop: (from: number, to: { xi: number } | { vacant: number }) => void;
}

export default function Pitch({
  squad,
  sel,
  vacant,
  pos,
  onPick,
  onPickVacant,
  onDrop,
}: Props) {
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
              draggable
              onDragStart={(e) => e.dataTransfer.setData("text", String(i))}
              onDragOver={(e) => e.preventDefault()}
              onDrop={(e) => {
                e.preventDefault();
                onDrop(Number(e.dataTransfer.getData("text")), { xi: i });
              }}
            >
              <i>{s.ca}</i>
              <small>{s.n.split(" ").pop()}</small>
              <em>{s.p}</em>
            </div>
          ),
      )}
      {vacant.map((v, k) => (
        <div
          key={`v${k}`}
          className={`pl ${v.p === pos && sel < 0 ? "sel" : ""}`}
          style={{ left: `${v.x}%`, top: `${v.y}%`, borderStyle: "dashed" }}
          onClick={() => onPickVacant(v.p)}
          onDragOver={(e) => e.preventDefault()}
          onDrop={(e) => {
            e.preventDefault();
            onDrop(Number(e.dataTransfer.getData("text")), { vacant: k });
          }}
        >
          <i>+</i>
          <small>Empty</small>
          <em>{v.p}</em>
        </div>
      ))}
    </div>
  );
}
