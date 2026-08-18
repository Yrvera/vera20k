# Riparius Growth/Spread Queue Standard YR Trace

Scenario: A standard YR map has a seeded Riparius ore patch with growth/spread enabled. Trace one queue-backed growth/spread processing opportunity through gamemd.exe and current Rust.

## Verdict

Status: COMPLETE

PASS: 2 | FAIL: 6 | UNCHECKED: 2 | NOT-IMPLEMENTED: 3

The mechanism is standard YR-active, not TS-only. The word `TiberiumClass` is inherited engine terminology, but stock `rulesmd.ini` defines `[Tiberiums] 0=Riparius`, `[Riparius] Growth=2200`, `GrowthPercentage=.06`, `Spread=2200`, and `SpreadPercentage=.06`, and the verified gamemd tick path calls the queue drivers in standard skirmish when ore growth/spread are enabled.

Current Rust does not implement that model. It implements an RA1-style full-map scan/reservoir system over `ResourceNode`s and derives new ore as level 1/frame 0. A YR spread opportunity creates a new Riparius cell through `PlaceTiberium(tib_type, 3)`, writes exact `OverlayData=3`, chooses a random flat overlay variant from the Riparius image range, and inserts the new cell into the growth queue.

## Pipeline

Standard YR:

`rulesmd.ini` Riparius data -> map overlay pack seeds ore cells -> `ScenarioClass::Full_Init` initializes per-type growth/spread queues -> live logic tick calls all-type growth driver then all-type spread driver -> mature Riparius processor consumes heap entries in a percentage batch -> growth calls `GrowTiberium` and feeds spread queue -> spread validates neighbors and calls `PlaceTiberium(type=Riparius, density=3)` -> overlay/radar dirty state becomes visible.

Current Rust:

`rulesmd.ini` general flags -> `seed_resource_nodes_from_overlays` creates ore/gem `ResourceNode`s -> `OreGrowthState` owns scan cursor and candidate vectors -> `Simulation::advance_tick` calls `tick_ore_growth` -> scanner collects candidates by cursor chunk/reservoir -> at scan wrap, growth adds `120` stock and spread inserts a new level-1 ore node, copying the source overlay id when present.

## Findings

1. PASS - Standard YR liveness is confirmed.
   Evidence: `rulesmd.ini:43-45` enables `GrowthRate=5`, `TiberiumGrows=yes`, `TiberiumSpreads=yes`; `rulesmd.ini:30372-30396` defines Riparius as tiberium type 0 with growth/spread percentages `.06`. The queue report marks `LogicClassPerTickUpdateLiveVector -> GrowthDriver_AllTypes -> SpreadDriver_AllTypes` active in standard YR skirmish.

2. NOT-IMPLEMENTED - Rust has no per-Riparius queue state.
   Gamemd: each `TiberiumClass` owns spread/growth heap pointers, entry arrays, entry counts, bitmaps, and timers. Map init rebuilds both queues after overlay packs are read.
   Rust: `ProductionState` stores `resource_nodes`, one `OreGrowthConfig`, and one `OreGrowthState` scanner at `src/sim/production/production_types.rs:203-210`; `seed_resource_nodes_from_overlays` only creates stock nodes at `src/sim/production/production_queue.rs:132-174`.

3. FAIL - Tick cadence and trigger model differ.
   Gamemd: live tick order is growth driver first, spread driver second; per-type timers use Riparius `Growth=2200` and `Spread=2200`, with processors only firing when those intervals mature.
   Rust: `Simulation::advance_tick` calls a single `tick_ore_growth` at `src/sim/world/mod.rs:1545-1553`; the scanner uses `[General] GrowthRate=5` converted to `300` seconds and a full-map scan cycle at `src/sim/ore_growth.rs:83-88` and `src/sim/ore_growth.rs:171-179`.

4. FAIL - Candidate ownership and batch consumption differ.
   Gamemd: Growth and spread processors pop persistent heap entries. Batch base is `floor(heap_count * 0.06)` clamped to `[5,50]` for growth and `[5,25]` for spread, then `Random::Next() % batch + 1`.
   Rust: candidates are discovered by scanning `resource_nodes` in cursor order and sampled into bounded vectors with `MAX_CANDIDATES=50` at `src/sim/ore_growth.rs:184-214` and `src/sim/ore_growth.rs:270-289`.

