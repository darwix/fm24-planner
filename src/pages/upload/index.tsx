import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useSetRecoilState } from "recoil";
import { clubState, selectedState, squadState } from "@/atoms/squadState";
import type { Club, ImportJob, SaveFile } from "@/models/save";
import {
  getSnapshot,
  importSave,
  listSaves,
  searchPlayers,
} from "@/services/save";
import { sz } from "@/utils/format";
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
  const setClub = useSetRecoilState(clubState);
  const [saves, setSaves] = useState<SaveFile[] | null>(null);
  const [file, setFile] = useState<SaveFile | null>(null);
  const [job, setJob] = useState<ImportJob | null>(null);
  const [snap, setSnap] = useState("");
  const [clubs, setClubs] = useState<Club[]>([]);
  const [name, setName] = useState("");
  const [err, setErr] = useState("");

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
      setClubs((await getSnapshot(id)).clubs);
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
    const { players } = await searchPlayers(snap, {
      club: club.id,
      limit: 100,
    });
    const squad = buildSquad(players);
    setSquad(squad);
    setSel(
      Math.max(
        0,
        squad.findIndex((s) => s.xi),
      ),
    );
    setClub({ name: club.name, snapshotId: snap });
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
              <button className="btn pri" onClick={open}>
                Open squad
              </button>{" "}
              <button
                className="btn"
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
