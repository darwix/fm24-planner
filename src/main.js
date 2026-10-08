import "./style.css";
import { squad, cands, pool, TAG, $, d, money } from "./data.js";
import {
  TARGET,
  need,
  matchFilter,
  asCandidate,
  contractLabel,
  isFmFile,
} from "./logic.js";

// inline onclick handlers write these, so they live on window
Object.assign(window, {
  sel: 3,
  open_: false,
  addPos: null,
  li: 0,
  pl: null,
  on: ["exp", "list", "cheap"],
  up: { f: null, st: -1, err: "" },
});
const row = (s, i) =>
  `<div class="row ${i === sel ? "sel" : ""}" onclick="sel=${i};r()"><span class="p">${s.p}</span><b>${s.n}</b><span class="ca">${s.ca}</span>${i === sel ? `<button class="btn" onclick="event.stopPropagation();addPos=null;go('short')">Find replacement</button><button class="btn del" onclick="event.stopPropagation();rm()">Remove</button>` : ""}</div>`;
function r() {
  $("#rt").hidden = !open_;
  $("#squad").classList.toggle("solo", !open_);
  $("#pitch").innerHTML = squad
    .map((s, i) =>
      s.xi
        ? `<div class="pl ${i === sel ? "sel" : ""} ${s.c <= 2052 ? "bad" : ""}" style="left:${s.x}%;top:${s.y}%" onclick="sel=${i};open_=true;r()"><i>${s.ca}</i><small>${s.n.split(" ").pop()}</small><em>${s.p}</em></div>`
        : "",
    )
    .join("");
  $("#grp").innerHTML = [
    ["Goalkeeper", ["GK"]],
    ["Right back", ["DR"]],
    ["Centre back", ["DC"]],
    ["Left back", ["DL"]],
    ["Defensive mid", ["DM"]],
    ["Attacking mid", ["AM"]],
    ["Right wing", ["AMR"]],
    ["Left wing", ["AML"]],
    ["Striker", ["ST"]],
  ]
    .filter(([g, ps]) => ps.includes(squad[sel]?.p))
    .map(([g, ps]) => {
      const rows = squad
        .map((s, i) => [s, i])
        .filter(([s]) => ps.includes(s.p))
        .sort(
          (a, b) =>
            ps.indexOf(a[0].p) - ps.indexOf(b[0].p) || b[0].ca - a[0].ca,
        );
      const T = TARGET[ps[0]],
        need_ = need(squad, ps[0]);
      return `<div class="bn">${g.toUpperCase()} <span class="cv ${need_ ? "lo" : ""}">${Math.min(rows.length, T)}/${T}</span></div><div class="list">${rows.map(([s, i]) => row(s, i)).join("")}${Array.from({ length: need_ }, () => `<div class="row ad" onclick="findPos('${ps[0]}')">+ Add cover · ${ps[0]}</div>`).join("")}</div>`;
    })
    .join("");
}
function go(p) {
  $("#cn").textContent = cands.length;
  if (location.hash !== "#/" + p) {
    location.hash = "/" + p;
    return;
  }
  for (const k of ["squad", "short", "scout", "player", "upload"]) {
    $("#" + k).hidden = p !== k;
    $("#n-" + k)?.classList.toggle(
      "on",
      p === k || (k === "scout" && p === "player"),
    );
  }
  if (p === "short") shortlist();
  if (p === "scout") scout();
  if (p === "player") detail();
  if (p === "upload") upView();
}
function shortlist() {
  const add = !!addPos,
    t = squad[sel],
    K = add ? [] : Object.keys(t.at),
    src = add
      ? addPos === "DC"
        ? cands
        : pool.filter((p) => p.p === addPos).map(asCandidate)
      : cands,
    L = src.filter((c) => c.tags.some((g) => on.includes(g)));
  $("#short").innerHTML =
    `${add ? `<div class="tc"><div class="sub" style="margin:0">ADDING COVER</div><h2>${addPos}</h2><div class="sub">${squad.filter((s) => s.p === addPos).length} in squad</div><button class="btn" onclick="addPos=null;go('squad')">← Back to squad</button></div>` : ""}<div class="tc" ${add ? "hidden" : ""}><div class="sub" style="margin:0">REPLACING</div><h2>${t.n}</h2><div class="sub" style="margin:0">${t.p} · ${t.a}y · contract ${t.c}</div>
 <div class="big" style="margin-top:14px">${t.ca}</div><div class="sub">CA · PA ${t.pa} · €${t.w}k/wk</div>
 <div class="at">${K.map((k) => `<div>${k}<b>${t.at[k]}</b></div>`).join("")}</div><button class="btn" onclick="go('squad')">← Back to squad</button></div>
 <div><h2>Shortlist</h2><div class="sub">Ranked by profile match · ${L.length} players</div>
 <div class="chips">${Object.keys(TAG)
   .map(
     (k) =>
       `<span class="chip ${on.includes(k) ? "on" : ""}" onclick="tg('${k}')">${TAG[k]}</span>`,
   )
   .join("")}</div>
 ${
   L.map(
     (
       c,
     ) => `<div class="cand"><div class="m">${c.m}</div><div><h3>${c.n} <span style="color:var(--mut);font-weight:400">· ${c.cl}</span></h3>
 <p>${c.p} · ${c.a}y · CA ${c.ca}/${c.pa} · ${contractLabel(c.left, c.c || 2052 + Math.ceil(c.left / 12))} · ${money(c.v)} · €${c.w}k/wk</p>${c.tags.map((g) => `<span class="tg ${g}">${TAG[g]}</span>`).join("")}
 <div class="dl">${K.map((k) => `<span>${k}<b>${c.at[k]}</b>${d(c.at[k], t.at[k])}</span>`).join("")}</div></div>
 <button class="btn pri" onclick="${add ? "addC" : "swap"}('${c.n}')">${add ? "Add to squad" : "Swap in"}</button></div>`,
   ).join("") || '<div class="sub">No matches. Turn a filter back on.</div>'
 }</div>`;
}
const LISTS = [
  {
    name: "Promising youngsters",
    f: { pos: "", min: 18, max: 23, exp: false, val: "" },
    sort: "pa",
    fixed: 1,
  },
  {
    name: "Contract expiring",
    f: { pos: "", min: 16, max: 45, exp: true, val: "" },
    fixed: 1,
  },
];
const cnt = (f) => pool.filter((p) => match(p, f)).length;
const match = matchFilter;
function scout() {
  const L = LISTS[li],
    f = L.f,
    ps = [...new Set(pool.map((p) => p.p))];
  const rows = pool.filter((p) => match(p, f)).sort((a, b) => b.pa - a.pa);
  $("#scout").innerHTML =
    `<div><h2>Lists</h2><div class="sub">Saved scouting searches</div><div class="ls">${LISTS.map((l, i) => `<div class="li ${i === li ? "sel" : ""}" onclick="li=${i};scout()">${l.name}<small>${cnt(l.f)}</small>${l.fixed ? "" : `<span class="x" onclick="event.stopPropagation();LISTS.splice(${i},1);li=0;scout()">✕</span>`}</div>`).join("")}</div>
 <button class="btn" style="margin-top:10px" onclick="LISTS.push({name:'New list',f:{pos:'',min:16,max:40,exp:false,val:''}});li=LISTS.length-1;scout()">+ New list</button></div>
 <div><h2>${L.fixed ? L.name : `<input type=text value="${L.name}" onchange="L_=LISTS[li].name=this.value;scout()" style="font:700 20px Fraunces,serif;border:0;background:none;width:100%">`}</h2><div class="sub">${rows.length} players · ranked by potential${L.fixed ? " · preset" : ""}</div>
 <div class="cr">
  <label>Position<select onchange="LISTS[li].f.pos=this.value;scout()"><option value="">Any</option>${ps.map((x) => `<option ${f.pos === x ? "selected" : ""}>${x}</option>`).join("")}</select></label>
  <label>Age<span><input type=number value="${f.min}" onchange="LISTS[li].f.min=+this.value;scout()"> – <input type=number value="${f.max}" onchange="LISTS[li].f.max=+this.value;scout()"></span></label>
  <label>Max value (€M)<input type=number value="${f.val}" placeholder="any" onchange="LISTS[li].f.val=+this.value;scout()"></label>
  <label class="ck"><input type=checkbox ${f.exp ? "checked" : ""} onchange="LISTS[li].f.exp=this.checked;scout()"> Contract expiring (≤12m)</label></div>
 <table class="tb"><thead><tr><th>PA</th><th>Name</th><th>Pos</th><th>Age</th><th>From</th><th>CA</th><th>Contract</th><th class="r">Value</th><th class="r">Wage</th><th></th></tr></thead><tbody>
 ${
   rows
     .map(
       (
         p,
       ) => `<tr class="cl" onclick="openP('${p.n}')"><td class="pa">${p.pa}</td><td><b>${p.n}</b><br><span style="color:var(--mut);font-size:12px">${p.cl}</span></td><td>${p.p}</td><td>${p.a}</td><td>${p.nat}</td><td>${p.ca}</td>
 <td style="${p.left <= 12 ? "color:var(--warn);font-weight:600" : ""}">${p.left} months</td><td class="r">${money(p.v)}</td><td class="r">€${p.w}k</td><td class="r"><button class="btn" onclick="event.stopPropagation();addP('${p.n}')">+ Add</button></td></tr>`,
     )
     .join("") ||
   '<tr><td colspan=10 class="sub" style="padding:24px">No players match.</td></tr>'
 }</tbody></table></div>`;
}
const findPos = (p) => {
  addPos = p;
  go("short");
};
const addC = (n) => {
  const c = [...cands, ...pool].find((x) => x.n === n);
  squad.push({
    n: c.n,
    p: addPos,
    a: c.a,
    ca: c.ca,
    pa: c.pa,
    c: c.c || 2052 + Math.ceil(c.left / 12),
    v: c.v,
    w: c.w,
    at: squad[0].at,
  });
  addPos = null;
  go("squad");
  r();
};
const openP = (n) => {
  pl = n;
  go("player");
};
const hash = (s) =>
  [...s].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);
