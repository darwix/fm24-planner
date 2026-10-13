import "./upload.css";
import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useSetRecoilState } from "recoil";
import {
  clubState,
  selectedState,
  squadState,
  vacantState,
} from "@/atoms/squadState";
import type { Club, ImportJob, SaveFile } from "@/models/save";
import {
  getSnapshot,
  importSave,
  listSaves,
  loadSquad,
  searchPlayers,
} from "@/services/save";
import { sz } from "@/utils/format";
import { loadArr } from "@/utils/store";
import { buildSquad } from "@/utils/squad";

const input = {
  font: "500 14px Geist",
  padding: "9px 10px",
  border: "1px solid var(--line)",
  borderRadius: 8,
  width: "100%",
  margin: "8px 0 16px",
};

export default function Upload() {
  const nav = useNavigate();
  const setSquad = useSetRecoilState(squadState);
  const setSel = useSetRecoilState(selectedState);
  const setVacant = useSetRecoilState(vacantState);
  const setClub = useSetRecoilState(clubState);
  const [saves, setSaves] = useState<SaveFile[] | null>(null);
  const [file, setFile] = useState<SaveFile | null>(null);
  const [job, setJob] = useState<ImportJob | null>(null);
  const [snap, setSnap] = useState("");
  const [clubs, setClubs] = useState<Club[]>([]);
  const [gameDate, setGameDate] = useState("");
  const [name, setName] = useState("");
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    listSaves()
      .then(setSaves)
      .catch(() => {
        setSaves([]);
        setErr(
          "Can't reach the SaveLens backend on port 4242. Start it and reload.",
        );
      });
  }, []);

  const pick = async (s: SaveFile) => {
    setFile(s);
    setErr("");
    try {
      const id = await importSave(s.id, setJob);
      const sn = await getSnapshot(id);
      setClubs(sn.clubs);
      setGameDate(sn.gameDate);
      setName(sn.clubs.find((c) => c.id === sn.managedClubId)?.name ?? "");
      setSnap(id);
    } catch (e: any) {
      setFile(null);
      setJob(null);
      setErr(e.message);
    }
  };

  const open = async () => {
    const club = clubs.find(
      (c) => c.name.toLowerCase() === name.trim().toLowerCase(),
    );
    if (!club) return setErr("Pick a club from the list.");
    setBusy(true);
    setErr("");
    let players;
    try {
      ({ players } = await searchPlayers(snap, { club: club.id, limit: 100 }));
    } catch (e: any) {
      setBusy(false);
      return setErr(e.message);
    }
    let kept;
    try {
      const saved = await loadArr({
        name: club.name,
        snapshotId: snap,
        gameDate,
      });
      kept = saved?.slots && (await loadSquad(snap, gameDate, saved));
    } catch (e: any) {
      // keep the saved arrangement instead of replacing it with a fresh squad
      setBusy(false);
      return setErr(
        `Couldn't restore your saved squad: ${e.message}. Try again.`,
      );
    }
    const squad = kept?.squad.length
      ? kept.squad
      : buildSquad(players, gameDate);
    setSquad(squad);
    setVacant(kept?.squad.length ? kept.vacant : []);
    setSel(
      Math.max(
        0,
        squad.findIndex((s) => s.xi),
      ),
    );
    setClub({ name: club.name, snapshotId: snap, gameDate });
    nav("/squad");
  };

  return (
    <section className="upl">
      <h2>Load your save</h2>
      <div className="sub">
        Football Manager 2024 · processed on this device, nothing is uploaded.
      </div>
      {err && <div className="err">{err}</div>}
      {!file &&
        (saves ? (
          <div className="list">
            {saves.map((s) => (
              <div key={s.id} className="row" onClick={() => pick(s)}>
                <b>{s.name}</b>
                <span className="ca">{sz(s.size)}</span>
              </div>
            ))}
          </div>
        ) : (
          <div className="sub">Looking for saves…</div>
        ))}
      {file && (
        <>
          <div className="fi">
            <b>{file.name}</b>
            <span>{sz(file.size)}</span>
          </div>
          {!snap ? (
            <>
              <div className="pb">
                <i style={{ width: `${job?.progress || 0}%` }} />
              </div>
              <div className="sub">{job?.message || "Starting"}</div>
            </>
          ) : (
            <>
              <div className="sub">Which club do you manage?</div>
              <input
                list="clubs"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Start typing a club"
                style={input}
              />
              <datalist id="clubs">
                {clubs.map((c) => (
                  <option key={c.id} value={c.name} />
                ))}
              </datalist>
              <button className="btn pri" onClick={open} disabled={busy}>
                {busy ? "Loading squad…" : "Open squad"}
              </button>{" "}
              <button
                className="btn"
                disabled={busy}
                onClick={() => {
                  setFile(null);
                  setSnap("");
                  setJob(null);
                }}
              >
                Choose another
              </button>
            </>
          )}
        </>
      )}
    </section>
  );
}
