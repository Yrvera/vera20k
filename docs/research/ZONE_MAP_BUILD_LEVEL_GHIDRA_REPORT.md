# ZoneMap BuildZoneLevel - Ghidra Re-Investigation

**Date:** 2026-05-14  
**Scope:** `ZoneMap__BuildZoneLevel @ 0x00581F90`, its scanline builder, temporary connection graph, final hierarchical zone graph, and the incremental sibling that reuses the same pipeline.  
**Output type:** Research only. No Rust code changes.

## Verdict

`ZoneMap__BuildZoneLevel` builds one level of the original YR hierarchical pathfinding graph. It is not a simple "zone adjacency extractor". It:

1. Clears a per-level 256-bucket temporary connection graph.
2. Resets the target zone-id slot in the per-cell `MapClass+0x70` zone data.
3. Creates a sentinel zone `0`.
4. Scans the map in level-dependent blocks:
   - level 2: block size `8`
   - level 1: block size `4`
   - level 0: block size `2`
5. Flood-fills same-cluster, height-continuous cells within those blocks into new zone ids.
6. Records per-zone parent, class, growth, and coarse representative fields.
7. Adds active bridge/tube connections into the temporary graph.
8. Emits final bidirectional 8-byte edges into the `MapClass+0x90`-family hierarchical graph.

The final edge flags investigated previously are copied from the temporary graph's third dword. Bridge/tube helpers write zero flags; scanline boundary detection can write flag low byte `1`.

## Active In YR

| Function | Active in YR? | Evidence |
|---|---:|---|
| `FUN_00567110` / full zone-map init | Yes | Called by `MapClass__Resize @ 0x00565C10`; calls `ZoneMap__BuildZoneLevel` for levels `2,1,0`. |
| `FUN_00581F50` / rebuild all zone levels | Yes | Callers include `CCINIClass__Constructor @ 0x00599650`, `FUN_0067E730`, `FUN_00684C30`, `FUN_006E21E0`; clears and rebuilds all three levels. |
| `ZoneMap__BuildZoneLevel @ 0x00581F90` | Yes | Called by full init, all-level rebuild, and incremental rebuild fallback. |
| `ZoneMap__FloodFillScanline @ 0x005824A0` | Yes | Sole direct flood-fill worker for `BuildZoneLevel` and the incremental updater. |
| `FUN_00584550` / incremental block rebuild | Yes | Called by damage, overlay, building placement/sell, terrain limbo, and area-damage paths. |
| `FUN_00582D70` / bridge-tube temp-edge injector | Yes | Called by full build and incremental rebuild when bridge records are active. |

## Pointer/Offset Caution

Existing docs often call the final hierarchical graph `MapClass+0x90`, with absolute singleton level 0 at `0x0087F878`. In `ZoneMap__BuildZoneLevel`, Ghidra renders the final level header as:

```asm
00581fb2: LEA EBX,[ESI + EDX*0x8 + 0x8c]  ; EDX = level * 3
```

Other functions, notably `AddBridgeZoneEdges`, render a related this-base as `+0x90`. The reliable facts for this report are:

- final level header stride is `0x18`
- final zone-block stride is `0x24`
- final edge stride is `8`
- the first absolute level header used by pathfinding docs is `0x0087F878`

Do not use this report alone to settle the inherited/base-pointer naming mismatch.

## Data Layouts

### Per-cell source data at `MapClass+0x68`

Prior docs identify this as a 4-byte-per-cell cache:

| Offset | Meaning |
|---:|---|
| `+0` | zone/class byte |
| `+1` | height byte |
| `+2` | cluster/node id, `u16` |

`BuildZoneLevel` copies the height and cluster into `MapClass+0x70` before rebuilding a level.

### Per-cell hierarchical zone data at `MapClass+0x70`

Stride is `10` bytes per cell:

| Offset | Meaning | Evidence |
|---:|---|---|
| `+0` | level 0 zone id, `u16` | `cell + level*2` writes for level 0 |
| `+2` | level 1 zone id, `u16` | same pattern |
| `+4` | level 2 zone id, `u16` | same pattern |
| `+6` | copied cluster/node id, `u16` | `00582015` -> `00582019` copies from `+68+2` |
| `+8` | copied height byte | `0058201D` -> `0058201F` copies from `+68+1` |
| `+9` | unused/unknown in this pass | no direct use found here |

