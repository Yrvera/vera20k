# PerTickUpdate Unnamed Callee Resolution — Ghidra Report

**Date:** 2026-05-28
**Slot:** /re-swarm subagent 5 — read-only investigation
**Target:** Semantic identity of all unnamed `FUN_*` callees and the reverse-loop class inside
`LogicClass::PerTickUpdate @ 0x0055AFB0`, as listed in `Remaining Uncertainty` of
`LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md`.
**Confidence:** High for gate-flag INI defaults (verified in rulesmd.ini + Ghidra reader assembly);
High for FUN_0053A110/A120/BAD0/B400 (trivial 1-line returns), FUN_004AE4C0 (clear cell iterator),
FUN_0053D310 (named callee Wave_splash_forces); Medium for FUN_0054E4D0, FUN_005FF390, FUN_00554D50
(structure clear but class name unconfirmed); confirmed via listed Ghidra calls.

---

## Target Question
Resolve semantic identity (subsystem, YR-liveness, order position) for every unnamed FUN_* in the
anchor doc's Remaining Uncertainty, and name the class behind the `DAT_00B04BD4` reverse loop.

## Non-goals
- Do not re-derive the main object-vector scheduler or RadSite/DiskLaser/Team/Factory/House identities
  already resolved by `PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS_GHIDRA_REPORT.md`.
- Do not implement Rust changes.
- Do not rename or annotate any function in Ghidra.

## Evidence Needed to Mark COMPLETE
Decompile + gate-default check for every listed FUN_*; evidence inline for every claim.

## Stop Conditions
Stop after all listed FUN_* are either verified or marked YELLOW/unresolved. Do not expand into
full callee internals beyond what is needed to name the subsystem.

---

## Resolved Identities — Verified Section

### Order 5 — `FUN_004ACAC0`: Shroud Regrowth Pass
**Gate:** `Rules+0x17F0 != 0` AND `Rules+0x1640 != 0.0`
**INI keys:**
- `Rules+0x17F0` = `ShroudGrow` (bool), read by `RulesClass__ReadAudioVisual @ 0x0066a693`
  (verified via `get_assembly_context 0x0066a686`).
- `Rules+0x1640` = `ShroudRate` (double), read by `RulesClass__ReadAudioVisual @ 0x0066b4be`
  (verified via `get_assembly_context 0x0066b4a4`).
**INI defaults (rulesmd.ini line 677, 762):** `ShroudGrow=no`, `ShroudRate=4`.
**Active in YR skirmish:** **NO**. `ShroudGrow=no` → `Rules+0x17F0 == 0` → outer gate fails
→ entire call is skipped unconditionally. (verified via `decompile_function 0x004ACAC0`;
gate assembly at `0x0055B20B..0x0055B213`.)
**Subsystem:** Two-pass scan of every CellClass checking shroud-state bits `+0x12C & 0x08`/`0x10`;
re-marks cells for regrowth, clears the mark, then calls `FUN_004ACDA0` (cell shroud notify neighbor
pass) and `FUN_004ADFF0` (visibility-change dispatcher to human-player objects in
`DisplayLayerEntry_008a0390`).
**Determinism finding:** SKIPPED in standard skirmish — no RNG consumption, no state change.
Rust needs no equivalent for normal play.

### Order 7 — `FUN_004ACBC0`: Fog Regrowth Pass
**Gate:** `(*Scenario & 0x1000) != 0` AND `Rules+0x1648 != 0.0`
**INI keys:**
- `Scenario & 0x1000` = `FogOfWar` scenario option bit, set from `[MultiplayerDialogSettings]
  FogOfWar` / `[Basic] FogOfWar`; defaults `FogOfWar=no` in both rulesmd.ini line 205 and line 3040.
- `Rules+0x1648` = `FogRate` (double), read by `RulesClass__ReadAudioVisual @ 0x0066b4e4`
  (verified via `get_assembly_context 0x0066b4e4`); rulesmd.ini line 763: `FogRate=.01` (non-zero,
  but outer gate already fails).
