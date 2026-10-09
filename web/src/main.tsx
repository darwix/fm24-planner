import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { RecoilRoot } from "recoil";
import { SWRConfig } from "swr";
import { clubState, squadState, vacantState } from "./atoms/squadState";
import { loadSquad } from "./services/save";
import { loadSession } from "./utils/store";
import Routes from "./routes";
import "./index.css";
import { apiFetcher } from "./services/api";

async function start() {
  // restore the last squad on fresh player data from the save
  let saved: Awaited<ReturnType<typeof loadSquad>> | null = null;
  const session = loadSession();
  if (session) {
    try {
      saved = await loadSquad(
        session.club.snapshotId,
        session.club.gameDate,
        session.arr,
      );
    } catch {
      saved = null;
    }
  }
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <RecoilRoot
        initializeState={({ set }) => {
          if (!session || !saved?.squad.length) return;
          set(clubState, session.club);
          set(squadState, saved.squad);
          set(vacantState, saved.vacant);
        }}
      >
        <SWRConfig value={{ fetcher: apiFetcher }}>
          <BrowserRouter>
            <Routes />
          </BrowserRouter>
        </SWRConfig>
      </RecoilRoot>
    </React.StrictMode>,
  );
}
start();