Reset/copy loop:

```asm
0058200f: MOV word ptr [EAX + EBP*0x2],0x0  ; clear target level zone id
00582015: MOV DX,word ptr [EDI + 0x1]        ; source +68+2 cluster/node id
00582019: MOV word ptr [EAX + 0x6],DX
0058201d: MOV DL,byte ptr [EDI]              ; source +68+1 height
0058201f: MOV byte ptr [EAX + 0x8],DL
```

### Per-level final graph header

Each final graph level header is `0x18` bytes. `BuildZoneLevel` computes it as `this + 0x8c + level*0x18`; other docs call the same final graph family `MapClass+0x90`.

| Offset | Meaning |
|---:|---|
| `+0x00` | pointer to zone-block array |
| `+0x04` | vector-owned storage pointer / capacity metadata, not fully decoded here |
| `+0x08` | capacity or upper bound for zone-block vector |
| `+0x0D` | ownership/grow byte |
| `+0x10` | zone-block count |
| `+0x14` | growth quantum |

### Final zone block, stride `0x24`

| Offset | Meaning | Evidence |
|---:|---|---|
| `+0x00` | edge-vector vtable | copied/constructed by `FUN_0058A500` / `FUN_0058ABD0` |
| `+0x04` | final edge array pointer | final edge writes use it |
| `+0x08` | final edge capacity | compared before grow |
| `+0x0D` | final edge array ownership/grow byte | grow gate |
| `+0x10` | final edge count | incremented on final edge insert |
| `+0x14` | final edge growth quantum | set to `0x10` for zones built here |
| `+0x18` | parent/representative zone id | next-level zone id, or `0` at level 2 |
| `+0x1C` | class/land-type byte widened to dword | copied from `MapClass+0x68` class byte |
| `+0x20` | coarse representative cell index | `x/4 + 0x83 + (y/4)*0x82` |

Zone 0 is a sentinel:

```asm
005820b9: MOV word ptr [EDX + 0x18],DI       ; parent/rep = 0
005820c0: MOV dword ptr [EAX + 0x1c],0x7     ; class = 7
```

The `+0x20` formula:

```asm
0058226c..005822a7: floor_div(y,4) * 0x82 + floor_div(x,4) + 0x83
005822ab: MOV dword ptr [ECX + 0x20],EAX
```

For non-negative map cells this is:

```text
zone_block.representative_cell = (x / 4) + 0x83 + (y / 4) * 0x82
```

`0x82` is a 130-wide internal row stride and `0x83` is `0x82 + 1`, so this points at a one-cell-inset representative of a 4x4 coarse block. Confidence is high for the formula, medium for the descriptive name.

### Temporary connection graph

`BuildZoneLevel` reads a per-level pointer at:

```asm
00581fab: MOV EDI,dword ptr [ESI + EBP*0x4 + 0x80]
```

It then treats `*EDI` as an array of `0x18`-byte bucket headers and emits over exactly `0x1800` bytes:

```asm
0058247d: ADD EDX,0x18
00582480: CMP EDX,0x1800
```

So the temporary graph is `256` buckets (`0x1800 / 0x18 = 256`).

Temporary bucket header, stride `0x18`:

| Offset | Meaning |
|---:|---|
| `+0x00` | entry-vector vtable |
| `+0x04` | temp entry array pointer |
| `+0x08` | capacity |
| `+0x0D` | ownership/grow byte |
| `+0x10` | temp entry count |
| `+0x14` | growth quantum |

Temporary entry, stride `0x0C`:

| Offset | Meaning |
|---:|---|
| `+0x00` | packed pair `(a << 16) | b` |
| `+0x04` | duplicate packed pair |
| `+0x08` | flag dword; low byte is copied to final `edge+4` |

Example insert:

```asm
00582aac: MOV dword ptr [ECX],EDI        ; temp+0 packed pair
00582aae: MOV dword ptr [ECX + 0x4],EDI  ; temp+4 duplicate
00582ab1: MOV dword ptr [ECX + 0x8],EDX  ; temp+8 flag dword
```

The bucket index is the low nibbles of the two zone ids:

```text
bucket = ((a & 0xF) << 4) | (b & 0xF)
```

The duplicate check is exact packed-pair equality within that bucket. It does not sort endpoints first. A reversed packed pair is a different key.

## Full Build Algorithm

### 1. Clear temporary buckets

At entry, `BuildZoneLevel` walks all live temporary buckets and calls their clear/destructor method:

```asm
00581fd8: MOV EAX,dword ptr [EDI]
00581fde: ADD ECX,EAX
00581fe0: MOV EDX,dword ptr [ECX]
00581fe2: CALL dword ptr [EDX + 0xc]
```

`FUN_00588C90` is the clear helper shape: it sets count to 0, frees the owned `+4` data pointer if `+0x0D` is nonzero, clears ownership, and zeros capacity.

### 2. Reset per-cell target level zone ids

For every cell in the `+0x70` zone-data array:

- target level zone id is set to `0`
- cluster id is copied to `+6`
- height is copied to `+8`

This means each level is rebuilt from the current low-level cluster/height cache, not by preserving existing per-level ids.

### 3. Create sentinel zone 0

Zone id 0 is inserted before any real zones. Real zones start at id `1`:

```c
next_zone_id = 1;
zone0.parent = 0;
zone0.class = 7;
```

This matters because `Zone_precheck` and graph walkers can safely treat zero as invalid/sentinel while still having a real block at index zero.

### 4. Compute block size

Block size is:

```text
block_size = 1 << (level + 1)
```

So:

| Level | Block size |
|---:|---:|
| 2 | 8 |
| 1 | 4 |
| 0 | 2 |

The all-level callers process levels in descending order: `2`, then `1`, then `0`.

### 5. Scan the map in row-major order

The scan walks the entire `+0x70` cell array. For each cell it reads the source class byte from `+0x68`:

```c
cell_class = *base68_cell;
```

It skips cells when:

- `cell_class == 7`
- the target level zone id is already nonzero

Otherwise it calls `ZoneMap__FloodFillScanline`.

### 6. Track the current aligned block rectangle

The flood-fill is passed four stack integers:

```text
block_x_min
block_y_min
block_width
block_height
```

For full rebuild these are maintained incrementally:

- width = height = `block_size`
- `block_x_min` updates when `(x & (block_size - 1)) == 0`
- `block_y_min` updates when a map row boundary is crossed

The flood-fill only recurses into unassigned cells inside this block. Cross-block contacts become temporary graph edges instead of expanding the same zone.

### 7. Write the new zone block

After flood-fill returns, `BuildZoneLevel` appends a new final zone block and writes:

```c
zone.parent = (level + 1 < 3) ? cell.level[level + 1] : 0;
zone.class = cell_class;
zone.edge_growth = 0x10;
zone.coarse_representative = x/4 + 0x83 + (y/4)*0x82;
```

Raw sites:

```asm
00582265: MOV dword ptr [ECX + 0x1c],EAX  ; class
00582268: MOV word ptr [ECX + 0x18],DX    ; parent/rep
00582275: MOV dword ptr [ECX + 0x14],0x10 ; edge grow quantum
005822ab: MOV dword ptr [ECX + 0x20],EAX  ; coarse representative
```

### 8. Store level zone count

After the scan completes:

```asm
00582332: MOV dword ptr [ESI + ECX*0x4 + 0x74],EAX
```

This stores the number of zones built for this level, including sentinel zone 0.

### 9. Add active bridge/tube temporary edges

After the regular flood-fill pass, `BuildZoneLevel` walks bridge records:

```asm
00582346: MOV CL,byte ptr [EAX + 0x8]  ; bridge_record.active
00582349: TEST CL,CL
00582358: CALL 0x00582d70
```

Only active records call `FUN_00582D70`.

`FUN_00582D70` handles high/wood bridge records and also has a non-bridge tube path. It computes three connection pairs and inserts them into the temporary graph. All three bridge/tube inserts set the temp flag low byte to `0`.