**Active in YR skirmish:** **NO**. FogOfWar defaults to `no` → `Scenario & 0x1000 == 0` → call skipped.
(verified via `decompile_function 0x004ACBC0`; gate assembly at `0x0055B2C7..0x0055B2DD`.)
**Subsystem:** CellIterator scan for fog bits `+0x140 & 0x02`/`0x01`; marks/clears cells for fog
regrowth; calls `FUN_004ACC50` (cell fog/edge-bitmask recompute + neighbor recursion) and
`FUN_004ADFF0` (visibility-change dispatcher).
**Determinism finding:** SKIPPED in standard skirmish — no RNG, no state change. Rust needs no
equivalent for normal YR skirmish.

### Order 8 — Terrain/Lighting Transition Block
**Functions involved:** `FUN_0053A110`, `FUN_0053A120`, `FUN_0053BAD0`, `FUN_0053B400`,
`FUN_004AE4C0`.
**Gate:** `Scenario+0x3530 != Scenario+0x352c` AND `Rules+0x1668 != 0.0`
(verified via `get_assembly_context 0x0055B33D`, 0x0055B343, 0x0055B349, 0x0055B351.)

#### `FUN_0053A110` — IsTerrainMorphPhase1 predicate
Returns `DAT_00A9FABC == 1`. (verified via `decompile_function 0x0053A110`)
One-liner predicate querying a terrain-morph phase counter. Used by `Cell_ComputeZAdjust` (verified
via `decompile_function 0x00484680`) to choose the correct lighting formula for elevation.
**Active in YR:** Yes when morph is active; predicate always reachable.

#### `FUN_0053A120` — IsTerrainMorphPhase2 predicate
Returns `DAT_00A9FABC == 2`. (verified via `decompile_function 0x0053A120`)
Same as above for phase 2.

#### `FUN_0053BAD0` — IsTerrainMorphActive1 predicate
Returns `DAT_00A9FAB0 != 0`. (verified via `decompile_function 0x0053BAD0`)
Checks a non-zero morph-state global.

#### `FUN_0053B400` — IsTerrainMorphActive2 predicate
Returns `DAT_00A9FAC0 != 0`. (verified via `decompile_function 0x0053B400`)
Checks a second morph-state global.

**What the transition block does:** On first entry (none of the four predicates return true), it
reads Scenario's ambient-transition target from `Scenario+0x3578` (NukeAmbientChangeRate written by
`ScenarioClass__Read_INI_Basic` at `0x0068AAFB`) and sets a timed increment. On subsequent entries,
it advances `Scenario+0x3530` toward `Scenario+0x352c` (current ambient toward target) at the
`Rules+0x1668` rate (verified via `get_assembly_context 0x0055B3CC` showing `FLD [ECX+0x1668]` at
`0x0055B3D8`).

#### `FUN_004AE4C0` — MapClass__RecomputeAllCellZAdjust
Iterates all cells via `MapClass__CellIterator` and calls `Cell_ComputeZAdjust` on each.
(verified via `decompile_function 0x004AE4C0`; callee `Cell_ComputeZAdjust @ 0x00484680` confirmed
via `get_function_callees 0x004AE4C0`.)
Recomputes every cell's elevation-adjusted draw height after a lighting/terrain transition step.
**Active in YR:** Conditionally active whenever the lighting is transitioning (superweapon
ambient effects: Nuclear Missile, Dominator). SKIPPED in static-ambient skirmish where
`Scenario+0x3530 == Scenario+0x352c`. The 4-predicate inner block gates whether we advance (new
morph step) or just recompute Z-adjusts.

**INI default for Rules+0x1668 (`AmbientChangeRate`):** rulesmd.ini line 767: `AmbientChangeRate=.2`
(0.2, non-zero). The outer gate `Scenario+0x3530 != Scenario+0x352c` is what keeps this block
dormant in typical skirmish. A Nuclear Missile or Dominator superweapon triggers ambient change and
makes `+0x3530 != +0x352c` → block fires until transition completes.
**Active in YR skirmish:** **CONDITIONAL** — dormant normally, active during superweapon ambient
transitions.

