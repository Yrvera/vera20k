# Bridge Direction Tables - Ghidra Research Report

**Addresses:** `0x0082A734`, `0x0082A774`, `0x0082A7B4`, `0x0082A7F4`, `0x0082A89C`, `0x0082A944`  
**Primary functions checked:** `MapClass::ComputeBridgeZones` at `0x0056D6E0`, `MapClass::AddBridgeZoneEdges` at `0x005851B0`, `MapClass::RemoveBridgeZoneEdges` at `0x00584E50`, `FUN_00582D70`  
**Confidence:** High for table contents and direct readers; medium for human-readable direction names because `g_DirectionOffsets` at `0x0089F688` is BSS/runtime-initialized in the static image.  
**Active in YR:** Yes. These readers are on map-load bridge-zone construction and runtime bridge validate/invalidate paths.

## 1. Overview

This report closes the open bridge-zone question from `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` section 16 about `DAT_0082A944`, and also records the adjacent bridge tables that `ComputeBridgeZones` uses.

The most important result: `DAT_0082A944` is exactly a 16-entry `i32` table. Its values match the older `BRIDGE_SYSTEM.md` table:

```text
[0, 0, -1, 2, 2, -1, 0, 0, 0, 0, 0, 2, 2, 2, 2, 2]
```

`AddBridgeZoneEdges`, `RemoveBridgeZoneEdges`, and `FUN_00582D70` use it by indexing `IsoTileTypeIndex - bridge_base`, then walking one cell in `dir & 7` and the opposite direction `(dir - 4) & 7`.

## 2. Table Contents

All values below are signed 32-bit integers read directly from Ghidra memory.

### 2.1 `DAT_0082A944` - bridgehead step direction, 16 entries

Address: `0x0082A944`, length 64 bytes.

| Index | Value |
|---:|---:|
| 0 | 0 |
| 1 | 0 |
| 2 | -1 |
| 3 | 2 |
| 4 | 2 |
| 5 | -1 |
| 6 | 0 |
| 7 | 0 |
| 8 | 0 |
| 9 | 0 |
| 10 | 0 |
| 11 | 2 |
| 12 | 2 |
| 13 | 2 |
| 14 | 2 |
| 15 | 2 |

Evidence:

```text
read_memory(0x0082A944, 64)
00000000 00000000 ffffffff 02000000
02000000 ffffffff 00000000 00000000
00000000 00000000 00000000 02000000
02000000 02000000 02000000 02000000
```

Tiny detail: entries `2` and `5` are `-1`. The callers do not special-case `-1` after reading this table. In the active Add/Remove/FUN_00582D70 path, the function first proves the cell is a bridge/wood-bridge tile, reads the table, then applies `dir & 7` and `(dir - 4) & 7`. Therefore `-1` becomes direction index `7`, and `(dir - 4) & 7` becomes direction index `3`. Whether these tile offsets are reachable in normal edge-building depends on the tile class chosen by the preceding bridge-cell predicates; do not assume `-1` means "skip" in these callers.

## 3. Adjacent Bridge Tables

These tables are adjacent in memory and are used by bridge record construction, bridge compatibility, and direction classification.

### 3.1 `DAT_0082A734` - start-height / compatibility gate, 16 entries

Address: `0x0082A734`, length 64 bytes.

```text
[7, 7, -1, 7, 7, -1, 4, 4, 4, 4, 4, 2, 2, 2, 2, 2]
```

`ComputeBridgeZones` reads this table at `0x0056D882` and compares the entry to `CellClass.Height`. If the value does not match, the cell is not used as the bridge-record start for that scan.

### 3.2 `DAT_0082A774` - bridge walk direction, 16 entries

Address: `0x0082A774`, length 64 bytes.

```text
[2, 2, -1, 4, 4, -1, 2, 2, 2, 2, 2, 4, 4, 4, 4, 4]
```

`ComputeBridgeZones` reads this table at `0x0056D88F` after the height gate succeeds. The value is passed to `Pathfinding_update_continued` to walk the bridge span and find the opposite endpoint.

Tiny detail: this table also has `-1` at indices `2` and `5`. In `ComputeBridgeZones`, those indices should normally fail the prior `DAT_0082A734` height gate unless the cell height equals `-1`, so the `-1` walk direction is effectively guarded.

### 3.3 `DAT_0082A7B4` - end-height / terminator gate, 16 entries

Address: `0x0082A7B4`, length 64 bytes.

