# CellClass::Reduce_Tiberium (FUN_00480A80) — Ghidra Research Report

**Address:** `0x00480A80`
**Confidence:** HIGH — full decompilation and disassembly verified directly via Ghidra MCP
**Active in YR:** Yes — standard `HARV`/`CMIN` reach the function through the
verified harvest path. Other callers are present in active code, but their
scenario-specific gates are not all proven by this report.
**Date:** 2026-05-20
**Last corrected:** 2026-07-24 against the active retail `gamemd.exe`

---

## 1. Overview

`CellClass::Reduce_Tiberium` removes ore density from a cell. If the reduction is partial
(amount < current_density + 1), only `OverlayData` is decremented. If the reduction empties
the cell, the overlay is fully removed, radar is dirtied, ALL tiberium types' spread bitmaps
are cleared, and the 8 valid neighbours are re-seeded into THIS tiberium type's spread queue
so the patch can grow back into the newly vacated cell.

---

## 2. Verified Decompilation — Annotated

```
uint CellClass::Reduce_Tiberium(int32_abi amount) // ECX = this, stack[+4] = 32-bit value
  // param_1 = this (CellClass*)
  // param_2 = amount (density levels to remove; signed guard/threshold semantics)

  // --- DIRTY-RECT SETUP ---
  // FUN_0047FDE0(local_20) — compute overlay sprite screen rect (for TacticalClass dirty)
  // FUN_0047FB90(local_10) — compute tile SHP screen rect
  // Both return int[4]: {left, top, width, height}
  // The combined dirty rect union is computed and passed to TacticalClass::DirtyScreenRect
  // at the very end (address 0x00480C89).
  // g_RadarViewportOffsetY subtracted from the Y coordinate at 0x00480B5D.

  // --- GUARD: bail fast if signed amount <= 0 or cell has no tiberium ---
  iVar5 = CellClass::OverlayToTiberiumIndex(this->OverlayTypeIndex)
           // = IsWallOverlay(cell->OverlayTypeIndex) at 0x005FDD20
  if ((int)amount <= 0 || iVar5 == -1)  return 0;
           // The check at 0x00480B77-0x00480B82:
           //   TEST EBX,EBX / JLE 0x480ca0   — signed <= 0 branch; returns 0 for signed-neg inputs.
           //   CMP EAX,-0x1 / JZ 0x480ca0    — bail if tibIdx == -1.

  // --- RESOLVE TIBERIUM CLASS POINTER ---
  TiberiumClass* tib = g_TiberiumClass_Array[tibIdx];
           // g_TiberiumClass_Array at 0x00B0F4EC (verified: MOV ECX,[0x00B0F4EC] at 0x480B88)

  // --- DENSITY-11 DETOUR ---
  if (this->OverlayData == 0x0B) {        // 0x0B = 11 = '\v' (verified at 0x00480B97: CMP AL,0xB)
    TiberiumClass::AddToGrowthQueue(&this->MapCoord);   // call at 0x00480BA1
    // For density 11 this call is a net no-op: AddToGrowthQueue rejects
    // density >= 11 at 0x007235C1..0x007235C8 before RNG or queue append.
    // The call exists, but it does not re-add this max-density cell.
  }

  // --- READ CURRENT DENSITY ---
  uint current = (uint)(byte)this->field_0x11E;   // CellClass+0x11E = OverlayData

  // --- PARTIAL REDUCTION PATH ---
  if ((int)amount < (int)(current + 1)) {          // signed comparison at 0x00480BB7-0x480BB9
    this->field_0x11E = this->field_0x11E - (char)amount;   // 0x00480BBD
    return amount;                                 // 0x00480BC3 → 0x00480C67 → return EBX
    // NOTE: OverlayData -= amount uses a 1-byte subtraction;
    //       amount is cast to char (signed 8-bit) before subtraction.
    //       For typical amounts (1, 6, etc.) this is equivalent to uint subtraction.
  }

  // --- FULL REMOVAL PATH ---
  this->OverlayTypeIndex = -1;          // 0x00480BCE: MOV [ESI+0x44], -1
  this->field_0x11E = 0;               // 0x00480BD5: MOV [ESI+0x11E], 0
  CellClass::RecalcAttributes(this);   // 0x00480BDC: CALL 0x0047D2B0
                                       // RecalcAttributes recomputes LandType from overlay
                                       // (now -1), resets LandType to terrain tile value.
                                       // Also handles slope removal logic (ore on steep cliff).

  // Density-zero edge: current==0 and any allowed positive amount reaches this
  // full-removal path because amount >= current+1. It performs every full
  // side effect below, but returns the pre-existing current value 0.

  // OverlayData already set to 0 BEFORE RecalcAttributes (0x480BD5 < 0x480BDC).
  // OverlayTypeIndex already set to -1 BEFORE RecalcAttributes (0x480BCE < 0x480BDC).

  RadarClass::MarkTerrainDirty(&this->MapCoord);   // 0x00480BEA: CALL 0x006551C0
                                                    // &this->MapCoord = ESI+0x24
                                                    // adds cell to radar dirty queue,
                                                    // sets RadarClass+0x14D9 = 1 (dirty flag)

  // NOTE: the address passed to RadarClass::MarkTerrainDirty is:
  //   ADD ESI,0x24 (0x480BE1) — ESI was the CellClass* this pointer.
  //   So &MapCoord_X (short at CellClass+0x24) is passed, not the full coord struct.

  TiberiumClass::ClearSpreadBitmaps_AllTypes();    // 0x00480BF1: CALL 0x00722AB0

  // --- 8-DIRECTION NEIGHBOR RESEED LOOP ---
  // Loop: uVar12 = 0 .. 7 (inclusive), 8 iterations total.
  // Direction table: g_DirectionOffsets at 0x0089F688 (runtime-initialized BSS).
  // Stride: each entry is 4 bytes (two int16: [dx, dy]).
  // Access: word ptr [EAX*4 + 0x89F688] for X, word ptr [EAX*4 + 0x89F68A] for Y.
  //         where EAX = uVar12 & 7 (always 0-7 since loop is 0-7 anyway).
  // Verified from disassembly at 0x00480BFA-0x00480C14.

  // Base cell for offsets: ESI+0x24 = this->MapCoord (after the ADD ESI,0x24 at 0x480BE1).
  //   ADD CX, word ptr [ESI]     => neighbor_X = dir_dx + this->MapCoord_X
  //   ADD DX, word ptr [ESI+0x2] => neighbor_Y = dir_dy + this->MapCoord_Y

  for (uint dir = 0; dir < 8; dir++) {
    CoordPair n = { this->MapCoord_X + dir_dx[dir],
                    this->MapCoord_Y + dir_dy[dir] };
    // in-bounds check at 0x00480C37: CALL Cell_in_bounds_check (0x00568300)
    if (!Cell_in_bounds(n)) continue;

    // Spread-bitmap dedup check at 0x00480C44-0x00480C53:
    int cellIdx = FUN_0042B1C0(&n);    // = cell linear index
    if (tib->SpreadBitmap[cellIdx] == 0) {    // CellClass+0xF8 = spread bitmap ptr
                                              // check: *(tib->field_0xF8 + cellIdx) == 0
      TiberiumClass::AddToSpreadQueue(&n);     // 0x00480C5C: CALL 0x00722AF0
      // AddToSpreadQueue also checks CanSpreadTiberium AND bitmap again internally,
      // but the outer bitmap check (0x480C4F) is still present as a first guard.
    }
    // NOTE: if bitmap[cellIdx] != 0, the neighbor is SKIPPED (not added).
    // ClearSpreadBitmaps_AllTypes() cleared only the REMOVED coordinate in
    // every type's bitmap. Neighbor bits remain unchanged, so this outer guard
    // preserves pre-existing neighbor deduplication.
  }

  // --- RETURN VALUE ---
  return current;    // = OverlayData before removal (0x00480C94: MOV EAX,EBX)
                     // (EBX was set to (uint)(byte)field_0x11E at 0x00480BC8)

  // --- TACTICAL DIRTY RECT ---
  // Emitted at 0x00480C89 for BOTH paths (partial and full removal).
  // TacticalClass::DirtyScreenRect(left, top, width, height, 0) at 0x00480C89.
```