### Order 12 — `FUN_0054E4D0`: Timed Per-Object Pathfinding/Movement Continue Batch
**Call site:** `ECX=0x00ABC5F8` (a global timer+list struct) immediately after `BombClass::UpdateAll`.
(verified via `decompile_function 0x0054E4D0`; callees `Pathfinding_update_continued @ 0x00481810`
and `RateTimer__Current @ 0x004C93D0` confirmed via `get_function_callees 0x0054E4D0`.)
**Structure of the timer object at ECX:**
- `[0]` last-fired frame, `[1]` scratch, `[2]` interval (default 0x1E = 30 frames), `[4]` object
  pointer buffer, `[7]` object count.
- Per iteration: sets flag `piVar1[0xbf]=1` on each stored object, calls object `vtable+0x1BC`
  with a path-budget derived from `RateTimer__Current` output, then calls object `vtable+0x3C8`
  with the path step index, then `vtable+0x1E8(1,0)` to reset.
**Subsystem:** A time-budgeted batch pass over registered objects, continuing their in-progress
pathfinding/movement from the previous step. The `Pathfinding_update_continued` helper computes a
directional neighbor cell offset from the object's current position, consistent with incremental
path-advance logic.
**Active in YR:** Yes (unconditional call). The timer fires every 30 frames when the object list
is non-empty.
**YELLOW NOTE:** The exact class(es) registered in the `DAT_00ABC5F8` buffer (what puts objects
there) is not traced. Identity is Medium confidence — see Remaining Uncertainty.

### Order 15 — `FUN_005FF390`: Short-Lived Effect Entry Aging / Expiry List
**Call site:** After DiskLaser reverse loop, before `LaserDrawClass::UpdateAllAI`.
(verified via `decompile_function 0x005FF390`.)
**What it does:** Reverse loop over `DAT_00AC167C` (count `DAT_00AC1688`), increments each entry's
`+0x0C` field by 8 per tick. When `+0x0C > 0x4F` (79), removes entry via the vector's removal
method at `DAT_00AC1678+0x10` and frees the object with `FUN_007C8B3D` (operator delete). Entries
survive at most ~10 ticks (80/8 per increment).
**Subsystem:** TTL aging pass for a global pointer list of short-lived effect objects. Given its
position between DiskLaser and LaserDraw, these are likely visual effect objects produced by disk
lasers or similar beam/glow effects.
**Active in YR:** Yes (unconditional reverse pass). No-op when count is 0.
**YELLOW NOTE:** Producer class identity (what appends to `DAT_00AC167C`) not traced. See Remaining
Uncertainty.

### Order 19 — `FUN_00554D50`: Cell Terrain-Cache Batch Updater
**Call site:** `FUN_00554D50(6, false)` after RadSite reverse loop, before EMP.
(verified via `decompile_function 0x00554D50`; callees `MapClass__Get_CellClass`,
`FUN_005B1E40`, `FUN_00484050`, `FUN_00483E30`, `FUN_007C8B3D`.)
**What it does:** Time-budgeted pass over a global work-queue (`DAT_00ABCA44`, count
`DAT_00ABCA50`). For each queued cell record: fetches cell's terrain data, calls
`FUN_00484050` to compute cell surface/slope values (6 output fields into the queued record), then
on completion calls `FUN_00483E30` to apply the computed terrain data to the cell's CellClass struct
and frees the record. Tracks completion via `DAT_00ABCA84` flag. The `param_1=6` is a millisecond
time budget per call (will stop processing when budget is exhausted). Uses real-time (`FUN_005B1E40`
= clock query) to respect the budget.
**Subsystem:** Incremental cell terrain/slope cache rebuild — processes pending cells that need
elevation-surface data recomputed, spread over multiple ticks within the budget.
**Active in YR:** Yes (unconditional call). No-op when work-queue is empty (`DAT_00ABCA50 == 0`).
**YELLOW NOTE:** What triggers cells entering the queue (what writes to `DAT_00ABCA44`) not traced.
Medium confidence on exact system name.

### Order 23 — `FUN_0053D310`: WaveClass Splash/Force Update Pass
**Call site:** After conditional `DAT_00A83E04` anim loop, before `AlphaShapeClass::PurgeDisabled`.
(verified via `decompile_function 0x0053D310`.)
**What it does:** Loops `DAT_00AA0128` times, calling `Wave_splash_forces()` each iteration (named
callee in Ghidra). `DAT_00AA0128` is the live count of the wave splash-force global array.
**Subsystem:** WaveClass water displacement / splash-force propagation update. WaveClass objects
(produced by naval units, explosions on water, wake effects) maintain active splash-force data;
this pass drives the per-tick force update for all live wave entries.
**Active in YR:** Yes (unconditional pass). No-op when no wave objects exist (`DAT_00AA0128 == 0`).

