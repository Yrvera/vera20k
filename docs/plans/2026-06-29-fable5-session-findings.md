# Fable-5 Session Findings — June 10–12, 2026

*Consolidated from 9 Fable-5 sessions on the vera20k / RA2-YR engine-substrate program.*

## Overview

This Fable-5 burst pushed the engine-substrate migration program ("Rust-native
structure, gamemd-native semantics") forward on several fronts at once. Research lanes
refreshed two substrate studies (MapClass/CellClass; ScenarioClass/RulesClass) and
produced a new one (Gadget/Dialog-control UI), while implementation lanes shipped the
work those studies unlocked: per-cell radiation, the TechnoClass/FootClass dispatch ladder
slices S2 and S3, the ScenarioClass/RulesClass scenario-session slices SC-1/RC-1/SC-2, and
the first third of the UI gadget substrate. Two adversarial reviews caught lockstep-fatal
bugs before merge (a coordinate-frame fog-bounds defect; a per-frame RNG desync). A
closing reflection session benchmarked the whole repo (~277k lines Rust, ~4k tests, ~60%
complete by gut feel) and named the project's real gate: someone to play against. Most
substrate-decode work merged to `dev`; a few research outputs produced docs/contracts only.

---

## MapClass / CellClass substrate

**Refresh, not redo.** The existing `CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md`
was extended against ~55 commits of churn via an 11-lane background workflow (tagged
LIVE-0610), then later closed out by a `/goal`-driven implementation session.

Concrete discoveries / decisions:
- **Per-cell radiation runtime was missing entirely in Rust** (new §2.6) — fires every
  Desolator deploy, also demo truck / nuke. Full native contract decoded: cell `+0xF0`=RadLevel,
  `+0xF8`=site ptr (center cell only); spread over (2·CellSpread+1)² square with 3D lepton
  distance and linear falloff `(radius−dist)/radius × level`, radius=`CellSpread×256+128`;
  same-center re-detonation MERGES, different centers stack; decay every `RadLevelDelay`
  frames; damage `ftol(min(level,RadLevelMax)×RadLevelFactor)` via RadSiteWarhead applied
  per-object at `frame % RadApplicationDelay == 0` (not a cell sweep), buildings exempt.
  Label trap: "ApplyRadDamage" is actually the decay step.
- **`g_DirectionOffsets 0x0089F688` RESOLVED** (two confirmations): index 0 = N (0,−1),
  clockwise, +X east/+Y south, idx=facing>>5 — matches Rust embeds.
- **Lifecycle drain model rewritten** 3 drains → 1+1 (hash-changing); the collapse caught
  a real repair-on-corpse bug.
- **MapClass is NOT the per-tick scheduler** — that's `LogicClass::PerTickUpdate`, which
  calls MapClass twice unconditionally (bridge-shroud on frame%120, crate-regen every tick).
- Byte-pinned offsets (cell `+0x30/+0x50/+0x5C..0x77/+0xFC`; MapClass bridge/zone DynVecs);
  `RecalcZoneType` decoded with two corrections (IsRubble/firestorm conflation; dormant
  FirestormWall/LaserFence branches).

Implemented (merged to `dev`):
- **Slice 7 — per-cell radiation field service** (`86b0d4bf`, new `src/sim/radiation.rs`,
  Ghidra-verified-first, 13 acceptance tests incl. exact falloff, `SNAPSHOT_VERSION`→21).
- **Slice 3b — playfield diamond wired into FNPC** (`7044fcec`) so off-diamond border cells
  can't be spawn/exit cells.
- **CliffBackImpassability consequence-set completed** (`8a7e2ea4`). Correction: a correct
  predicate had existed since March; verify lanes had missed it.
- **§4.2 #6 SpeedType/MovementZone "row confusion" downgraded by audit** — the confused
  mapping is dead fallback under stock INI; resolved with evidence, no code.

Artifacts: ~31 edits to the study doc; doc later marked **CLOSED** with a successor tracker
`SUBSTRATE_OPEN_ITEMS_20260610.md`.

Open: radiation **green glow** (needs render-layer dynamic-light infra; visible every
Desolator deploy); `reveal_by_height` re-enable (units currently see over cliffs — biggest
visible win); **crowd-jam invented constants** (threshold 3, 0.7 jam factor, radius-2 scan —
no binary/INI citation, suspected drift); A*-snapshot/corner-cutting, reservation-on-intent.

---

## ScenarioClass / RulesClass substrate contract + implementation

A three-stage arc across sessions: an initial scoping session was **aborted mid-research**
(produced nothing but resolved a prompt ambiguity — target is ScenarioClass/RulesClass, not
MapClass/CellClass); then an `/implementation-contract` run distilled the verified contract;
then a `/goal`-hooked session implemented all three slices.

Contract (three REQUIRED_FIX gaps):
- **SC-1** — per-match seed pipeline never wired; Rust ran hardcoded `DEFAULT_SIM_SEED`,
  and the replay runner recorded `header.seed` but never re-applied it.
- **SC-2** — no `ScenarioClass` session aggregate; scenario identity, theater, map bounds,
  MP waypoints were app-layer or lazily derived.
- **RC-1** — map-INI rules override missing; gamemd re-runs the INI reader on the map file
  over rules sections, so maps with overrides were playing stock.
- Verified-CORRECT and preserved: SC-3 three-stream RNG, SC-4 frame-clock late-commit,
  SC-5 per-map flags, GameOptions mirror. Marked STALE: `RNG_SYSTEM` §3.1/3.2/6 routing
  claims and `RULESCLASS` §8 Rust-status table.

Implemented (merged to `dev`, suite green):
- **SC-1** — fresh u32 seed per real launch via `ScenarioDescriptor` →
  `Simulation::from_descriptor`; replay reconstructs from `header.seed`; `DEFAULT_SIM_SEED`
  demoted to dev/test.
- **RC-1** — rules chain now rules.ini → rulesmd.ini → **map-INI overrides**
  (`merge_rules_overrides`); existing sections only; registries excluded.
- **SC-2** — seed/clock/GameOptions/identity/bounds/MP start table moved onto
  `Simulation.session` (new `src/sim/scenario_session.rs`); `SNAPSHOT_VERSION` walked 20→22.
- **Headline BLOCKER caught by an 18-agent adversarial review:** fog bounds were seeded
  from raw `[Map] Size=`, but sim cells live in the iso-array frame (Dustbowl 70×76 vs grid
  146×146), so start positions sat outside the fog window on every real map → player base
  **permanently shrouded**. The project's documented coordinate-frame bug class. Fixed +
  retail-map regression test + launch tripwire. Review also hardened AT-8 to absolute
  per-stream RNG fingerprints and closed two registry-corruption holes in the map merge.

Artifacts: contract doc; `…-substrate-plan.md`; `scenario_session.rs`; tests
`launch_seed_guard.rs`, `session_bounds_frame.rs`.

Open: **map-INI second pass** — can a map INI *allocate* new type records or only override,
and what a present-but-empty value (`BuildSpeed=`) does (`INIClass::Load` empty-value handling
is the unverified piece; current merge matches neither gamemd branch — RC-1 BLOCKED). Plus
RC-3/RC-4/RC-7/RC-8 follow-ups; SC-6 `.SED` mapgen seeding and SC-7 cell-action timer BLOCKED.

---

## TechnoClass / FootClass dispatch ladder (S2, S3)

Two `/goal`-hooked design→review→plan→implement→adversarial-review→merge slices of the
per-object mission-authority ladder.

**Slice S2 — dispatch-time mission authority** (commits `32f9ef36..7b79a186`, merged):
- P1 blocker fixed: a post-load re-derive won over serde and would desync once
  `mission.current` is real authority → load now trusts the serialized `MissionCom`
  (round-trips, fully hashed). `SNAPSHOT_VERSION` 19→20.
- Per-object `tick_counter++ → dispatch → Process` for scoped movers; dock↔move
  double-write shown structurally impossible; golden baseline empirically UNSHIFTED.
- Review caught real bugs twice (nonexistent `EntityStore::iter_mut()`; Sleep-vs-None
  assertions; mis-cited test-vs-production sites).

**Slice S3 — per-object post-Foot Fire→Facing + idle→Guard** (`073c5ac4`, merged):
- **Kill-tick barrel hold** (the real fidelity fix): turret destinations now read
  per-object inside the combat pass, before the death batch. A tank whose target dies this
  tick keeps aiming this tick and idle-returns next — the old port snapped the barrel back
  one tick early on every kill in every match.
- Idle Units now hash mission **Guard(5)** instead of port-artifact "None" (gamemd's idle
  mission for ground vehicles); also unblocks S4's passive-acquire gate.
- `SNAPSHOT_VERSION` collision among three concurrent slices resolved (radiation→21, SC-2→22,
  S3→23); both goldens **re-measured from the combined tree**, not textually merged.

Cross-session bug surfaced (not S3's): the **radiation slice uses f64 in sim logic and hashes
raw f64 bits into the lockstep hash** — no-float violation + desync risk; flagged for its owner.
Verb-layer trap documented: `is_busy`/`override_mission` still treat "None" as idle, invalidated
by the Guard change; must be re-derived before S5.

Open: **S4** handed off (needs live Ghidra) — damage-fire particle RNG position
(lockstep-critical), death early-returns, health smoothing (render-only), passive-acquire
shadow (missions {2,10,5}, 45-frame timer; needs OpportunityFire/CanPassiveAcquire/CanRetaliate
INI parsing), iron-curtain/temporal timer assertions. Two S3 YELLOW gates carried (kill-tick
composition spot-check, idle→Guard assigner trace). Separate punt: radiation f64→fixed-point.

---

## Gadget / Dialog-control UI substrate

A new study + first-third implementation of the two parallel UI frameworks.

- **Two frameworks:** Framework A (GadgetClass retained-mode tree — sidebar/radar/tabs) was
  new ground; Framework B (Win32 owner-draw shell dialogs) already had a 2026-05-31 study +
  shipped `ui::shell` Slices 0–5, folded in as a delta.
- **Behavior contract:** sticky capture, smallest-area-wins hit-testing, fire-on-release-inside
  + drag-off cancel, half-open rects, dialog-over-game coexistence; full 33-slot vtable map +
  derived-class census (active-YR vs TS-dormant via retail byte-scan).
- **30 claims re-verified in Ghidra: 27 VERIFIED, 3 WRONG.** Census isn't purely CRT-static
  (extra static ShapeButtons; chat TextLabels heap-built per message); ShapeButton `Set_Shape`
  is an appended 35th vtable slot; a "queued-event processor" is actually the superweapon
  screen-flash machine — commands enter sim via Queue_AI → DoList → `EventClass::Execute` in
  `Main_Tick`, AFTER render and AFTER `LogicClass::Update` (this pins the Rust UI→sim seam).
- **Top DRIFT:** sidebar fired on mouse-DOWN; gamemd fires on RELEASE-inside (every click,
  every match). Plus missing tooltips, mixed rect edges, no sticky-capture/hold-repeat, no
  chat-label surface.
- **Task-29 binary spot-checks caught 3 real bugs:** scroll buttons use 3-frame R-UP/R-DN
  (the "5 frames" guess was wrong, pressed art unreachable); scroll clicks play GUITabSound
  even when clamped; repair/sell play GUIMainButtonSound.

Implemented (branch merged to `dev` @ `347b5f9c`, 28 commits): Slice A0 (`ui::gadget` core
~1,570 lines), A1 (fire-on-release), A4 (1000ms tooltips), A5 (14-slot chat messages,
replaced egui banner), D-B3 (Esc), R1 (deleted `in_game_hud.rs`). Study doc + 29-task plan.
(Git confirms A2/A3/A6 landed in later sessions.)

Open / fidelity debt: tooltip box art, scroll-button placement (blocked on a user policy
call), cameo tooltip format string, B-track Slices 4/5b, keyboard routing, movies/campaign
off egui.

---

## Project / workflow reflection

A mentoring/repo-health session benchmarked the program and ran a six-auditor scan.

Measured stats: ~277k lines Rust, ~4,000 tests, ~2,330 research docs, ~10 weeks
(first commit Mar 29), 409 plans, ~300 commits/2 wks on `dev`.

Defects / risks (none fixed this session):
- **Desync vector (fix first):** `app_building_anim.rs:183` draws from the lockstep scenario
  RNG **per render frame, not per tick** → replays/MP diverge once a building catches fire.
  (Echoes the S3-flagged radiation-f64 hash issue: lockstep-RNG hygiene is the live theme.)
- **CI effectively dead:** triggers only on `pull_request` (work lands on `dev`), and a fresh
  checkout can't compile — `ini/` is gitignored but `skirmish_modes.rs` does `include_str!`
  on it.
- Missing `[profile.release]` (overflow-checks off in release); 3 runtime float LUTs are the
  only cross-platform determinism hole (freeze to compile-time tables); zero tests on
  engineer-capture / aircraft missions / SW charge timers; idle scatter disabled via a
  comment block in `advance_tick`.
- Fundamentals passed: sim-layering invariant holds; no wall-clock/external RNG in sim;
  all sim unwraps guarded.

Verdict: **~60% complete (gut feel)** — sim feel ~80%, UI ~70%, roster ~55% (uncertain),
"someone to play against" ~10% = the gate; two-human LAN is the shorter path than AI. Code
quality judged above typical professional; the remaining distance is in *proving* (network
play, perf at the 20k target). Top advice: back up `docs/research` (biggest risk, no own
repo); build a **golden-trace harness** (capture gamemd observable outputs per tick, assert
in-engine) since the only ground-truth gate today is the user's eyeballs.

Artifact: `docs/plans/2026-06-12-repo-improvement-scan.md`.

---

## Load-bearing discoveries

1. **Coordinate-frame fog-bounds bug** — fog seeded from raw `[Map] Size=` left every real
   map's player base permanently shrouded; the canonical iso-array frame fixes it. Caught by
   adversarial review before merge.
2. **Lockstep-RNG hygiene is a recurring live hazard** — the per-render-frame building-fire
   RNG draw (desync vector) and the radiation slice hashing raw f64 bits both feed the world
   hash and diverge replays/MP. Two independent instances in one burst.
3. **Per-cell radiation runtime was entirely missing** and is now implemented with a fully
   decoded native contract — fires every Desolator deploy.
4. **Kill-tick barrel hold (S3)** — the old port snapped turrets back one tick early on every
   kill in every match; fixed by reading turret destinations per-object before the death batch.
5. **UI commands enter sim after render and after LogicClass::Update** (Queue_AI → DoList →
   `EventClass::Execute` in `Main_Tick`) — pins the Rust UI→sim seam; plus sidebar must fire
   on release-inside, not mouse-down.
6. **Project is ~60% complete and the gate is an opponent, not more engine** — two-human LAN
   over AI; a golden-trace harness is the missing ground-truth gate.

---

## Open / unresolved from the burst

- **Radiation: f64→fixed-point** (lockstep desync risk) and the **green-glow render layer**
  (visible every Desolator deploy, needs dynamic-light infra).
- **Building-fire RNG desync** (`app_building_anim.rs:183`) — offered, not fixed.
- **`reveal_by_height` re-enable** — units currently see over cliffs (biggest single visible win).
- **Crowd-jam invented constants** — suspected drift, no binary/INI citation; Ghidra-prove
  absence then remove.
- **Map-INI second pass (RC-1 BLOCKED)** — allocate-vs-override-only and empty-value handling
  (`INIClass::Load`) unverified; current merge matches neither gamemd branch.
- **S4 ladder slice** — needs live Ghidra (damage-fire particle RNG position, passive-acquire
  shadow + its INI keys, iron-curtain/temporal timers); two S3 YELLOW gates carried.
- **CI + reproducibility debt** — dead CI on `dev`, fresh checkout won't compile (gitignored
  `ini/` + `include_str!`), missing `[profile.release]`, runtime float LUTs.
- **`docs/research` has no backup** — flagged as the single biggest project risk, deferred again.
- **ScenarioClass substrate anchor doc** — research-index still lacks a synthesized one.
- Verb-layer "None"-as-idle re-derive before S5; verify-doc passes for GSCREEN and stale
  RNG_SYSTEM / RULESCLASS tables.