---

## 3. CellClass Fields Touched — Complete List

| Field | Offset | Type | Access | Path |
|-------|--------|------|--------|------|
| OverlayTypeIndex | +0x44 | int | READ (guard + IsWallOverlay input) | Both paths |
| OverlayData (density) | +0x11E | byte | READ (current density) | Both paths |
| OverlayData (density) | +0x11E | byte | WRITE -= amount | Partial path only |
| OverlayTypeIndex | +0x44 | int | WRITE = -1 | Full removal only |
| OverlayData (density) | +0x11E | byte | WRITE = 0 | Full removal only |
| MapCoord_X | +0x24 | short | READ (neighbor calc, radar, spread) | Full removal only |
| MapCoord_Y | +0x26 | short | READ (neighbor calc, radar, spread) | Full removal only |

**CellClass+0x122 IS NOT TOUCHED anywhere in `Reduce_Tiberium`.** Confirmed by:
- Full decompilation read — no reference to `+0x122`.
- Full disassembly scan — no memory access at base+0x122 pattern.

CellClass+0x122 **is** decremented, but only in `CellClass::PostDestructionWallCleanup`
(decompiled at `0x00480838`), gated on `OverlayTypeClass.Wall (+0x2A8)`. It is a
**WallNeighborCount**, not an ore counter. Confirmed by direct read of
`*(char *)(iVar13 + 0x122) = *(char *)(iVar13 + 0x122) + -1` inside the wall-destroy path.

