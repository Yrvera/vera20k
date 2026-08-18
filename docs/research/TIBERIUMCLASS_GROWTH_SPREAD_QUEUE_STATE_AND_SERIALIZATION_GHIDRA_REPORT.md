# TiberiumClass Growth/Spread Queue State and Serialization - Ghidra Research Report

**Address(es):** `0x007221B0`, `0x00722440`, `0x00722AF0`, `0x00722AB0`, `0x00722240`, `0x007228B0`, `0x00722C40`, `0x00722F00`, `0x007235A0`, `0x007233A0`, `0x00722D00`
**Investigation Mode:** coverage-map
**Claimed Scope:** queue fields, bitmap semantics, init/rebuild/pop/reinsert behavior, live tick integration, map-load seeding, per-type indexing, and Rust ownership implications for standard YR ore growth/spread.
**Non-Scope:** `CellClass::PlaceTiberium` germination internals beyond call contracts; exact native savegame stream layout; exact runtime values of the direction table; visual/radar dirty composition.
**Confidence:** High for queue ownership/timing/bitmap behavior; Medium for native save/load conclusions.
**Active in YR:** Yes for standard skirmish when ore growth/spread are enabled; conditional on `ScenarioClass+0x34A6` / global spread flag and each TiberiumClass percentage.

## Target Question

What TiberiumClass queue state must Rust own, hash, and potentially serialize to reproduce standard YR ore patch growth/spread after miners empty cells?

## Non-Goals

- Do not re-investigate CMIN movement, return, docking, or deposit.
- Do not re-investigate the full `PlaceTiberium` overlay-frame algorithm.
- Do not write Rust code.
- Do not mutate Ghidra labels or structures.

## Evidence Needed To Mark COMPLETE

- Verify live standard-YR entry points for queue initialization and tick drivers.
- Verify per-type queue fields and bitmap fields.
- Verify rebuild, runtime add, pop, and reinsert behavior.
- Verify whether depletion-time reseed can be correct without a queue model.
- Scan current Rust ownership, hash, and snapshot surfaces enough to hand off implementation.

## Stop Conditions

- Stop after queue ownership/timing is proven enough for Rust architecture.
- Defer exact native savegame stream details if no named TiberiumClass save/load function exists in available symbols.
- Defer `PlaceTiberium` candidate-cell internals to slot 3.

## 1. Overview

YR does not use the current Rust RA1-style map scanner/reservoir model for ore growth. Each `TiberiumClass` owns two runtime queues: one for spread and one for growth. Each queue has a min-heap of cell entries plus a one-byte-per-map-cell membership bitmap. Standard map load seeds both queues after overlay packs are read, and the live tick loop calls growth then spread every logic tick.

This means depletion-time reseed cannot be reproduced by a one-off neighbor insert into the current scan model. The reseed only makes sense against persistent per-type spread queues and spread bitmaps.

## 2. Class Layout / Key Offsets

| Offset | Field | Type / shape | Purpose | Active in YR |
|---:|---|---|---|---|
| `+0x98` | ArrayIndex | int | Tiberium type index matched against `CellClass::GetTiberiumType`. | Yes; decompiled rebuild processors compare cells to this index. |
| `+0x9C` | Spread | int | Spread driver interval; stock Riparius `2200`, Cruentus `10000`. | Yes; `0x007221B0` reloads `+0x108` from `+0x9C`. |
| `+0xA0` | SpreadPercentage | double | Gates spread and batch fraction. | Conditional; stock Riparius `.06`, Cruentus `0`. |
| `+0xA8` | Growth | int | Growth driver base interval. | Yes; read through growth timing path. |
| `+0xB0` | GrowthPercentage | double | Gates growth and batch fraction. | Conditional; stock Riparius `.06`, Cruentus `0`. |
| `+0xE4` | MaxDensity | int | Maximum density count, `12`; visible max `OverlayData=11`. | Yes. |
| `+0xF0` | Spread entry count | int | Count of spread entries allocated in `+0xFC`. | Yes. |
| `+0xF4` | Spread heap pointer | heap object pointer | Heap count/capacity/pointer/min/max metadata. | Yes. |
| `+0xF8` | Spread bitmap | byte array pointer | `1` means cell is already represented in spread queue for this tib type. | Yes. |
| `+0xFC` | Spread entries | array pointer | 8-byte `{cell_coord, priority_f32}` entries. | Yes. |
| `+0x100/+0x104/+0x108` | Spread timer | ints | last frame / cached field / interval. | Yes. |
| `+0x10C` | Growth entry count | int | Count of growth entries allocated in `+0x118`. | Yes. |
| `+0x110` | Growth heap pointer | heap object pointer | Heap count/capacity/pointer/min/max metadata. | Yes. |
| `+0x114` | Growth bitmap | byte array pointer | `1` means cell is already represented in growth queue. | Yes. |
| `+0x118` | Growth entries | array pointer | 8-byte `{cell_coord, priority_f32}` entries. | Yes. |
| `+0x11C/+0x120/+0x124` | Growth timer | ints | last frame / cached field / interval. | Yes. |