```text
[-1, -1, 4, -1, -1, 2, 4, 4, 4, 4, 4, 2, 2, 2, 2, 2]
```

`ComputeBridgeZones` reads this table at `0x0056D91D` while walking the span. It compares the table entry against the current candidate cell height. A match marks that the opposite endpoint was found.

### 3.4 `DAT_0082A7F4` - bridge height-class table, 42 entries

Address: `0x0082A7F4`, length 168 bytes.

```text
[0, 0, 0, 1, 2, 3, 4, 4, 5, 5, 5, 6, 7, 8,
 9, 9, 10, 10, 10, 11, 12, 13, 14, 14, 15, 15, 15, 16,
 17, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 23, 24, 25]
```

This table is not the `DAT_0082A944` bridgehead-step table. It begins immediately after the 16-entry end-height table.

### 3.5 `DAT_0082A89C` - bridge direction-class table, 42 entries

Address: `0x0082A89C`, length 168 bytes.

```text
[4, 4, 4, 4, 4, 4, 3, 3, 2, 2, 2, 2, 2, 2,
 1, 1, 0, 0, 0, 0, 0, 0, 7, 7, 6, 6, 6, 6,
 6, 6, 5, 5, 3, 3, 1, 1, 7, 7, 5, 5, 4, 4]
```

This table is also not `DAT_0082A944`. It is a separate 42-entry classifier.

## 4. Direct Binary Readers

### 4.1 `MapClass::AddBridgeZoneEdges` at `0x005851B0`

Direct read at `0x0058523F`:

```asm
MOV EDI,dword ptr [EAX*0x4 + 0x82a944]
```

Context:

- `EAX` is `cell.IsoTileTypeIndex - bridge_base`.
- `bridge_base` is `DAT_00AA0E28` for high bridges or `DAT_00ABAD1C` for wood/low bridges.
- The code derives four coords using `dir & 7` and `(dir - 4) & 7`.
- Those coords provide two extra bridgehead-extension edge pairs in addition to endpoint_a <-> endpoint_b.

Observed decompile pattern:

```text
dir = DAT_0082A944[cell.IsoTileTypeIndex - bridge_base]
coord1 = endpoint + g_DirectionOffsets[dir & 7]
coord2 = endpoint + g_DirectionOffsets[(dir - 4) & 7]
```

Tiny detail: Add does not deduplicate edges. If the same bridge record is validated twice, this table will drive another append of the same derived edge pairs.

### 4.2 `MapClass::RemoveBridgeZoneEdges` at `0x00584E50`

Direct read at `0x00584EEB`:

```asm
MOV EDI,dword ptr [EAX*0x4 + 0x82a944]
```

This is the same table access pattern as Add. Remove computes the same coord pairs, then finds and removes the first matching edge from each zone block. The Add/Remove pair is therefore a strict inverse only for single Add followed by single Remove. Duplicate edges remain possible after Add x2, Remove x1.

### 4.3 `FUN_00582D70`

Direct read at `0x00582F42`:

```asm
MOV ESI,dword ptr [EAX*0x4 + 0x82a944]
```

This function uses the same high-vs-wood bridge-base selection and the same `dir & 7` / `(dir - 4) & 7` direction-pair idea. If the current cell is not a bridge/wood bridge, it falls back to tube handling via `CellClass::GetTubeAtCell` instead of reading `DAT_0082A944`.

## 5. ComputeBridgeZones Use of the Adjacent Tables

`MapClass::ComputeBridgeZones` at `0x0056D6E0` uses the 16-entry tables in order:

1. Determine high vs wood/low bridge base (`DAT_00AA0E28` or `DAT_00ABAD1C`).
2. Compute `tile_offset = CellClass.IsoTileTypeIndex - bridge_base`.
3. Compare `DAT_0082A734[tile_offset]` to `CellClass.Height`.
4. If equal, read `DAT_0082A774[tile_offset]` as the bridge-span walk direction.
5. Walk cells with `Pathfinding_update_continued`.
6. While walking, compare `DAT_0082A7B4[next_tile_offset]` to the candidate cell height to detect the far endpoint.

Tiny details:

- `DAT_0082A734` is both a real bridge table and the address immediately after `g_PassabilityMatrix`; xrefs that compare a pointer against `0x82A734` may be passability-table loop bounds, not bridge-table reads.
- `DAT_0082A774` is read only after the start-height comparison succeeds.
- `DAT_0082A7B4` is read while scanning the far endpoint, not during the initial start-cell gate.
- `ComputeBridgeZones` writes `BridgeRecord+0x0C = 0` for the high-bridge path and `1` for the low/wood path.

