import { useState } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { useRecoilState } from "recoil";
import { selectedState, squadState } from "@/atoms/squadState";
import DepthPanel from "@/components/DepthPanel";
import Pitch from "@/components/Pitch";

export default function Squad() {
  const nav = useNavigate();
  const [squad, setSquad] = useRecoilState(squadState);
  const [sel, setSel] = useRecoilState(selectedState);
  const [open, setOpen] = useState(false);
  if (!squad.length) return <Navigate to="/upload" replace />;
  return (
    <div className={`wrap ${open ? "" : "solo"}`}>
      <div>
        <Pitch
          squad={squad}
          sel={sel}
          onPick={(i) => {
            setSel(i);
            setOpen(true);
          }}
        />
      </div>
      {open && (
        <DepthPanel
          squad={squad}
          sel={sel}
          onSelect={setSel}
          onDetails={(i) => nav(`/player/${encodeURIComponent(squad[i].n)}`)}
          onReplace={() => nav("/shortlist")}
          onRemove={() => {
            setSquad(squad.filter((_, i) => i !== sel));
            setSel(0);
          }}
          onAddCover={(p) => nav(`/shortlist?pos=${p}`)}
          onClose={() => setOpen(false)}
        />
      )}
    </div>
  );
}