Queue entry shape is 8 bytes: packed cell coordinate at `+0`, float priority at `+4`.

## 3. Core Logic

### 3.1 Live Tick Order

`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` calls:

1. `TiberiumClass::GrowthDriver_AllTypes @ 0x00722C40`
2. `TiberiumClass::SpreadDriver_AllTypes @ 0x007221B0`

This is active in standard YR skirmish. Evidence: direct decompile of `0x0055AFB0`, with calls near the `0x0055B4D7` block. The older timing note `timing/logic-vs-render-loop.md` agrees.

### 3.2 Map-Load Seeding

`ScenarioClass::Full_Init @ 0x00686B20` reads overlay packs, recalculates cell attributes for every map cell, reads terrain, then calls:

1. `TiberiumClass::InitGrowthQueues_All @ 0x00722D00`
2. `TiberiumClass::InitSpreadQueues_All @ 0x00722240`

This is active for standard Skirmish `g_GameMode==5`. Evidence: decompile of `0x00686B20` shows `ReadMapOverlayPacks`, all-cell `CellClass::RecalcAttributes`, `TerrainClass__Read_Map_Section`, then the two queue init calls.

### 3.3 Queue Init/Rebuild

Both init functions free any prior heap/entry/bitmap storage, allocate arrays sized to map cell count, construct a 0x14-byte heap object, then call the matching rebuild function.

Spread rebuild `0x007228B0`:

- Clears spread entry count and heap count.
- Zeroes the per-cell spread bitmap.
- Iterates every map cell.
- Calls `CellClass::GetTiberiumType`.
- Requires matching `TiberiumClass+0x98`.
- Requires `CellClass::CanSpreadTiberium`.
- Adds entry `{cell_coord, priority=0.0}`.
- Sets spread bitmap for that cell to `1`.

Growth rebuild `0x007233A0` is identical except it uses growth fields and `CellClass::CanGrowTiberium`.

Active in YR: Yes. These functions are called during standard scenario initialization and when processors detect queue capacity pressure.

### 3.4 Drivers

Spread driver `0x007221B0`:

- Exits unless `ScenarioClass+0x34A6` is nonzero.
- Iterates all `g_TiberiumClass_Array` entries.
- Reads last-fire frame `+0x100` and interval `+0x108`.
- Fires when last is `-1`, interval is `0`, or elapsed frames reach interval.
- Calls spread processor, then writes last-fire frame to current frame and reloads interval from `TiberiumClass+0x9C`.

Growth driver `0x00722C40`:

- Same all-type iteration and `ScenarioClass+0x34A6` gate.
- Uses `+0x11C/+0x124`.
- Calls growth processor, then reloads interval via a `Math__ftol` path. Existing timing report identifies this as `Growth * multiplier`, where the standard YR global tiberium growth flag uses a `0.3` multiplier.

Active in YR: Yes for standard skirmish with growth enabled. The drivers are called every logic tick, but processors only run when their per-type intervals mature.

### 3.5 Spread Processor

`TiberiumClass::SpreadProcessor @ 0x00722440`:

- Exits if heap pointer is null, heap count is zero, or `SpreadPercentage <= 0.0`.
- Computes a batch target from heap count times `SpreadPercentage`, clamps it to `[5, 25]`.
- Chooses actual count as `Random__Next() % batch + 1`.
- Rebuilds if heap count is within 20 cells of map cell count.
- Pops heap entries and processes until actual successful spreads reach the chosen count or heap empties.
- For each popped cell, counts 8 neighbors that pass `CanPlaceTiberium`.
- If no neighbor can accept ore, clears this tib type's spread bitmap for the source cell.
- If at least one neighbor exists, calls `CellClass::SpreadTiberium`.
- If more than one valid neighbor exists, reinserts the source cell with priority `0.0` and sets its spread bitmap to `1`.

Tiny details:

- The spread processor has no `priority > currentFrame` gate after popping. Priority orders heap entries, but it is not a wake-up timestamp.
- Spread processor's successful reinsert priority is literal `0.0`, unlike runtime `AddToSpreadQueue`.
- The source cell is reinserted only when valid-neighbor count is greater than 1, not when it is exactly 1.
- The bitmap clear on no-neighbor is for the popped source cell and this tib type only.

Active in YR: Yes for Riparius/Vinifera/Aboreus where `SpreadPercentage > 0`; stock Cruentus has `SpreadPercentage=0`, so its processor exits.

### 3.6 Growth Processor

`TiberiumClass::GrowthProcessor @ 0x00722F00`:

- Exits if heap pointer is null, heap count is zero, or `GrowthPercentage <= 0.0`.
- Computes a batch target from heap count times `GrowthPercentage`, clamps it to `[5, 50]`.
- Chooses actual count as `Random__Next() % batch + 1`.
- Rebuilds if heap count is too close to map cell count after accounting for `actual * 2`.
- Pops heap entries.
- If popped cell still maps to this tib type, calls `CellClass::GrowTiberium`.
- If resulting density is still `< 11`, reinserts cell into growth queue with priority `currentFrame + Random__Next() % 50`, sets growth bitmap, and calls `AddToSpreadQueue` for the same cell.
- If density is `>= 11`, clears growth bitmap and does not reinsert.

Tiny details:

- Growth calls `CellClass::GrowTiberium` before checking whether the resulting density is still `< 11`.
- Growth feeds spread after a successful still-growable growth result.
- Like spread, growth does not defer popped entries whose priority is greater than current frame.

Active in YR: Yes for Riparius/Vinifera/Aboreus where `GrowthPercentage > 0`; stock Cruentus has `GrowthPercentage=0`.

### 3.7 Runtime Add Helpers

`TiberiumClass::AddToSpreadQueue @ 0x00722AF0`:

- Computes the cell's linear index.
- Gets the `CellClass` for the coordinate.
- Requires `CellClass::CanSpreadTiberium`.
- Requires this tib type's spread bitmap for that cell to be `0`.
- Rebuilds if capacity pressure is near map cell count minus 20.
- Appends entry `{cell_coord, currentFrame + Random__Next() % 50}`.
- Inserts entry pointer into heap.
- Sets this tib type's spread bitmap to `1`.

`TiberiumClass::AddToGrowthQueue @ 0x007235A0`:

- Computes the cell linear index.
- Gets the `CellClass`.
- Requires `OverlayData < 11`; it does not check the growth bitmap before appending in the decompiled output.
- Rebuilds if growth entry count approaches map cell count minus 10.
- Appends entry `{cell_coord, currentFrame + Random__Next() % 50}`.
- Inserts entry pointer into heap.
- Sets this tib type's growth bitmap to `1`.

Active in YR: Yes. `PlaceTiberium`, `GrowTiberium`, and `Reduce_Tiberium` call these helpers on live ore paths.

### 3.8 Depletion-Time Reseed Contract

`CellClass::Reduce_Tiberium @ 0x00480A80` full-removal path calls `TiberiumClass::ClearSpreadBitmaps_AllTypes @ 0x00722AB0`, then tests the eight neighboring cells and calls this removed cell's tib type `AddToSpreadQueue` for eligible neighbors.

`ClearSpreadBitmaps_AllTypes` clears the removed cell's spread-bitmap entry in every `TiberiumClass`, not every bit in every bitmap.

Active in YR: Yes. This path is reached by standard harvesters and ore-damaging effects. This is why Rust needs a real spread bitmap: without it, the reseed has nowhere parity-equivalent to land.

