# Chrono Miner Harvest Density/Cargo Reduce_Tiberium Trace

Date: 2026-05-23
Scenario: Standard YR Allied Chrono Miner (`CMIN`) has arrived at a Riparius ore cell with `OverlayData=11` and empty cargo.
Scope: first harvest extraction through `CellClass::Reduce_Tiberium`, cargo insertion, overlay update, and visible cargo/ore relationship.
Write limit: this report is the only file written for this trace.

## Verdict

FAIL overall.

The concrete player-visible mismatch is that Rust can turn one maximum-density ore cell into 12 carried ore bales, while gamemd's live standard-harvester path removes that cell and adds 11.0 Riparius storage units. Rust therefore overpays this exact pickup by 25 credits of carried cargo before refinery deposit, and likely advances the selected-miner cargo pips one step earlier.

Tally: PASS: 3 | FAIL: 3 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Pipeline

Arrived CMIN in Harvest state -> harvest wait gate -> compute empty capacity -> call/reproduce `Reduce_Tiberium` -> mutate ore cell -> add Riparius cargo -> render ore overlay and cargo pips.

## Stage Results

### Stage 1 - Standard YR Scenario Data

Verdict: PASS.

gamemd/YR data:
- `CMIN`: `Harvester=yes`, `PipScale=Tiberium`, `Storage=20`, `Teleporter=yes` in `ini/rulesmd.ini` lines 7351-7396.
- Riparius: registered as tiberium index 0, `Value=25`, `Growth=2200`, `Spread=2200` in `ini/rulesmd.ini` lines 30372-30396.
- This is active YR, not TS legacy: `UnitClass__Harvest_Ore_Tick` checks `UnitType+0xE0E Harvester`; CMIN sets that flag. The TS `Weeder` branch is gated by `UnitType+0xE0F` and is not set on standard CMIN.

Rust data:
- `miner_kind_for_object` maps `Harvester=yes` + `Teleporter=yes` to `MinerKind::Chrono` in `src/sim/miner/mod.rs` lines 371-383.
- `Miner::new` honors object `Storage=` as `capacity_bales`; for CMIN this is 20 in `src/sim/miner/mod.rs` lines 300-318.
- `MinerConfig` default ore bale value is 25 in `src/sim/miner/mod.rs` lines 184-195.

### Stage 2 - Harvest Cadence Gate

Verdict: FAIL.

gamemd:
- `UnitClass::Mission_Harvest` state 1 waits for 9 StepTimer steps.
- `HarvesterLoadRate` default is 2 frames, so the first extraction gate is 18 frames after timer initialization. Evidence: `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`, State 1 and RulesClass offsets.

Rust:
- Arrival into the ore cell sets `snap.miner.harvest_timer = config.harvest_tick_interval` at `src/sim/miner/miner_system.rs` lines 471-472.
- `handle_harvest` extracts only when the timer is already 0; while it is positive, it decrements and returns at lines 520-524.
- With default `harvest_tick_interval=18`, arrival tick T sets 18, T+1..T+18 decrement to 0, and extraction occurs on T+19.

Player-visible difference:
- First pickup is one sim tick late after arrival. At 15 Hz this is about 66.7 ms. Small but literal numerical mismatch.

### Stage 3 - Amount Requested From Ore Cell

Verdict: PASS.

gamemd:
- `UnitClass__Harvest_Ore_Tick` at `0x0073D450` computes `Storage - StorageClass::GetTotalAmount`, converts through `Math__ftol`, and passes that to `CellClass__Reduce_Tiberium`.
- Empty CMIN: `Storage=20`, total storage `0.0`, so requested amount is `ftol(20.0) = 20`.
- Read-only Ghidra spot-check: `decompile_function 0x0073D450` on 2026-05-23.

Rust:
- `handle_harvest` computes empty capacity as `capacity_bales - cargo.len()` at `src/sim/miner/miner_system.rs` lines 526-530.
- Empty CMIN: `20 - 0 = 20`.
- `extract_bales_max` receives `empty_capacity_bales=20` at line 534.

### Stage 4 - Density 11 Reduce_Tiberium Result

Verdict: FAIL.

gamemd:
- Starting cell: Riparius ore, `OverlayData=11`.
- Requested amount: 20.
- `CellClass__Reduce_Tiberium` at `0x00480A80` reads `current = OverlayData = 11`.
- Because `20 < 12` is false, it takes the full-removal path: `OverlayTypeIndex=-1`, `OverlayData=0`, `RecalcAttributes`, radar dirty, spread-bitmap maintenance, neighbor reseed.
- Return value is `current`, which is `11`.
- Read-only Ghidra spot-check: `decompile_function 0x00480A80` on 2026-05-23; same finding is documented in `CELLCLASS_REDUCE_TIBERIUM_FUN_00480A80_GHIDRA_REPORT.md`.

Rust:
- Live map overlay seeding converts `entry.frame=11` into `richness = frame.min(11) + 1 = 12`, `stock = 120 * 12 = 1440` in `src/sim/production/production_queue.rs` lines 155-170.
- `extract_bales_max` computes `density_levels = node.remaining / base = 1440 / 120 = 12` at `src/sim/miner/miner_system.rs` lines 819-828.
- It extracts `min(20, 12) = 12`, removes the resource node, and clears the overlay at lines 828-842.

