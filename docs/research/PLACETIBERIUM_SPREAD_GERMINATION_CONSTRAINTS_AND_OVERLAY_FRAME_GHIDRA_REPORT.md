# PlaceTiberium Spread Germination Constraints And Overlay Frame - Ghidra Research Report

**Address(es):** `0x00483780` (`CellClass::SpreadTiberium`), `0x00487190` (`CellClass::PlaceTiberium`), `0x004838E0` (`CellClass::CanPlaceTiberium`), `0x00722440` (`TiberiumClass::SpreadProcessor`), `0x00722AF0` (`TiberiumClass::AddToSpreadQueue`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Standard YR runtime tiberium spread/germination placement path: spread queue execution through candidate validation, neighbor choice, new-cell density/data, overlay type selection, dirty effects, gates/defaults, and ore/gem differences relevant to spread.
**Non-Scope:** Full growth queue timing, save/load serialization, `Reduce_Tiberium` full-removal reseed internals, TIBTRE terrain-spawner exact animation cadence, map-load overlay density derivation, and exact pixel draw composition of ore SHPs.
**Confidence:** High for the claimed spread placement slice; Medium only for exact runtime-initialized direction-table coordinate ordering, which is not needed to prove validation/density/frame behavior.
**Active in YR:** Yes for Riparius ore in standard YR skirmish. Conditional for other tiberium types based on per-type `SpreadPercentage`, `GrowthPercentage`, `Image`, and map/special flags. Cruentus gems are active data but do not spread in stock YR because `SpreadPercentage=0`.

## 1. Overview

Runtime ore spread in YR does not create a level-1 copy of the source overlay. The active spread path pops a cell from that tiberium type's spread queue, chooses a random neighbor start direction, validates candidates through `CellClass::CanPlaceTiberium`, and calls `CellClass::PlaceTiberium(tib_type, 3)` on the first valid neighbor. New cells are placed with `OverlayData=3`, a randomly selected overlay type from the tiberium type image range, a growth-queue insertion, tactical dirty rectangle work, and `RadarClass::MarkTerrainDirty`.

Current Rust still uses an RA1-style scan/reservoir model and `try_spread_ore` creates level-1 ore using the source overlay id. That is the key implementation mismatch for the player-visible scenario "ore regrows/spreads into a neighboring empty cell after harvesting empties a patch."

## 2. Class Layout / Key Offsets

| Owner | Offset | Type / role | Verified use | Active in YR |
|---|---:|---|---|---|
| `CellClass` | `+0x24` | packed map coord | Passed to queue helpers, overlay ctor, radar dirty, neighbor base | Yes |
| `CellClass` | `+0x38` | `IsoTileTypeClass` index | `CanPlaceTiberium` reads tile `AllowTiberium` at `IsoTile+0x306` | Yes |
| `CellClass` | `+0x44` | overlay type index | Must be `-1` for germination; written indirectly by `OverlayClass` ctor | Yes |
| `CellClass` | `+0xE4` | object list head | Rejects live visible buildings; rejects `SpawnsTiberium` terrain objects on target cell | Yes when `g_GameActive != 0` |
| `CellClass` | `+0xEC` | land type | `CanPlaceTiberium` indexes land-type Buildable table at `0x0089EA60` | Yes |
| `CellClass` | `+0x11C` | slope index | Source spread requires flat; target placement requires flat | Yes |
| `CellClass` | `+0x11E` | overlay data / density | Source must be above type-derived threshold; new germination writes exact density argument `3` | Yes |
| `CellClass` | `+0x140` | cell flags | `CanPlaceTiberium` rejects mask `0x500` bridge/rail structural cells | Yes |
| `TiberiumClass` | `+0x98` | array index | Used to match cell tiberium type during queue rebuild/processor | Yes |
| `TiberiumClass` | `+0x9C` | `Spread` interval | Spread driver writes timer interval from this field after firing | Yes |
| `TiberiumClass` | `+0xA0` | `SpreadPercentage` double | Spread candidate and processor gate; `<=0.0` exits processor | Yes |
| `TiberiumClass` | `+0xB0` | `GrowthPercentage` double | Existing-cell growth path gate, not new-cell spread germination gate | Yes |
| `TiberiumClass` | `+0xE0` | `Image` / base `OverlayTypeClass*` | New overlay variant selection starts from `Image->ArrayIndex` | Yes |
| `TiberiumClass` | `+0xE4` | max density | Entry gate rejects `density >= MaxDensity`; clamp target is `MaxDensity-1` | Yes |
| `TiberiumClass` | `+0xE8` | number of flat images | Sloped variant formula only; runtime spread target cannot be sloped | Conditional, but target validation blocks spread slope |
| `TiberiumClass` | `+0xF0` | spread queue entry count | Incremented on spread queue insertion | Yes |
| `TiberiumClass` | `+0xF4` | spread heap pointer | Popped/reinserted by `SpreadProcessor` | Yes |
| `TiberiumClass` | `+0xF8` | spread bitmap pointer | Dedup bit per map cell | Yes |
| `TiberiumClass` | `+0xFC` | spread entries array | `{coord, f32 priority}` entries | Yes |
| `TiberiumClass` | `+0x100/+0x108` | spread timer last/interval | Per-type driver cadence | Yes |
| `TiberiumClass` | `+0x10C/+0x110/+0x114/+0x118` | growth queue count/heap/bitmap/entries | New germinated cell is added to growth queue | Yes |
| `ScenarioClass` | `+0x34A6` | tiberium growth enabled | Gates spread driver and growth/grow helpers | Yes, default enabled in standard YR |
| `RadarClass` | `+0x1228/+0x1234/+0x14D9` | dirty terrain list/count/dirty flag | New ore cell is queued for radar redraw and dirty flag set | Yes |

## 3. Core Logic

### 3.1 Spread driver and processor call contract

`TiberiumClass::SpreadDriver_AllTypes` (`0x007221B0`) is called from `LogicClassPerTickUpdateLiveVector` (`0x0055B4DC`). It first checks `ScenarioClass+0x34A6`; if false, no tiberium type runs. For each tiberium type it reads `+0x100` last-fired frame and `+0x108` interval, and calls `TiberiumClass::SpreadProcessor` only when the interval has elapsed or when the last-fired field is `-1` and the interval path permits first fire. After firing it writes `last = g_CurrentFrameCounter` and `interval = TiberiumClass+0x9C` (`Spread`).

Active in YR: Yes. Standard `rulesmd.ini` has `[General] TiberiumGrows=yes`, `TiberiumSpreads=yes`, and `[Riparius] Spread=2200`, `SpreadPercentage=.06`. The driver also iterates `[Cruentus]`, but Cruentus spread execution exits because `SpreadPercentage=0`.

`TiberiumClass::SpreadProcessor` (`0x00722440`) exits if the spread heap is null/empty or `SpreadPercentage <= 0.0`. Otherwise it computes a batch base as `ftol(heap_count * SpreadPercentage)`, clamps it to `[5,25]`, rolls `Random::Next() % batch_base + 1`, then pops up to that many entries. The popped heap priority is used for order only; this function does not compare entry priority against the current frame before processing.

Active in YR: Yes for Riparius. Conditional for any tiberium type with `SpreadPercentage > 0.0`.

### 3.2 Candidate precheck before actual spread

For each popped spread entry, `SpreadProcessor` counts valid neighboring cells before attempting actual spread. It iterates eight neighbor offsets, calls `MapClass::Get_CellClass` through the neighbor helper path, then calls `CellClass::CanPlaceTiberium` on each candidate. If no valid neighbor exists, it clears this cell's spread bitmap entry and does not reinsert it. If at least one valid neighbor exists, it calls `CellClass::SpreadTiberium(0)`. If more than one valid neighbor exists, it reinserts the source cell with priority `0.0` and marks the spread bitmap bit again.

Active in YR: Yes. The `>1 valid neighbor` reinsertion is important: a spread source with exactly one valid target is not reinserted by this path after the attempt; a source with multiple possible targets stays in the spread queue.

### 3.3 Neighbor choice and spread call

`CellClass::SpreadTiberium` (`0x00483780`) does the actual target selection. With `param_2 == 0`, it first requires:

- scenario/special flag byte has bit `0x80` set,
- current cell maps to a valid tiberium index,
- `OverlayData > tib_index / 2`,
- source slope index `+0x11C == 0`,
- this tiberium type's `SpreadPercentage >= 0.0`,
- source object-list pointer `+0xE4 == 0`.

Then it chooses a random start direction with `Random::RandomRanged(0,7)`. It checks directions `(start + i) & 7` for `i=0..7`, obtains the neighbor cell through `g_DirectionOffsets`, validates the target with `CellClass::CanPlaceTiberium`, and breaks on the first valid candidate. It then calls `CellClass::PlaceTiberium(tib_type, 3)`.

Active in YR: Yes for the standard spread queue. The source precheck partly duplicates `CanSpreadTiberium` and prevents sloped/occupied source cells from being live spread sources.

### 3.4 Target validation: `CellClass::CanPlaceTiberium`

`CellClass::CanPlaceTiberium` (`0x004838E0`) is the target-cell validation gate for new spread germination. All checks must pass:

1. `MapClass::Is_Cell_In_Playfield(&cell->MapCoord, 1)` returns true.
2. `(cell->Flags & 0x500) == 0`; this rejects bridge/rail structural cells.
3. If `g_GameActive != 0`, scan the cell object list for RTTI `6` (`BuildingClass`). A live building with health `>0` rejects placement unless the building type has either byte `+0xC9A` or byte `+0x1701` set.
4. If `g_GameActive != 0`, scan the cell object list for RTTI `0x24` (`TerrainClass`). A terrain object whose type byte `+0x2B1` is nonzero rejects placement. This is the `SpawnsTiberium` terrain-tree exclusion.
5. The land-type table byte at `0x0089EA60 + cell->LandType * 0x24` must be nonzero. This is the land-type Buildable gate.
6. `cell->OverlayTypeIndex == -1`; any existing overlay blocks new germination.
7. `cell->SlopeIndex == 0`; target cell must be flat.
8. If `IsoTileTypeIndex` is in range, `IsoTileTypeClass+0x306` (`AllowTiberium`) must be nonzero. Invalid/out-of-range tile indices pass this final tile flag fallback.

Active in YR: Yes. These checks are on the standard spread path and are not TS-only. The object-list checks are live only while `g_GameActive != 0`, which is true during normal gameplay.

### 3.5 New overlay type and frame/data

`CellClass::PlaceTiberium` (`0x00487190`) first loads `g_TiberiumClass_Array[tib_type]` and rejects immediately if `density >= TiberiumClass+0xE4` (`MaxDensity`; stock value 12). Runtime spread passes density `3`, so this gate does not reject.

Because spread called `CanPlaceTiberium` first and selected an empty valid target, the germinate branch is taken. For a flat cell (`SlopeIndex == 0`), the function:

- allocates a new `OverlayClass` of size `0xB0`,
- copies `CellClass+0x24` map coord into a local,
- rolls `Random::RandomRanged(0, 0xB)`, inclusive `0..11`,
- selects overlay type pointer from `g_OverlayTypeClass_Array[Image->ArrayIndex + random_0_11]`,
- calls `OverlayClass::Constructor(selected_overlay_type, &coord, -1)`,
- calls `TiberiumClass::AddToGrowthQueue(&cell->MapCoord)`,
- writes `cell->OverlayData = (char)density`, so spread writes exact value `3`,
- dirties tactical screen rectangle,
- calls `RadarClass::MarkTerrainDirty(&cell->MapCoord)`,
- returns `1`.

Active in YR: Yes. For standard Riparius, `[Riparius] Image=1` and the overlay type range covers the flat ore overlays associated with the tiberium type. Spread does not copy the source overlay id; it chooses a random flat variant from the type's image range.

The sloped germination branch exists, rolling `Random::RandomRanged(0,1)` and selecting `Image->ArrayIndex + NumImages + SlopeIndex * 2 + random - 2`, but runtime spread targets cannot reach it because `CanPlaceTiberium` requires `SlopeIndex == 0`. It can be relevant only to other callers that call `PlaceTiberium` without the same target validation.

Active in YR: Conditional. The code exists and is callable by other `PlaceTiberium` callers, but the standard spread target path does not use it.

### 3.6 Dirty and side effects

New-cell germination through `PlaceTiberium` does not call `CellClass::RecalcAttributes` directly. The overlay constructor stamps the overlay into the cell, then `PlaceTiberium` computes tactical dirty rectangles and calls `RadarClass::MarkTerrainDirty`. `RadarClass::MarkTerrainDirty` (`0x006551C0`) deduplicates the coord in the dirty list, appends if absent/capacity allows, and sets `RadarClass+0x14D9 = 1`.

Active in YR: Yes. New ore cells should be visible in the tactical view and minimap after placement. Any Rust path that only updates `resource_nodes` without an overlay mutation and dirty/recalc path will be player-visible wrong.

`RecalcAttributes` (`0x0047D2B0`) is not directly invoked by `PlaceTiberium`. The current Rust `OverlayGrid::place_overlay` marks cells dirty, and the app layer later drains dirty cells and calls `recalc_overlay_passability`; that is a reasonable Rust architecture boundary, but the placement path still needs to write the right overlay id/data and ensure the dirty cell is emitted.

Active in YR: Yes for the binary side effects; Rust scheduling equivalence remains an implementation concern.

### 3.7 Ore vs gem spread

Stock YR data:

| Type | INI section | Image | Value | Growth | GrowthPercentage | Spread | SpreadPercentage | Runtime spread active? |
|---|---|---:|---:|---:|---:|---:|---:|---|
| Riparius | `[Riparius]` | `1` | `25` | `2200` | `.06` | `2200` | `.06` | Yes |
| Cruentus | `[Cruentus]` | `2` | `50` | `10000` | `0` | `10000` | `0` | No, processor exits |

Active in YR: Yes. Gems are harvestable tiberium overlays, but stock Cruentus does not grow or spread under the standard queue processor because both percentages are zero. Rust's current `ResourceType::Gem` "does not grow or spread" matches stock data for gems, but the implementation should be per-tiberium-type data-driven rather than a permanent hardcoded gem rule if later support includes nonstandard rules or the unused Vinifera/Aboreus entries.

## 4. INI Keys

| Key | Location | Stock YR value | Binary effect | Rust status |
|---|---|---|---|---|
| `GrowthRate` | `[General]` | `5` minutes | Legacy/general interval, not the active spread queue's per-type `Spread` value | Rust uses it as RA1-style full scan cycle (`ore_growth.rs:83`) |
| `TiberiumGrows` | `[General]` | `yes` | Part of standard YR defaults; active growth/spread also gated through scenario/special flags | Rust parses and uses it (`ruleset.rs:897`, `ore_growth.rs:79`) |
| `TiberiumSpreads` | `[General]` | `yes` | Enables spread behavior at rules/options level; binary spread driver also gates on scenario byte | Rust parses and uses it (`ruleset.rs:898`, `ore_growth.rs:82`) |
| `TiberiumGrowthEnabled` | map `[Basic]` | default true when absent | Binary `ScenarioClass+0x34A6` gates spread and growth drivers | Rust parses and combines it into `grows`, but not into `spreads` (`ore_growth.rs:79-82`) |
| `TiberiumGrows` | map `[SpecialFlags]` | default true when absent | Special flag contributes to active scenario growth/spread gates | Rust parses for `grows` (`basic.rs:87`, `ore_growth.rs:81`) |
| `TiberiumSpreads` | map `[SpecialFlags]` | default true when absent | Special flag contributes to active spread gate | Rust parses for `spreads` (`basic.rs:88`, `ore_growth.rs:82`) |
| `Image` | `[Riparius]`, `[Cruentus]` | Riparius `1`, Cruentus `2` | Determines the base overlay type range for random variant selection | Rust spread currently copies source overlay id instead |
| `Growth` | per tiberium type | Riparius `2200`, Cruentus `10000` | Growth driver interval | Rust does not model per-type growth queue intervals |
| `GrowthPercentage` | per tiberium type | Riparius `.06`, Cruentus `0` | Growth processor batch/gates; existing-cell `PlaceTiberium` grow branch gate | Rust hardcodes ore grows, gem does not |
| `Spread` | per tiberium type | Riparius `2200`, Cruentus `10000` | Spread driver interval at `TiberiumClass+0x9C` | Rust does not model per-type spread queue intervals |
| `SpreadPercentage` | per tiberium type | Riparius `.06`, Cruentus `0` | Spread processor batch/gates and source spread check | Rust uses fixed threshold/reservoir model |
| `Value` | per tiberium type | Riparius `25`, Cruentus `50` | Credit value per bail/density consumed elsewhere | Rust resource stock partly mirrors value through `ResourceType` base values, but this report does not prove the harvest value path |
| `AllowTiberium` | theater `[TileSetNNNN]` | true on selected grass/dirt/etc tiles | `IsoTileTypeClass+0x306` final target gate | Rust theater parser currently parses `Morphable`, not `AllowTiberium` (`theater.rs:176`, `theater.rs:321`) |
| `SpawnsTiberium` | terrain object types | true on TIBTRE* | Blocks germination on the spawner's own cell | Rust terrain spawner state exists; ore spread `can_germinate` does not check it |

## 5. Integration Points

| Point | Evidence | Contract |
|---|---|---|
| Per-tick activation | `0x0055B4DC -> 0x007221B0` xref | Spread queues run from the live logic tick, not from a map scan cycle. |
| Spread queue initialization | `0x005993A0`, `0x00687A8A`, `0x0067E6B3 -> 0x00722240`; `0x00722240 -> 0x007228B0` | Spread queues are allocated/rebuilt during scenario init / full init contexts. |
| Runtime candidate enqueue | `0x00480C5C`, `0x0048760A`, `0x00723113 -> 0x00722AF0` | Full ore removal, existing-cell growth, and growth processor feed spread queue candidates. |
| Target validation | `0x00722553`, `0x004838A1`, `0x004871C3 -> 0x004838E0` | `CanPlaceTiberium` is the live target-validation function for spread and germination. |
| New overlay placement | `0x004838C5 -> 0x00487190` | Spread target calls `PlaceTiberium(tib_type, 3)`. |
| Growth-queue add after new placement | `0x00487291 -> 0x007235A0` | Newly germinated ore enters that type's growth queue immediately after overlay construction. |
| Radar dirty | `0x00487685 -> 0x006551C0` | New ore cell is added to radar terrain dirty list and dirty flag set. |
| Rust tick integration | `src/sim/world/mod.rs:1546`, `src/sim/world/mod.rs:1556` | Current Rust runs `ore_growth` then TIBTRE spawning in phase 7. |
| Rust dirty terrain integration | `src/app_sim_tick.rs:683`, `src/app_sim_tick.rs:688` | Current app drains overlay dirty cells and recalculates passability/terrain metadata after sim tick. |

## 6. Current Rust Implementation Status

Current Rust spread is RA1-style, not YR queue-style:

- `src/sim/ore_growth.rs:156` ticks an incremental scanner over `resource_nodes`.
- `src/sim/ore_growth.rs:31-37` uses `ORE_BASE_PER_LEVEL=120`, `MAX_ORE_LEVELS=12`, and `SPREAD_THRESHOLD=6 levels`.
- `src/sim/ore_growth.rs:206` treats ore above that threshold as spreadable.
- `src/sim/ore_growth.rs:296` `try_spread_ore` uses a random start direction and the first valid adjacent cell.
- `src/sim/ore_growth.rs:326` inserts new spread ore with `remaining = ORE_BASE_PER_LEVEL`, i.e. level 1.
- `src/sim/ore_growth.rs:332` copies the source overlay id and writes overlay data `0`.
- `src/sim/ore_growth.rs:346` `can_germinate` only rejects existing resource nodes and non-walkable path-grid cells.

Rust already has useful integration surfaces:

- `src/sim/overlay_grid.rs:102` `place_overlay` writes `overlay_id` / `overlay_data` and pushes a dirty cell.
- `src/sim/overlay_grid.rs:184` `recalc_overlay_passability` updates tiberium land type / terrain metadata after dirty drain.
- `src/sim/production/production_queue.rs:132` seeds `resource_nodes` from map overlays and distinguishes ore vs gems.
- `src/sim/terrain_spawn.rs:193` has a density-3 additive placement helper for terrain spawners, but it is TIBTRE-oriented and not the YR spread queue contract.

Missing or mismatching for this slice:

- No per-tiberium-type spread heap/bitmap/interval model.
- No `PlaceTiberium(type, 3)` primitive for standard spread.
- New spread cells are level 1/frame 0 in Rust, but stock YR spread writes `OverlayData=3`.
- New spread cells copy the source overlay id, but stock YR randomly chooses one of the tiberium type's 12 flat image variants.
- Target validation lacks bridge mask, live visible building exception semantics, spawner terrain-object exclusion, land-type Buildable, slope==0, and theater `AllowTiberium`.
- Rust parses theater `Morphable`, but not theater `AllowTiberium`.
- Rust uses `[General] GrowthRate` full scan cadence rather than per-type `Spread` queue cadence and `SpreadPercentage` batch processing.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CellClass::PlaceTiberium` germinate branch | verified | Ghidra `0x00487190` | none for spread target |
| `CellClass::PlaceTiberium` grow-existing branch | touched-not-exhausted | Ghidra `0x00487190`, prior report | Out-of-scope except it calls spread queue after growth |
| `CellClass::CanPlaceTiberium` | verified | Ghidra `0x004838E0` | none for target validation |
| `CellClass::SpreadTiberium` | verified | Ghidra `0x00483780` | exact runtime direction-table coordinate order deferred; algorithm order verified |
| `CellClass::CanSpreadTiberium` | verified | Ghidra `0x00483690` | none for source eligibility |
| `CellClass::CanGrowTiberium` / `GrowTiberium` | touched-not-exhausted | Ghidra `0x00483620`, `0x00483720` | Full growth behavior belongs to slot 2/queue timing |
| `TiberiumClass::SpreadDriver_AllTypes` | verified | Ghidra `0x007221B0`, xref from `0x0055B4DC` | none for call contract |
| `TiberiumClass::SpreadProcessor` | verified | Ghidra `0x00722440` | none for placement call contract |
| `TiberiumClass::AddToSpreadQueue` | verified | Ghidra `0x00722AF0` | none for enqueue gate/priority |
| `TiberiumClass::InitSpreadQueues_All` / `RebuildSpreadQueue` | touched-not-exhausted | Ghidra `0x00722240`, `0x007228B0` | Serialization/save-load state belongs to slot 2 |
| `TiberiumClass::AddToGrowthQueue` | verified for germination handoff | Ghidra `0x007235A0`, call from `0x00487291` | none for new-cell growth queue add |
| `RadarClass::MarkTerrainDirty` | verified | Ghidra `0x006551C0` | none |
| `CellClass::RecalcAttributes` direct call absence from `PlaceTiberium` | verified | Ghidra `0x00487190`, `0x0047D2B0` spot-check | none |
| Per-type INI values Riparius/Cruentus | verified | `ini/rulesmd.ini` `[Riparius]`, `[Cruentus]` | none for stock YR |
| Theater `AllowTiberium` parser in Rust | verified missing from targeted scan | `src/map/theater.rs:176`, `src/map/theater.rs:321`; INI `AllowTiberium=true` entries | implementer must add parser if fixing validation |
| Current Rust spread placement | verified | `src/sim/ore_growth.rs:296`, `:326`, `:332`, `:346` | none |
| Exact `g_DirectionOffsets` dx/dy pair order | deferred | `0x00483780` uses table and random start | Not needed for density/frame/validation proof; a direction-table report can resolve exact orientation labels |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - What is the investigation mode and exact scope? -> exhaustive-slice for runtime spread/germination placement through candidate validation, density/frame, overlay selection, dirty effects, and gates` (evidence: user target and report header)
- `[RESOLVED] OQ2 - Is `CellClass::PlaceTiberium` active in standard YR spread? -> yes, `CellClass::SpreadTiberium` calls it with density 3` (evidence: `0x00483780`, call at `0x004838C5`; xref to `0x00487190`)
- `[RESOLVED] OQ3 - What density does runtime spread place? -> exact density argument `3`, written to `CellClass+0x11E` in germinate branch` (evidence: `0x00483780`, `0x00487190`)
- `[RESOLVED] OQ4 - Does runtime spread place frame/data 0 or 3? -> data 3; visual overlay type is independently randomized from 12 flat variants` (evidence: `0x00487190`)
- `[RESOLVED] OQ5 - Does spread copy the source overlay id? -> no; it selects `Image->ArrayIndex + RandomRanged(0,11)` for flat targets` (evidence: `0x00487190`)
- `[RESOLVED] OQ6 - Is sloped new-cell placement live for standard spread? -> no for standard spread targets because `CanPlaceTiberium` requires `SlopeIndex==0`; conditional for other callers` (evidence: `0x004838E0`, `0x00487190`)
- `[RESOLVED] OQ7 - What validates candidate cells? -> `CanPlaceTiberium` eight-gate chain: playfield, bridge mask, visible building, spawner terrain object, buildable land type, no overlay, flat, `AllowTiberium`` (evidence: `0x004838E0`)
- `[RESOLVED] OQ8 - Is walkability alone enough to match target validation? -> no; binary validation includes non-path-grid checks such as overlay absence, slope, tile `AllowTiberium`, and spawner terrain objects` (evidence: `0x004838E0`; Rust `src/sim/ore_growth.rs:346`)
- `[RESOLVED] OQ9 - What map/rules flags gate spread execution? -> scenario byte `+0x34A6`, special bit path in `SpreadTiberium`, per-type `SpreadPercentage > 0`, and per-type spread interval` (evidence: `0x007221B0`, `0x00483780`, `0x00722440`; `ini/rulesmd.ini`)
- `[RESOLVED] OQ10 - Are gems spread-active in stock YR? -> no, Cruentus has `SpreadPercentage=0`, so processor exits` (evidence: `0x00722440`; `ini/rulesmd.ini [Cruentus]`)
- `[RESOLVED] OQ11 - Does new-cell placement dirty radar/tactical surfaces? -> yes; `PlaceTiberium` dirties tactical rect and calls `RadarClass::MarkTerrainDirty`, which appends/dedups dirty coord and sets `+0x14D9=1`` (evidence: `0x00487190`, `0x006551C0`)
- `[RESOLVED] OQ12 - Does `PlaceTiberium` directly call `RecalcAttributes`? -> no; direct `RecalcAttributes` is absent from `0x00487190`` (evidence: `0x00487190`; spot-check `0x0047D2B0`)
- `[RESOLVED] OQ13 - Does a newly spread cell enter the growth queue? -> yes, after overlay construction and before writing `OverlayData=3`` (evidence: `0x00487291`, `0x007235A0`)
- `[RESOLVED] OQ14 - Does spread queue priority frame-gate processing? -> no observed current-frame comparison in `SpreadProcessor`; priority orders heap pops` (evidence: `0x00722440`)
- `[RESOLVED] OQ15 - What happens when a popped source has no valid neighbor? -> spread bitmap entry for that source cell is cleared and the source is not reinserted` (evidence: `0x00722440`)
- `[RESOLVED] OQ16 - What happens when a popped source has more than one valid neighbor? -> source is reinserted with priority `0.0` and spread bitmap set` (evidence: `0x00722440`)
- `[RESOLVED] OQ17 - Which Rust function creates current spread cells? -> `try_spread_ore`, inserting level-1 ore and source overlay id/data 0` (evidence: `src/sim/ore_growth.rs:296`, `:326`, `:332`)
- `[RESOLVED] OQ18 - Does Rust parse theater `AllowTiberium` today? -> not in the targeted theater lookup; it parses `Morphable` but no `AllowTiberium` field was found` (evidence: `src/map/theater.rs:176`, `:321`; `rg AllowTiberium src`)
- `[RESOLVED] OQ19 - Does current Rust have a dirty overlay path suitable for germination? -> yes, `OverlayGrid::place_overlay` records dirty cells and app layer recalculates overlay passability` (evidence: `src/sim/overlay_grid.rs:102`, `src/app_sim_tick.rs:683`, `:688`)
- `[RESOLVED] OQ20 - What is the null/empty queue edge? -> `SpreadProcessor` returns if heap pointer null, heap count zero, or `SpreadPercentage <= 0.0`` (evidence: `0x00722440`)
- `[RESOLVED] OQ21 - What is the max-density density argument edge? -> `PlaceTiberium` rejects if passed density is `>= MaxDensity`; spread passes 3, so active stock spread is unaffected` (evidence: `0x00487190`)
- `[RESOLVED] OQ22 - What is the first-tick/driver edge? -> driver checks last-fired `-1` and interval state before processing; exact first-fire cadence is queue timing non-scope, but call contract is verified` (evidence: `0x007221B0`)
- `[RESOLVED] OQ23 - Does target object-list scanning run during normal play? -> yes when `g_GameActive != 0`; standard gameplay has it true` (evidence: `0x004838E0`)
- `[DEFERRED] OQ24 - What are the exact cardinal labels for `g_DirectionOffsets[0..7]`?` (category: bounded-cost-too-high; reason: table is runtime-initialized and this slice only needs random-start wrapped index order plus validation/density; next-step-if-pursued: trace writes to `0x0089F688` initializer and compare to coordinate conventions)
- `[DEFERRED] OQ25 - How is spread/growth queue state serialized in saves?` (category: out-of-scope; reason: assigned to queue-state/serialization slot, not placement constraints; next-step-if-pursued: inspect save/load xrefs for `TiberiumClass+0xF0..0x118`)

Adversarial corner-case answers:

- If the target has any overlay, spread does not place there because `OverlayTypeIndex` must be `-1`.
- If the target is a valid walkable road-like land type but theater `AllowTiberium=false`, spread does not place there.
- If the source has exactly one valid neighbor, one spread attempt can happen but the source is not reinserted by the `>1` valid-neighbor branch.
- If the tiberium type is Cruentus stock gems, the spread processor exits before candidate processing because `SpreadPercentage=0`.
- If allocation of the new `OverlayClass` returns null, the decompilation falls through to add to growth queue and write overlay data; this is an out-of-memory edge, not a normal gameplay branch. Future Rust should not emulate broken allocation behavior unless an OOM policy requires it.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Runtime spread places new cells via `PlaceTiberium(tib_type, 3)`, not level 1 | `0x00483780`, `0x00487190` | mismatch: Rust inserts `remaining=ORE_BASE_PER_LEVEL` and overlay data `0` | `src/sim/ore_growth.rs:296`, `src/sim/miner` resource density model | New spread germination should produce density/data 3 and equivalent stock/credit amount for Riparius | `yr_spread_germination_places_density_three_ore_cell` | Do not use frame/data 0 for standard runtime spread |
| New spread overlay type is randomly selected from the tiberium type flat image range | `0x00487190` | mismatch: Rust copies source overlay id | Need tiberium type metadata / overlay image range; `OverlayGrid::place_overlay` | Select one of 12 flat Riparius variants for stock ore using binary-equivalent RNG range `0..11` | `yr_spread_germination_randomizes_flat_riparius_overlay_variant` | Do not copy source overlay id; that freezes visual variety incorrectly |
| Target validation is `CanPlaceTiberium`, not generic walkability | `0x004838E0` | mismatch: `can_germinate` only checks existing resource node and `PathGrid::is_walkable` | `src/sim/ore_growth.rs:346`; terrain/occupancy/theater data | Add playfield, bridge, live building, spawner terrain, land Buildable, no overlay, flat, `AllowTiberium` gates or an equivalent data-backed helper | `yr_spread_rejects_clear_walkable_tile_when_allow_tiberium_false`; `yr_spread_rejects_tibtree_occupied_cell` | Do not collapse validation to passability; passability misses several YR gates |
| New germinated cell is added to growth queue | `0x00487291`, `0x007235A0` | missing: current Rust has no per-type growth queue/bitmap | `src/sim/ore_growth.rs`, `ProductionState::ore_growth_state` | Queue newly placed cell for future growth under the same tiberium type | `yr_spread_germination_enqueues_new_cell_for_growth` | Do not rely on map scan eventually finding it if implementing YR queues |
| Spread source processing uses a per-type heap/bitmap and reinserts only if more than one valid neighbor remains | `0x00722440` | mismatch: Rust reservoir-samples resource nodes and each sampled source tries once per scan | `src/sim/ore_growth.rs`, production state serialization/hash | Model per-type spread queue bitmap/heap semantics or explicitly stage a design that reproduces them deterministically | `yr_spread_source_with_one_valid_neighbor_not_reinserted_after_spread`; `yr_spread_source_with_two_valid_neighbors_reinserted` | Do not approximate with full-map periodic scans if queue timing/order parity is targeted |
| Stock gems do not spread because `Cruentus SpreadPercentage=0` | `0x00722440`; `ini/rulesmd.ini [Cruentus]` | current behavior matches stock by hardcoded `ResourceType::Gem` no spread | rules/tiberium type parser if generalized | Keep stock gems non-spreading, but prefer data-driven per-type percentages for parity and mods | `yr_stock_cruentus_spread_percentage_zero_skips_spread_processor` | Do not hardcode "all gems never spread" as a binary behavior beyond stock data |
| New germination dirties tactical and radar terrain | `0x00487190`, `0x006551C0` | partial: `OverlayGrid::place_overlay` dirty path exists, radar dirty is render/app-side implicit | `src/sim/overlay_grid.rs:102`, `src/app_sim_tick.rs:683` | Ensure spread placement mutates `OverlayGrid` and causes terrain/passability/radar update in the same tick boundary | `yr_spread_germination_marks_overlay_dirty_for_passability_and_radar` | Do not add only a `resource_nodes` entry; invisible/non-radar ore is wrong |
| `PlaceTiberium` does not directly call `RecalcAttributes`; Rust may recalc through dirty overlay drain | `0x00487190`, `src/app_sim_tick.rs:683` | architecture acceptable if dirty drain is guaranteed | `OverlayGrid`, app tick dirty drain | Preserve deterministic dirty-cell drain after spread placement | `yr_spread_germination_dirty_cell_recalc_restores_tiberium_land_type` | Do not couple `sim` to render/app; keep boundary via overlay dirty metadata |
| Theater `AllowTiberium` is a real target gate | `0x004838E0`, theater INI `AllowTiberium=true` entries | missing parser | `src/map/theater.rs` / resolved terrain cell data | Parse and expose per-tile `AllowTiberium` or equivalent on `ResolvedTerrainCell` | `yr_spread_rejects_tileset_without_allow_tiberium_even_if_land_buildable` | Do not infer `AllowTiberium` purely from land class; the binary checks both |
| `TiberiumGrowthEnabled` scenario byte gates spread driver | `0x007221B0`; `src/map/basic.rs:75` | partial: Rust applies `basic.tiberium_growth_enabled` only to `grows`, not `spreads` | `src/sim/ore_growth.rs:79-82` | Verify/adjust map flag resolution so disabled tiberium growth suppresses spread driver if binary scenario byte is false | `yr_tiberium_growth_enabled_false_suppresses_spread_driver` | Do not treat growth-enabled as growth-only if scenario byte gates both drivers |

Stale Docs / Follow-up Docs:

- Replace any claim that runtime spread creates "level-1 ore" with: "standard YR runtime spread calls `CellClass::PlaceTiberium(tib_type, 3)`, writes `OverlayData=3`, and selects a random flat overlay variant from the tiberium type image range."
- Replace any claim that spread target validation is "empty adjacent walkable cell" with: "target validation is `CellClass::CanPlaceTiberium`: playfield, no bridge mask `0x500`, no live visible building, no `SpawnsTiberium` terrain object, buildable land type, no overlay, flat slope, and theater `AllowTiberium`."
- Keep the prior claim that `PlaceTiberium` itself does not directly call `RecalcAttributes`; it does call radar dirty only for germination/new placement.

## 10. Negative Facts / Do Not Do

- Do not implement standard runtime spread as source-overlay copying.
- Do not place runtime-spread ore at frame/data 0.
- Do not treat `PathGrid::is_walkable` as a complete replacement for `CanPlaceTiberium`.
- Do not let stock Cruentus gems spread under default YR rules.
- Do not put ore on sloped targets in the standard spread path.
- Do not skip theater `AllowTiberium` forever if the goal is tile-accurate placement parity.
- Do not make `sim` depend on render/radar. Represent dirty/terrain consequences through sim-owned overlay/terrain metadata and let the app/render layer consume them.

## 11. Remaining Uncertainty

- Exact `g_DirectionOffsets[0..7]` cardinal label order remains deferred. The binary proof establishes random start index, wrapped sequential scan, and 8-neighbor table use; a separate direction-table initializer pass can name index 0 as N/NE/etc.
- Save/load serialization of spread/growth queues is outside this slot and should be handled by the queue-state/serialization investigation.
- Exact FPU rounding mode for `ftol(heap_count * SpreadPercentage)` was not runtime-instrumented here. The clamp and batch formula are verified, but a runtime FPU-control-word trace would close the last rounding nuance.

## Sources

- Ghidra decompiled: `0x00487190` `CellClass::PlaceTiberium`
- Ghidra decompiled: `0x004838E0` `CellClass::CanPlaceTiberium`
- Ghidra decompiled: `0x00483780` `CellClass::SpreadTiberium`
- Ghidra decompiled: `0x00483690` `CellClass::CanSpreadTiberium`
- Ghidra decompiled: `0x00483620` `CellClass::CanGrowTiberium`
- Ghidra decompiled: `0x00483720` `CellClass::GrowTiberium`
- Ghidra decompiled: `0x007221B0` `TiberiumClass::SpreadDriver_AllTypes`
- Ghidra decompiled: `0x00722240` `TiberiumClass::InitSpreadQueues_All`
- Ghidra decompiled: `0x00722440` `TiberiumClass::SpreadProcessor`
- Ghidra decompiled: `0x007228B0` `TiberiumClass::RebuildSpreadQueue`
- Ghidra decompiled: `0x00722AF0` `TiberiumClass::AddToSpreadQueue`
- Ghidra decompiled: `0x007235A0` `TiberiumClass::AddToGrowthQueue`
- Ghidra decompiled: `0x006551C0` `RadarClass::MarkTerrainDirty`
- Ghidra decompiled: `0x0047D2B0` `CellClass::RecalcAttributes` spot-check
- Ghidra xrefs: `0x00487190`, `0x004838E0`, `0x00483780`, `0x00722440`, `0x00722AF0`, `0x007221B0`, `0x00722240`, `0x006551C0`
- Prior reports: `CELLCLASS_PLACETIBERIUM_FUN_00487190_GHIDRA_REPORT.md`, `CELL_VALIDATION_TIBERIUM_PLACEMENT_REPORT.md`, `TIBERIUM_QUEUE_SEEDING_AND_TIMING_REPORT.md`, `CELLCLASS_REDUCE_TIBERIUM_FUN_00480A80_GHIDRA_REPORT.md`
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, theater INIs with `AllowTiberium=true`
- Rust scanned: `src/sim/ore_growth.rs`, `src/sim/overlay_grid.rs`, `src/sim/terrain_spawn.rs`, `src/sim/production/production_queue.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`, `src/map/theater.rs`, `src/map/basic.rs`, `src/rules/ruleset.rs`
