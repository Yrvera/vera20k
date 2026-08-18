# Theater Cliff/Ramp Tile Classification - Ghidra Research Report

**Address(es):** `0x00545150`, `0x004863d0`, `0x00578d80`, `0x005746c0`, `0x00486900`
**Investigation Mode:** coverage-map
**Claimed Scope:** Theater INI cliff/ramp/waterfall key reads, numeric range classifiers, bridge-ramp helper liveness, standard YR INI status, and current Rust delta.
**Non-Scope:** Full `CellClass::RecalcAttributes` land-byte derivation, full pathfinder `Can_Enter_Cell` closure, cliff destruction side effects, visual randomization/fixup.
**Confidence:** High for decompiled helper bodies and INI/default facts; medium for standard-map exercise because only a bounded stock-map sample was checked.
**Active in YR:** Yes/conditional. Numeric cliff/ramp/waterfall classification is live in loaded non-lunar theaters. Direct pathfinding callers for `FUN_004863d0` were not found in this slice; the helper is live through bridge/tile application and random map generation. Actual runtime movement primarily consumes cell land/slope/height bytes downstream.

## Summary

Gamemd classifies cliff/ramp tiles by numeric theater tile-id ranges loaded from theater `[General]` keys, not by tileset names. The read path converts `[General]` tileset ordinals such as `CliffSet=10` into cumulative tile-id starts during theater load, then helper functions test hard-coded half-open ranges such as `[CliffSet, CliffSet+0x28)` and `[CliffRamps, CliffRamps+0x14)`.

The most important Rust-facing mismatch is that current Rust still uses SetName/TMP-land heuristics (`contains("cliff")`, `contains("shore")`, `contains("rock")`, `terrain_rules`) and does not store the binary's cliff/ramp/waterfall range globals. That can overblock shores/rocks and under-model waterfall slope exceptions and exact range boundaries.

## Verified Binary Findings

### Theater INI reader

`Read_Theater_TileSets_INI @ 0x00545150` reads these theater `[General]` keys with `CCINIClass::ReadInt`, then maps the ordinal value to the current cumulative tile-id start while iterating `[TileSetNNNN]` sections:

| Key | Stack temp in decompile | Runtime global | Default | Binary mapping |
|---|---:|---:|---:|---|
| `CliffSet` | `iStack_918` | `DAT_00aa1020` | `-1` | write cumulative tile id when tileset ordinal equals key value |
| `CliffRamps` | `iStack_8a4` | `DAT_00abbebc` | `-1` | same |
| `WaterCliffs` | `iStack_944` | `DAT_00aa101c` | `-1` | same |
| `DestroyableCliffs` | `iStack_94c` | `DAT_00abc2c8` | `-2` | same |
| `WaterCaves` | `iStack_93c` | `DAT_00abad24` | `-1` | same |
| `WaterfallEast` | `iStack_89c` | `DAT_00aa073c` | `-1` | same |
| `WaterfallWest` | `iStack_8ec` | `DAT_00abb110` | `-1` | same |
| `WaterfallNorth` | `iStack_8bc` | `DAT_00aa10a0` | `-1` | same |
| `WaterfallSouth` | `iStack_8e4` | `DAT_00aa1050` | `-1` | same |

Evidence: decompile of `0x00545150`; key read block around `0x00545714`; global reset block initializes `DAT_00aa1020`, `DAT_00abbebc`, `DAT_00aa101c` to `-1` and `DAT_00abc2c8` to `-2`; assignment block writes globals from `iVar16`, the cumulative tile-id cursor.

Tiny details:

- The INI value is a tileset ordinal, not a tile id. The global stores a cumulative tile id.
- The classifier ranges are fixed constants and can extend beyond the first tileset count. Example: `CliffRamps` checks `+0x14` (20 tile ids), while stock theater set 25 often has 10 files and the next Z/MM ramp set supplies the rest.
- Lunar/interior theater path (`local_95c == 5`) zeros `ShorePieces`, `WaterSet`, `CliffSet`, `WaterCliffs`, `WaterBridge`, `BridgeSet`, and `WoodBridgeSet` before returning. It does not visibly zero every cliff-adjacent global in this decompile slice.

### `IsCliffOrImpassableTile @ 0x004863d0`

Input is an `IsometricTileTypeClass*` in `ECX`. The function reads:

| Field | Offset | Meaning in this helper |
|---|---:|---|
| tile id / tile set index field | `+0x38` | Numeric cumulative tile id tested against theater globals |
| waterfall slope byte | `+0x11a` | Direction/slope exception byte for waterfall edge tiles |

Return `AL=1` means "classified as cliff/impassable" for this helper. Return `AL=0` means not classified.

