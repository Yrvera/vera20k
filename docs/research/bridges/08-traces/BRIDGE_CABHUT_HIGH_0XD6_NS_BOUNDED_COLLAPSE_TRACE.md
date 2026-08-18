# Bridge CABHUT High 0xD6 NS Bounded Collapse Trace

Date: 2026-05-22
Scenario: SEAL/C4 on a BridgeRepairHut adjacent to a high bridge whose body overlay is in the `0xD6` family.
Scope: CABHUT/C4 hut-death path through `DestroyBridgeFromCell_High`, physical sweep axis, four-step bound, far-cell survival expectation, and current Rust behavior after the recent physical-axis fix.
Status: COMPLETE for static trace and Ghidra spot-checks; no Rust/gamemd runtime fixture was executed because this subagent may write only this report file.

## Pipeline

SEAL/Tanya C4 capability in stock YR data -> `BuildingClass::Update` pending-C4 timer -> `BridgeRepairHut` branch before normal building damage -> low/high hut-local bridge selection -> `MapClass::DestroyBridge_High_OnHutDeath` 5x5 high-overlay scan -> `MapClass::DestroyBridgeFromCell_High` overlay-family dispatch -> `MapClass::CollapseBridge_NS_High` for `0xD6..=0xDE`, `0xE3..=0xE6`, `0xE8` -> per-step `DestroyBridge_High` retries -> zone/redraw refresh -> Rust `dispatch_bridge_collapse_from_hut` / `run_hut_collapse_bounded` / `apply_hut_bridge_execution`.

## Stage Verdicts

| Stage | gamemd output | Current Rust output | Verdict |
|---|---|---|---|
| 1. Standard YR activation data | `rulesmd.ini`: `GHOST` and `TANY` have `C4=yes`; `[CABHUT]` has `BridgeRepairHut=yes` and `Immune=yes`; `DestroyableBridges=yes`; `BridgeExplosions=TWLT026,TWLT036,TWLT050,TWLT070`. | Same local INI data is present. | PASS |
| 2. CABHUT C4 branch ordering | `BuildingClass::Update @ 0x0043FB20` checks pending C4, then `Type+0x16B6`, calls high hut entry at `0x0044031B`, clears `+0x6DF/+0x540`, and skips normal building damage. | `world_orders.rs:734` detects `bridge_repair_hut` before normal damage, dispatches hut collapse, preserves hut HP, and returns `consumed_pending_marker`. | PASS |
| 3. Exact C4 claim/timer movement | gamemd claim/timer path is active, but this run did not compute a concrete SEAL position timeline, exact `C4Delay` frame conversion, and Rust tick-by-tick output for the same map. | Static code matches broad pending-marker shape, but literal frame equality was not computed. | UNCHECKED |
| 4. High-hut overlay scan order | `MapClass::DestroyBridge_High_OnHutDeath @ 0x00574000` scans 5x5 with outer X offset `-2..2`, inner Y offset `-2..2`; first matching high overlay calls `DestroyBridgeFromCell_High` immediately. For hut center `(10,10)`, first three interior probe cells are `(8,8)`, `(8,9)`, `(8,10)`. | `cells_in_5x5_scan` in `src/sim/bridge_state/mod.rs:1511` is Y-major: for `(10,10)`, first three cells are `(8,8)`, `(9,8)`, `(10,8)`. `find_destroy_overlay_seed` in `bridge_orchestrator.rs:240` uses that order for the first high overlay seed. | FAIL |
| 5. `0xD6` physical sweep axis | `DestroyBridgeFromCell_High @ 0x005749C0` classifies `0xD6..=0xDE`, `0xE3..=0xE6`, `0xE8` and calls `CollapseBridge_NS_High @ 0x00575BA0`. The walker changes Y each step, so the physical sweep axis is NS. | `physical_span_axis_for_destroy_overlay` in `bridge_orchestrator.rs:623` maps the walker-family result for `0xD6` to `Axis::NS`; `step_axis` changes Y for `Axis::NS`. Regression at `bridge_orchestrator.rs:1494` asserts `0xD6 -> Axis::NS`. | PASS |
| 6. Four-step bounded collapse | `CollapseBridge_NS_High @ 0x00575BA0` sets `local_2c = 4`, decrements once per axial step, and breaks early if the next overlay leaves `[0xCD..=0xE8]`. | `MAX_HUT_SWEEP_STEPS = 4` at `bridge_orchestrator.rs:326`; `run_hut_collapse_bounded` loops `0..MAX_HUT_SWEEP_STEPS` and breaks when the next cell leaves the bridge band. | PASS |
| 7. Far-cell survival on a long 0xD6-family bridge | Inference from verified four-step bound: cells beyond the bounded footprint survive one CABHUT/C4 event. This run did not compute a literal gamemd destroyed-cell set for a named long-map coordinate. | Rust has bounded logic, but this run found no concrete long-span 0xD6 CABHUT regression producing a literal Rust destroyed-cell set against gamemd. | UNCHECKED |
| 8. Per-step visual/RNG ordering | `CollapseBridge_NS_High @ 0x00575BA0` spawns three `BridgeExplosions` before each per-step `DestroyBridge_High` retry group, unless the current center overlay is `0xE8`; the anim choice uses `RulesClass+0x15C/+0x168`. | `apply_hut_bridge_execution` aggregates destroyed cells, then calls `spawn_bridge_debris` once after all state outcomes (`bridge_orchestrator.rs:299`); `spawn_bridge_debris` also includes a `MetallicDebris` path and delayed BridgeExplosion order (`bridge_orchestrator.rs:1049`). | FAIL |
| 9. Zone/path refresh tail | gamemd tail calls `UpdateBridgeZonesHelper` and sets `g_Tactical+0xD7C = 1` after the collapse walker. | Rust calls `refresh_bridge_zones_if_dirty` from aggregated outcomes and returns `bridge_state_changed`; exact same-tick redraw/path invalidation output was not computed. | UNCHECKED |
| 10. Hut survival after detonation | gamemd high-hut branch skips normal C4 damage; hut entity survives while bridge collapse runs. | Existing `c4_on_cabhut_collapses_bridge_and_hut_survives` asserts hut HP unchanged and pending marker cleared. Static code agrees. | PASS |