## 4. INI Keys

| Key | Source | Stock YR value | Binary effect | Active in YR |
|---|---|---:|---|---|
| `[Tiberiums]` | `rulesmd.ini` | `0=Riparius`, `1=Cruentus`, `2=Vinifera`, `3=Aboreus` | Constructs per-type `TiberiumClass` array; index is stored at `+0x98`. | Yes. |
| `[Riparius] Growth` | `rulesmd.ini` | `2200` | Growth driver interval source. | Yes. |
| `[Riparius] GrowthPercentage` | `rulesmd.ini` | `.06` | Growth processor gate and batch fraction. | Yes. |
| `[Riparius] Spread` | `rulesmd.ini` | `2200` | Spread driver interval source. | Yes. |
| `[Riparius] SpreadPercentage` | `rulesmd.ini` | `.06` | Spread processor gate and batch fraction. | Yes. |
| `[Cruentus] GrowthPercentage` | `rulesmd.ini` | `0` | Growth processor exits for gems. | Conditional; class exists but no stock growth. |
| `[Cruentus] SpreadPercentage` | `rulesmd.ini` | `0` | Spread processor exits for gems. | Conditional; class exists but no stock spread. |
| `TiberiumGrows` | `[General]` / SpecialFlags | `yes` | Participates in scenario/global gates. | Yes by default. |
| `TiberiumSpreads` | `[General]` / SpecialFlags | `yes` | Participates in spread gate. | Yes by default. |
| `[Basic] TiberiumGrowthEnabled` | map basic | default true in Rust config; binary gate at `ScenarioClass+0x34A6` | Gates drivers/growth helpers. | Conditional per map; standard skirmish maps normally enabled. |

## 5. Integration Points

| Integration | Evidence | Active in YR |
|---|---|---|
| Scenario init seeds queues after overlay pack load. | `ScenarioClass::Full_Init @ 0x00686B20` calls queue init after overlay load/recalc. | Yes. |
| Tick loop runs growth before spread. | `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`. | Yes. |
| Full ore removal reseeds spread. | `Reduce_Tiberium @ 0x00480A80` plus `ClearSpreadBitmaps_AllTypes @ 0x00722AB0` and `AddToSpreadQueue @ 0x00722AF0`. | Yes. |
| New/grown ore can enter queues. | `PlaceTiberium @ 0x00487190`, `GrowTiberium @ 0x00483710`, `AddToGrowthQueue`, `AddToSpreadQueue`. | Yes. |
| CRC does not include queue runtime state. | `TiberiumClass::ComputeCRC @ 0x00721DC0` hashes exactly 7 rules fields: `+0x9C`, `+0xA8`, `+0xB8`, `+0xBC`, `+0xC0`, `+0xE4`, `+0xE8`. (corrected 2026-05-28: was "such as +0x9C, +0xA8, +0xB8, +0xE4, +0xE8" — incomplete enumeration omitted +0xBC and +0xC0; binary decompile `0x00721DC0` shows all 7 FUN_004a1d50 calls — ROOT_CAUSE: INFERENCE_HARDENED) | Yes, but CRC is not save/load. |

## 6. Current Rust Implementation Status

Current Rust stores:

- `ProductionState::resource_nodes` as the visible resource stock map.
- `OreGrowthConfig` with a single `grows/spreads/growth_rate_seconds` model.
- `OreGrowthState` as a scan cursor plus reservoir candidate vectors.
- `tick_ore_growth` from `Simulation::advance_tick`.

Mismatch:

- `src/sim/ore_growth.rs` explicitly documents and implements an RA1 map-scan/reservoir algorithm.
- No per-tiberium queue objects exist.
- No per-type spread/growth bitmap exists.
- No entry priorities exist.
- `world_hash.rs` hashes `resource_nodes`, terrain spawners, and default overlay id, but not `ore_growth_state` queue/timer/candidate state.
- `ProductionState` derives serde, so the current scanner state serializes, but it is not a gamemd queue model.

Relevant Rust surfaces:

- `src/sim/ore_growth.rs:1`
- `src/sim/ore_growth.rs:119`
- `src/sim/ore_growth.rs:156`
- `src/sim/production/production_types.rs:196`
- `src/sim/production/production_queue.rs:132`
- `src/sim/world/mod.rs:1545`
- `src/sim/world/world_hash.rs:140`

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TiberiumClass::InitSpreadQueues_All` | verified | decompile `0x00722240`; assembly context `0x00722240` | none for queue ownership |
| `TiberiumClass::RebuildSpreadQueue` | verified | decompile `0x007228B0`; assembly context `0x007228B0` | none for queue ownership |
| `TiberiumClass::SpreadDriver_AllTypes` | verified | decompile `0x007221B0`; tick caller `0x0055AFB0` | exact SpecialFlags writer not re-traced |
| `TiberiumClass::SpreadProcessor` | verified | decompile `0x00722440`; assembly context `0x00722440` | exact heap helper internals not separately named |
| `TiberiumClass::AddToSpreadQueue` | verified | decompile `0x00722AF0`; assembly context `0x00722AF0` | none for ownership |
| `TiberiumClass::ClearSpreadBitmaps_AllTypes` | verified | decompile `0x00722AB0` | none |
| `TiberiumClass::InitGrowthQueues_All` | verified | decompile `0x00722D00`; scenario init caller `0x00686B20` | none for queue ownership |
| `TiberiumClass::RebuildGrowthQueue` | verified | decompile `0x007233A0`; assembly context `0x007233A0` | none for queue ownership |
| `TiberiumClass::GrowthDriver_AllTypes` | verified | decompile `0x00722C40`; tick caller `0x0055AFB0` | exact multiplier source accepted from prior timing report |
| `TiberiumClass::GrowthProcessor` | verified | decompile `0x00722F00`; assembly context `0x00722F00` | none for ownership |
| `TiberiumClass::AddToGrowthQueue` | verified | decompile `0x007235A0`; assembly context `0x007235A0` | bitmap duplicate semantics oddity should be retested if duplicate queue entries appear |
| Native save/load stream | touched-not-exhausted | `search_functions Save/Load`; `TiberiumClass::ComputeCRC @ 0x00721DC0` | exact savegame reconstruction or rebuild behavior |
| Current Rust hash/serde | verified-source-scan | `world_hash.rs`, `production_types.rs`, `ore_growth.rs` | future implementation must add queue hash tests |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Are queues per map or per tiberium type? -> Per TiberiumClass; every queue field hangs off the class and rebuild filters by `+0x98`.` (evidence: `0x007228B0`, `0x007233A0`)
- `[RESOLVED] OQ-02 - Does map load seed queues from overlay cells? -> Yes, after overlay packs and all-cell RecalcAttributes in Full_Init.` (evidence: `0x00686B20`, `0x00722D00`, `0x00722240`)
- `[RESOLVED] OQ-03 - Does the live tick run queues in standard YR? -> Yes, growth then spread are called from the logic tick loop.` (evidence: `0x0055AFB0`)
- `[RESOLVED] OQ-04 - Are priorities wake-up times? -> No frame-gate was found after pop; priorities order heap entries only.` (evidence: `0x00722440`, `0x00722F00`)
- `[RESOLVED] OQ-05 - Does spread reinsert with jitter? -> Not in the processor; processor reinsert uses priority `0.0`, while runtime AddToSpreadQueue uses current frame plus `Random % 50`.` (evidence: `0x00722440`, `0x00722AF0`)
- `[RESOLVED] OQ-06 - Does growth reinsert with jitter? -> Yes, still-growable entries reinsert with `currentFrame + Random % 50`.` (evidence: `0x00722F00`)
- `[RESOLVED] OQ-07 - Are bitmaps membership state or dirty bits? -> Membership/dedup state; rebuild and add set to 1, no-neighbor/full-grown paths clear to 0.` (evidence: `0x007228B0`, `0x007233A0`, `0x00722440`, `0x00722F00`)
- `[RESOLVED] OQ-08 - Can depletion-time reseed be implemented without queues? -> Not parity-equivalently; it depends on clearing and testing spread bitmap membership and adding neighbors to this tib type's spread queue.` (evidence: `0x00480A80`, `0x00722AB0`, `0x00722AF0`)
- `[RESOLVED] OQ-09 - Do stock gems use the queue machinery? -> The class exists, but stock `GrowthPercentage=0` and `SpreadPercentage=0`, so processors exit.` (evidence: `rulesmd.ini [Cruentus]`, `0x00722440`, `0x00722F00`)
- `[RESOLVED] OQ-10 - Does Rust currently hash queue state? -> No real queue state exists; `world_hash.rs` hashes resources and terrain spawners but not `ore_growth_state`.` (evidence: `src/sim/world/world_hash.rs:140`)
- `[RESOLVED] OQ-11 - Does Rust serialize current ore growth state? -> Yes indirectly through `ProductionState` serde, but that state is the wrong scan/reservoir model.` (evidence: `src/sim/production/production_types.rs:196`, `src/sim/ore_growth.rs:119`)
- `[RESOLVED] OQ-12 - Is native TiberiumClass CRC a queue serialization proxy? -> No; it hashes rules fields, not runtime queue entries/bitmaps/timers.` (evidence: `0x00721DC0`)
- `[DEFERRED] OQ-13 - Exact native save/load stream behavior for queue fields.` (category: bounded-cost-too-high; reason: no named TiberiumClass Save/Load appeared in available symbol search; requires a dedicated savegame stream xref investigation; next-step-if-pursued: trace savegame object graph and post-load queue rebuild calls.)
- `[DEFERRED] OQ-14 - Exact source of the growth 0.3 multiplier.` (category: requires-different-system-context; reason: prior timing report records it, but this slot did not re-trace SpecialFlags/Rules writer; next-step-if-pursued: verify SpecialFlags bit/default writer chain.)
- `[DEFERRED] OQ-15 - Whether AddToGrowthQueue duplicate entries are possible in exotic paths.` (category: bounded-cost-too-high; reason: decompile shows no bitmap guard in `0x007235A0`, but proving all callers avoid duplicate adds requires caller-specific traces; next-step-if-pursued: xref all `AddToGrowthQueue` callers and inspect bitmap state before calls.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard YR owns growth and spread as per-tiberium queues with entry arrays, heaps, bitmaps, and per-type timers. | `0x00722240`, `0x007228B0`, `0x00722D00`, `0x007233A0` | Missing; current `OreGrowthState` is scan/reservoir. | `src/sim/ore_growth.rs`, `ProductionState` | Introduce queue-backed per-type ore growth state or an equivalent deterministic representation with identical membership/order effects. | `ore_growth_seeds_per_type_growth_and_spread_queues_from_overlay_data` | Do not extend the RA1 scan cursor and call it YR parity. |
| Full ore removal clears spread membership for the removed cell in all tib types, then reseeds eligible neighbors into the removed tib type's spread queue. | `0x00480A80`, `0x00722AB0`, `0x00722AF0` | Missing. | miner/combat `reduce_tiberium` paths plus `ore_growth` state | Full removal must update spread bitmaps and enqueue neighbors immediately. | `reduce_tiberium_full_removal_reseeds_neighbor_spread_queue_for_same_tiberium_type` | Do not just spawn ore or add a random future scan candidate. |
| Growth runs before spread in the same logic tick. | `0x0055AFB0` | Rust currently runs `tick_ore_growth`, then TIBTRE terrain spawners; inside old function it grows then spreads, but with wrong algorithm. | `Simulation::advance_tick`, `src/sim/ore_growth.rs` | Preserve growth-before-spread ordering for queue processors. | `ore_growth_processor_runs_before_spread_processor_same_tick` | Do not swap order while refactoring around terrain spawners. |
| Spread processor batch count is `clamp(ftol(heapCount * SpreadPercentage), 5, 25)`, then `Random % batch + 1`; processor reinsert priority is `0.0`. | `0x00722440` | Missing. | `src/sim/ore_growth.rs`, `SimRng` | Use processor batch/jitter semantics and consume RNG in matching places. | `spread_processor_uses_clamped_percentage_batch_and_zero_priority_reinsert` | Do not use one random adjacent spread per full scan cycle. |
| Growth processor batch count is `clamp(ftol(heapCount * GrowthPercentage), 5, 50)`, then `Random % batch + 1`; still-growable cells reinsert with `currentFrame + Random % 50` and feed spread queue. | `0x00722F00` | Missing. | `src/sim/ore_growth.rs`, `SimRng` | Implement growth queue processing with reinsert and spread-feed semantics. | `growth_processor_reinserts_still_growable_cell_and_feeds_spread_queue` | Do not treat `Growth=` as a whole-map scan interval. |
| Stock Cruentus/gems are represented by TiberiumClass but do not grow/spread because percentages are zero. | `rulesmd.ini [Cruentus]`, `0x00722440`, `0x00722F00` | Current Rust also blocks gem growth/spread but by resource type shortcut. | `ResourceType` / future tib type model | Preserve no stock gem growth while still allowing per-type data to exist. | `cruentus_queue_processors_exit_when_percentages_are_zero` | Do not hardcode "only ore can ever have queues" if YR data has other tib classes. |
| Queue state affects deterministic future behavior and must be part of Rust lockstep hash; native CRC does not cover it. | queue processors plus `TiberiumClass::ComputeCRC @ 0x00721DC0`; Rust `world_hash.rs` | Missing hash coverage for current/future ore growth state. | `src/sim/world/world_hash.rs` | Hash queue timers, ordered entries, bitmaps or canonical membership state, and per-type config needed for deterministic future output. | `ore_growth_queue_state_changes_world_hash` | Do not hash only visible `resource_nodes`; two sims with same ore but different queue order will diverge later. |
| Rust snapshots must either serialize the exact queue state or deliberately rebuild using a verified native save/load contract. | Rust `ProductionState` serde; native save/load unresolved | Current serde serializes wrong model. | `ProductionState`, snapshot tests | For now, serialize queue state once implemented; revisit if a native save/load investigation proves rebuild-on-load. | `ore_growth_queue_state_round_trips_through_snapshot` | Do not drop queue entries on restore unless a later report proves gamemd rebuild semantics. |

## Negative Facts / Do Not Do

- Do not implement depletion reseed as a direct ore placement.
- Do not model YR growth/spread as a full-map scan cursor.
- Do not use `GrowthRate` from `[General]` as the YR TiberiumClass queue interval; YR uses per-tiberium `Growth` and `Spread`.
- Do not treat heap priority as a strict "not before current frame" gate; no such gate was found in the processors.
- Do not hardcode stock `Riparius` only. YR has four TiberiumClass entries in stock rules; percentages decide activity.
- Do not assume native `ComputeCRC` proves runtime queue serialization.

## Remaining Uncertainty

- Exact native save/load behavior for queue arrays/bitmaps/timers remains open. No named TiberiumClass `Save`/`Load` appeared in the available `search_functions Save/Load` result, and `ComputeCRC` does not include runtime queue state. A dedicated save/load xref pass is needed before claiming gamemd either serializes these queues or rebuilds them after load.
- The exact writer/default chain for `ScenarioClass+0x34A6` was not re-traced in this slot.
- `AddToGrowthQueue` appears not to check the growth bitmap before appending; duplicate-entry behavior should be tested if future implementation encounters duplicate queues.

## Sources

- Ghidra decompile: `0x00686B20`, `0x0055AFB0`, `0x007221B0`, `0x00722440`, `0x00722AF0`, `0x00722AB0`, `0x00722240`, `0x007228B0`, `0x00722C40`, `0x00722F00`, `0x007235A0`, `0x007233A0`, `0x00722D00`, `0x00721DC0`, `0x00483620`, `0x00483690`, `0x00483710`, `0x00483780`, `0x00487190`.
- Ghidra assembly context: `0x00722440`, `0x00722F00`, `0x00722AF0`, `0x007235A0`, `0x007228B0`, `0x007233A0`.
- Prior docs: `TIBERIUM_QUEUE_SEEDING_AND_TIMING_REPORT.md`, `CELLCLASS_REDUCE_TIBERIUM_FUN_00480A80_GHIDRA_REPORT.md`, `CELLCLASS_PLACETIBERIUM_FUN_00487190_GHIDRA_REPORT.md`, `timing/logic-vs-render-loop.md`.
- INI: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust: `src/sim/ore_growth.rs`, `src/sim/production/production_types.rs`, `src/sim/production/production_queue.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_hash.rs`, `src/app_init.rs`.
