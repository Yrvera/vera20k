# Bridge Zone Lifecycle — Ghidra Research Report

**Phase:** Phase 4 of approved plan `docs/plans/2026-05-13-bridge-pathfinding-locomotion-investigation-plan.md`
**Plan items covered:** #33 (UpdateBridgeZonesHelper), #34 (ZoneFloodFillScanLine), #35 (GetZoneID), #36 (Can_Reach_Zone), #37 (ComputeBridgeZones), #38 (FindBridgeRecord), #39 (InvalidateBridgeZones), #40 (ValidateBridgeZones), #41 (AddBridgeZoneEdges), #42 (RemoveBridgeZoneEdges)
**Companion doc:** [BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md](BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md) (items #43–#46)
**Phase 1–3 dependencies:** [BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md](../03-traversal-pathfinding-entry/BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md), [BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md](BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md) (especially §4 Zone_precheck), [BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md](../03-traversal-pathfinding-entry/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md)
**Date:** 2026-05-13
**Active in YR:** **Yes** — every function in this report is reachable in standard YR skirmish (no `SpecialFlags`, fog, subterranean, or other TS-only gates discovered).

> Every claim cites a Ghidra address + decompilation excerpt or raw asm or `read_memory` byte dump or `get_xrefs_to` / `get_function_callers` result.
> Confidence axes: **C** = content (algorithm verified), **I** = identity (function name verified), **B** = binding (caller path verified).

---

## 0. Phase 4 Checkpoint Summary

The end-of-Phase-4 summary the plan §3 requires:

### (a) Zone-system lifecycle end-to-end

| Phase | Trigger | Function | Effect |
|-------|---------|----------|--------|
| **Build (cold-init)** | Map load (`CCINIClass::Constructor`) | `ComputeBridgeZones` (#37) → `UpdateBridgeZonesHelper` (#33) | Scans every cell, creates BridgeRecord per bridge, flood-fills cluster IDs, builds 256-bucket connection hash (`MapClass+0x14`), populates 13 per-MovementZone ID arrays (`MapClass+0x18..0x48`) |
| **Invalidate (bridge destroyed)** | Damage state machine completes collapse | `InvalidateBridgeZones` (#39) → `RemoveBridgeZoneEdges` (#42) | For all records within 3 cells of the impact coord: sets `BridgeRecord+0x08 = 0` (destroyed) and removes 18 edges (6 per level × 3 levels) from the hierarchical zone graph (`MapClass+0x90`). Caller decides whether to call `UpdateBridgeZonesHelper` for full rebuild. |
| **Revalidate (bridge repaired)** | Repair walker completes | `ValidateBridgeZones` (#40) → `AddBridgeZoneEdges` (#41) → `Can_Reach_Zone` (#36) | For all records within 3 cells: sets `BridgeRecord+0x08 = 1` (intact) and inserts up to 18 edges. Then probes `Can_Reach_Zone` to detect whether the bridge actually created a new connectivity (else returns 0 → no full rebuild needed). |
| **Incremental** | Building placed / cell terrain change | `AssignOrphanedCellZone` (#43) or `MergeAdjacentCellZone` (sibling, 0x56D5A0) | 8-neighbor cluster-inheritance fast path; falls back to `UpdateBridgeZonesHelper` if no consistent neighbor cluster. See companion doc. |

The `+0x08 is_intact` byte is the **only mutable field of BridgeRecord**; the rest (endpoint_a, endpoint_b, +0x0C bridge_kind) is set once at scan time and never changes.

### (b) GetZoneID perpendicular-walk — DEFINITIVELY RESOLVED

**The walk is ALONG the bridge axis, NOT perpendicular.** This refutes the claim in `CELLCLASS_ZONES_SPEED_BRIDGES.md §1.7`.

See §6 below. The formula `(-(uint)(sVar1 != sVar2) & 0xFFFFFFFE) + 4` yields direction **4 (S)** when endpoint X values match (bridge body lies N-S) and direction **2 (E)** when X values differ (bridge body lies E-W). Both directions step *along the bridge body's longitudinal axis*, not across it.

### (c) MapClass+0x90 struct layout — verified field-by-field

See §11 below. Verified offsets via raw asm in `AddBridgeZoneEdges` and `FloodFillReachableZones`. The plan's claim that "stride is 0x24" mixed two levels of nesting: **level header stride is 0x18 (3 × 0x18 = 0x48 bytes total for 3 levels)**; **zone-block stride within a level's zone_blocks array is 0x24**.

### (d) Address drift from plan

None. All Phase 4 plan addresses match the live binary:

| Plan address | Function | Verified |
|---|---|---|
| 0x56C510 | `MapClass::UpdateBridgeZonesHelper` | ✓ |
| 0x56CB90 | `MapClass::ZoneFloodFillScanLine` | ✓ |
| 0x56D100 | `MapClass::Can_Reach_Zone` | ✓ |
| 0x56D230 | `MapClass::GetZoneID` | ✓ |
| 0x56D460 | `MapClass::AssignOrphanedCellZone` | ✓ (covered in companion doc) |
| 0x56D6E0 | `MapClass::ComputeBridgeZones` | ✓ |
| 0x56DA10 | `MapClass::FindBridgeRecord` | ✓ |
| 0x56DAE0 | `MapClass::InvalidateBridgeZones` | ✓ |
| 0x56DB70 | `MapClass::ValidateBridgeZones` | ✓ |
| 0x5851B0 | `MapClass::AddBridgeZoneEdges` | ✓ |
| 0x584E50 | `MapClass::RemoveBridgeZoneEdges` | ✓ |
| 0x583180 | `MapClass::ResolvePathCoord_BridgeAware` | ✓ (companion doc) |

**Plan addresses confirmed live**. Plan's structural claim "3 × 0x24 stride entries" for MapClass+0x90 is *refined* per §11 below (level stride is 0x18; 0x24 is zone-block stride).

---

## 1. Foundation: BridgeRecord layout (Item #37, partial #38)

`MapClass` has a heap-allocated dynamic array of `BridgeRecord` at `MapClass+0x54` (ptr), `MapClass+0x58` (capacity), `MapClass+0x60` (count). Each record is **16 bytes (0x10)**.

### 1.1 Layout — verified field-by-field

| Offset | Type | Field | Evidence |
|--------|------|-------|----------|
| `+0x00` | `CellStruct` (X:Y packed, X low, Y high u16) | endpoint_a | Writer at `ComputeBridgeZones` 0x56D9AE: `*puVar9 = uVar2;` where uVar2 = `*(undefined4 *)&this->MapCoord_X`. Readers in GetZoneID, Add/RemoveBridgeZoneEdges. |
| `+0x04` | `CellStruct` | endpoint_b | Writer at 0x56D9B6: `puVar9[1] = uVar1;` where uVar1 was loaded from the far cell's MapCoord_X via `Pathfinding_update_continued(iVar7-4 & 7)`. |
| `+0x08` | u8 | **is_intact** (1=intact, 0=destroyed) | Writer in `ValidateBridgeZones` (0x56DB97 sets to 1) and `InvalidateBridgeZones` (0x56DB29 sets to 0). Reader at `GetZoneID` 0x56D2A5: `MOV AL, byte ptr [EBX + 0x8]; TEST AL, AL` — non-zero gates the bridge-record-found short-circuit. |
| `+0x09..+0x0B` | u8[3] | Padding/unused | Writer at 0x56D9BA: `puVar9[2] = uStack_8` — high 3 bytes of uStack_8 are uninitialised stack bytes; only the low byte (uStack_31) is meaningful. |
| `+0x0C` | i32 | **bridge_kind** (0 = high bridge, 1 = low bridge/tube) | Writer high path: `puVar9[3] = 0;` at 0x56D9BF. Writer low path: `puVar9[3] = 1;` at 0x56D7C8 (low-bridge tube branch). |

### 1.2 The +0x08 is_intact semantic — verified at write sites

In `ComputeBridgeZones`, the writer initializes `+0x08 = uStack_31`. `uStack_31` starts at **1** at the top of the high-bridge path (`uStack_31 = 1`). It is cleared to **0** only if the perpendicular walk along the bridge encounters a non-bridge, non-flag-0x100 cell (`if ((this_00->Flags & 0x100U) == 0) { uStack_31 = 0; }` at 0x56D958). So at map-load time, `is_intact = 1` for cleanly intact bridges; `is_intact = 0` if the bridge body is structurally broken at load (rare for unmodified maps).

`InvalidateBridgeZones` and `ValidateBridgeZones` toggle this byte at runtime. **No other writer exists** (verified: `get_xrefs_to 0x56DB29 / 0x56DB97` and surrounding writes are local to these two functions).

Confidence: C=HIGH (writes confirmed in raw decomp from 3 distinct functions), I=HIGH (BridgeRecord layout cross-referenced in 5+ readers), B=HIGH (callers traced).

---

## 2. ComputeBridgeZones (#37) @ 0x56D6E0 — initial scan

`param_1` type: **`int`** (direct byte offsets). `__fastcall(int param_1)`.

### 2.1 Algorithm

1. Initialise iterator state: `MapClass+0x10C..+0x118` (cell iterator span: stores +0xF4 mapOriginX, +0xF4-1, and the linear-end-of-map computed as `mapOriginX * 0x800 + 4 + +0x13C`).
2. Walk cells via `MapClass::CellIterator_Next` (does not appear to use a per-call `this`; the iterator state is in `MapClass+0x10C..`).
3. For each cell:
   - **If cell is a bridge tile** (`IsBridge` or `IsWoodBridge`): determine "high bridge" direction-table base (`DAT_00AA0E28` for IsBridge, `DAT_00ABAD1C` for IsWoodBridge), then look up the per-tile-orientation walk-direction at `DAT_0082A734[tile_offset]` and the per-tile expected-height at `DAT_0082A774[tile_offset]`. Skip if cell height doesn't match expected. Walk via `Pathfinding_update_continued(direction)` until a "matching" cell (height per `DAT_0082A7B4`) is found, accumulating non-bridge cells into `uStack_31 = 0` if any are encountered. After the walk, derive endpoint_b's coordinate via one opposite-direction step (`Pathfinding_update_continued(iVar7 - 4U & 7)`), reading `+0x24` (MapCoord) of the returned cell.
   - **Else if cell is a low-bridge cell** (`CellClass::IsLowBridgeCell` = `TubeIndex >= 0 && LandType == 10`): step direction 2 (E), then 6 (W), then 4 (S), then 0 (N), looking for the perpendicular low-bridge cell to identify the tube line. Then use `CellClass::GetTubeAtCell` and `FUN_0042B1C0` to look up the two tube endpoint coords. Use the smaller-index endpoint as endpoint_b.
4. Push a 16-byte record:
   - +0x00 = current cell's MapCoord (endpoint_a)
   - +0x04 = far endpoint MapCoord (endpoint_b)
   - +0x08 = uStack_31 byte (1 for high path with no breakage; 1 for low path; 0 for high path with structural breaks)
   - +0x0C = 0 (HIGH bridge path) or 1 (LOW bridge path)

### 2.2 Cell-iterator span seed at +0x10C..+0x118

```c
*(undefined4 *)(param_1 + 0x10c) = 1;
*(int *)(param_1 + 0x110) = *(int *)(param_1 + 0xf4);          // map origin X
*(int *)(param_1 + 0x114) = *(int *)(param_1 + 0xf4) + -1;     // origin X - 1
*(int *)(param_1 + 0x118) = *(int *)(param_1 + 0xf4) * 0x800   // origin X * 0x800
                          + 4 + *(int *)(param_1 + 0x13c);    // + 4 + +0x13C
```

These seed `CellIterator_Next`'s state. `0x800 = 2048` is `MAP_WIDTH * 4` (with 512-cell map width, each cell-array entry being 4 bytes → 512*4 = 2048). So `+0x118` is the byte-address END of the linear scan range. Confidence: C=HIGH (decomp), I=MEDIUM (semantic by analogy with other iterator functions), B=HIGH (xref to CellIterator_Next).

### 2.3 Records are never removed — only is_intact toggled

The capacity-grow path is triggered if `MapClass+0x60 < MapClass+0x58` fails (`>=` condition, grow via vtable+0x8 call on the vector at MapClass+0x50). The grow is via the vector vtable. **No release code path is present**. After cold init, records persist for the lifetime of the map; destruction merely flips +0x08 to 0.

This means `FindBridgeRecord` will return matches for **destroyed bridges too** if their records are still in the array (intact byte is irrelevant to FindBridgeRecord's match criteria — it only checks the bridge_kind at +0x0C and the geometric distance). Callers must check is_intact themselves.

### 2.4 Active in YR

**Yes.** Callers via `get_function_callers 0x56D6E0`:
- `MapClass::DestroyBridge_*_MapInit @ 0x57400.., 0x574C20` (called by CCINIClass init for any pre-destroyed bridges in the map file — rare)
- Implicit caller via `Invalidate/ValidateBridgeZones` (these call ComputeBridgeZones as a fallback if FindBridgeRecord can't locate any record — see §5.3 below)
- `MapClass::ToggleBridgePavement` and other indirect callers via UpdateBridgeZonesHelper chain (via xrefs)

Reachable on every map load (via CCINIClass::Constructor); no SpecialFlags gating. Confidence: C=HIGH, I=HIGH, B=HIGH.

---

## 3. FindBridgeRecord (#38) @ 0x56DA10 — high-bridge-only linear scan

`param_1` type: **`int`** (this, MapClass*). `__thiscall(int param_1, short *param_2, int param_3, int param_4)`.

| Param | Type | Meaning |
|-------|------|---------|
| `param_1` | MapClass* | this |
| `param_2` | short* | coord (X at +0, Y at +2) |
| `param_3` | int | tolerance (max Manhattan distance from line) |
| `param_4` | int | start index (linear-scan offset) |

### 3.1 Raw asm verification of the high-bridge gate

```asm
0056da3a: MOV EAX, dword ptr [ECX + 0xc]    ; EAX = record[i] +0x0C (bridge_kind)
0056da3d: TEST EAX, EAX
0056da3f: JNZ 0x0056daa5                    ; skip if bridge_kind != 0 (low bridge / tube)
```

The `JNZ` at `0056da3f` jumps to the next-record advance (0x56DAA5 = `ADD ECX, 0x10`). **Records with +0x0C != 0 are unconditionally skipped.** Since `ComputeBridgeZones` writes +0x0C = 0 for high bridges and +0x0C = 1 for low bridges, `FindBridgeRecord` is **high-bridge-only**.

Stride confirmed: `0056daae: ADD ECX, 0x10` (advance 16 bytes per iteration).

Confidence: C=HIGH (raw asm), I=HIGH, B=HIGH.

### 3.2 Geometric match — vertical vs horizontal

The branch at `0056da58: JNZ 0x0056da80` splits based on whether `endpoint_a.X == endpoint_b.X` (after `MOV EBP, EDX; SUB EBP, EBX; ...JNZ`):

- **Vertical (sVar7 == sVar2, same X — bridge runs N-S)**: probe coord must have Y within `[endpoint_a.Y, endpoint_b.Y]` inclusive AND `abs(probe.X - endpoint.X) <= param_3`.
- **Horizontal (different X — bridge runs E-W)**: probe coord must have X within `[endpoint_a.X, endpoint_b.X]` inclusive AND `abs(probe.Y - endpoint.Y) <= param_3`.

Returns the first matching index (starting from param_4), or `-1`.

### 3.3 What param_3 (tolerance) caller-sites use

| Caller | param_3 (tolerance) | Purpose |
|--------|---------------------|---------|
| `GetZoneID` @ 0x56D28A | **1** | Tight — caller already knows cell is on bridge (flag 0x100); just confirm record exists. |
| `InvalidateBridgeZones` @ 0x56DAEE / 0x56DB68 | **3** | Damage-impact tolerance: search records within 3 cells of impact coord. |
| `ValidateBridgeZones` @ 0x56DB7F / 0x56DBF1 | **3** | Repair-impact tolerance: same as Invalidate. |
| `ResolvePathCoord_BridgeAware` @ 0x583197 | **2** | Path-coord snap tolerance. |

The **distance-3 tolerance** of Invalidate/Validate is what lets a damage hit on a cell *near* (but not exactly on) a bridge cell still invalidate the bridge.

Confidence: C=HIGH (decomp), I=HIGH, B=HIGH (caller addresses extracted from xref list).

---

## 4. UpdateBridgeZonesHelper (#33) @ 0x56C510 — full zone rebuild

`param_1` type: **`int`** (this, MapClass*). `__fastcall(int param_1)`. Body length: ~0xC0E bytes.

The function has **8 logical phases** (the plan's "8-phase decomp" claim is verified). It returns the cluster ID of the LARGEST connected component as a `u32` (sentinel `0xFFFFFFFF` if no clusters formed).

### 4.1 Phase 1 — Clear zone connection graph buckets (MapClass+0x14)

```c
piVar19 = *(int **)(param_1 + 0x14);   // pointer to bucket array
iVar17 = 0;
if (0 < piVar19[2]) {                   // piVar19[+0x08] = bucket count
  iVar20 = 0;
  do {
    (**(code **)(*(int *)(*piVar19 + iVar20) + 0xc))();   // vtable+0x0C call (Clear)
    iVar17 = iVar17 + 1;
    iVar20 = iVar20 + 0x18;             // bucket stride = 0x18 bytes (24)
  } while (iVar17 < piVar19[2]);
}
```

Confirmed bucket count = **256** via the Phase-6 loop end `while (iVar17 < 0x1800);` and `0x1800 / 0x18 = 256`. Each bucket is a 24-byte vector-of-edges descriptor. Pre-clearing wipes them via vtable+0xC (Clear() on each bucket vector). Confidence: C=HIGH (decomp), I=HIGH (256 = 16×16 hash matches §4.3 hash key), B=HIGH.

### 4.2 Phase 2 — Free per-MovementZone zone-ID arrays (MapClass+0x18..+0x48)

```c
piVar19 = (int *)(param_1 + 0x18);
iVar17 = 0xd;                            // 13 iterations
do {
  if (*piVar19 != 0) {
    FUN_007c8b3d(*piVar19);              // free() helper
    *piVar19 = 0;
  }
  piVar19 = piVar19 + 1;
  iVar17 = iVar17 + -1;
} while (iVar17 != 0);
```

**13 MovementZones** (one per `MovementZone` enum value). Confirms the 13×4-byte ptr array at +0x18.

### 4.3 Phase 3 — Clear per-cell cluster IDs (MapClass+0x68)

```c
for (pbVar5 = *(byte **)(param_1 + 0x68); pbVar5 < pbVar1; pbVar5 = pbVar5 + 4) {
  pbVar5[2] = 0;
  pbVar5[3] = 0;
}
```

Iterates every cell zone entry (4 bytes each) and clears bytes [2-3] (the cluster_id short). Bytes [0] (zoneType) and [1] (height) are preserved.

### 4.4 Phase 4 — Flood-fill cluster assignment

Iterates every cell. For each cell with `zoneType != 7` (OoB sentinel) AND `cluster_id == 0` (unassigned), calls `MapClass::ZoneFloodFillScanLine(cell_entry_ptr, cluster_id, &iStack_34)`. Increments `cluster_id` after each successful flood. Tracks the largest cluster via `puVar14` and saves its id to `local_48` (the return value).

The per-cluster zoneType is recorded in a separate dynamic vector at `local_14` (vector base) with capacity at `local_4 = 300` (initial), grow-vtable at `local_18 = &PTR_FUN_007ED580`. First entry pushed is **7** (out-of-bounds sentinel), then one entry per real cluster.

```c
uVar21 = 1;                              // cluster_id counter starts at 1
// push '7' as cluster 0's zoneType (OoB sentinel)
*(undefined4 *)(local_14 + local_8 * 4) = 7;
local_8 = local_8 + 1;
...
while (pbVar5 < pbVar1) {
  local_28 = (ushort *)(uint)*pbVar5;    // cell.zoneType
  if ((local_28 == (ushort *)0x7) || (*(short *)(pbVar5 + 2) != 0)) {
    pbVar5 = pbVar5 + 4;
  } else {
    puVar6 = (ushort *)MapClass__ZoneFloodFillScanLine(pbVar5, uVar21, &iStack_34);
    // ... track largest, push zoneType ...
    uVar21 = uVar21 + 1;
    pbVar5 = pbVar5 + iStack_34 * 4;     // skip past assigned cells
  }
}
*(uint *)(param_1 + 0x4c) = uVar21 & 0xffff;  // store total cluster count at +0x4C
```

`iStack_34` is `ZoneFloodFillScanLine`'s OUT param = "cells advanced from seed". Caller skips that many entries.

### 4.5 Phase 5 — Bridge-edge baking into MapClass+0x14 hash table

Iterates BridgeRecord array BACKWARDS (last to first):

```c
puStack_3c = *(undefined4 **)(param_1 + 0x60);    // record count
iVar17 = ((int)puStack_3c + -1) * 0x10;            // last record byte offset
do {
  psVar2 = (short *)(*(int *)(param_1 + 0x54) + iVar17);
  if (*(char *)(*(int *)(param_1 + 0x54) + 8 + iVar17) != '\0') {     // is_intact
    // Get cluster_id at endpoint_a and endpoint_b via linear cell index
    uVar21 = cluster_id_endpoint_a;
    uVar10 = cluster_id_endpoint_b;
    if (uVar10 != uVar21) {
      // Sort: smaller first
      uVar13 = (uVar10 < uVar21) ? uVar10 : uVar21;
      uVar10 = (uVar10 < uVar21) ? uVar21 : uVar10;
      uVar18 = uVar13 << 0x10 | uVar10;             // packed edge key
      // Hash: low-4-bits of smaller and larger → 8-bit hash
      uVar21 = (uVar13 & 0xf) << 4 | uVar10 & 0xf;
      // bucket = ZoneGraph[hash * 0x18]
      // Linear-search backward for duplicate
      while (...) {
        if (uVar18 == *puVar15) goto next_record;
        puVar15 = puVar15 + 2;                       // edge stride 8 bytes
      }
      // Insert (capacity-check then grow via vtable+0x8 if needed)
      *(uint *)(iVar7 + iVar20 * 8) = uVar18;       // edge.+0 = packed key
      *(uint *)(iVar7 + 4 + iVar20 * 8) = uVar18;   // edge.+4 = packed key (duplicate)
    }
  }
  iVar17 = iVar17 - 0x10;
  puStack_3c = puStack_3c - 1;
} while (puStack_3c != 0);
```

**Key details:**
- **Hash function**: `(smaller_cluster & 0xF) << 4 | larger_cluster & 0xF` — 8 bits = 256 buckets. Confirmed via Phase 1 bucket count.
- **Edge layout**: 8 bytes; both `+0` and `+4` get the same `packed_key = (smaller << 16) | larger`. The duplicate at `+4` is *NOT* a flags field for this graph (unlike the per-zone-block hierarchical graph at MapClass+0x90 where `+4` holds the flag byte). Same address, different schema.
- **Iteration backward**: prevents records being processed in their creation order. May be related to vector-growth-safety, or just stylistic.
- **Skips destroyed records**: only `is_intact != 0` records are baked into the connection graph. Destroyed bridges don't appear as edges.

Confidence: C=HIGH (decomp), I=HIGH (hash matches §4.1 bucket count), B=HIGH.

### 4.6 Phase 6 — Build per-cluster neighbor arrays

First, scan all 256 buckets and count edges per cluster (each edge increments BOTH endpoints' counts):

```c
puVar6 = operator_new(*(int *)(param_1 + 0x4c) << 1);   // alloc short array (count per cluster)
// initialize to 0
iVar17 = 0;
do {
  iVar7 = **(int **)(param_1 + 0x14) + iVar17;
  iVar20 = *(int *)(iVar7 + 0x10);              // bucket.count (+0x10)
  if (0 < iVar20) {
    puVar15 = (uint *)(*(int *)(iVar7 + 4) + 4); // bucket.edges_ptr + 4 (the duplicate slot)
    do {
      uVar21 = *puVar15;                          // packed key
      puVar15 = puVar15 + 2;
      puVar6[uVar21 & 0xffff]++;                  // smaller cluster gets +1 neighbor
      puVar6[uVar21 >> 0x10]++;                   // larger cluster gets +1 neighbor
      iVar20--;
    } while (iVar20 != 0);
  }
  iVar17 = iVar17 + 0x18;
} while (iVar17 < 0x1800);
```

Then allocate per-cluster neighbor arrays:
```c
puVar8 = operator_new(*(int *)(param_1 + 0x4c) << 2);    // array of ptrs (4 bytes each)
for each cluster i:
  puVar8[i] = operator_new(puVar6[i] * 2);                // alloc short[neighbor_count]
```

Finally, reset counts to 0 and re-iterate buckets to populate the neighbor arrays:
```c
for each bucket:
  for each edge:
    uVar10 = edge.smaller; uVar13 = edge.larger;
    neighbors[smaller][count[smaller]++] = larger;
    neighbors[larger][count[larger]++] = smaller;
```

Confidence: C=HIGH, B=HIGH (this is standard CSR-style adjacency construction).

### 4.7 Phase 7 — Per-cluster zoneType byte array

Compacts each cluster's zoneType (originally stored at `local_14 + i*4`, low byte of a 4-byte slot) into a single-byte array `pvVar9`:

```c
pvVar9 = operator_new(*(uint *)(param_1 + 0x4c));
for i = 0 to cluster_count - 1:
  *(byte *)((int)pvVar9 + i) = *(byte *)(local_14 + (i+1-1)*4);
```

Note the indexing `local_14 + -4 + iVar20 * 4` with `iVar20 = i + 1` — produces `local_14 + i * 4`. So pvVar9[i] = zoneType byte of cluster i.

### 4.8 Phase 8 — Per-MovementZone zone-ID BFS assignment

```c
puVar11 = operator_new(*(int *)(param_1 + 0x4c) << 1);   // shared BFS queue (short array)
puStack_40 = (undefined4 *)(param_1 + 0x18);              // ZoneIdArrays base
puStack_3c = &g_PassabilityMatrix;                          // matrix at 0x82A594

do {
  uVar4 = 2;                                                // zone_id starts at 2
  puVar12 = operator_new(*(int *)(param_1 + 0x4c) << 1);   // per-cluster zone_id (short[cluster_count])
  *puStack_40 = puVar12;                                    // store at ZoneIdArrays[mvZone]

  // Pass 1: mark blocked clusters
  for each cluster i:
    puVar12[i] = (puStack_3c[zoneType_of_cluster_i] != 1) ? 1 : 0;
    // value = 1 if passability matrix entry != 1 (BLOCKED)
    // value = 0 if passability matrix entry == 1 (PASSABLE, awaiting BFS)

  // Pass 2: BFS from each unassigned passable cluster
  iStack_2c = 0;
  for each cluster i:
    if (puVar12[i] == 0) {  // unassigned passable
      iVar17 = 1;
      *puVar11 = i;
      puVar12[i] = uVar4;                                   // assign current zone_id
      iStack_24 = puStack_3c[zoneType_of_i];                // passability VALUE of seed
      do {
        iVar17--;
        puVar16 = puVar11 + iVar17;                          // pop from queue
        // Iterate neighbors[*puVar16] backward:
        for each neighbor n of cluster *puVar16:
          if ((puStack_3c[zoneType_of_n] == iStack_24) && (puVar12[n] == 0)) {
            *puVar16 = n;                                   // overwrite queue slot (in-place push)
            iVar17++;
            puVar16++;
            puVar12[n] = uVar4;                              // assign zone_id
          }
      } while (iVar17 != 0);
      uVar4++;
    }

  puStack_3c += 8;          // next row of passability matrix (8 ints per row)
  puStack_40 += 1;          // next ZoneIdArrays slot
  *puVar12 = 0xFFFF;        // OVERWRITE cluster 0 with sentinel
} while ((int)puStack_3c < 0x82a734);
```

**Key details:**

- **PassabilityMatrix layout**: starts at `0x82A594`. Each row is 8 ints (32 bytes). Loop iterates while base < 0x82A734 = 0x82A594 + 0x1A0 = 13 × 32 bytes → **13 movement zones** × 8 LandType columns. ✓ Confirms prior `ZONE_PASSABILITY_VERIFIED.md` table size.
- **Zone-ID encoding**:
  - `0` is unassigned in the working array, but DON'T appear in final since BFS covers all clusters
  - `1` = "blocked" (sentinel for non-passable clusters per this MovementZone)
  - `2+` = actual passable connected-component zone IDs
  - `0xFFFF` = forced post-BFS at cluster index 0 (the OoB sentinel) — overrides whatever the BFS assigned to it
- **Same-passability constraint**: BFS only expands to neighbors with `passabilityMatrix[zoneType] == iStack_24` (the seed's passability). Since seeds are always passable (puVar12==0 means passable), this is effectively a "passable-to-passable" expansion. Non-passable clusters never become seeds.
- **In-place queue pop-then-push**: `*puVar16 = n; iVar17++; puVar16++;` reuses the popped slot. Classic compact-frontier BFS.

**The 0xFFFF cluster-0 override matters.** Cluster 0 is the OoB sentinel; it's never visited by flood-fill (zoneType 7 cells are skipped in Phase 4). But Phase 8's blocked-pass would still write `1` to cluster 0 (since matrix[7] is typically != 1). The explicit `*puVar12 = 0xFFFF` overwrites that. **Any code reading zone_id 0xFFFF should interpret as "OoB".**

### 4.9 Phase 9 — Cleanup

Frees the temporary allocations (puVar8 entries, pvVar9, puVar8, puVar6, puVar11) and the cluster-zoneType vector at local_14. The destination ZoneIdArrays at MapClass+0x18..+0x48 are kept.

### 4.10 Caller inventory (item #33 — wide caller list)

Via `get_function_callers 0x56C510`:

| Caller | When |
|--------|------|
| `CCINIClass::Constructor @ 0x599650` | Map load (cold init) |
| `MapClass::CollapseBridge_{EW,NS}_{High,Low}` (4 functions @ 0x575220..0x575BA0) | Bridge collapse completion |
| `MapClass::DestroyBridgeWalker_*` (4 functions) | Bridge destruction walker termination |
| `MapClass::DestroyBridge_*_MapInit` (2 functions) | Pre-destroyed bridges in scenario |
| `MapClass::RepairBridgeWalker_*` (4 functions) | Bridge repair walker termination |
| `ProcessBridgeDamageStateMachine_{High,Low}` (2 functions) | Damage state transition |
| `ProcessBridgeDestruction_{High,Low}` (2 functions) | Destruction confirm |
| `MapClass::AssignOrphanedCellZone @ 0x56D460` | Building placement orphan fallback (companion doc) |
| `MapClass::MergeAdjacentCellZone @ 0x56D5A0` | Cell-zone merge fallback (companion doc) |
| `FUN_00567110`, `FUN_00568E40`, `FUN_00569760`, `FUN_00581140`, `FUN_00594B50`, `FUN_00684C30`, `FUN_006E21E0` | Unlabeled callers — likely overlay placement / terrain edit / scenario events |

All callers are reachable in standard YR. No SpecialFlags gating found at any caller site.

---

## 5. Damage / repair handlers (Items #39, #40)

### 5.1 InvalidateBridgeZones (#39) @ 0x56DAE0

`param_1` type: **`int`** (this, MapClass*). `__thiscall(int param_1, undefined4 param_2)`. Returns u8.

```c
uVar1 = param_2;                                           // save coord
iVar2 = MapClass__FindBridgeRecord(param_2, 3, 0);         // search with tolerance 3 from index 0
if (iVar2 == -1) {
  MapClass__ComputeBridgeZones();                          // RE-SCAN — rebuild record list from scratch
  iVar2 = MapClass__FindBridgeRecord(param_2, 3, 0);
  if (iVar2 == -1) {
    return 0;                                              // bridge not found even after rescan
  }
}
param_2._0_1_ = 0;                                          // return value = 0 initially
for (; iVar2 != -1; iVar2 = MapClass__FindBridgeRecord(uVar1, 3, iVar2 + 1)) {
  iVar3 = *(int *)(param_1 + 0x54) + iVar2 * 0x10;
  if (*(char *)(iVar3 + 8) != '\0') {                       // currently is_intact
    MapClass__RemoveBridgeZoneEdges(iVar3);                 // unhook from hierarchical graph
    param_2._0_1_ = 1;                                      // set return flag
    *(undefined1 *)(*(int *)(param_1 + 0x54) + 8 + iVar2 * 0x10) = 0;  // flip to destroyed
  }
}
return (undefined1)param_2;                                 // 1 if anything was invalidated, 0 otherwise
```

**Critical detail:** the fallback `ComputeBridgeZones()` on miss is a **full record-list rebuild**. This can occur if a bridge was added by an editor edit or never properly scanned. This is the only known recovery path; an absent record would otherwise block invalidation forever.

**Tolerance 3** means: the impact coord must be within Manhattan-distance 3 cells of the bridge axis. So a hit one cell off the bridge can still invalidate it.

**Loop semantics**: continues searching from `iVar2 + 1` so multiple records at the same coord (overlapping bridges, rare) all get processed.

Return semantic: **1 = caller must invoke `UpdateBridgeZonesHelper` for full zone recompute**. 0 = bridge already destroyed (no change made).

Callers (via `get_function_callers 0x56DAE0`):
- `ProcessBridgeDamageStateMachine_High @ 0x576BA0`
- `ProcessBridgeDamageStateMachine_Low @ 0x571490`

Confidence: C=HIGH, I=HIGH, B=HIGH.

### 5.2 ValidateBridgeZones (#40) @ 0x56DB70

Same skeleton as InvalidateBridgeZones, with three differences:

1. The is_intact check is **inverted**: `if (*(char *)(...8 + iVar4) == '\0')` (currently destroyed → fix it).
2. Writes `is_intact = 1` before calling `AddBridgeZoneEdges`.
3. After `AddBridgeZoneEdges`, calls `Can_Reach_Zone(endpoint_a, endpoint_b, 0, 0, 0, 0)` to verify the bridge actually creates new connectivity.

```c
if (cVar2 == '\0') {                          // Can_Reach_Zone returned 0 (cells STILL not reachable)
  param_2._0_1_ = 1;                          // return flag = 1
}
```

The reachability test uses MovementZone 0 (param_3=0) and all-false flags. The check happens BEFORE the next zone rebuild, so `Can_Reach_Zone` reads the OLD cluster IDs (the cells still hold the cluster IDs assigned when the bridge was destroyed). If the cluster IDs at endpoints differ (i.e., they're in separate components from when the bridge was broken), Can_Reach_Zone returns 0 → "this validation created new connectivity → needs full rebuild".

If `Can_Reach_Zone` returns 1, the endpoints were already in the same cluster → bridge was redundant → no rebuild needed.

Callers (via `get_function_callers 0x56DB70`):
- `ProcessBridgeDestruction_High @ 0x573540`
- `ProcessBridgeDestruction_Low @ 0x570050`
- `FUN_00568E40`, `FUN_00569760` (unlabeled — likely repair-related)

Confidence: C=HIGH, I=HIGH, B=HIGH.

### 5.3 ComputeBridgeZones-fallback design implication

Both Invalidate and Validate fall back to `ComputeBridgeZones` on miss. This means **the bridge record list can be reconstructed at any time** — it's an O(N_cells) operation but it's bounded and correct. This is the engine's robustness mechanism against missing/stale records.

---

## 6. GetZoneID (#35) @ 0x56D230 — DEFINITIVE perpendicular-walk resolution

`param_1` type: **`int`** (this, MapClass*). `__thiscall(int param_1, short *param_2, int param_3, char param_4)`.

| Param | Type | Meaning |
|-------|------|---------|
| `param_1` | MapClass* | this |
| `param_2` | short* | coord (X at +0, Y at +2) |
| `param_3` | int | MovementZone (0..12) — selects ZoneIdArrays[param_3] |
| `param_4` | char | checkBridge flag (0 = skip bridge logic, just look up cluster-id directly) |

### 6.1 Refutation of CELLCLASS_ZONES_SPEED_BRIDGES.md §1.7

The prior doc claims:

> // Walk in the direction perpendicular to bridge orientation
> // until we find a non-bridge cell

**This is WRONG.** The walk is **ALONG the bridge axis**, not perpendicular. Resolved definitively from raw assembly below.

### 6.2 Raw asm of the loop (0x56D2E5..0x56D318)

```asm
0056d2e5: MOV SI, word ptr [EBX]              ; SI = *psVar7 = endpoint_a.X (low 16 of record+0x00)
0056d2e8: LEA EBP, [EBX + 0x4]                ; EBP = &record.+0x04 (endpoint_b)
0056d2eb: MOV EAX, dword ptr [EDI + 0x140]    ; EAX = current_cell->Flags
0056d2f1: SUB SI, word ptr [EBP]              ; SI -= endpoint_b.X
                                              ; SI = endpoint_a.X - endpoint_b.X
0056d2f5: NEG SI                              ; SI = -(endpoint_a.X - endpoint_b.X)
                                              ; CF=0 iff SI==0 (after NEG)
0056d2f8: SBB ESI, ESI                        ; ESI = 0 if equal (no carry), 0xFFFFFFFF if differ
0056d2fa: AND ESI, 0xfffffffe                 ; ESI = 0 or 0xFFFFFFFE
0056d2fd: ADD ESI, 0x4                        ; ESI = 4 (equal X) or 2 (different X)
0056d300: TEST AH, 0x1                        ; bit 0x100 of Flags
0056d303: JZ 0x0056d31a                       ; exit if not bridge-structural
0056d305: PUSH ESI                            ; direction
0056d306: MOV ECX, EDI                        ; this = current_cell
0056d308: CALL 0x00481810                     ; Pathfinding_update_continued (CellClass::Get_Neighbor)
0056d30d: MOV EDI, EAX                        ; EDI = neighbor cell
0056d30f: MOV EAX, [EDI + 0x140]              ; EAX = neighbor.Flags
0056d315: TEST AH, 0x1                        ; bit 0x100
0056d318: JNZ 0x0056d305                      ; loop while still on bridge
```

### 6.3 Direction-code semantics (cross-referenced with offset table)

Verified via `read_memory 0x7E3774` (32 bytes, 8 directions × 4 bytes = cell-array linear offsets):

| Direction code | Offset (signed) | Meaning |
|----------------|-----------------|---------|
| 0 | -512 | N (-1 row) |
| 1 | -511 | NE |
| 2 | **+1** | **E (+1 col)** |
| 3 | +513 | SE |
| 4 | **+512** | **S (+1 row)** |
| 5 | +511 | SW |
| 6 | -1 | W |
| 7 | -513 | NW |

Hex dump: `00 fe ff ff | 01 fe ff ff | 01 00 00 00 | 01 02 00 00 | 00 02 00 00 | ff 01 00 00 | ff ff ff ff | ff fd ff ff` (little-endian signed i32, MAP_WIDTH = 512).

So:
- `direction 2 = E (+X)`
- `direction 4 = S (+Y)`

### 6.4 What the formula actually means

| Endpoint comparison | Bridge body axis | Walk direction |
|---------------------|------------------|----------------|
| `endpoint_a.X == endpoint_b.X` | Same X, varying Y → **bridge runs N-S** (body cells aligned along Y) | **4 (S, +Y)** = ALONG the body axis |
| `endpoint_a.X != endpoint_b.X` | Different X (same Y) → **bridge runs E-W** (body cells aligned along X) | **2 (E, +X)** = ALONG the body axis |

**Both cases walk along the bridge's longitudinal body axis, NOT perpendicular to it.** The walker proceeds along the bridge body until it leaves the structural (`Flags & 0x100`) region — typically arriving past the bridgehead near endpoint_b.

### 6.5 Post-walk endpoint flip semantic

```c
bVar4 = CellClass__IsBridge(this);
if (((bVar4) || (bVar4 = CellClass__IsWoodBridge(this), bVar4)) && (this->LandType != 3)) {
  psVar7 = psVar7 + 2;          // psVar7 is short* → +4 bytes → point at endpoint_b coord
}
```

If the exit cell is a bridge tile (concrete or wood) AND LandType != 3 (Rock) → use endpoint_b coord for the final zone lookup. Otherwise → keep endpoint_a.

**Net effect for a destroyed bridge:** the unit's "logical" zone becomes the zone at one of the two endpoints, chosen by walking along the bridge body in a specific direction. NOT a perpendicular walk off the side.

### 6.6 Final lookup formula

After resolving the coord:
```c
iVar5 = (mapWidth + 1 + mapOriginX) * coord.Y + coord.X;
iVar5 = clamp(iVar5, 0, totalCells - 1);
return *(ushort *)(zoneIdArrays[movementZone] + cellZoneData[iVar5].cluster_id * 2);
```

So GetZoneID is **2-step**: cell → cluster_id → zone_id-for-this-movement-zone.

### 6.7 OoB-cell fallback (DAT_00ABDC50)

If the linear cell index is OoB or the cell pointer is NULL:
```c
DAT_00abdc74 = *(undefined4 *)param_2;        // stash the original coord
puVar6 = &DAT_00abdc50;                        // synthetic "fake cell" buffer
```

`DAT_00ABDC50` is a 36-byte scratch CellClass-like buffer at file scope. Code writes the coord at `DAT_00ABDC50 + 0x24 = 0x00ABDC74`. This is the "phantom cell" pattern used when probing coords outside the cell array. The phantom cell has all flags clear, so `Flags & 0x100 = 0` → bridge logic skipped.

Confidence: C=HIGH (raw asm), I=HIGH, B=HIGH (caller list earlier).

---

## 7. Can_Reach_Zone (#36) @ 0x56D100 — playfield shortcuts + zone-ID equality

`param_1` is **NOT** `this` here — Ghidra signature shows `__cdecl` with 6 params (no implicit MapClass*). However the function reads from `DAT_0087F8DC` and `DAT_0087F8E0`, which are at addresses `MapClass_singleton+0xF4` and `MapClass_singleton+0xF8` (the singleton is at `0x0087F7E8`, verified by `MOV ECX, 0x87f7e8` in `Pathfinding_update_continued`). So it accesses MapClass via fixed global addresses — i.e., it's a `__cdecl` function semantically, with the MapClass* baked in.

### 7.1 Parameters

| Param | Type | Meaning |
|-------|------|---------|
| `param_1` | short* | coord A (X:Y) |
| `param_2` | short* | coord B |
| `param_3` | int | MovementZone (or -1 to bypass) |
| `param_4` | u32 | checkBridge for A |
| `param_5` | u32 | checkBridge for B |
| `param_6` | char | "allow B-OoB-while-A-in" flag (see §7.4) |

### 7.2 Early-out: MovementZone == -1

```c
if (param_3 == -1) {
  return true;
}
```

Bypasses zone check entirely for "ignore zones" callers.

### 7.3 Diamond-playfield OoB test (per cell)

Hex playfield boundary check, applied independently to each coord:

```c
bVar1 = (DAT_0087f8dc < y + x) &&
        (x - y < DAT_0087f8dc) &&
        (y - x < DAT_0087f8dc) &&
        (y + x <= DAT_0087f8dc + DAT_0087f8e0 * 2);
```

This is the **isometric diamond test**: a cell is in-playfield iff its (x, y) lies inside the rotated diamond defined by `MapClass+0xF4 (origin X)` and `MapClass+0xF8 (width)`. This corresponds to standard RA2 hex-cell-coordinate-in-playfield testing.

### 7.4 OoB short-circuits (with subtle asymmetry)

```c
cVar2 = MapClass__Is_Cell_In_Playfield(param_1, 1);     // is A in tactical playfield?
... compute bVar1 = A in diamond ...
if ((cVar2 == '\0') && (bVar1)) {
  return true;                                            // A outside tactical but inside diamond → reachable
}
cVar3 = MapClass__Is_Cell_In_Playfield(param_2, 1);     // is B in tactical playfield?
... compute bVar1 = B in diamond ...
if ((((param_6 != '\0') && (cVar2 != '\0')) && (cVar3 == '\0')) && (bVar1)) {
  return true;
}
```

**Two distinct OoB shortcuts:**

1. **A outside tactical AND A inside diamond** → return true unconditionally. The semantic is: "we asked about reaching FROM somewhere OoB but inside the diamond — assume always reachable" (this is the "off-map but valid" edge case for spawned units, AI scouts, etc.).
2. **param_6=1 AND A inside tactical AND B outside tactical AND B inside diamond** → return true. So `param_6` is "allow reaching INTO a diamond-OoB target cell". Used by callers that issue movement commands targeting fringe cells.

The asymmetry between A and B is intentional. If the user explicitly opts in (`param_6 != 0`), Can_Reach_Zone permits routing into B-as-fringe.

### 7.5 Zone-ID equality (the final test)

```c
iVar4 = MapClass__GetZoneID(param_2, param_3, param_5);
iVar5 = MapClass__GetZoneID(param_1, param_3, param_4);
return iVar5 == iVar4;
```

NOTE: Ghidra's decomp shows 3-arg calls because Can_Reach_Zone is a non-thiscall function and Ghidra doesn't surface the implicit `this`. GetZoneID is `__thiscall`, so the actual calling convention is `this::GetZoneID(coord, mvZone, checkBridge)` with `this = MapClass_singleton`. The B coord goes first (slightly odd ordering, no functional implication).

### 7.6 No subzone walk — just equality

There's no graph traversal here. Two cells are reachable iff their zone IDs (per the given MovementZone) match. **This is the O(1) reachability cache** that pathfinding queries before launching A*. Reachability is precomputed by `UpdateBridgeZonesHelper`'s flood-fill — the zone-ID encodes the connected component.

### 7.7 Active in YR

Callers via `get_function_callers 0x56D100`: very wide — every "can I path to X?" query in the engine ultimately funnels here. No SpecialFlags gate found.

Confidence: C=HIGH, I=HIGH, B=HIGH.

---

## 8. AddBridgeZoneEdges (#41) @ 0x5851B0 — full body, verified up-to-6 edges per level

`param_1` type: **`int`** (this, MapClass*). `__thiscall(int param_1, undefined4 *param_2)`.

| Param | Type | Meaning |
|-------|------|---------|
| `param_1` | MapClass* | this |
| `param_2` | undefined4* | BridgeRecord* (points at +0x00 of record) |

### 8.1 Outer structure — 3 hierarchical levels

Verified via raw asm (`0x00585570: ADD EBP, 0x2; ... CMP EBP, 0x6; JL 0x005852d9`):

```
EBP = 0, 2, 4    (3 iterations)
local_50 += 0x18 per iteration (next level header)
```

So the loop processes **3 hierarchical levels**. The level stride is `0x18` (24 bytes) for the level header at MapClass+0x90, NOT `0x24` as the plan stated (plan §3 Phase 4 row #45 had the stride confused with per-zone-block stride).

### 8.2 Bridgehead-approach coord computation

Before the loop, the function reads the cell at endpoint_a:
```c
this = cell_at(endpoint_a);
if (this is a bridge tile) {
  uVar3 = *(uint *)(&DAT_0082a944 + (this->IsoTileTypeIndex - bridge_base) * 4);
  // Compute 4 derived coords by stepping endpoint_a or endpoint_b in directions uVar3 and uVar3-4&7
  local_3c = endpoint_a + offset(uVar3 & 7);
  local_34 = endpoint_a + offset(uVar3-4 & 7);
  local_38 = endpoint_b + offset(uVar3 & 7);
  local_44 = endpoint_b + offset(uVar3-4 & 7);
}
```

(Note: Ghidra's decomp shows all 4 `MapCoord_Add(&param_2, ...)` calls with `param_2` — but `param_2` is in fact rewritten between calls. Raw asm at 0x585256/0x585271/0x585285/0x585299 confirms two distinct source pointers — `[ESP + 0x14]` (endpoint_a) for the first pair and `[ESP + 0x14]` again for the second pair, but with the result going into local_38 and local_44 respectively. The decomp variable propagation is misleading; the operation produces 4 DISTINCT coords.)

`DAT_0082A944` is a per-tile-type direction table (the bridgehead-step-direction lookup). `uVar3 & 7` and `(uVar3 - 4) & 7` are opposite directions.

If endpoint_a's cell is NOT a bridge tile, the 4 derived coords default to `DAT_00ABD480` (a sentinel "invalid coord" marker).

### 8.3 Per-level loop body — 6 edge insertions

Raw asm verified flag-byte init at outer-loop start:
```asm
005852b5: MOV byte ptr [ESP + 0x34], 0x0    ; flag byte for edge 1
005852ba: MOV byte ptr [ESP + 0x3c], 0x0    ; edge 2
005852bf: MOV byte ptr [ESP + 0x44], 0x0    ; edge 3
005852c4: MOV byte ptr [ESP + 0x4c], 0x0    ; edge 4
005852c9: MOV byte ptr [ESP + 0x54], 0x0    ; edge 5
005852ce: MOV byte ptr [ESP + 0x5c], 0x0    ; edge 6
```

**6 flag bytes, all 0**. The decomp's `local_2c & 0xffffff00` etc. masks are the C-level representation — actual asm initializes ONLY the low byte to 0 (uppers are stack residue, unread by the writer).

Per iteration, **3 edge PAIRS** are inserted (each pair = 2 directed edges: A→B and B→A):

1. **endpoint_a ↔ endpoint_b** (zone(local_4c), zone(local_48))
2. **bridgehead-extension pair 1** (zone(local_3c), zone(local_38))
3. **bridgehead-extension pair 2** (zone(local_34), zone(local_44))

= **6 directed edges per level × 3 levels = 18 total directed edges**.

The plan's "up to 6 edges" likely refers to per-level directed edges. Total inserted in the hierarchical zone graph is 18 (when all bridgehead-approach checks succeed) or fewer (if some derived coords were `DAT_00ABD480` and resolve to identical zones).

### 8.4 Edge entry layout at MapClass+0x90's per-zone edge array

Raw asm at 0x58535E/0x585361 (first insert):
```asm
0058535e: MOV [EDX + EAX*8], ECX            ; edge.+0 = neighbor zone_id (int from MOVSX'd short)
00585361: MOV [EDX + EAX*8 + 4], EDI        ; edge.+4 = flag dword (loaded from [ESP + 0x34], low byte = 0)
```

**Edge layout: 8 bytes — neighbor_zone_id (u32, but only low 16 are meaningful) at +0, flags (u32) at +4. AddBridgeZoneEdges always writes flags = 0 (low byte; uppers are stack residue).**

### 8.5 Zone block (within a level) layout — verified

Each zone block in the per-level zones-array is 0x24 bytes. Layout extracted from AddBridgeZoneEdges + FloodFillReachableZones + Zone_precheck cross-references:

| Offset | Type | Field | Evidence |
|--------|------|-------|----------|
| `+0x00` | code** | vtable | `**(code **)(*piVar1 + 8)` is vtable+0x8 (Grow); `**(code **)(*piVar1 + 0x10)` is vtable+0x10 (Find). |
| `+0x04` | int* | edges_ptr | `piVar1[1]` = `iVar4 = piVar1[1]; *(uint *)(iVar4 + ...) = ...` |
| `+0x08` | int | **capacity** | `iVar9 = *(int *)(*local_50 + 8 + iVar10 * 0x24);` — compared as upper bound against count. |
| `+0x0D` | char | grow flag byte | `*(char *)((int)piVar1 + 0xd) != '\0'` gates the grow fallback. |
| `+0x10` | int | **count** | `piVar1[4]++` post-insert; `piVar1[4]--` in `FUN_00589290` (remove helper). |
| `+0x14` | int | growth quantum | `piVar1[5]` — added to capacity on grow (`piVar1[5] + iVar7`). |
| `+0x18` | u16 | representative cell zone ID (level-N parent) | Per `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md §4.4`. |
| `+0x1A` | u16 | (unknown / pad) | Reserved. |
| `+0x1C` | i32 | LandType-or-class index | Per `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md §4.4`. |
| `+0x20` | int | (unknown) | Final 4 bytes of the 0x24-byte block. |

**Refinement of plan §3 item #45**: the plan stated "+0x08 count, +0x10 cap" — this is **INVERTED**. From AddBridgeZoneEdges, the comparison `count(+0x10) < cap(+0x08)` proves +0x08 is capacity and +0x10 is count. The prior `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md §4.4` correctly stated `+0x10 = count`; the plan's data was inconsistent.

### 8.6 Per-zone insert algorithm

```c
piVar1 = (int *)(zone_blocks_array + zone_id * 0x24);
old_count = piVar1[+0x10];
capacity = piVar1[+0x08];
if (old_count < capacity || /* grow path conditions */) {
  piVar1[+0x10] = old_count + 1;
  edges_ptr = piVar1[+0x04];
  *(int *)(edges_ptr + old_count * 8) = neighbor_zone_id;
  *(int *)(edges_ptr + 4 + old_count * 8) = flag_dword;   // low byte = 0
}
```

Grow path: `vtable+0x8(piVar1, growth_quantum + old_count, 0)`. Returns non-zero on success → re-fetch piVar1 fields and insert.

### 8.7 No duplicate-edge detection

Unlike `UpdateBridgeZonesHelper` Phase 5 (which has a backward-linear-search for duplicates before inserting into the 256-bucket connection graph), `AddBridgeZoneEdges` **does NOT check for duplicates**. Every call appends, regardless of whether the edge already exists. This means **double-calling AddBridgeZoneEdges on the same record produces duplicate edges**.

The pairing with `RemoveBridgeZoneEdges` (which DOES use a find-then-remove pattern via vtable+0x10) makes this asymmetric: 1× Add inserts, 1× Remove removes the first match. **Two Adds and one Remove leaves a stale duplicate edge.** This is a parity-sensitive detail — Rust ports must replicate this same insert-without-dedup behavior to match gamemd's edge counts exactly. (See §15 below for full discussion.)

Confidence: C=HIGH (raw asm verified), I=HIGH, B=HIGH (callers traced via Validate/Invalidate).

---

## 9. RemoveBridgeZoneEdges (#42) @ 0x584E50 — verified strict inverse

`param_1` type: **`int`** (this, MapClass*). `__thiscall(int param_1, uint *param_2)`.

### 9.1 Decomp had heavy stack-register confusion

Ghidra's decompilation shows several `unaff_*` and `piStack_*` placeholders (e.g., `unaff_retaddr`, `piStack_8`, `piStack_c`, `piStack_10`, `unaff_EBX`, `unaff_ESI`) — these are stack/register values that Ghidra couldn't trace cleanly. From raw asm and structural analogy with AddBridgeZoneEdges (the loop body is byte-identical in pattern), the structure is:

- Compute same 4 bridgehead-extension coords (local_40, local_38, local_3c, local_34) using the SAME `DAT_0082A944` direction table — verified mirror.
- 3-level loop (iVar4 = 0..2, param_2 += 0x18 ints per iteration — wait, actually `param_2 = param_2 + 6` in decomp = 6 uints = 24 bytes = 0x18 — confirmed).
- Per level, **6 removal operations** (3 pairs × 2 directions):
  - Remove edge `zone(local_4c)` → `zone(local_48)` and inverse
  - Remove edge `zone(local_3c)` → `zone(local_38)` and inverse
  - Remove edge `zone(local_34)` → `zone(local_44)` and inverse

### 9.2 Removal algorithm (per edge)

```c
piVar1 = zone_blocks_array + source_zone * 0x24;
edge_index = (**(code **)(*piVar1 + 0x10))(&target_zone_buffer);   // vtable+0x10 Find
if (edge_index != -1) {
  FUN_00589290(piVar1, edge_index);                                 // remove at index
}
```

`FUN_00589290` (verified at 0x589290) is:
```c
if (count > edge_index) {
  count--;
  for (i = edge_index; i < count; i++) {
    edges[i] = edges[i+1];      // shift down
  }
  return 1;
}
return 0;
```

This is the **standard array-shift-down delete-at-index**. Confirms zone-block +0x10 is count, +0x04 is edges_ptr, edge stride is 8 bytes. Mirrors AddBridgeZoneEdges.

### 9.3 Two removal sites inline the shift loop

The 5th and 6th removal sites per level inline the shift-down rather than calling `FUN_00589290`:

```c
iVar6 = (**(code **)(*piVar1 + 0x10))(&uStack_20);     // find
if ((iVar6 != -1) && (iVar6 < piVar1[4]) &&
    (iVar7 = piVar1[4] + -1, piVar1[4] = iVar7, iVar6 < iVar7)) {
  do {
    iVar7 = piVar1[1];
    iVar6 = iVar6 + 1;
    *(undefined4 *)(iVar7 + -8 + iVar6 * 8) = *(undefined4 *)(iVar7 + iVar6 * 8);
    *(undefined4 *)(iVar7 + -4 + iVar6 * 8) = *(undefined4 *)(iVar7 + 4 + iVar6 * 8);
  } while (iVar6 < piVar1[4]);
}
```

The inlined version uses the same algorithm as FUN_00589290. Why two of the six are inlined is unclear — likely compiler inlining decision based on call-site profile / register pressure. **No semantic difference.**

### 9.4 Strict inverse confirmation — independent

The Add and Remove pairs operate on the same 3 coord pairs at the same 3 levels using the same direction table (DAT_0082A944) and the same offset table (g_DirectionOffsets at 0x89F688). Both walk +0x18 strides at the per-level loop and use +0x04 (edges_ptr), +0x10 (count) at the zone-block layer. **Confirmed strict inverse** — independently decompiled and compared, not inherited from a prior doc.

Edge-case asymmetry: Add does not dedup; Remove finds the FIRST match. So if duplicates exist (from double-add), Remove leaves one stale. See §15.

Confidence: C=HIGH, I=HIGH, B=HIGH.

---

## 10. ZoneFloodFillScanLine (#34) @ 0x56CB90 — asymmetric height thresholds CONFIRMED

`param_1` type: **`int`** (this, MapClass*). `__thiscall(int param_1, char *param_2, uint param_3, int *param_4)`.

| Param | Type | Meaning |
|-------|------|---------|
| `param_1` | MapClass* | this |
| `param_2` | char* | pointer into MapClass+0x68 zone data array (4 bytes per cell: zoneType, height, cluster_lo, cluster_hi) |
| `param_3` | uint | cluster ID to assign |
| `param_4` | int* | OUT: number of cells advanced from seed |

### 10.1 Asymmetric height thresholds — VERIFIED via decomp

**Left scan** (towards earlier addresses):
```c
do {
  uVar14 = (int)((byte)pcVar16[1] - uVar8) >> 0x1f;
  if (1 < (int)(((byte)pcVar16[1] - uVar8 ^ uVar14) - uVar14)) break;
  // abs(consecutive_diff) <= 1 — break if > 1
  *(undefined2 *)(pcVar16 + 2) = (undefined2)param_3;
  pcVar17 = pcVar16 + -4;
  uVar8 = (uint)(byte)pcVar16[1];      // update reference to current's height
  pcVar16 = pcVar16 + -4;
} while (*pcVar17 == cVar3);            // continue while next-left has same zoneType
```

**Right scan** (towards later addresses):
```c
while ((cVar7 == cVar3 &&
       (uVar14 = (int)((byte)pcVar17[1] - uVar8) >> 0x1f,
       (int)(((byte)pcVar17[1] - uVar8 ^ uVar14) - uVar14) < 4))) {
  *(undefined2 *)(pcVar17 + 2) = (undefined2)param_3;
  pbVar1 = (byte *)(pcVar17 + 1);
  pcVar17 = pcVar17 + 4;
  cVar7 = *pcVar17;
  uVar8 = (uint)*pbVar1;
}
```

**Verified thresholds**:
- **Left**: `abs(consecutive_height_diff) <= 1` (the `< 2` check)
- **Right**: `abs(consecutive_height_diff) <= 3` (the `< 4` check)

This **asymmetry is REAL and matches the prior CELLCLASS_ZONES_SPEED_BRIDGES.md §1.6 claim**. The right-scan leniency (≤3 vs left-scan's ≤1) is preserved.

**Cleanup-pass detail**: the comparison is **consecutive-pair**, not seed-vs-current. `uVar8` is updated to the CURRENT cell's height after each assignment. So the abs-diff is between adjacent cells in the scanline, not between the scanline tail and the seed. This matters for steep slopes where consecutive small steps could accumulate but each individual step is within threshold.

### 10.2 Boundary edge addition (post-scan, both directions)

When a scan terminates at a different-cluster neighbor, the function may insert an edge into the **MapClass+0x14 connection graph** (NOT the MapClass+0x90 hierarchical graph):

```c
uVar11 = (uint)*(ushort *)(pcVar16 + 2);    // failing cell's cluster_id (presumably already set)
if (((uVar11 != 0) &&
    ((... abs_diff_check ... || bVar19)) &&     // height-diff check OR Impassable seed bypass
    (uVar11 != DAT_00abde8c) &&                  // not equal to "last edge added" memo
    (uVar11 != (param_3 & 0xffff)))) {           // not our own cluster
  // ... compute hash, search bucket for duplicate, insert ...
}
```

**Key details:**

- **`bVar19 = (cVar3 == '\x06')`** — set at function entry if seed's zoneType is **6 (Impassable)**. When true, the height-diff check is BYPASSED for boundary edge additions. This is how impassable-zone connectivity is preserved across height jumps that would otherwise prevent flood-fill from connecting them via the connection graph.
- **`DAT_00ABDE8C`** is a "last edge added to" memo — a global short. It's set after each successful edge insertion to `uVar11` (the just-edged neighbor cluster). The next edge-add candidate is rejected if it equals this memo. This prevents redundant edges to the SAME other cluster from CONSECUTIVE scan terminations. Verified writer pattern: `DAT_00ABDE8C = uVar14;` after each edge-add block.
- **Edge-add hash and storage**: identical to UpdateBridgeZonesHelper Phase 5. Same 256-bucket hash table at MapClass+0x14, same `(smaller << 16) | larger` packed key, same duplicate field at +0x04.

### 10.3 Boundary edge height-diff check

The post-scan edge-add height check is **`abs(consecutive_diff) <= 1`** (`< 2`) — same as left scan, NOT the lenient right-scan threshold. So even if the right scan terminated because of a height diff > 1 but <= 3, the BOUNDARY EDGE is NOT added unless the diff is also <= 1 (or bVar19 is true).

**This is a third distinct threshold** in the same function:
- Left-scan walk: <= 1 (`< 2`)
- Right-scan walk: <= 3 (`< 4`)
- Boundary edge-add: <= 1 (`< 2`)

(Recall similar third-threshold-asymmetry was found in Phase 3 cleanup re Drive::Process_Drive_Track — three distinct thresholds in one function.)

### 10.4 Recursion into rows above/below the span

After both scans, the function recurses for cells in the rows above and below the just-scanned span:

```c
local_1c = ((int)pcVar17 - (int)pcVar16 >> 2) + -1;       // span_width - 1
*param_4 = ((int)pcVar17 - (int)param_2 >> 2) + -1;       // cells advanced from seed (OUT)
iVar5 = mapWidth + 1 + mapOriginX;                          // row stride in cells
iVar4 = iVar5 * 4 + 4;                                      // (rowStride * 4) + 4 = row-byte-stride + 1 cell
iVar9 = iVar5 * 4;                                          // row-byte-stride
pcVar18 = pcVar16 + (4 - iVar4);                            // row ABOVE leftmost
pcVar13 = pcVar17 + iVar5 * -4 + -4;                        // row ABOVE rightmost
pcVar16 = pcVar16 + iVar9;                                  // row BELOW leftmost
pcVar17 = pcVar17 + iVar5 * 4;                              // row BELOW rightmost
```

The recursive part walks the above and below rows. For each cell:
- If unassigned (cluster == 0) AND same zoneType AND `abs(height_diff vs reference cell in same column) < 2` → recurse via `ZoneFloodFillScanLine(neighbor, cluster_id, &local_iStack)`, advancing the iterator by the returned span width.
- Else if assigned to a DIFFERENT cluster AND abs(height_diff) < 2 (or bVar19) → emit a boundary edge as in §10.2.

The recursive **height-diff check is <= 1** (`< 2`), matching left-scan strictness, with the impassable bypass.

**The "reference cell in same column" logic** uses a diagonal-skewing offset for the leftmost and rightmost cells of the below row, accounting for the cell-pointer-array's iso-skewed layout. Specifically:
- For pcVar16 strictly inside the span: `iVar10 = -rowStride * 4 + 4` (above and right)
- For pcVar16 at right edge (= pcVar17 - 4): `iVar10 = -rowStride * 4` (directly above)
- For pcVar16 just past right edge: `iVar10 = -(rowStride * 4 + 4)` (above and left)

This is the **iso-tile diagonal-neighbor offset pattern** — adjacent rows in isometric tile coordinates skew by 1 cell horizontally.

Confidence: C=HIGH (decomp, raw threshold values directly extracted), I=HIGH, B=HIGH (single caller from UpdateBridgeZonesHelper Phase 4).

---

## 11. MapClass+0x90 region — verified struct layout (Item #45)

### 11.1 BSS check

```
read_memory 0x0087F878 (= MapClass_singleton+0x90), len 32 → all zeros
```

(BSS, runtime-initialised. Static dump unavailable; structure derived from access patterns.)

### 11.2 Verified layout — level headers

| Address | Offset from MapClass | Size | Contents |
|---------|----------------------|------|----------|
| 0x87F878 | +0x90 | 0x18 | Level 0 header |
| 0x87F890 | +0xA8 | 0x18 | Level 1 header |
| 0x87F8A8 | +0xC0 | 0x18 | Level 2 header |

**Level stride confirmed 0x18** via:
- AddBridgeZoneEdges raw asm: `local_50 += 6 ints` per level-loop iteration = 0x18 bytes
- RemoveBridgeZoneEdges decomp: `param_2 += 6 uints` per level = 0x18 bytes
- Total range: 3 levels × 0x18 = 0x48 bytes

### 11.3 Level header layout (24 bytes)

| Offset | Type | Field | Evidence |
|--------|------|-------|----------|
| +0x00 | int* | zone_blocks_array_ptr | `*local_50` in AddBridgeZoneEdges; `*piStack_8/10/c` in RemoveBridgeZoneEdges |
| +0x04..+0x17 | (unknown) | (5 ints, possibly count/cap/vtable for the zones-array itself) | Not directly written by Add/RemoveBridgeZoneEdges |

The level header's remaining 5 ints (+0x04..+0x17) are not touched by AddBridgeZoneEdges or RemoveBridgeZoneEdges. They're likely populated by a separate `BuildZoneGraph` function called at map init. The single DATA xref to 0x87F878 is in `PathfinderClass::InvalidateZoneEdge @ 0x42D082` (per `get_xrefs_to 0x87F878`).

### 11.4 Zone-block layout (within zone_blocks_array, 0x24 bytes per zone)

Already documented in §8.5 above. Repeated here for completeness:

| Offset | Type | Field |
|--------|------|-------|
| +0x00 | code** | vtable (+8 = Grow, +0x10 = Find) |
| +0x04 | int* | edges_ptr (8 bytes per edge) |
| +0x08 | int | capacity |
| +0x0D | char | grow-or-fixed flag byte |
| +0x10 | int | edge count |
| +0x14 | int | growth quantum |
| +0x18 | u16 | representative cell zone id (level-N parent) |
| +0x1A | u16 | (pad / unknown) |
| +0x1C | i32 | LandType-or-class index |
| +0x20 | int | (unknown, fills the 0x24-byte block) |

### 11.5 Edge entry layout (8 bytes)

| Offset | Type | Field |
|--------|------|-------|
| +0x00 | i32 | neighbor zone_id (only low 16 are meaningful; written via MOVSX EBX,short) |
| +0x04 | u32 | flags (low byte; written 0 by AddBridgeZoneEdges) |

**The flags field's low byte is set to 0 by AddBridgeZoneEdges.** This creates a discrepancy with `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md §4.5`, which claimed "edge.flags_low_byte != 0 → bridge_penalty (0.001) applied". Open question §16-(g) below.

### 11.6 Refined contradiction with plan §3 item #45

The plan said:
> 3 × 0x24 stride entries; +0x00 vtable, +0x04 edges_ptr, +0x08 count, +0x10 cap, +0x18 endpoint pair

**Corrected**:
- Stride for level headers = `0x18`, not 0x24.
- `0x24` stride is for **zone blocks WITHIN** a level's zone_blocks_array, not the level headers.
- Zone-block `+0x08` = **capacity**, `+0x10` = **count** (plan inverted these).
- The "+0x18 endpoint pair" phrasing was confused; +0x18 is a u16 = "representative cell zone id" per Zone_precheck §4.4, not an endpoint pair.

Confidence: C=HIGH (offsets read via raw asm + cross-correlated with prior Zone_precheck doc), I=HIGH, B=HIGH (3 writer functions + 1 reader function corroborate).

---

## 12. Active in YR — summary table

| Function | Active in YR? | Gating | Evidence |
|----------|---------------|--------|----------|
| `ComputeBridgeZones @ 0x56D6E0` | Yes | None | Called by CCINIClass init, MapInit, and as fallback by Invalidate/ValidateBridgeZones |
| `FindBridgeRecord @ 0x56DA10` | Yes | None — but HIGH-bridge-only by design (skips +0x0C != 0) | 4 known caller types (GetZoneID, Invalidate, Validate, ResolvePathCoord) |
| `UpdateBridgeZonesHelper @ 0x56C510` | Yes | None | ~28 callers from map init, bridge damage/repair, terrain edit |
| `ZoneFloodFillScanLine @ 0x56CB90` | Yes | None | Sole caller: UpdateBridgeZonesHelper Phase 4 |
| `GetZoneID @ 0x56D230` | Yes | None | Very wide caller list (~70+ via get_xrefs_to) |
| `Can_Reach_Zone @ 0x56D100` | Yes | None | Funnel for "can path?" queries engine-wide |
| `InvalidateBridgeZones @ 0x56DAE0` | Yes | None | ProcessBridgeDamageStateMachine_{High,Low} |
| `ValidateBridgeZones @ 0x56DB70` | Yes | None | ProcessBridgeDestruction + 2 unlabeled (likely repair) |
| `AddBridgeZoneEdges @ 0x5851B0` | Yes | None | Called only by ValidateBridgeZones |
| `RemoveBridgeZoneEdges @ 0x584E50` | Yes | None | Called only by InvalidateBridgeZones |

**No SpecialFlags-gated branches found.** No fog-of-war gates (no `& 0x1000` reads). No tunnel/burrow code paths intersect with the zone-lifecycle functions. This is all live YR code.

---

## 13. Cross-doc contradictions resolved

### 13.1 CELLCLASS_ZONES_SPEED_BRIDGES.md §1.7 — "perpendicular walk" claim

**Refuted.** GetZoneID's walk is ALONG the bridge axis (direction 4 for N-S bridges, direction 2 for E-W bridges). The prior doc's "perpendicular" description is wrong; the formula's correctness was never in question, just the natural-language interpretation. See §6 above.

### 13.2 Plan §3 item #45 — MapClass+0x90 layout

**Refined.** Level stride is 0x18 (not 0x24). Zone-block stride within a level is 0x24. The plan conflated these. Also, zone-block +0x08 is capacity (not count) and +0x10 is count (not cap) — the plan inverted these. See §11 above.

### 13.3 Zone-precheck §4.5 — bridge-edge flag low byte

**Open contradiction (not resolved here).** `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md §4.5` claims `edge.flags_low_byte != 0 → bridge_penalty 0.001 applied`. But `AddBridgeZoneEdges` writes flags low_byte = **0** for every edge it inserts (verified in raw asm). Either:
- (a) The Zone_precheck doc inverted the condition (low_byte == 0 = bridge, != 0 = non-bridge); or
- (b) Bridge-flagged edges are added by a separate function not yet identified; or
- (c) The flag field has more than one bit, and the LOW BYTE is purely "bridge marker" while upper bits encode something else — and AddBridgeZoneEdges-written edges aren't bridge-flagged (they're regular connectivity edges that happen to be added on bridge events).

This is in §16 Open Questions.

### 13.4 Bridge record +0x08 semantic

Prior doc said "Active flag (1 = bridge intact, 0 = destroyed)". **Confirmed**, with refinement: at cold-init via ComputeBridgeZones, +0x08 starts at 1 unless the structural walk encountered a broken cell, in which case +0x08 is initialised to 0. Runtime is toggled exclusively by Invalidate / ValidateBridgeZones.

### 13.5 Records-never-removed invariant

Prior doc claim: "Records never removed; +0x08 is_intact toggled". **Confirmed.** No release code path exists for individual records. The `ComputeBridgeZones` fallback path in Invalidate/Validate rebuilds the entire array, but this isn't "removal" — it's a full rescan.

---

## 14. Current Rust Implementation Status

**This section maps verified findings to existing Rust code. NOT a port plan.** Per CLAUDE.md, Rust internals are free to be cleaner — match observable output, not internal mechanics.

| Binary feature | Rust file | Status |
|----------------|-----------|--------|
| `ComputeBridgeZones` scan (bridge tile detection via DAT_82A734/A774 tables) | [src/sim/pathfinding/zone_build.rs:612](../../ra2-rust-game/src/sim/pathfinding/zone_build.rs#L612) `inject_bridge_adjacency`, [src/sim/pathfinding/zone_build.rs:653](../../ra2-rust-game/src/sim/pathfinding/zone_build.rs#L653) `build_bridge_redirect` | **Partial**. Rust constructs bridge adjacency from BridgeRuntimeState records, not via tile-iteration with per-tile-orientation walk. Equivalent observable result; design difference is fine per CLAUDE.md. **Verify**: bridge_kind tracking — Rust may not distinguish high/low bridge records the same way. |
| `FindBridgeRecord` high-bridge-only filter (+0x0C != 0 skip) | [src/sim/world/bridge_orchestrator.rs:47](../../ra2-rust-game/src/sim/world/bridge_orchestrator.rs#L47) bridge event dispatch | **Audit**. Rust orchestrator handles per-event bridge records; the equivalent of "high-only" filter would be `RuntimeState.is_high_bridge` or similar. Not directly equivalent to gamemd's data-driven skip; observable equivalence depends on whether all bridge events in Rust correspond to high-bridge gamemd events. |
| `UpdateBridgeZonesHelper` 8-phase rebuild | [src/sim/pathfinding/zone_build.rs:463](../../ra2-rust-game/src/sim/pathfinding/zone_build.rs#L463) `flood_fill`, [src/sim/pathfinding/zone_build.rs:568](../../ra2-rust-game/src/sim/pathfinding/zone_build.rs#L568) `extract_adjacency` | **Partial**. Rust uses 8-neighbor BFS rather than scanline; semantically equivalent for unit reachability. Rust does NOT implement the 3-level hierarchical zone graph (only 1 layer per movement zone). The 13-movement-zone per-cluster zone-ID array is implemented via `ZoneGrid`. |
| `ZoneFloodFillScanLine` asymmetric height thresholds (L≤1, R≤3, edge≤1 with Impassable bypass) | [src/sim/pathfinding/zone_build.rs:463](../../ra2-rust-game/src/sim/pathfinding/zone_build.rs#L463) `flood_fill` | **MISSING**. Rust enforces symmetric height-diff ≤1 only (line 246-250 comment). The right-scan ≤3 leniency would cause a steep slope to be in the same zone in gamemd but a different zone in Rust. **Player-visible** for steep ramps onto bridges. |
| `GetZoneID` along-axis walk for destroyed bridges | [src/sim/pathfinding/zone_map.rs:86](../../ra2-rust-game/src/sim/pathfinding/zone_map.rs#L86) `zone_at` | **MISSING**. Rust does nearest-endpoint redirect, not along-axis walk. For straightforward destroyed bridges the result is similar; differs for partially-intact bridges (where Flags & 0x100 persists for some cells). |
| `Can_Reach_Zone` OoB shortcuts (diamond test + 2-way OoB allowance) | [src/sim/pathfinding/zone_map.rs:288](../../ra2-rust-game/src/sim/pathfinding/zone_map.rs#L288) `ZoneGrid::can_reach` | **MISSING**. Rust just does zone-ID equality. The two OoB short-circuits (A-outside-tactical-but-in-diamond → true, A-in + B-outside-but-in-diamond + param_6 → true) are NOT implemented. Player-visible only for fringe-cell movement orders. |
| `InvalidateBridgeZones` / `ValidateBridgeZones` is_intact toggling with tolerance=3 | [src/sim/world/bridge_orchestrator.rs:418](../../ra2-rust-game/src/sim/world/bridge_orchestrator.rs#L418) `refresh_bridge_zones_if_dirty` | **Partial**. Rust uses event-driven flag flips, not search-by-tolerance. Observable: gamemd allows damage **3 cells off** a bridge to invalidate it; Rust may require exact-cell hit. **Verify in playtests.** |
| `AddBridgeZoneEdges` 18-edge insertion (6 per level × 3 levels), no dedup | (none) | **MISSING**. The 3-level hierarchical graph isn't in Rust. Critical for accurate Zone_precheck behavior; absent here means the precheck (already absent) couldn't work in the gamemd manner. |
| `RemoveBridgeZoneEdges` strict-inverse 18-edge removal | (none) | **MISSING**. Same as above. |
| BridgeRecord +0x0C bridge_kind (high vs low) | (partial) | **Audit**. Rust treats bridges via specific high/low events but doesn't surface a single is_high field in BridgeRuntimeState that mirrors +0x0C. |
| Zone-ID `0xFFFF` sentinel at cluster 0 (OoB) | [src/sim/pathfinding/zone_map.rs](../../ra2-rust-game/src/sim/pathfinding/zone_map.rs) | **Audit**. Verify Rust's OoB sentinel matches u16::MAX or similar. If different, the rare OoB cell path will differ in behavior. |
| 256-bucket hash table (8-bit hash from low-4-bits) at MapClass+0x14 | (none) | **MISSING**. Rust uses a different graph representation (per-cluster adjacency vectors); hash-bucket equivalence isn't needed for observable parity unless Zone_precheck is also being implemented. |
| `DAT_00ABDE8C` consecutive-duplicate-edge memo | (n/a) | Internal optimization, no Rust counterpart needed if the observable graph is built equivalently. |

### 14.1 Severity priorities (player-visibility × trigger-frequency)

Per CLAUDE.md, severity = player-visibility × frequency. Quick triggers:

- **HIGH**: `ZoneFloodFillScanLine` right-scan height ≤3 leniency — fires on every steep ramp at every bridge. Affects whether a unit on a ramp shares a zone with the bridgehead cell. Visible as wrong-pathing through steep ramp cells. **Frequency**: every map with bridges (most maps).
- **MEDIUM**: `Invalidate/ValidateBridgeZones` tolerance=3 — damage indicators 3 cells off-bridge currently won't break the bridge in Rust. **Frequency**: every bridge-destruction event where the damage hit is off-cell (common with splash damage from artillery, IFV missiles, etc.).
- **LOW**: 3-level hierarchical zone graph — only matters if/when Zone_precheck is implemented. Currently not exposed.
- **LOW**: GetZoneID along-axis walk vs nearest-endpoint — distinguishable only for partially-broken bridges, an edge case.

(Final severity-tagging deferred to Phase 7 synthesis.)

---

## 15. Edge-case detail catalogue (sanity-check the report rigor)

Per the skill's "tiny-details-are-the-whole-point" doctrine, the following are findings that would be invisible in a top-level summary but matter for parity:

1. **+0x0C != 0 is the high-bridge skip predicate** (not "low-bridge marker") — same effect, but the SKIP direction matters if you're writing an iterator.
2. **FindBridgeRecord tolerance is *per-caller***: 1 (GetZoneID), 2 (ResolvePathCoord), 3 (Invalidate, Validate). Mis-matching the tolerance produces visible misbehavior on splash-damaged or path-snapped near-bridge cases.
3. **InvalidateBridgeZones fallback rebuild**: `if FindBridgeRecord(coord, 3, 0) == -1: ComputeBridgeZones(); retry`. A Rust port that omits the rebuild fallback will fail to invalidate bridges added by mid-game editor actions (rare but possible).
4. **AddBridgeZoneEdges does NOT dedup**. Double-call = double-edges. Pair count is **not idempotent**.
5. **RemoveBridgeZoneEdges finds FIRST match**. So Add×2 then Remove×1 leaves one stale duplicate.
6. **ZoneFloodFillScanLine right-scan threshold is `< 4`** (≤3), not `< 2` (≤1). Verified in raw decomp.
7. **Boundary-edge addition threshold is `< 2`** (≤1) even after right-scan with ≤3 — three distinct thresholds in one function.
8. **Impassable seed (zoneType == 6) bypass**: `bVar19 = (cVar3 == '\x06')` enables boundary edges across height jumps for impassable seeds, but NOT for the walk steps themselves.
9. **DAT_00ABDE8C is a "last edge added" memo**, not a flag. Cleared at function entry, set after each edge add, read to suppress consecutive-duplicate edge adds.
10. **The 256-bucket hash is 8-bit**: `(smaller & 0xF) << 4 | (larger & 0xF)`. Low 4 bits only.
11. **Each connection-graph edge stores its key at BOTH +0 and +4** (duplicate). Wasteful storage but consistent — no flag field on the connection graph. This is DIFFERENT from the hierarchical-graph edge layout, which uses +0 for neighbor_zone_id and +4 for flags.
12. **Zone-ID 1 is "blocked"** sentinel; 2+ are real zones; **0xFFFF is OoB sentinel** (forced overwrite at cluster 0). 0 is transient (only inside BFS before assignment).
13. **Phase 4 cluster IDs start at 1** (`uVar21 = 1`); cluster 0 is the reserved unassigned sentinel.
14. **Bridge baking iterates records BACKWARDS** in Phase 5 of UpdateBridgeZonesHelper. Likely a hot-cache-or-stylistic choice; semantically order-independent for the hash-bucket structure but observable if a Rust port iterates differently and then triggers a vector-grow at a different point.
15. **Validate's Can_Reach_Zone post-test uses MovementZone 0** (`Can_Reach_Zone(... 0, 0, 0, 0)`). MovementZone 0 is typically "Foot" or similar — selecting a different zone here would change the "needs rebuild" return.
16. **OoB phantom cell at DAT_00ABDC50**: GetZoneID populates `DAT_00ABDC74 = coord` and uses `&DAT_00ABDC50` as a synthetic CellClass. Bridge flags read from it are always 0 because the 36-byte buffer at DAT_00ABDC50 is zero-initialized.
17. **Asymmetric Y-traversal in iso flood-fill**: rows above and below the span use diagonal offsets (rowstride±4 byte adjustments) depending on column position within the span — accounting for the iso-coord-pair-shift between adjacent rows.
18. **ZoneFloodFillScanLine recursion termination**: returns `local_1c` = `(pcVar17 - pcVar16) >> 2 - 1` = span width counted in cells (4-byte stride). Each recursive call adds its own width to local_1c. This is the "total cells visited" return.
19. **The "growth quantum" at zone-block +0x14**: when capacity is exceeded, the grow call requests `current_count + quantum`. Quantum size is per-block; not yet enumerated.
20. **Pathfinding_update_continued's direction arg is `& 7`**: only low 3 bits matter, but the caller passes the full uint. Indirect implication: if a caller passes a value like 0x108, the masking still works.

---

## 16. Open Questions

1. **Bridge-edge flag low-byte semantic** (carries over from §13.3): definitively determine the meaning of `edge.flags & 0xFF` in the MapClass+0x90 hierarchical graph. AddBridgeZoneEdges writes 0. Zone_precheck reads it for bridge_penalty. Either the Zone_precheck doc inverted the condition, or another writer also exists. **Recommended follow-up**: decompile every writer of edges to MapClass+0x90 — likely a `BuildZoneGraph` or `ZoneMap::Initialize` function called once at map init.

2. **The 5 unknown int fields in the level header** (MapClass+0x90 +0x04..+0x17). What do they store? Likely vector count/cap/vtable for the zone_blocks_array. Need to find the writer.

3. **Zone-block +0x18 representative cell** semantic: `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK §4.4` calls it "level-N parent". Confirm by tracing a concrete write site at map init.

4. **Zone-block +0x1C LandType class index** vs zoneType (cell-level 0..7): are they the same enum, or different (e.g., aggregated LandType for a whole zone)?

5. **The growth quantum at zone-block +0x14**: typical values? Per-graph-level? Per-zone?

6. **MergeAdjacentCellZone @ 0x56D5A0** — sibling of AssignOrphanedCellZone. Same algorithm? Different threshold? Covered in companion doc.

7. **Cluster-zoneType vector** at `local_14` in UpdateBridgeZonesHelper Phase 4 — uses vtable at `PTR_FUN_007ED580` and initial cap 300. Is this a hard upper bound on cluster count, or just initial capacity? If cluster count exceeds 300, what happens?

8. **The DAT_0082A944 direction table** used by AddBridgeZoneEdges and RemoveBridgeZoneEdges for bridgehead-step direction. Is it 16-entry (per tile orientation) like the prior bridge tables? Need to read.

9. **ComputeBridgeZones direction tables DAT_0082A734 vs DAT_0082A774 vs DAT_0082A7B4**: prior docs identified them but their full content / number of entries isn't enumerated.

10. **What does FUN_005835D0 do for the no-record-found case** in ResolvePathCoord_BridgeAware? Covered in companion doc.

11. **The 4 unlabeled callers of UpdateBridgeZonesHelper** (FUN_00567110, FUN_00568E40, FUN_00569760, FUN_00581140, FUN_00594B50, FUN_00684C30, FUN_006E21E0): likely overlay-edit, scenario triggers, multiplayer-resync. Need identification.

---

## 17. Sources

**Ghidra functions decompiled** (Phase 4, this report):
- `MapClass::ComputeBridgeZones` @ 0x0056D6E0 (full body, ~720 bytes)
- `MapClass::FindBridgeRecord` @ 0x0056DA10 (full body, ~196 bytes)
- `MapClass::UpdateBridgeZonesHelper` @ 0x0056C510 (full body, ~3094 bytes — the largest in Phase 4)
- `MapClass::ZoneFloodFillScanLine` @ 0x0056CB90 (full body, ~1700 bytes)
- `MapClass::GetZoneID` @ 0x0056D230 (full body, ~360 bytes)
- `MapClass::Can_Reach_Zone` @ 0x0056D100 (full body, ~280 bytes)
- `MapClass::InvalidateBridgeZones` @ 0x0056DAE0 (full body, ~144 bytes)
- `MapClass::ValidateBridgeZones` @ 0x0056DB70 (full body, ~190 bytes)
- `MapClass::AddBridgeZoneEdges` @ 0x005851B0 (full body, ~990 bytes)
- `MapClass::RemoveBridgeZoneEdges` @ 0x00584E50 (full body, ~810 bytes)
- `FUN_00589290` @ 0x00589290 (remove-edge-at-index helper, ~70 bytes)
- `Pathfinding_update_continued` @ 0x00481810 (CellClass::Get_Neighbor — see companion doc Item #46)
- `ZoneMap::FloodFillReachableZones` @ 0x005840C0 (cross-ref for MapClass+0x90 layout)

**Raw assembly verified for**:
- GetZoneID loop at 0x56D2E5..0x56D318 (perpendicular-walk inverted-ternary RESOLUTION)
- FindBridgeRecord +0x0C skip at 0x56DA3A..0x56DA3F
- AddBridgeZoneEdges flag-byte init at 0x5852B5..0x5852CE (six bytes, all 0)
- AddBridgeZoneEdges outer-loop structure at 0x585570..0x585581 (EBP += 2, < 6)
- AddBridgeZoneEdges edge writes at 0x58535E/0x585361 (neighbor + flag layout)

**Memory reads**:
- 0x0089F688 len 32 (Pathfinding_update_continued direction offset table — BSS, runtime-init, cold zero)
- 0x007E3774 len 32 (g_CellNeighborOffsets_8Dir; signed i32 linear cell offsets confirming direction convention)
- 0x0087F878 len 32 (MapClass+0x90 = zone-graph base; BSS)
- 0x0089F68A len 36 (alias for direction table; BSS)

**Cross-reference checks** (binding evidence):
- `get_xrefs_to 0x56D230` — 70+ callers (GetZoneID is widely consumed)
- `get_function_callers 0x56C510` — 28 callers (UpdateBridgeZonesHelper)
- `get_function_callers 0x56DAE0` — 2 callers (Invalidate: damage state machines)
- `get_function_callers 0x56DB70` — 4 callers (Validate: destruction completion + 2 unlabeled repair)
- `get_function_callers 0x481810` — 50+ callers (Pathfinding_update_continued = CellClass::Get_Neighbor; pervasive utility)
- `get_xrefs_to 0x87F878` — 1 DATA xref from `PathfinderClass::InvalidateZoneEdge @ 0x42D082`
- `get_xrefs_to 0x89F688` — 100+ readers/data refs (per-call CellClass::Get_Neighbor users)

**Companion doc**:
- [BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md](BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md) (items #43–#46: AssignOrphanedCellZone, ResolvePathCoord_BridgeAware, MapClass+0x90 detail, Pathfinding_update_continued identification)

**Phase 1–3 dependency docs**:
- [BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md](../03-traversal-pathfinding-entry/BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md)
- [BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md](BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md) — especially §4 Zone_precheck
- [BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md](../03-traversal-pathfinding-entry/BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md)
- [BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md](../03-traversal-pathfinding-entry/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md)

**Prior research docs cross-checked / refuted / refined**:
- [CELLCLASS_ZONES_SPEED_BRIDGES.md](CELLCLASS_ZONES_SPEED_BRIDGES.md) — §1.7 perpendicular-walk REFUTED (§6 above), §1.5 algorithm CONFIRMED, §3.2 BridgeRecord layout CONFIRMED with field-by-field re-verification
- [TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md](../../TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md) — connection-graph hash (256 buckets) CONFIRMED
- [ZONE_INCREMENTAL_DIVERGENCE_GHIDRA_REPORT.md](../../ZONE_INCREMENTAL_DIVERGENCE_GHIDRA_REPORT.md) — bridge_kind missing from Rust call-out CORROBORATED here
- [ZONE_PASSABILITY_VERIFIED.md](../../ZONE_PASSABILITY_VERIFIED.md) — 13×8 matrix CONFIRMED (matrix-base 0x82A594, row stride 8 ints, end 0x82A734)
- [BRIDGE_RUNTIME_DEEP_DIVE_GHIDRA_REPORT.md](../00-system-models/BRIDGE_RUNTIME_DEEP_DIVE_GHIDRA_REPORT.md) — endpoint-active flag toggling CONFIRMED

---

## 18. Audit log entry

For inclusion in `AUDIT_LOG.md`:

```
2026-05-13 — Phase 4 zone-lifecycle re-investigation (this doc).
Refuted: CELLCLASS_ZONES_SPEED_BRIDGES.md §1.7 "perpendicular walk" claim.
Refined: plan's MapClass+0x90 stride claim (level=0x18, zone-block=0x24).
Confirmed: BridgeRecord layout +0x00/+0x04/+0x08/+0x0C verified at writer sites.
Confirmed: 256-bucket hash on +0x14 connection graph, 18-edges (6×3) per bridge in +0x90 hierarchical graph.
Confirmed: ZoneFloodFillScanLine asymmetric height thresholds (L≤1, R≤3, edge≤1, Impassable bypass).
Confirmed: Invalidate/Validate tolerance=3, ComputeBridgeZones fallback on no-record-found.
Open: bridge-edge flag low-byte semantic (see §13.3 / §16-1).
```
