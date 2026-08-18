# Next-Work Survey & Decision (2026-06-25)

Lead decision across the 9 in-flight engine-substrate tracks. Authority: code + git log
over plan-status lines. Branch `dev`.

---

## A. TOP 3 PICKS (ranked)

### #1 — Helper-services: Cell-validation T7 FNPC authoritative cutover (start with miner dock-exit)
- **Track:** Engine-substrate helper-services (cell-validation family).
- **Exact next slice:** Cut the miner dock-exit path onto the substrate diamond-ring FNPC.
  `src/sim/miner/miner_dock_sequence.rs:413` still calls its own pre-substrate
  `find_nearby_passable_cell_with_index` (also reached from `bunker_link.rs:222`,
  `miner_system.rs:1226`) instead of `find_nearby_cell::find_nearby_passable_cell`.
  Then continue the ~39-caller cutover (rally / scatter / chrono-warp / paradrop /
  slave-deploy / crate) behind the shadow→invert→authoritative→`SNAPSHOT_VERSION` 17→18
  →parity-harness discipline (T8).
- **Plan doc:** `docs/plans/2026-06-04-cell-validation-facade-implementation-plan.md:510`.
- **Why #1:** Highest player-visible parity payoff of anything actionable right now, AND
  the substrate + shadow FNPC + all three T4 reconciles already exist (only
  `production_spawn` is wired) — so this is finishing landed work, not a context switch.
  The facade picks a *visibly different cell* than gamemd today: units exit buildings and
  miners re-dock at the wrong nearby cell on a coarse box-ring instead of gamemd's
  diamond-ring frame-counter pick. Fires constantly in normal skirmish (every building
  exit, every miner return). No research blocker; selection logic is already proven.
- **Effort:** M (per-caller arg-plumbing across ~39 sites + one hashed flip + harness).
  The miner dock-exit sub-batch alone is S and is the highest-frequency single caller.

### #2 — Deferred sweep P1: move damage-fire RNG out of the per-frame render path
- **Track:** Deferred markers / cross-cutting (repo-improvement-scan §P1).
- **Exact next slice:** Move the damage-fire spawn decision + RNG draws out of
  `tick_damage_fire_overlays` (`src/app_building_anim.rs:182` draws from
  `sim.anim_rng()` = hashed `scenario_rng`), which is invoked per render frame with
  wall-clock dt at `src/app_sim_tick.rs:271`. Relocate the decision + draws into the
  building-anims phase of `World::advance_tick`, emit an event for the app, keep only
  visual frame-advance app-side. Hash-version bump.
