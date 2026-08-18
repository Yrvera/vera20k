# Bridge Zone Edge Flags — Ghidra Re-Investigation

**Date:** 2026-05-14  
**Scope:** Resolve the `MapClass+0x90` hierarchical-zone edge `+4` low-byte semantic, especially the conflict between “bridge-edge flag” wording and `AddBridgeZoneEdges` writing zero.  
**Output type:** Research only. No Rust code changes.

## Verdict

The `edge+4` low byte is real and is read by `Zone_precheck`. If nonzero, `Zone_precheck` adds `0.001` to that candidate zone-edge cost.

However, the flag is **not proven to mean “bridge edge”**. The cold-init writer found in this pass, `ZoneMap__FloodFillScanline @ 0x005824A0`, sets the low byte to `1` for a hierarchy-building scanline/block-boundary condition. `ZoneMap__BuildZoneLevel @ 0x00581F90` then copies that byte into final `MapClass+0x90` edge entries.

`MapClass__AddBridgeZoneEdges @ 0x005851B0`, used when a bridge is repaired/validated, writes zero to `edge+4` for all inserted bridge adjacency edges. Those repaired-bridge edges therefore **do not receive** the `0.001` flag cost in `Zone_precheck`.

Recommended wording in future docs: **zone edge tiebreak flag** or **hierarchy-boundary edge flag**, not “bridge-edge flag”.

## Prior Contradiction

Relevant existing docs:

- `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md` correctly identified `Zone_precheck` reading `edge+4` low byte and adding `0.001`, but described the byte as “1 if bridge-edge”.
- `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` and `BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md` correctly noticed that `AddBridgeZoneEdges` writes zero to this field and left the semantic open.

This report resolves the contradiction by finding the cold-init source of nonzero flag bytes.

## Active In YR

| Function | Active in YR? | Evidence |
|---|---:|---|
| `FUN_00567110` / Map zone init | Yes | Called by `MapClass__Resize @ 0x00565C10`; allocates zone arrays, computes bridge zones, calls `ZoneMap__BuildZoneLevel` for levels 2, 1, 0. |
| `ZoneMap__BuildZoneLevel @ 0x00581F90` | Yes | Called from map zone init, `FUN_00581F50`, and `FUN_00584550`; builds final hierarchical graph used by pathfinding. |
| `ZoneMap__FloodFillScanline @ 0x005824A0` | Yes | Called by `ZoneMap__BuildZoneLevel` during hierarchy construction. |
| `MapClass__AddBridgeZoneEdges @ 0x005851B0` | Yes | Called by `MapClass__ValidateBridgeZones @ 0x0056DB70` when a bridge record becomes active. |
| `MapClass__RemoveBridgeZoneEdges @ 0x00584E50` | Yes | Called by `MapClass__InvalidateBridgeZones @ 0x0056DAE0` when a bridge record becomes inactive. |
| `Zone_precheck @ 0x0042C290` | Yes | Called by `AStar_pathfind_search @ 0x0042C900` and `FUN_0042D170`. |

## Data Layout

`MapClass+0x90` is the hierarchical zone graph region. The exact singleton address varies with base interpretation in inherited class contexts, but field access resolves to the known global `0x0087F878` for the first level header.

Level headers:

| Level | Address | Header stride |
|---:|---:|---:|
| 0 | `0x0087F878` | `0x18` |
| 1 | `0x0087F890` | `0x18` |
| 2 | `0x0087F8A8` | `0x18` |

Within each level, `zone_blocks_array[zone_id]` has stride `0x24`.

Zone block fields used here:

| Offset | Meaning | Evidence |
|---:|---|---|
| `+0x04` | edge array pointer | `BuildZoneLevel`, `AddBridgeZoneEdges`, `Zone_precheck` read/write through it. |
| `+0x08` | capacity | Compared against count before grow. |
| `+0x0D` | grow/ownership byte | Grow fallback gate. |
| `+0x10` | edge count | Incremented on insert, decremented on remove. |
| `+0x14` | growth quantum | Usually set to `0x10` for final zone blocks in `BuildZoneLevel`. |
| `+0x18` | parent/representative zone id | Written by `BuildZoneLevel`; read by `Zone_precheck`. |
| `+0x1C` | land type/class index | Written by `BuildZoneLevel`; read by `Zone_precheck`. |
| `+0x20` | coarse representative/cell index | Written by `BuildZoneLevel`; exact consumer not part of this pass. |

Final edge entry layout:

| Offset | Meaning |
|---:|---|
| `+0x00` | neighbor zone id |
| `+0x04` | flag dword; `Zone_precheck` consumes only low byte |