---

## 4. ClearSpreadBitmaps_AllTypes — Scope Verification

**Address:** `0x00722AB0` — verified via search_functions.

```c
void TiberiumClass::ClearSpreadBitmaps_AllTypes() {
    int cellIdx = FUN_0042B1C0(null?);   // this is called with NO argument — ECX-based
    for (int i = 0; i < g_TiberiumClass_Array_Count; i++) {
        TiberiumClass* t = g_TiberiumClass_Array[i];
        t->SpreadBitmap[cellIdx] = 0;    // *(t->field_0xF8 + cellIdx) = 0
    }
}
```

**Scope: ALL tiberium types**, not just the one matching this cell's overlay.
The loop iterates `0 .. g_TiberiumClass_Array_Count` (exclusive), clearing the
spread-bitmap entry at `cellIdx` for EVERY registered tiberium type.

**Critical detail:** Only the **single cell's entry** (at index `cellIdx`) is cleared
in each tiberium type's bitmap — NOT the entire bitmap. The `cellIdx` computed by
`FUN_0042B1C0` is the linear index of the specific cell being harvested.

So after `ClearSpreadBitmaps_AllTypes()`:
- For the removed cell's position: spread-bitmap entry = 0 in ALL tib types.
- All OTHER cells' bitmap entries: unchanged.

This means if the same cell was in the spread queue for multiple tib types,
all those entries are cleared simultaneously, then the neighbor re-seed loop
re-queues only into THIS tib's spread queue.

---

## 5. The 8-Direction Neighbor Loop — Full Algorithm

**Table:** `g_DirectionOffsets` at `0x0089F688` (BSS, runtime-initialized).
**Stride:** 4 bytes per entry = `{int16 dx, int16 dy}`.
**Loop:** `dir = 0` to `7` inclusive, in ascending order (not random, not shuffled).
**Verified from disassembly:**
- `0x00480BF8:` `MOV EAX,EDI` / `AND EAX,0x7` (dir index masking)
- `0x00480BFD:` `MOV CX, word ptr [EAX*4 + 0x89F688]` (dx)
- `0x00480C05:` `MOV DX, word ptr [EAX*4 + 0x89F68A]` (dy = dx_addr + 2)
- `0x00480C62:` `INC EDI` / `CMP EDI,0x8` / `JL 0x480BF8` (loop condition)

