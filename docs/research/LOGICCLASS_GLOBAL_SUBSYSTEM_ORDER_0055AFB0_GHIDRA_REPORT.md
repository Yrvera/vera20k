# LogicClass Global Subsystem Order @ 0x0055AFB0 - Ghidra Report

**Date:** 2026-05-28  
**Target:** `LogicClass::PerTickUpdate @ 0x0055AFB0` global subsystem sequence  
**Investigation mode:** `/re-swarm` slot 2, read-only Ghidra + Rust/doc comparison  
**Active in YR:** Yes. `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` and `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md` record the standard `Main_Tick` call into `LogicClass::PerTickUpdate` with the `LogicClass` singleton. This slot rechecked the function body directly in Ghidra.  
**Confidence:** High for relative order and loop shapes visible in `0x0055AFB0`; Medium for semantic names of still-unidentified `FUN_*` callees.

## Target Question

What is the exact active standard-YR global subsystem order inside `LogicClass::PerTickUpdate @ 0x0055AFB0`, and where does current Rust `Simulation::advance_tick` differ in ordering?

## Non-goals

- Do not re-investigate the main live LogicClass vector scheduler internals beyond citing the existing scheduler report.
- Do not identify every unknown `FUN_*` callee unless the function body or existing names make the subsystem clear.
- Do not modify Rust, INI, or non-research docs.
- Do not claim equivalence between Rust phases and gamemd phases without a verified order bridge.

## Evidence Needed To Mark COMPLETE

- Direct Ghidra decompile of `0x0055AFB0`.
- Ordered table covering scenario/cell timers, tiberium, bombs, teams, disk lasers, lighting/laser/lightning/EMP, main object vector, conditional non-local loop, tactical, factories, houses, and last-ref-object handling.
- Current Rust phase inventory from `src/sim/world/mod.rs::advance_tick`.
- Explicit stale-doc wording where existing docs compress or misplace the order.

## Stop Conditions

- Stop at the first exact ordered table for `0x0055AFB0`; deeper decompilation of all unknown callees is a follow-up.
- Stop if Ghidra becomes unavailable or decompile of `0x0055AFB0` fails.
- Stop after documenting Rust deltas; no implementation patch is allowed in this slot.

## Verified Binary Order

Ghidra decompile evidence: `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`.