Temporary connection entries used by `ZoneMap__BuildZoneLevel` are 12 bytes:

| Offset | Meaning |
|---:|---|
| `+0x00` | packed zone pair |
| `+0x04` | duplicate packed zone pair |
| `+0x08` | flag dword; low byte is copied into final edge `+4` |

## Finding 1 — `Zone_precheck` Adds `0.001` Only When `byte(edge+4) != 0`

`Zone_precheck @ 0x0042C290` reads the final graph edge flag low byte:

```asm
0042c53e: MOV EBX,dword ptr [EAX]          ; edge+0 neighbor zone id
0042c540: MOV DL,byte ptr [EAX + 0x4]      ; low byte of edge+4
0042c543: MOV byte ptr [ESP + 0x11],DL
...
0042c59e: MOV AL,byte ptr [ESP + 0x11]
0042c5a2: TEST AL,AL
0042c5a4: JZ 0x0042c5ae
0042c5a6: FLD double ptr [0x007e3818]      ; 0.001
0042c5ae: FLD double ptr [0x007e2800]      ; 0.0
```

The branch is literal: zero flag gets `0.0`, nonzero flag gets `0.001`.

Confidence: **HIGH**. Raw assembly and decompile agree.  
Active in YR: **Yes**.

## Finding 2 — `AddBridgeZoneEdges` Writes Zero Flags For Bridge Repair Edges

`MapClass__AddBridgeZoneEdges @ 0x005851B0` appends six directed edges per hierarchy level, across three levels. For each final edge insert it writes `edge+0 = neighbor_zone_id` and `edge+4 = local flag dword`.

First insert:

```asm
0058535a: MOV EDI,dword ptr [ESP + 0x34]          ; local flag dword
0058535e: MOV dword ptr [EDX + EAX*0x8],ECX       ; edge+0 neighbor
00585361: MOV dword ptr [EDX + EAX*0x8 + 0x4],EDI ; edge+4 flag
```

The locals used for the six flag dwords are masked with `& 0xFFFFFF00` in the decompile, and prior raw-asm inspection found the low byte initialized to `0`. This pass rechecked the final write sites: the direct bridge-add path does not set the low byte to `1`.

Effect: bridge edges inserted by repair/validation are valid adjacency edges, but they are **unflagged** for `Zone_precheck` cost purposes.

Confidence: **HIGH**. Raw write sites and decompile agree.  
Active in YR: **Yes**, through `MapClass__ValidateBridgeZones`.

## Finding 3 — Cold Init Builds Final Edge Flags From Temporary Connection Flags

`FUN_00567110` performs map zone initialization:

1. Frees/reallocates `MapClass+0x68` and `MapClass+0x70`.
2. Calls `MapClass__InitCellAttributes`.
3. Calls `MapClass__ComputeBridgeZones`.
4. Calls `MapClass__UpdateBridgeZonesHelper`.
5. Calls `ZoneMap__BuildZoneLevel` for levels `2`, `1`, then `0`.

`ZoneMap__BuildZoneLevel @ 0x00581F90` is the final writer for many `MapClass+0x90` edges. It scans temporary connection buckets and emits two directed final edges for each temporary connection.

The important copy:

```asm
00582395: MOV ECX,dword ptr [ESI + -0x4]          ; temp packed pair
00582398: MOV DL,byte ptr [ESI]                   ; temp flag low byte at temp+8
005823a4: MOV byte ptr [ESP + 0x80],DL
005823ab: MOV byte ptr [ESP + 0x40],DL
...
005823fb: MOV ECX,dword ptr [ESP + 0x40]
005823ff: MOV dword ptr [EDX + EAX*0x8],EBP       ; final edge+0 neighbor
00582402: MOV dword ptr [EDX + EAX*0x8 + 0x4],ECX ; final edge+4 flag
...
00582454: MOV ECX,dword ptr [ESP + 0x38]
00582458: MOV dword ptr [EDX + EAX*0x8],EDI       ; reverse edge+0
0058245b: MOV dword ptr [EDX + EAX*0x8 + 0x4],ECX ; reverse edge+4 flag
```

So the final `edge+4` low byte is not invented in `Zone_precheck`; it is copied from the temporary connection entry's third dword.

Confidence: **HIGH**. Raw assembly directly shows the copy.  
Active in YR: **Yes**, map init path.

## Finding 4 — `ZoneMap__FloodFillScanline` Is A Nonzero Flag Writer

`ZoneMap__FloodFillScanline @ 0x005824A0` inserts temporary 12-byte connection entries into the level's bucket graph. Most insert paths leave the low byte zero:

```c
local_4 = local_4 & 0xffffff00;
...
temp[0] = packed_pair;
temp[1] = packed_pair;
temp[2] = local_4;          // low byte remains 0
```

Two vertical-neighbor branches can set low byte `1`:

```asm
00582a28: CMP EBX,ECX
00582a2a: JL 0x00582a39
00582a30: MOV byte ptr [ESP + 0x68],0x0
00582a35: CMP EBX,ECX
00582a37: JLE 0x00582a3e
00582a39: MOV byte ptr [ESP + 0x68],0x1
```

and the symmetric branch:

```asm
00582c70: CMP EDI,EAX
00582c72: JL 0x00582c81
00582c78: MOV byte ptr [ESP + 0x68],0x0
00582c7d: CMP EDI,EAX
00582c7f: JLE 0x00582c86
00582c81: MOV byte ptr [ESP + 0x68],0x1
```

The decompile expresses the condition as:

```c
if ((scan_x < range_start) || (range_end < scan_x)) {
    flag_low_byte = 1;
}
```

The surrounding values come from `param_5`: `range_start = *param_5`, `range_end = param_5[2] - 1 + range_start`, with matching vertical bounds in `param_5[1]` / `param_5[3]`. In `BuildZoneLevel`, those values are derived from the current hierarchy block size `1 << (level + 1)`.

Interpretation: this flag marks a connection discovered while the scanline flood-fill is touching an adjacent zone outside the current hierarchy build block/range. It is a hierarchy-construction boundary marker/tiebreak input.

Confidence: **HIGH** for the condition and write; **MEDIUM** for the human-readable name “hierarchy-boundary edge flag”.  
Active in YR: **Yes**, map init path.

## Finding 5 — Bridge Baking Into Temporary Buckets Uses Zero Flags Too

Inside `ZoneMap__BuildZoneLevel`, active bridge records are folded in before final graph emission:

```c
if (*(char *)(bridge_record + 8) != 0) {
    FUN_00582d70(bridge_record, level);
}
```

`FUN_00582D70` computes three bridge/tube connection pairs and inserts them into the temporary 12-byte connection buckets. Its local flag values are initialized to zero:

```c
local_8 = 0;
local_4 = 0;
...
local_c = zone_a << 16 | zone_b;
local_4 = 0;
FUN_0058AF80(&local_c);
...
local_4 = 0;
FUN_00589E20(bucket, &local_c);
```

Raw assembly confirms explicit zero before insert:

```asm
00583020: MOV dword ptr [ESP + 0x30],EAX
00583024: MOV byte ptr [ESP + 0x34],0x0
...
0058314f: MOV dword ptr [ESP + 0x30],EAX
00583160: MOV byte ptr [ESP + 0x3c],0x0
00583165: CALL 0x00589e20
```

So bridge-derived edges included during cold `BuildZoneLevel` are also zero-flagged by the bridge-specific helper. The nonzero writer found in this pass is the generic scanline boundary path, not the bridge helper.

Confidence: **HIGH**. Decompile and raw assembly agree.  
Active in YR: **Yes**, for active bridge records during map init/build.

## Finding 6 — `RemoveBridgeZoneEdges` Ignores The Flag

`MapClass__RemoveBridgeZoneEdges @ 0x00584E50` removes by neighbor zone id using vector find/remove helpers. It does not use the `edge+4` flag as a match key.

Behavioral consequence:

- Direct bridge repair add: appends zero-flag edge.
- Direct bridge destruction remove: removes matching neighbor id, independent of flag.
- If duplicate same-neighbor edges exist, removal removes one matching entry, preserving older duplicate behavior described in lifecycle docs.

Confidence: **HIGH** for flag-ignored matching; the function decompile remains stack-confused, but raw removal pattern and prior lifecycle verification agree.  
Active in YR: **Yes**.

## Finding 7 — `PathfinderClass__InvalidateZoneEdge` Uses Exclusion Lists, Not Edge Flags

`PathfinderClass__InvalidateZoneEdge @ 0x0042CF80` is reached from `PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0`, itself called when main A* fails and the pathfinder retries with updated hierarchy exclusions.

It reads the final graph adjacency:

```c
zone_block = *(int *)(&DAT_0087f878 + level * 0x18) + zone_id * 0x24;
neighbor = *(uint *)(zone_block.edges + i * 8);
```

It writes packed ordered zone pairs into the pathfinder per-level exclusion lists at `pathfinder+0x78/0x84` style fields. It does not mutate final graph `edge+4`.

