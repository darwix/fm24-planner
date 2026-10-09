import "./swipe.css";
import { useCallback, useEffect, useRef, useState } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { useRecoilState, useRecoilValue } from "recoil";
import { keptState, listIndexState, listsState } from "@/atoms/scoutState";
import { clubState } from "@/atoms/squadState";
import type { PoolPlayer } from "@/models/player";
import {
  searchPage,
  useAttributeCatalog,
  useRealPlayer,
} from "@/services/save";
import { money, wage } from "@/utils/format";
import {
  addPassed,
  clearPassed,
  loadPassed,
  removePassed,
} from "@/utils/passed";
import { scoutQuery, swipeBody, toPool } from "@/utils/player";

const THRESH = 110;
const SHOWN = ["Technical", "Mental", "Physical"];
const tone = (v: number) => (v >= 14 ? "v-hi" : v >= 11 ? "v-mid" : "");

function Face({ p, drag = 0 }: { p: PoolPlayer; drag?: number }) {
  return (
    <>
      <div
        className="stamp sign"
        style={{ opacity: Math.max(0, drag / THRESH) }}
      >
        SIGN
      </div>
      <div
        className="stamp pass"
        style={{ opacity: Math.max(0, -drag / THRESH) }}
      >
        PASS
      </div>
      <h2>
        {p.n}
        {p.left !== undefined && p.left <= 12 && (
          <span className="exp">EXPIRING</span>
        )}
      </h2>
      <div className="meta">
        {p.p} · {p.a} · {p.cl || "No club"}
      </div>
      <div className="tags">
        {p.nat && <span>{p.nat}</span>}
        <span>CA {p.ca}</span>
        <span>PA {p.pa}</span>
      </div>
      <div className="marks">
        <div className="m">
          <span className="lbl">Potential</span>
          <b>{p.pa}</b>
          <div className="bar">
            <i style={{ width: `${(p.pa / 200) * 100}%` }} />
          </div>
        </div>
        <div className="m">
          <span className="lbl">Current</span>
          <b>{p.ca}</b>
          <div className="bar">
            <i style={{ width: `${(p.ca / 200) * 100}%` }} />
          </div>
        </div>
        <div className="m">
          <span className="lbl">Growth room</span>
          <b>+{p.pa - p.ca}</b>
        </div>
        <div className="m">
          <span className="lbl">Age</span>
          <b>{p.a}</b>
        </div>
      </div>
      <div className="sec fin">
        <div>
          <span className="lbl">Wage</span>
          <b>{wage(p.w)}</b>
        </div>
        <div>
          <span className="lbl">Contract</span>
          <b>{p.left === undefined ? "n/a" : `${p.left} months`}</b>
        </div>
        <div>
          <span className="lbl">Value</span>
          <b>{money(p.v)}</b>
        </div>
      </div>
    </>
  );
}

function Detail({
  snap,
  p,
  onClose,
}: {
  snap: string;
  p: PoolPlayer;
  onClose: () => void;
}) {
  const real = useRealPlayer(snap, p.id);
  const catalog = useAttributeCatalog();
  const attrs = real.data?.attributes;
  return (
    <>
      <div className="ribbon" />
      <button className="close" onClick={onClose}>
        Close ✕
      </button>
      <div className="lbl">Attributes</div>
      <h3>{p.n}</h3>
      <div className="sub">
        {p.p} · age {p.a}
      </div>
      {!attrs && <div className="sub">Loading…</div>}
      {attrs &&
        SHOWN.map((g) => (
          <div key={g} className="sec">
            <span className="lbl">{g}</span>
            <div className="ag1">
              {(catalog.data?.attributes ?? [])
                .filter((a) => a.group === g)
                .map((a) => (
                  <div key={a.key}>
                    <span>{a.label}</span>
                    <b className={tone(attrs[a.key] ?? 0)}>
                      {attrs[a.key] ?? 0}
                    </b>
                  </div>
                ))}
            </div>
          </div>
        ))}
    </>
  );
}

