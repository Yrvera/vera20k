# Bridge Deep Slot 4 - Engineer Repair Mutation Trace

Scenario: an Engineer repair action has reached a `BridgeRepairHut=yes` CABHUT beside a destroyed high-bridge NS strip. This trace covers only the mutation details after the successful hut branch: high/low repair dispatch, high-strip walker mutation, repaired overlay variant RNG, stale damage byte behavior, damaged TMP variant clearing, zone/path rebuild, radar dirty cells/minimap propagation, and current Rust tests.

## Sources

- gamemd active branch: `InfantryClass::PerCellProcess @ 0x00519630`, spot-checked live. The BridgeRepairHut branch is reached from normal infantry per-cell processing, requires an Engineer type byte, building RTTI 6, and `BuildingTypeClass+0x16B6 != 0`, then dispatches low or high repair from the infantry/hut cell.
- gamemd high walker: `MapClass::RepairBridgeWalker_NS_High @ 0x005800D0`, spot-checked live. It is called only from `MapClass::RepairBridge_High @ 0x0057F440`.
- gamemd RNG helper: `FUN_00598030 @ 0x00598030`, spot-checked live. It loops `Random__Next(); Math__ftol(); while result > limit`.
- Verified docs: `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`, `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md`, `VOXCLASS_PLAYEVA_FFFFFFFF_SENTINEL_GHIDRA_REPORT.md`.
- Rust paths: `src/sim/world/world_orders.rs`, `src/sim/bridge_state/walker.rs`, `src/sim/bridge_state/mod.rs`, `src/app_sim_tick.rs`.

## Pipeline

Engineer on CABHUT cell -> `InfantryClass::PerCellProcess` BridgeRepairHut branch -> 5x5 scan around hut/infantry cell -> no low candidate in concrete high-strip scenario -> high repair dispatcher -> `RepairBridgeWalker_NS_High` -> per-step 3-cell perpendicular strip rewrite -> dirty/render/radar/zone side effects -> engineer disposal.

Rust equivalent: `Simulation::tick_bridge_repair_orders` -> `cells_in_5x5_scan(building cell)` -> `BridgeRuntimeState::repair_bridge_from_engineer_scan` -> `repair_bridge_high_from_scan` -> `repair_bridge_walker_ns_high` -> `apply_repair_to_strip_cell` -> `TickResult.bridge_state_changed` -> app-level `rebuild_dynamic_path_grid`.

## Stage Verdicts

### Stage 1 - Active YR BridgeRepairHut branch

gamemd: the branch is inside active `InfantryClass::PerCellProcess`, not dormant TS code. It checks the building in the infantry's current cell and requires that building to match the infantry target/alt target before the BridgeRepairHut repair path.

Rust: once `tick_bridge_repair_orders` fires, it reads `BridgeRepairHut=yes` from rules and uses the target building cell. For this concrete "reaches CABHUT" scenario, the mutation entry has the same hut center.

Verdict: PASS for this scenario. Adjacent-cell early trigger is adjacent and not traced here.

### Stage 2 - 5x5 scan and high-vs-low dispatch

gamemd: scans `-2..=+2` around the infantry/hut cell. If any low bridge tile/overlay candidate is found it calls low repair; otherwise it calls `ProcessBridgeDestruction_High`.

Rust: `cells_in_5x5_scan` yields `-2..=+2` around the building cell at `src/sim/bridge_state/mod.rs:1511`; `repair_bridge_from_engineer_scan` uses any low candidate first, then high at `src/sim/bridge_state/walker.rs:65`.

Concrete high-strip result: no low candidate -> high dispatcher.

Verdict: PASS.

### Stage 3 - High NS walker mutation footprint

gamemd: `RepairBridgeWalker_NS_High` walks west to the start of the high overlay band, then walks east while `0xCD <= overlay < 0xE9`. For each main cell it rewrites three cells: `(x,y)`, `(x,y-1)`, `(x,y+1)`.

Rust: `repair_bridge_walker_ns_high` walks `x-1` to the high-band start, then `x+1` while high-band overlays remain at `src/sim/bridge_state/walker.rs:237`; `ns_triple` returns center, north, south at `src/sim/bridge_state/walker.rs:693`.

Concrete destroyed strip output: the three-cell high NS strip is the mutation set.

Verdict: PASS.

### Stage 4 - Destroyed high overlay to repaired variant

gamemd: original overlay `0xE7` maps to `FUN_00598030(limit=3) + 0xCD`, so the output overlay is one of `0xCD..=0xD0`.

Rust: `RepairFamily::HighNs` maps `0xD1..=0xD5 | 0xE7` to `RandomHealthy { base: 0xCD }` at `src/sim/bridge_state/walker.rs:397`; `repair_variant_offset` uses a Rust rejection/modulo helper at `src/sim/bridge_state/walker.rs:412`.

Concrete equality not proven: the output range matches, but this trace did not compute a shared gamemd and Rust seed/state sample to prove that the exact chosen variant byte is identical for a given RNG state.

Verdict: UNCHECKED.

### Stage 5 - Stale body damage byte/state

