# LogicClass PerTickUpdate Full Ordering Ladder - Ghidra Research Report

**Address(es):** `LogicClass::PerTickUpdate @ 0x0055AFB0`; active caller `Main_Tick @ 0x0055D360` call site `0x0055DC99..0x0055DC9E`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** top-level execution order inside active YR `LogicClass::PerTickUpdate`, including major loops/calls around the already-settled main `LogicClass+0x04/+0x10` object-vector loop and a Rust `Simulation::advance_tick` ordering comparison.  
**Non-Scope:** internal semantics of each callee, class-specific `vtable+0x5C` bodies, save/load reconstruction, replay restore, and the already-proven live-vector append/remove contract except as an ordering anchor.  
**Confidence:** High for top-level order and active caller; Medium for class names on anonymous globals where only prior Ghidra reports or constructor xrefs establish identity.  
**Active in YR:** Yes. `Main_Tick` loads `ECX=0x87F778` and calls `0x0055AFB0` at `0x0055DC99..0x0055DC9E`; Ghidra caller query reports `Main_Tick` as caller.

## 0. Investigation Contract

**Target question:** What is the active-YR top-level execution ladder inside `LogicClass::PerTickUpdate @ 0x0055AFB0`, and which pieces map or fail to map to current Rust `Simulation::advance_tick`?

**Non-goals:** Do not re-prove main live-vector append/remove semantics; do not exhaust every callee; do not implement Rust; do not rename or mutate Ghidra.

**Evidence needed to mark COMPLETE:** direct Ghidra decompile for `0x0055AFB0`; assembly/disassembly context for key call/loop ranges; caller proof from `Main_Tick`; current Rust source file/line evidence for `advance_tick`; prior verified reports only as class-name or subsystem support.

**Stop conditions:** stop at top-level ladder once every major call/loop in `0x0055AFB0` is placed in order, has an active-YR label, has an evidence range, and has a Rust mapping verdict; defer callee internals that require separate system context.

## 1. Overview

`LogicClass::PerTickUpdate` is a late main-tick service ladder, not a single equivalent of Rust's phased `advance_tick`. It runs scenario action/timer work, ore/lighting/bomb/team/laser/weather/radiation/EMP/object/animation/wave/alpha/crate/tactical/factory/house/refocus services in a fixed native order before `Main_Tick` increments `g_CurrentFrameCounter`.

The old statement that Rust phases are a "richer" non-comparable version is stale for parity purposes. The binary's top-level order is a byte/pixel-visible contract unless an implementation proves equivalence for every affected system.

## 2. Top-Level Ordering Ladder

