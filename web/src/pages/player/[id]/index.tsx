import "./player.css";
import { useNavigate, useParams } from "react-router-dom";
import { useRecoilState, useRecoilValue } from "recoil";
import { clubState, squadState } from "@/atoms/squadState";
import { useAttributeCatalog, useRealPlayer } from "@/services/save";
import { money, wage } from "@/utils/format";
import { contractLabel } from "@/utils/logic";
import { toPool } from "@/utils/player";

const tone = (v: number) => (v >= 14 ? "v-hi" : v >= 11 ? "v-mid" : "");
const SHOWN = ["Technical", "Mental", "Physical"];

export default function Player() {
  const nav = useNavigate();
  const id = useParams().id;
  const [squad, setSquad] = useRecoilState(squadState);
  const club = useRecoilValue(clubState);
  const real = useRealPlayer(club?.snapshotId, id);
  const catalog = useAttributeCatalog();
  const own = squad.find((s) => String(s.id) === id);

  if (!real.data)
    return (
      <section className="pd">
        {real.error ? "Player not found." : "Loading…"}
      </section>
    );

  const r = real.data;
  const q = toPool(r, club?.gameDate ?? "");
  const p = { ...q, p: own?.p ?? q.p }; // keep the squad slot position
  const groups = SHOWN.concat(p.p === "GK" ? ["Goalkeeping"] : []).map(
    (g): [string, [string, number][]] => [
      g,
      (catalog.data?.attributes ?? [])
        .filter((a) => a.group === g)
        .map((a): [string, number] => [a.label, r.attributes[a.key] ?? 0]),
    ],
  );

  return (
    <section className="pd">
      <a className="btn" style={{ margin: "0 -14px" }} onClick={() => nav(-1)}>
        {own ? "← Back to squad" : "← Back to scouting"}
      </a>
      <div className="top">
        <div>
          <div className="sub" style={{ margin: 0 }}>
            {p.cl}
            {p.nat ? ` · ${p.nat}` : ""}
          </div>
          <h2>{p.n}</h2>
          <div className="sub" style={{ margin: 0 }}>
            {p.p} · {p.a} years
          </div>
        </div>
        <div className="big">
          {p.pa}
          <small>POTENTIAL · CA {p.ca}</small>
        </div>
      </div>
      <div className="fx">
        <div>
          Contract
          <b
            style={
              p.left !== undefined && p.left <= 12
                ? { color: "var(--warn)" }
                : undefined
            }
          >
            {contractLabel(p.left, p.c)}
          </b>
        </div>
        <div>
          Value<b>{money(p.v)}</b>
        </div>
        <div>
          Wage<b>{wage(p.w)}</b>
        </div>
        <div>
          Age<b>{p.a}</b>
        </div>
        <div>
          Growth room<b>+{p.pa - p.ca} CA</b>
        </div>
      </div>
      <div className="ag">
        {groups.map(([g, list]) => (
          <div key={g}>
            <h3>{g.toUpperCase()}</h3>
            {list.map(([label, v]) => (
              <div className="ar" key={label}>
                <span>{label}</span>
                <b className={tone(v)}>{v || "–"}</b>
              </div>
            ))}
          </div>
        ))}
      </div>
      {!own && (
        <div style={{ marginTop: 28 }}>
          <button
            className="btn pri"
            onClick={() => {
              setSquad([
                ...squad,
                {
                  id: p.id,
                  n: p.n,
                  p: p.p,
                  a: p.a,
                  ca: p.ca,
                  pa: p.pa,
                  c: p.c,
                  left: p.left,
                  v: p.v,
                  w: p.w,
                  at: {},
                },
              ]);
              nav("/squad");
            }}
          >
            + Add to squad
          </button>
        </div>
      )}
    </section>
  );
}