Verdict tally: PASS: 5 | FAIL: 2 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0

## Failures

1. Stage 4 - High-hut overlay scan order mismatch.
   - Player-visible difference: on a normal multi-cell bridge inside the hut scan, Rust can choose a different first `0xD6`-family seed than gamemd, shifting the bounded collapse footprint even with the correct NS physical axis.
   - Rust: `src/sim/bridge_state/mod.rs:1511` yields Y-major scan; `src/sim/world/bridge_orchestrator.rs:240` consumes first match.
   - gamemd: `MapClass::DestroyBridge_High_OnHutDeath @ 0x00574000` decompile shows outer X offset, inner Y offset, immediate call to `DestroyBridgeFromCell_High`.

2. Stage 8 - CABHUT collapse visual/RNG ordering mismatch.
   - Player-visible difference: explosion positions, timing, and RNG consumption can differ; gamemd emits three `BridgeExplosions` before each walker step's destruction call, while Rust batches debris/effects after all collapsed cells are collected.
   - Rust: `src/sim/world/bridge_orchestrator.rs:299` and `src/sim/world/bridge_orchestrator.rs:1049`.
   - gamemd: `MapClass::CollapseBridge_NS_High @ 0x00575BA0` decompile shows three anim constructions before the `DestroyBridge_High` retry loop.

## Implementation Notes

- The recent physical-axis fix is correct for this concrete `0xD6` path: `0xD6` routes to `CollapseBridge_NS_High`, and Rust now uses `Axis::NS` for the hut sweep.
- The full-span collapse concern is not present in the current Rust shape; both gamemd and Rust use a hard four-step walker. The missing proof is a literal long-span 0xD6 fixture comparing destroyed coordinates.
- The scan-order mismatch should be fixed before relying on long-span footprint tests, otherwise the test may encode Rust's current seed order rather than gamemd's.

## Adjacent Findings

- The same 5x5 scan-order risk likely affects low bridges and high `0xCD` family hut collapse, because the shared Rust scan helper feeds both family choice and seed selection. This trace did not expand into those scenarios.
- `destroy_bridge_high` still uses Ghidra-label-style walker-family naming for the per-cell primitive. That is acceptable for this path because `CollapseBridge_NS_High` in gamemd itself calls `DestroyBridge_High`, which dispatches `0xD6` to the corresponding per-cell write primitive.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_COLLAPSE_SYSTEM_MODEL_SYNTHESIS.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md`
- Ghidra read-only decompile: `BuildingClass::Update @ 0x0043FB20`
- Ghidra read-only decompile: `MapClass::DestroyBridge_High_OnHutDeath @ 0x00574000`
- Ghidra read-only decompile: `MapClass::DestroyBridgeFromCell_High @ 0x005749C0`
- Ghidra read-only decompile: `MapClass::CollapseBridge_NS_High @ 0x00575BA0`
- Ghidra xrefs: `0x00574000` called from `BuildingClass::Update @ 0x0044031B` and `BombClass::Detonate @ 0x00438982`; `0x00575BA0` called from `DestroyBridgeFromCell_High @ 0x00574B8D/0x00574BDA/0x00574C13`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_orders.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/walker.rs`