| Order | Binary range / call | Major work | Active in YR | Current Rust mapping |
|---:|---|---|---|---|
| 1 | `0x0055AFB3..0x0055AFC6` | Increment `DAT_00ABCD40`; clear `DAT_00A83CDC`; set up scenario-action loop count from `DAT_008B40D8`. | Yes; active caller proof `0x0055DC99..0x0055DC9E`. | No direct mapped counter found; `advance_tick` commits `total_sim_ms`/`binary_frame` and `tick` LATE (end of tick, in `run_late_region` at `world/mod.rs:1397..1399`), matching the native late `g_CurrentFrameCounter` increment. (corrected 2026-05-29: re-anchored; verified via Read src/sim/world/mod.rs:1397) |
| 2 | `0x0055AFBD..0x0055B172` | Scenario/cell-action dispatch loop. Each pass calls `FUN_006E53A0` with gated action IDs: `+0x34BE`→`0x32`; `+0x34AA`→`0x1B`,`0x1C`,`0x24`,`0x25`; `+0x34AB`→`0x2D`,`0x2E`. Then the non-gated common IDs `0x0D`, `0x33`, and finally the timer-gated fallback `0x0E` (gated on `ScenarioClass+0x11E8/+0x11F0` timer). Loop counter `DAT_00A83CDC` increments until `< DAT_008B40D8`. (corrected 2026-05-29: gated action IDs added — `+0x34BE`→0x32 at push `0x0055B006`; `+0x34AA`→0x1B/0x1C/0x24/0x25 at pushes `0x0055B033/0x0055B051/0x0055B06F/0x0055B08C`; `+0x34AB`→0x2D/0x2E at pushes `0x0055B0BA/0x0055B0D8`; common 0x0D/0x33 at `0x0055B0F1/0x0055B10B`; fallback 0x0E at `0x0055B155` — verified via disassemble_function 0x0055AFB0) | Yes; loop is in active function; flags are reset later in same active path. | Not directly mapped in `advance_tick`; trigger/cell-action style dispatch appears absent or split elsewhere. |
| 3 | `0x0055B174..0x0055B1D8` | Scenario timer expiry handling for `ScenarioClass+0x47A/+0x47C`, then `FUN_004F42F0(2)` if timer expires. | Yes; active path uses `g_CurrentFrameCounter`. | No direct mapped pre-ore scenario timer stage found. |
| 4 | `0x0055B1D8..0x0055B200` | Clear scenario one-tick flags `+0x34AA`, `+0x34A9`, `+0x34AB`, `+0x34BE`. | Yes; unconditional after scenario loop/timer path. | No direct mapped scenario flag reset found. |
| 5 | `0x0055B200..0x0055B29A` | Rules-gated tiberium growth driver precursor: if `Rules+0x17F0` and `Rules+0x1640 != 0.0`, checks timer fields at scenario `+0x486/+0x488`, reloads using `Math__ftol`, then calls `FUN_004ACAC0`. | Yes, conditional on rule/timer gates. | Rust ore growth is much later in Phase 7 at `src/sim/world/mod.rs:1884..1924`; not same placement. (corrected 2026-05-29: re-anchored after Rust refactor; verified via Read src/sim/world/mod.rs:1884) |
| 6 | `0x0055B29A..0x0055B2AD` | Every `0x78` frames, call `MapClass::RecalcBridgeShroudFlags`. | Yes; condition is `g_CurrentFrameCounter % 120 == 0`. | Bridge work exists, but no verified same `PerTickUpdate` cadence/placement in this slot. |
| 7 | `0x0055B2B8..0x0055B33D` | TS-style fog/secondary tiberium branch: if `Scenario.SpecialFlags & 0x1000` and `Rules+0x1648 != 0.0`, reloads timer and calls `FUN_004ACBC0`. | Conditional; code is active but standard YR fog-of-war is normally off unless this special flag is set. | Do not map to default YR shroud behavior; Rust should not enable TS fog by this ladder alone. |
| 8 | `0x0055B33D..0x0055B4D7` | Tiberium spread/ambient transition precursor: compares scenario counters `+0x3530/+0x352C`, checks `Rules+0x1668`, probes helpers `0x0053A110/120/0x0053BAD0/0x0053B400`, updates scenario counters, sets `+0x34AB`, calls `FUN_004AE4C0`, then `FUN_004F42F0(1)`. | Yes, conditional on counters/rules/helper results. | Partly related to Rust ore/lighting/radar dirty flows, but no same pre-driver placement found. |
| 9 | `0x0055B4D7` | `TiberiumClass::GrowthDriver_AllTypes`. | Yes; unconditional once tail block reached; prior report verifies call `0x0055B4D7`. | Rust native ore growth runs in Phase 7 after combat/production repairs at `world/mod.rs:1899..1910` (`tick_native_growth_driver`); ordering drift unless proven equivalent. (corrected 2026-05-29: re-anchored after Rust refactor; verified via Read src/sim/world/mod.rs:1899) |
| 10 | `0x0055B4DC` | `TiberiumClass::SpreadDriver_AllTypes`. | Yes; immediately after all growth drivers. | Rust native spread follows growth in Phase 7 at `world/mod.rs:1911..1923` (`tick_native_spread_driver`); relative growth-before-spread preserved, global placement differs. (corrected 2026-05-29: re-anchored; verified via Read src/sim/world/mod.rs:1911) |
| 11 | `0x0055B4E1..0x0055B4E6` | `BombClass::UpdateAll` with `ECX=0x87F5D8`. | Yes; BombClass report verifies sole per-tick update. | Rust has C4/Ivan-style order logic inside combat/order phases, not a native bomb list at this ladder position. |
| 12 | `0x0055B4EB..0x0055B4F0` | `FUN_0054E4D0`, a 30-frame scripted/action queue helper per prior doc. | Yes; direct call in active ladder. | No direct mapped stage found. |
| 13 | `0x0055B4F5..0x0055B580` | Build a temporary vector from `g_TeamClass_Array`; source count `g_TeamClass_Array_Count` is used during copy. | Yes. | Rust AI/team behavior is later Phase 8 (`world/mod.rs:1312..1343`, inside `run_late_region`) and not proven equivalent. (corrected 2026-05-29: re-anchored; verified via Read src/sim/world/mod.rs:1312) |
| 14 | `0x0055B582..0x0055B59F` | Iterate the temporary team vector by copied stack count, calling each entry `vtable+0x5C`. | Yes. | Rust does not have an equivalent copied-count team vector pass here. |
| 15 | `0x0055B5A1..0x0055B5BC` | Reverse loop over `g_DiskLaserClass_Array` from count-1 down to 0, calling `vtable+0x5C`. | Yes. | No direct Rust disk-laser pass found. |
| 16 | `0x0055B5BE` | `FUN_005FF390` age-based object/FX reaper per prior doc. | Yes. | Rust cleanup/effects run near Phase 9 (`world/mod.rs:1352..1400`, inside `run_late_region`), not here. (corrected 2026-05-29: re-anchored; verified via Read src/sim/world/mod.rs:1352) |
| 17 | `0x0055B5C3` | `LaserDrawClass::UpdateAllAI`. | Yes. | No exact mapped laser-draw AI stage found. |
| 18 | `0x0055B5C8` | `LightningStorm::Process`. | Yes. | Rust superweapons run much earlier in Phase 4.5 (`world/mod.rs:1606..1610`), so ordering differs. (corrected 2026-05-29: re-anchored; verified via Read src/sim/world/mod.rs:1606) |
| 19 | `0x0055B5CD..0x0055B5E8` | Reverse loop over `DAT_00B04BD4/DAT_00B04BE0` radiation-site pool, calling `vtable+0x5C`. | Yes; prior docs tie this pool to `RadSiteClass`. | Rust radiation/particle-like work is not mapped to this exact location; particle systems tick after combat in Phase 5.5 at `world/mod.rs:1849..1852`. (corrected 2026-05-29: re-anchored; verified via Read src/sim/world/mod.rs:1849) |
| 20 | `0x0055B5EC..0x0055B5F6` | Call `FUN_00554D50` with `ECX=6`, then `EMPulseClass::UpdateAll`. | Yes; light-source docs verify dirty queue drain after RadSite and before EMP. | Dynamic lighting dirty drain is missing; EMP/superweapon handling is not in this position. |
| 21 | `0x0055B5FB..0x0055B619` | Settled anchor: main `LogicClass+0x04/+0x10` live forward object vector, `vtable+0x5C`, count reloaded after each call. | Yes; prior scheduler report proves live-vector contract. | Missing as a unified live appendable scheduler; Rust uses staged passes and many `keys_sorted()` snapshots. |
| 22 | `0x0055B61B..0x0055B649` | If `g_GameMode != 0 && g_GameMode != 5`, forward loop over `DAT_00A83E04/DAT_00A83E10`, calling `vtable+0x5C` (prior doc: AnimClass pool). | Conditional; skipped in modes 0 and 5. | Rust world effects/building animations tick in Phase 9 (`world/mod.rs:1352..1400`, inside `run_late_region`), not native conditional pool. (corrected 2026-05-29: re-anchored; verified via Read src/sim/world/mod.rs:1352) |
| 23 | `0x0055B64B` | `FUN_0053D310` wave splash forces tick. | Yes. | No direct mapped wave-splash force stage found. |
| 24 | `0x0055B650` | `AlphaShapeClass::PurgeDisabled`. | Yes. | No direct mapped alpha-shape purge found. |
| 25 | `0x0055B655..0x0055B65D` | `MapClass::UpdateCrateRegenTimers`. (corrected 2026-05-29: was single address `0x0055B655`; binary shows MOV ECX at `0x0055b655`, CALL `0x0056bbe0` at `0x0055b65a` — range corrected via disassemble_function 0x0055AFB0 — OPERATOR_OR_ORDER_DRIFT) | Yes; active when crate system/game option permits effects. | Crate regen not found in `advance_tick`. |
| 26 | `0x0055B65F..0x0055B667` | `g_Tactical->vtable+0x5C` tactical AI/camera/scroll service. (corrected 2026-05-29: was `0x0055B65A..0x0055B663`; the former start `0x0055B65A` is the CALL instruction of step 25, not step 26; actual MOV ECX for tactical dispatch is `0x0055b65f`, CALL [EAX+0x5C] at `0x0055b667` — verified via disassemble_function 0x0055AFB0 — OPERATOR_OR_ORDER_DRIFT) | Yes. | Rust sim cannot depend on render/UI; equivalent belongs outside sim or a split tactical service. |
| 27 | `0x0055B66A..0x0055B68B` | Forward loop over `g_FactoryClass_Array`, calling `vtable+0x5C`; count reloaded after each call. | Yes; Factory reports verify production tick here. | Rust production runs Phase 7 before AI (`world/mod.rs:1873..1880`); relative placement differs after native factories run after tactical. (corrected 2026-05-29: re-anchored; verified via Read src/sim/world/mod.rs:1873) |
| 28 | `0x0055B68F..0x0055B6B1` | Forward loop over `g_HouseClass_Array`, null-checking each pointer before `vtable+0x5C`; count reloaded after each iteration. | Yes. | Rust AI/player-house updates are split; no exact HouseClass AI tail pass found. |
| 29 | `0x0055B6B3..0x0055B6CC` | If `DisplayClass::GetLastRefObject()` non-null, read object coords `+0x9C/+0xA0/+0xA4` and call `FUN_006D6070` to refocus/recenter. | Yes, conditional on last-ref object. | No sim mapping; likely UI/tactical camera surface. |
| 30 | `0x0055B6D0..0x0055B72F` | Free temporary vector buffer if allocated/owned, then return. | Yes. | Rust temporary vectors free via RAII; behavior only matters insofar as copied team-vector contract differs. |

