# CMIN Ore Depletion Retarget Short Scan Trace

Date: 2026-05-23  
Trace slot: 3  
Mechanic: Chrono Miner ore depletion retarget short scan  
Scope: Standard YR Allied Chrono Miner (`CMIN`) harvests the last density from its current Riparius ore cell, fully depletes/removes the overlay, then short-scans nearby ore and receives the next target.

## Concrete Scenario

`CMIN` is already in `Mission_Harvest` state 1 / Rust `MinerState::Harvest`, physically on Riparius ore cell `(20,20)`, with no movement destination and non-full cargo. The cell `(20,20)` contains its last density levels and is removed by this harvest action. A reachable nearby Riparius ore cell `(21,20)` exists inside `TiberiumShortScan=6`; no other candidate is needed for the concrete retarget outcome.

This is the depleted-before-full continuation path. Full-cargo return, refinery docking, archive consumption after unloading, and no-ore miss behavior are adjacent systems and are not traced here.

## Active Standard YR Confirmation

- `ini/rulesmd.ini:7351-7399` defines `[CMIN]` with `Harvester=yes`, `Storage=20`, `Teleporter=yes`, and teleport locomotor.
- `ini/rulesmd.ini:311-312` defines `TiberiumShortScan=6` and `TiberiumLongScan=48`.
- Read-only Ghidra spot-check on `UnitClass::Mission_Harvest` at `0073E5E0` shows the live harvester state switch uses `TypeClass+0xE0E` (`Harvester`) and the state-1 empty-cell branch calls `FootClass::Search_For_Tiberium_And_Move(TiberiumShortScan, 0)` for harvesters, then writes state 1 and `UnitClass+0x6D2=1` if a target/destination exists.
- Read-only Ghidra spot-check on `CellClass::Reduce_Tiberium` at `00480A80` confirms this is the live standard ore removal function used by standard harvesters; the starting research report also lists `UnitClass::Harvest_Ore_Tick` as an active caller.

## Pipeline

`Harvest timer gate expires -> Harvest_Ore_Tick / extract_bales_max drains current cell -> full removal clears overlay -> growth/spread reseed side effects -> next empty harvest attempt short-scans radius 6 -> Set_Destination / target_ore_cell set -> harvest visual/state continues or changes`

## Stage Table

| Stage | Boundary Checked | Our Output | gamemd Output | Verdict |
|---|---|---:|---:|---|
| 1. Rules/entity setup | `CMIN` identity and short-scan radius | `MinerKind::Chrono`, `capacity_bales=20`, `local_continuation_radius=6` from config/defaults | `[CMIN] Harvester=yes`, `Storage=20`, `Teleporter=yes`; `TiberiumShortScan=6` | PASS |
| 2. Depletion amount | Last-density harvest with non-full cargo | `n = min(empty_capacity, density_levels)` bales; for 5 density levels and 20 empty slots, removes 5 | `Reduce_Tiberium(amount)` removes `min(amount, OverlayData+1)`; for 5 levels, returns 5 | PASS |
| 3. Overlay/resource removal order | Fully depleted `(20,20)` | Removes `resource_nodes[(20,20)]`, then `OverlayGrid::clear_overlay(20,20)` if grid exists | Writes `OverlayTypeIndex=-1`, `OverlayData=0`, calls `RecalcAttributes`, marks radar/dirty rect | FAIL |
| 4. Growth queue on max-density detour | If depleted cell had `OverlayData==11` before removal | No growth queue add or equivalent | Calls `TiberiumClass::AddToGrowthQueue(&cell)`, but verified report says internal `< 11` guard sees pre-decrement density 11, so net enqueue is blocked | PASS |
| 5. Spread queue reseed on full removal | Neighbor reseed after full removal | No immediate spread-bitmap clear and no 8-neighbor enqueue from the removed cell | Clears spread bitmap entry for all tib types, then visits 8 in-bounds neighbors sequentially and calls `AddToSpreadQueue` for this tiberium type when bitmap allows | NOT-IMPLEMENTED |
| 6. Dirty/passability propagation | Removed overlay becomes non-tiberium terrain | `OverlayGrid::clear_overlay` pushes dirty cell; app tick later recalculates overlay passability from dirty cells | `RecalcAttributes` runs immediately inside `Reduce_Tiberium`; terrain/radar/tactical dirtying happen in the same call | UNCHECKED |
| 7. Empty-cell continuation trigger | Next harvest attempt after current cell gone | After timer reset, next `handle_harvest` finds no node/bales and runs continuation scan | State 1 waits step gate; next `Harvest_Ore_Tick` returns 0, then state-1 continuation branch runs | PASS |
| 8. Short-scan radius | Retarget scan radius after empty cell, not full | `config.local_continuation_radius = 6` | `RulesClass+0x1778` -> `TiberiumShortScan=6`, shifted to cells | PASS |
| 9. Short-scan order/selection | Nearby one-candidate cell `(21,20)` | `search_local_ore` ring 0 misses, ring 1 scans top/bottom/left/right arms, returns `(21,20)` | `FootClass::Scan_For_Tiberium` ring 0 checks LandType, then rings `1..radius-1`, arms top/bottom/left/right, highest value in first hit ring; returns `(21,20)` | PASS |
| 10. Retarget state write | Next target received | Sets `target_ore_cell=Some((21,20))`, `state=MoveToOre` | `Search_For_Tiberium_And_Move` sets destination; Mission_Harvest writes substate `1` and `UnitClass+0x6D2=1` | FAIL |
| 11. Archive/ghost cell reseeding | Whether short-scan hit saves archive | No `last_harvest_cell` write on this non-full short-scan hit | No `SetGhostCell(found)` on non-full retarget hit; ghost cell is cleared on miss/full-path cases only | PASS |
| 12. Visual harvest flag | What player sees while moving to adjacent ore | `tick_miners` derives voxel/oregath visibility from `state == Harvest`; entering `MoveToOre` turns it off | Binary writes `UnitClass+0x6D2=1`; exact render consumer to pixels not traced in this run | UNCHECKED |