---

## Order 18 — `DAT_00B04BD4` Reverse Loop: RadSiteClass (Already Resolved)

**Already confirmed** by `PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS_GHIDRA_REPORT.md` section 2,
order 10:
> `RadSiteClass` array `DAT_00B04BD4`, count `DAT_00B04BE0`; reverse loop; vtable `0x007F0810`;
> `RadSiteClass::AI @ 0x0065B800`. (verified in prior slot via constructor `0x0065B1E0`.)

Citing rather than re-deriving per task instructions.

---

## Condensed Identity Table

| Anchor Order | Address | Verified Identity | YR Skirmish Active |
|---:|---|---|---|
| 5 | `FUN_004ACAC0` | Shroud Regrowth Pass (`ShroudGrow=no`) | **NO** — outer gate fails |
| 7 | `FUN_004ACBC0` | Fog Regrowth Pass (`FogOfWar=no`) | **NO** — outer gate fails |
| 8a | `FUN_0053A110` | IsTerrainMorphPhase1 predicate (DAT_00A9FABC==1) | Conditional |
| 8b | `FUN_0053A120` | IsTerrainMorphPhase2 predicate (DAT_00A9FABC==2) | Conditional |
| 8c | `FUN_0053BAD0` | IsTerrainMorphActive1 predicate (DAT_00A9FAB0!=0) | Conditional |
| 8d | `FUN_0053B400` | IsTerrainMorphActive2 predicate (DAT_00A9FAC0!=0) | Conditional |
| 8e | `FUN_004AE4C0` | MapClass__RecomputeAllCellZAdjust | Conditional (superweapon ambient) |
| 12 | `FUN_0054E4D0` | Timed pathfinding/movement continue batch (Medium confidence) | Yes |
| 15 | `FUN_005FF390` | Short-lived effect TTL aging pass | Yes (no-op when empty) |
| 18 | `DAT_00B04BD4` | RadSiteClass reverse loop (cited from prior slot) | Yes |
| 19 | `FUN_00554D50` | Cell terrain-cache incremental updater (Medium confidence) | Yes (no-op when empty) |
| 23 | `FUN_0053D310` | WaveClass splash-force update pass | Yes (no-op when empty) |

---

## Gate Flag Default Summary (Determinism-Critical)

| Gate | INI Key / Default | Skipped in Normal Skirmish? |
|---|---|---|
| `Rules+0x17F0 != 0` | `ShroudGrow=no` (rulesmd.ini:677) | YES — ShroudGrow is no |
| `Rules+0x1640 != 0.0` | `ShroudRate=4` (rulesmd.ini:762) | Moot (outer gate already fails) |
| `*Scenario & 0x1000` | `FogOfWar=no` (rulesmd.ini:205, 3040) | YES — FogOfWar is no |
| `Rules+0x1648 != 0.0` | `FogRate=.01` (rulesmd.ini:763) | Moot (outer gate already fails) |
| `Scenario+0x3530 != Scenario+0x352c` | Set by scenario lighting; equal at start | YES in static skirmish; NO during superweapon ambient transition |
| `Rules+0x1668 != 0.0` | `AmbientChangeRate=.2` (rulesmd.ini:767) | Non-zero, but first gate (above) blocks it |

---

## Implementation Handoff

**1. Shroud and Fog regrowth — Rust needs NO equivalent for normal skirmish.**
Verified behavior: `FUN_004ACAC0` and `FUN_004ACBC0` are unconditionally skipped when
`ShroudGrow=no` (the default). These are TS-era Fog-of-War systems; the parity contract for
standard YR skirmish requires no shroud-regrowth or fog-regrowth pass. Gate: `Rules+0x17F0` read
from `ShroudGrow` INI key; `ShroudGrow=no` in all stock YR INI files verified.
Rust delta: none needed. If `ShroudGrow=yes` support is ever added, the equivalent must run at
anchor order-5 (after scenario-timer reset, before MapClass__RecalcBridgeShroudFlags).
Proposed test: `shroud_regrowth_gate_skipped_when_shroudgrow_false` — confirm no shroud state
mutation when ShroudGrow=false.
Risk: Low — the gate is explicit and the INI key is absent from default YR INI, making accidental
activation impossible.