Verified half-open ranges:

| Range | Binary condition |
|---|---|
| `CliffSet` | `DAT_00aa1020 != -1 && tile >= CliffSet && tile < CliffSet + 0x28` |
| `CliffRamps` | `DAT_00abbebc != -1 && tile >= CliffRamps && tile < CliffRamps + 0x14` |
| `WaterCliffs` | `DAT_00aa101c != -1 && tile >= WaterCliffs && tile < WaterCliffs + 0x1c` |
| `DestroyableCliffs` | `DAT_00abc2c8 != -1 && tile >= DestroyableCliffs && tile < DestroyableCliffs + 2` |
| `BridgeSet` | `DAT_00aa0e28 != -1 && tile >= BridgeSet && tile < BridgeSet + 0x10` |
| `WoodBridgeSet` | `DAT_00abad1c != -1 && tile >= WoodBridgeSet && tile < WoodBridgeSet + 0x10` |
| `WaterCaves` | `DAT_00abad24 != -1 && tile >= WaterCaves && tile < WaterCaves + 4` |

Waterfall exception logic:

| Key/global | Range | Edge tiles that can be passable | Passable slope bytes on those edge tiles | Middle tiles |
|---|---|---|---|---|
| `WaterfallEast` / `DAT_00aa073c` | `[base, base+4)` | `base`, `base+3` | `0` or `4` | always impassable |
| `WaterfallWest` / `DAT_00abb110` | `[base, base+4)` | `base`, `base+3` | `1` or `3` | always impassable |
| `WaterfallSouth` / `DAT_00aa1050` | `[base, base+4)` | `base`, `base+3` | `0` or `1` | always impassable |
| `WaterfallNorth` / `DAT_00aa10a0` | `[base, base+4)` | `base`, `base+3` | `2` or `3` | always impassable |

Evidence: decompile and disassembly of `0x004863d0..0x004865ac`.

### `IsOnBridgeRamp @ 0x00578d80`

This helper takes numeric tile id in `ECX` and slope/direction byte in `EDX`. It returns true only for:

- `CliffSet` range `[CliffSet, CliffSet+0x28)`.
- Four waterfall ranges, with the same edge-tile/slope exceptions as `FUN_004863d0`.
- `CliffRamps` range `[CliffRamps, CliffRamps+0x14)`.

It does not check `WaterCliffs`, `DestroyableCliffs`, `WaterCaves`, `BridgeSet`, or `WoodBridgeSet`.

Tiny detail: unlike `FUN_004863d0`, this function does not test `global != -1` before range checks. It appears to rely on the caller/theater context having meaningful globals. This matters for any future reusable Rust helper: do not silently add sentinel guards unless a caller-equivalence proof exists.

Evidence: decompile and disassembly of `0x00578d80..0x00578e5a`; only xref found in this slice is `MapClass__ApplyBridgeTile @ 0x0057b74e`.

### `MapClass::IsBridgeRampTile @ 0x005746c0`

This is a separate bridge-specific helper. It checks bridge-ramp tile ids loaded from `BridgeTopRight*`, `BridgeTopLeft*`, `BridgeMiddle1`, and `BridgeMiddle2`, combined with `cell+0x11a` values:

| Global(s) | Required `cell+0x11a` |
|---|---:|
| `DAT_00aa1548`, `DAT_00aa0740` | `0x0c` |
| `DAT_00abad30..+3` | `0x04` |
| `DAT_00abc2b4`, `DAT_00aa1130` | `0x08` |
| `DAT_00aa1028..+3` | `0x02` |

Evidence: decompile and disassembly of `0x005746c0..0x00574772`; xrefs found from bridge-destruction functions `0x00574415` and `0x00575031`.

### DestroyableCliffs check

`FUN_00486900 @ 0x00486900` returns true only when `tile+0x38 == DAT_00abc2c8` or `tile+0x38 == DAT_00abc2c8 + 1`.

Evidence: decompile and disassembly of `0x00486900..0x00486916`.

This corrects a stale claim in `CLIFF_OBJECTS_GHIDRA_REPORT.md`: local standard YR theater INIs do define `DestroyableCliffs` for non-lunar theaters. Whether stock maps place those two tile ids enough to exercise cliff destruction remains outside this slot, but the INI reader and classifier are not dormant merely because the default is `-2`.

## Active YR Status

Local standard YR theater INI facts:

| Theater INI | CliffSet | WaterCliffs | CliffRamps | Waterfalls | DestroyableCliffs | Notes |
|---|---:|---:|---:|---:|---:|---|
| `temperatmd.ini` | set 10, 40 tiles | set 15, 28 | set 25 | four 4-tile sets | set 56, 2 | active non-lunar |
| `snowmd.ini` | set 10, 40 | set 15, 28 | set 25 | four 4-tile sets at 30/35/36/37 | set 61, 2 | active non-lunar |
| `urbanmd.ini` | set 10, 40 | set 15, 28 | set 25 | four 4-tile sets | set 56, 2 | `WaterCaves` key maps to 0-count set |
| `urbannmd.ini` | set 10, 40 | set 15, 28 | set 25 | four 4-tile sets | set 56, 2 | `WaterCaves` key maps to 0-count set |
| `desertmd.ini` | set 10, 40 | set 15, 0 | set 25 | keys present but 0-count sets | set 56, 2 | water/waterfall paths mostly inert by content |
| `lunarmd.ini` | key present, 0-count | key present, 0-count | key present, 0-count | keys present, 0-count | key present, 0-count | lunar special load path zeros some water/cliff globals |

Bounded stock-map exercise check:

- `Transylv.yro` loaded as `NEWURBAN`, max tile id `1169`, and loaded `urbannmd.ini`; this theater has active cliff/waterfall/destroyable ranges.
- `DeepFrze.yro` loaded as `SNOW`; tile loading first needed TMP included `Clifx10.sno`, proving stock-map cliff TMP use in the sampled map.
- `MoonPatr.yro` loaded as `LUNAR`; lunar cliff/water/waterfall sets are 0-count and the loader has a lunar zeroing branch for several water/cliff globals.

## Rust Delta

Current Rust surfaces:

| Surface | Current behavior | Delta |
|---|---|---|
| `src/map/theater.rs::TheaterData` | Stores bridge/tunnel keys only. No `CliffSet`, `CliffRamps`, `WaterCliffs`, `DestroyableCliffs`, `Waterfall*`, or `WaterCaves` fields. | Missing numeric classifier inputs. |
| `src/map/theater.rs::TilesetLookup::is_cliff` | Classifies by SetName containing `cliff`. | Not gamemd-equivalent; misses `CliffRamps`/waterfall numeric rules and depends on localized/string naming. |
| `src/map/resolved_terrain.rs::metadata_from_set_name` | Sets `is_cliff_like` on names containing `cliff`, `rock`, or `shore`. | Overblocks non-cliff shore/rock names relative to this numeric helper; may also under-model waterfall exceptions. |
| `src/rules/terrain_rules.rs` | Land-type semantics mark `Rock`/`Cliff` as cliff-like blocked. | Useful for land-byte passability, but not a replacement for theater tile-id range classification. |
| `src/sim/pathfinding/terrain_cost.rs::from_resolved_terrain` | Treats `cell.is_cliff_like` as hard-blocked unless bridge deck overrides. | Depends on the heuristic above, so classifier drift becomes pathfinding drift. |

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Theater `[General]` cliff keys map from ordinal to cumulative tile-id start. | `0x00545150` key read and assignment blocks | missing | `src/map/theater.rs::TheaterData`, `load_theater` | Store numeric starts for `CliffSet`, `CliffRamps`, `WaterCliffs`, `DestroyableCliffs`, `WaterCaves`, `Waterfall*` using `TilesetLookup::bounds()[ordinal].start`, preserving omitted/default behavior. | For `temperatmd.ini`, `CliffSet=10` resolves to tile-id start `49`, not ordinal `10`; `[49,89)` classifies as cliff set. | Do not compare raw tileset ordinals to map tile ids. |
| `IsCliffOrImpassableTile` uses exact half-open ranges and waterfall exceptions. | `0x004863d0..0x004865ac` | mismatch/partial | theater/resolved terrain classification helper | Implement a numeric predicate matching the binary ranges and waterfall edge-tile exceptions. | Tile at `WaterfallEast+0` with slope byte `0` is not impassable by this helper; `WaterfallEast+1` is impassable regardless of slope. | Do not block all waterfall tiles solely because SetName contains `Waterfalls`. |
| `IsOnBridgeRamp` checks only `CliffSet`, `CliffRamps`, and waterfall exceptions, with no explicit sentinel guards. | `0x00578d80..0x00578e5a`; xref `0x0057b74e` | missing as exact helper | bridge/tile application and any future ramp predicate | Keep this predicate separate from the broader impassable predicate. | A `WaterCliffs` tile returns false from `IsOnBridgeRamp` even though it returns true from `IsCliffOrImpassableTile`. | Do not reuse the broader cliff/impassable classifier for bridge-ramp decisions. |
| `DestroyableCliffs` default is `-2`, but standard non-lunar theater INIs define the key and have 2-tile sets. | `0x00545150`, `0x00486900`, local `ini/*md.ini` | stale docs, Rust unchecked | future cliff destruction and classification docs | Treat destruction behavior as a separate follow-up, but the classifier input is not globally dormant by default. | `temperatmd.ini` set 56 resolves to a 2-tile range; `FUN_00486900` matches only those two tile ids. | Do not repeat the stale "no standard YR theater defines DestroyableCliffs" claim. |
| Current pathfinding hard-blocks `is_cliff_like` from name/land heuristics. | `src/map/resolved_terrain.rs`, `src/sim/pathfinding/terrain_cost.rs` | drift risk | resolved terrain to pathfinding cost grid | Feed pathfinding from numeric/theater-aware classification plus verified cell land/slope bytes, not broad strings. | A shore tile whose SetName contains `Shore` but is outside numeric cliff/water cliff ranges is not blocked merely by name. | Do not use `contains("shore")` as cliff equivalence. |

