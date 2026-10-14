import { listIndexState, listsState } from "@/atoms/scoutState";
import { clubState } from "@/atoms/squadState";
import Brief from "@/components/Brief";
import type { ScoutFilter } from "@/models/player";
import { useSearchResult } from "@/services/save";
import { nameQuery, scoutQuery, toPool } from "@/utils/player";
import { useEffect, useState } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { useRecoilState, useRecoilValue } from "recoil";
import "./scouting.css";

export default function Scouting() {
  const nav = useNavigate();
  const [lists, setLists] = useRecoilState(listsState);
  const [li, setLi] = useRecoilState(listIndexState);
  const club = useRecoilValue(clubState);
  const L = lists[li] ?? lists[0];
  const f = L.f;
  // the backend filters and ranks by potential
  const [text, setText] = useState("");
  // wait for a pause in typing before querying
  const [typed, setTyped] = useState("");
  useEffect(() => {
    const t = setTimeout(() => setTyped(text), 300);
    return () => clearTimeout(t);
  }, [text]);
  const searching = nameQuery(typed);
  const result = useSearchResult(
    club?.snapshotId,
    club ? scoutQuery(f, club.gameDate, club.id) : null,
  );
  const found = useSearchResult(club?.snapshotId, searching)?.players;
  const hits = club && found?.map((p) => toPool(p, club.gameDate));
  if (!club) return <Navigate to="/upload" replace />;
  const patch = (next: Partial<ScoutFilter>) =>
    setLists(
      lists.map((l, i) => (i === li ? { ...l, f: { ...l.f, ...next } } : l)),
    );
  const rename = (name: string) =>
    setLists(lists.map((l, i) => (i === li ? { ...l, name } : l)));
  return (
    <section className="wrap2 sc">
      <div>
        <div className="find">
          <input
            type="search"
            placeholder="Find a player by name"
            value={text}
            onChange={(e) => setText(e.target.value)}
          />
          {hits && (
            <div className="res">
              {hits.slice(0, 8).map((p) => (
                <div key={p.id} onClick={() => nav(`/player/${p.id}`)}>
                  <b>{p.n}</b>
                  <small>
                    {p.p} · {p.a} · {p.cl || "Free agent"}
                  </small>
                </div>
              ))}
              {!hits.length && (
                <div>
                  <small>No players match.</small>
                </div>
              )}
            </div>
          )}
        </div>
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
                f: { pos: [], min: 16, max: 40, con: 0, val: "" },
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
        <div className="sub">{L.fixed ? "Preset" : "Custom list"}</div>
        <Brief
          f={f}
          patch={patch}
          total={result?.total}
          onStart={() => nav("/scouting/swipe")}
        />
      </div>
    </section>
  );
}
