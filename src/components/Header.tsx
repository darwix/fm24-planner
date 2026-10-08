import { NavLink, useLocation, useNavigate } from "react-router-dom";
import { useRecoilValue } from "recoil";
import { clubState, squadState } from "@/atoms/squadState";
import { useCands } from "@/services/save";

const tab = ({ isActive }: { isActive: boolean }) => (isActive ? "on" : "");

export default function Header() {
  const club = useRecoilValue(clubState);
  const squad = useRecoilValue(squadState);
  const cands = useCands();
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
          Shortlist <em>{cands?.length ?? 0}</em>
        </NavLink>
        <NavLink
          to="/scouting"
          className={({ isActive }) =>
            isActive ||
            (pathname.startsWith("/player/") &&
              !squad.some((s) => s.n === decodeURIComponent(pathname.slice(8))))
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
