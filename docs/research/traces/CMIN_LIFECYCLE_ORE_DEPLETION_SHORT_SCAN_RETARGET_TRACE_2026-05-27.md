# CMIN Lifecycle Ore Depletion Short-Scan Retarget Trace

Date: 2026-05-27

Scenario: Standard YR Allied Chrono Miner (`CMIN`) is in `Mission_Harvest` state 1 / Rust `MinerState::Harvest` on a Riparius ore cell. The current source cell fully depletes, the miner is still below `Storage=20`, and the only remaining nearby ore is an adjacent harvestable Riparius cell at `(21,20)`, inside `TiberiumShortScan=6`.

Scope: one depletion-to-nearby-retarget cycle only: final source-cell extraction, full-removal side effects, next empty-cell harvest gate, `TiberiumShortScan` target selection, destination/mission continuity, active harvest visual flag, and whether the miner stays in harvest rather than exits to refinery return.

Write limit: this report is the only file written for this trace.

## Verdict

FAIL overall.

The most visible mismatch is that a CMIN that depletes a cell and finds adjacent ore does not preserve stock harvest-substate/presentation continuity in Rust. gamemd stays in `Mission_Harvest` state 1, restores the active harvest flag in the same retarget tick, and installs a destination immediately. Rust sets a separate `MoveToOre` state, hides/resets the harvest overlay while moving, and issues the movement later through the normal `MoveToOre` path. The same source-cell depletion also inherits the known real-overlay off-by-one cargo problem and lacks gamemd's `Reduce_Tiberium` spread/radar/tactical side-effect bundle.

Tally: PASS: 3 | FAIL: 4 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Pipeline

`CMIN harvest gate` -> `Harvest_Ore_Tick final extraction` -> `Reduce_Tiberium full removal` -> `next state-1 empty-cell gate` -> `TiberiumShortScan` -> `Set_Destination / target_ore_cell` -> `state + harvest visual flag` -> `OREGATH / movement-visible result`.

## Stage Results

### Stage 1 - Standard YR Scenario Data

Verdict: PASS.

gamemd / INI:
- `ini/rulesmd.ini:7351-7396` defines `[CMIN]` as `Harvester=yes`, `PipScale=Tiberium`, `Storage=20`, `Dock=NAREFN,GAREFN`, and `Teleporter=yes`.
- `ini/rulesmd.ini:311` defines `TiberiumShortScan=6`.
- `ini/rulesmd.ini:30372-30396` registers `Riparius` as tiberium type 0 with `Value=25`.
- Active in standard YR: yes. `UnitClass::Mission_Harvest @ 0x0073E5E0` and `UnitClass::Harvest_Ore_Tick @ 0x0073D450` gate standard ore harvesting on `UnitTypeClass+0xE0E Harvester`; no TS-only `Weeder` flag is required for CMIN.

Rust:
- `src/sim/miner/mod.rs:189-207` gives default ore value 25, CMIN capacity 20, harvest interval 18, and short scan radius 6.
- `src/sim/miner/mod.rs:243-280` stores `target_ore_cell`, discrete cargo bales, `harvest_timer`, and `last_harvest_cell`.

Concrete values: source cell `(20,20)`, adjacent ore `(21,20)`, empty CMIN cargo `0/20`, source cell `OverlayData=5`, adjacent cell harvestable and the only ring-1 candidate.

### Stage 2 - Final Source-Cell Extraction Amount

Verdict: FAIL.

gamemd:
- `Harvest_Ore_Tick @ 0x0073D450` computes `Storage - StorageClass::GetTotalAmount`, converts through `Math__ftol`, then calls `CellClass::Reduce_Tiberium`.
- Empty CMIN request: `ftol(20.0 - 0.0) = 20`.
- `Reduce_Tiberium @ 0x00480A80` full-removal path returns pre-removal `OverlayData`. For this concrete `OverlayData=5`, `20 >= 6`, so it clears the overlay and returns `5`.
- Storage add is `5.0` Riparius, later worth `5 * 25 = 125` credits.