5. FAIL - Growth side effects differ.
   Gamemd: growth pops a Riparius growth queue entry, calls `CellClass::GrowTiberium`, then if the resulting density is still `< 11`, reinserts into the growth queue with `currentFrame + Random % 50` and calls `AddToSpreadQueue`.
   Rust: growth mutates stock by `+120`, updates overlay data from `remaining / 120 - 1`, and has no growth bitmap, priority, or spread-queue handoff at `src/sim/ore_growth.rs:220-238`.

6. FAIL - Spread new-cell density differs.
   Gamemd: standard spread calls `CellClass::PlaceTiberium(tib_type, 3)`; the germinate branch writes exact `OverlayData=3`, adds the new cell to the growth queue, and dirties radar/tactical terrain.
   Rust: `try_spread_ore` inserts `ResourceNode { resource_type: Ore, remaining: 120 }` and places overlay data `0` by copying the source overlay id at `src/sim/ore_growth.rs:321-334`.

7. FAIL - Target validation differs.
   Gamemd: `CanPlaceTiberium` requires playfield, no bridge/rail mask, building exception handling, no blocking `SpawnsTiberium` terrain object, buildable land type, no existing overlay, flat slope, and theater `AllowTiberium`.
   Rust: `can_germinate` only rejects existing resource nodes and non-walkable path-grid cells at `src/sim/ore_growth.rs:346-361`.

8. FAIL - Per-type data is collapsed.
   Gamemd: Riparius, Cruentus, Vinifera, and Aboreus are per-type `TiberiumClass` entries; this scenario uses Riparius index 0 and `.06` percentages.
   Rust: growth/spread hardcodes `ResourceType::Ore` eligibility and constants `ORE_BASE_PER_LEVEL=120`, `MAX_ORE_LEVELS=12`, `SPREAD_THRESHOLD=720` at `src/sim/ore_growth.rs:30-39` and `src/sim/ore_growth.rs:190-207`.

9. UNCHECKED - Exact RNG output for this concrete map opportunity.
   The formula is verified, but this trace did not compute a specific heap count, current frame, or native RNG state for a named map coordinate. Therefore the exact number of popped entries and the selected neighbor direction are UNCHECKED, not PASS.

10. UNCHECKED - Exact pixel/frame presentation after dirty propagation.
    Gamemd dirty/radar calls are verified for new germination, and Rust overlay dirty plumbing exists elsewhere, but this trace did not run a visual frame comparison. The sim-side density/overlay-id mismatch is already enough to fail visible parity.

11. NOT-IMPLEMENTED - Depletion/growth queue reseed target is absent here.
    Gamemd `Reduce_Tiberium` and successful growth feed spread queues via per-type bitmap helpers. Current Rust has no equivalent destination for those reseed operations, so later patch recovery behavior cannot match this queue-backed scenario.

12. NOT-IMPLEMENTED - YR queue/timer state is not represented in deterministic state.
    Current `world_hash.rs` hashes `resource_nodes` but not any YR-style queue/bitmap/timer state at `src/sim/world/world_hash.rs:173-178`. Since the YR queue model is absent, there is no parity-equivalent state to hash or snapshot.

## TS/YR Boundary

This trace did not use TS `Weeder`, weed, vein, or TS fog behavior. The active facts come from `gamemd.exe` YR queue reports, `rulesmd.ini`, and current Rust source. `Cruentus` gems are active data but stock spread/growth processors exit because both percentages are `0`; that adjacent gem behavior is not part of this Riparius trace.

## Evidence Notes

- Primary reports: `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`, `PLACETIBERIUM_SPREAD_GERMINATION_CONSTRAINTS_AND_OVERLAY_FRAME_GHIDRA_REPORT.md`, `CELLCLASS_PLACETIBERIUM_FUN_00487190_GHIDRA_REPORT.md`.
- INI evidence: `ini/rulesmd.ini:43-45`, `ini/rulesmd.ini:30372-30396`.
- Rust evidence: `src/sim/ore_growth.rs`, `src/sim/world/mod.rs`, `src/sim/production/production_queue.rs`, `src/sim/production/production_types.rs`, `src/sim/world/world_hash.rs`.
- Read-only Ghidra MCP spot-check was attempted for the queue addresses, but the current MCP session returned `Function not found` for those addresses. No Ghidra mutations were attempted.