Example:

```asm
00583020: MOV dword ptr [ESP + 0x30],EAX  ; duplicate packed pair
00583024: MOV byte ptr [ESP + 0x34],0x0    ; flag low byte = 0
0058304b: CALL 0x0058af80                  ; append 12-byte temp entry
```

Last pair:

```asm
0058314f: MOV dword ptr [ESP + 0x30],EAX
00583160: MOV byte ptr [ESP + 0x3c],0x0
00583165: CALL 0x00589e20
```

### 10. Emit final bidirectional graph edges

For each temp bucket and temp entry, the final emitter:

1. Reads the duplicate packed pair from temp `+4`.
2. Reads temp flag low byte from temp `+8`.
3. Emits `low16 -> high16`.
4. Emits `high16 -> low16`.
5. Copies the same flag low byte into both final edge records.

Raw sites:

```asm
00582395: MOV ECX,dword ptr [ESI + -0x4]         ; temp+4 duplicate packed pair
00582398: MOV DL,byte ptr [ESI]                  ; temp+8 low flag byte
...
005823ff: MOV dword ptr [EDX + EAX*0x8],EBP      ; edge+0 neighbor
00582402: MOV dword ptr [EDX + EAX*0x8 + 0x4],ECX; edge+4 flag
...
00582458: MOV dword ptr [EDX + EAX*0x8],EDI      ; reverse edge+0
0058245b: MOV dword ptr [EDX + EAX*0x8 + 0x4],ECX; reverse edge+4
```

The final emitter does not deduplicate against existing final edges. It appends if the final edge vector has capacity or can grow.

## `ZoneMap__FloodFillScanline`

Signature, corrected by use:

```c
int FloodFillScanline(
    MapClass* map,
    CellZone10* start_cell,
    int level,
    uint zone_id,
    int block_rect[4],      // x, y, width, height
    short start_xy[2]
)
```

The return value is the filled horizontal span length used by the caller to skip already-handled cells in the outer row-major scan.

### Continuity rule

The fill stays within cells that satisfy:

- same cluster id at `+70+6`
- height difference less than `2`, i.e. absolute height delta `0` or `1`
- unassigned at the target level, when recursing
- inside the current block rectangle, when recursing into unassigned cells

Height delta is computed with signed absolute-value code:

```c
delta = next_height - previous_height;
if (abs(delta) >= 2) stop_or_skip;
```

This means a height step of exactly `1` remains connected; a step of `2` is a boundary.

### Horizontal fill

The function first walks left from the start cell, then right from the start cell. It assigns the target level zone id to each accepted cell:

```c
cell.level[level] = zone_id;
```

The left walk stops on:

- x before block start
- cluster mismatch
- height delta `>= 2`

The right walk stops on:

- x after block end
- cluster mismatch
- height delta `>= 2`

### Horizontal boundary edges

If the cell just beyond the left/right span already has a different positive zone id and height delta is still less than `2`, the function inserts a zero-flag temporary edge.

The edge is skipped if:

- adjacent zone id is `0`
- adjacent zone id is `0xffffffff` in the left-boundary special check
- adjacent zone id repeats the small local "last seen" cache
- playfield checks fail
- an exact packed-pair duplicate already exists in the temp bucket

### Vertical recursion and vertical boundary edges

For each x in the filled span, the function checks the row above and the row below.

If the neighboring cell is unassigned, within the block rectangle, same cluster, and height-continuous, it recurses.

If the neighboring cell is already assigned to another nonzero zone, height-continuous, and in playfield, it inserts a temporary edge instead.

### Flag low byte

The temp flag low byte is set to `1` only in vertical boundary-edge paths when the x coordinate lies outside the current block's horizontal range:

```asm
00582a28: CMP EBX,ECX
00582a2a: JL 0x00582a39
00582a30: MOV byte ptr [ESP + 0x68],0x0
00582a35: CMP EBX,ECX
00582a37: JLE 0x00582a3e
00582a39: MOV byte ptr [ESP + 0x68],0x1
```

