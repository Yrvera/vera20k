# vera20k — FULL-PARITY Completion Strategy (2026-07-05)

**Target: vera20k works the same as gamemd.exe — 100% indistinguishable in skirmish** (CLAUDE.md bar).
Includes: every roster mechanic, every superweapon, skirmish AI opponents, MP lockstep, and
proven-parity closure over every already-implemented system. Campaign remains out of scope
(the bar is skirmish); flag if that changes.

Built by two workflows (28 agents total): 19 miners over all 41 session transcripts
(2026-06-04 → 07-05), 4 state mappers, synthesis + adversarial critique (15 defects folded in),
then 3 sizing lanes (full surface checklist / AI sizing / closure extrapolation). Evidence:
scratchpad `digest-01..19.md`, `state-*.md`, `full-parity-checklist.md`, `ai-sizing.md`,
`closure-extrapolation.md`, `synthesis-draft.md`.

---

## 1. Where the project stands

- ~281k LOC; `src/sim` ≈ 148k; **4,060 tests** green. `src/net` = **64 lines** (MP missing entirely).
- Marker hygiene excellent: 0 `todo!()`/`unimplemented!()`, 2 `TODO(parity)`, 12 `TODO(RE)`.
- **173 observable skirmish surfaces enumerated. Only 3 have ever been parity-scanned** (miner,
  chrono-warp, sidebar audit). 71 implemented-unchecked (presumed DRIFT per default-to-DRIFT),
  29 partial, 43 missing, 19 need research first.
- The one deep scan run (miner, 2026-07-02) found **119 confirmed gaps** on the project's
  MOST-worked system — after which it sits at ~70% observable parity. That is the calibration
  point for everything below.
- Research corpus ~2,410 docs; 16 systems researched-but-unbuilt. `docs/research` has **no backup**.
  No `[profile.release]`. **Zero commits since 2026-06-18** — research is outpacing implementation.

## 2. The full remaining program, sized

Four distinct sub-programs remain. Sessions = one focused sitting (observed: doc-ready slice ≈ 1
session; decode-first cycle ≈ 1+ day).

| Sub-program | Content | Sessions |
|---|---|---|
| **A. Feature completion** | 16 researched-unbuilt systems (mind control, cloak, SWs, spawners, naval…) **+ 43 missing surfaces** the checklist newly surfaced (crates, veterancy XP/promotion, attack dogs, Ivan bombs, suicide detonators, chaos drone, magnetron, floating disc, robot tank link, siege chopper, grinder, cloning vats, tech structures, score screen, ambient audio…) | **~55–75** |
| **B. Parity closure** | Binary-driven disparity scans over ~27 implemented systems (14 scan sessions) + fixing the projected **~550–1,450 gaps** (miner-calibrated; clustering ≈2–3 gaps/fix) | **~100–260** (HIGH+MED proven; +30–60 for literal all-LOW) |
| **C. MP lockstep** | 6 frame-model docs exist; transport undecoded. Slices: command serialization → seed handshake → frame barrier → UDP transport → desync detect → lobby + chat | **UNESTIMATED until transport decode; guess 10–25** |
| **D. Skirmish AI** | gamemd-parity AI opponents: 11 decode units (TeamClass, 20-opcode ScriptClass interpreter, AITrigger evaluator, expert system, threat map, Hunt/Guard missions…) + 10 implementation units replacing the 1,232-line placeholder `ai.rs` | **~45–70** (decode 22–33 ∥-izable + implement 24–36 mostly serial) |