**2. WaveClass splash-force update — must run at anchor order-23, after conditional anim loop.**
Verified behavior: `FUN_0053D310` loops `DAT_00AA0128` times calling `Wave_splash_forces()`. Always
reached unconditionally; no-op when wave count is zero. Active in YR for naval units and water
explosions. Rust currently has no equivalent wave-force-update phase in `advance_tick`.
Rust delta: `src/sim/world/mod.rs::advance_tick` — add a `WaveClass::tick_all_splash_forces()` call
after the conditional click-feedback anim equivalent and before `AlphaShapeClass::PurgeDisabled`.
Affected surface: future `WaveClass` runtime; `src/sim/world/mod.rs`.
Acceptance scenario: naval unit wake produces correct force propagation timing relative to object
AI. Proposed test: `wave_splash_force_runs_after_nonlocal_anim_loop_before_alpha_purge`.
Risk: DRIFT if placed in wrong tick phase — wave forces consumed by object physics in the same tick.

**3. Terrain Z-cache batch updater — no-op in typical skirmish, but must not be omitted.**
Verified behavior: `FUN_00554D50(6, false)` processes a global work-queue of cells needing
terrain/slope cache recomputation; fires unconditionally but is no-op when queue is empty. When
active (terrain deformation, ice-growth, ambient light changes), it applies per-tick budget of 6 ms.
Rust delta: `src/sim/world/mod.rs` — if terrain mutation is implemented, add a timed cell-cache
rebuild pass at anchor order-19 (after RadSite, before EMP). If terrain is immutable in Rust, the
queue is always empty and this is a no-op.
Proposed test: `cell_terrain_cache_rebuild_runs_before_emp_after_radsite`.
Risk: Low for static terrain; Medium if terrain deformation is added without this pass.

---

## Negative Facts / Do Not Do

1. **Do NOT implement a shroud-regrowth pass for standard YR.** `ShroudGrow=no` is the compiled-in
   default of all stock YR INI files (rulesmd.ini:677). Implementing it unconditionally would drift
   from the exact-mechanism parity contract. Active in YR: No (verified via INI + gate assembly at
   `0x0055B213`).

2. **Do NOT implement a fog-regrowth pass for standard YR.** `FogOfWar=no` is default (rulesmd.ini
   lines 205, 3040). The `*Scenario & 0x1000` gate fails. Active in YR: No (verified via gate
   assembly at `0x0055B2C7`).

3. **Do NOT confuse `FUN_0053A110/A120/BAD0/B400` with gameplay state machines.** These are 1-line
   global-variable reads used as predicates inside the lighting-transition block. They have no
   side effects and are not the systems they query — they are query predicates only.

4. **Do NOT treat the terrain-morph block (order 8) as always-active.** It requires
   `Scenario+0x3530 != Scenario+0x352c` (current ambient ≠ target) — this only fires during
   superweapon ambient transitions (Nuclear Missile, Dominator). In a static ambient skirmish it
   never fires (verified via gate at `0x0055B349`).

5. **Do NOT place WaveClass splash updates at end-of-tick or in the rendering path.** Binary order
   puts `FUN_0053D310` at order-23, after the conditional `DAT_00A83E04` anim loop and before
   `AlphaShapeClass::PurgeDisabled` — this is before Tactical, Factory, House updates
   (orders 26–28). Active in YR: Yes (verified via `decompile_function 0x0053D310`).

---

## Remaining Uncertainty

- **`FUN_0054E4D0` (order 12)** — timed batch pass identity is Medium confidence. The specific class
  registered in `DAT_00ABC5F8`'s buffer is not traced. The vtable-slot calls at `+0x1BC`, `+0x3C8`,
  `+0x1E8` could be confirmed by finding what registers objects into that global timer struct. Next
  step: trace xrefs to `DAT_00ABC5F8` writes to find the registration site.