## Failures

### Stage 3 - Rust does not perform the full `Reduce_Tiberium` removal side effects

Player-visible problem: the depleted ore cell disappears, but Rust does not perform all same-tick terrain/radar/tactical side effects that gamemd performs inside `Reduce_Tiberium`. The overlay is cleared in `extract_bales_max`, but there is no Rust equivalent of `RecalcAttributes`, `RadarClass::MarkTerrainDirty`, or `TacticalClass::DirtyScreenRect` at that boundary.

Current Rust:
- `src/sim/miner/miner_system.rs:838-842`: full depletion removes the resource node and clears `OverlayGrid`.
- `src/sim/overlay_grid.rs:92-99`: `clear_overlay` clears the cell and queues a dirty cell.
- `src/app_sim_tick.rs:680-689`: passability is recalculated later from dirty overlay cells.

gamemd evidence:
- `CellClass::Reduce_Tiberium` at `00480A80` writes `OverlayTypeIndex=-1`, `OverlayData=0`, then calls `CellClass::RecalcAttributes`, `RadarClass::MarkTerrainDirty`, `TiberiumClass::ClearSpreadBitmaps_AllTypes`, neighbor spread reseed, and `TacticalClass::DirtyScreenRect`.

### Stage 10 - Short-scan hit exits the harvest state in Rust

Player-visible problem: when the miner drains `(20,20)` and retargets `(21,20)`, Rust sets the miner to `MoveToOre`; gamemd remains in harvest substate 1 and sets the active harvesting flag. In Rust, downstream harvest visuals are keyed directly from `state == Harvest`, so the retarget hop can turn off the ore-gathering visual state.

Current Rust:
- `src/sim/miner/miner_system.rs:583-586`: continuation hit writes `target_ore_cell=Some(next_cell)` and `MinerState::MoveToOre`.
- `src/sim/miner/miner_system.rs:178-205`: voxel animation and `HarvestOverlay` visibility are driven by `snap.miner.state == MinerState::Harvest`.

gamemd evidence:
- `UnitClass::Mission_Harvest` at `0073E5E0`, state 1: after failed `Harvest_Ore_Tick`, non-full harvester calls `FootClass::Search_For_Tiberium_And_Move(TiberiumShortScan, 0)`. If the call succeeds or a destination exists, it writes `param_1[0x2F]=1` and byte `UnitClass+0x6D2=1`, then returns 1.

## Not Implemented

### Stage 5 - Depletion-time spread queue reseed is missing