export default function Swipe() {
  const nav = useNavigate();
  const club = useRecoilValue(clubState);
  const lists = useRecoilValue(listsState);
  const li = useRecoilValue(listIndexState);
  const [kept, setKept] = useRecoilState(keptState);
  const [stack, setStack] = useState<PoolPlayer[]>([]);
  const [total, setTotal] = useState<number>();
  const [loaded, setLoaded] = useState(false);
  const [i, setI] = useState(0);
  const [hist, setHist] = useState<{ p: PoolPlayer; dir: number }[]>([]);
  const [open, setOpen] = useState(false);
  const [closing, setClosing] = useState(false);
  const [flip, setFlip] = useState(false);
  const [out, setOut] = useState(0);
  const [dx, setDx] = useState<number | null>(null);
  const x0 = useRef(0);
  const page = useRef(0);
  const busy = useRef(false);
  const keptIds = useRef<number[]>([]);
  keptIds.current = kept.map((k) => k.id);
  const snap = club?.snapshotId;
  const f = (lists[li] ?? lists[0]).f;

  const more = useCallback(async () => {
    if (!club || busy.current) return;
    busy.current = true;
    try {
      const passed = [...loadPassed(club.snapshotId), ...keptIds.current];
      const r = await searchPage(
        club.snapshotId,
        swipeBody(scoutQuery(f, club.gameDate), passed, page.current + 1),
      );
      page.current += 1;
      setTotal(r.total);
      setStack((s) => {
        const have = new Set(s.map((q) => q.id));
        return [
          ...s,
          ...r.players
            .filter((q) => !have.has(q.id))
            .map((q) => toPool(q, club.gameDate)),
        ];
      });
    } finally {
      busy.current = false;
      setLoaded(true);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [club?.snapshotId]);

  useEffect(() => {
    more();
  }, [more]);
  // top up before the stack runs dry
  useEffect(() => {
    if (loaded && total !== undefined && stack.length - i <= 5) {
      const loadedCount = page.current * 50;
      if (loadedCount < total) more();
    }
  }, [i, stack.length, total, loaded, more]);

  const keys = useRef<Record<string, () => void>>({});
  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      keys.current[e.key]?.();
    };
    window.addEventListener("keydown", on);
    return () => window.removeEventListener("keydown", on);
  }, []);

  if (!club || !snap) return <Navigate to="/upload" replace />;
  const top = stack[i];
  const rest = stack.slice(i, i + 3);
  const left = total === undefined ? undefined : total - i;

  const commit = (dir: number) => {
    if (!top) return;
    setHist((h) => [...h, { p: top, dir }]);
    if (dir > 0) setKept((k) => [...k, top]);
    else addPassed(snap, top.id);
    setI(i + 1);
    setOut(0);
    setDx(null);
    setFlip(false);
  };
  const fly = (dir: number) => {
    if (!top || out || flip) return;
    if (open) {
      setFlip(true);
      setTimeout(() => commit(dir), 380);
    } else {
      setOut(dir);
      setTimeout(() => commit(dir), 220);
    }
  };
  const undo = () => {
    const h = hist[hist.length - 1];
    if (!h) return;
    setHist(hist.slice(0, -1));
    if (h.dir > 0) setKept((k) => k.filter((q) => q.id !== h.p.id));
    else removePassed(snap, h.p.id);
    setI(i - 1);
  };
  keys.current = {
    ArrowLeft: () => fly(-1),
    ArrowRight: () => fly(1),
    ArrowUp: undo,
    z: undo,
  };
  const book = () => {
    if (!open) return setOpen(true);
    setClosing(true);
    setTimeout(() => {
      setOpen(false);
      setClosing(false);
    }, 340);
  };

  const cardStyle = (depth: number) => {
    if (depth) {
      return {
        transform: `translateY(${depth * 10}px) scale(${1 - depth * 0.04})`,
      };
    }
    if (out)
      return {
        transform: `translate(${out * 600}px,40px) rotate(${out * 25}deg)`,
        opacity: 0,
      };
    if (dx !== null && !open)
      return {
        transform: `translate(${dx}px,${Math.abs(dx) * 0.08}px) rotate(${dx / 18}deg)`,
      };
    return {};
  };

  return (
    <section className="swipe">
      <div className="sw-head">
        <button className="btn" onClick={() => nav("/scouting")}>
          ← Filters
        </button>
        <span className="sub">
          Swipe right to shortlist, left to pass
          {total !== undefined && ` · ${total.toLocaleString()} match`}
        </span>
        <button
          className="btn"
          onClick={() => {
            clearPassed(snap);
            setStack([]);
            setHist([]);
            setI(0);
            page.current = 0;
            setLoaded(false);
            more();
          }}
        >
          Reset passed
        </button>
      </div>
      <div
        className={`stage ${open ? "open" : ""} ${closing ? "closing" : ""}`}
      >
        <div className="main">
          <div className="deck">
            {!top && (
              <div className="card">
                <div className="done">
                  <h2>{loaded ? "All scouted" : "Loading…"}</h2>
                  {loaded && <div>{kept.length} on your shortlist</div>}
                </div>
              </div>
            )}
            {[...rest].reverse().map((p, k) => {
              const depth = rest.length - 1 - k;
              const isTop = depth === 0;
              return (
                <div
                  key={p.id}
                  className={`card ${isTop && dx !== null && !open ? "drag" : ""}`}
                  style={cardStyle(depth)}
                  onPointerDown={
                    isTop
                      ? (e) => {
                          x0.current = e.clientX;
                          e.currentTarget.setPointerCapture(e.pointerId);
                          setDx(0);
                        }
                      : undefined
                  }
                  onPointerMove={
                    isTop && dx !== null
                      ? (e) => setDx(e.clientX - x0.current)
                      : undefined
                  }
                  onPointerUp={
                    isTop && dx !== null
                      ? (e) => {
                          const d = e.clientX - x0.current;
                          if (Math.abs(d) < 5) {
                            setDx(null);
                            book();
                          } else if (Math.abs(d) > THRESH && !open) {
                            fly(Math.sign(d));
                          } else setDx(null);
                        }
                      : undefined
                  }
                >
                  <Face p={p} drag={isTop && !open ? (dx ?? 0) : 0} />
                </div>
              );
            })}
          </div>
        </div>
        <div className={`panel ${flip ? "flip" : ""}`}>
          {open && top && <Detail snap={snap} p={top} onClose={book} />}
        </div>
      </div>
      <div className="controls">
        <div className="acts">
          <button className="round sm" onClick={undo} disabled={!hist.length}>
            Undo
          </button>
          <button className="round no" onClick={() => fly(-1)}>
            Pass
          </button>
          <button className="round yes" onClick={() => fly(1)}>
            Sign
          </button>
        </div>
        <div className="count">
          {top && left !== undefined ? `${left.toLocaleString()} left` : ""}
        </div>
        <div className="picked">
          {kept.map((p) => (
            <span key={p.id} onClick={() => nav(`/player/${p.id}`)}>
              {p.n}
            </span>
          ))}
        </div>
      </div>
    </section>
  );
}