**Exact direction order:** the contiguous initializer at
`0x0049F2F0..0x0049F39B` writes the runtime table at `0x0089F688`.
Decoding each little-endian pair as signed `int16 {dx,dy}` gives indices
`0..7` exactly:

```text
(0,-1), (1,-1), (1,0), (1,1),
(0,1), (-1,1), (-1,0), (-1,-1)
```

The `.data` startup-function pointer table contains `0x0049F2F0` at
`0x00812BAC`. `Reduce_Tiberium` consumes these indices in ascending order, so
the verified traversal is `N, NE, E, SE, S, SW, W, NW`.

**In-bounds check:** `Cell_in_bounds_check` at `0x00568300`. Verified:
```c
// Checks that:
//   n.x + n.y > param_1->f4     (map width sentinel)
//   n.x - n.y < param_1->f4
//   n.y - n.x < param_1->f4
//   n.x + n.y <= param_1->f4 + param_1->f8 * 2
// Returns 0 (fail) or non-zero (pass).
// Skips neighbors that would wrap around the map edge.
```

**Spread-bitmap dedup before add:**
At `0x00480C44`: `CALL FUN_0042B1C0` — compute linear cell index for the neighbor coord.
At `0x00480C4F`: `CMP byte ptr [EAX + EDX*1], 0` — check `tib->SpreadBitmap[neighborIdx]`.
At `0x00480C53`: `JNZ 0x480C61` — skip if already in queue.

After `ClearSpreadBitmaps_AllTypes()`, all entries are 0, so this guard is initially
a no-op. But `TiberiumClass::AddToSpreadQueue` (`0x00722AF0`) sets the bitmap entry
internally. If two neighbours of the removed cell share a common third neighbour,
the outer check catches the duplicate on the second visit.

**Queue target:** `TiberiumClass::AddToSpreadQueue` is called on `EBP` (the TiberiumClass*
resolved from `g_TiberiumClass_Array[tibIdx]`). This is **this tib's spread queue** only —
the tib whose overlay was removed. Other tib types' spread queues are NOT fed.

**AddToSpreadQueue internal re-check:**
`TiberiumClass::AddToSpreadQueue` at `0x00722AF0` also calls `CellClass::CanSpreadTiberium`
AND re-checks the spread bitmap. So a cell can be skipped at three levels:
1. Outer bitmap check (before call, at 0x480C4F)
2. `CanSpreadTiberium` inside AddToSpreadQueue
3. Inner bitmap check inside AddToSpreadQueue

**No random starting direction.** The loop starts at dir=0 and increments to 7 in order.
The prior §11 pseudocode implied "8 directions" without specifying order — confirmed now
as sequential 0-7, same direction table as used elsewhere.

---

## 6. Density-11 Detour — Verified Detail

The comparison is `CMP AL, 0x0B` at `0x00480B97`.
`0x0B` = 11 decimal = `'\v'` (vertical tab — a Ghidra display artifact).
This is the maximum density value (MaxDensity - 1 = 12 - 1 = 11).