Player-visible problem: an ore patch that has just been harvested empty is not immediately reseeded into the original engine's tiberium spread queue. On maps with ore growth/spread enabled, this can change where and when ore regrows around harvested patches.

Current Rust:
- `extract_bales_max` removes the node and clears the overlay but does not touch `OreGrowthState`.
- `src/sim/ore_growth.rs:156-260` collects growth/spread candidates by incrementally scanning existing `resource_nodes`; it does not receive a depletion-time neighbor enqueue.
- `src/sim/ore_growth.rs:292-337` spreads from sampled existing source cells with a random starting direction, which is a different timing/source model from gamemd's immediate neighbor reseed after `Reduce_Tiberium` full removal.

gamemd evidence:
- `CellClass::Reduce_Tiberium` at `00480A80` full-removal branch calls `TiberiumClass::ClearSpreadBitmaps_AllTypes`, then loops `dir=0..7`, computes each neighbor from `g_DirectionOffsets`, checks in-bounds and the current tiberium type's spread bitmap, then calls `TiberiumClass::AddToSpreadQueue`.
- The verified `CELLCLASS_REDUCE_TIBERIUM_FUN_00480A80_GHIDRA_REPORT.md` confirms this call path is active in standard YR through `UnitClass::Harvest_Ore_Tick`.

## Timing Notes

- For this scenario, both engines delay the short-scan until the next harvest action after the successful draining call. Rust resets `harvest_timer` to `config.harvest_tick_interval` after extracting bales while not full; gamemd state 1 waits for the StepTimer gate before the next `Harvest_Ore_Tick`.
- The exact same-tick ordering of Rust overlay passability refresh relative to renderer/minimap output was not computed to literal equality with gamemd, so Stage 6 is UNCHECKED.
- No compile/test run was performed because the task allowed writing exactly one file, and Rust builds/tests would write under `target/`.

## Adjacent Findings

- The older trace archive's `search_local_ore` mismatch is partly stale. Current Rust now uses a diamond ring scan with early exit per ring and top/bottom/left/right arm order, matching the read-only Ghidra spot-check for this one-candidate retarget scenario.
- The direct Ghidra spot-check shows `FootClass::Scan_For_Tiberium` uses a strict `old_value < new_value` comparison; equal-value ties keep the first-seen candidate, not the last. This trace did not expand into tie scenarios.
- `src/sim/miner/mod.rs:395-427` contains a combat-facing `reduce_tiberium` helper that removes resource nodes but has no overlay-grid or spread-queue side effects. It was not used by the Chrono Miner path traced here.

## Verdict Tally

PASS: 7 | FAIL: 2 | UNCHECKED: 2 | NOT-IMPLEMENTED: 1

## Player-Visible Ranking

1. Stage 5 - Harvested-empty ore cells do not reseed the spread queue; ore regrowth/spread around depleted patches can happen from different sources/timing than gamemd.
2. Stage 10 - Miner retargets adjacent ore but enters `MoveToOre`; harvest voxel/oregath state can turn off during the hop while gamemd keeps harvest substate 1 and `UnitClass+0x6D2=1`.
3. Stage 3 - Full ore removal lacks same-call `RecalcAttributes`/radar/tactical dirty side effects; terrain/radar/visual invalidation timing may differ.

## Sources

- `docs/research/CELLCLASS_REDUCE_TIBERIUM_FUN_00480A80_GHIDRA_REPORT.md`
- `docs/research/miner/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md`
- `docs/research/miner/traces/MINER_FSM_ORE_DEPLETION_RETARGET_ARCHIVE_TRACE.md`
- Read-only Ghidra decompile: `CellClass::Reduce_Tiberium` at `00480A80`
- Read-only Ghidra decompile: `UnitClass::Mission_Harvest` at `0073E5E0`
- Read-only Ghidra decompile: `FootClass::Scan_For_Tiberium` at `004DD0A0`
- Read-only Ghidra decompile: `FootClass::Search_For_Tiberium_And_Move` at `004DCFE0`
- `ini/rulesmd.ini`
- `src/sim/miner/miner_system.rs`
- `src/sim/miner/mod.rs`
- `src/sim/overlay_grid.rs`
- `src/sim/ore_growth.rs`
- `src/app_sim_tick.rs`

## Status

COMPLETE
