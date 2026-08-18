# CMIN Cruentus Gem Harvest Non-Weeder Branch Trace

**Date:** 2026-05-24
**Scenario:** Standard YR Allied `CMIN`, empty cargo, enough free storage, standing on a stock GEM/Cruentus overlay cell with `OverlayData=11`, no movement destination. Trace one extraction gate only.

## Verdict

PASS: 3 | FAIL: 3 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

The TS concern is resolved for this slice: stock `CMIN` and `HARV` use the standard `Harvester=yes`, `Weeder=no` ore/gem branch. Gems are not a special harvester path and not the TS weed path. They are YR `Cruentus` tiberium type index `1`, later valued at `50` credits per removed unit.

The player-visible mismatch is still real: current Rust over-harvests a real max-frame GEM overlay by one unit. For `OverlayData=11`, gamemd stores 11 Cruentus units; Rust's overlay seeding creates 12 gem bales.

## Evidence

- Stock INI: `ini/rulesmd.ini:7351-7399` has `[CMIN] Harvester=yes`, `Storage=20`, `PipScale=Tiberium`, `Teleporter=yes`, and no `Weeder=yes`.
- Stock INI: `ini/rulesmd.ini:8215-8236` has `[HARV] Harvester=yes`, `Storage=40`, `PipScale=Tiberium`, and no `Weeder=yes`.
- Stock INI: `ini/rulesmd.ini:30372-30407` maps `[Tiberiums] 1=Cruentus`; `[Cruentus] Value=50`, `GrowthPercentage=0`, `SpreadPercentage=0`.
- Read-only Ghidra: `UnitClass::Mission_Harvest @ 0x0073E5E0` state 1 is the live caller of `UnitClass::Harvest_Ore_Tick @ 0x0073D450`.
- Read-only Ghidra: `Harvest_Ore_Tick` gates on `Harvester`, storage not full, current `LandType==5`, then branches to TS weed only if `UnitTypeClass+0xE0F Weeder` is set.
- Read-only Ghidra: standard ore/gem branch calls `CellClass::GetTiberiumType`, computes `ftol(Storage - GetTotalAmount)`, calls `CellClass::Reduce_Tiberium`, then `StorageClass::AddAmount(removed, tib_type)`.
- Read-only Ghidra: `CellClass::OverlayToTiberiumIndex` resolves the overlay into the matching `TiberiumClass` image range and returns that type index; stock GEM overlays map to Cruentus index `1`.
- Read-only Ghidra: `CellClass::Reduce_Tiberium @ 0x00480A80` full-removal return uses pre-removal `OverlayData`; if `OverlayData=11` and request is `20`, it returns `11`, clears overlay type/data, recalculates attributes, radar-dirties, clears spread bitmap entries, reseeds neighbors for the removed type, and tactical-dirties.
- Existing verified report: `HARVEST_ORE_TICK_TIMING_PARTIAL_FULL_EDGE_CASES_ORE_GEMS_GHIDRA_REPORT.md` independently records that gems use Cruentus storage/value and standard `CMIN`/`HARV` skip Weeder.

## Concrete Values