**Total: ~210–430 focused sessions.** At June-burst cadence (1–3/day) that is ~5–9 months of
calendar time; at observed July cadence (limits, Ghidra deaths, infra tax ≈3× on fan-outs),
**realistically 9–18 months**. The bottleneck is session cadence + usage-limit management, not
model capability. Biggest single lever on the estimate: **run closure Wave 1 first** (5 scans of
the complex systems) — it converts the widest uncertainty (is miner's 140-gap yield typical?)
into data for ~5 sessions of cost.

### Key user decisions (surfaced, not made)
1. **LOW-gap fix bar** — 56% of projected gap volume is LOW; "fix all" vs "fix stock-visible,
   defer mod-only" swings ~40–50 sessions. All get surfaced regardless (CLAUDE.md rule).
2. **ScriptClass opcode scope** — stock aimd.ini uses 20 of 64 opcodes; full-64 roughly doubles D3/I3.
3. **Sequencing MP vs AI** (roadmap below assumes MP first — smaller, and it's the named gate).
4. Deferred-pending-sign-off: waypoint visuals, taunts, `.SAV` files, campaign.

## 3. Process evidence (41 sessions — what to keep, what to stop)

**Keep (proven):** the plan pipeline (ground → `/write-plan` → `/review-plan` → execute; 9 tasks/33 min
best case; review finds real defects in ~2/3 of plans) · ~25 min of live-Ghidra verification of a
plan's load-bearing claims before implementing (killed a wrong premise, 2 would-be desyncs, and a
5-of-6-lanes-agreed-wrong consensus) · shadow→hash-neutral→flip→golden ladder for lockstep changes
(0 regressions across 3,879 tests) · `/goal`+Stop-hook autonomous days (~4 user messages in 9h) ·
workflows for read-only grounding/scans, sequential for implementation · incremental disk artifacts
(survived 3 limit massacres + reboot) · frozen-test-count/QUICKPLAY-smoke/user-eyeball gates.

**Stop (measured waste):** launching fan-outs without limit-clock + Ghidra-health preflight (3 lane
massacres, 11h outage) · rustfmt beyond leaf files (4 incidents, worst hour-class) · trusting
docs/plans/memory for status (the #1 waste category; status = git log + code grep) · relaying
unverified lane claims (user caught 3/3) · hand-computed goldens · `cargo test && git commit` ·
two sessions touching SNAPSHOT/goldens the same day.

## 4. Operating model (work type → tool + model)

| Work | Pattern | Model |
|---|---|---|
| Disparity scans (closure program) | Miner-style workflow: FUN_-by-xref lanes + per-lane adversarial verify + live-Ghidra escalation; NOT /gap-scan. Preflight `/ghidra-up` + limit clock; lanes fail loudly, write incrementally | Sonnet 5 lanes, Opus 4.8 verify, Fable 5 orchestrates |
| RE decode (AI, transport) | `/decode-system` / staged anchor→verifiers→adversarial re-derivation; free-text returns for long lanes | Sonnet 5 lanes, Opus 4.8 synthesis |
| Design | `/verify-doc` preflight → `/brainstorm` → `/design-review` | Fable 5 |
| Lockstep-sensitive implementation | plan→review→execute + shadow/flip/golden ladder; ONE SNAPSHOT-bumper at a time; execute in the session holding the grounding when context allows | Fable 5 |
| Bounded/disjoint fix batches | `/verified-fix-swarm`, 3–5 LOW fixes/session, disjoint files only | Opus 4.8 workers |
| Adversarial review | Background 3-reviewer workflow (one re-derives from binary); `/code-review` high–xhigh; trial `ultra` cloud review once on a hash-affecting merge (never yet used; sandbox can't see gitignored docs) | Opus 4.8 |
| Doc hygiene | `/verify-doc-fix-swarm` waves; patch stale docs on sight; mark "landed" on merge | Sonnet 5 |
| Mechanical | Haiku/Bash; rustfmt leaf-only | Haiku 4.5 |
| Ground truth | User side-by-side vs gamemd per system (checklist handed over) + golden-trace harness nightly | human |

## 5. Step-by-step roadmap to full parity

### Phase 0 — Infrastructure (~2 sessions; start today)
0.1 Nightly backup of `docs/research`+plans+contracts+Ghidra project (local scheduled task) — #1 flagged risk.
0.2 `[profile.release]` pinned to dev overflow semantics.
0.3 Workflow templates with the paid-for lessons (incremental writes, fail-loud Ghidra clause, no cargo in background agents).
0.4 `/verify-doc-fix-swarm` repair of the 23 frontier docs degraded by silent doc-fallback.
0.5 *(User call)* local `ini/` copy script for worktree/fresh-checkout compilability.

### Phase 1 — Determinism unblock (~6–8 sessions; prereq for MP and all golden-trace work)
1.1 Damage-fire RNG out of render path (`app_building_anim.rs:182`) — **the restart commit; 1 session, research done.**
1.2 Radiation f64-out-of-hash + f64 trig LUTs → fixed tables (shadow→flip→golden).
1.3 S5 passive-acquire flip (plan exists, FIX FIRST verdict).
1.4 Cell-validation T7 cutover (~39 callers, miner dock-exit first).
1.5 Save/load divergence audit (hash-compare vs live run).
1.6 **Golden-trace harness v1** (scripted skirmish → per-tick observables → nightly regression) — the standing gate for every later phase.
Background: launch **MP transport `/decode-system`** now (read-only).

### Phase 2 — Closure Wave 1: de-risk the big number (~10–15 sessions)
2.1 Five miner-style scans, one system/session: **combat** (first — miner scan already exposed
inverted retaliation there), **drive locomotion/tracks**, **pathfinding/zones**,
**production/factory/placement**, **bridges**. +NV burn-down (~2 sessions).
2.2 Fix the HIGH findings from each scan while fresh (interleave scan→fix; clustering ≈2–3 gaps/fix).
2.3 **Re-estimate sub-program B with real data** — this is where 100–260 collapses to a real number.
Background: AI decode lanes D1 (aimd.ini object model) + D10 (DifficultyClass) can start — read-only.

### Phase 3 — Feature completion (~55–75 sessions, DEFAULT SERIAL on sim files)
Loop per item: `/verify-doc` preflight → (`/brainstorm` if non-obvious) → `/write-plan` →
`/review-plan` → execute → suite + QUICKPLAY + golden-trace + user eyeball → doc "landed".
- 3a **Verified-gap burn-down**: miner-scan 19 HIGH + Wave-1 HIGH remainder via `/verified-fix-swarm` where disjoint.
- 3b **Faction-breaking**: mind control/CaptureManager → Psychic Dominator; cloaking (+waiting render stubs) → submarines; Chronosphere; Nuke; naval zone-legality one-liner.
- 3c **Roster mechanics (checklist additions)**: crates; veterancy XP/promotion-on-kill; attack dog leap+spy detection; Ivan bombs; suicide detonators (Terrorist/Demo Truck); terror drone parasite; chaos drone; magnetron; floating disc; robot tank/control center; siege chopper deploy; IFV full mode matrix; Grinder; Cloning Vats; SpawnManager (carrier/Dreadnought/V3); Kirov run (needs small RE).
- 3d **World & match flow**: tech structures (Oil/Hospital/Machine Shop/Outpost/Secret Lab/Airport); prism forwarding; tesla charge; slave-manager completion; Ship production category; score screen (`/re-investigate` first); surrender/diplomacy; EVA+audio cues; ambient/working sounds; options sub-dialogs; Movies/Credits playback.
Exit: all 43 MISSING surfaces closed or explicitly user-deferred; every faction's signature kit works.

### Phase 4 — Closure Waves 2–4 (~30–60 scan+fix sessions, interleaved scan→fix)
4.1 Wave 2 (mid systems, 2/session): vision+radar, ore+garrison, SW subset, aircraft+veterancy completion-contracts, triggers+EVA+music.
4.2 Wave 3 (simple, 3–4/session): power/radiation/gates/defeat; RNG routing (adversarial re-derivation — lockstep-critical) + brackets + menu shell.
4.3 Wave 4 (special): asset parsers via byte-golden comparison vs retail files; skirmish shell + substrate AFTER in-flight work lands.
4.4 Fix waves per scan; LOW tail per the user's triage decision; per-system user side-by-side gate.

### Phase 5 — MP lockstep (10–25 sessions; UNESTIMATED until 5.1)
5.1 Transport decode (backgrounded since Phase 1) → `/implementation-contract` merging 6 frame docs + transport.
5.2 `/brainstorm` net architecture → slice ladder: command serialization → seed handshake (RNG_MP_SEED_HANDSHAKE) → frame barrier + FRAMESENDRATE → LAN/UDP → desync detection (per-tick hash exists) → host/join lobby + minimal chat/desync banner.
5.3 Loopback two-client harness (two `World`s in-process) before real sockets, every slice.
5.4 Exit: two-human LAN skirmish, zero desyncs, 30+ min soak, fault injection recovers per gamemd, MP replay bit-identical.

### Phase 6 — Skirmish AI (~45–70 sessions; decode lanes background-start during Phases 3–5)
6.1 Decode D1–D11 (TaskForce/Team/Script object model; TeamClass lifecycle; 20 stock opcodes; AITrigger evaluator + weight economy — biggest lockstep risk; expert-system base plan/placement; threat map; Hunt/Area Guard; IQ; AI SW targeting; difficulty; autocreate). Parallelizable as `/decode-system` runs.
6.2 Implement I1→I10: ini parser+registries → TeamClass runtime → Script interpreter → AITrigger glue → expert system replacing `ai.rs` (XL, deepest integration) → threat map → mission verbs → IQ → SW targeting → difficulty (fixed-point-exact). Serial chain I2→I3→I4, D5→I5. Placeholder AI keeps the game playable throughout.
6.3 Exit: AI plays a full skirmish indistinguishably (build order, team composition, attack cadence side-by-side vs gamemd at each difficulty).

### Phase 7 — Final closure (~10–15 sessions)
7.1 Cross-system re-scan pass (early fixes can mask/create gaps in interacting systems).
7.2 NV-tail resolution; deferred-tail burn-down (§2D list of the prior report stays visible).
7.3 Full manual side-by-side program: user checklist per system, every faction, every SW, MP + AI.
Exit = the bar: **a skirmish in vera20k cannot be told apart from gamemd.**

## 6. Standing guardrails (each mapped to a measured failure)

| Risk | Guardrail |
|---|---|
| Limit massacres of fan-outs (×3) | Reset-clock preflight; wave-batch ≤ headroom; Sonnet/Haiku lanes; incremental writes; resumeFromRunId |
| Ghidra death mid-run (×4+) | `/ghidra-up` + live probe before any dependent launch; lanes fail loudly — silent doc-fallback is forbidden |
| rustfmt churn (×4) | Leaf files only, diff-check, never mod.rs/crate-wide |
| Stale docs as truth (#1 waste) | Status = git log + code grep; re-verify plan anchors first 2 min; patch stale docs on sight |
| Unverified lane claims (3/3 caught) | Spot-verify load-bearing claims before presenting; severity needs a consumer trace |
| Lockstep constants from docs (2 near-desyncs) | Live decompile for every RNG stream/default/threshold |
| Tree contention | Serial by default on sim; one SNAPSHOT-bumper at a time; worktree+ini copy when parallel |
| Verification theater | Literal `test result:` line; never `test && commit`; dead review ≠ clean review |
| Work loss | Commit-per-task; end-of-session "Goal:" seed prompt; nightly backup |
