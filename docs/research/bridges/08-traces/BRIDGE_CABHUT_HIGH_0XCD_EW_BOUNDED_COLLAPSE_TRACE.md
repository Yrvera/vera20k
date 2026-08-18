# Bridge CABHUT High 0xCD EW Bounded Collapse Trace

**Scenario:** SEAL/C4 on a `BridgeRepairHut` adjacent to a high bridge body overlay in the `0xCD` family.

**Scope:** CABHUT/C4 hut-death path through `DestroyBridgeFromCell_High`, `CollapseBridge_EW_High`, bounded collapse scope, far-cell survival, and current Rust behavior after the physical-axis fix.

**Status:** COMPLETE for axis and bounded-scope comparison; visual/RNG fallout remains a FAIL; no Rust or INI files were modified.

## Pipeline

`SEAL/Tanya C4 plant` -> `BuildingClass::Update C4 timer` -> `BridgeRepairHut` branch -> high hut 5x5 overlay scan -> `DestroyBridgeFromCell_High` on a `0xCD` family cell -> `CollapseBridge_EW_High` -> per-step `DestroyBridge_High` retries -> zone/path redraw flag -> visual effects.

## Evidence Summary

- Stock YR path is active: `CABHUT` has `BridgeRepairHut=yes`, C4-capable infantry exist (`GHOST` and `TANY` have `C4=yes`), and `DestroyableBridges=yes` is enabled in `ini/rulesmd.ini`.
- `CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md` verifies `BuildingClass::Update @ 0x0043FB20` routes `BridgeRepairHut` before normal building damage, calls the high hut entry at `0x0044031B`, and clears the C4 marker afterward.
- Read-only Ghidra spot-check of `DestroyBridgeFromCell_High @ 0x005749C0` confirms a `0xCD..0xD5 / 0xDF..0xE2 / 0xE7` overlay routes to `MapClass__CollapseBridge_EW_High`.
- Read-only Ghidra spot-check of `CollapseBridge_EW_High @ 0x00575870` confirms the walker measures X extents, computes start as `seed_x - (back - fwd) / 2`, sets the loop count to `4`, calls `DestroyBridge_High` up to `3` times per step, advances X, and stops if the next overlay leaves `[0xCD..=0xE8]`.
- Read-only Ghidra spot-check of `CollapseBridge_NS_High @ 0x00575BA0` confirms the NS twin uses the same bound and retry structure on Y.
- Current Rust maps `0xCD` through `physical_span_axis_for_destroy_overlay` to `Axis::EW`, so the hut sweep steps in X rather than using the old walker-family label.

## Stage Verdicts

| Stage | gamemd output | Rust output | Verdict |
|---|---:|---:|---|
| 0xCD physical sweep axis | `CollapseBridge_EW_High`; X/EW step | `Axis::EW` from `physical_span_axis_for_destroy_overlay` | PASS |
| Collapse loop bound | `4` axial iterations max | `MAX_HUT_SWEEP_STEPS = 4` | PASS |
| Per-step destroy retries | up to `3` `DestroyBridge_High` calls | `MAX_HUT_ATTEMPTS_PER_STEP = 3` | PASS |
| Far-cell survival scope | max local footprint is about `3 x 6`; long-span cells beyond that survive one event | bounded 4-step sweep with overlapping 3-cell primitive; no full-span flood | PASS |
| Hut scan exact first seed for this concrete map | needs actual map cell ordering and hut coordinate | no concrete fixture supplied or executed | UNCHECKED |
| Exact destroyed overlay set for a concrete retail map | not computed from a real map instance here | not computed from a real map instance here | UNCHECKED |
| Zone/path rebuild timing | gamemd calls `UpdateBridgeZonesHelper` and sets `g_Tactical+0xD7C = 1` | Rust refreshes bridge zones/path state through orchestrator helpers | UNCHECKED |
| Visual/RNG fallout | up to `12` `BridgeExplosions` for 4 steps, spawned `3` per step before `DestroyBridge_High`; `48` RNG draws max for those anims | `spawn_bridge_debris` runs after destroyed cells are aggregated; one delayed BridgeExplosion per destroyed cell plus optional MetallicDebris | FAIL |

## Current Rust Comparison

The recent axis fix is correct for this scenario. In `src/sim/world/bridge_orchestrator.rs`, `find_destroy_overlay_seed` calls `physical_span_axis_for_destroy_overlay`; that helper maps the walker-family `Axis::NS` returned for `0xCD` to physical `Axis::EW`. The regression `hut_destroy_overlay_seed_uses_physical_span_axis_not_walker_family` explicitly asserts `0xCD -> Axis::EW`.

The bounded walker is also correctly shaped at the orchestration level. `run_hut_collapse_bounded` measures both directions along the physical axis, computes signed bias with Rust integer division, runs `MAX_HUT_SWEEP_STEPS = 4`, calls the per-family destroy primitive up to `MAX_HUT_ATTEMPTS_PER_STEP = 3`, then advances one axial cell and stops when the next overlay leaves the bridge band.

Far-cell survival follows from those numbers: gamemd's high EW walker can cover at most four center positions, and each per-step primitive covers a 3-cell axial window, so the unique axial footprint is at most `4 + 2 = 6` cells on a normal 3-wide body. Current Rust has the same four-step orchestration bound, so it should not collapse a full long span from one 0xCD CABHUT/C4 event.

## Failures

### FAIL - Visual/RNG fallout ordering and count

**Player-visible problem:** bridge explosion effects can appear in a different count/order/timing from gamemd, and lockstep RNG can diverge.

**gamemd evidence:** `CollapseBridge_EW_High @ 0x00575870` spawns three `BridgeExplosions` before each per-step `DestroyBridge_High` retry block. With four steps, that is up to `3 x 4 = 12` animations and `4 x 3 x 4 = 48` RNG draws for these bridge explosion anims.

**Rust evidence:** `src/sim/world/bridge_orchestrator.rs:299` calls `spawn_bridge_debris` after `destroyed_set` aggregation. `spawn_bridge_debris` at `src/sim/world/bridge_orchestrator.rs:1049` iterates destroyed cells and emits optional MetallicDebris plus one delayed BridgeExplosion per destroyed cell.

**Why it matters:** a full 3-wide bounded collapse can produce about 18 destroyed cells, so Rust's effect basis is cell-finalization count while gamemd's is walker-step/perpendicular position count. The player sees different explosion cadence, and multiplayer RNG order is not the gamemd order for this CABHUT collapse path.

## Unchecked Items

- I did not execute `cargo test`; the hard constraint allowed writing exactly one file, and a test run would write build artifacts under `target/`.
- I did not instantiate a concrete retail map/hut coordinate, so exact first seed selection within the 5x5 scan and exact final overlay set remain UNCHECKED for this slot.
- I did not trace campaign trigger side effects for event `0x1F`; that is adjacent to collapse fallout and outside this concrete mechanic.

## Adjacent Findings

- `src/sim/bridge_state/walker.rs` still uses `Axis::NS`/`Axis::EW` names for walker/write-family classification. That is fine internally, but callers must not interpret those labels as physical span axes.
- The visual/RNG failure likely affects high NS, low EW, and low NS hut collapses too, but those variants were not traced in this slot.

## Verdict Tally

PASS: 4 | FAIL: 1 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0