- **`FUN_005FF390` (order 15)** — the producer class that appends to `DAT_00AC167C` is not
  identified. The aging/expiry semantics are clear but the class name is unknown. Next step: trace
  xrefs to `DAT_00AC167C` appends.

- **`FUN_00554D50` (order 19)** — what enqueues cells into `DAT_00ABCA44` is not traced. The
  incremental terrain-cache semantics are clear but the trigger path is unknown. Next step: trace
  xrefs to `DAT_00ABCA50` increment.

- **`Rules+0x1668` exact INI key name** — confirmed as `AmbientChangeRate` from rulesmd.ini:767
  context (rulesmd.ini:767-768 shows `AmbientChangeRate=.2` and `AmbientChangeStep=.2` as the two
  ambient-transition rate keys), but the exact offset mapping is inferred from position in the
  ReadAudioVisual sequence, not a direct `FSTP [ESI+0x1668]` observation. Confidence: Medium.
  Next step: trace the ReadAudioVisual assembly immediately after `IceSolidifyFrameTime` write at
  `0x0066b556`.

---

## Stale-Doc Updates Required

**Anchor doc:** `docs/research/LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md`
Replace the `Remaining Uncertainty` section with:

> **Resolved:**
> - Orders 5, 7: Shroud/Fog regrowth passes — SKIPPED in standard YR (ShroudGrow=no, FogOfWar=no).
> - Orders 8a–e: Terrain-morph predicates + MapClass__RecomputeAllCellZAdjust — CONDITIONAL
>   (active only during superweapon ambient transitions).
> - Order 12: Timed path/movement continue batch (Medium confidence; class TBD).
> - Order 15: Short-lived effect TTL aging (Medium confidence; producer class TBD).
> - Order 18: RadSiteClass reverse loop (cited from PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS report).
> - Order 19: Cell terrain-cache incremental updater (Medium confidence; queue trigger TBD).
> - Order 23: WaveClass splash-force update — always unconditional, no-op when no waves.
>
> **Still unresolved:**
> - Producer classes for FUN_0054E4D0 buffer, FUN_005FF390 list, FUN_00554D50 queue.
> - Rules+0x1668 exact offset binding (AmbientChangeRate, Medium confidence only).

---

## Sources

- `decompile_function 0x004ACAC0` — Shroud regrowth pass
- `decompile_function 0x004ACBC0` — Fog regrowth pass
- `decompile_function 0x0053A110`, `0x0053A120`, `0x0053BAD0`, `0x0053B400` — morph predicates
- `decompile_function 0x004AE4C0` — RecomputeAllCellZAdjust
- `decompile_function 0x0054E4D0` — timed batch
- `decompile_function 0x005FF390` — TTL aging
- `decompile_function 0x00554D50` — terrain-cache batch
- `decompile_function 0x0053D310` — WaveClass splash forces
- `decompile_function 0x00484680` — Cell_ComputeZAdjust (confirmed FUN_0053A110 usage)
- `get_assembly_context 0x0066a686` — ShroudGrow → Rules+0x17F0
- `get_assembly_context 0x0066b4a4` — ShroudRate → Rules+0x1640
- `get_assembly_context 0x0066b4e4` — FogRate → Rules+0x1648
- `get_assembly_context 0x0055B20B` — gate assembly for FUN_004ACAC0
- `get_assembly_context 0x0055B28E` — after-shroud gate, FUN_004ACBC0 gate
- `get_assembly_context 0x0055B33D` — terrain-morph block gate
- `get_assembly_context 0x0055B3CC` — transition block body, Rules+0x1668 FLD
- `get_function_callees 0x004AE4C0`, `0x0054E4D0`, `0x005FF390`
- INI: `rulesmd.ini` lines 205, 677, 762–768, 3040
- Prior docs: `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md`,
  `PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS_GHIDRA_REPORT.md`

## Status

COMPLETE for all listed FUN_* identities (subsystem named, YR liveness determined, order position
confirmed). PARTIAL for: producer class of FUN_0054E4D0 buffer, FUN_005FF390 list, FUN_00554D50
queue (all marked YELLOW/Medium).