When `OverlayData == 11` before any reduction:
- Call `TiberiumClass::AddToGrowthQueue(&this->MapCoord)` — adds cell to GROWTH queue.
- This happens regardless of whether the reduction will be partial or full.
- The AddToGrowthQueue call at `0x007235A0` guards internally:
  `if (*(byte*)(cellClass + 0x11E) < 0xB)` — it only queues if density < 11.
  But since density IS 11 at this point and we haven't decremented yet, the guard
  would fail! Reading the AddToGrowthQueue decompilation: it checks `< 0xB` (< 11),
  and density == 11 fails this check.
  
  **Key detail:** AddToGrowthQueue at `0x007235A0` reads `*(byte*)(cellClass + 0x11E) < 0xB`
  BEFORE the decrement in Reduce_Tiberium. Since density is currently 11, the `< 11` check
  fails and the cell is NOT actually added to the growth queue at this point.
  
  **Correction:** Re-reading the decompilation of AddToGrowthQueue:
  ```c
  iVar4 = MapClass::Get_CellClass(param_2);
  if (*(byte*)(iVar4 + 0x11E) < 0xB) {  // < 11 check
  ```
  The address `param_2` is `&this->MapCoord` — GetCellClass looks up the cell by coords
  and returns the CellClass pointer, then reads its OverlayData. Since the cell's OverlayData
  has NOT been decremented yet at this point in Reduce_Tiberium, this check sees value 11
  and fails. The growth queue add is a no-op when called from the density-11 detour.
  
  **Conclusion:** The density-11 branch calls AddToGrowthQueue but the internal guard
  (`density < 11`) blocks the actual enqueue since density is still 11 at that moment.
  The intent appears to be ensuring the cell re-enters the growth queue after being
  reduced from max — but the re-queue only succeeds after the OverlayData is actually
  decremented, which happens in the PARTIAL path (not the full removal path).
  For the full removal path, the growth queue add is moot (cell is being deleted).

---

## 7. RecalcAttributes — What It Does Here

**Address:** `0x0047D2B0`

Called in the full removal path after:
1. `OverlayTypeIndex = -1` (already cleared)
2. `OverlayData = 0` (already cleared)

RecalcAttributes with no overlay present recomputes `LandType` from the terrain tile
(`IsoTileTypeIndex`), resets `SlopeIndex`, clears the "has tiberium" state, and
calls `RecalcZoneType`.

The earlier claim that `RecalcAttributes` indirectly clears a
`CellClass+0x140` bit `0x80` is **UNVERIFIED**. A live search of the
`0x0047D2B0` body found no such clear/set operation. Do not use that field claim
as an implementation requirement without tracing a separate callee.

**The verified parity-critical outcome:** after a cell is fully harvested,
`RecalcAttributes` immediately recomputes slope/LAT state, reads the underlying
tile's `LandType` through `0x00544BE0` when `OverlayTypeIndex == -1`, calls
`RecalcZoneType`, and writes the zone caches before radar dirtying.

---

## 8. RadarClass::MarkTerrainDirty — Verified

**Address:** `0x006551C0` — verified via search_functions returning `RadarClass__MarkTerrainDirty`.

The function:
1. Deduplicates: scans existing dirty-list entries to avoid double-adding.
2. Appends `{MapCoord_X, MapCoord_Y}` to a dirty list at `RadarClass+0x1228`.
3. Sets `RadarClass+0x14D9 = 1` (minimap needs redraw).

The address passed is `ESI + 0x24` (after `ADD ESI,0x24` at 0x480BE1), which is a pointer
to the `MapCoord_X` short at `CellClass+0x24`. This is a `short*` pointing to a 4-byte
region `{MapCoord_X, MapCoord_Y}` — used as an `undefined4` coord pair.

---

## 9. Caller Table — All Verified

| Caller | Address | Context | amount passed |
|--------|---------|---------|---------------|
| `UnitClass::Harvest_Ore_Tick` | `0x0073D450` | Standard ore harvester (War Miner, Chrono Miner) | computed from capacity/density |
| `FUN_00522E70` (Slave mine tick) | `0x00522E70` | Slave mine ore harvesting | computed |
| `MapClass::ReduceTiberiumInRadius` | `0x0057B790` | AoE ore reduction (radius 5, square) | hardcoded param |
| `Apply_area_damage` | `0x00489280` | Warhead ore destruction — gated on `OverlayTypeClass+0x2B1` (Explodes/Tiberium flag) AND warhead `+0x148` flag | caller-computed |
| `AnimClass::Middle` | `0x00424CE0` | Tiberium chain reaction anim — `TiberiumChainReaction=yes` on anim, passes `(byte)cell->field_0x11E + 1` (full current density +1 = guaranteed full removal) | `current + 1` |
| `AnimClass::Start` | `0x00424F00` | Scorch/crater animation on ore cell — `Scorch=yes` on AnimTypeClass | hardcoded `6` |
| `BuildingClass::ExtendWallInDirection` | `0x00452DC0` | Wall extension — reduces ore to make room | unknown |