Symmetric branch:

```asm
00582c70: CMP EDI,EAX
00582c72: JL 0x00582c81
00582c78: MOV byte ptr [ESP + 0x68],0x0
00582c7d: CMP EDI,EAX
00582c7f: JLE 0x00582c86
00582c81: MOV byte ptr [ESP + 0x68],0x1
```

In decompiler terms:

```c
flag = (x < block_x_min || x > block_x_max) ? 1 : 0;
```

That flag then becomes final `edge+4` low byte and `Zone_precheck`'s `0.001` tiebreak input.

## Incremental Rebuild Sibling: `FUN_00584550`

`FUN_00584550 @ 0x00584550` is not the full build, but it is important because it reuses the same graph semantics after local map changes.

Callers include (corrected 2026-05-28: was 7 callers; binary shows 11 via `get_function_callers 0x00584550` — STALE, INFERENCE_HARDENED):

- `Apply_area_damage @ 0x00489280`
- `BuildingClass__Place_OccupyMap @ 0x00441F60`
- `HouseClass__Sell_Building_At_Cell @ 0x004FCE80`
- `CellClass__DestroyOverlay @ 0x00480CB0`
- `CellClass__PostDestructionWallCleanup @ 0x00480630`
- `OverlayClass__Mark @ 0x005FC570`
- `TerrainClass__Limbo @ 0x0071C930`
- `AnimClass__Middle @ 0x00424CE0`
- `FUN_00581140 @ 0x00581140`
- `FUN_0074e930 @ 0x0074e930`
- `MapClass__RecalcCellsAndRebuildZones @ 0x00586990`

For each level `2,1,0`, it:

1. Computes the aligned block containing the changed cell:
   - level 2: 8x8
   - level 1: 4x4
   - level 0: 2x2
2. Clears the temporary 256-bucket graph.
3. Collects old zone ids in that block.
4. Clears target level zone ids for cells in the block.
5. Removes final edges pointing to/from the replaced old zones.
6. Clears old final zone blocks.
7. Appends replacement zones starting at the current level zone count.
8. Calls `ZoneMap__FloodFillScanline` for unassigned passable cells in the block.
9. Adds active bridge/tube temp edges for records whose endpoints touch the block.
10. Emits final bidirectional edges from temp buckets.
11. Updates `MapClass+0x74+level*4` zone count.

It also refreshes parent links over an 8x8 area around the changed cell after the three level passes:

```c
for each cell in aligned_8x8_area:
    for level 2 then 1:
        zone_blocks[level][cell.level[level]].parent = cell.level[level+1]
```

That final pass is not present in the same form in full `BuildZoneLevel`, because full build writes parent ids at creation time.

There is a decompiler artifact around `if (puStack_8c == 0)` after incrementing a zone-block offset by `0x24`; this likely represents a capacity/overflow fallback to full three-level rebuild. The high-confidence behavior is that the function can fall back to the all-level rebuild path:

```asm
... clear level
CALL ZoneMap__BuildZoneLevel(level)
...
CALL 0x0042c1c0
```

## Pathfinder Array Refresh

After all three levels are rebuilt, callers invoke `FUN_0042C1C0` with `ECX = 0x87E8B8`, the pathfinder singleton:

```asm
0056721a: MOV ECX,0x87e8b8
0056721f: CALL 0x0042c1c0
```

`FUN_0042C1C0` reallocates/zeros three groups of pathfinder arrays using the current per-level zone counts. This means zone graph rebuild and pathfinder scratch sizing are coupled. A port that rebuilds graph levels without refreshing the pathfinder's zone arrays can keep stale capacities/counts.

## Relationship To `Zone_precheck`

`Zone_precheck @ 0x0042C290` consumes the graph built here:

- starts from level 2 and works down to level 0
- reads zone-block `+0x18` parent ids
- reads zone-block `+0x1C` class/land-type cost index
- scans final edge array at `+0x04`, `+0x10`
- adds `0.001` when final `edge+4` low byte is nonzero

This report fills in the writer side for several previously open `Zone_precheck` fields.

## Rust Relevance

Current Rust does not implement this exact YR hierarchy:

- `zone_hierarchy.rs` is a union-find super-zone reachability cache, not the 3-level block hierarchy.
- `zone_search.rs` runs an approximate corridor Dijkstra over one adjacency graph and then A* with a corridor filter.
- `zone_build.rs` has terrain-aware node/zone construction, but it does not build 8/4/2 block levels, temp buckets, exact final edge flags, or `+0x20` coarse representative fields.

This is not a direct bug by itself. It becomes implementation-critical if the goal is exact `Zone_precheck` parity, route tie ordering, or binary-like hierarchical retry behavior.

## Corrections To Earlier Docs

1. `MapClass+0x90` edge flags are now explained by the `BuildZoneLevel` temporary graph, not by bridge-edge insertion.
2. Zone block `+0x20` is no longer unknown for the full build path: it is `x/4 + 0x83 + (y/4)*0x82`.
3. The three hierarchy levels are explicitly block-sized `2`, `4`, and `8`, built in descending order.
4. The temporary graph is a 256-bucket exact-pair hash using low-nibble bucket selection and 12-byte entries.
5. Bridge/tube records enter before final edge emission, through temporary buckets, with flag low byte `0`.

## Confidence

| Claim | Confidence |
|---|---:|
| Full build levels are processed `2,1,0` | HIGH |
| Block sizes are `8,4,2` for levels `2,1,0` | HIGH |
| `+70` cell stride is 10 bytes and stores three level ids plus cluster/height | HIGH |
| Final zone block stride is `0x24` | HIGH |
| Final edge stride is 8 bytes | HIGH |
| Temporary graph has 256 buckets of stride `0x18` | HIGH |
| Temporary entry stride is `0x0C` | HIGH |
| Temp flag low byte becomes final edge flag low byte | HIGH |
| `+0x20` formula is `x/4 + 0x83 + (y/4)*0x82` | HIGH |
| `+0x20` human name as "coarse representative cell index" | MEDIUM |
| Incremental fallback condition exact trigger | LOW-MEDIUM, due Ghidra stack/register artifact |

## Ghidra Comments Added

Plate comments:

- `FUN_00581F50 @ 0x00581F50`
- `ZoneMap__BuildZoneLevel @ 0x00581F90` (expanded from prior pass)
- `ZoneMap__FloodFillScanline @ 0x005824A0`
- `FUN_00584550 @ 0x00584550`
- `FUN_00582D70 @ 0x00582D70`
- `FUN_0058AF80 @ 0x0058AF80`

Inline comments:

- `0x0058200F` target-level zone id reset
- `0x005820B9` zone 0 sentinel parent
- `0x005820C0` zone 0 sentinel class
- `0x005822AB` zone block `+0x20` formula
- `0x00582AAE` temp entry duplicate packed pair
- `0x00582AB1` temp entry flag dword

## Evidence Sources

Ghidra functions decompiled or rechecked:

- `MapClass__constructor @ 0x00565090`
- `FUN_00567110`
- `FUN_00581F50`
- `ZoneMap__BuildZoneLevel @ 0x00581F90`
- `ZoneMap__FloodFillScanline @ 0x005824A0`
- `FUN_00582D70`
- `FUN_00584550`
- `FUN_0042C1C0`
- `FUN_0042DD60`
- `FUN_00588C90`
- `FUN_00589100`
- `FUN_0058A500`
- `FUN_0058ABD0`
- `FUN_0058AC70`
- `FUN_0058AE60`
- `FUN_0058AF80`
- `FUN_00589E20`

Docs consulted:

- `BRIDGE_ZONE_EDGE_FLAGS_GHIDRA_REPORT.md`
- `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`
- `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md`
- `BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md`
- `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md`
- `PATHFINDERCLASS_GHIDRA_REPORT.md`
- `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md`
- `TODO_ZONE_FIDELITY_FIXES.md`

Rust scan:

- `src/sim/pathfinding/zone_build.rs`
- `src/sim/pathfinding/zone_hierarchy.rs`
- `src/sim/pathfinding/zone_map.rs`
- `src/sim/pathfinding/zone_search.rs`