Rust:
- Real map overlay seeding uses `richness = entry.frame.min(11) + 1` at `src/sim/production/production_queue.rs:155`, so a real `OverlayData/frame=5` cell becomes `6 * 120 = 720` stock.
- `extract_bales_max` computes `density_levels = node.remaining / base` at `src/sim/miner/miner_system.rs:830-839`, so it extracts `6` bales for the same real overlay-backed cell.
- Cargo added at `src/sim/miner/miner_system.rs:536-537`: `6` ore bales, carried value `150`.

Player-visible difference: this source-cell depletion overfills the CMIN by one bale and overpays the eventual deposit by 25 credits.

### Stage 3 - Full-Removal Side Effects

Verdict: NOT-IMPLEMENTED.

gamemd:
- `Reduce_Tiberium @ 0x00480A80` full removal writes `OverlayTypeIndex=-1`, writes `OverlayData=0`, calls `CellClass::RecalcAttributes`, calls `RadarClass::MarkTerrainDirty`, clears spread bitmaps for all tiberium types, checks 8 neighbors, queues valid neighbors into the removed type's spread queue, then calls `TacticalClass::DirtyScreenRect`.
- Active in standard YR: yes through `Harvest_Ore_Tick` on `[CMIN] Harvester=yes`.

Rust:
- `extract_bales_max` removes the `ResourceNode` and calls `OverlayGrid::clear_overlay` at `src/sim/miner/miner_system.rs:849-853`.
- `OverlayGrid::clear_overlay` clears the overlay and pushes a dirty cell at `src/sim/overlay_grid.rs:92-98`.
- No authoritative RA2/YR growth/spread bitmap clear, neighbor reseed, radar dirty, or tactical dirty side-effect bundle is executed from this harvest path.

Player-visible difference: mined-out patches can feed growth/spread and terrain/radar invalidation differently after depletion.

### Stage 4 - Empty-Cell Retarget Timing

Verdict: FAIL.

gamemd:
- The final successful extraction resets the state-1 StepTimer to `HarvesterLoadRate`.
- `Mission_Harvest @ 0x0073E5E0` waits until the step counter reaches `9`; with stock `HarvesterLoadRate=2`, the next empty-cell decision occurs 18 frames after timer initialization.
- On that next gate, `Harvest_Ore_Tick` sees the current cell is no longer `LandType=5`, resets timer fields to zero/rate-zero, and returns false.

Rust:
- Successful extraction sets `snap.miner.harvest_timer = config.harvest_tick_interval` at `src/sim/miner/miner_system.rs:553`.
- `handle_harvest` decrements while positive and returns at `src/sim/miner/miner_system.rs:520-524`; the empty-cell branch executes only after an extra countdown tick.
- With stock interval `18`, the retarget decision occurs at `T+19` Rust miner ticks after success, not `T+18`.

Player-visible difference: the depleted-cell continuation waits one extra sim tick before choosing the adjacent ore.

### Stage 5 - Short-Scan Target Selection

Verdict: PASS.

gamemd:
- After `Harvest_Ore_Tick` false and while not full, `Mission_Harvest @ 0x0073E5E0` calls `FootClass::Search_For_Tiberium_And_Move((Rules+0x1778 >> 8), 0)`, i.e. `TiberiumShortScan=6` and zone argument `0`.
- `FootClass::Search_For_Tiberium_And_Move @ 0x004DCFE0` calls vtable `+0x338` only if no destination exists, then sets the destination to the selected cell if it is not the current cell.
- `FootClass::Scan_For_Tiberium @ 0x004DD0A0` checks the current cell first, then scans outward rings and keeps the highest value in the first ring with a hit.
- Concrete output with exactly one harvestable adjacent candidate at `(21,20)`: selected cell `(21,20)`.

Rust:
- `handle_harvest` calls `search_local_ore` with `config.local_continuation_radius` at `src/sim/miner/miner_system.rs:571-581`.
- `search_local_ore` checks ring 0, then ring 1 first, and returns the best hit in that ring at `src/sim/miner/miner_system.rs:1367-1424`.
- Concrete output with exactly one harvestable adjacent candidate at `(21,20)`: selected cell `(21,20)`.

