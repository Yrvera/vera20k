#!/usr/bin/env python3
"""Recover raw audit findings + verifier verdicts from the workflow agent transcripts,
join them, and emit (1) a complete findings JSON sidecar and (2) a pixel/geometry
disparity doc. Nothing is paraphrased — these are the agents' exact StructuredOutput objects."""
import json, glob, os, re, collections

TDIR = r"C:/Users/enok/.claude/projects/C--Users-enok-Documents-ra2-rust-game/9a0c3d0e-3f4f-4d07-9b35-631498a4d5e3/subagents/workflows/wf_8ef3ce9c-3ac"
OUT_JSON = r"C:/Users/enok/Documents/ra2-rust-game/docs/research/UI_PARITY_AUDIT_2026_05_29.findings.json"
OUT_PIX  = r"C:/Users/enok/Documents/ra2-rust-game/docs/research/UI_PARITY_PIXEL_DISPARITIES_2026_05_29.md"

def structured_outputs(fn):
    outs = []
    for line in open(fn, encoding="utf-8"):
        line = line.strip()
        if not line:
            continue
        try:
            o = json.loads(line)
        except Exception:
            continue
        m = o.get("message", {})
        c = m.get("content") if isinstance(m, dict) else None
        if isinstance(c, list):
            for b in c:
                if isinstance(b, dict) and b.get("type") == "tool_use" and b.get("name") == "StructuredOutput":
                    outs.append(b.get("input"))
    return outs

audits, verdicts = [], {}
for fn in glob.glob(os.path.join(TDIR, "agent-*.jsonl")):
    for inp in structured_outputs(fn):
        if not isinstance(inp, dict):
            continue
        if "findings" in inp:           # audit agent
            audits.append(inp)
        elif "verdict" in inp and "id" in inp:  # verifier agent
            verdicts[inp["id"]] = inp

# Join findings to verdicts by id
records = []
for a in audits:
    surface = a.get("surface", "?")
    dim = a.get("dimension", "?")
    for f in a.get("findings", []):
        rec = dict(f)
        rec["surface"] = surface
        rec["dimension"] = dim
        rec["verdict_obj"] = verdicts.get(f.get("id"))
        records.append(rec)

# De-dup by id (keep first)
seen, uniq = set(), []
for r in records:
    rid = r.get("id")
    if rid in seen:
        continue
    seen.add(rid)
    uniq.append(r)

with open(OUT_JSON, "w", encoding="utf-8") as fh:
    json.dump({
        "audit_run": "wf_8ef3ce9c-3ac",
        "date": "2026-05-29",
        "audit_dimensions": [{"dimension": a.get("dimension"), "surface": a.get("surface"),
                              "rust_files_read": a.get("rust_files_read"),
                              "docs_consulted": a.get("docs_consulted"),
                              "ghidra_calls": a.get("ghidra_calls")} for a in audits],
        "total_findings": len(uniq),
        "findings": uniq,
    }, fh, indent=2)

# ---- Pixel / geometry / off-by-one filter -------------------------------
PIX_CAT = {"drift-layout"}
# Tight geometry/offset signal — avoid generic width/height/position/x/y prose.
PIX_RE = re.compile(r"(\bpixel\b|\bsub-?pixel\b|\bpx\b|off-?by-?one|"
                    r"[+−-]\s?\d+\s?(px|pixel|cell|frame|row)|"
                    r"center(ed|ing)?\b|centre|"
                    r"\boffset\b|\borigin\b|\brect\b|\brectangle\b|coordinate|"
                    r"\bgeometry\b|\balign(ed|ment)?\b|row height|frame offset|"
                    r"\bdlu\b|\d{2,4}\s?[x×]\s?\d{2,4}|baseline|\binset\b|"
                    r"\bcentered\b|placement|\bnudge|1-?pixel|one pixel|by one)", re.I)

def is_pixel(r):
    if r.get("category") in PIX_CAT:
        return True
    blob = " ".join(str(r.get(k, "")) for k in
                    ("title", "gamemd_behavior", "rust_behavior", "rust_location", "evidence"))
    return bool(PIX_RE.search(blob))

pix = [r for r in uniq if is_pixel(r)]
def vd(r):
    return (r.get("verdict_obj") or {}).get("verdict", "unknown")