| Order | Address / range evidence | Verified action | Loop shape / gate | Notes |
|---:|---|---|---|---|
| 1 | function entry | Increment `DAT_00ABCD40`; clear `DAT_00A83CDC`. | unconditional | Global per-tick counter/state setup. |
| 2 | entry through `LAB_0055B164` | Repeated scenario cell-action processing via `TechnoClass__ProcessCellAction` action IDs `0x32`, `0x1B`, `0x1C`, `0x24`, `0x25`, `0x2D`, `0x2E`, `0x0D`, `0x33`, and timer-expired `0x0E`. | `DAT_008B40D8` count loop; gated by scenario bytes `+0x34BE`, `+0x34AA`, `+0x34AB`; uses `g_CurrentFrameCounter` vs scenario timer `+0x47A/+0x47C`. | Active scenario/cell-action timer block. Exact action semantics are out of scope here. |
| 3 | `LAB_0055B164..LAB_0055B1D8` | Scenario timer completion/remaining-time update, timer start reset to `-1`, then `FUN_004F42F0(2)`. | gated by scenario timer `+0x47A != -1`. | Runs before clearing scenario dirty/action bytes. |
| 4 | `LAB_0055B1D8` | Clear scenario bytes `+0x34AA`, `+0x34A9`, `+0x34AB`, `+0x34BE`. | unconditional | This reset happens before tiberium/global arrays. |
| 5 | `LAB_0055B1D8..LAB_0055B29A` | Timed global call `FUN_004ACAC0()` after resetting scenario timer fields `+0x486..+0x488`. | gated by `Rules+0x17F0 != 0` and `Rules+0x1640 != 0.0`. | Subsystem name not resolved in this slot. Preserve order as unknown timed global A. |
| 6 | `LAB_0055B29A` | `MapClass__RecalcBridgeShroudFlags()`. | `g_CurrentFrameCounter % 0x78 == 0`. | Bridge shroud flags update precedes the next timed global and all object/vector loops. |
| 7 | `LAB_0055B29A..LAB_0055B33D` | Timed global call `FUN_004ACBC0()` after resetting scenario timer fields `+0x489..+0x48B`. | gated by `(*Scenario & 0x1000) != 0` and `Rules+0x1648 != 0.0`. | Unknown timed global B. |
| 8 | `LAB_0055B33D..LAB_0055B4D7` | Scenario/global transition block: calls `FUN_0053A110`, `FUN_0053A120`, `FUN_0053BAD0`, `FUN_0053B400`, adjusts scenario fields `+0xD4B/+0xD4C`, sets scenario byte `+0x34AB`, calls `FUN_004AE4C0()` and `FUN_004F42F0(1)`. | gated by `Scenario+0xD4C != Scenario+0xD4B` and `Rules+0x1668 != 0.0`; timer `+0x492..+0x494`. | Unknown transition block. Do not collapse it into tiberium or object AI. |
| 9 | `LAB_0055B4D7` | `TiberiumClass__GrowthDriver_AllTypes()`. | unconditional call | Growth is before spread. |
| 10 | immediately after order 9 | `TiberiumClass__SpreadDriver_AllTypes()`. | unconditional call | Spread is after growth and before bombs. |
| 11 | immediately after order 10 | `BombClass__UpdateAll()`. | unconditional call | Bomb update precedes the next unknown global and teams. |
| 12 | after `BombClass__UpdateAll` | `FUN_0054E4D0()`. | unconditional call | Unknown global C. |
| 13 | after order 12, build loop `0x0055B502..0x0055B580`, iteration `0x0055B582..0x0055B59F` per prior scheduler report | `FUN_0055BB40(0,0)`, build a scratch list from `g_TeamClass_Array`, then call `vtable+0x5C` for each scratch-listed team. | copied/scratch count, not live TeamClass count | Existing scheduler report already warns this is not the main live object-vector contract. |
| 14 | reverse loop `0x0055B5A1..0x0055B5BC` per prior scheduler report | `g_DiskLaserClass_Array` reverse `vtable+0x5C` loop. | reverse from count-1 to 0 | Disk lasers tick after teams and before lighting/laser draw. |
| 15 | after disk laser reverse loop | `FUN_005FF390()`. | unconditional call | Likely global visual/light effect work by context, but exact semantic name not verified here. |
| 16 | immediately after order 15 | `LaserDrawClass__UpdateAllAI()`. | unconditional call | Laser draw update precedes lightning storm. |
| 17 | immediately after order 16 | `LightningStorm__Process()`. | unconditional call | Lightning storm is before the other reverse global loop and before EMP. |
| 18 | reverse loop `0x0055B5CD..0x0055B5E8` per prior scheduler report | Reverse `vtable+0x5C` loop over `DAT_00B04BD4` with count `DAT_00B04BE0`. | reverse from count-1 to 0 | Other global reverse loop; class not resolved in this slot. |
| 19 | after order 18 | `FUN_00554D50()`. | unconditional call | Unknown global D. |
| 20 | immediately after order 19 | `EMPulseClass__UpdateAll()`. | unconditional call | EMP runs before the main LogicClass object vector. |
| 21 | main loop `0x0055B5FB..0x0055B619` per scheduler report | Main `LogicClass+0x04/+0x10` live object vector, forward `vtable+0x5C`. | live count reload after each object AI | Scheduler internals are covered by `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`. |
| 22 | after main vector | Conditional non-local/non-skirmish loop over `DAT_00A83E04` count `DAT_00A83E10`, forward `vtable+0x5C`. | only if `g_GameMode != 0 && g_GameMode != 5`; count reloaded in loop | Active only outside game modes 0 and 5. Standard local skirmish does not take this branch. |
| 23 | after conditional loop | `FUN_0053D310()`. | unconditional call | Unknown global E. |
| 24 | immediately after order 23 | `AlphaShapeClass__PurgeDisabled()`. | unconditional call | Alpha shape purge precedes crate regen/tactical. |
| 25 | immediately after order 24 | `MapClass__UpdateCrateRegenTimers()`. | unconditional call | Crate regen timers precede tactical AI. |
| 26 | immediately after order 25 | `g_Tactical->vtable+0x5C`. | unconditional virtual call | Tactical update precedes factories and houses. |
| 27 | forward loop `0x0055B675..0x0055B68B` per scheduler report | `g_FactoryClass_Array` forward `vtable+0x5C`. | forward live global count loop | Factories tick after tactical and before houses. |
| 28 | forward loop `0x0055B698..0x0055B6B1` per scheduler report | `g_HouseClass_Array` forward `vtable+0x5C`, with null check. | forward live global count loop | Houses tick after factories. |
| 29 | after house loop | `DisplayClass__GetLastRefObject()` twice; if non-null, copy object fields `+0x9C/+0xA0/+0xA4` to stack and call `FUN_006D6070`. | gated by non-null last reference object | Last-ref-object handling is after houses. |
| 30 | function tail | Free scratch list if allocated. | local scratch-list cleanup | Not gameplay order, but it confirms team scratch-list lifetime. |

## Current Rust `Simulation::advance_tick` Order

