import { useState } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { useRecoilState } from "recoil";
import { selectedState, squadState, vacantState } from "@/atoms/squadState";
import { inGroup } from "@/utils/logic";
import DepthPanel from "@/components/DepthPanel";
import Pitch from "@/components/Pitch";

export default function Squad() {
  const nav = useNavigate();
  const [squad, setSquad] = useRecoilState(squadState);
  const [sel, setSel] = useRecoilState(selectedState);
  const [vacant, setVacant] = useRecoilState(vacantState);
  const [open, setOpen] = useState(false);
  const [pos, setPos] = useState(squad[sel]?.p ?? "");
  if (!squad.length) return <Navigate to="/upload" replace />;
  return (
    <div className={`wrap ${open ? "" : "solo"}`}>
      <div>
        <Pitch
          squad={squad}
          sel={sel}
          vacant={vacant}
          pos={pos}
          onPick={(i) => {
            setSel(i);
            setPos(squad[i].p);
            setOpen(true);
          }}
          onDrop={(from, to) => {
            const a = squad[from];
            if (!a?.xi) return;
            if ("xi" in to) {
              const b = squad[to.xi];
              if (from === to.xi) return;
              setSquad(
                squad.map((s, i) =>
                  i === from
                    ? { ...s, p: b.p, x: b.x, y: b.y }
                    : i === to.xi
                      ? { ...s, p: a.p, x: a.x, y: a.y }
                      : s,
                ),
              );
            } else {
              const v = vacant[to.vacant];
              setSquad(
                squad.map((s, i) =>
                  i === from ? { ...s, p: v.p, x: v.x, y: v.y } : s,
                ),
              );
              setVacant(
                vacant
                  .filter((_, k) => k !== to.vacant)
                  .concat({ p: a.p, x: a.x!, y: a.y! }),
              );
            }
            setSel(from);
            setPos("xi" in to ? squad[to.xi].p : vacant[to.vacant].p);
          }}
          onPickVacant={(p) => {
            setSel(-1);
            setPos(p);
            setOpen(true);
          }}
        />
      </div>
      {open && (
        <DepthPanel
          squad={squad}
          sel={sel}
          pos={pos}
          onSelect={setSel}
          onDetails={(i) => nav(`/player/${squad[i].id}`)}
          onReplace={() => nav("/shortlist")}
          onRemove={() => {
            const r = squad[sel];
            const rest = squad.filter((_, i) => i !== sel);
            if (r.xi && r.x !== undefined && r.y !== undefined) {
              const best = rest.reduce(
                (b, s, i) =>
                  !s.xi && inGroup(s, r.p) && (b < 0 || s.ca > rest[b].ca)
                    ? i
                    : b,
                -1,
              );
              if (best < 0) setVacant([...vacant, { p: r.p, x: r.x, y: r.y }]);
              else
                rest[best] = { ...rest[best], p: r.p, xi: 1, x: r.x, y: r.y };
            }
            setSquad(rest);
            setSel(-1);
          }}
          onAddCover={(p) => nav(`/shortlist?pos=${p}`)}
          onClose={() => setOpen(false)}
        />
      )}
    </div>
  );
}