**Standard-YR activation proven here:** stock `HARV`/`CMIN` use the first
caller. The remaining caller bodies/xrefs are verified, but this report does
not prove that every scenario-specific gate fires in every standard skirmish.
For stock standard harvest success, the next helper eligibility is exactly 19
binary-frame numbers later on the ordinary live-object path: the ninth
two-frame StepTimer increment is written after mission dispatch at `F+18`, and
the mission first observes it at `F+19`. Physical-arrival latency is not fixed
because state 1/timer can start while the miner is still traveling.

---

## 10. OverlayToTiberiumIndex / IsWallOverlay Verification

**Address:** `0x005FDD20` — confirmed by search_functions returning `CellClass__OverlayToTiberiumIndex`.

Despite the misleading name `IsWallOverlay`, this function:
1. Returns -1 if `OverlayTypeIndex == -1`.
2. Returns -1 if `OverlayTypeClass+0x2A9` (Tiberium bool) is false.
3. Iterates TiberiumClass array, checks primary range `[firstIdx, firstIdx+NumImages)`
   and extra range `[firstIdx+NumImages, firstIdx+NumImages+NumExtraImages)`.
4. Returns `TiberiumClass->ArrayIndex (+0x98)` on match.
5. Logs warning "Overlay %s not really tiberium" and returns 0 (not -1!) if no match.

**Return value 0 vs -1:** The fallthrough case returns **0** (not -1). This means an
overlay marked `Tiberium=yes` but not in any TiberiumClass range would be treated as
tiberium index 0 (Riparius) by Reduce_Tiberium. The guard in Reduce_Tiberium only
checks `iVar5 != -1`, so this fallthrough does NOT bail out — it proceeds with tib index 0.

---

## 11. Discrepancies vs §11 Pseudocode in ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md

| Claim in §11 | Verified status | Detail |
|--------------|-----------------|--------|
| Partial reduction: `cell->OverlayData -= amount` | CONFIRMED | Byte subtraction, amount cast to `char` |
| Partial returns `amount` | CONFIRMED | Returns `EBX = amount` |
| Full: `OverlayTypeIndex = -1, OverlayData = 0` | CONFIRMED | Both writes before RecalcAttributes |
| Full: RecalcAttributes called | CONFIRMED | Call at 0x480BDC |
| Full: RadarClass::MarkTerrainDirty called | CONFIRMED | Call at 0x480BEA |
| Full: `TiberiumClass::ClearSpreadBitmaps_AllTypes()` — ALL types | CONFIRMED | Loop over all tib types at 0x00722AB0 |
| Full: 8-neighbor loop, add each valid neighbor to THIS tib's spread queue | CONFIRMED | Loop 0..7, calls AddToSpreadQueue on `tib` (this cell's tib pointer) |
| `CellClass+0x122` (wall-neighbor counter) NOT touched | CONFIRMED | No access to +0x122 in Reduce_Tiberium; the decrement of +0x122 is in PostDestructionWallCleanup, wall-only path |
| Density-11 detour: "re-add to growth queue before reducing" | PARTIAL CORRECTION | Called BUT AddToGrowthQueue's internal `< 11` guard blocks enqueue since density is still 11. Net effect: no-op for the growth queue at that moment |
| `amount < current + 1` guard | CONFIRMED | Signed comparison `JLE` at 0x480BB9 |
| Spread-bitmap dedup before neighbor add | CONFIRMED | `CMP byte ptr [EAX + EDX*1], 0` at 0x480C4F |

---

## 12. Open Questions — Final State