Rust evidence: `src/sim/world/mod.rs::advance_tick`.

| Rust order | Rust evidence | Current action |
|---:|---|---|
| 1 | `world/mod.rs:1199..1200` | Advance `total_sim_ms` and compute `binary_frame` at the start of the tick. |
| 2 | `world/mod.rs:1202..1243` | Rebuild owner index, sort and apply due commands. |
| 3 | `world/mod.rs:1245..1274` | Ground movement and gates. |
| 4 | `world/mod.rs:1276..1306` | Air/special movement: air, teleport, tunnel, rocket, homing, droppod, parachute, piggyback restore. |
| 5 | `world/mod.rs:1310..1322` | Body rocking/slope transition. |
| 6 | `world/mod.rs:1327` | Aircraft mission state machines. |
| 7 | `world/mod.rs:1331..1376` | Ship wakes. |
| 8 | `world/mod.rs:1380..1391` | Vision/fog refresh. |
| 9 | `world/mod.rs:1394..1401` | Power. |
| 10 | `world/mod.rs:1404..1408` | Superweapon tick. |
| 11 | `world/mod.rs:1411..1419` | Deploy/fear/prone. |
| 12 | `world/mod.rs:1422..1644` | Combat, capture/C4/bridge/wall/terrain/smudge/reveal/ejection/radar. |
| 13 | `world/mod.rs:1646..1648` | Particle systems. |
| 14 | `world/mod.rs:1650..1653` | Retaliation, passengers, post-combat order intents. |
| 15 | `world/mod.rs:1655..1754` | Production, repairs, docks, ore growth/spread, terrain spawners. |
| 16 | `world/mod.rs:1757..1787` | AI commands, applied immediately. |
| 17 | `world/mod.rs:1790..1794` | Defeat detection. |
| 18 | `world/mod.rs:1797..1803` | Building-up/building-down animation and undeploy spawn. |
| 19 | `world/mod.rs:1806..1821` | Radar event aging and world-effect animation. |
| 20 | `world/mod.rs:1835` | Commit `self.tick = execute_tick`. |

## Comparison Findings

| System / edge | Verified gamemd order | Current Rust order | Verdict |
|---|---|---|---|
| Frame counter visibility | `LogicClass::PerTickUpdate` uses current pre-increment `g_CurrentFrameCounter`; `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` records late increment in `Main_Tick`. | `binary_frame` is derived at tick start (`world/mod.rs:1199..1200`). | DRIFT risk: same-tick timer boundaries can be one frame early unless individually compensated. |
| Tiberium growth/spread | Orders 9-10, before bombs, teams, EMP, object vector, tactical, factories, houses. | Ore growth/spread runs in Phase 7 after combat, particles, retaliation, production, repairs, and docks (`world/mod.rs:1655..1754`). | DRIFT. |
| Bombs | Order 11, before teams and object vector. | No direct global `BombClass__UpdateAll` phase found in `advance_tick`; explosive effects are distributed through combat/superweapon/C4. | UNCHECKED/DRIFT until a BombClass-equivalent owner/order exists. |
| Teams | Order 13, before disk lasers, EMP, main object vector, tactical, factories, houses. | AI is Phase 8 after production/ore and after combat (`world/mod.rs:1757..1787`); no verified TeamClass scratch-list equivalent in this function. | DRIFT. |
| Disk lasers / laser draw / lightning / EMP | Orders 14-20, all before the main object vector. | Superweapons run before combat (`world/mod.rs:1404..1408`); EMP is not visible as a matching global phase here; render/effects are distributed. | UNCHECKED/DRIFT by default. |
| Main object AI vector | Order 21, after EMP and before conditional non-local loop/tactical/factories/houses. | No single live LogicClass active-object vector in `advance_tick`; Rust is split into subsystem phases. | DRIFT relative to scheduler contract. |
| Tactical | Order 26, after object vector and crate regen, before factories. | Tactical/UI equivalent is not modeled as a deterministic sim phase; fog/vision runs early and radar events late. | UNCHECKED/DRIFT depending on surface. |
| Factories and houses | Orders 27-28, factories before houses. | Production is Phase 7; AI and defeat are Phase 8/8.5. HouseClass-equivalent update is split across power, production, AI, defeat, superweapon grants, etc. | Partial local match only for "production before AI/defeat"; not a global order match. |
| Last-ref-object | Order 29 after houses. | No matching last-ref-object post-house call found in `advance_tick`. | UNCHECKED. |

## Implementation Handoff