Player-visible difference:
- Same ore cell disappears, but Rust gives one extra carried bale.

### Stage 5 - Cargo Insertion And Carried Value

Verdict: FAIL.

gamemd:
- Since `Reduce_Tiberium` returns 11, `StorageClass__AddAmount((float)11, RipariusSlot0)` runs.
- Carried ore value represented for later deposit is `11 * Riparius.Value(25) = 275` credits.

Rust:
- `extract_bales_max` creates one `CargoBale { resource_type: Ore, value: 25 }` per extracted level at `src/sim/miner/miner_system.rs` lines 831-836.
- `handle_harvest` appends all returned bales at line 537.
- For this live overlay-seeded scenario, Rust appends 12 ore bales, carried value `12 * 25 = 300`.

Player-visible difference:
- The miner will later deposit 25 extra credits from this exact cell if no later correction occurs.

### Stage 6 - Ore Overlay Visual After First Extraction

Verdict: PASS.

gamemd:
- Full-removal path clears `OverlayTypeIndex` and `OverlayData`, recalculates attributes, marks radar terrain dirty, and dirties the tactical screen.
- Result: the ore body no longer renders on that cell.

Rust:
- Full drain removes the `ResourceNode` and calls `OverlayGrid::clear_overlay` at `src/sim/miner/miner_system.rs` lines 838-842.
- `clear_overlay` sets the cell to default and pushes the dirty cell at `src/sim/overlay_grid.rs` lines 92-98.
- `build_overlay_instances` skips rendering a resource overlay when live `OverlayGrid` has no overlay at `src/app_instances/overlays.rs` lines 257-269.

Player-visible result:
- The ore disappears after the first extraction in both engines.

### Stage 7 - Reduce_Tiberium Full-Removal Queue Side Effects

Verdict: NOT-IMPLEMENTED.

gamemd:
- Full removal clears this cell's spread bitmap entry for all tiberium types, then checks the 8 valid neighbors and adds them to this Riparius type's spread queue. This is active in standard YR; `UnitClass__Harvest_Ore_Tick` is a live caller.
- Evidence: `CELLCLASS_REDUCE_TIBERIUM_FUN_00480A80_GHIDRA_REPORT.md` and read-only Ghidra spot-check of `0x00480A80`.

Rust:
- The miner extraction path does not call `reduce_tiberium`; it uses `extract_bales_max`.
- `extract_bales_max` removes the node/overlay only, with no growth/spread queue reseed at `src/sim/miner/miner_system.rs` lines 804-855.
- The current ore growth implementation is a scan/reservoir system described as RA1-derived, not gamemd's RA2/YR tiberium priority queues, in `src/sim/ore_growth.rs` lines 1-15 and 114-160.

Player-visible difference:
- Depleted patches can regrow/spread on different timing and source-cell selection after a miner empties a cell.

### Stage 8 - Selected Cargo Pips

Verdict: UNCHECKED.

Rust exact value:
- Rust renders selected CMIN cargo pips only for `PipScale=Tiberium` and miner entities at `src/app_ui_overlays.rs` lines 672-684.
- `cargo_pips()` returns `(cargo.len() * 5) / capacity_bales`, floored, capped at 5 in `src/sim/miner/mod.rs` lines 349-356.
- After this Rust extraction: `(12 * 5) / 20 = 3` filled pips.

gamemd exact pip formula:
- Not fully traced in this slot. CMIN definitely uses `PipScale=Tiberium`, but the exact selected-unit tiberium pip count formula was not decompiled here.

Risk:
- If gamemd uses the same 5-pip proportional floor behavior, its `11/20` storage would show 2 filled pips while Rust shows 3. This is a likely visible symptom, but it remains UNCHECKED until `UnitClass::DrawPips` / `DrawPipScalePips` is traced numerically.

## Adjacent Findings

- Existing Rust tests around `extract_bales_max` seed "11 density levels" as `11 * 120`, for example `src/sim/miner/miner_tests.rs` lines 3727-3740. That is not the same as a real map overlay cell with `OverlayData=11`, which `seed_resource_nodes_from_overlays` converts to 12 levels. This test-data naming can hide the live overlay mismatch.
- `src/sim/miner/mod.rs` `reduce_tiberium` claims to mirror `CellClass::Reduce_Tiberium`, but it models density as `remaining/base` and lacks overlay/radar/spread side effects. It is used by combat/smudge paths, not by miner harvest.

## Sources

- `docs/research/CELLCLASS_REDUCE_TIBERIUM_FUN_00480A80_GHIDRA_REPORT.md`
- `docs/research/ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/miner/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`
- Read-only Ghidra MCP `decompile_function 0x0073D450` (`UnitClass__Harvest_Ore_Tick`)
- Read-only Ghidra MCP `decompile_function 0x00480A80` (`CellClass__Reduce_Tiberium`)
- `ini/rulesmd.ini`
- `src/sim/miner/miner_system.rs`
- `src/sim/miner/mod.rs`
- `src/sim/production/production_queue.rs`
- `src/app_instances/overlays.rs`
- `src/app_ui_overlays.rs`