Confidence: **HIGH**.  
Active in YR: **Yes**, retry/failure path from `AStar_pathfind_search`.

## Cross-Doc Corrections

Replace:

> `edge.flags_low_byte = 1 if bridge-edge`

with:

> `edge.flags_low_byte != 0` causes `Zone_precheck` to add `0.001`. Known nonzero writer: `ZoneMap__FloodFillScanline` during hierarchical zone construction when an adjacent zone connection crosses outside the current build block/range. Bridge-specific add/build helpers write zero for bridge-derived edges.

Specific docs affected:

- `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md` §4.4/§4.5: cost branch is right; “bridge-edge” label is wrong/overbroad.
- `PATHFINDING_ASTAR_GHIDRA_REPORT.md` wording “diagonal penalty” should be treated cautiously if it maps to this same `0x007E3818` site. This pass verifies the branch is controlled by edge flag low byte, not directly by diagonal geometry.
- `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` §13.3 / §16-1: open question resolved.
- `BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md` §7.3 / open item 5: open question resolved.

## Ghidra Labels/Comments Added

Plate comments added:

- `Zone_precheck @ 0x0042C290`
- `ZoneMap__BuildZoneLevel @ 0x00581F90`
- `ZoneMap__FloodFillScanline @ 0x005824A0`
- `MapClass__AddBridgeZoneEdges @ 0x005851B0`

Inline disassembly comments added at key write/read sites:

- `0x00582402` final graph forward `edge+4` write
- `0x0058245B` final graph reverse `edge+4` write
- `0x00582A3F` intended local flag-set area; note: nearest instruction boundary is `0x00582A39`
- `0x00585361` bridge-repair final `edge+4` write

## Rust Relevance

Current Rust pathfinding/zone code does not implement this exact original three-level `MapClass+0x90` hierarchy or `Zone_precheck` edge-flag tiebreak. Therefore this report does not imply an immediate narrow Rust bug.

It matters when implementing original hierarchical pathfinding parity:

- The `0.001` branch must be tied to the stored edge flag byte, not to “bridge” by name.
- Repaired bridge edges inserted through the original direct add path must be zero-flagged.
- Cold hierarchy construction can generate nonzero flags from scanline/block-boundary conditions.

## Open Questions

1. Exact player-visible effect of the `0.001` hierarchy-boundary tiebreak remains unmeasured. The binary behavior is verified; the gameplay impact likely appears only as rare zone-corridor tie ordering.
2. `ZoneMap__BuildZoneLevel` is now understood for the flag copy path, but the full purpose of zone block `+0x20` remains outside this pass.
3. The exact best human-readable name for the flag is still inferential. “Hierarchy-boundary edge flag” matches the discovered writer better than “bridge-edge flag”, but the original engine may have had a different internal name.

## Evidence Sources

Ghidra functions decompiled or rechecked:

- `Zone_precheck @ 0x0042C290`
- `AStar_pathfind_search @ 0x0042C900`
- `FUN_0042D170`
- `PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0`
- `PathfinderClass__InvalidateZoneEdge @ 0x0042CF80`
- `FUN_00567110`
- `ZoneMap__BuildZoneLevel @ 0x00581F90`
- `ZoneMap__FloodFillScanline @ 0x005824A0`
- `FUN_00582D70`
- `MapClass__AddBridgeZoneEdges @ 0x005851B0`
- `MapClass__RemoveBridgeZoneEdges @ 0x00584E50`
- `MapClass__ValidateBridgeZones @ 0x0056DB70`
- `MapClass__InvalidateBridgeZones @ 0x0056DAE0`
- `FUN_0058AF80`
- `FUN_00589E20`

Local docs consulted:

- `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`
- `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md`
- `BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md`
- `BRIDGE_DIRECTION_TABLES_GHIDRA_REPORT.md`
- `PATHFINDERCLASS_GHIDRA_REPORT.md`
- `PATHFINDING_ASTAR_GHIDRA_REPORT.md`

Repo scan:

- `src/sim/pathfinding/zone_map.rs`
- `src/sim/pathfinding/zone_build.rs`
- `src/sim/pathfinding/zone_search.rs`
- `src/sim/world/bridge_orchestrator.rs`
- `src/sim/bridge_state/*`

INI scan:

- `rules.ini` / `rulesmd.ini` contain bridge gameplay keys such as `DestroyableBridges`, `BridgeStrength`, `BridgeDestruction`, `BridgeRepairHut`, `BridgeExplosions`, and `RepairBridgeSound`; no INI key was found that controls this hierarchy edge flag.