pix.sort(key=lambda r: (r.get("surface", ""), 0 if vd(r) == "confirmed-drift" else 1, r.get("id", "")))

SEV = lambda r: (r.get("verdict_obj") or {}).get("final_severity", r.get("player_visibility", "?"))

lines = []
lines.append("# UI Pixel / Geometry Disparities — Main Menu / Skirmish Shell / Loading Screen")
lines.append("")
lines.append("**Date:** 2026-05-29  •  **Source run:** `wf_8ef3ce9c-3ac`  •  derived from `UI_PARITY_AUDIT_2026_05_29.findings.json`")
lines.append("")
lines.append("Every finding below involves a position/size/offset/centering/origin/frame-level "
             "difference — the 1-pixel-and-up geometry class. Verdicts are the adversarial "
             "verifier's (confirmed-drift / false-positive / needs-research). Records are the "
             "agents' exact output, not paraphrased.")
lines.append("")
lines.append("## Coverage boundary (read first)")
lines.append("")
lines.append("This is a **docs+Ghidra-driven** geometry audit: it catches pixel drifts that are "
             "documented in the verified RE reports or derivable from decompiled layout constants "
             "(rect tables, DLU→pixel math, centering formulas, frame offsets). It is **NOT** an "
             "exhaustive framebuffer pixel-diff against a running gamemd.exe — a true "
             "per-element screenshot diff (e.g. text kerning, 1px palette-edge bleed, anti-alias "
             "fringes) would require a side-by-side capture pass and is listed as future work at "
             "the end. Absence from this list is therefore **not** proof of pixel-identity; "
             "unproven geometry stays DRIFT/UNCHECKED per the parity bar.")
lines.append("")

confirmed = [r for r in pix if vd(r) == "confirmed-drift"]
other = [r for r in pix if vd(r) != "confirmed-drift"]
lines.append(f"**Pixel/geometry findings:** {len(pix)} total — {len(confirmed)} confirmed-drift, {len(other)} false-positive/needs-research.")
lines.append("")

for surf in ["main_menu", "skirmish_shell", "loading"]:
    sr = [r for r in pix if r.get("surface") == surf]
    if not sr:
        continue
    lines.append(f"## {surf}")
    lines.append("")
    for r in sr:
        v = r.get("verdict_obj") or {}
        lines.append(f"### `{r.get('id')}` — {r.get('title')}")
        lines.append("")
        lines.append(f"- **Verdict:** {v.get('verdict','?')}  •  **Severity:** {SEV(r)}  •  "
                     f"**Category:** {r.get('category')}  •  **Frequency:** {r.get('trigger_frequency','?')}")
        lines.append(f"- **Rust:** `{r.get('rust_location','?')}` — {r.get('rust_behavior','')}")
        lines.append(f"- **gamemd:** {r.get('gamemd_behavior','')}")
        lines.append(f"- **Evidence:** {r.get('evidence','')}")
        if v.get("reasoning"):
            lines.append(f"- **Verifier:** {v.get('reasoning')}")
        if v.get("correction"):
            lines.append(f"- **Correction:** {v.get('correction')}")
        lines.append("")

lines.append("## Future work — true pixel-diff pass not yet done")
lines.append("")
lines.append("- Capture retail gamemd.exe frames for each surface (main menu idle, skirmish "
             "setup populated, loading screen mid-progress) and diff against our render at the "
             "same resolution/palette. This is the only way to close sub-pixel/kerning/edge-bleed "
             "claims that a layout-constant audit cannot reach.")
lines.append("- Anything not appearing above is **UNCHECKED at the pixel level**, not confirmed identical.")
lines.append("")

with open(OUT_PIX, "w", encoding="utf-8") as fh:
    fh.write("\n".join(lines))

# Console summary
bycat = collections.Counter(r.get("category") for r in uniq)
byverdict = collections.Counter(vd(r) for r in uniq)
print(f"audit agents parsed : {len(audits)}")
print(f"verdicts parsed     : {len(verdicts)}")
print(f"total findings (uniq): {len(uniq)}")
print(f"verdict breakdown   : {dict(byverdict)}")
print(f"category breakdown  : {dict(bycat)}")
print(f"pixel/geometry subset: {len(pix)}  (confirmed={len(confirmed)})")
print(f"wrote: {OUT_JSON}")
print(f"wrote: {OUT_PIX}")