## 6. INI Keys

No INI key populates these tables. They are static binary lookup tables.

Relevant INI data only determines which overlay/tile types exist:

| INI | Key/section | Relevance |
|---|---|---|
| `rulesmd.ini` | `[OverlayTypes]` entries `LOBRDG01..LOBRDG28`, `LOBRDB01..LOBRDB28`, endpoints | Names the bridge overlay types. |
| `rulesmd.ini` | `[CABHUT] BridgeRepairHut=yes` | Repair hut interaction eventually calls bridge validate/repair paths, but does not alter these tables. |
| `rulesmd.ini` | `[CombatDamage] DestroyableBridges=yes`, `BridgeStrength=1500` | Enables bridge damage/repair behavior, not table content. |
| `artmd.ini` | `[LOBRDG*]`, `[LOBRDB*]` | Art binding for bridge overlays. |

YR `rulesmd.ini` keeps `DestroyableBridges=yes`; therefore the Add/Remove edge paths are active in normal YR.

## 7. Current Rust Implementation Status

Rust does not appear to have a direct port of the `MapClass+0x90` hierarchical zone graph or these static tables.

Relevant current Rust areas:

| Rust area | Status |
|---|---|
| `src/sim/bridge_state/mod.rs` | Builds `BridgeRuntimeState` and endpoint records from resolved terrain, not from these exact binary lookup tables. |
| `src/sim/pathfinding/zone_build.rs` | Builds bridge adjacency/redirects in Rust's own zone model; no direct 3-level Add/RemoveBridgeZoneEdges equivalent. |
| `src/sim/pathfinding/zone_map.rs` | Has bridge redirects, but not the original hierarchical graph edge arrays. |

This is acceptable as an internal design difference only if the observable pathing and bridge invalidation/revalidation behavior match. If the original 3-level graph is ported later, `DAT_0082A944` should be represented as the exact 16-entry signed table above, and the `-1` entries must not be silently treated as "no direction" in Add/Remove without proving the same guard conditions.

## 8. Resolved / Open

Resolved:

- `DAT_0082A944` is a 16-entry signed i32 table.
- Its exact values are verified from memory.
- It is read by Add, Remove, and `FUN_00582D70`.
- Add/Remove use `dir & 7` and `(dir - 4) & 7`; they do not treat `-1` as a skip at the read site.
- `DAT_0082A734`, `DAT_0082A774`, and `DAT_0082A7B4` are separate 16-entry tables with full contents listed above.
- `DAT_0082A7F4` and `DAT_0082A89C` are separate 42-entry classifier tables with full contents listed above.

Still open:

- Human-readable compass labels for direction indices should be tied to the runtime-initialized `g_DirectionOffsets` table, not guessed from static BSS.
- The bridge-edge flag low-byte semantic in the `MapClass+0x90` edge array is not resolved by this report.
- The unknown fields in the `MapClass+0x90` level header and zone blocks remain open.

## Sources

Ghidra memory reads:

- `read_memory(0x0082A734, 64)`
- `read_memory(0x0082A774, 64)`
- `read_memory(0x0082A7B4, 64)`
- `read_memory(0x0082A7F4, 168)`
- `read_memory(0x0082A89C, 168)`
- `read_memory(0x0082A944, 64)`

Ghidra decompilation / assembly:

- `MapClass::ComputeBridgeZones` at `0x0056D6E0`
- `MapClass::AddBridgeZoneEdges` at `0x005851B0`
- `MapClass::RemoveBridgeZoneEdges` at `0x00584E50`
- `FUN_00582D70`
- `MapCoord_Add` at `0x0042D510`
- `Pathfinding_update_continued` at `0x00481810`
- `get_bulk_xrefs(0x0082A944, 0x0082A734, 0x0082A774, 0x0082A7B4)`
- `get_assembly_context` for `0x00582F42`, `0x00584EEB`, `0x0058523F`, `0x0056D882`, `0x0056D88F`, `0x0056D91D`

Docs referenced:

- `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md`
- `BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md`
- `BRIDGE_SYSTEM.md`
- `MAPCLASS_GHIDRA_REPORT_FOLLOWUP.md`
- `ADDRESS_MAP.md`

