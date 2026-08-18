# CMIN Max-Density Riparius Harvest / Weeder Exclusion Trace

Date: 2026-05-24

Scenario: Empty standard YR Allied Chrono Miner (`CMIN`) is already in `Mission_Harvest` state 1 / Rust `MinerState::Harvest`, standing on a Riparius ore cell with `OverlayData=11`, no active movement destination, and enough free storage.

Scope: one extraction gate only: harvest timer gate, active standard-YR branch selection, `Reduce_Tiberium` amount/result, cargo/storage amount, overlay clearing, timer reset behavior, and stock `CMIN` exclusion from the TS `Weeder` branch.

## Verdict

FAIL overall for current Rust parity.

The TS `Weeder` concern is resolved for this concrete stock scenario: `gamemd.exe` contains a `Weeder` branch inside `UnitClass::Harvest_Ore_Tick`, but stock `[CMIN]` has `Harvester=yes` and no `Weeder=yes`, so the active YR path is the normal ore/gem branch. The main Rust mismatch remains the max-density cell amount: gamemd removes the `OverlayData=11` cell and adds 11.0 Riparius storage units, while Rust overlay seeding can make the same real map cell harvest as 12 bales.

Tally: PASS: 3 | FAIL: 3 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Pipeline

Harvest-state CMIN on ore -> 9-step harvest gate -> `Harvest_Ore_Tick` normal ore branch -> `Reduce_Tiberium(20)` -> cell full removal -> Riparius storage add -> timer reset / next harvest wait -> overlay and carried-value presentation.

## Stage Results

### Stage 1 - Stock YR Data And Weeder Exclusion

Verdict: PASS.

gamemd/YR:
- `[CMIN]` has `Harvester=yes`, `PipScale=Tiberium`, `Storage=20`, and `Teleporter=yes` in `ini/rulesmd.ini:7351-7398`.
- `[CMIN]` does not set `Weeder=yes`.
- `[Tiberiums] 0=Riparius`; `[Riparius] Value=25` in `ini/rulesmd.ini:30372-30396`.
- Read-only Ghidra spot-check of `UnitClass::Harvest_Ore_Tick @ 0x0073D450` shows `UnitType+0xE0E` gates the standard harvester path, then `UnitType+0xE0F` selects the TS `Weeder` branch only when nonzero.

Rust:
- `miner_kind_for_object` maps `Harvester=yes` plus `Teleporter=yes` to `MinerKind::Chrono` in `src/sim/miner/mod.rs:368`.
- `Miner::new` honors object `Storage=` as cargo capacity; stock CMIN capacity is 20 in `src/sim/miner/mod.rs:304`.
- No Rust standard CMIN path applies a weed-specific `HarvesterLoadRate * 3` behavior.

### Stage 2 - Harvest Gate Timing

Verdict: FAIL.

gamemd:
- `Mission_Harvest @ 0x0073E5E0` state 1 initializes the StepTimer with `HarvesterLoadRate`, then calls `Harvest_Ore_Tick` only after 9 StepTimer steps.
- Stock/default `HarvesterLoadRate=2`, so first extraction gate is 18 frames after timer initialization.
- This path is active in standard YR; xrefs show the direct caller of `Harvest_Ore_Tick` is `UnitClass::Mission_Harvest` at `0x0073E987`.

Rust:
- `MinerConfig::from_general_rules` uses `HarvesterLoadRate` default 2 and computes `2 * 9 = 18` in `src/sim/miner/mod.rs:217`.
- Arrival into harvest state sets `harvest_timer = config.harvest_tick_interval`; `handle_harvest` decrements while positive and returns in `src/sim/miner/miner_system.rs:520`.
- With `harvest_timer=18`, extraction occurs after 18 decrement returns, on the following tick. That is one sim tick later than the 18-frame gate.

Player-visible difference: first pickup after reaching ore is delayed by one 15 Hz sim tick, about 66.7 ms.

### Stage 3 - Requested Removal Amount

Verdict: PASS.

gamemd:
- Normal ore/gem branch reads `Storage=20`, calls `StorageClass::GetTotalAmount`, computes `20.0 - 0.0`, converts through `Math__ftol`, then calls `CellClass::Reduce_Tiberium(20)`.
- Concrete request: `20`.

Rust:
- `handle_harvest` computes `empty = capacity_bales - cargo.len()` in `src/sim/miner/miner_system.rs:526`.
- Empty stock CMIN: `20 - 0 = 20`.
- `extract_bales_max(..., empty_capacity_bales=20)` receives the same requested capacity clamp in `src/sim/miner/miner_system.rs:534`.

### Stage 4 - `OverlayData=11` Reduction Result

Verdict: FAIL.

