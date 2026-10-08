import { useNavigate } from "react-router-dom";
import { useRecoilState } from "recoil";
import { listIndexState, listsState } from "@/atoms/scoutState";
import { squadState } from "@/atoms/squadState";
import type { PoolPlayer, ScoutFilter } from "@/models/player";
import { usePool } from "@/services/save";
import { money } from "@/utils/format";
import { matchFilter } from "@/utils/logic";

export default function Scouting() {
  const nav = useNavigate();
  const [lists, setLists] = useRecoilState(listsState);
  const [li, setLi] = useRecoilState(listIndexState);
  const [squad, setSquad] = useRecoilState(squadState);
  const pool = usePool() ?? [];
  const L = lists[li] ?? lists[0];
  const f = L.f;
  const positions = [...new Set(pool.map((p) => p.p))];
  const rows = pool
    .filter((p) => matchFilter(p, f))
    .sort((a, b) => b.pa - a.pa);
  const patch = (next: Partial<ScoutFilter>) =>
    setLists(
      lists.map((l, i) => (i === li ? { ...l, f: { ...l.f, ...next } } : l)),
    );
  const rename = (name: string) =>
    setLists(lists.map((l, i) => (i === li ? { ...l, name } : l)));
  const addToSquad = (p: PoolPlayer) => {
    setSquad([
      ...squad,
      {
        n: p.n,
        p: p.p,
        a: p.a,
        ca: p.ca,
        pa: p.pa,
        c: 2052 + Math.ceil(p.left / 12),
        v: p.v,
        w: p.w,
        at: {},
      },
    ]);
    nav("/squad");
  };

  return (
    <section className="wrap2 sc">
      <div>
        <h2>Lists</h2>
        <div className="sub">Saved scouting searches</div>
        <div className="ls">
          {lists.map((l, i) => (
            <div
              key={i}
              className={`li ${i === li ? "sel" : ""}`}
              onClick={() => setLi(i)}
            >
              {l.name}
              <small>{pool.filter((p) => matchFilter(p, l.f)).length}</small>
              {!l.fixed && (
                <span
                  className="x"
                  onClick={(e) => {
                    e.stopPropagation();
                    setLists(lists.filter((_, j) => j !== i));
                    setLi(0);
                  }}
                >
                  ✕
                </span>
              )}
            </div>
          ))}
        </div>
        <button
          className="btn"
          style={{ marginTop: 10 }}
          onClick={() => {
            setLists([
              ...lists,
              {
                name: "New list",
                f: { pos: "", min: 16, max: 40, exp: false, val: "" },
              },
            ]);
            setLi(lists.length);
          }}
        >
          + New list
        </button>
      </div>
      <div>
        <h2>
          {L.fixed ? (
            L.name
          ) : (
            <input
              type="text"
              value={L.name}
              onChange={(e) => rename(e.target.value)}
              style={{
                font: "700 20px Fraunces,serif",
                border: 0,
                background: "none",
                width: "100%",
              }}
            />
          )}
        </h2>
        <div className="sub">
          {rows.length} players · ranked by potential
          {L.fixed ? " · preset" : ""}
        </div>
        <div className="cr">
          <label>
            Position
            <select
              value={f.pos}
              onChange={(e) => patch({ pos: e.target.value })}
            >
              <option value="">Any</option>
              {positions.map((x) => (
                <option key={x}>{x}</option>
              ))}
            </select>
          </label>
          <label>
            Age
            <span>
              <input
                type="number"
                value={f.min}
                onChange={(e) => patch({ min: +e.target.value })}
              />{" "}
              –{" "}
              <input
                type="number"
                value={f.max}
                onChange={(e) => patch({ max: +e.target.value })}
              />
            </span>
          </label>
          <label>
            Max value (€M)
            <input
              type="number"
              value={f.val}
              placeholder="any"
              onChange={(e) =>
                patch({ val: e.target.value === "" ? "" : +e.target.value })
              }
            />
          </label>
          <label className="ck">
            <input
              type="checkbox"
              checked={f.exp}
              onChange={(e) => patch({ exp: e.target.checked })}
            />{" "}
            Contract expiring (≤12m)
          </label>
        </div>
        <table className="tb">
          <thead>
            <tr>
              <th>PA</th>
              <th>Name</th>
              <th>Pos</th>
              <th>Age</th>
              <th>From</th>
              <th>CA</th>
              <th>Contract</th>
              <th className="r">Value</th>
              <th className="r">Wage</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {rows.map((p) => (
              <tr
                key={p.n}
                className="cl"
                onClick={() => nav(`/player/${encodeURIComponent(p.n)}`)}
              >
                <td className="pa">{p.pa}</td>
                <td>
                  <b>{p.n}</b>
                  <br />
                  <span style={{ color: "var(--mut)", fontSize: 12 }}>
                    {p.cl}
                  </span>
                </td>
                <td>{p.p}</td>
                <td>{p.a}</td>
                <td>{p.nat}</td>
                <td>{p.ca}</td>
                <td
                  style={
                    p.left <= 12
                      ? { color: "var(--warn)", fontWeight: 600 }
                      : undefined
                  }
                >
                  {p.left} months
                </td>
                <td className="r">{money(p.v)}</td>
                <td className="r">€{p.w}k</td>
                <td className="r">
                  <button
                    className="btn"
                    onClick={(e) => {
                      e.stopPropagation();
                      addToSquad(p);
                    }}
                  >
                    + Add
                  </button>
                </td>
              </tr>
            ))}
            {!rows.length && (
              <tr>
                <td colSpan={10} className="sub" style={{ padding: 24 }}>
                  No players match.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
