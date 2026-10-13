import "./shortlist.css";
import { useEffect, useState } from "react";
import { mutate } from "swr";
import { Navigate, useNavigate } from "react-router-dom";
import { useRecoilState, useRecoilValue } from "recoil";
import { clubState, squadState } from "@/atoms/squadState";
import type { PoolPlayer } from "@/models/player";
import { useSearchResult } from "@/services/save";
import { getShortlist, removeShortlist } from "@/services/shortlist";
import { money, wage } from "@/utils/format";
import { POS_INDEX, toPool } from "@/utils/player";
import {
  type ShortlistFilter,
  type SortKey,
  shortlistQuery,
  stars,
} from "@/utils/shortlist";

// label, key, first direction on click
const COLS: [string, SortKey, 1 | -1][] = [
  ["Player", "n", 1],
  ["Position", "p", 1],
  ["Ability", "ca", -1],
  ["Price", "v", 1],
  ["Contract", "left", 1],
  ["Wage", "w", 1],
];
const STATUS: [keyof Omit<ShortlistFilter, "pos">, string][] = [
  ["tl", "Transfer listed"],
  ["ll", "Loan listed"],
  ["exp", "Contract expiring"],
];

function Stars({ ca, pa }: { ca: number; pa: number }) {
  const s = stars(ca, pa);
  const star = (i: number) =>
    i <= s.ca ? (
      <b key={i}>★</b>
    ) : i - 0.5 === s.ca ? (
      <b key={i} className="half">
        ★
      </b>
    ) : i - 0.5 <= s.pa ? (
      <u key={i}>☆</u>
    ) : (
      <s key={i}>★</s>
    );
  return (
    <span className="st" title={`CA ${ca} · PA ${pa}`}>
      {[1, 2, 3, 4, 5].map(star)}
    </span>
  );
}

export default function Shortlist() {
  const nav = useNavigate();
  const club = useRecoilValue(clubState);
  const [squad, setSquad] = useRecoilState(squadState);
  const [f, setF] = useState<ShortlistFilter>({
    pos: [],
    tl: false,
    ll: false,
    exp: false,
  });
  const [sort, setSort] = useState<{ k: SortKey; dir: 1 | -1 }>({
    k: "v",
    dir: 1,
  });
  const snap = club?.snapshotId;
  // reading the list first moves an old snapshot's saved list into the table
  const [ready, setReady] = useState(false);
  useEffect(() => {
    if (snap) getShortlist(snap).then(() => setReady(true), console.error);
  }, [snap]);
  const base =
    ready && club?.id !== undefined
      ? { shortlist: 1, excludeClub: club.id, limit: 250 }
      : null;
  const all = useSearchResult(snap, base);
  // a picked position nobody left on the list can play has no chip to untick
  const everyone = (all?.players ?? []).map((p) => toPool(p, club!.gameDate));
  const positions = Object.keys(POS_INDEX).filter((x) =>
    everyone.some((p) => p.pos?.includes(x)),
  );
  const pos = f.pos.filter((x) => positions.includes(x));
  const found = useSearchResult(
    snap,
    base && shortlistQuery({ ...f, pos }, sort, club!.gameDate, club!.id!),
  );
  if (!club || club.id === undefined) return <Navigate to="/upload" replace />;

  const rows = (found?.players ?? []).map((p) => toPool(p, club.gameDate));
  const inSquad = new Set(squad.map((s) => s.id));
  const addToSquad = (p: PoolPlayer) =>
    setSquad([
      ...squad,
      {
        id: p.id,
        n: p.n,
        cl: p.cl,
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
  const drop = async (p: PoolPlayer) => {
    await removeShortlist(club.snapshotId, p.id);
    // refetch every shortlist search on this page
    mutate((k) => Array.isArray(k) && k[1]?.shortlist === 1);
  };
  const togglePos = (x: string) =>
    setF({
      ...f,
      pos: f.pos.includes(x) ? f.pos.filter((y) => y !== x) : [...f.pos, x],
    });

  return (
    <section className="slw">
      <div className="sum">
        <h2>Shortlist</h2>
        <span>
          {rows.length} of {everyone.length} signed ·{" "}
          {everyone.filter((p) => p.left !== undefined && p.left <= 12).length}{" "}
          expiring within 12 months
        </span>
      </div>
      <div className="bar2">
        <span className="lbl">Position</span>
        {positions.map((x) => (
          <span
            key={x}
            className={`chip ${f.pos.includes(x) ? "on" : ""}`}
            onClick={() => togglePos(x)}
          >
            {x}
          </span>
        ))}
      </div>
      <div className="bar2">
        <span className="lbl">Status</span>
        {STATUS.map(([k, l]) => (
          <span
            key={k}
            className={`chip ${f[k] ? "on" : ""}`}
            onClick={() => setF({ ...f, [k]: !f[k] })}
          >
            {l}
          </span>
        ))}
      </div>
      <div className="sheet">
        <div className="r h lbl">
          {COLS.map(([l, k, d]) => (
            <span
              key={k}
              className={sort.k === k ? "on" : ""}
              onClick={() =>
                setSort(
                  sort.k === k
                    ? { k, dir: sort.dir === 1 ? -1 : 1 }
                    : { k, dir: d },
                )
              }
            >
              {l}
              {sort.k === k && (sort.dir === 1 ? " ▲" : " ▼")}
            </span>
          ))}
          <span />
        </div>
        {rows.map((p) => (
          <div className="r" key={p.id}>
            <div className="nm" onClick={() => nav(`/player/${p.id}`)}>
              <b>{p.n}</b>
              <span>
                {p.a} · {p.cl || "Free agent"} · {p.nat}
                {p.tl && <span className="tag">Transfer listed</span>}
                {p.ll && <span className="tag">Loan listed</span>}
              </span>
            </div>
            <div>{p.p}</div>
            <div>
              <Stars ca={p.ca} pa={p.pa} />
            </div>
            <div>{money(p.v)}</div>
            <div className={p.left !== undefined && p.left <= 12 ? "warn" : ""}>
              {p.left === undefined ? "n/a" : `${p.left} mo`}
            </div>
            <div>{wage(p.w)}</div>
            <div className="acts">
              {inSquad.has(p.id) ? (
                <span className="done">In squad ✓</span>
              ) : (
                <button className="btn pri" onClick={() => addToSquad(p)}>
                  Add to squad
                </button>
              )}
              <button
                className="btn del"
                onClick={() => drop(p).catch(console.error)}
              >
                Drop
              </button>
            </div>
          </div>
        ))}
        {!rows.length && (
          <div className="empty">
            {!found
              ? "Loading…"
              : everyone.length
                ? "No players match these filters."
                : "Nothing on the shortlist. Swipe right on a card to add players."}
          </div>
        )}
      </div>
    </section>
  );
}