Literal equality for this scoped target cell: `(21,20) == (21,20)`.

### Stage 6 - Destination Write and Mission/Substate Continuity

Verdict: FAIL.

gamemd:
- On short-scan success, `FootClass::Search_For_Tiberium_And_Move @ 0x004DCFE0` calls vtable `+0x480` in the same mission tick to set the destination cell.
- `Mission_Harvest @ 0x0073E5E0` then writes `UnitClass+0xBC = 1` and `UnitClass+0x6D2 = 1`, and returns. The miner does not leave harvest mission state 1.
- If the scan returned false but `UnitClass+0x5A4` destination was already nonzero, the caller would still take the same continuation-success block.

Rust:
- The continuation hit writes `snap.miner.target_ore_cell = Some(next_cell)` and `snap.miner.state = MinerState::MoveToOre` at `src/sim/miner/miner_system.rs:583-586`.
- Movement issuing belongs to the later `MoveToOre` path, where target scan/arrival/movement logic runs at `src/sim/miner/miner_system.rs:447-492` and below.

Player-visible difference: stock keeps the harvest mission/substate continuous and installs the destination immediately; Rust exits to a separate top-level movement state and delays movement-command issuance to the movement state path.

### Stage 7 - Return-To-Refinery Decision

Verdict: PASS.

gamemd:
- In the scoped case, storage is below full and the short scan succeeds, so `Mission_Harvest` does not set substate `2` and does not enter the refinery-return branch.
- Concrete boolean: `return_to_refinery = false`.

Rust:
- In the scoped case, `snap.miner.is_full()` is false and `continuation_target` is `Some((21,20))`, so lines `583-586` return before `begin_return`.
- Concrete boolean: `return_to_refinery = false`.

Literal equality for this scoped boolean: `false == false`.

### Stage 8 - Active Harvest Visual Flag / OREGATH Continuity

Verdict: FAIL.

gamemd:
- `Mission_Harvest` clears `UnitClass+0x6D2 = 0` immediately after `Harvest_Ore_Tick` false, then restores `+0x6D2 = 1` in the same continuation-success block.
- `UnitClass::DrawExtras @ 0x0073CEC0` consumes `Harvester=yes` plus `+0x6D2`, then suppresses OREGATH while the locomotor moving predicate returns true. It does not require a mission switch to resume presentation.
- OREGATH frame uses `(UnitClass+0x538 + g_CurrentFrameCounter) % 15 + facing_index * 15`, so this branch does not restart the animation at frame 0.

Rust:
- Voxel animation playing is keyed directly to `miner.state == MinerState::Harvest` and resets frame/elapsed when false at `src/sim/miner/miner_system.rs:178-187`.
- `HarvestOverlay.visible` is also keyed directly to `miner.state == MinerState::Harvest`, and hide/show resets frame/elapsed at `src/sim/miner/miner_system.rs:192-205`.
- Because the continuation hit sets `MinerState::MoveToOre`, Rust hides and resets the harvest overlay during the retarget hop.

Player-visible difference: stock preserves the active harvest presentation flag across the retarget and only gates drawing on locomotor movement; Rust treats the retarget as leaving harvest presentation and restarts the overlay animation.

### Stage 9 - Exact Pixel Result During Retarget Movement

Verdict: UNCHECKED.

Known gamemd mechanism:
- `DrawExtras` skips OREGATH while locomotor vfunc `+0x80` returns moving, then draws using the global-frame formula when the gate clears.

Known Rust mechanism:
- Rust hides and resets `HarvestOverlay` on `MoveToOre`.

Why not PASS:
- This trace did not capture a runtime gamemd frame and Rust frame at the exact movement-completion tick, so exact pixel coordinates/frame/RGB were not computed for both engines.

Risk:
- The mechanism mismatch in Stage 8 is already a FAIL. The exact on-screen first-resume frame remains UNCHECKED until a runtime/pixel trace compares both outputs numerically.

## Failures and Missing Pieces

