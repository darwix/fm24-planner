import { useNavigate, useParams } from "react-router-dom";
import { useRecoilState, useRecoilValue } from "recoil";
import { clubState, squadState } from "@/atoms/squadState";
import { useAttributeCatalog, usePool, useRealPlayer } from "@/services/save";
import { money } from "@/utils/format";
import { GROUPS, fakeAttr } from "@/utils/placeholder";

const tone = (v: number) => (v >= 14 ? "v-hi" : v >= 11 ? "v-mid" : "");
const SHOWN = ["Technical", "Mental", "Physical"];

export default function Player() {
  const nav = useNavigate();
  const name = decodeURIComponent(useParams().name ?? "");
  const [squad, setSquad] = useRecoilState(squadState);
  const club = useRecoilValue(clubState);
  const pool = usePool();
  const own = squad.find((s) => s.n === name);
  const scouted = pool?.find((x) => x.n === name);
  const real = useRealPlayer(club?.snapshotId, own?.id);
  const catalog = useAttributeCatalog();

  if (!own && !scouted)
    return (
      <section className="pd">
        {pool ? "Player not found." : "Loading…"}
      </section>
    );

  const back = own ? "← Back to squad" : "← Back to scouting";
  const p = own
    ? {
        cl: real.data?.club ?? club?.name ?? "",
        nat: real.data?.nationalities?.[0] ?? "",
        p: own.p,
        a: own.a,
        ca: own.ca,
        pa: own.pa,
        left: undefined as number | undefined,
        v: own.v,
        w: own.w,
      }
    : { ...scouted!, left: scouted!.left as number | undefined };

  // own players use parsed attributes; scouted ones are placeholders
  const groups: [string, [string, number][]][] = own
    ? SHOWN.concat(own.p === "GK" ? ["Goalkeeping"] : []).map((g) => [
        g,
        (catalog.data?.attributes ?? [])
          .filter((a) => a.group === g)
          .map((a): [string, number] => [
            a.label,
            real.data?.attributes[a.key] ?? 0,
          ]),
      ])
    : Object.entries(GROUPS).map(([g, l]) => [
        g,
        l.map((a): [string, number] => [a, fakeAttr(scouted!, a)]),
      ]);

  return (
    <section className="pd">
      <a className="btn" style={{ margin: "0 -14px" }} onClick={() => nav(-1)}>
        {back}
      </a>
      <div className="top">
        <div>
          <div className="sub" style={{ margin: 0 }}>
            {p.cl}
            {p.nat ? ` · ${p.nat}` : ""}
          </div>
          <h2>{name}</h2>
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
            {p.left !== undefined ? `${p.left} months left` : "n/a"}
          </b>
        </div>
        <div>
          Value<b>{p.v !== undefined ? money(p.v) : "n/a"}</b>
        </div>
        <div>
          Wage<b>{p.w !== undefined ? `€${p.w}k/wk` : "n/a"}</b>
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
                  n: scouted!.n,
                  p: scouted!.p,
                  a: scouted!.a,
                  ca: scouted!.ca,
                  pa: scouted!.pa,
                  c: 2052 + Math.ceil(scouted!.left / 12),
                  v: scouted!.v,
                  w: scouted!.w,
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
