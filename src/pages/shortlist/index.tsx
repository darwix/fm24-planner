import { useState } from "react";
import { Link, Navigate, useNavigate, useSearchParams } from "react-router-dom";
import { useRecoilState, useRecoilValue } from "recoil";
import { selectedState, squadState } from "@/atoms/squadState";
import Delta from "@/components/Delta";
import { TAG, type Candidate, type Tag } from "@/models/player";
import { useCands, usePool } from "@/services/save";
import { money } from "@/utils/format";
import { asCandidate, contractLabel } from "@/utils/logic";

export default function Shortlist() {
  const nav = useNavigate();
  const [params] = useSearchParams();
  const addPos = params.get("pos");
  const [squad, setSquad] = useRecoilState(squadState);
  const sel = useRecoilValue(selectedState);
  const [on, setOn] = useState<Tag[]>(["exp", "list", "cheap"]);
  const cands = useCands();
  const pool = usePool();
  if (!squad.length) return <Navigate to="/upload" replace />;

  const t = squad[sel] ?? squad[0];
  const K = addPos ? [] : Object.keys(t.at);
  const src: Candidate[] = addPos
    ? addPos === "DC"
      ? (cands ?? [])
      : (pool ?? []).filter((p) => p.p === addPos).map(asCandidate)
    : (cands ?? []);
  const list = src.filter((c) => c.tags.some((g) => on.includes(g)));
  const toggle = (k: Tag) =>
    setOn(on.includes(k) ? on.filter((x) => x !== k) : [...on, k]);
  const end = (c: Candidate) => c.c ?? 2052 + Math.ceil(c.left / 12);

  const add = (c: Candidate) => {
    setSquad([
      ...squad,
      {
        n: c.n,
        p: addPos!,
        a: c.a,
        ca: c.ca,
        pa: c.pa,
        c: end(c),
        v: c.v,
        w: c.w,
        at: {},
      },
    ]);
    nav("/squad");
  };
  const swap = (c: Candidate) => {
    setSquad(
      squad.map((s, i) =>
        i === sel
          ? {
              ...s,
              n: c.n,
              a: c.a,
              ca: c.ca,
              pa: c.pa,
              c: end(c),
              v: c.v,
              w: c.w,
              at: c.at ?? {},
            }
          : s,
      ),
    );
    nav("/squad");
  };

  return (
    <section className="wrap2">
      {addPos ? (
        <div className="tc">
          <div className="sub" style={{ margin: 0 }}>
            ADDING COVER
          </div>
          <h2>{addPos}</h2>
          <div className="sub">
            {squad.filter((s) => s.p === addPos).length} in squad
          </div>
          <Link className="btn" to="/squad">
            ← Back to squad
          </Link>
        </div>
      ) : (
        <div className="tc">
          <div className="sub" style={{ margin: 0 }}>
            REPLACING
          </div>
          <h2>{t.n}</h2>
          <div className="sub" style={{ margin: 0 }}>
            {t.p} · {t.a}y{t.c ? ` · contract ${t.c}` : ""}
          </div>
          <div className="big" style={{ marginTop: 14 }}>
            {t.ca}
          </div>
          <div className="sub">
            CA · PA {t.pa}
            {t.w ? ` · €${t.w}k/wk` : ""}
          </div>
          <div className="at">
            {K.map((k) => (
              <div key={k}>
                {k}
                <b>{t.at[k]}</b>
              </div>
            ))}
          </div>
          <Link className="btn" to="/squad">
            ← Back to squad
          </Link>
        </div>
      )}
      <div>
        <h2>Shortlist</h2>
        <div className="sub">
          Ranked by profile match · {list.length} players
        </div>
        <div className="chips">
          {(Object.keys(TAG) as Tag[]).map((k) => (
            <span
              key={k}
              className={`chip ${on.includes(k) ? "on" : ""}`}
              onClick={() => toggle(k)}
            >
              {TAG[k]}
            </span>
          ))}
        </div>
        {list.map((c) => (
          <div className="cand" key={c.n}>
            <div className="m">{c.m}</div>
            <div>
              <h3>
                {c.n}{" "}
                <span style={{ color: "var(--mut)", fontWeight: 400 }}>
                  · {c.cl}
                </span>
              </h3>
              <p>
                {c.p} · {c.a}y · CA {c.ca}/{c.pa} ·{" "}
                {contractLabel(c.left, end(c))} · {money(c.v)} · €{c.w}k/wk
              </p>
              {c.tags.map((g) => (
                <span key={g} className={`tg ${g}`}>
                  {TAG[g]}
                </span>
              ))}
              <div className="dl">
                {K.map((k) => (
                  <span key={k}>
                    {k}
                    <b>{c.at?.[k]}</b>
                    <Delta a={c.at?.[k] ?? 0} b={t.at[k]} />
                  </span>
                ))}
              </div>
            </div>
            <button
              className="btn pri"
              onClick={() => (addPos ? add(c) : swap(c))}
            >
              {addPos ? "Add to squad" : "Swap in"}
            </button>
          </div>
        ))}
        {!list.length && (
          <div className="sub">No matches. Turn a filter back on.</div>
        )}
      </div>
    </section>
  );
}
