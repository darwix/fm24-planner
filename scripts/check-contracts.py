"""Check a snapshot's contracts against a full in-game Player Search export (see RULES.md).

    python -I scripts/check-contracts.py <export.html> <snapshot.sqlite> [<previous snapshot.sqlite>]

The export needs Name, Club, F/PT, Salary and Expires columns. Prints, per contract type, how
many players agree on expired status, end date and wage (within the game's display rounding).
With a previous snapshot, also lists the players whose wage or end changed.
"""
import collections
import html
import re
import sqlite3
import sys

sys.stdout.reconfigure(encoding="utf-8")


def export_rows(path):
    page = open(path, encoding="utf-8", errors="replace").read()
    heads = [html.unescape(re.sub(r"<[^>]+>", "", h)).strip() for h in re.findall(r"<th>(.*?)</th>", page, re.S)]
    for tr in re.findall(r"<tr[^>]*>(.*?)</tr>", page, re.S):
        cells = [html.unescape(re.sub(r"<[^>]+>", "", c)).strip() for c in re.findall(r"<td>(.*?)</td>", tr, re.S)]
        if len(cells) == len(heads) and any(cells):
            yield dict(zip(heads, cells))


def money(s):
    m = re.match(r"£([\d,.]+)(K|M)? p/w", s)
    return float(m[1].replace(",", "")) * {"K": 1e3, "M": 1e6, None: 1}[m[2]] if m else None


def date(s):
    m = re.match(r"(\d+)/(\d+)/(\d{4})", s)
    return f"{m[3]}-{int(m[1]):02}-{int(m[2]):02}" if m else None


def players(path):
    con = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    return con.execute("select id, name, full_name, club, wage, contract_end from players").fetchall()


def check(export, snapshot):
    by = collections.defaultdict(list)
    for p in players(snapshot):
        for name in {p[1], p[2]}:
            by[(name, p[3] or "")].append(p)
    used, stats, unmatched = set(), collections.defaultdict(collections.Counter), 0
    for g in export:
        words = g["Name"].split()
        surname_first = " ".join(words[1:] + words[:1])
        found = [p for k in [(g["Name"], g["Club"]), (surname_first, g["Club"])] for p in by.get(k, []) if p[0] not in used]
        if not found:
            unmatched += 1
            continue
        end, wage = date(g["Expires"]), money(g["Salary"])
        p = min(found, key=lambda p: (p[5] != end, p[0]))
        used.add(p[0])
        s = stats[g["F/PT"]]
        s["n"] += 1
        game_expired, ours_expired = end is None, p[4] is None and p[5] is None
        s["expired ok" if game_expired == ours_expired else "expired WRONG"] += 1
        if end:
            s["end ok" if p[5] == end else "end none" if p[5] is None else "end WRONG"] += 1
        if wage is not None:
            # the game rounds the wage it shows (538,133 shows as 550,000)
            ok = p[4] is not None and abs(p[4] - wage) <= max(15, wage * 0.06)
            s["wage ok" if ok else "wage none" if p[4] is None else "wage WRONG"] += 1
    cols = ["n", "expired ok", "expired WRONG", "end ok", "end none", "end WRONG", "wage ok", "wage none", "wage WRONG"]
    print(f"matched {sum(s['n'] for s in stats.values())}, unmatched {unmatched}")
    print(f"{'type':24}" + "".join(f"{c:>14}" for c in cols))
    total = collections.Counter()
    for t, s in sorted(stats.items(), key=lambda x: -x[1]["n"]):
        total.update(s)
        print(f"{t:24}" + "".join(f"{s[c]:>14}" for c in cols))
    print(f"{'TOTAL':24}" + "".join(f"{total[c]:>14}" for c in cols))


def diff(previous, snapshot):
    old = {p[0]: p for p in players(previous)}
    changed = [(old[p[0]], p) for p in players(snapshot) if p[0] in old and old[p[0]][4:] != p[4:]]
    print(f"\n{len(changed)} players changed wage or end")
    for o, n in changed[:50]:
        print(f"  {n[0]} {n[1]} ({n[3] or '-'}): {o[4]} {o[5]} -> {n[4]} {n[5]}")


if __name__ == "__main__":
    check(list(export_rows(sys.argv[1])), sys.argv[2])
    if len(sys.argv) > 3:
        diff(sys.argv[3], sys.argv[2])