1. Stage 2 - Source-cell cargo amount: Rust can extract 6 bales from real `OverlayData=5`; gamemd returns 5. Affects `src/sim/production/production_queue.rs:155` and `src/sim/miner/miner_system.rs:830-839`. Evidence: `CellClass::Reduce_Tiberium @ 0x00480A80`, `Harvest_Ore_Tick @ 0x0073D450`.
2. Stage 3 - Full-removal side effects: Rust harvest depletion lacks the gamemd `Reduce_Tiberium` side-effect bundle. Affects `src/sim/miner/miner_system.rs:849-853` and `src/sim/overlay_grid.rs:92-98`. Evidence: `CellClass::Reduce_Tiberium @ 0x00480A80`.
3. Stage 4 - Retarget timing: Rust waits one extra sim tick before empty-cell short scan after a successful depletion. Affects `src/sim/miner/miner_system.rs:520-524` and `src/sim/miner/miner_system.rs:553`. Evidence: `Mission_Harvest @ 0x0073E5E0`.
4. Stage 6 - Mission/substate continuity: Rust sets `MinerState::MoveToOre`; gamemd keeps `Mission_Harvest` substate 1 and sets destination in the same tick. Affects `src/sim/miner/miner_system.rs:583-586`. Evidence: `Mission_Harvest @ 0x0073E5E0`, `FootClass::Search_For_Tiberium_And_Move @ 0x004DCFE0`.
5. Stage 8 - Visual harvest flag continuity: Rust hides/resets harvest overlay while moving to the adjacent cell; gamemd restores `+0x6D2` and only suppresses OREGATH through the locomotor moving gate. Affects `src/sim/miner/miner_system.rs:178-205`. Evidence: `Mission_Harvest @ 0x0073E5E0`, `UnitClass::DrawExtras @ 0x0073CEC0`.

## Adjacent Findings

- `search_local_ore` computes candidate value as `base * (node.remaining + 1)` at `src/sim/miner/miner_system.rs:1359-1365`, while the comment says density. This trace has only one adjacent candidate, so the value formula cannot change the selected cell here.
- The existing tests at `src/sim/miner/miner_tests.rs:4454-4507` expect `MinerState::MoveToOre` after the short scan. That is useful for the current Rust implementation, but stale as a parity expectation because stock keeps harvest substate 1.
- Exact DriveLocomotion vfunc `+0x80` conditions are not re-traced here. The scoped conclusion only depends on `DrawExtras` gating OREGATH on that return value.

## Sources

- Read-only Ghidra MCP:
  - `decompile_function 0x0073E5E0` - `UnitClass::Mission_Harvest`
  - `decompile_function 0x0073D450` - `UnitClass::Harvest_Ore_Tick`
  - `decompile_function 0x00480A80` - `CellClass::Reduce_Tiberium`
  - `decompile_function 0x004DCFE0` - `FootClass::Search_For_Tiberium_And_Move`
  - `decompile_function 0x004DD0A0` - `FootClass::Scan_For_Tiberium`
  - `decompile_function 0x004DCE80` - `FootClass::Is_Cell_Harvestable`
  - `decompile_function 0x0073CEC0` - `UnitClass::DrawExtras`
  - `get_function_callers 0x0073D450` - only direct caller returned: `UnitClass::Mission_Harvest @ 0x0073E5E0`
- Prior verified docs:
  - `docs/research/miner/HARV_HARVEST_STATE_RETARGET_VISUAL_FLAG_GHIDRA_REPORT.md`
  - `docs/research/HARVEST_ORE_TICK_TIMING_PARTIAL_FULL_EDGE_CASES_ORE_GEMS_GHIDRA_REPORT.md`
  - `docs/research/REDUCE_TIBERIUM_FULL_REMOVAL_SIDE_EFFECTS_AND_RETURN_VALUE_GHIDRA_REPORT.md`
  - `docs/research/traces/CMIN_HARVEST_DENSITY_CARGO_REDUCE_TIBERIUM_TRACE.md`
- INI:
  - `ini/rulesmd.ini`
- Rust scanned read-only:
  - `src/sim/miner/miner_system.rs`
  - `src/sim/miner/mod.rs`
  - `src/sim/miner/miner_tests.rs`
  - `src/sim/production/production_queue.rs`
  - `src/sim/overlay_grid.rs`