- `[RESOLVED] Q1 — Does Reduce_Tiberium touch CellClass+0x122?`
  → NO. Confirmed by full disassembly scan. +0x122 is decremented only in
  PostDestructionWallCleanup, wall-only path. (evidence: disassemble 0x00480A80, decompile 0x00480838)

- `[RESOLVED] Q2 — Is ClearSpreadBitmaps_AllTypes clearing ALL types or just the matching tib?`
  → ALL types. Loop at 0x00722AB0 iterates `g_TiberiumClass_Array_Count` entries.
  (evidence: decompile 0x00722AB0)

- `[RESOLVED] Q3 — Is the 8-neighbor loop seeding THIS tib's queue or ALL tib queues?`
  → THIS tib only. AddToSpreadQueue called with `EBP` = tib pointer for current cell's tib.
  (evidence: disassemble 0x00480A80 at 0x480C59: `MOV ECX,EBP; PUSH EAX; CALL 0x00722AF0`)

- `[RESOLVED] Q4 — What is the direction order for the 8-neighbor loop?`
  → Sequential dir=0..7, from g_DirectionOffsets table at 0x89F688. No randomization.
  (evidence: disassemble 0x00480A80 at 0x480BFA-0x480C65)

- `[RESOLVED] Q5 — Is there a spread-bitmap dedup check before AddToSpreadQueue?`
  → Yes. Outer check at 0x480C4F; AddToSpreadQueue also has internal re-check.
  (evidence: disassemble 0x00480A80)

- `[RESOLVED] Q6 — What does the density-11 detour actually do?`
  → Calls AddToGrowthQueue but the internal `density < 11` guard makes it a no-op
  when density is still 11 (not yet decremented). (evidence: decompile 0x007235A0)

- `[RESOLVED] Q7 — What are all callers of Reduce_Tiberium?`
  → 7 callers: UnitClass::Harvest_Ore_Tick, FUN_00522E70 (slave mine), MapClass::ReduceTiberiumInRadius,
  Apply_area_damage, AnimClass::Middle, AnimClass::Start, BuildingClass::ExtendWallInDirection.
  (evidence: get_function_callers 0x00480A80)

- `[RESOLVED] Q8 — Does OverlayToTiberiumIndex return 0 or -1 as fallback?`
  → Returns 0 (not -1) when overlay is Tiberium=yes but not in any TiberiumClass range.
  The Reduce_Tiberium guard only checks != -1, so index-0 (Riparius) is used as fallback.
  (evidence: decompile 0x005FDD20)

- `[RESOLVED] Q9 — Is CellClass+0x44 cleared before or after RecalcAttributes?`
  → BEFORE. Write at 0x480BCE, RecalcAttributes call at 0x480BDC. (evidence: disassemble 0x00480A80)

- `[RESOLVED] Q10 — Is the amount comparison signed or unsigned?`
  → `if (amount < current + 1)` uses `JLE` (signed ≤). The bail-out guard uses `JLE 0x480CA0`
  for `amount <= 0` — treats amount as signed. A negative amount fails that
  signed-positive guard directly. (evidence: disassemble 0x00480A80 at 0x480B77)

- `[RESOLVED] Q10b — What happens for OverlayData 0 and a positive amount?`
  → `current+1` is 1, so every positive amount takes the full-removal path:
  overlay type/data clear, attributes recalculate, radar dirties, spread
  bitmaps clear/reseed, and tactical dirties. The function returns the
  pre-existing current value, which is 0. (evidence: disassemble
  `0x00480BA6..0x00480C9A`)

- `[RESOLVED] Q11 — Exact dx/dy values for g_DirectionOffsets 8-direction table.`
  → Initializer `0x0049F2F0..0x0049F39B` writes signed-int16 pairs at
  `0x0089F688` in reducer index order:
  `(0,-1),(1,-1),(1,0),(1,1),(0,1),(-1,1),(-1,0),(-1,-1)`.
  Its startup pointer is stored at `0x00812BAC`.
  (evidence: `search_instructions` for `89f688`;
  `disassemble_bytes 0x0049F2F0..0x0049F3C0`;
  `search_byte_patterns "F0 F2 49 00"`; `read_memory 0x00812B80`)