## 3. Rust Ordering Contrast

Current `World::advance_tick` (`src/sim/world/mod.rs:1402`) commits `total_sim_ms`/`binary_frame`/`tick` LATE (end of tick, in `run_late_region` at `1397..1399`, matching the native late frame increment — see correction below), applies commands first (`apply_due_commands`), then runs explicit phases: movement (`1428..`), vision (`1582..`), power/superweapons (Phase 4.5 `1606..1610`)/deploy/combat (Phase 5 from `1624`), particles (Phase 5.5 `1849..1852`), retaliation/passengers (Phase 6 from `1853`), production/repairs/docks/ore (Phase 7 from `1858`; production `1873..1880`, ore growth/spread `1899..1923`), AI (Phase 8 `1312..1343` in `run_late_region`), defeat/building/world-effect cleanup (Phase 9 from `1352` in `run_late_region`). (corrected 2026-05-29: all line citations re-anchored after Rust refactor; verified via Read src/sim/world/mod.rs:1402,1606,1849,1873,1899,1312,1352)

Earlier-doc delta "binary_frame committed early (at tick entry)" is OBSOLETE and removed: the code now commits `binary_frame` LATE at `world/mod.rs:1397..1398`, matching the native late `g_CurrentFrameCounter` increment, so consumers observe frame N during the tick. (corrected 2026-05-29: verified via Read src/sim/world/mod.rs:1390..1399)

