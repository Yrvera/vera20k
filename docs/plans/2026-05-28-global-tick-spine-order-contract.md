# Global Tick-Spine Order Contract (Contract #4)

Date: 2026-05-28
Verified native order: `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md`
(re-verified from scratch this session via `decompile_function 0x0055AFB0`).

## Parity-bar framing (decisive)

Per CLAUDE.md, **internals are ours to design; only OUTPUT must match.** Reorder
`advance_tick` ONLY where the current order produces a different *observable* result
than gamemd AND current Rust gets it wrong AND the change is in scope. This is the
Phase-3 gate. A wholesale "mirror the binary sequence" rewrite is explicitly NOT the
goal.

## Verified native order (gamemd `0x0055AFB0`), classified

| Native stage | In Rust? | Scope verdict |
|---|---|---|
| Scenario/cell-action timers (1-4) | partial | OUT — scenario-trigger machinery, not a tick-order parity item here |
| Shroud regrowth (5), Fog regrowth (7) | n/a | OUT — **SKIPPED in YR** (ShroudGrow=no, FogOfWar=no) |
| Bridge-shroud recalc (6) | yes-ish | OUT — visual recalc, render-adjacent |
| Terrain-morph transition block (8) | n/a | OUT — gated, not stock-skirmish gameplay |
| **Tiberium growth → spread (9-10)** | yes, but LATE | **IN — DRIFT candidate A** |
| Bombs / BombClass (11) | NO | OUT — a *new system*, not a reorder |
| TeamClass AI scratch list (13) | no equiv | OUT — AI excluded at this stage (project decision) |
| Disk lasers (14), wave splash (23), RadSite (18) | partial | OUT — effect/visual passes, low/no gameplay-output coupling |
| Laser draw (16) | render | OUT — render |
| **LightningStorm process (17) + EMPulse (20)** | yes, but AFTER movement | **IN — DRIFT candidate B** |
| Main live object vector (21) | Contract 1 primitive | foundational (done); consumers not migrated |
| Conditional non-local loop (22) | n/a | OUT — only if game mode ∉ {0,5}; stock skirmish skips |
| Alpha-shape purge (24), crate regen (25) | partial | OUT — crate regen is a minor timer, low coupling |
| Tactical (26) | no sim equiv | OUT — UI/render |
| Factories (27) → Houses (28) | Contract 5 | deferred to Contract 5 |
| Last-ref-object (29) | no | OUT — UI/selection |
| Frame-counter late increment | DONE | already fixed (Native Frame/Tick contract) |

## Phase-3 verdict: the wholesale reorder is NOT justified

Most native stages are out of scope (skipped-in-YR, AI/teams, new-system BombClass,
render/UI), output-equivalent, already-done (frame counter), or deferred to
Contract 5 (factory/house). Only **two** genuine in-scope ordering DRIFTs remain,
and **both are risky with marginal-to-occasional visibility**:

### DRIFT A — tiberium growth/spread early vs late
- gamemd: growth/spread BEFORE the object vector (so combat craters this tick affect
  growth NEXT tick — growth reads pre-crater density).
- Rust: ore growth runs LATE (Phase 7, after combat), and this is a **deliberate
  decision** — `world/mod.rs:1675-1679` "Ledger #6: crater-path Reduce_Tiberium must
  land before ore-growth reads density." Rust growth reads POST-crater density.
- So Rust and gamemd differ by one tick in the crater↔growth interaction, but in
  OPPOSITE directions, and Rust's choice is intentional and documented.
- Visible effect: ≤1-tick shift in ore density / miner harvest amount. Marginal.
- Risk: HIGH — entangled with smudge/combat/occupancy (the research explicitly warns
  to resolve these first) and directly conflicts with Ledger #6. Fixing means
  re-litigating a deliberate parity decision.

### DRIFT B — LightningStorm/EMP before vs after movement
- gamemd: LightningStorm + EMPulse BEFORE the object vector (before movement AND
  combat), unconditional each tick.
- Rust: `tick_superweapons` (Phase 4.5, `world/mod.rs:1485`) runs AFTER movement
  (Phase 1-2) but BEFORE combat. So vs combat Rust matches; vs movement it drifts —
  an EMP'd/struck unit gets one extra movement tick in Rust.
- Visible effect: 1-tick — EMP'd unit moves one extra step; lightning strikes the
  post-move position. OCCASIONAL (only when EMP/Lightning Storm superweapons active).
- Risk: MEDIUM — `tick_superweapons` is a monolithic phase covering many superweapons
  (IC, force shield, chrono, genetic, nuke, weather/lightning, EMP). A correct fix
  must split out ONLY lightning-damage + EMP-application to before movement without
  disturbing the other superweapons' ordering.

## Recommendation

Neither reorder is a clean low-risk win. Recommended:
- **DEFER DRIFT A** (ore growth): high risk, conflicts with deliberate Ledger #6,
  marginal visibility. Surface as a known DRIFT; do not reorder without re-deciding
  Ledger #6 with the user.
- **DRIFT B** is the better implementation candidate if any: surgically split
  lightning-damage + EMP-application to run before ground movement, leaving the rest
  of `tick_superweapons` in place. Needs its own premise proof (construct an EMP'd
  unit that moves one extra tick) + acceptance test + determinism check.

Final call (fix-now which / defer) is the user's per the parity bar — the agent
surfaces; the user prioritizes.

## Resolution (2026-05-28): DRIFT A DEFERRED, DRIFT B DEFERRED

After feasibility investigation, both in-scope DRIFTs are deferred as documented
sub-perceptible items — implementing either would harm more than it helps:

**DRIFT A (ore growth) — DEFER.** The ore-growth driver reads/writes ore density
directly against the live `overlay_grid` throughout (~1300 lines, re-reads its own
writes at `ore_growth.rs:554`). Two ways to match gamemd's growth-reads-pre-crater
ordering, both rejected:
- *Phase-move* (growth before combat): ore-growth consumes the shared RNG, so moving
  it reorders the ENTIRE global RNG stream — breaks RNG-dependent tests (Phase-7
  blocker), invalidates replays/saves, changes all downstream random outcomes. And it
  buys ZERO observable convergence: Rust's PRNG ≠ gamemd's PRNG, so which cells grow
  already cannot match gamemd regardless of phase order. Net regression.
- *Snapshot* (growth reads pre-crater density copy): theoretically most parity-correct,
  but requires threading a density snapshot through a deeply density-coupled system
  while growth still writes live density — invasive and bug-prone, for a
  sub-perceptible ≤1-tick single-cell edge case (a cell both cratered and growing the
  same tick). Cost ≫ value.
- Current Rust (Ledger #6, crater-before-growth) is a deliberate, harmless ≤1-tick
  micro-DRIFT. Per the parity bar (internals are ours; this micro-interaction's output
  is swamped by unavoidable PRNG divergence), leave it and surface the DRIFT here.

**DRIFT B (EMP/lightning before movement) — DEFER.** Requires splitting the monolithic
`tick_superweapons` phase; occasional visibility (only when those superweapons fire).
Lower priority than the implemented contracts; revisit if EMP/lightning timing becomes
a reported issue.

## Phase status — analyzed; implementation deferred by design
- PHASE 1 (evidence): DONE — native order verified from scratch (`0x0055AFB0`).
- PHASE 2 (this doc): DONE — order classified, two in-scope DRIFTs isolated.
- PHASE 3 (premise): DONE — both DRIFTs real but sub-perceptible/risky; ore-growth's
  fixes either regress (phase-move) or cost ≫ value (snapshot). DEFERRED.
- PHASES 4-8: not executed — no implementation, by the Phase-3 conclusion above.