gamemd: the high repair walker rewrites `CellClass+0x44` overlay values and calls cell recalculation, but the verified reports state the body damage byte remains stale after repair.

Rust: `apply_repair_to_strip_cell` rewrites `overlay_byte` but does not change `BridgeRuntimeCell.damage_state`; the focused test asserts the cells remain `DamageState::Destroyed` at `src/sim/bridge_state/walker.rs:1670`.

Concrete output: repaired overlay is walkable while the Rust damage state remains stale `Destroyed`, matching the documented gamemd quirk.

Verdict: PASS.

### Stage 6 - Damaged TMP/pavement variant clearing

gamemd: repair walkers call the pavement/variant repair path; verified docs say only repair clears `CellFlags & 0x2000` via the bridge pavement repair family.

Rust: after rewriting the strip, `apply_repair_to_strip_cell` calls `apply_damaged_variant_flood_fill(..., false, terrain)` for each touched cell at `src/sim/bridge_state/walker.rs:362`.

Concrete output on the touched strip: damaged variant is cleared. Full flood-fill equality outside the touched strip was not numerically computed in this trace.

Verdict: PASS for the touched strip; broader flood-fill extent remains UNCHECKED.

### Stage 7 - Zone/path rebuild timing

gamemd: `RepairBridgeWalker_NS_High` sets `bVar1=true` for `0xE7 -> 0xCD..=0xD0`; when the walk exits, it calls `MapClass::UpdateBridgeZonesHelper()` before returning from the repair branch.

Rust: `apply_repair_to_strip_cell` sets `outcome.zones_dirty=true` at `src/sim/bridge_state/walker.rs:341`, and `tick_bridge_repair_orders` converts this into `TickResult.bridge_state_changed` at `src/sim/world/world_orders.rs:350`. The actual `PathGrid` and zone rebuild happens later in app code after `Simulation::advance_tick` returns: `src/app_sim_tick.rs:703` and `src/app_sim_tick.rs:775`.

Player-visible difference: Rust can leave same-tick sim logic using the pre-repair path/zone graph, while gamemd rebuilds zones inside the repair branch before returning.

Verdict: FAIL.

### Stage 8 - Radar dirty cells and minimap update

gamemd: for original `0xE7`, `RepairBridgeWalker_NS_High` calls `RadarClass::MarkTerrainDirty` for the main cell, `y-1`, and `y+1`.

Rust: `apply_repair_to_strip_cell` collects those three cells into `outcome.radar_cells` for original `0xE7` at `src/sim/bridge_state/walker.rs:365`, and the focused test expects `[(2,2),(2,1),(2,3)]` at `src/sim/bridge_state/walker.rs:1670`. But `tick_bridge_repair_orders` explicitly drops them because no render-side dirty-cell API is wired at `src/sim/world/world_orders.rs:354`.

Player-visible difference: minimap terrain dirty propagation is not the gamemd per-cell update. It may be masked by broader app refreshes, but the exact radar dirty-cell contract is missing.

Verdict: NOT-IMPLEMENTED.

### Stage 9 - Existing test coverage

Covered by Rust tests:

- `repair_destroyed_high_ns_strip_rewrites_overlay_retains_stale_damage_and_radar_cells` checks high `0xE7` overlay rewrite range, stale damage state, and collected radar cells.
- `repair_scan_without_low_overlay_or_wood_tile_dispatches_high` checks high dispatch when no low candidate exists.
- `engineer_enters_cabhut_repairs_bridge` checks end-to-end repair signal, engineer consumption, repaired high overlay range, walkability, stale state, and sound event.

Missed by Rust tests:

- exact gamemd/Rust RNG sample identity for the repaired variant byte;
- same-tick zone/path rebuild timing versus post-tick app rebuild;
- actual radar/minimap dirty-cell propagation;
- full damaged TMP variant flood-fill extent beyond the synthetic touched strip.

Verdict: UNCHECKED for full end-to-end parity coverage.

## Failures and Missing Pieces

1. Zone/path rebuild timing differs. gamemd rebuilds bridge zones inside the high repair walker before returning; Rust sets `bridge_state_changed` and rebuilds PathGrid/zone grid after the sim tick in app code.
2. Radar dirty-cell/minimap propagation is not implemented. Rust records the three correct cells but drops them at the world-order layer.

## Adjacent Findings

- The broader engineer action path still has a known adjacent-cell timing concern: gamemd requires the infantry to be in the hut cell, while Rust's current `tick_bridge_repair_orders` accepts Chebyshev distance `<= 1`. This trace did not score that as a failure because the concrete scenario begins after the repair action has reached CABHUT.
- Listener/hut callback side effects after repair are adjacent to mutation propagation, but this slot was scoped to high-strip mutation, zone/path, radar/minimap, and tests.

## Tests

No cargo tests were run in this slot. The hard constraint allowed exactly one written file, and running cargo would create or update build artifacts. Test coverage above is from source inspection only.

## Verdict Tally

PASS: 5 | FAIL: 1 | UNCHECKED: 3 | NOT-IMPLEMENTED: 1

Status: COMPLETE