This is not topologically equivalent to `PerTickUpdate`: native ore growth/spread precedes bombs, teams, object AI, tactical, factories, and houses; Rust ore runs after combat and production. Native superweapon/weather-like `LightningStorm::Process` runs after laser updates and before RadSite/EMP/main-object loops; Rust superweapons run before combat. Native factory/house AI is near the tail after tactical; Rust production and AI are earlier and split.

## 4. Coverage Ledger

| Area | Status | Evidence | What remains |
|---|---|---|---|
| Active caller from `Main_Tick` | verified | `0x0055DC99..0x0055DC9E`; caller query | none |
| Full top-level ladder `0x0055AFB0..0x0055B72F` | verified as coverage-map | Ghidra decompile plus disassembly success for range | callee internals out-of-scope |
| Main live-vector loop | verified by prior report | `0x0055B5FB..0x0055B619` | no re-proof here |
| Other loop shapes | verified | copied-count team loop `0x0055B582..0x0055B59F`; reverse loops `0x0055B5A1..0x0055B5E8`; forward factory/house loops `0x0055B66A..0x0055B6B1` | exact mutation semantics per global array deferred |
| Rust `advance_tick` ordering | verified for current source | `src/sim/world/mod.rs:1402..1973` (advance_tick body; late region `1303..1400`) | no Rust code changed (corrected 2026-05-29: re-anchored after refactor; verified via Read src/sim/world/mod.rs:1402) |
| TS legacy filter | touched-not-exhausted | fog branch `0x0055B2B8..0x0055B33D` gated by `SpecialFlags & 0x1000` | exact default writer belongs to timing/shroud follow-up |