## Acceptance Tests

1. Theater parser resolves ordinals to cumulative tile starts: `temperatmd.ini CliffSet=10 -> start 49`, `WaterCliffs=15 -> start 148`, `DestroyableCliffs=56 -> start 572`.
2. Boundary tests: `CliffSet+39` is true, `CliffSet+40` is false; `WaterCliffs+27` true, `WaterCliffs+28` false; `DestroyableCliffs+1` true, `DestroyableCliffs+2` false.
3. Waterfall tests: East `base+0` slope `0`/`4` false, slope `1` true; East `base+1` true for every slope; West/North/South use the exact slope pairs listed above.
4. Predicate separation test: a `WaterCliffs` tile is true for the broad impassable predicate and false for `IsOnBridgeRamp`.
5. Rust pathfinding fixture: a shore-only tile outside cliff/water-cliff numeric ranges must not become `is_cliff_like` just because the SetName contains `Shore`.
6. Lunar fixture: lunar 0-count cliff/water ranges and the loader's lunar zeroing behavior must not classify ordinary lunar terrain as cliff/ramp.

## Remaining Uncertainty

- `[DEFERRED] OQ1 - Full pathfinder caller closure for FUN_004863d0.` (category: requires-different-system-context; reason: xrefs in this slot show bridge/tile application and random map generation, not direct A*/Can_Enter_Cell callers; next-step-if-pursued: trace cell land-byte derivation through `CellClass::RecalcAttributes` and `Can_Enter_Cell`.)
- `[DEFERRED] OQ2 - Exact stock-map frequency for each range.` (category: bounded-cost-too-high; reason: only three stock maps were loaded as a liveness sample; next-step-if-pursued: run a map corpus scan counting tile ids in each resolved numeric range.)
- `[DEFERRED] OQ3 - Destroyable cliff destruction active map usage.` (category: out-of-scope; reason: this slot verified key/read/classifier facts, not destruction triggers; next-step-if-pursued: trace `FUN_00581140` callers plus map corpus tile placement.)
- `[DEFERRED] OQ4 - Lunar IsOnBridgeRamp sentinel edge if called directly.` (category: requires-different-system-context; reason: the helper lacks `-1` guards and lunar zeroes only some globals, but caller evidence did not show a live lunar bridge path; next-step-if-pursued: trace all bridge-application callers under lunar maps.)

## Sources

- Ghidra decompile: `Read_Theater_TileSets_INI @ 0x00545150`
- Ghidra decompile/disassembly: `FUN_004863d0 @ 0x004863d0..0x004865ac`
- Ghidra decompile/disassembly: `IsOnBridgeRamp @ 0x00578d80..0x00578e5a`
- Ghidra decompile/disassembly: `MapClass__IsBridgeRampTile @ 0x005746c0..0x00574772`
- Ghidra decompile/disassembly: `FUN_00486900 @ 0x00486900..0x00486916`
- Ghidra xrefs: `FUN_004863d0`, `IsOnBridgeRamp`, `MapClass__IsBridgeRampTile`, `Read_Theater_TileSets_INI`
- INI checked: `ini/temperatmd.ini`, `ini/snowmd.ini`, `ini/urbanmd.ini`, `ini/urbannmd.ini`, `ini/desertmd.ini`, `ini/lunarmd.ini`
- Rust checked: `src/map/theater.rs`, `src/map/resolved_terrain.rs`, `src/rules/terrain_rules.rs`, `src/sim/pathfinding/terrain_cost.rs`
- Prior docs checked: `docs/research/CLIFF_OBJECTS_GHIDRA_REPORT.md`, `docs/research/CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md`, `docs/research/ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md`, `docs/research/LAT_GROUPS_AND_SLOPE_FIXUP_GHIDRA_REPORT.md`, `docs/research/SEA_TILES_GHIDRA_REPORT.md`
