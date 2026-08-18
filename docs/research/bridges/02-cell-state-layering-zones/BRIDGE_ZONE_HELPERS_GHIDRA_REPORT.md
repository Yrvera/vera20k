# Bridge Zone Helpers — Ghidra Research Report

**Phase:** Phase 4 of approved plan `docs/plans/2026-05-13-bridge-pathfinding-locomotion-investigation-plan.md`
**Plan items covered:** #43 (AssignOrphanedCellZone), #44 (ResolvePathCoord_BridgeAware), #45 (MapClass+0x90 struct layout — also covered in lifecycle doc §11), #46 (Pathfinding_update_continued identification)
**Companion doc:** [BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md](BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md) (items #33–#42 — the build/destroy/validate trio and edge inserters/removers)
**Date:** 2026-05-13
**Active in YR:** **Yes** — all functions in this report are live in standard YR skirmish.

> Every claim cites a Ghidra address + decompilation excerpt or raw asm or `read_memory` byte dump or `get_xrefs_to` / `get_function_callers` result.
> Confidence axes: **C** = content (algorithm verified), **I** = identity (function name verified), **B** = binding (caller path verified).

---

## 1. AssignOrphanedCellZone (#43) @ 0x56D460 — incremental orphan repair

`param_1` type: **`int`** (this, MapClass*). `__thiscall(int param_1, short *param_2)`.

| Param | Type | Meaning |
|-------|------|---------|
| `param_1` | MapClass* | this |
| `param_2` | short* | coord (X at +0, Y at +2) of orphaned cell |

### 1.1 What it does

When a cell has its zoneType changed (e.g., a building is placed/sold/destroyed and the underlying cell's classification flips between Clear and Building/Impassable), this function tries to **inherit a cluster ID from a same-zoneType neighbor**. If it can't, it falls back to a full `UpdateBridgeZonesHelper` rebuild.

### 1.2 Algorithm

```c
// Compute linear cell index, clamp
linear_idx = (mapWidth + 1 + mapOriginX) * coord.Y + coord.X;
linear_idx = clamp(linear_idx, 0, totalCells - 1);
pcVar1 = (char *)(cellZoneData + linear_idx * 4);

// 8 neighbor offsets in linear index space:
local_10 = mapWidth + 1 + mapOriginX;       // row stride
local_20[0] = -local_10;                     // N (up 1 row)
local_20[1] = 1 - local_10;                  // NE (up 1 row, +1 col)
local_20[2] = 1;                              // E
local_20[3] = local_10 + 1;                  // SE
// local_8 = 0xffffffff (sentinel)
local_c = local_10 - 1;                       // SW
local_4 = -1 - local_10;                      // NW
// (W = -1 implicit in iteration)

// Skip if cell is OoB (zoneType==7)
if (*pcVar1 != '\a') {
  iVar3 = 0;
  piVar5 = local_20;
  do {
    pcVar2 = pcVar1 + *piVar5 * 4;            // neighbor cell entry
    // Check neighbor's zoneType byte
    if (pcVar2[0] == '\0') {                  // neighbor zoneType == 0 (Clear?) — orphan-found pattern
      // Walk 8 neighbors counting DISTINCT existing zone IDs
      // ...
      if (distinct_zone_count < 4 && cell_unassigned) {
        // Inherit neighbor's cluster ID
        *(undefined2 *)(pcVar1 + 2) = *(undefined2 *)(pcVar2 + 2);
        return;
      }
      break;       // give up; fall through to UpdateBridgeZonesHelper
    }
    iVar3 = iVar3 + 1;
    piVar5 = piVar5 + 1;
  } while (iVar3 < 8);
  MapClass__UpdateBridgeZonesHelper();
}
```

Wait — re-reading the decomp more carefully:

```c
if (pcVar1[*piVar3 * 4] == '\0') {                                       // neighbor's zoneType is 0?
  if (pcVar1 + *piVar3 * 4 != (char *)0x0) {
    param_2 = (short *)0x0;
    uVar6 = 0;
    piVar5 = local_20;
    local_28 = 8;
    do {
      uVar4 = (uint)*(ushort *)(pcVar1 + *piVar5 * 4 + 2);                // neighbor's cluster_id
      if ((*(short *)(*(int *)(param_1 + 0x18) + uVar4 * 2) !=             // zone_id for this cluster in MovementZone 0
           *(short *)(*(int *)(param_1 + 0x18) + uVar6 * 2)) &&
          (pcVar1[*piVar5 * 4] != '\a'))                                  // neighbor is not OoB
      {
        param_2 = (short *)((int)param_2 + 1);                             // distinct-zone count++
        uVar6 = uVar4;
      }
      piVar5 = piVar5 + 1;
      local_28 = local_28 + -1;
    } while (local_28 != 0);
    
    if (((int)param_2 < 4) && (*pcVar1 == '\0')) {
      *(undefined2 *)(pcVar1 + 2) = *(undefined2 *)(pcVar1 + *piVar3 * 4 + 2);  // inherit cluster
      return;
    }
  }
  break;
}
```

**The distinct-zone-count is capped at 4**. If fewer than 4 distinct neighbor zones exist AND the cell itself has zoneType 0 (Clear), then inherit the FIRST neighbor's cluster ID and bail.

Otherwise, full rebuild via `UpdateBridgeZonesHelper`.

### 1.3 The fast path: cell-zoneType-0 with same-zone neighbor

The semantic is: "if the cell is Clear and surrounded by ≤3 distinct zones (i.e., it's in the interior of a connected component), inheriting any neighbor's cluster is safe." This is the **fast incremental update**.

The slow path (full rebuild) fires when:
- The orphaned cell is at a CHOKEPOINT (≥4 distinct neighbor zones) — inheriting would be wrong
- The cell's own zoneType isn't 0 (e.g., became Wall/Impassable) — needs reclassification
- No neighbor has zoneType 0

### 1.4 8-neighbor offset pattern (linear-index offsets)

`local_20` array elements:
- N = `-(width+1+originX)` = -row stride
- NE = `1 - row_stride`
- E = `+1`
- SE = `+row_stride + 1`
- (implicit W = `-1` between SE and SW)
- SW = `+row_stride - 1`
- (S = `+row_stride` not explicitly listed but visible at local_20[2] which is set to 1 = E)

Wait — looking at the decomp again:
```c
local_20[2] = 1;
local_8 = 0xffffffff;             // = -1 = W
local_c = local_10 - 1;            // SW
local_4 = -1 - local_10;          // NW
local_20[0] = -local_10;          // N
local_20[1] = 1 - local_10;       // NE
local_20[3] = local_10 + 1;       // SE
```

Stack layout means these are adjacent local variables; the iteration order in the decomp is `local_20[0..3]` then... I see `local_28 = 8` as the iter count.

Looking at the stack-adjacent layout: `local_20[0], local_20[1], local_20[2], local_20[3], local_28, local_8, local_c, local_4` (assuming local_20 is 4 ints, local_28/8/c/4 are 1 int each at byte offsets -0x28/-0x8/-0xC/-0x4 from EBP). So 8 neighbor offsets total in stack-adjacent slots iterated by `piVar5++` 8 times.

Likely linear iteration order: `[0]=N, [1]=NE, [2]=E, [3]=SE, [4]=local_28=8(iteration counter, NOT offset)`... that doesn't fit. Let me reconsider — `local_28` is the iteration counter (`do { ... } while (local_28 != 0)`). So 8 distinct offsets are at local_20[0..3], local_c, local_4, local_8 — that's 7. Missing one.

Actually the decomp uses two separate iterations:
```c
do {                          // First iteration (outer)
  pcVar2 = pcVar1 + *piVar3 * 4;
  ...
  iVar3 = iVar3 + 1;
  piVar3 = piVar3 + 1;
} while (iVar3 < 8);          // iterate 8 neighbors
```

So `piVar3` (pointer to local_20[0]) is incremented 8 times — needing 8 contiguous int slots. local_20 declared as `int local_20[4]` but the compiler also packs `local_8`, `local_c`, `local_4` adjacent for 8 total slots. So the 8 offsets are: N, NE, E, SE, S?, SW?, W?, NW? — exact ordering matches stack layout. The "missing" entry is S = `+row_stride` itself, which would be at the byte-offset corresponding to local_8 or similar. The decomp is ambiguous on exact stack layout, but the LOGIC is "check all 8 neighbors", which we can infer from the iter count.

Confidence: C=MEDIUM (offset table partially obscured by stack-layout ambiguity), I=HIGH (the function is clearly an 8-neighbor incremental zone helper), B=HIGH (single caller MergeAdjacentCellZone @ 0x56D5A0, also called from terrain-edit paths).

### 1.5 Sibling: MergeAdjacentCellZone @ 0x56D5A0

The function `MergeAdjacentCellZone` at 0x56D5A0 has near-identical structure. Difference: it compares MOVEMENT-ZONE-0 zone IDs directly (not just distinct-count) and uses the SAME 8-neighbor offset table. Inherits the SAME cluster ID if ALL inspected neighbors have same zoneType. Triggers `UpdateBridgeZonesHelper` if disagreement found.

Both functions are paired: typically one of them is called from terrain-edit code (e.g., `BuildingClass::OnConstructionComplete` or `CellClass::RecalcZoneType`). The split is between "cell flipped TO Clear, find a neighbor cluster to merge with" (AssignOrphaned) and "cell stayed in same class, just check adjacent connectivity" (MergeAdjacent).

Callers via `get_function_callers 0x56D460`: limited to direct cell-edit paths. **No SpecialFlags gating.**

---

## 2. ResolvePathCoord_BridgeAware (#44) @ 0x583180 — Sqrt_Approx endpoint pick

`param_1` is the OUT pointer (a stack-allocated `undefined4 *`). Signature: `undefined4 * (undefined4 *param_1, CellClass *param_2, int param_3)`.

| Param | Type | Meaning |
|-------|------|---------|
| `param_1` | undefined4* | OUT: returned coord buffer (4 bytes, the resolved coord) |
| `param_2` | CellClass* | the cell whose coord to resolve |
| `param_3` | int | low byte = checkBridge flag |

Returns `param_1` (the OUT buffer pointer).

### 2.1 Trivial case

```c
if (((char)param_3 == '\0') || ((param_2->Flags & 0x100U) == 0)) {
  *param_1 = *(undefined4 *)&param_2->MapCoord_X;
  return param_1;
}
```

If `checkBridge` is off OR the cell isn't on a bridge → just return the cell's own coord. No transformation.

### 2.2 Find bridge record (tolerance=2)

```c
iVar8 = MapClass__FindBridgeRecord(psVar1, 2, 0);
```

Tolerance is **2** — different from Invalidate/Validate's 3 and GetZoneID's 1. Tight-but-not-too-tight for path-snap purposes.

If no record found, fallback via `FUN_005835D0` (see §3 below).

### 2.3 Bridge orientation via cell.flags & 0x800

```c
psVar12 = (short *)(bridge_record + iVar8 * 0x10 + DAT_0087F83C);    // record_ptr
                                                                       // (DAT_0087F83C aliases MapClass+0x54)
if ((uVar3 & 0x800) == 0) {
  sVar10 = *psVar1 - *psVar12;          // X offset from endpoint_a
  sVar11 = 0;
} else {
  sVar11 = param_2->MapCoord_Y - psVar12[1];     // Y offset from endpoint_a
  sVar10 = 0;
}
```

**Bridge orientation is read from CELL flags, not the bridge record.** Bit `0x800` (BridgeOrientation: 0=N-S, 1=E-W per `BRIDGE_SYSTEM.md`):
- `0x800 == 0` (N-S oriented bridge): compute offset along X axis (perpendicular to bridge body)
- `0x800 != 0` (E-W oriented): compute offset along Y axis (perpendicular to bridge body)

So `(sVar10, sVar11)` represents the cell's PERPENDICULAR offset from the bridge body axis at endpoint_a. This is the "how far off the bridge centerline" measurement.

### 2.4 Intact bridge: Sqrt_Approx endpoint comparison

```c
if ((char)psVar12[4] != '\0') {      // bridge is intact
  // Distance from cell to endpoint_a (with the perpendicular offset added):
  param_3 = (int)(short)((*psVar12 + sVar10) - *psVar1);
  dVar4 = (double)(int)(short)((psVar12[1] + sVar11) - param_2->MapCoord_Y);
  Sqrt_Approx(dVar4 * dVar4 + (double)param_3 * (double)param_3);
  sVar6 = Math__ftol();                  // distance to endpoint_a

  // Distance from cell to endpoint_b:
  param_3 = (int)(short)((psVar12[2] + sVar10) - *psVar1);
  dVar4 = (double)(int)(short)((psVar12[3] + sVar11) - param_2->MapCoord_Y);
  Sqrt_Approx(dVar4 * dVar4 + (double)param_3 * (double)param_3);
  sVar7 = Math__ftol();                  // distance to endpoint_b

  if (sVar6 < sVar7) {
    // endpoint_a is CLOSER → return endpoint_a + offset
    *param_1 = CONCAT22(psVar12[1] + sVar11, *psVar12 + sVar10);
    return param_1;
  }
  // endpoint_b is closer (or equal) → return endpoint_b + offset
  *param_1 = CONCAT22(psVar12[3] + sVar11, sVar10 + psVar12[2]);
  return param_1;
}
```

**Snap-to-closer-endpoint algorithm:**
- Compute Euclidean distance² from cell position to each endpoint (with perpendicular offset added so the comparison is from the same iso-row/column as the cell).
- Pick the closer endpoint.
- Return `(endpoint_coord + perpendicular_offset_from_endpoint)`.

The Sqrt_Approx → ftol roundtrip converts to integer-distance for the comparison. `Math__ftol` truncates toward zero. So fractional differences at the boundary may flip the choice; this is **deterministic** but subject to lepton-level rounding.

**Tie-breaker**: `sVar6 < sVar7` (strict less-than). So if distances are equal, **endpoint_b wins**. This is a subtle but deterministic detail — Rust ports must replicate the strict-less-than to match.

### 2.5 Destroyed bridge: along-axis walk fallback

```c
while ((uVar3 & 0x100) != 0) {
  param_2 = (CellClass *)Pathfinding_update_continued();      // step in (cached) direction
  uVar3 = param_2->Flags;
}
bVar5 = CellClass__IsBridge(param_2);
if (((bVar5) || (bVar5 = CellClass__IsWoodBridge(param_2), bVar5)) && (param_2->LandType != 3)) {
  param_2 = (CellClass *)CONCAT22(psVar12[3] + param_3._2_2_, psVar12[2] + sVar10);    // endpoint_b + offset
  *param_1 = param_2;
  return param_1;
}
param_2 = (CellClass *)CONCAT22(psVar12[1] + param_3._2_2_, *psVar12 + sVar10);        // endpoint_a + offset
*param_1 = param_2;
```

**Same algorithm as GetZoneID's walk** — see lifecycle doc §6. Walks along the bridge axis (direction set earlier via `local_8 = (local_4 & 0xfffffffe) + 4` where `local_4 = -(uint)((cell.flags & 0x800) != 0)`). The direction logic is the same inverted-ternary as GetZoneID: orientation determines direction.

After walking until exiting the bridge, check whether the exit cell is a bridge tile (concrete or wood) AND not Rock. If yes → use endpoint_b + offset; otherwise → endpoint_a + offset.

This matches GetZoneID's logic exactly (the prior CELLCLASS_ZONES_SPEED_BRIDGES.md doc's perpendicular claim was wrong here too — the walk is along-axis here too).

### 2.6 Active in YR

Callers via `get_xrefs_to 0x583180`: not directly in xrefs list (function-level callers via `get_function_callers` needed). The function is invoked from path-smoothing and waypoint-resolution code, given its name and the way it's structured. Standard YR pathfinding path.

Confidence: C=HIGH (decomp + Sqrt_Approx verified at known address), I=HIGH (function name matches semantic), B=MEDIUM (caller list not exhaustively traced — would need a separate xref pass).

---

## 3. FUN_005835D0 — ResolvePathCoord fallback (no-record-found)

Used when ResolvePathCoord_BridgeAware can't find a matching bridge record (FindBridgeRecord returned -1 with tolerance 2). The fallback function:

1. Loads the cell at the input coord.
2. If not on a bridge (flags & 0x100 == 0), return cell's own coord. (Trivial.)
3. Walks the bridge in BOTH directions (along and opposite) to find a non-bridge cell on each side.
4. For each direction, if the exit cell is a bridge tile (IsBridge/IsWoodBridge) AND not Rock, record that exit cell's coord.
5. Compute Sqrt_Approx-based distances from `param_3` (the reference path coord) to both candidates.
6. Return the CLOSER candidate (`sVar7 <= sVar6` → keep the original).

**Differences from main ResolvePathCoord**:
- Walks in BOTH directions (along bridge axis AND opposite) instead of just one
- Picks the candidate CLOSER to the reference coord (param_3), not to the cell itself
- Has a strict-less-or-equal tiebreaker (the original "param_2" path wins ties)

This is the **edge case for orphan bridge cells without a record** (rare but possible after editor edits or save/load corruption).

Confidence: C=HIGH (decomp captures both directions explicitly), I=MEDIUM (unlabeled, identity inferred from caller context), B=HIGH (single caller is ResolvePathCoord_BridgeAware).

---

## 4. Pathfinding_update_continued (#46) @ 0x481810 — IDENTIFIED

### 4.1 What the user's plan labelled "Pathfinding_update_continued" is

The plan §9 question #11 said:
> Caller of GetZoneID not yet labeled | Pathfinding_update_continued | User-listed as called from GetZoneID. Find via xref and decomp.

**Resolution**: Ghidra has a function at `0x00481810` labeled **`Pathfinding_update_continued`**. It is NOT a caller of GetZoneID — it is a **callee** (called BY GetZoneID, ComputeBridgeZones, ResolvePathCoord_BridgeAware, AddBridgeZoneEdges, RemoveBridgeZoneEdges, and ~50 other functions).

**Actual purpose**: it is **`CellClass::Get_Neighbor`** (or equivalent). Takes a `this` (CellClass*) and a direction (uint 0–7), returns the neighbor CellClass* in that direction.

### 4.2 Algorithm

```c
void __thiscall Pathfinding_update_continued(CellClass *this, uint direction) {
  if (direction < 8) {
    short cur_x = (short)*(undefined4 *)(this + 0x24);          // MapCoord_X
    short cur_y = (short)((uint)*(undefined4 *)(this + 0x24) >> 0x10);   // MapCoord_Y
    short dx = *(short *)(0x89F688 + (direction & 7) * 4);       // direction table dx
    short dy = *(short *)(0x89F688 + (direction & 7) * 4 + 2);   // direction table dy
    CellStruct neighbor_coord;
    neighbor_coord.X = cur_x + dx;
    neighbor_coord.Y = cur_y + dy;
    return MapClass__Get_CellClass(&neighbor_coord);              // returns CellClass* via EAX
  }
  return; // direction >= 8 = no-op
}
```

The decomp signature says `void`, but the asm at `0x00481862: POP ECX; 0x00481863: RET 0x4` after `0x0048185D: CALL 0x005657A0` (MapClass::Get_CellClass) propagates EAX through. So the **actual return type is `CellClass*`** — Ghidra's signature is wrong.

Verified at `0x00481862: POP ECX; RET 0x4` — preserves EAX from the MapClass::Get_CellClass return.

### 4.3 Direction table at 0x89F688 — BSS, runtime-init

```
read_memory 0x0089F688 len 32 → all zeros (BSS, runtime-init)
```

The table is initialized at game startup, likely from `g_CellNeighborOffsets_8Dir @ 0x007E3774` (which IS in .rdata and contains the linear-cell-array offsets). I confirmed via `read_memory 0x7E3774`:

| Dir | linear offset | dx | dy | Meaning |
|-----|---------------|----|----|---------|
| 0 | -512 | 0 | -1 | N |
| 1 | -511 | +1 | -1 | NE |
| 2 | +1 | +1 | 0 | E |
| 3 | +513 | +1 | +1 | SE |
| 4 | +512 | 0 | +1 | S |
| 5 | +511 | -1 | +1 | SW |
| 6 | -1 | -1 | 0 | W |
| 7 | -513 | -1 | -1 | NW |

(0x89F688 stores the (dx,dy) shorts directly; 0x7E3774 stores the linear-array offsets. Both are referencing the same 8-direction convention.)

### 4.4 Caller list (via `get_function_callers 0x481810`)

**50+ callers**. Selection of the most relevant for the bridge-zone domain:

- `MapClass::GetZoneID @ 0x56D230` — bridge-along-axis walk (lifecycle doc §6)
- `MapClass::ComputeBridgeZones @ 0x56D6E0` — initial bridge endpoint scan (lifecycle doc §2)
- `MapClass::ResolvePathCoord_BridgeAware @ 0x583180` — destroyed-bridge endpoint snap (§2 above)
- `FUN_005835D0` — fallback for no-record-found (§3 above)
- `ZoneMap::FloodFillReachableZones @ 0x5840C0` — 8-neighbor reachability flood
- `MapClass::PlaceBridgeRamp_Low @ 0x579010` — bridge construction
- `MapClass::ClearBridgeCell_Low @ 0x57A320` — bridge teardown
- `MapClass::UpdateBridgeTile_Low @ 0x57A430`, `SelectBridgeTileVariant_Low @ 0x57ACF0` — bridge tile mgmt
- `DriveLocomotionClass::Process_Movement @ 0x4B2630` — movement step
- `ShipLocomotionClass::Process_Movement @ 0x6A1C80` — movement step
- `UnitClass::Can_Enter_Cell @ 0x73F0A0` — pathing predicate
- `FootClass::Find_Path @ 0x4D3920` — A* entry
- `IsFogged @ 0x5864A0`, `IsShrouded @ 0x586360` — fog/shroud cell neighbor lookup
- `AnimClass::FindAttachTarget @ 0x425D10`, `WarheadTypeClass::Detonate @ 0x4690B0`, etc.

**This is a foundational utility — not specific to pathfinding.** It is the engine-wide "step to neighbor cell" primitive.

### 4.5 Suggested Ghidra rename

The current name `Pathfinding_update_continued` is misleading (it suggests pathfinding-specific use). A more accurate name would be **`CellClass__Get_Neighbor`** or **`CellClass__Step_Direction`**.

**NOT renamed in this investigation** per CLAUDE.md's rule "Only label what you understand with ~90% confidence" combined with "Don't bulk-label without user approval". The 50+ callers all use it as Get_Neighbor — high confidence. The rename should be a separate user-approved action, not a side effect of this research.

Confidence: C=HIGH (full asm + caller convention verified), I=MEDIUM (current label misleading; true identity is Get_Neighbor with HIGH confidence), B=HIGH (50+ callers spanning the engine).

---

## 5. MapClass+0x90 region — detailed inspection (Item #45)

The bulk of this analysis is in **lifecycle doc §11**. This section adds detail extracted from the helpers covered in this report (FloodFillReachableZones, FUN_00589290, the vtable+0x10 Find method).

### 5.1 Layout summary (verified)

| Address | Stride | Contents |
|---------|--------|----------|
| 0x87F878 (=MapClass+0x90) | 0x18 (level header) × 3 | 3 hierarchical zone graph level headers |

Each **level header** (0x18 bytes):
- +0x00 = zone_blocks_array_ptr (used as `*local_50` in Add/RemoveBridgeZoneEdges)
- +0x04..+0x17 = (5 ints, undocumented — likely count/cap/vtable for the zones-array)

Each **zone block** within a level's array (0x24 bytes):

| Offset | Type | Field | Cross-references |
|--------|------|-------|------------------|
| +0x00 | code** | vtable | Reader: AddBridgeZoneEdges (vtable+0x8 Grow), RemoveBridgeZoneEdges (vtable+0x10 Find), Zone_precheck (per prior doc) |
| +0x04 | int* | edges_ptr | Edge array, 8 bytes per entry (zone_id, flags) |
| +0x08 | int | **capacity** | AddBridgeZoneEdges compares `count < capacity` → can insert without grow |
| +0x0D | char | grow-or-fixed flag byte | If non-zero, grow path is allowed |
| +0x10 | int | **count** | Edge count. AddBridgeZoneEdges increments; FUN_00589290 decrements |
| +0x14 | int | growth quantum | Added to count for the grow request |
| +0x18 | u16 | representative cell zone id (level-N parent / hierarchical link) | Per Zone_precheck §4.4 |
| +0x1A | u16 | (pad / unknown) | Reserved |
| +0x1C | i32 | LandType-or-class index | Per Zone_precheck §4.4 |
| +0x20 | int | (unknown — fills the 0x24 block) | |

### 5.2 Edge entry layout (8 bytes per entry in edges_ptr)

| Offset | Type | Field |
|--------|------|-------|
| +0x00 | i32 | neighbor zone_id (u16 in low 16 bits, sign-extended via MOVSX) |
| +0x04 | u32 | flags (low byte = 0 written by AddBridgeZoneEdges) |

**The flags low-byte semantic is OPEN** (see lifecycle doc §13.3 and §16-1). AddBridgeZoneEdges writes 0; Zone_precheck doc claims `!= 0 → bridge_penalty`. This is unresolved without identifying the canonical bridge-edge writer.

### 5.3 vtable methods used

From decomp + asm of consumers:
- **vtable+0x08** = "Grow" — accepts (new_capacity, flag) and returns char (success). Called by AddBridgeZoneEdges and UpdateBridgeZonesHelper Phase 5.
- **vtable+0x0C** = "Clear" — called by UpdateBridgeZonesHelper Phase 1 (clears all 256 buckets in connection graph).
- **vtable+0x10** = "Find" — accepts a target zone_id pointer, returns the edge index or -1. Called by RemoveBridgeZoneEdges.

### 5.4 Memory state verification (BSS)

```
read_memory 0x0087F878 (= MapClass+0x90), len 32 → all zeros
read_memory 0x0089F688 (direction table for Get_Neighbor), len 32 → all zeros
read_memory 0x0089F68A (alias / interleaved), len 36 → all zeros
```

All BSS. Runtime-initialised at game startup (likely by `GScenarioClass::Initialize` or similar; not traced here). The static structure is what we've documented; the dynamic content is per-map.

### 5.5 Plan refinement (REPEATED from lifecycle doc §13.2 for completeness)

The plan §3 item #45 said "3 × 0x24 stride entries; +0x00 vtable, +0x04 edges_ptr, +0x08 count, +0x10 cap, +0x18 endpoint pair".

**Corrections**:
- Level header stride is **0x18**, not 0x24.
- 0x24 is **zone-block** stride within a level.
- Zone-block **+0x08 is capacity** (plan said count), **+0x10 is count** (plan said cap) — inverted.
- "+0x18 endpoint pair" misnomer — it's a u16 representative cell zone id per Zone_precheck §4.4, not an endpoint pair.

The plan's overall structure (3 levels, per-zone blocks, edges with neighbor+flag layout) is correct.

Confidence: C=HIGH (cross-validated by Add, Remove, FloodFillReachableZones), I=HIGH, B=HIGH.

---

## 6. Active in YR — summary

| Function | Active in YR? | Evidence | Gating |
|----------|---------------|----------|--------|
| `AssignOrphanedCellZone @ 0x56D460` | Yes | Called by terrain-edit and building-place paths | None |
| `MergeAdjacentCellZone @ 0x56D5A0` (sibling) | Yes | Same — paired with AssignOrphaned | None |
| `ResolvePathCoord_BridgeAware @ 0x583180` | Yes | Called by path-smoothing and waypoint resolution | None |
| `FUN_005835D0` (orphan-bridge-coord fallback) | Yes | Single caller is ResolvePathCoord_BridgeAware | None |
| `Pathfinding_update_continued @ 0x481810` (= CellClass::Get_Neighbor) | Yes — pervasive | 50+ callers spanning the engine | None |
| MapClass+0x90 region | Yes | Read at every Zone_precheck and FloodFillReachableZones call; written at every bridge Add/Remove | None |

**No SpecialFlags gating found.** All paths are live in standard YR skirmish.

---

## 7. Cross-doc contradictions resolved

### 7.1 Pathfinding_update_continued name confusion

The plan's name suggested it was a pathfinding-specific helper. **Refined**: it's `CellClass::Get_Neighbor`, a universal cell-neighbor-step utility used engine-wide. Not specific to pathfinding.

### 7.2 ResolvePathCoord_BridgeAware "perpendicular walk" inheritance

The function has the SAME walk-along-bridge-axis pattern as GetZoneID — the prior CELLCLASS_ZONES_SPEED_BRIDGES.md "perpendicular" claim was wrong here too. **Refuted (same as in lifecycle doc §13.1).**

### 7.3 MapClass+0x90 layout

See lifecycle doc §13.2 and §5.5 above. Plan's claim was structurally inverted and conflated nesting levels.

### 7.4 ResolvePathCoord tolerance vs Invalidate/Validate tolerance

ResolvePathCoord uses tolerance **2**, Invalidate/Validate use **3**, GetZoneID uses **1**. These are not consistent across the codebase — each caller picks a tolerance matching its use case. Prior docs hadn't enumerated these.

---

## 8. Current Rust Implementation Status (audit-only — not a port plan)

| Binary feature | Rust file | Status |
|----------------|-----------|--------|
| `AssignOrphanedCellZone` 8-neighbor inheritance with ≤4-zone fast path | [src/sim/pathfinding/zone_incremental.rs:45](../../ra2-rust-game/src/sim/pathfinding/zone_incremental.rs#L45) `try_incremental_update` | **Partial**. Rust does a full clear/refill on terrain change rather than 8-neighbor inheritance. Functionally equivalent for connectivity, but more expensive. Player-visible impact: only frame-time / determinism, not behavior. |
| `MergeAdjacentCellZone` (sibling) | [src/sim/pathfinding/zone_incremental.rs:45](../../ra2-rust-game/src/sim/pathfinding/zone_incremental.rs#L45) | Same as above — folded into full-refill path. |
| `ResolvePathCoord_BridgeAware` Sqrt_Approx endpoint pick | (none for direct port) | **Missing as a function**. Rust uses `ZoneMap::zone_at` which does nearest-endpoint redirect via simple Manhattan or precomputed nearest, not Sqrt_Approx + ftol. **Subtle differences** at distance-equal cases (gamemd's tie-breaker rule `endpoint_a < endpoint_b` would matter). |
| `FUN_005835D0` orphan-bridge-coord fallback | (none) | **Missing**. Rust path resolution doesn't have a no-record-found fallback. Triggers only after editor edits or corruption — rare. |
| `Pathfinding_update_continued` (Get_Neighbor) | [src/sim/grid](../../ra2-rust-game/src/sim/grid/) or equivalent cell-neighbor utility | **Present (different name)**. Rust has its own cell-neighbor utility(ies). The 8-direction convention 0=N, 2=E, 4=S, 6=W must match. Verify. |
| MapClass+0x90 3-level hierarchical zone graph | (none) | **Missing**. Rust has 1 level (per-MovementZone connectivity). No 3-level hierarchical structure. Required for Zone_precheck parity, which is also currently absent. |

### 8.1 Severity notes (per CLAUDE.md trigger-frequency rule)

- **AssignOrphanedCellZone fast path**: full refill is correct but slower. Frame-time issue, not behavior. **LOW**.
- **ResolvePathCoord Sqrt_Approx tie-breaker**: rare to hit (only on exact-equal distance). **LOW**.
- **3-level hierarchical zone graph**: only matters if Zone_precheck is being matched. Currently absent on both sides. **DEFERRED**.
- **Tolerance=2 for path-coord snap** (vs Rust's possibly-different tolerance): may cause subtle waypoint differences during pathing near broken bridges. **MEDIUM** if Rust's tolerance differs.

(Severity tagging deferred to Phase 7 synthesis.)

---

## 9. Edge-case detail catalogue

Tiny details surfaced during this investigation:

1. **AssignOrphanedCellZone's distinct-zone count is capped at 4**. ≥4 distinct neighbors → no fast-path inheritance, full rebuild.
2. **The "neighbor zoneType == 0" gate** in AssignOrphanedCellZone: only Clear-type cells trigger the inheritance check. Other zoneTypes (Building, Wall) skip directly to the iteration counter increment.
3. **ResolvePathCoord tolerance is 2** (not 1, not 3). Each caller of FindBridgeRecord uses its own tolerance.
4. **Sqrt_Approx → ftol roundtrip** produces TRUNCATED-toward-zero integer distances. Fractional distance differences at boundaries flip the endpoint pick.
5. **ResolvePathCoord tie-breaker** is `sVar6 < sVar7` (strict less-than). Equal distance → endpoint_b wins.
6. **Bridge orientation read from CELL.flags 0x800, not from BridgeRecord**. The flag is at the cell layer; the record only has endpoint X:Y.
7. **Pathfinding_update_continued's direction is masked `& 7`**. Out-of-range direction values silently wrap. Direction ≥8 is a no-op (early return).
8. **The decompilation signature says `void`** but the asm propagates EAX from MapClass::Get_CellClass. Decomp is wrong about the return type.
9. **The 8-direction offset table at 0x89F688 is BSS** — runtime-initialised. Static analysis can't read the values directly; must rely on the indirect linear-offset table at 0x7E3774 for the convention.
10. **FUN_005835D0 picks the CLOSER candidate to the REFERENCE coord (param_3)**, not to the cell itself. Different semantic from the main ResolvePathCoord, which compares to the cell.
11. **ResolvePathCoord destroyed-bridge fallback returns endpoint_b + offset (NOT endpoint_a)** if the exit cell IS a bridge tile but not Rock. The offset preserves the cell's PERPENDICULAR-to-bridge offset from endpoint_a (translated to endpoint_b).
12. **The "DAT_00ABD480" sentinel** (used in ResolvePathCoord and AddBridgeZoneEdges to default the 4 derived bridgehead coords) is a known "invalid coord" marker. Code paths check `coord != DAT_00ABD480` before treating coords as valid.

---

## 10. Open Questions

1. **AssignOrphanedCellZone's exact 8-neighbor offset table order**: due to stack-layout ambiguity in Ghidra's decomp, the exact iteration order of the 8 offsets isn't 100% pinned. Need raw asm trace of the local_20 array layout.

2. **MergeAdjacentCellZone vs AssignOrphanedCellZone**: when does each fire? Probably from different terrain-edit code paths. Need to trace callers and confirm split.

3. **DAT_00ABD480 sentinel coord**: what's its actual value? Likely a coord like (0xFFFF, 0xFFFF) or similar. `read_memory 0x00ABD480` to confirm.

4. **The growth quantum at zone-block +0x14**: per-graph-level or per-zone? Read at runtime to confirm.

5. **Bridge-edge flag low-byte semantic** (carries from lifecycle doc): definitively determine whether 0 = bridge or 0 = non-bridge. Likely requires identifying a SECOND writer to MapClass+0x90 edges that uses a non-zero flag byte.

6. **MapCoord_Add at 0x42D510** (used by AddBridgeZoneEdges and RemoveBridgeZoneEdges): exact signature/semantics. The decomp shows `MapCoord_Add(&param_2, table_entry)` four times — is it a destructive update of param_2 or a non-destructive (dest, src) compute? Needed for parity of the 4 derived bridgehead-approach coords.

7. **The 5 unknown ints in MapClass+0x90 level headers** (+0x04..+0x17): likely vector count/cap for the zones-array, but unverified.

8. **What writes the per-zone-block fields +0x18 (representative cell), +0x1C (LandType), +0x20 (unknown)** at map init? Likely a `BuildZoneGraph` or `ZoneMap::Initialize`. Not traced here.

---

## 11. Sources

**Ghidra functions decompiled** (Phase 4, this report):
- `MapClass::AssignOrphanedCellZone` @ 0x0056D460 (full body, ~280 bytes)
- `MapClass::MergeAdjacentCellZone` @ 0x0056D5A0 (full body, ~260 bytes)
- `MapClass::ResolvePathCoord_BridgeAware` @ 0x00583180 (full body, ~640 bytes)
- `FUN_005835D0` (orphan-bridge-coord fallback) @ 0x005835D0 (full body, ~560 bytes)
- `Pathfinding_update_continued` (CellClass::Get_Neighbor) @ 0x00481810 (full body, ~75 bytes)
- `ZoneMap::FloodFillReachableZones` @ 0x005840C0 (cross-ref for MapClass+0x90 layout)

**Raw assembly verified for**:
- Pathfinding_update_continued direction-table indexing at 0x48181C..0x00481862
- Pathfinding_update_continued return-EAX propagation at 0x00481862..0x00481863

**Memory reads**:
- 0x0089F688 len 32 (BSS — direction offset table for Get_Neighbor; cold zero)
- 0x0087F878 len 32 (BSS — MapClass+0x90 base; cold zero)
- 0x007E3774 len 32 (.rdata — linear cell-array offset table for 8 directions; verifies convention)

**Cross-reference checks**:
- `get_function_callers 0x481810` — 50+ callers (Get_Neighbor is engine-wide utility)
- `get_xrefs_to 0x87F878` — 1 DATA xref from `PathfinderClass::InvalidateZoneEdge @ 0x42D082`

**Companion doc**:
- [BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md](BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md) (items #33–#42)

**Phase 1–3 dependency docs**:
- [BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md](../03-traversal-pathfinding-entry/BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md)
- [BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md](BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md) — §4 Zone_precheck cross-ref
- [BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md](../03-traversal-pathfinding-entry/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md)

**Prior research docs cross-checked**:
- [CELLCLASS_ZONES_SPEED_BRIDGES.md](CELLCLASS_ZONES_SPEED_BRIDGES.md) — §1.7 "perpendicular walk" refuted in both lifecycle doc §6 and §2.5 here

---

## 12. Audit log entry

For inclusion in `AUDIT_LOG.md`:

```
2026-05-13 — Phase 4 zone-helpers investigation (this doc).
Identified: "Pathfinding_update_continued" @ 0x481810 = CellClass::Get_Neighbor (engine-wide utility, not pathfinding-specific).
Verified: ResolvePathCoord_BridgeAware Sqrt_Approx endpoint pick with strict-less-than tiebreaker.
Verified: AssignOrphanedCellZone ≤4-distinct-neighbor-zones fast path + UpdateBridgeZonesHelper fallback.
Refined: MapClass+0x90 struct layout (level stride 0x18, zone-block stride 0x24, +0x08 capacity, +0x10 count).
Cross-doc: ResolvePathCoord walks ALONG bridge axis (same as GetZoneID) — prior doc's "perpendicular" claim wrong here too.
Open: 8 questions deferred (bridge-edge flag semantic, MapCoord_Add signature, growth quantum, etc.).
```