const GROUPS = {
  Technical: [
    "Crossing",
    "Dribbling",
    "Finishing",
    "First touch",
    "Heading",
    "Marking",
    "Passing",
    "Tackling",
    "Technique",
  ],
  Mental: [
    "Anticipation",
    "Composure",
    "Decisions",
    "Determination",
    "Positioning",
    "Teamwork",
    "Vision",
    "Work rate",
  ],
  Physical: [
    "Acceleration",
    "Agility",
    "Balance",
    "Jumping",
    "Pace",
    "Stamina",
    "Strength",
  ],
};
const attr = (p, a) =>
  Math.max(
    3,
    Math.min(20, Math.round(p.ca / 9.5 + ((hash(p.n + a) % 7) - 3) * 0.9)),
  );
function detail() {
  const p = pool.find((x) => x.n === pl);
  if (!p) return go("scout");
  $("#player").innerHTML =
    `<a class="btn" style="margin:0 -14px" onclick="history.back()">← Back to scouting</a>
 <div class="top"><div><div class="sub" style="margin:0">${p.cl} · ${p.nat}</div><h2>${p.n}</h2><div class="sub" style="margin:0">${p.p} · ${p.a} years</div></div>
 <div class="big">${p.pa}<small>POTENTIAL · CA ${p.ca}</small></div></div>
 <div class="fx"><div>Contract<b style="${p.left <= 12 ? "color:var(--warn)" : ""}">${p.left} months left</b></div><div>Value<b>${money(p.v)}</b></div><div>Wage<b>€${p.w}k/wk</b></div><div>Age<b>${p.a}</b></div><div>Growth room<b>+${p.pa - p.ca} CA</b></div></div>
 <div class="ag">${Object.entries(GROUPS)
   .map(
     ([g, l]) =>
       `<div><h3>${g.toUpperCase()}</h3>${l
         .map((a) => {
           const v = attr(p, a);
           return `<div class="ar"><span>${a}</span><b class="${v >= 14 ? "v-hi" : v >= 11 ? "v-mid" : ""}">${v}</b></div>`;
         })
         .join("")}</div>`,
   )
   .join("")}</div>
 <div style="margin-top:28px"><button class="btn pri" onclick="addP('${p.n}')">+ Add to squad</button></div>`;
}
const addP = (n) => {
  const p = pool.find((x) => x.n === n);
  squad.push({
    n: p.n,
    p: p.p,
    a: p.a,
    ca: p.ca,
    pa: p.pa,
    c: 2052 + Math.ceil(p.left / 12),
    v: p.v,
    w: p.w,
    at: squad[0].at,
  });
  go("squad");
  r();
};
const tg = (k) => {
  on = on.includes(k) ? on.filter((x) => x !== k) : [...on, k];
  shortlist();
};
const rm = () => {
  squad.splice(sel, 1);
  sel = 0;
  r();
};
const swap = (n) => {
  const c = cands.find((x) => x.n === n),
    o = squad[sel];
  Object.assign(o, {
    n: c.n,
    a: c.a,
    ca: c.ca,
    pa: c.pa,
    c: c.c,
    v: c.v,
    w: c.w,
    at: c.at,
  });
  go("squad");
  r();
};
const UPS = [
  "Reading file",
  "Decompressing",
  "Parsing players",
  "Building your squad",
];
const sz = (b) =>
  b >= 1e9 ? (b / 1e9).toFixed(2) + " GB" : (b / 1e6).toFixed(0) + " MB";
