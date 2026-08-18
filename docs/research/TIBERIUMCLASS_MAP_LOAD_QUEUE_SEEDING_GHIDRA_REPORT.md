# TiberiumClass Map-Load Queue Seeding and Timer Initialization - Ghidra Research Report

**Address(es):** `0x00686B20`, `0x005FD2E0`, `0x0071D000`, `0x00722D00`, `0x00722240`, `0x007233A0`, `0x007228B0`, `0x00483620`, `0x00483690`, `0x007216C0`, `0x007221B0`, `0x00722C40`, `0x00689E90`, `0x0055AFB0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard-YR map-load/init behavior for TiberiumClass growth/spread queue seeding after overlay packs and terrain Unlimbo, including queue ownership, bitmap initialization, load-time priority values, relevant timer field initialization, and Rust-facing map-init deltas.  
**Non-Scope:** native save/load stream behavior, full queue processor internals after the first driver fire, exact heap helper implementation beyond observable insertion order, and TIBTRE placement/damage facts already covered by sibling reports.  
**Confidence:** High for map-load order, queue rebuild predicates, priority zero seeding, timer start/interval fields, and current Rust delta; Medium for exact heap object min/max metadata semantics.  
**Active in YR:** Yes. `ScenarioClass::Full_Init @ 0x00686B20` is the standard scenario/map initialization path and `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` calls the Tiberium drivers in standard play.

## Working Notes

Target question: What exactly does standard YR seed into TiberiumClass growth/spread queues at map load, and how are the relevant timer fields initialized?  
Non-goals: Do not re-investigate save/load, TIBTRE AI timing, new-cell PlaceTiberium insertion, or queue processor pop/reinsert behavior except where timer init makes the first driver fire relevant.  
Evidence needed to mark COMPLETE: Full_Init call order, OverlayPack writes, TerrainClass Unlimbo source clear, queue allocation/rebuild decompiles, CanGrow/CanSpread predicates, timer constructor writes, driver first-fire behavior, INI/default gates, and Rust map-init scan.  
Stop conditions: Stop after load-time seeding/timer initialization is implementation-ready; list processor/save-load details as out-of-scope rather than expanding.

## 1. Overview

Standard YR seeds TiberiumClass queues after map overlays are stamped, cell attributes are recalculated, and `[Terrain]` objects are read/unlimboed. This ordering matters for TIBTRE source cells: same-cell tiberium under a spawning terrain object is cleared by `TerrainClass::Unlimbo` before the growth/spread queue rebuild scans map cells.

Both queue families are per `TiberiumClass`, not global map lists. Load-time rebuild entries use priority `0.0`, set the corresponding one-byte-per-cell membership bitmap, and insert pointers into a 1-based heap. Growth and spread queues are rebuilt from separate predicates, so they are not guaranteed to contain the same cells.

## 2. Key Offsets

| Owner | Offset | Meaning in this slice | Active in YR | Evidence |
|---|---:|---|---|---|
| `ScenarioClass` | `+0x34A6` | `[Basic] TiberiumGrowthEnabled`; gates growth driver and `CanGrowTiberium`. | Conditional; read from map, commonly enabled. | `0x00689E90`, `0x00483620`, `0x007221B0`, `0x00722C40` |
| `ScenarioClass` flags | bit `0x80` | `SpecialFlags.TiberiumSpreads`; gates `CanSpreadTiberium`. | Conditional; default enabled in standard rules/session paths. | `0x00483690`; `SPECIAL_FLAGS_SYSTEM.md` |
| `CellClass` | `+0x44` | Overlay type index; `-1` means no overlay. | Yes. | `0x005FD2E0`, `0x0071D000` |
| `CellClass` | `+0x11C` | Slope index; nonzero blocks both queue rebuild predicates. | Yes. | `0x00483620`, `0x00483690` |
| `CellClass` | `+0x11E` | Overlay data/density byte. | Yes. | `0x005FD2E0`, `0x00483620`, `0x00483690` |
| `CellClass` | `+0xE4` | Object-list/content pointer checked by spread predicate. | Yes for spread seeding. | `0x00483690` |
| `TiberiumClass` | `+0x98` | Tiberium array index; rebuild filters cells to this type. | Yes. | `0x007233A0`, `0x007228B0` |
| `TiberiumClass` | `+0xF0/+0xF4/+0xF8/+0xFC` | spread entry count, heap pointer, bitmap pointer, entry array pointer. | Yes. | `0x00722240`, `0x007228B0` |
| `TiberiumClass` | `+0x100/+0x108` | spread timer start frame and interval. | Yes. | `0x007216C0`, `0x007221B0` |
| `TiberiumClass` | `+0x10C/+0x110/+0x114/+0x118` | growth entry count, heap pointer, bitmap pointer, entry array pointer. | Yes. | `0x00722D00`, `0x007233A0` |
| `TiberiumClass` | `+0x11C/+0x124` | growth timer start frame and interval. | Yes. | `0x007216C0`, `0x00722C40` |

## 3. Core Logic

### 3.1 Full_Init order

`ScenarioClass::Full_Init @ 0x00686B20` reaches the active map-load sequence:

1. `Read_Map_Section_And_IsoMapPacks`.
2. `ReadMapOverlayPacks @ 0x005FD2E0`.
3. Iterate all cells and call `CellClass::RecalcAttributes`.
4. `TerrainClass::Read_Map_Section`, whose terrain placement reaches `TerrainClass::Unlimbo`.
5. `TiberiumClass::InitGrowthQueues_All @ 0x00722D00`.
6. `TiberiumClass::InitSpreadQueues_All @ 0x00722240`.
7. `RadarClass::RebuildRadarSurfaces`.

Active in YR: Yes. Evidence is the `0x00686B20` decompile in the normal successful scenario load path.

The terrain step is load-bearing. `TerrainClass::Unlimbo @ 0x0071D000` calls `ObjectClass::Reveal`, increments the eight neighbor ore-neighbor bytes, and if the source cell has a tiberium overlay, writes `Cell+0x44 = -1` and `Cell+0x11E = 0`. Because queue init runs after this, a same-cell ore overlay under a map TIBTRE does not seed either queue.

Active in YR: Yes. Evidence: `0x0071D000`; sibling report `TIBTRE_SOURCE_OVERLAY_TYPE_REACHABILITY_AFTER_UNLIMBO_GHIDRA_REPORT.md`.

### 3.2 OverlayPack and OverlayDataPack state before queue rebuild

`ReadMapOverlayPacks @ 0x005FD2E0` first decodes `[OverlayPack]`, iterates a fixed `512 x 512` byte stream, skips `0xFF`, constructs an `OverlayClass` for valid non-empty overlay IDs, and thereby stamps `Cell+0x44`. It then decodes `[OverlayDataPack]` and unconditionally writes one byte to `Cell+0x11E` for in-bounds cells.

Active in YR: Yes. The function is directly called from Full_Init before queue rebuild.

### 3.3 Growth queue init/rebuild

`TiberiumClass::InitGrowthQueues_All @ 0x00722D00` iterates all `g_TiberiumClass_Array` entries. For each type it:

- frees existing growth heap `+0x110`, entry array `+0x118`, and bitmap `+0x114` if present;
- sets count `+0x10C = 0`;
- allocates an entry array sized `map_cell_count * 8`;
- allocates a bitmap sized `map_cell_count` bytes;
- allocates a `0x14`-byte heap object and a pointer array sized `map_cell_count * 4 + 4`;
- calls `TiberiumClass::RebuildGrowthQueue @ 0x007233A0`.

`RebuildGrowthQueue` clears count/heap/bitmap, iterates every map cell, maps the cell overlay to a tiberium type, requires that type to equal this class's `+0x98`, then requires `CellClass::CanGrowTiberium`. Each accepted cell writes `{coord, priority=0.0}` into the growth entry array, inserts that entry pointer into the heap, increments `+0x10C`, and writes bitmap byte `1`.

Active in YR: Yes. Called from Full_Init on standard map load.

### 3.4 Spread queue init/rebuild

`TiberiumClass::InitSpreadQueues_All @ 0x00722240` has the same allocation/free pattern for spread offsets `+0xF0/+0xF4/+0xF8/+0xFC`, then calls `TiberiumClass::RebuildSpreadQueue @ 0x007228B0`.

`RebuildSpreadQueue` clears count/heap/bitmap, iterates every map cell, maps the overlay to a tiberium type, requires equality with this class `+0x98`, then requires `CellClass::CanSpreadTiberium`. Each accepted cell writes `{coord, priority=0.0}`, inserts into the heap, increments `+0xF0`, and sets spread bitmap byte `1`.

Active in YR: Yes. Called from Full_Init on standard map load.

### 3.5 Queue membership predicates and density thresholds

`CanGrowTiberium @ 0x00483620` returns true only when:

- `ScenarioClass+0x34A6 != 0`;
- `OverlayToTiberiumIndex(cell) != -1`;
- `Cell+0x11C == 0` flat;
- `Cell+0x11E < TiberiumClass+0xE4 - 1` (stock max `12`, so data `0..10`);
- `GrowthPercentage` is not negative.

Active in YR: Yes. Called by load-time `RebuildGrowthQueue` and later growth paths.

`CanSpreadTiberium @ 0x00483690` returns true only when:

- `ScenarioClass` flag bit `0x80` is set;
- `OverlayToTiberiumIndex(cell) != -1`;
- `Cell+0x11E > (tiberium_index / 2)` using integer division before the class pointer reload;
- `Cell+0x11C == 0` flat;
- `SpreadPercentage` is not negative;
- `Cell+0xE4 == 0`.

Active in YR: Yes for normal spread queue seeding; conditional on `TiberiumSpreads` bit. This is not the forced TIBTRE `SpreadTiberium` path.

Therefore growth and spread queues do not seed from the same exact cell set. Stock Riparius type `0` flat cells with data `0` are growth-eligible but not spread-eligible. Data `11` cells are spread-eligible if other gates pass but not growth-eligible.

### 3.6 Timer initialization and first driver fire

`TiberiumClass::Constructor @ 0x007216C0` initializes the key timer fields before queue seeding:

- assembly `0x0072176A..0x00721776`: load `g_CurrentFrameCounter`, write `+0x100 = current frame`, write `+0x108 = 0`;
- assembly `0x00721794..0x007217A0`: load `g_CurrentFrameCounter`, write `+0x124 = 0`, write `+0x11C = current frame`.

`InitGrowthQueues_All` and `InitSpreadQueues_All` rebuild queue storage and membership, but do not reset these timer start/interval fields in the decompiled bodies.

`SpreadDriver_AllTypes @ 0x007221B0` reads `+0x100/+0x108`. If the elapsed frame count is not less than the interval, it calls the spread processor, then writes `+0x100 = g_CurrentFrameCounter` and reloads `+0x108 = TiberiumClass+0x9C` (`Spread=`).

`GrowthDriver_AllTypes @ 0x00722C40` reads `+0x11C/+0x124`. With interval `0`, the same comparison shape fires immediately, then writes `+0x11C = g_CurrentFrameCounter` and reloads `+0x124` from the growth interval calculation path.

Active in YR: Yes. `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` calls growth driver first, then spread driver, every logic tick.

## 4. INI Keys

| Key | Source | Stock YR value | Binary effect in this slice | Active in YR |
|---|---|---:|---|---|
| `[Basic] TiberiumGrowthEnabled` | map | commonly absent/default true | Read into `ScenarioClass+0x34A6`; gates growth driver and `CanGrowTiberium`. | Conditional per map; yes on normal stock maps unless disabled. |
| `[SpecialFlags] TiberiumSpreads` | map/session special flags | default enabled in standard paths | Scenario flag bit `0x80`; gates `CanSpreadTiberium` and non-forced spread. | Conditional; yes by default. |
| `[Tiberiums]` | `rulesmd.ini` | `0=Riparius`, `1=Cruentus`, `2=Vinifera`, `3=Aboreus` | Constructs per-type classes; `+0x98` is used during rebuild filtering. | Yes. |
| `[Riparius] Growth/Spread` | `rulesmd.ini` | `2200/2200` | Reloaded into timer intervals after the first driver fire. | Yes. |
| `[Riparius] GrowthPercentage/SpreadPercentage` | `rulesmd.ini` | `.06/.06` | Negative values block seeding predicates; processor percentages control later batch work. | Yes. |
| `[Cruentus] GrowthPercentage/SpreadPercentage` | `rulesmd.ini` | `0/0` | Zero is not negative, so load-time predicates can admit cells; processors later exit because percentage is not positive. | Conditional; stock gems exist but do not process growth/spread. |
| `[General] GrowthRate` | `rulesmd.ini` | `5` | Not used by the verified TiberiumClass queue seeding path. | No for this queue path; Rust currently uses it. |

## 5. Integration Points

| Integration | Verified behavior | Active in YR | Evidence |
|---|---|---|---|
| Overlay pack before terrain and queue rebuild | Overlay data exists before Terrain Unlimbo and queue seeding. | Yes | `0x00686B20`, `0x005FD2E0` |
| Terrain Unlimbo before queue rebuild | Source-cell tiberium can be cleared before the queue scan. | Yes | `0x00686B20`, `0x0071D000` |
| Queue init before radar rebuild and units/buildings | Growth/spread queues are seeded before normal objects/structures are loaded later in Full_Init. | Yes | `0x00686B20` |
| Logic tick order | Growth driver runs before spread driver. | Yes | `0x0055AFB0` |
| Driver first fire | Constructor intervals are zero, so the first eligible driver pass processes then reloads real intervals. | Yes | `0x007216C0`, `0x007221B0`, `0x00722C40` |

## 6. Current Rust Implementation Status

Rust currently seeds visible resources from overlay entries before the mutable overlay grid and before the app-init TIBTRE source-cell clear:

- `src/app_init.rs:759` seeds `production.resource_nodes` from map overlays.
- `src/app_init.rs:767` seeds live terrain/spawner indices.
- `src/app_init.rs:788` builds `OverlayGrid`.
- `src/app_init.rs:801` clears same-cell TIBTRE source overlays/resources as a reconciliation helper.
- `src/app_init.rs:849` builds `OreGrowthConfig` from `[General] GrowthRate`, `[General] TiberiumGrows`, `[General] TiberiumSpreads`, `[Basic] TiberiumGrowthEnabled`, and map `[SpecialFlags]`.
- `src/app_init.rs:857` creates `OreGrowthState::new(map_w, map_h)` without scanning visible overlay data into native per-type queues.

`src/sim/ore_growth.rs` still documents and implements a RA1-style incremental scan/reservoir model. It now has partial native-shaped event queues for TIBTRE placement and reduction reseed, but the load-time queue model is not equivalent to GameMD: no per-TiberiumClass growth/spread arrays, no per-cell growth bitmap, no load-time priority-zero heap entries, no native timer start/interval state, and wrong global rate ownership.

`src/sim/world/mod.rs:1644` still runs `tick_ore_growth` as an incremental scan before terrain spawners. `world_hash.rs` hashes the current Rust `ore_growth_state`, but that state is the wrong model for YR parity.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ScenarioClass::Full_Init` queue order | verified | `0x00686B20` | none for map-load order |
| `ReadMapOverlayPacks` overlay/data writes | verified | `0x005FD2E0` | exact decompressor internals out of scope |
| TIBTRE source clear before queue seed | verified | `0x0071D000`; sibling report | none for standard map load |
| Growth queue allocation/rebuild | verified | `0x00722D00`, `0x007233A0` | heap helper internals not named |
| Spread queue allocation/rebuild | verified | `0x00722240`, `0x007228B0` | heap helper internals not named |
| Growth predicate threshold | verified | `0x00483620` | none |
| Spread predicate threshold | verified | `0x00483690`; `SPECIAL_FLAGS_SYSTEM.md` | exact session writer of bit 0x80 not re-traced in this slot |
| Timer constructor fields | verified | `0x007216C0`; assembly `0x0072176A..0x007217A0` | scratch fields `+0x104/+0x120` not semantically named |
| Driver first-fire/reload behavior | verified | `0x007221B0`, `0x00722C40`, `0x0055AFB0` | exact growth interval multiplier remains sibling-scope |
| Current Rust map-init bridge | verified-source-scan | `src/app_init.rs`, `src/sim/ore_growth.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_hash.rs` | future implementation |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which mode? -> exhaustive-slice for map-load queue seeding/timer init only.` (evidence: user slot scope)
- `[RESOLVED] OQ-02 - Does Full_Init seed queues after OverlayPack? -> yes, `ReadMapOverlayPacks` precedes both queue init calls.` (evidence: `0x00686B20`)
- `[RESOLVED] OQ-03 - Does Full_Init seed queues after terrain Unlimbo clears TIBTRE source ore? -> yes, `TerrainClass__Read_Map_Section` precedes queue init, and `Unlimbo` clears tiberium overlay/data.` (evidence: `0x00686B20`, `0x0071D000`)
- `[RESOLVED] OQ-04 - Are queues per type? -> yes, init iterates `g_TiberiumClass_Array` and rebuild filters by `TiberiumClass+0x98`.` (evidence: `0x00722D00`, `0x00722240`, `0x007233A0`, `0x007228B0`)
- `[RESOLVED] OQ-05 - Are map-load queue priorities jittered? -> no, rebuild writes float bits `0` for priority; jitter belongs to runtime add helpers.` (evidence: `0x007233A0`, `0x007228B0`)
- `[RESOLVED] OQ-06 - Are growth and spread seeded from the same cells? -> no, growth uses `CanGrowTiberium`; spread uses `CanSpreadTiberium` with different gates and thresholds.` (evidence: `0x00483620`, `0x00483690`)
- `[RESOLVED] OQ-07 - What density can growth seed? -> flat matching tiberium cells with data `< MaxDensity-1`, stock `0..10`.` (evidence: `0x00483620`)
- `[RESOLVED] OQ-08 - What density can spread seed? -> flat matching tiberium cells with data `> tiberium_index/2`, no source object-list entry, and `TiberiumSpreads` bit set.` (evidence: `0x00483690`)
- `[RESOLVED] OQ-09 - Are bitmaps zeroed before rebuild? -> yes, rebuild loops clear the per-cell bitmap bytes then set accepted cells to `1`.` (evidence: `0x007233A0`, `0x007228B0`)
- `[RESOLVED] OQ-10 - Do init functions reset driver timers? -> no timer writes are present in the queue init decompiles; constructor owns initial `+0x100/+0x108/+0x11C/+0x124` values.` (evidence: `0x007216C0`, `0x00722D00`, `0x00722240`)
- `[RESOLVED] OQ-11 - How does first driver pass behave with interval zero? -> it processes immediately when the driver runs, then reloads the interval.` (evidence: `0x007221B0`, `0x00722C40`)
- `[RESOLVED] OQ-12 - Does current Rust seed native queues at map load? -> no; it creates a scanner state after resource/overlay/terrain helper setup.` (evidence: `src/app_init.rs:849-857`, `src/sim/ore_growth.rs:1-15`)
- `[RESOLVED] OQ-13 - Does Rust's map-load source clear happen before native-equivalent queue seed? -> Rust has no native queue seed; the clear runs before `OreGrowthState::new`, but after resource seeding.` (evidence: `src/app_init.rs:759-857`)
- `[RESOLVED] OQ-14 - Is `[General] GrowthRate` part of native queue seeding? -> no observed use in the queue init/rebuild functions.` (evidence: `0x00722D00`, `0x00722240`, `0x007233A0`, `0x007228B0`)
- `[DEFERRED] OQ-15 - Exact native save/load queue reconstruction.` (category: out-of-scope; reason: user explicitly scoped map-load/init/rebuild and excluded save/load except adjacent findings; next-step-if-pursued: savegame stream xref investigation.)
- `[DEFERRED] OQ-16 - Exact growth interval multiplier source used by `Math__ftol` in `0x00722C40`.` (category: requires-different-system-context; reason: driver reload math is adjacent to timer initialization but not needed to seed queues; next-step-if-pursued: focused growth-driver interval report.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Full_Init seeds queues after OverlayPack, RecalcAttributes, and Terrain Unlimbo source clears. | `0x00686B20`, `0x005FD2E0`, `0x0071D000` | missing native seed; Rust resource seed happens before source clear, then queue state is just `OreGrowthState::new`. | `src/app_init.rs`, future `src/sim/ore_growth.rs` queue seed API | Build native queue state from post-Unlimbo overlay/resource state, not raw OverlayPack entries. | Map has ore under `[Terrain] TIBTRE01`; after init, visible source ore is cleared and no growth/spread queue entry exists for that source cell. | `ore_queue_seed_after_tibtre_unlimbo_clear_excludes_source_cell`; risk: reintroducing source-cell ore/growth under trees. |
| Growth and spread queues are separate per-type rebuilds with priority-zero entries and independent bitmaps. | `0x00722D00`, `0x00722240`, `0x007233A0`, `0x007228B0` | missing; current scanner has candidate vectors plus partial event queues. | `src/sim/ore_growth.rs`, `ProductionState`, `world_hash.rs` | Store per-tiberium growth/spread queue entries, bitmap/membership state, and deterministic heap/order representation. | Two cells with same visible ore but different queue membership/order hash differently and process differently. | `ore_queue_seed_builds_per_type_priority_zero_growth_and_spread_entries`; risk: a BTreeSet-only model loses heap insertion/order parity. |
| Growth seed predicate differs from spread seed predicate. | `0x00483620`, `0x00483690` | mismatch; current Rust uses ore-only scan thresholds and `GrowthRate`. | `src/sim/ore_growth.rs`, map/resource type model | Seed growth for flat matching cells with data `< max-1`; seed spread only when spread bit/threshold/object-list gates pass. | Stock Riparius data `0` seeds growth but not spread; data `11` seeds spread but not growth when other gates pass. | `ore_queue_seed_growth_and_spread_use_distinct_density_thresholds`; risk: seeding both queues from one shared candidate predicate. |
| Constructor initializes timer start fields to current frame and intervals to zero; queue init does not reset them. | `0x007216C0`; assembly `0x0072176A..0x007217A0`; drivers `0x007221B0`, `0x00722C40` | missing native timer fields; current Rust scans by `[General] GrowthRate`. | future `OreGrowthState` timer model; `Simulation::binary_frame` integration | Preserve initial immediate first-fire behavior and later interval reloads per type. | First logic tick after map load with eligible queues processes before waiting `Spread=2200`/growth interval. | `ore_queue_initial_zero_interval_fires_before_reloading_type_interval`; risk: delaying first processing by one full interval. |
| `[General] GrowthRate` is not the native queue seed/timer interval owner in this path. | queue init/rebuild decompiles; `TiberiumClass::ReadINI @ 0x00721A50` reads per-type `Growth`/`Spread` | mismatch; Rust derives `growth_rate_seconds` from `[General] GrowthRate`. | `rules::tiberium`/`ruleset`, `src/sim/ore_growth.rs` | Use per-TiberiumClass `Growth`, `Spread`, percentages, and map/scenario gates for the queue model. | Changing `[Riparius] Growth` changes the post-first-fire interval; changing `[General] GrowthRate` alone does not drive this queue path. | `ore_queue_uses_tiberiumclass_growth_spread_not_general_growthrate`; risk: preserving RA1-style tuning under a YR label. |

## Negative Facts / Do Not Do

- Do not seed queues from raw `[OverlayPack]` before terrain Unlimbo; source-cell TIBTRE ore must be cleared first. Evidence: `0x00686B20`, `0x0071D000`.
- Do not seed growth and spread from the same generic “ore candidate” list. Evidence: distinct `0x00483620` and `0x00483690` predicates.
- Do not jitter map-load queue priorities; rebuild writes priority `0.0`. Evidence: `0x007233A0`, `0x007228B0`.
- Do not treat `[General] GrowthRate` as the owner of YR queue timing. Evidence: `0x00721A50`, `0x007221B0`, `0x00722C40`.
- Do not use `GrowthPercentage=0` / `SpreadPercentage=0` alone as a load-time queue seed blocker; the checked predicate rejects negative percentages, while later processors decide whether zero-percent types do work. Evidence: `0x00483620`, `0x00483690`; stock `[Cruentus]` percentages are `0`.

## Remaining Uncertainty

- Exact native save/load behavior remains out of scope for this report.
- Exact source of the `GrowthDriver` interval multiplier through `Math__ftol` was not re-expanded; it is adjacent to later timer reload, not map-load queue seeding.
- Heap helper metadata fields beyond count/capacity/pointer/min/max observations were not independently named; the queue-visible entry order/insertion behavior in rebuild is verified.

## Stale Docs / Follow-up Docs

- `docs/research/TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`: replace any wording implying standard map load seeds both queue families from the same cells with: "Map load rebuilds growth and spread queues separately. Growth uses `CellClass::CanGrowTiberium`; spread uses `CellClass::CanSpreadTiberium`, so priority-zero seed membership can differ by density, SpecialFlags bit `0x80`, and source object-list state."
- `docs/research/TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`: replace any wording implying queue init initializes timers with: "TiberiumClass constructor initializes the relevant timer start fields to `g_CurrentFrameCounter` and intervals to `0`; queue init/rebuild allocates and seeds queue storage but does not reset those timer fields."

## Sources

- Ghidra decompile: `0x00686B20`, `0x005FD2E0`, `0x0071D000`, `0x00722D00`, `0x00722240`, `0x007233A0`, `0x007228B0`, `0x00483620`, `0x00483690`, `0x007216C0`, `0x007221B0`, `0x00722C40`, `0x00689E90`, `0x0055AFB0`.
- Ghidra assembly context: `0x0072176A..0x007217A0` for constructor timer writes.
- Existing docs: `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`, `TIBTRE_SOURCE_OVERLAY_TYPE_REACHABILITY_AFTER_UNLIMBO_GHIDRA_REPORT.md`, `TIBTRE_SPREADTIBERIUM_FORCE_TYPE_AND_FLAG_GATE_GHIDRA_REPORT.md`, `SPECIAL_FLAGS_SYSTEM.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/app_init.rs`, `src/sim/ore_growth.rs`, `src/sim/production/production_types.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_hash.rs`.