## 5. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `0x0055AFB0` active in YR? -> Yes, called from `Main_Tick` with `ECX=0x87F778`.` (evidence: `0x0055DC99..0x0055DC9E`)
- `[RESOLVED] OQ-02 - What is the ladder order? -> Listed in Section 2 from direct decompile and assembly contexts.` (evidence: `0x0055AFB0..0x0055B72F`)
- `[RESOLVED] OQ-03 - Is the main object vector the only loop shape? -> No; copied-count, reverse, conditional, null-checked, and live-count loops coexist.` (evidence: `0x0055B582..0x0055B6B1`)
- `[RESOLVED] OQ-04 - Does Rust `advance_tick` place equivalent work in the same top-level order? -> No; multiple mapped systems are staged in different order.` (evidence: `src/sim/world/mod.rs:1402..1973`; corrected 2026-05-29: re-anchored via Read src/sim/world/mod.rs:1402)
- `[RESOLVED] OQ-05 - Which part is TS-legacy/conditional? -> The `SpecialFlags & 0x1000` fog branch is conditional and should not be treated as default YR shroud behavior.` (evidence: `0x0055B2B8..0x0055B33D`; global timing/shroud docs)
- `[DEFERRED] OQ-06 - Exact identity and mutation semantics for every anonymous global array loop.` (category: `requires-different-system-context`; reason: this slot maps top-level order only; next-step-if-pursued: run slot-2 style report over non-object global loops.)
- `[DEFERRED] OQ-07 - Full callee internals for scenario action helper and ambient/tiberium precursor helpers.` (category: `bounded-cost-too-high`; reason: each helper is a separate mechanism; next-step-if-pursued: investigate `0x006E53A0`, `0x004ACAC0`, `0x004ACBC0`, `0x004AE4C0` individually.)

## 6. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `PerTickUpdate` top-level order runs ore growth/spread, bombs, team temp-vector AI, laser/weather/rad/light/EMP, main live-object AI, optional anims, wave/alpha/crates/tactical, then factories and houses. | `0x0055B4D7..0x0055B6B1` | Mismatch: Rust `advance_tick` places production/ore/AI/superweapons in different staged order. | `src/sim/world/mod.rs::advance_tick` and future scheduler surfaces. | Future parity work must preserve native ordering for any system moved under a `PerTickUpdate`-equivalent scheduler. | On a tick where ore growth, a timed bomb, a factory completion, and House AI all mature, native order is ore -> bomb -> object/tactical -> factory -> house. | Do not justify phase order as "richer" without byte/pixel-equivalence proof. |
| Native `g_CurrentFrameCounter` is read throughout this ladder before late `Main_Tick` increment. | `0x0055AFB0` decompile; global timing report; caller context `0x0055DC99..0x0055DC9E` before late increment | RESOLVED (no longer a mismatch): Rust commits `binary_frame` LATE at `world/mod.rs:1397..1398` (end of tick, in `run_late_region`), so consumers observe frame N during the tick — matches the native late increment. (corrected 2026-05-29: obsolete "binary_frame committed early" delta removed; verified via Read src/sim/world/mod.rs:1390..1399) | `World::advance_tick`, timer users, animation/combat/ore/factory schedulers. | Systems started during a tick should observe the pre-increment native frame until the late increment point. | Start a frame-based timer during PerTickUpdate-equivalent work and query another subsystem later same tick; it still sees frame N, not N+1. | Do not advance native frame-visible counters at tick entry for systems claiming gamemd timer parity. |
| The main object loop is one middle service in the ladder; factories/houses run after it, while teams, lasers, rad sites, light drains, and EMP run before it. | main loop `0x0055B5FB..0x0055B619`; factories/houses `0x0055B66A..0x0055B6B1` | Missing: Rust has no unified live object scheduler and splits per-class systems across phases. | Future `LogicClass` scheduler plus class-specific tick routing. | Object AI side effects must be visible to later factory/house AI in the same native tick, but not retroactively to earlier team/laser/rad/light stages. | Entity AI toggles a factory/house-visible flag during main object loop; factory/house AI observes it same tick, earlier RadSite/EMP stages do not. | Do not collapse all `vtable+0x5C` owners into one unordered entity-store pass. |