1. Branch selection: gamemd stock `CMIN` has `Harvester=1`, `Weeder=0`; current Rust classifies `Harvester=yes + Teleporter=yes` as `MinerKind::Chrono` and has no Weeder branch in `src/sim/miner/mod.rs:371-385`. Verdict: PASS.
2. Type/value: gamemd GEM overlay maps to Cruentus type index `1`, value `50`; Rust seeds `GEM*` overlays as `ResourceType::Gem` and uses `gem_bale_value=50` at `src/sim/production/production_queue.rs:149-170` and `src/sim/miner/mod.rs:184-200`. Verdict: PASS for value/type distinction.
3. Request amount: gamemd empty `CMIN` computes `ftol(20 - 0) = 20`; Rust empty CMIN capacity is 20 and `handle_harvest` passes `empty=20` to `extract_bales_max` at `src/sim/miner/miner_system.rs:526-534`. Verdict: PASS.
4. Overlay reduction/removal: gamemd `Reduce_Tiberium(20)` on `OverlayData=11` removes the overlay and returns `11`; Rust seeds frame `11` as `remaining=12*180=2160`, so `extract_bales_max` removes `12` gem bales and clears the overlay at `src/sim/production/production_queue.rs:155-157` and `src/sim/miner/miner_system.rs:819-842`. Verdict: FAIL.
5. Cargo/storage after one extraction: gamemd calls `StorageClass::AddAmount(11.0, 1)`, so Cruentus slot increases by 11.0; Rust appends 12 `CargoBale { resource_type: Gem, value: 50 }`. Verdict: FAIL.
6. Later base credit basis: gamemd later credits `11 * Cruentus.Value(50) = 550` before purifier modifiers; Rust cargo/deposit basis is `12 * 50 = 600` through `phase_unloading` at `src/sim/miner/miner_dock_sequence.rs:815-842`. Verdict: FAIL.
7. Full side-effect bundle: gamemd clears overlay type/data and immediately runs recalc/radar/tactical/spread-queue side effects. Rust clears the overlay/resource node, but immediate terrain/radar/tactical/queue equality was not computed for this gem scenario. Verdict: UNCHECKED.
8. Exact first extraction frame: existing evidence says gamemd reaches the first extraction gate after the state-1 9-step timer; current Rust decrements `harvest_timer` before extraction. This trace did not recompute the arrival-to-extraction frame count for a GEM cell. Verdict: UNCHECKED.

## Player-Visible Findings

### FAIL 1 - Max-density gems overpay by one unit

For a real GEM overlay with `OverlayData=11`, gamemd removes and stores 11 Cruentus units. Rust's overlay seeding treats frame 11 as 12 harvestable gem bales, so a CMIN receives 12 gems from the same cell. The player can receive 600 credits instead of 550 from that one cell before bonuses.

Rust evidence: `src/sim/production/production_queue.rs:155-157`, `src/sim/miner/miner_system.rs:824-842`, `src/sim/miner/miner_tests.rs:4255-4273`.
gamemd evidence: `CellClass::Reduce_Tiberium @ 0x00480A80`; `Harvest_Ore_Tick @ 0x0073D450`; `rulesmd.ini [Cruentus] Value=50`.

### FAIL 2 - Cargo/storage unit count diverges

gamemd adds one storage-slot update of `11.0` to tiberium type index `1`. Rust appends 12 discrete Gem cargo bales. This is not just an internal-shape difference because the carried amount and later payout differ numerically.

Rust evidence: `src/sim/miner/miner_system.rs:831-836`.
gamemd evidence: `StorageClass::AddAmount(removed, tib_type)` from `Harvest_Ore_Tick`.

### FAIL 3 - Deposit value basis diverges for the harvested cell

gamemd's later base value is `11 * 50 = 550`. Rust's cargo values sum to `12 * 50 = 600` and `phase_unloading` credits the whole slot value. This affects visible credits once the CMIN unloads.

Rust evidence: `src/sim/miner/miner_dock_sequence.rs:815-842`.
gamemd evidence: `rulesmd.ini [Cruentus] Value=50`; storage type index `1` from `GetTiberiumType`.

## Non-Findings

- No TS Weeder path applies to standard `CMIN` or `HARV`; `Weeder=yes` is absent and the gamemd branch is gated by that flag.
- Gems are not "two ore bales"; they are Cruentus storage units with value 50.
- `Cruentus GrowthPercentage=0` and `SpreadPercentage=0` do not disable harvesting; they only keep stock gems from normal growth/spread processing.

## Adjacent Findings

- The same `OverlayData=11` full-removal off-by-one applies to real overlay-backed ore and gems because `Reduce_Tiberium` is type-generic after tiberium type resolution. This report traced only the GEM/Cruentus scenario.
- Immediate full-removal side effects should be centralized in a shared sim tiberium reduction helper, not patched only in the miner path.

## Status

COMPLETE for the requested stock CMIN/Cruentus harvest branch, with two explicitly UNCHECKED timing/side-effect equality details that require a separate frame/dirty-propagation trace.
