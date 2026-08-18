# CMIN Lifecycle Max-Density Riparius Harvest Trace - 2026-05-27

Scope: standard YR Allied Chrono Miner (`CMIN`) is already in the harvest slice on a harvestable max-density Riparius ore cell, has empty cargo, no active movement destination, and the first harvest gate fires. This trace covers only that one pickup: gate, request amount, `Reduce_Tiberium`, cargo/value insertion, overlay result, and selected cargo-pip implication.

Non-scope: ore acquisition, drive/teleport return, refinery unload/deposit, short-scan retargeting after this pickup, slave miners, TS weeders, combat/crater ore reduction, and runtime framebuffer capture.

## Evidence Used

- `docs/research/HARVEST_ORE_TICK_TIMING_PARTIAL_FULL_EDGE_CASES_ORE_GEMS_GHIDRA_REPORT.md`
- `docs/research/REDUCE_TIBERIUM_FULL_REMOVAL_SIDE_EFFECTS_AND_RETURN_VALUE_GHIDRA_REPORT.md`
- `docs/research/CELLCLASS_REDUCE_TIBERIUM_FUN_00480A80_GHIDRA_REPORT.md`
- `docs/research/CURRENT_RUST_TIBERIUM_INTEGRATION_GAPS_AND_OWNERSHIP_GHIDRA_REPORT.md`
- `docs/research/units/allied/CMIN.md`
- `ini/rulesmd.ini`, `ini/rules.ini`
- `src/sim/miner/miner_system.rs`
- `src/sim/miner/mod.rs`
- `src/sim/production/production_queue.rs`
- `src/app_ui_overlays.rs`

No Ghidra mutation was performed. No Rust, INI, or published research docs were modified.

## Active Standard YR Check

This path is active in standard YR. Existing verified reports state `UnitClass::Mission_Harvest @ 0x0073E5E0` calls `UnitClass::Harvest_Ore_Tick @ 0x0073D450` in state 1, and `Harvest_Ore_Tick` calls `CellClass::Reduce_Tiberium @ 0x00480A80` for ordinary harvesters. Stock `CMIN` has `Harvester=yes`, `Storage=20`, `PipScale=Tiberium`, and no `Weeder=yes`; stock Riparius is tiberium type index 0 with `Value=25`. The TS Weeder branch is excluded for this scenario.

## Concrete Inputs

- Unit: `CMIN`, standard Allied Chrono Miner.
- Cargo before pickup: `0` storage units / Rust bales.
- Capacity: `20`.
- Ore type: Riparius, tiberium index `0`, value `25`.
- Current cell: LandType `5` in gamemd; Rust resource node present at the miner cell.
- Max-density overlay: gamemd `OverlayData=11`; Rust real map seeding currently converts overlay frame `11` into `remaining = 120 * (11 + 1) = 1440`, i.e. `12` Rust density levels.
- Movement destination: none. The gamemd destination-present shortcut is not active.

## Pipeline

`Mission_Harvest state 1 timer gate` -> `Harvest_Ore_Tick request` -> `CellClass::Reduce_Tiberium` -> `StorageClass::AddAmount` -> `selected cargo pips/render implication`

## Stage Results

### Stage 1 - First Harvest Gate

Rust: arrival sets `harvest_timer = config.harvest_tick_interval` at `src/sim/miner/miner_system.rs:469-473`; `handle_harvest` decrements while positive and returns at `src/sim/miner/miner_system.rs:520-524`. With the stock default `18`, extraction occurs after the countdown has already reached zero, making the first pickup one Rust tick later than the verified gamemd gate.

gamemd: `Mission_Harvest` state 1 initializes a StepTimer using `HarvesterLoadRate=2`, waits while the step counter is `< 9`, then calls `Harvest_Ore_Tick`; stock first gate is `9 * 2 = 18` frames.

Verdict: FAIL. The player sees the first ore pickup one frame late.

### Stage 2 - Amount Requested From The Cell

Rust: empty capacity is `capacity_bales - cargo.len() = 20 - 0 = 20` at `src/sim/miner/miner_system.rs:526-534`, passed to `extract_bales_max` as `empty_capacity_bales`.

gamemd: `Harvest_Ore_Tick` computes `ftol(Storage - StorageClass::GetTotalAmount()) = ftol(20 - 0.0) = 20`, then passes `20` to `CellClass::Reduce_Tiberium`.

Verdict: PASS for this integer empty-cargo scenario.

### Stage 3 - Reduce_Tiberium Return / Extraction Count

