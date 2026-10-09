import "./shortlist.css";
import { useState } from "react";
import { Link, Navigate, useNavigate, useSearchParams } from "react-router-dom";
import { useRecoilState, useRecoilValue } from "recoil";
import {
  clubState,
  selectedState,
  squadState,
  vacantState,
} from "@/atoms/squadState";
import Delta from "@/components/Delta";
import { TAG, type Candidate, type Tag } from "@/models/player";
import { useSearch } from "@/services/save";
import { money, wage } from "@/utils/format";
import { asCandidate, contractLabel, inGroup } from "@/utils/logic";
import { POS_INDEX, toPool } from "@/utils/player";

export default function Shortlist() {
  const nav = useNavigate();
  const [params] = useSearchParams();
  const addPos = params.get("pos");
  const club = useRecoilValue(clubState);
  const [squad, setSquad] = useRecoilState(squadState);
  const [vacant, setVacant] = useRecoilState(vacantState);
  const sel = useRecoilValue(selectedState);
  const [on, setOn] = useState<Tag[]>(["exp", "cheap"]);
  const t = squad[sel] ?? squad[0];
  const pos = addPos ?? t?.p;
  // target CA: the player being replaced, or the squad average when adding cover
  const ref = addPos
    ? Math.round(squad.reduce((n, s) => n + s.ca, 0) / (squad.length || 1)) - 10
    : t?.ca;
  const found = useSearch(
    club?.snapshotId,
    pos && ref !== undefined
      ? {
          position: POS_INDEX[pos],
          caMin: Math.max(1, ref - 10),
          caMax: ref + 30,
          sort: "ca",
          limit: 250,
        }
      : null,
  );
  if (!squad.length || !club) return <Navigate to="/upload" replace />;

  const K = addPos ? [] : Object.keys(t.at);
  // cover at the same position already at the club, ready to step into the XI
  const own = squad
    .map((s, i) => ({ s, i }))
    .filter(({ s, i }) =>
      addPos
        ? // cover from another group who can also play here
          !s.xi && !inGroup(s, addPos) && (s.pos ?? [s.p]).includes(addPos)
        : i !== sel &&
          (s.pos ?? [s.p]).includes(t.p) &&
          // a cover can only be replaced by other cover
          (t.xi || (!s.xi && !inGroup(s, t.p))),
    )
    .map(({ s, i }) => ({
      i,
      c: {
        ...asCandidate({ ...s, id: s.id as number, cl: club.name }, ref),
        at: s.at,
      },
    }))
    .sort((a, b) => b.c.m - a.c.m);
  // squad players stay in the list unless "From your club" already shows them,
  // so one player can be cover for several groups
  const list: Candidate[] = (found ?? [])
    .filter((p) => p.id !== (addPos ? undefined : t.id))
    .filter((p) => !own.some(({ c }) => c.id === p.id))
    .map((p) => asCandidate({ ...toPool(p, club.gameDate), p: pos }, ref))
    .filter((c) => c.tags.some((g) => on.includes(g)))
    .sort((a, b) => b.m - a.m);
  const ownLabel = (i: number) =>
    addPos || !t.xi ? "Use as cover" : squad[i].xi ? "Swap spots" : "Promote";
  const promote = (i: number) => {
    const o = squad[i];
    if (addPos) {
      if (inGroup(o, addPos)) return nav("/squad");
      const v = vacant.find((x) => x.p === addPos);
      if (v) setVacant(vacant.filter((x) => x !== v));
      setSquad(
        squad.map((s, k) =>
          k !== i
            ? s
            : v
              ? { ...s, p: addPos, xi: 1 as const, x: v.x, y: v.y }
              : { ...s, cover: [...(s.cover ?? []), addPos] },
        ),
      );
      return nav("/squad");
    }
    if (!t.xi) {
      // cover for cover: he also covers this group, nobody leaves a group
      setSquad(
        squad.map((s, k) =>
          k === i ? { ...s, cover: [...(s.cover ?? []), t.p] } : s,
        ),
      );
      return nav("/squad");
    }
    setSquad(
      squad.map((s, k) => {
        if (k === i) return { ...s, xi: 1 as const, p: t.p, x: t.x, y: t.y };
        if (k !== sel) return s;
        // a starter takes the other's spot; cover just drops out of the XI
        if (o.xi) return { ...s, p: o.p, x: o.x, y: o.y };
        const { xi, x, y, ...rest } = s;
        return rest;
      }),
    );
    nav("/squad");
  };
  const toggle = (k: Tag) =>
    setOn(on.includes(k) ? on.filter((x) => x !== k) : [...on, k]);

  const fields = (c: Candidate) => ({
    id: c.id,
    n: c.n,
    a: c.a,
    ca: c.ca,
    pa: c.pa,
    c: c.c,
    left: c.left,
    v: c.v,
    w: c.w,
  });
  const add = (c: Candidate) => {
    const v = vacant.find((x) => x.p === addPos);
    if (v) setVacant(vacant.filter((x) => x !== v));
    setSquad([
      ...squad,
      {
        ...fields(c),
        p: addPos!,
        at: {},
        ...(v && { xi: 1 as const, x: v.x, y: v.y }),
      },
    ]);
    nav("/squad");
  };
  const swap = (c: Candidate) => {
    setSquad(
      squad.map((s, i) =>
        i === sel ? { ...s, ...fields(c), pos: undefined, at: c.at ?? {} } : s,
      ),
    );
    nav("/squad");
  };

  const card = (c: Candidate, label: string, go: () => void) => (
    <div className="cand" key={`${label}${c.id}`}>
      <div className="m">{c.m}</div>
      <div>
        <h3>
          <Link to={`/player/${c.id}`}>{c.n}</Link>{" "}
          <span style={{ color: "var(--mut)", fontWeight: 400 }}>· {c.cl}</span>
        </h3>
        <p>
          {c.p} · {c.a}y · CA {c.ca}/{c.pa} · {contractLabel(c.left, c.c)} ·{" "}
          {money(c.v)} · {wage(c.w)}
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
      <button className="btn pri" onClick={go}>
        {label}
      </button>
    </div>
  );

  return (
    <section className="wrap2">
      {addPos ? (
        <div className="tc">
          <div className="sub" style={{ margin: 0 }}>
            ADDING COVER
          </div>
          <h2>{addPos}</h2>
          <div className="sub">
            {squad.filter((s) => inGroup(s, addPos)).length} in squad
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
            CA · PA {t.pa} · {wage(t.w)}
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
        {own.length > 0 && (
          <>
            <h2>From your club</h2>
            <div className="sub">
              {own.length} {own.length === 1 ? "player" : "players"} who can
              play {pos}
            </div>
            {own.map(({ i, c }) => card(c, ownLabel(i), () => promote(i)))}
          </>
        )}
        <h2>Shortlist</h2>
        <div className="sub">
          {found
            ? `Ranked by CA match · ${list.length} ${list.length === 1 ? "player" : "players"}`
            : "Searching…"}
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
        {list.map((c) =>
          squad.some((s) => s.id === c.id)
            ? card(c, ownLabel(squad.findIndex((s) => s.id === c.id)), () =>
                promote(squad.findIndex((s) => s.id === c.id)),
              )
            : card(c, addPos ? "Add to squad" : "Swap in", () =>
                addPos ? add(c) : swap(c),
              ),
        )}
        {found && !list.length && (
          <div className="sub">No matches. Turn a filter back on.</div>
        )}
      </div>
    </section>
  );
}