## 7. Negative Facts / Do Not Do

- Do not claim Rust's staged `advance_tick` is a parity-safe "richer" replacement for `PerTickUpdate` without proving exact equivalence. Active in YR: Yes; evidence: ordered native ladder `0x0055AFB0..0x0055B72F`.
- Do not move ore growth/spread after production/combat by default when modeling the native ladder. Active in YR: Yes; evidence: growth/spread calls `0x0055B4D7/0x0055B4DC`.
- Do not treat TS fog branch `SpecialFlags & 0x1000` as default YR shroud. Active in YR: Conditional; evidence: branch `0x0055B2B8..0x0055B33D`.
- Do not treat all `vtable+0x5C` loops as one class or one mutation contract. Active in YR: Yes; evidence: copied team vector, reverse global loops, main live vector, conditional anim loop, factory/house loops.
- Do not map TacticalClass AI into `sim/` directly; preserve project layering while matching player-visible order at the app orchestration level.

## 8. Remaining Uncertainty

- Exact identity and mutation semantics for every anonymous global loop are deferred to the non-object-global-loop slot. The ladder order is verified; class naming is medium confidence where this report relies on prior constructor/xref reports.
- Scenario action helper `0x006E53A0` and the ambient/tiberium precursor helpers were not exhausted internally. This report only proves their top-level placement and active gating.
- Save/load, replay restore, and class-specific `vtable+0x5C` removal cases remain outside this slot.

## 9. Stale Docs / Follow-up Wording

Replace `LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md` lines 370-384 wording:

> `World::advance_tick` is a staged Rust implementation, not an automatically parity-equivalent "richer" version of `LogicClass::PerTickUpdate`. Active YR `PerTickUpdate @ 0x0055AFB0` has a fixed top-level order: scenario/timer work, ore precursor work, growth/spread drivers, bombs, team temp-vector AI, laser/weather/rad/light/EMP services, the main live LogicClass object vector, optional animation pool, wave/alpha/crate/tactical services, then factory and house AI. Any Rust phase order that differs is DRIFT unless byte-perfect state and pixel-perfect output equivalence is proven for the affected systems.

## Sources

- Ghidra decompile: `LogicClass::PerTickUpdate @ 0x0055AFB0`.
- Ghidra assembly/disassembly context: `0x0055AFB3..0x0055B72F`; successful disassembly of `0x0055AFB0..0x0055B72F`.
- Ghidra caller/context: `Main_Tick @ 0x0055D360`, call site `0x0055DC99..0x0055DC9E`.
- Prior reports: `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`, `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`, `LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md`, `TIBERIUMCLASS_GROWTH_SPREAD_DRIVER_TIMERS_GHIDRA_REPORT.md`, `TIBERIUMCLASS_GROWTH_PROCESSOR_EXACT_QUEUE_PROCESSING_GHIDRA_REPORT.md`, `BOMB_CLASS_GHIDRA_REPORT.md`, `BUILD_QUEUE_GHIDRA_REPORT.md`, `CRATE_SYSTEM_GHIDRA_REPORT.md`, `LIGHTSOURCE_DIRTY_SCHEDULING_00554AF0_00554D50_GHIDRA_REPORT.md`.
- Rust source read-only: `src/sim/world/mod.rs:1402..1973` (advance_tick; late region `1303..1400`). (corrected 2026-05-29: re-anchored after refactor)