Rust: real overlay seeding uses `richness = entry.frame.min(11) + 1` at `src/sim/production/production_queue.rs:155`, so overlay frame `11` becomes `12` levels. `extract_bales_max` computes `density_levels = node.remaining / base = 1440 / 120 = 12`, then `n = min(20, 12) = 12` at `src/sim/miner/miner_system.rs:830-840`.

gamemd: for `OverlayData=11`, `Reduce_Tiberium(20)` takes the full-removal path and returns `11`, not `12`. The verified report calls this out as the max-density edge: empty CMIN request `20` on `OverlayData=11` adds exactly `11.0` Riparius storage.

Verdict: FAIL. Rust overharvests one Riparius unit from a real max-density overlay-backed cell.

### Stage 4 - Cargo / Value Insertion

Rust: `handle_harvest` extends cargo with the returned vector at `src/sim/miner/miner_system.rs:536-538`. For this real-overlay Rust state, that is `12` `CargoBale { resource_type: Ore, value: 25 }`, total carry value `300`.

gamemd: `StorageClass::AddAmount((float)removed, tib_type)` receives `removed=11`, `tib_type=0` Riparius. Deposit value later is `11 * 25 = 275`; no credits are added during harvest.

Verdict: FAIL. The player eventually receives 25 extra credits from this one cell pickup, and the miner becomes one unit fuller than gamemd.

### Stage 5 - Overlay Clearing / Remaining Density

Rust: `remaining_after = 1440 - 12 * 120 = 0`; the resource node is removed and `OverlayGrid::clear_overlay` runs at `src/sim/miner/miner_system.rs:849-853`.

gamemd: `Reduce_Tiberium` full-removal path writes `OverlayTypeIndex=-1`, writes `OverlayData=0`, calls `RecalcAttributes`, marks radar terrain dirty, clears all-type spread bitmaps, reseeds valid neighbors for the current type, dirties tactical screen, and returns `11`.

Verdict: NOT-IMPLEMENTED for the full side-effect bundle. The visible overlay disappears in both, but Rust does not implement the same atomic terrain/radar/tactical/queue side effects at this reduction boundary.

### Stage 6 - Density-11 Growth Queue Detour

Rust: no density-11 growth queue detour exists in `extract_bales_max`.

gamemd: `Reduce_Tiberium` calls `TiberiumClass::AddToGrowthQueue` before clearing when `OverlayData==11`, but the callee checks the still-current density and the verified net effect is no growth queue entry for this full-removal `OverlayData=11` case.

Verdict: PASS for the net result in this concrete scenario. The mechanism differs, but the verified queue insertion output for this one pickup is `0` on both sides.

### Stage 7 - Selected Cargo Pip Implication

Rust: `Miner::cargo_pips()` computes `(cargo.len() * 5) / capacity` at `src/sim/miner/mod.rs:403-410`; `12` bales in a CMIN gives `(12 * 5) / 20 = 3` filled cargo pips. Selected rendering uses that count at `src/app_ui_overlays.rs:702-705`.

gamemd: stock `CMIN` has `PipScale=Tiberium`, and the correct storage after this pickup is `11/20`. The exact selected-unit cargo pip formula/frame count for `11/20` was explicitly out of scope in the verified harvest reports and was not recomputed in this trace.

Verdict: UNCHECKED. The Rust overharvest changes the input to pip rendering from `11/20` to `12/20`; whether the visible pip count differs depends on the exact gamemd selected-pip formula, which remains unverified here.

## Failures

1. First pickup timing is one Rust tick late because the countdown decrements before extraction instead of firing exactly on the verified 18-frame gate.
2. Real max-density overlay-backed Riparius cells overharvest by one unit: Rust extracts `12`, gamemd extracts `11`.
3. Cargo/value insertion follows the overharvest: Rust carries `12 * 25 = 300` value, gamemd carries `11 * 25 = 275`.
4. Full-removal side effects are incomplete: Rust clears node/overlay but lacks same-call `RecalcAttributes`, radar dirty, tactical dirty, all-type spread bitmap clear, and neighbor spread reseed semantics.

## Adjacent Findings

- Exact selected cargo pip rendering for `11/20` CMIN storage still needs a dedicated pip-rendering investigation. This trace only proves the harvest output that feeds it.
- The existing direct-resource-node tests do not prove real overlay-frame parity because they bypass `seed_resource_nodes_from_overlays`.
- Fixing only overlay seeding is risky unless harvest, combat, smudge, and growth all share one authoritative `Reduce_Tiberium`-equivalent boundary.

## Verdict Tally

PASS: 2 | FAIL: 3 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Status

COMPLETE.