- Build a small "gamemd global order ledger" around `Simulation::advance_tick` before more ad hoc phase changes. The first parity cut should decide whether Rust will introduce a `LogicClass`-style late global update section or document every deliberate remap with proof.
- Move or wrap native tiberium growth/spread ordering only after resolving smudge/combat interactions: gamemd calls growth/spread before bombs, teams, object AI, tactical, factories, and houses; Rust currently calls ore growth after combat/production/repairs/docks.
- Factory/house parity must be checked against the full tail order: tactical -> factories -> houses -> last-ref-object, not just "object AI before factories before houses."

## Negative Facts / Do Not Do

- Do not describe `0x0055AFB0` as simply "objects -> factories -> houses." That omits active earlier global systems and later tactical/last-ref-object ordering.
- Do not place lightning storm/EMP after object AI based on Rust phase convenience. Binary order has `LightningStorm__Process`, other reverse loop, `FUN_00554D50`, and `EMPulseClass__UpdateAll` before the main object vector.
- Do not treat TeamClass AI as equivalent to Rust `ai::tick_ai`. The binary TeamClass scratch-list AI runs before disk lasers and before the main object vector; Rust AI runs near the end after production/ore.
- Do not assume standard local skirmish executes the conditional `DAT_00A83E04` loop. The branch requires `g_GameMode != 0 && g_GameMode != 5`.
- Do not rename unknown `FUN_*` callees to specific subsystems without separate evidence.

## Remaining Uncertainty

- **RESOLVED 2026-05-28 (swarm slot, see `PERTICKUPDATE_UNNAMED_CALLEE_RESOLUTION_GHIDRA_REPORT.md`):**
  - `FUN_004ACAC0` (order 5) = **shroud regrowth pass**, **SKIPPED in YR** (gate `Rules+0x17F0` = `ShroudGrow`, stock `rulesmd.ini` = `no`). (verified via `decompile_function 0x004ACAC0` + `get_assembly_context 0x0066a686`)
  - `FUN_004ACBC0` (order 7) = **fog regrowth pass**, **SKIPPED in YR** (gate `*Scenario & 0x1000` = `FogOfWar`, stock = `no`). (verified via `decompile_function 0x004ACBC0` + `get_assembly_context 0x0055B2C7`)
  - `FUN_0053A110`/`FUN_0053A120`/`FUN_0053BAD0`/`FUN_0053B400` (order 8) = **terrain-morph query predicates** (1-line reads of `DAT_00A9FABC`/`DAT_00A9FAB0`/`DAT_00A9FAC0`, no side effects). (verified via `decompile_function` on each)
  - `FUN_004AE4C0` (order 8) = **`MapClass__RecomputeAllCellZAdjust`** (all-cell Z-adjust recompute; conditional, fires during superweapon ambient transitions). (verified via `decompile_function 0x004AE4C0`)
  - `FUN_0053D310` (order 23) = **WaveClass splash-force update** (loop over `DAT_00AA0128`; active in YR, no-op when count=0; absent from Rust `advance_tick`). (verified via `decompile_function 0x0053D310`)
  - Reverse `DAT_00B04BD4/DAT_00B04BE0` loop (order 18) = **`RadSiteClass`**.
- **Still YELLOW (subsystem clear, producer-class not traced):** `FUN_0054E4D0` (order 12, timed batch pass — "path/movement continue", medium confidence), `FUN_005FF390` (order 15, effect-TTL aging), `FUN_00554D50` (order 19, terrain Z-cache batch updater).
- This report does not prove which Rust phases should be moved first; it only proves current global ordering is not mechanically equivalent.

## Stale / Compressed Existing Doc Wording

- `FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md` section "LogicClass::AI - Global Tick Order (0x0055AFB0)" is stale/compressed:
  - It names the function `LogicClass::AI`, but this target is `LogicClass::PerTickUpdate @ 0x0055AFB0`.
  - Its pseudocode says `... superweapons, EMP, lightning storm ...` after factories/houses, but direct decompile shows `LightningStorm__Process` and `EMPulseClass__UpdateAll` before the main object vector, tactical, factories, and houses.
  - It collapses the active sequence to "objects -> factories -> houses", omitting scenario timers, bridge shroud recalc, tiberium growth/spread, bombs, teams, disk lasers, laser draw, lightning storm, reverse global loop, EMP, tactical, crate regen, and last-ref-object handling.
- `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` is mostly directionally correct but intentionally summarized. Its "ore/spread, bombs, teams, lasers, factories, houses, etc." wording should be supplemented by this report when exact order matters.

## Sources

- Ghidra read-only decompile: `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`.
- `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`.
- `docs/research/GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`.
- `docs/research/FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md`.
- `src/sim/world/mod.rs::advance_tick`.

## Status

COMPLETE for ordered `0x0055AFB0` subsystem table and Rust `advance_tick` comparison. PARTIAL for semantic naming of unknown callees.