gamemd:
- `CellClass::Reduce_Tiberium @ 0x00480A80` reads current `OverlayData=11`.
- The full-removal branch runs because request `20 >= current + 1` (`12`).
- Full removal writes `OverlayTypeIndex=-1`, writes `OverlayData=0`, calls `RecalcAttributes`, marks radar terrain dirty, clears spread bitmap membership, reseeds neighbors, dirties tactical, and returns the pre-removal current value.
- Concrete return: `11`, not `12`.

Rust:
- Real map overlay seeding converts `entry.frame=11` into `richness = frame.min(11) + 1 = 12`, then `stock = 120 * 12 = 1440` in `src/sim/production/production_queue.rs:155`.
- `extract_bales_max` computes `density_levels = 1440 / 120 = 12`, then extracts `min(20, 12) = 12` in `src/sim/miner/miner_system.rs:824`.

Player-visible difference: the same max-density ore cell disappears, but Rust grants one extra carried ore bale.

### Stage 5 - Cargo / Storage Amount And Future Credits

Verdict: FAIL.

gamemd:
- Since `Reduce_Tiberium` returns 11, `Harvest_Ore_Tick` calls `StorageClass::AddAmount((float)11, RipariusSlot0)`.
- Future deposit value for this pickup is `11 * Riparius.Value(25) = 275` credits.

Rust:
- `extract_bales_max` creates one `CargoBale { resource_type: Ore, value: 25 }` per extracted level in `src/sim/miner/miner_system.rs:831`.
- For the real overlay-seeded max cell, Rust appends 12 ore bales.
- Future deposit value represented by carried bales is `12 * 25 = 300` credits.

Player-visible difference: this one pickup can overpay by 25 credits after refinery deposit.

### Stage 6 - Overlay Clearing

Verdict: PASS.

gamemd:
- The full-removal branch clears overlay type/data during the same `Reduce_Tiberium` call, so the ore cell no longer renders as ore.

Rust:
- `extract_bales_max` removes the resource node and calls `OverlayGrid::clear_overlay` on full drain in `src/sim/miner/miner_system.rs:838`.

Observable result: the ore disappears in both engines after this extraction gate.

### Stage 7 - Full-Removal Side Effects Beyond Visual Clearing

Verdict: NOT-IMPLEMENTED.

gamemd:
- Full removal synchronously calls terrain/radar/tactical dirtying and tiberium spread bitmap/neighbor reseed logic inside `CellClass::Reduce_Tiberium`.
- This is active for stock CMIN because `Harvest_Ore_Tick` calls `Reduce_Tiberium` on the normal non-Weeder ore branch.

Rust:
- The miner path uses `extract_bales_max`, not the existing `miner::reduce_tiberium` helper.
- `extract_bales_max` clears `resource_nodes` and overlay only; it does not perform the RA2/YR tiberium queue reseed or same-call radar/tactical side-effect boundary.

Player-visible risk: depleted ore patches can affect later growth/spread, minimap/dirtying, and terrain-observer timing differently.

### Stage 8 - Selected Cargo Pips

Verdict: UNCHECKED.

Rust exact value:
- `cargo_pips()` computes `(cargo.len() * 5) / capacity_bales`, floored, in `src/sim/miner/mod.rs:351`.
- With Rust's 12 carried bales: `(12 * 5) / 20 = 3` pips.

gamemd:
- This slot did not decompile selected-unit tiberium pip drawing. With gamemd's 11.0 storage, a same proportional floor formula would produce 2 pips, but the literal gamemd pip formula was not computed here.

## Adjacent Findings

- Existing Rust tests that seed `11 * 120` as "11 density levels" are not equivalent to a real map overlay with `OverlayData=11`, because `seed_resource_nodes_from_overlays` currently maps that real overlay to 12 levels.
- The existing `src/sim/miner/mod.rs` `reduce_tiberium` helper does not currently represent the full `CellClass::Reduce_Tiberium` side-effect contract and is not used by miner harvest extraction.

## Sources

- Read-only Ghidra MCP `decompile_function 0x0073D450` (`UnitClass::Harvest_Ore_Tick`)
- Read-only Ghidra MCP `decompile_function 0x00480A80` (`CellClass::Reduce_Tiberium`)
- Read-only Ghidra MCP `get_function_xrefs 0x0073D450`
- `C:/Users/enok/Documents/ra2-rust-game-docs/HARVEST_ORE_TICK_TIMING_PARTIAL_FULL_EDGE_CASES_ORE_GEMS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/REDUCE_TIBERIUM_FULL_REMOVAL_SIDE_EFFECTS_AND_RETURN_VALUE_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/CMIN_HARVEST_DENSITY_CARGO_REDUCE_TIBERIUM_TRACE.md`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game/src/rules/ruleset.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_system.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_queue.rs`
