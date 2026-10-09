import { useEffect } from "react";
import { NavLink, useLocation, useNavigate } from "react-router-dom";
import { useRecoilValue } from "recoil";
import { clubState, squadState, vacantState } from "@/atoms/squadState";
import { saveArr, saveSession, toArrangement } from "@/utils/store";

const tab = ({ isActive }: { isActive: boolean }) => (isActive ? "on" : "");

export default function Header() {
  const club = useRecoilValue(clubState);
  const squad = useRecoilValue(squadState);
  const vacant = useRecoilValue(vacantState);
  // keep the arrangement so a reload or a later "Open squad" restores it
  useEffect(() => {
    if (!club || !squad.length) return;
    const arr = toArrangement(squad, vacant);
    saveSession({ club, arr });
    saveArr(club, arr);
  }, [club, squad, vacant]);
  const nav = useNavigate();
  const { pathname } = useLocation();
  return (
    <header>
      <h1>{club?.name ?? "Squad planner"}</h1>
      <span>FM24 save</span>
      <nav>
        <NavLink to="/squad" className={tab}>
          Squad
        </NavLink>
        <NavLink to="/shortlist" className={tab}>
          Shortlist
        </NavLink>
        <NavLink
          to="/scouting"
          className={({ isActive }) =>
            isActive ||
            (pathname.startsWith("/player/") &&
              !squad.some((s) => String(s.id) === pathname.slice(8)))
              ? "on"
              : ""
          }
        >
          Scouting
        </NavLink>
      </nav>
      <button
        className="btn"
        style={{ marginLeft: "auto" }}
        onClick={() => nav("/upload")}
      >
        Load save
      </button>
    </header>
  );
}