- `[DEFERRED] Q12 — BuildingClass::ExtendWallInDirection — what amount does it pass?`
  Category: out-of-scope; reason: wall extension is a separate system; ore reduction
  is a side-effect to clear the overlay before placing a wall. Amount likely 999 or
  OverlayData+1 (guaranteed full removal).

---

## 13. Sources

### Ghidra MCP calls (all in this session)
- `decompile_function 0x00480A80` — CellClass::Reduce_Tiberium (primary target)
- `disassemble_function 0x00480A80` — full disassembly for field-offset verification
- `decompile_function 0x00722AB0` — TiberiumClass::ClearSpreadBitmaps_AllTypes
- `decompile_function 0x007235A0` — TiberiumClass::AddToGrowthQueue
- `decompile_function 0x00722AF0` — TiberiumClass::AddToSpreadQueue
- `decompile_function 0x005FDD20` — CellClass::OverlayToTiberiumIndex (IsWallOverlay)
- `decompile_function 0x00568300` — Cell_in_bounds_check
- `decompile_function 0x006551C0` — RadarClass::MarkTerrainDirty
- `decompile_function 0x00424CE0` — AnimClass::Middle (caller)
- `decompile_function 0x00424F00` — AnimClass::Start (caller)
- `decompile_function 0x00489280` — Apply_area_damage (caller)
- `decompile_function 0x0057B790` — MapClass::ReduceTiberiumInRadius (caller)
- `decompile_function 0x00522E70` — FUN_00522E70 (slave mine caller)
- `decompile_function 0x00480838` — CellClass::PostDestructionWallCleanup (confirms +0x122)
- `decompile_function 0x0042B1C0` — FUN_0042B1C0 (cell linear index formula)
- `get_function_callers 0x00480A80` — caller table
- `get_xrefs_to 0x007235A0` — AddToGrowthQueue call sites
- `get_xrefs_to 0x00722AF0` — AddToSpreadQueue call sites
- `get_xrefs_to 0x0089F688` — g_DirectionOffsets usage sites
- `search_functions ClearSpreadBitmap` — found 0x00722AB0
- `search_functions OverlayToTiberiumIndex` — found 0x005FDD20
- `search_functions Cell_in_bounds` — found 0x00568300
- `read_memory 0x0089F688` — confirmed BSS (all zeros at static read)

### 2026-07-24 correction calls

- `list_open_programs` and `list_instances` — sole connected program is the
  retail `gamemd.exe` at
  `<ra2-install>/gamemd.exe`,
  `x86:LE:32:default`, image base `0x00400000`
- `search_instructions` with operand `89f688` — initializer write at
  `0x0049F305` and reducer reads at `0x00480BFD`/`0x00480C14`
- `disassemble_bytes 0x0049F2F0..0x0049F3C0` — complete direction-table
  initializer and exact signed-int16 pair writes
- `search_byte_patterns "F0 F2 49 00"` plus
  `read_memory 0x00812B80` — startup pointer at `0x00812BAC`
- `batch_decompile 0x00722AB0,0x00722AF0` — removed-coordinate-only bitmap
  clearing and preserved neighbor-bit deduplication
- `batch_decompile 0x0047D2B0`, then `search_instructions` in that function for
  operands `0x140` and `0x80` — verified immediate slope/LAT, underlying
  `LandType`, zone recalculation/cache effects and disproved the prior
  `+0x140 bit 0x80` assertion

### Docs referenced
- `ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md` — §11 pseudocode being verified
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md` — field offsets cross-reference
- `TIBERIUM_QUEUE_SEEDING_AND_TIMING_REPORT.md` — queue seeding context
- `CELLSPREAD_OFFSET_TABLE_DUMP_GHIDRA_REPORT.md` — direction table context (different table)