function upView() {
  const u = up;
  let h = `<h2>Load your save</h2><div class="sub">Football Manager 2024 · processed on this device, nothing is uploaded.</div>`;
  if (!u.f)
    h += `<label class="dz" id="dz" ondragover="event.preventDefault();this.classList.add('over')" ondragleave="this.classList.remove('over')" ondrop="event.preventDefault();pickF(event.dataTransfer.files[0])"><b>Drop your .fm save here</b><span>or click to choose a file</span><input type="file" accept=".fm" hidden onchange="pickF(this.files[0])"></label>${u.err ? `<div class="err">${u.err}</div>` : ""}`;
  else {
    h += `<div class="fi"><b>${u.f.name}</b><span>${sz(u.f.size)}</span></div><div class="stp">${UPS.map((x, i) => `<div class="${i < u.st ? "done" : i === u.st ? "on" : ""}"><i>${i < u.st ? "✓" : ""}</i>${x}</div>`).join("")}</div>
  <div class="pb"><i style="width:${Math.min(100, (u.st / UPS.length) * 100)}%"></i></div>`;
    if (u.st >= UPS.length)
      h += `<div class="sm"><div>Club<b>Meong Racun</b></div><div>Season<b>2052</b></div><div>Players found<b>13,284</b></div></div><button class="btn pri" onclick="go('squad')">Open squad</button> <button class="btn" onclick="up={f:null,st:-1,err:''};upView()">Choose another</button>`;
  }
  $("#upload").innerHTML = h;
}
function pickF(f) {
  if (!f) return;
  if (!isFmFile(f.name)) {
    up = {
      f: null,
      st: -1,
      err: "That is not a .fm save file. Pick a Football Manager save.",
    };
    return upView();
  }
  up = { f, st: 0, err: "" };
  upView();
  const t = () =>
    setTimeout(() => {
      up.st++;
      upView();
      if (up.st < UPS.length) t();
    }, 900);
  t();
}
addEventListener("hashchange", () => go(location.hash.slice(2) || "upload"));
r();
go(location.hash.slice(2) || "upload");

Object.assign(window, {
  go,
  r,
  tg,
  rm,
  swap,
  addC,
  addP,
  openP,
  findPos,
  scout,
  upView,
  pickF,
  LISTS,
});