- **Plan doc:** `docs/plans/2026-06-12-repo-improvement-scan.md` §P1.
- **Why #2:** It is the one genuinely-open *active lockstep-desync vector* — a hashed RNG
  stream is consumed at render-frame cadence, so two clients (or a replay) running at
  different frame rates diverge. That directly threatens the determinism invariant every
  other substrate track is built to protect, and it is the prerequisite hygiene for the
  MP-lockstep work (#3 below would inherit this bug). Confirmed still live in code this
  session. Player sees an out-of-sync drop the first time any building hits the
  damage-fire health threshold — i.e. every match with combat.
- **Effort:** M (cross-layer move + event plumbing + hash bump).

### #3 — Object-AI dispatch S5: passive-acquire scan-timer + scenario_rng consume
- **Track:** Object-AI / TechnoClass-FootClass dispatch (master-TODO #1).
- **Exact next slice:** Un-gate `s4c_passive_acquire_eligible` (currently cfg-gated,
  `src/sim/world/techno_ai.rs:556`), add a hashed `passive_scan_timer: MissionTimer` to
  `GameEntity`, parse `NormalTargetingDelay=27` / `GuardAreaTargetingDelay=36`
  (`rulesmd.ini:304-305`, not yet parsed), and add `passive_scan_consume` in
  `techno_common_post` doing the per-scan `n(0,2)` draw on `scenario_rng` consume-only
  (no TarCom set). `SNAPSHOT_VERSION` 25→26 + golden re-baseline.
- **Plan doc:** `docs/plans/2026-06-11-s5-passive-acquire-consume-plan.md`
  (design: `…-s5-passive-acquire-consume-design.md`).
- **Why #3:** Strong in-flight momentum — mirrors the just-landed S4b pattern exactly
  (rules parse → one hashed timer → version bump → re-baseline), so near-zero ramp cost.
  It is RNG-stream parity: consume-only, no visible behavior this slice, but it keeps the
  deterministic `scenario_rng` stream bit-aligned with gamemd so every downstream draw
  (damage, scatter) stays in lockstep — fires every tick any weaponed Unit is on
  Move/Guard/Harvest, i.e. constantly. Ranked below #1/#2 only because it ships no
  player-visible change on its own; it is parity insurance, not a fix.
- **Effort:** M.

**Start now: #1, miner dock-exit cutover.** It is the most player-visible finish of
already-landed infrastructure, has no research gate, and the selection logic is proven.

---

## B. ONE-LINER PER TRACK (next item + status)

- **mission-radio** — Slice 7a airfield/helipad radio adoption + V2 airstrike radio-deaf latch & `WaitForDock` distance gate; in-flight, distance gate UNCHECKED vs `Find_Docking_Bay 0x004DF040`, latch needs a `sim/airstrike` owner-service first. Effort M.
- **object-ai-dispatch** — S5 passive-acquire scan-timer + `scenario_rng` consume-only draw (`SNAPSHOT_VERSION` 25→26); in-flight, no blocker, plan written. Effort M. **(Top-3 #3)**
- **factory-house-economy** — No pending P-slice; substrate migration P0-P9 all landed. Only loose end is the war-factory exit-link BREAK radio code (study-only, UNCHECKED in Ghidra). Effectively-closed (re-investigation S).
- **ui-shell-gadget** — D-B4 mirror retirement (`app.rs:1469/1480`, stale "retired" comment `app.rs:175`); in-flight, internal-only single-authority cleanup, no blocker. Effort S. (§5-D doc audit is a separate M research task.)
- **lookup-tables** — U3 drive_track byte-equality verification gate + move to substrate tree (`read_memory 0x007E7A28/0x007E7B28`); in-flight but **gated on pure-RE byte-equality verification first**, plus OQ-3 user decision on UNVERIFIED BSS consts. Effort M.
- **helper-services** — Cell-validation T7 authoritative FNPC cutover (~39 callers, start miner dock-exit `miner_dock_sequence.rs:413`); in-flight, no blocker, plan written. Effort M. **(Top-3 #1)**
- **core-spine** — MP lockstep transport (system #8 second half: seed handshake + command barrier + execution-frame schedule + transport); `src/net/lockstep.rs` is a 44-line scheduling-only stub. **Blocked on research** (no verified gamemd frame-sync/barrier doc) → `/re-swarm --handoff-plan`. Effort L.
- **render-visual** — No code item; Gap + Radiation threads code-complete, only manual in-game side-by-side verification passes remain (Gap Task 6, Radiation Tasks 6/7). Effectively-closed. Effort S (verification).
- **deferred-sweep** — Repo-scan P1: move damage-fire RNG out of the per-frame render path into `advance_tick` (active desync vector, `app_building_anim.rs:182`); in-flight, no blocker, plan written. Effort M. **(Top-3 #2)**

---

## C. BLOCKED / STUDY-ONLY (needs a research/planning step first)

- **core-spine (MP lockstep)** — BLOCKED on research. No verified doc on gamemd's
  command-barrier / execution-frame / seed-handshake protocol. Required step:
  `/re-swarm --handoff-plan save load hash MP lockstep substrate gaps`, then a plan doc,
  then a `LockstepSession` gating `advance_tick` on a per-frame all-houses-received
  barrier. (Smaller unstarted sub-item that could land first without the handoff:
  save/load-rebuild-matches-live divergence audit, TODO #8 bullet 3.)
- **lookup-tables (U3)** — Gated on a pure-RE byte-equality verification of the 72-entry
  `TurnTrack` / 16-entry `RawTrack` / ~492 `TrackPoint` arrays vs
  `read_memory 0x007E7A28 / 0x007E7B28` and `transform_track_point` vs
  `Transform_Track_Coords` BEFORE moving the table. Plus OQ-3: USER decision on whether to
  accept the constrained-but-unbit-dumped BSS consts (`0x0089F688`, `0x0089F6D8`,
  `0x0089EA40`) or do a live-debugger capture.
- **mission-radio (7a, partial)** — The V2 airstrike radio-deaf latch is blocked on
  building a `sim/airstrike` owner-service (7a only reads `airstrike_owner`, which does
  not yet exist anywhere in `src/sim`). The distance-gate half is UNCHECKED vs
  `Find_Docking_Bay 0x004DF040` — verify in Ghidra before wiring (no code blocker, but a
  research confirm). Distance gate alone can proceed; the latch cannot.
- **factory-house-economy (loose end)** — Study-only: the war-factory exit-link BREAK
  radio code was never isolated in Ghidra (establish=0x02 known, break UNCHECKED). Small
  re-investigation, no plan doc yet.
- **render-visual** — Not blocked code-wise (closed); remaining work is manual gamemd
  side-by-side. Deferred Ghidra-gated follow-ups (gap restore-gate `g_PlayerPtr+0x577A`,
  SNOW-theater channel forcing, `EMPulseSparkles` secondary anim) are study-first, M each.
- **helper-services (study-only families)** — pathfinding-helpers, target-scoring, and
  drawing families have **no implementation plan** (study-only; only the 4 families dated
  2026-06-04 have plans). Target-scoring's equal-score tie-break + cell C22 save-order are
  additionally gated on master-TODO #1 (native live-object vector).

---

## D. DEFERRED TAIL (genuinely-open markers worth knowing)

From the deferred-sweep report, verified against current code:

- **P1 — damage-fire RNG in per-frame render path** (`app_building_anim.rs:182`,
  invoked `app_sim_tick.rs:271`). Active lockstep-desync vector. **→ Top-3 #2.**
- **P3 — no `[profile.release]` in `Cargo.toml`** → release wraps overflow vs dev/test
  panic; different arithmetic semantics → dev-vs-release desync. Effort S.
- **P4 — runtime f64 trig LUTs feeding hashed state** (`homing_movement.rs:127-158`,
  `bam_cos_table`/`bam_sin_table` built from `f64::cos/sin`); not frozen as const. Effort S.
- **Idle scatter disabled** — `src/sim/world/mod.rs:2552-2555` `tick_idle_scatter`
  commented out ("units were moving on their own"); player-visible in crowded battles.
- **Selling-in-progress buildings not rejected as C4 targets** —
  `world_commands.rs:1013` `TODO(parity)` (Mission==0x13).
- **No sim-layering enforcement test** (P6; convention-only).
- **Pathfinding hierarchical-zone layer written-but-unwired** — ~14 `TODO(RE)` across
  `zone_search.rs`/`zone_hierarchy.rs`/`zone_map.rs`/`zone_build.rs`/`cell_entry.rs`/`core.rs`.
  Wire-or-delete is L.
- **Particle-system warhead/damage-to-occupants deferred** —
  `src/sim/particles/{fire,gas,smoke}.rs` (bridge collision + cell-occupant damage no-op).
- **Special locomotors stubbed-as-ground** — `locomotor.rs:19,232`
  (Teleport/Rocket/DropPod; Tunnel is TS-skip).
- **VeteranAbilities in-range bonus stub returns 0** — `in_range.rs:83,118,135`.
- **Menu shell playback not implemented** — Movies/Credits/Sneak-Preview/random-map
  (`main_menu_dialogs.rs:199-203`, `app.rs:1087,1959`); shell-only, never reaches skirmish.

**Stale memory to retire (code refutes the memos):**
- `project_force_track_bib_step.md` — OBSOLETE; Force_Track 0x47 refinery-exit bib step is
  implemented (`miner_dock_sequence.rs:58-60`, commit 973149bf).
- `project_c4_bridge_hut_followup.md` — OBSOLETE; SEAL/Tanya C4 on CABHUT is fixed
  (`world_orders.rs:783-816` + tests).
