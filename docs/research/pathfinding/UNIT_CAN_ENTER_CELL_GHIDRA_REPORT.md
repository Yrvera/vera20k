# UnitClass::Can_Enter_Cell — Ghidra Decompilation Report

**Address:** `0x0073F0A0` (gamemd.exe)
**Size:** 3238 bytes (0x73F0A0 - 0x73FD45)
**Signature:** `int __thiscall UnitClass__Can_Enter_Cell(CellClass* cell, int facing, int targetHeight, ??? param5)`
**Returns:** int 0-7 (passability code)
**Research date:** 2026-03-23
**Source:** Ghidra MCP live decompilation, 466 lines fully paginated
**Confidence:** HIGH (cross-referenced with disassembly and existing docs)

> **Corrections (2026-05-13, supersedes 2026-05-12):** the prior 2026-05-12 note said "`vtable[0x1B0]` is `CheckBridgeTraversal`, not `TechnoClass::Can_Enter_Cell`." That slot identity is correct, but the framing was **incomplete** — it implied vtable+0x1B0 is the A* entry, which it is NOT. The Phase 2 investigation (`docs/plans/2026-05-13-bridge-pathfinding-locomotion-investigation-plan.md`) clarifies:
>
> | Vtable slot | Role | Where called from |
> |-------------|------|-------------------|
> | **+0x1AC** | **A* Can_Enter_Cell entry** (returns code 0–7) — THIS is what `AStar_main_loop @ 0x429F54` dispatches to via `CALL [EDX+0x1AC]` | `AStar_main_loop` |
> | **+0x1B0** | **Bridge traversal sub-check** (`CheckBridgeTraversal @ 0x4D9C60`, returns 0/7, may update `path_height` via out-param) | called from inside the +0x1AC handler |
>
> **`UnitClass::Can_Enter_Cell @ 0x73F0A0` (the subject of THIS doc) lives at UnitClass vtable+0x1AC** (slot `0x7F5E1C` — verified by `read_memory`, with UnitClass vtable base `0x7F5C70` from destructor write at `0x735794`). It is NOT at +0x1B0. Per-class A* entry table:
>
> | Class | vtable+0x1AC (A* entry) | vtable+0x1B0 (bridge sub-check) |
> |-------|------------------------|---------------------------------|
> | UnitClass | `0x73F0A0` UnitClass::Can_Enter_Cell (this doc) | `0x4D9C60` CheckBridgeTraversal |
> | InfantryClass | `0x51BF90` (FUN_0051BF90, unlabeled — InfantryClass-specific A* entry, ~2.3 KB) | `0x4D9C60` CheckBridgeTraversal |
> | AircraftClass | `0x415B10` (8-direction landing-pad scanner — semantically NOT a per-cell predicate) | `0x5F4B10` **ObjectClass::DrawIt** (inherited; aircraft have NO bridge sub-check) |
> | BuildingClass | `0x449440` (FUN_00449440, unlabeled — returns ONLY 0/7) | `0x4264D0` `AnimClass__Click_stub` (shared return-0 stub) |
> | FootClass (base) | `0x4D9C10` FootClass::LocomotorPassabilityCheck (thin) | `0x4D9C60` CheckBridgeTraversal |
>
> Full Phase 2 details and binding evidence: [BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md](../bridges/03-traversal-pathfinding-entry/BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md) and [BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md](../bridges/03-traversal-pathfinding-entry/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md).
>
> The earlier 2026-05-12 correction note (preserved below for traceability) referenced `G6_TWO_PASS_DIVERGENCE_SUPPLEMENT.md §1` for the original cross-vtable check; that supplement covers 4 classes; the Phase 2 doc adds BuildingClass and FootClass and verifies the +0x1AC / +0x1B0 split.
>
> **Original 2026-05-12 correction (retained):**
> - **`vtable[0x1B0]` is `CheckBridgeTraversal` (0x4D9C60), NOT `TechnoClass::Can_Enter_Cell`.** The Phase 5 framing as "parent class virtual" is wrong — it's a small height-diff legality gate. See §1 of [G6_TWO_PASS_DIVERGENCE_SUPPLEMENT.md](../G6_TWO_PASS_DIVERGENCE_SUPPLEMENT.md) for live Ghidra verification across 4 class vtables. Phase 5 section below is rewritten accordingly.
> - **`FUN_004d9c10` at Phase 8 is `FootClass__LocomotorPassabilityCheck`** (Ghidra label). Located 0x50 bytes before `CheckBridgeTraversal` — easy to confuse.
> - **The 4th parameter is `targetHeight` (an in/out i32), NOT `prevFacing`.** Older pseudocode used `prevFacing`; current corrected sections use `targetHeight`. The value is a height (compared to `cell.Level + 4` in the bridge-deck check) — never a direction code.
> - **`CellClass+0x11B` is `Level` (signed i8, terrain height), NOT `Height`.** Older pseudocode used `Height_0x11B`; read any remaining historical mention as `Level_0x11B`. `CellClass+0x11A` is the tile-sub-type byte (dual-semantic: terrain tile-sub for normal cells, tube direction for tube cells), NOT "HeightLevel". `CellClass+0x11C` is `SlopeIndex`. Field Offsets table at bottom of this doc has the corrected entries.
> - The body pseudocode in §Phases 1-4, 6, 9-12 was NOT mechanically rewritten — too invasive. Read it with the renames above in mind.

---

## Overview

This is the central passability function for all vehicle/unit movement. It determines
whether a `UnitClass` can enter a target `CellClass` and returns one of 8 codes (0-7)
that the locomotor dispatches on.

The function is called by `DriveLocomotionClass::Process_Movement` (0x4B2630),
`ShipLocomotionClass::Process_Movement`, and the pathfinder. The `this` pointer is the
moving unit (`UnitClass*`, referred to as `self` below), and `param_2` (`cell`) is the
target cell. `param_3` is the movement facing (0-7, or 8 for tunnel entry), `param_4`
is the in/out target height (-1 if none). (corrected 2026-06-01: was "previous facing";
binary passes `&param_4` into `CheckBridgeTraversal` and later compares it to
`CellClass+0x11B + 4` via `decompile_function 0x0073F0A0` - PARAM1_TYPE_MISREAD)

### Return Code Summary

| Code | Name | Meaning | Locomotor Response |
|------|------|---------|-------------------|
| 0 | OK / Passable | Cell is free to enter | Proceed with movement |
| 1 | Special blockage | Neutral object blocking (e.g. civilian) | Scatter blocker |
| 2 | Temporarily blocked | Moving friendly or crushable ally present | Wait + repath |
| 3 | Scatter required | Friendly stationary unit | Bump/scatter it |
| 4 | Friendly unit blocking | Friendly unit/wall on cell | Try scatter or wait |
| 5 | Enemy unit blocking | Enemy unit/wall/building | Attack while waiting |
| 6 | FriendlyStationary | Allied non-building stationary object (set in friendly-mover handler when object type != Building) | Soft-block, route around (cost 8.0) |
| 7 | Impassable | Cannot enter at all | Full stop, clear path |

---

## Parameters and Local State

```
self          = UnitClass* (this, ECX)
cell          = CellClass* (param_2, [ESP+0x88] at entry, later [ESP+0x94] after pushes)
facing        = int param_3 (0-7 direction, or 8 for tunnel, or -1 for none)
targetHeight  = int param_4 (in/out height sentinel/value, -1 if none; passed by address to `CheckBridgeTraversal`)
param5        = ??? (passed through to parent call)
```

Key local variables:
- `isBridgeCell` (byte at [ESP+0x13]): Set if cell has bridge AND (`targetHeight` is -1 OR `abs(targetHeight - cell.Level_0x11B) >= 2`)
- `occupancyBits` (byte at [ESP+0x14]): Low byte of `CellClass+0x124` (ground occupancy bitfield)
- `hasUnitOnCell` (byte at [ESP+0x15]): Bit 5 of `CellClass+0x124` >> 5 (vehicle occupying)
- `hasFriendlyMoving` (byte at [ESP+0x16]): Set if blocker is moving (has locomotor + IsMoving)
- `crushCandidate` (byte at [ESP+0x17]): Set when a crushable enemy is found
- `resultCode` (EBP/[ESP+0x18]): Running result code, starts at return of parent call
- `piVar15` (ESI in loop): Current object being examined in cell's object linked list

---

## Execution Flow — Phase by Phase

### Phase 1: Bridge & Height Pre-checks (lines 1-30)

**Bridge cell detection:**
```c
if ((cell->Flags_0x140 & 0x100) == 0) {      // Not a bridge cell
    isBridgeCell = false;
} else if (targetHeight != -1) {
    int heightDiff = abs(targetHeight - cell->Level_0x11B);
    isBridgeCell = (heightDiff >= 2);         // Significant height change = bridge
} else {
    isBridgeCell = true;                      // No target height on bridge = bridge
}
```
(corrected 2026-06-01: was still written as `prevFacing`/`Height_0x11B`; binary shows `param_4` is the in/out target-height value and compares it to signed `CellClass+0x11B` via `decompile_function 0x0073F0A0` and `decompile_function 0x004D9C60` - PARAM1_TYPE_MISREAD)

**Read cell occupancy:**
```c
occupancyBits = cell->Occupancy_0x124 & 0xFF;   // Infantry sub-cell bits
hasUnitOnCell = (cell->Occupancy_0x124 >> 5) & 1; // Vehicle present flag
```

### Phase 2: Tunnel System Check (lines 30-62)

```c
TubeClass* tube = CellClass__GetTubeAtCell(cell);  // 0x00484F20
```

**Tunnel locomotor compatibility:**
If `self->TechnoTypeClass[0x1B1]+0xDFC != -1` (unit has a tunnel locomotor index):
```c
LandType landType = cell->LandType_0xEC;
if (landType == 10) {                         // LandType::Tunnel
    IsometricTileType* tileType = g_TileTypeArray[cell->TileTypeIndex_0x38];
    int rampType = tileType->RampType_0x2E4;
    int rampDir  = tileType->RampDir_0x2E8;

    // Ramp direction must match cell height level for entry
    if ((rampType == 5 && rampDir == 3) || (rampType == 4 && rampDir == 3)) {
        if (cell->TubeSubType_0x11A != 2) return 7;  // Wrong tube subtype for ramp
    }
    else if (rampType == 3 && (rampDir == 4 || rampDir == 5)) {
        if (cell->TubeSubType_0x11A != 6) return 7;  // Wrong tube subtype for ramp
    }
}

// If land type doesn't match our tunnel type AND isn't the universal Tunnel(10),
// AND overlay index is in range [0xED, 0xEE] where targetHeight must differ from Level:
if (landType != self->tunnelType && landType != 10) {
    if (cell->OverlayIndex_0x44 >= 0xED && cell->OverlayIndex_0x44 <= 0xEE
        && targetHeight != cell->Level_0x11B) {
        // Overlay tunnel exception = allow
    } else {
        return 7;   // IMPASSABLE — wrong tunnel type
    }
}
```
(corrected 2026-06-01: was `prevFacing == cell->Height_0x11B`; binary returns 7 when overlay is outside `[0xED,0xEE]` OR `param_4 == *(char *)(cell+0x11B)`, so the overlay exception requires `targetHeight != Level` via `decompile_function 0x0073F0A0` - OPERATOR_OR_ORDER_DRIFT)
(corrected 2026-06-01: ramp checks in this block were still named `HeightLevel_0x11A`; binary reads raw `*(char *)(cell+0x11A)` only as the tube/tile subtype sentinel values 2 or 6 under `LandType==10`, not terrain height, via `decompile_function 0x0073F0A0` - OFFSET_RETYPED_WRONG)

### Phase 3: Tunnel Entry Direction (lines 62-70)

```c
if (facing == 8) {   // Special "tunnel entry" facing
    if (tube == NULL) return 7;
    if (tube->Entry_0x28 == 0 && tube->Exit_0x2A == 0) return 7;  // Dead tube
    return 0;        // OK — can enter tube
}
```

### Phase 4: Tube Direction Mismatch (lines 70-110)

Two tube checks: once for the raw facing, once for `(facing - 4) & 7` (reverse facing).
Both check if the tube exists and if the direction difference from the tube's direction
(`tube->Direction_0x2C`) is in range [3, 5] (i.e., roughly perpendicular or opposing):

```c
if (tube != NULL) {
    int diff = abs(facing - tube->Direction_0x2C);
    if (diff > 2 && diff < 6 && facing != -1) return 7;  // Opposing tube direction
}
// Same check for the adjacent cell's tube (via (facing-4)&7 offset)
adjustedFacing = (facing - 4) & 7;
MapCoord_StepByDir_GetCell(cell, adjustedFacing);  // Gets adjacent cell
adjacentTube = CellClass__GetTubeAtCell();
if (adjacentTube != NULL) {
    int diff = abs(adjustedFacing - adjacentTube->Direction_0x2C);
    if (diff > 2 && diff < 6 && facing != -1) return 7;
}
```
(corrected 2026-06-01: was stale `Pathfinding_update_continued`; current binary label/body shows `MapCoord_StepByDir_GetCell`, a cell-step helper that adds `g_DirectionOffsets[dir]` then calls `MapClass__Get_CellClass`, via `decompile_function 0x00481810` - RTTI_LABEL_DRIFT)

### Phase 5: Vtable Bridge-Traversal Gate (line 113)

> **Correction (2026-05-12):** The original claim in this section — that
> `vtable[0x1B0]` is `TechnoClass::Can_Enter_Cell` (parent class) — is WRONG.
> Direct Ghidra MCP verification confirms `vtable[0x1B0]` of the ground moving
> classes that use the bridge sub-check (UnitClass, FootClass, InfantryClass)
> holds `CheckBridgeTraversal` (0x4D9C60), not a parent virtual.
> AircraftClass is the explicit exception: `vtable+0x1B0` is
> `ObjectClass::DrawIt` (0x5F4B10), so aircraft have no bridge sub-check.
> Details and proof:
> [BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md](../bridges/03-traversal-pathfinding-entry/BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md).

```c
resultCode = self->vtable[0x1B0](cell, facing, &targetHeight, &bridgePassFlag);
                                  // CheckBridgeTraversal (0x4D9C60)
if (resultCode == 7) return 7;
```

**vtable+0x1B0** is `CheckBridgeTraversal` — a small height-diff legality gate
(diff-0 / diff-1-with-slope / diff-4-bridge-entry permits, else return 7). It
takes the cell being entered as its "src cell" and computes a "dst cell" from
the reverse facing offset; the diff arithmetic is between those two cells'
Level bytes (+0x11B). The function ignores its `this` pointer despite being
registered as a vtable entry (thiscall ABI with unused ECX). It can write
`*targetHeight = parent.Level + 4` (the side-effect in CBT's normal-mode entry
when targetHeight was -1 AND parent has bridge flag 0x100) and `*bridgePassFlag = 1`
(the diff-4-going-up branch when entering a bridgehead from below). Both
writes feed downstream — Phase 6's bit-overwrite reads the updated targetHeight,
and Phase 10's list-selection reads the updated bridgePassFlag.

This call site is Step 3 of the "two-pass `Can_Enter_Cell`" mechanism
analyzed in [BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md §3.2](../bridges/00-system-models/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md).
There is no recursion into a parent `Can_Enter_Cell`; the parent-virtual
framing in the older draft was a misreading.

### Phase 6: Bridge Cell Re-read (lines 118-128)

If `targetHeight != -1` AND the target cell has a bridge (`flags & 0x100`), AND
`targetHeight == cell->Level_0x11B + 4`:

```c
// Re-read occupancy from bridge level instead of ground
occupancyBits = cell_0x128 & 0xFF;      // Bridge-level infantry bits
hasUnitOnCell = (cell_0x128 >> 5) & 1;  // Bridge-level vehicle flag
```

The `+4` check determines if movement is transitioning onto the bridge deck
(height level for bridge = ground height + 4).
(corrected 2026-06-01: was `prevFacing == cell->Height_0x11B + 4`; binary reuses the in/out `param_4` target-height local after `CheckBridgeTraversal` and compares it to signed `CellClass+0x11B + 4` via `decompile_function 0x0073F0A0` - PARAM1_TYPE_MISREAD)

### Phase 7: Fog of War / Shroud Check (lines 128-138)

```c
if (g_MapEditorMode == 0                           // Not in editor
    && !TechnoClass__IsOnScreen(cell, 1)           // Cell not revealed
    && !self->vtable[0x320]()                      // FUN_004DA1D0: fog-passability gate
    && self->field_0x3D5 != 0) {                   // Unit requires revealed cells
    return 7;
}
```

**vtable+0x320** dispatches to `FUN_004DA1D0` (unlabelled). This function returns 1 when the unit
should be allowed past the fog check: it returns 0 immediately if `field_0x3D5 == 0` (unit doesn't
require revealed cells at all), and otherwise returns 1 for JumpJet units, units with certain
locomotor flags, or units not on Mission_Attack. It is NOT a simple "IsPlayerControlled" check —
it combines the RequiresRevealedCells flag with jump-jet and mission state.
(corrected 2026-05-28: was described as "likely returns whether the unit is player-controlled
(allowing AI units to pathfind through fog)"; binary at vtable+0x320 slot 0x7F5F90 = 0x4DA1D0
which is NOT HouseClass__IsPlayerControl — it checks field_0x3D5 / IsJumpJet / mission state.
ROOT_CAUSE: INFERENCE_HARDENED; verified via read_memory 0x7F5F90 + decompile_function 0x4DA1D0)

### Phase 8: Locomotor Passability Check (lines 138-145)

```c
resultCode = FootClass__LocomotorPassabilityCheck(cell, facing, targetHeight, param5);
if (resultCode == 7) return 7;
```

**Ghidra label (verified 2026-05-12):** `0x4D9C10` is
`FootClass__LocomotorPassabilityCheck`, not the earlier guess `FUN_004d9c10`.
Located 0x50 bytes before `CheckBridgeTraversal` (0x4D9C60) in the same file;
the two are easy to confuse. The function dispatches to the unit's locomotor
COM interface (`self+0x674`) via the locomotor's vtable to ask whether the
mover can traverse the destination cell's terrain. Different from
`CheckBridgeTraversal` at Phase 5 — that one was the height-diff legality
check; this one is the locomotor-type-vs-terrain check.

### Phase 9: Overlay Check (lines 145-175)

```c
if (cell->OverlayIndex_0x44 != -1) {
    OverlayTypeClass* ovType = g_OverlayTypeArray[cell->OverlayIndex_0x44];

    // 9a: Crate/special overlay check
    if (ovType->Flag_0x2AA != 0) {                    // Flag at OverlayTypeClass+0x2AA
        if (!HouseClass__IsPlayerControl(self->Owner)  // Not player-controlled house?
            && g_GameMode == 0) {                      // Single-player only
            return 7;
        }
    }

    // 9b: Wall overlay check
    if (ovType->IsWall_0x2A8 != 0) {
        bool canCrushWall = (ovType->Crushable_0x22D != 0)
                         && (self->TechnoTypeClass->Crusher_0xD28 != 0
                             || TechnoClass__HasWeaponAbility(self, 0x11));  // veteran/elite Crusher ability

        if (!canCrushWall) {
            // Check if wall is breakable AND locomotor speed type == 0xC (ship?)
            if (ovType->IsWall_0x2A8 == 0 || self->TechnoTypeClass->LocoType_0x5B4 != 0xC) {
                // Additional check: can unit fire?
                if (!self->vtable[0x2AC]()) return 7;     // CanFire check
                // Check weapon can destroy walls
                WeaponStruct* weapon = self->vtable[0x3F8](0);  // GetWeapon(0)
                if (weapon->WeaponType == NULL) return 7;
                WarheadTypeClass* wh = weapon->WeaponType->Warhead_0xAC;
                if (wh->Wall_0x144 == 0              // Warhead can't destroy walls
                    && (wh->Wood_0x147 == 0           // ...or wood
                        || ovType->Armor_0x9C != 6))  // ...but only if wall armor is wood(6)
                    return 7;

                // Can see/own the wall?
                if (!HouseClass__Is_Ally_ByIndex(self->Owner, cell->OwnerHouse_0x50)) {
                    if (resultCode < 5) resultCode = 5;  // Enemy wall
                    goto OBJECT_LOOP;
                }
            } else {
                // Ship locomotor + wall: just check ownership
                if (!HouseClass__Is_Ally_ByIndex(self->Owner, cell->OwnerHouse_0x50))
                    goto OBJECT_LOOP;
            }
        }
        if (resultCode < 4) resultCode = 4;   // Friendly wall
    }
}
```
(corrected 2026-06-01: was `OverlayCrushable || (Crusher && HasWeaponAbility)`; binary enters the weapon-required path when `OverlayCrushable == 0 OR (Crusher == 0 AND HasWeaponAbility == 0)`, so the pass-through case is `OverlayCrushable && (Crusher || HasWeaponAbility)` via `decompile_function 0x0073F0A0` - OPERATOR_OR_ORDER_DRIFT)

**`HouseClass__Is_Ally_ByIndex`** (`0x004F9A10`, older drafts call it `FUN_004f9a10`) checks own/allied status by house index:
```c
bool IsOwnedOrAllied(int houseIndex) {
    if (houseIndex == this->Index_0x30) return true;
    if (houseIndex == -1) return false;
    return (this->AllianceBitmask_0x5788 & (1 << houseIndex)) != 0;
}
```
(corrected 2026-06-01: was stale `FUN_004f9a10`/`HouseClass::IsOwnedOrAllied`; current binary label and body compare `param_2` to `HouseClass+0x30` then test `HouseClass+0x5788` alliance bits via `decompile_function 0x004F9A10` - RTTI_LABEL_DRIFT)

**`HouseClass__IsPlayerControl`** (`0x0050B730`) returns whether the house is player-controlled.
Checks `field_0x1EC` / `field_0x1ED` vs `g_GameMode`. Called in Phase 9a to gate crate
pickup: crates are blocked only for non-player-controlled houses in singleplayer (`g_GameMode == 0`).
(corrected 2026-05-28: was described as "checks if a house can use multiplayer crate pickups";
binary shows `HouseClass__IsPlayerControl` checks player-control status, not crate eligibility.
Verified via `get_function_by_address 0x0050B730` + `decompile_function 0x0050B730` — ROOT_CAUSE: RTTI_LABEL_DRIFT)

**`TechnoClass__HasWeaponAbility`** (`0x0070D0D0`) checks whether a unit has a
veteran/elite weapon ability enabled (`+0x29C`/`+0x2AE` flags on TechnoTypeClass checked via
`VeterancyClass__IsVeteran`/`IsElite`). Returns 1 if the unit's veterancy grants the
secondary weapon ability (`param_2` = ability index, 0x11 = Crusher in the wall/crush check).
(corrected 2026-05-28: was described as "Crusher height-range check (can unit crush at this
terrain height?)"; binary shows `TechnoClass__HasWeaponAbility` — veteran/elite ability gate,
NOT a terrain-height range check. Verified via `get_function_by_address 0x0070D0D0` +
`decompile_function 0x0070D0D0` — ROOT_CAUSE: RTTI_LABEL_DRIFT)

### Phase 10: Object Iteration Loop (lines 175-466)

This is the main loop that iterates through all objects in the cell. Depending on
`isBridgeCell`, it starts from either:
- `cell->FirstObject_0xE4` (ground-level linked list)
- `cell->BridgeObject_0xE8` (bridge-level linked list)

Each object `piVar15` is traversed via `piVar15->Next_0x30` (offset 0x30 = `piVar15[0xC]`).

#### 10a: Self-check

```c
if (self == piVar15) {
    // Clear occupancy bits for self — don't block yourself
    occupancyBits &= ~0x20;    // Clear vehicle bit
    hasUnitOnCell = 0;
    continue;
}
```

#### 10b: Transport/Passenger Destination Match

```c
if (self->Transport_0x69C != NULL) {    // self->field_0x1A7 (at int* offset)
    ILocomotor* transportLoco = self->Transport->field_0x28;
    if (transportLoco != NULL) {
        CellStruct* myDest = self->vtable[0x1B8]();       // GetDestCell
        CellStruct* theirDest = piVar15->vtable[0x1B8]();  // GetDestCell
        if (myDest->X == theirDest->X && myDest->Y == theirDest->Y)
            return 0;   // Same destination as our transport — OK
    }
}
```

#### 10c: Owner's DontScore List Check

```c
if (DynamicVectorClass__Contains(self->DontScoreList, piVar15)  // Is object in list?
    && piVar15->vtable[0x2C]() == 6   // WhatAmI == Building
    && FUN_00458a00(piVar15, cell) == 0) {   // Helper says this building does NOT occupy/block this row
    goto NEXT_OBJECT;
}
```
(corrected 2026-06-01: was `FUN_00458a00 != 0`; binary skips the DontScore-list building only on the false/zero helper result, while nonzero falls through into normal object processing, via `decompile_function 0x0073F0A0` and `decompile_function 0x00458A00` - OPERATOR_OR_ORDER_DRIFT)

#### 10d: Infantry Mutual-Ignore (Both IsTrain)

```c
if (self->TechnoTypeClass->IsTrain_0xE18 != 0
    && piVar15->vtable[0x2C]() == 1      // WhatAmI == Infantry
    && piVar15->TechnoTypeClass->IsTrain_0xE18 != 0) {
    return 0;   // Both are trains — pass through each other
}
```

#### 10e: Building Processing

When `piVar15->vtable[0x2C]() == 6` (Building):

**Mission_Enter + Target match:**
```c
if (self->vtable[0x184]() == 7     // GetMission == Enter (7)
    && piVar15 == self->NavTarget_0x5A4) {   // Entering this building

    BuildingTypeClass* bldType = piVar15->field_0x520;   // piVar15[0x148]
    if (bldType->CanUnitEnter_0x16AE != 0          // UnitCanEnter flag
        && !piVar15->vtable[0x1D4]()) {             // !IsBeingWarped
        return 0;   // OK — entering the target building (e.g., refinery)
    }
}
```

**Mission_Capture + Target match:**
```c
if (self->vtable[0x184]() == 9     // GetMission == Capture (9)
    && piVar15 == self->NavTarget_0x5A4
    && bldType->CanBeCaptured_0x16AD != 0
    && !piVar15->vtable[0x1D4]()) {
    return 0;   // OK — capturing this building
}
```

**Capturable building on destination cell:**
```c
if (bldType->CanBeCaptured_0x16AD != 0) {
    // Get self's cell coords, look up first object there
    Coords selfCoords = self->vtable[0x48]();  // GetCoords
    CellStruct selfCell = { selfCoords.X >> 8, selfCoords.Y >> 8 };
    CellClass* myCell = MapClass__Get_CellClass(selfCell);
    // Walk the cell's object list
    for (obj = myCell->FirstObject_0xE4; obj != NULL; obj = obj->Next_0x30) {
        if (obj == piVar15) return 0;  // Building is on our cell — OK
    }
}
```

**CanBeGarrisoned check (HasActiveAnim / CanGarrison):**
```c
if (bldType->HasActiveAnim_0x16B7 != 0) {
    // This is a garrisonable/deployable building
    if (!BuildingClass__CanGarrison(piVar15)) {
        // Can't garrison — check alliance
        if (HouseClass__IsAlliedWith(piVar15->Owner, self->Owner)) {
            if (resultCode < 3) resultCode = 3;   // Friendly building → scatter
        } else {
            if (!self->vtable[0x2AC]()) return 7;  // Can't fire → impassable
            if (resultCode < 5) resultCode = 5;     // Enemy building → attack
        }
    }
    goto NEXT_OBJECT;
}
```

**IsRepairDepot / UnitRepair:**
```c
if ((bldType->IsRepairDepot_0x16A9 != 0 || bldType->UnitRepair_0x16AB != 0)
    && piVar15 == Look_up_building_in_cell(cell)  // Is primary building here?
    && FUN_00458a00(piVar15) == 0) {              // Can't be bypassed?
    goto NEXT_OBJECT;                              // Skip (handled by mission)
}
```

**Radiation-immune buildings:**
```c
if (bldType->ImmuneToRadiation_0x1701 != 0) goto NEXT_OBJECT;  // Skip these
```

**IsGate check:**
```c
if (bldType->IsGate_0x16BF != 0
    && (piVar15->MissionData_0x618 == 0xC    // Gate mission state = opening
        || piVar15->MissionData_0x618 == 8)) {  // or open
    goto NEXT_OBJECT;                        // Gate is open — passable
}
```

**IsLaserFence:**
```c
if (bldType->IsLaserFence_0x16C0 != 0) {
    if (piVar15->Owner_0x21C->LegalTarget_0x1FA != 0)
        return 7;   // Active laser fence = impassable
    goto NEXT_OBJECT;
}
```
(corrected 2026-06-01: was `Owner->HouseTypeClass_0x21C->LegalTarget_0x1FA`; binary reads `*(char *)(piVar15[0x87] + 0x1FA)`, i.e. owner `HouseClass+0x1FA` directly, via `decompile_function 0x0073F0A0` - OFFSET_RETYPED_WRONG)

**HasBib (large foundation) check:**
```c
if (bldType->HasBib_0x1570 != 0) {
    // Check if the building actually occupies this specific cell
    // using the bib offset from DAT_0089F690
    CellStruct adjusted = { cell->X + DAT_0089F690.X, cell->Y + DAT_0089F690.Y };
    CellClass* bibCell = MapClass__Get_CellClass(adjusted);
    if (Look_up_building_in_cell(bibCell) != piVar15) goto NEXT_OBJECT;
    // Building doesn't own this bib cell — skip
}
```

**Default building path (LAB_0073f823):**

For non-special buildings, checks mission context:

```c
// Mission_Enter + target building is an infantry type (WhatAmI==1)
if (self->vtable[0x184]() == 7 && piVar15 == self->NavTarget_0x5A4) {
    if (piVar15->vtable[0x2C]() == 1) return 0;  // Entering infantry = OK
}

// Mission_Unload + carried unit is this object
if (self->vtable[0x184]() == 0xB && self->Cargo_0x218 == piVar15) {
    return 7;   // Can't enter cell with our own cargo on it
}
```

**Blocker movement detection:**
```c
if (piVar15->IsOnMap_0x14 & 0x4) {    // Object has movement flags set
    hasFriendlyMoving = (piVar15->NavTarget_0x5A4 == NULL
                        && CDTimerClass__Remaining(piVar15->MoveTimer_0x388) == 0
                        && piVar15->Locomotor_0x674->vtable[0x10]() /* IsMoving */);
}
```

#### 10f: Alliance Check and Result Assignment

```c
if (HouseClass__Is_Ally(self->Owner, piVar15)) {
    // ALLIED object
    if (!hasFriendlyMoving) {
        // Stationary ally
        if (piVar15->vtable[0x2C]() == 6) return 7;  // Building = impassable
        if (resultCode < 6) resultCode = 6;            // Other = obstacle
    } else {
        // Moving ally: compute facing comparison
        // (complex atan2-based facing comparison, see below)
        // If facing within same octant AND distance < 0x200 leptons:
        //   return 7;  // Too close, same direction = deadlock
        // Else:
        //   resultCode = max(resultCode, 2);  // Temporarily blocked
    }
} else {
    // ENEMY object
    FUN_0040dd20(piVar15);  // Check if it's a "real" unit (Infantry/Vehicle/Building/Aircraft)

    if (result != NULL && result->MissionData_0x220 == 2) {
        // Neutral/civilian with specific state
        if (resultCode < 1) resultCode = 1;  // Special blockage
    } else {
        // Check crusher/cloaking
        bool canCrush = (self->TechnoTypeClass->Crusher_0xD28 != 0)
                     || TechnoClass__HasWeaponAbility(self, 0x11);  // veteran/elite Crusher ability
        bool canCrushTarget = TechnoClass__CanCrushCheck(blocker, self);  // target crushable + alliance/IronCurtain check

        if (canCrush && canCrushTarget) {
            // Can crush AND cloak handling allows passage
            if (HouseClass__Is_Ally(self->Owner, piVar15)) {
                // Wait — friendly after cloak resolution
            } else {
                crushCandidate = true;  // Mark as crushable
            }
        } else {
            // Check weapon compatibility
            WeaponStruct* weapon = self->vtable[0x3F8](0);  // GetWeapon(0)
            if (weapon->WeaponType == NULL
                && self->TechnoTypeClass->IsJumpJet_0xC94 == 0) {
                return 7;  // No weapon AND not jumpjet = impassable
            }

            if (piVar15->vtable[0x2C]() == 6) {      // Enemy building
                if (piVar15->TypeClass->IsInvisible_0x16B6 != 0)
                    return 7;  // Invisible building = impassable
            }
            else if (piVar15->vtable[0x2C]() == 0x24) {  // AnimClass (0x24)
                // Special case: anim blocking
                uVar14 = self->vtable[0x2E4](piVar15);   // GetThreatRating?
                weapon = self->vtable[0x3F8](uVar14);
                if (weapon->WeaponType == NULL) return 7;
                WarheadTypeClass* wh = weapon->WeaponType->Warhead_0xAC;
                if (wh == NULL) return 7;
                if (wh->Wood_0x147 == 0) return 7;  // Can't destroy terrain
            }

            if (resultCode < 5) resultCode = 5;  // Enemy occupant
        }
    }
}
```

#### 10g: Moving Ally — Facing/Distance Check (Deadlock Prevention)

When a **moving friendly** unit is found, the engine computes whether both units are
heading in the same direction and are very close, to prevent deadlocks:

```c
// Get current game tick phase (for randomized tiebreaking)
uint tickPhase = (RateTimer__Current() >> 12 + 1) >> 1 & 7;

// Get facing from self's coords to blocker's coords
int selfFacing = atan2(blockerY - selfY, selfX - blockerX);  // 0-65535
selfFacing = (selfFacing >> 12 + 1) >> 1 & 7;  // Convert to octant (0-7)

if (tickPhase == selfFacing) {
    // Same facing direction — check distance
    Coords selfPos = self->vtable[0x48]();
    Coords blockerPos = piVar15->vtable[0x48]();
    int dist = Sqrt_Approx(dx*dx + dy*dy + dz*dz);

    if (dist < 0x200 && facingMatch) {   // Very close + same direction
        return 7;  // Deadlock — one must yield
    }
}

// Check if blocker is moving via locomotor
if (blocker->field_0x6B6 == 0   // Not IsStuck?
    || blocker->vtable[0x2C]() == 0xF) {   // WhatAmI == Aircraft (0xF)
    if (blocker->Locomotor_0x674->vtable[0xA4]()) {   // Locomotor::IsReallyMoving
        if (resultCode < 2) resultCode = 2;  // Temporarily blocked
    }
}
```

### Phase 11: Post-Loop SpeedType/LandType Check (lines ~175-190 in final section)

After the object loop, if `isBridgeCell == false`:

```c
float speed = g_SpeedType_LandType_Table[cell->LandType_0xEC * 9
              + self->TechnoTypeClass->SpeedType_0x67C];
if (speed == 0.0f)       // 0.0 at FLOAT_007e1748
    return 7;            // Terrain impassable for this speed type
```

The table at `0x0089EA40` is a 12-row x 9-column float array:
- Rows = LandType (Clear, Road, Water, Rock, Wall, Tiberium, Beach, Rough, Ice, Railroad, Tunnel, Weeds)
- Columns = SpeedType (Foot, Track, Wheel, Hover, Winged, Float, Amphibious, FloatBeach, ???)
  **[Corrected 2026-04-06: Float/Winged were swapped; verified order from binary at 0x81da58 is Foot,Track,Wheel,Hover,Winged,Float,Amphibious,FloatBeach]**

`speed == 0.0` means the terrain is completely impassable for that speed type.

### Phase 12: Final Code Resolution (lines ~190-end)

```c
if (resultCode == 0) {   // All clear so far
    if (crushCandidate) {
        if (hasUnitOnCell) {
            // Check the first UnitClass object from the selected ground/bridge list
            int* unit = CellClass__FindFirstUnit(cell, bridgePassFlag);
            if (unit != NULL) {
                CellClass__FindFirstUnit(cell, bridgePassFlag);  // Re-check
                if (TechnoClass__CanCrushCheck(unit, self) != 0)
                    return 0;   // Can crush through
            }
            return 2;  // Temporarily blocked (wait for crush opportunity)
        }
    }
    else if ((occupancyBits & 0x3F) != 0) {    // Infantry sub-cells occupied
        int bridgeOwner = cell->OccupantHouse_0x58;  // Bridge/occupant house check
        if (!hasUnitOnCell && (bridgeOwner == -1 || !HouseClass__Is_Ally_ByIndex(self->Owner, bridgeOwner))) {
            // Not own/allied infantry occupant
            if (self->TechnoTypeClass->Crusher_0xD28 == 0
                && !TechnoClass__HasWeaponAbility(self, 0x11)) {
                // Can't crush
                if (self->TechnoTypeClass->IsJumpJet_0xC94 == 0) {
                    WeaponStruct* weapon = self->vtable[0x3F8](0);
                    if (weapon->WeaponType == NULL
                        || weapon->WeaponType->Warhead->CellSpread_0x2A5 == 0) {
                        return 7;  // No weapon to deal with infantry
                    }
                }
                return 5;  // Enemy infantry = attack
            }
            // Enemy infantry with Crusher/weapon ability falls through with the current resultCode
            // (usually 0).
        } else {
            return 2;  // Vehicle bit set, or infantry owner is own/allied
        }
    }
}
return resultCode;
```
(corrected 2026-06-01 and rechecked 2026-06-06: was `CellClass__FindFirstBuilding` and unconditional `return 2` for can-crush/friendly infantry; binary shows `0x0047EBA0` returns the first object whose `WhatAmI()==1`, i.e. UnitClass, not Building `6` or InfantryClass `0xF`. The final infantry/occupancy branch only assigns code 2 on the allied/owned-or-vehicle-bit path while enemy infantry with Crusher/ability leaves the current `resultCode` unchanged via `decompile_function 0x0047EBA0`, `decompile_function 0x0073F0A0`, and the RTTI mapping in `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md` - RTTI_LABEL_DRIFT + OPERATOR_OR_ORDER_DRIFT)

---

## Field Offsets Referenced

### UnitClass (inherits TechnoClass -> ObjectClass)

| Byte Offset | Index | Field | Purpose |
|-------------|-------|-------|---------|
| 0x014 | [5] | Flags byte | bit 0=IsOnMap, bit 2=IsBuilding, bit 4=movement |
| 0x030 | [0xC] | Next ptr | Next object in cell linked list |
| 0x218 | [0x86] | Cargo | Pointer to carried unit (for Mission_Unload check) |
| 0x21C | [0x87] | Owner HouseClass* | Owner house pointer |
| 0x388 | -- | MoveTimer | CDTimerClass for movement delay |
| 0x3D5 | -- | RequiresRevealedCells | If nonzero, can't path through shroud |
| 0x520 | [0x148] | TypeClass* | Pointer to UnitTypeClass / BuildingTypeClass |
| 0x5A4 | [0x169] | NavTarget | Navigation target (building to enter/capture) |
| 0x618 | [0x186] | MissionData | Mission-specific state (gate open/close state) |
| 0x674 | [0x19D] | Locomotor* | ILocomotor COM interface pointer |
| 0x69C | [0x1A7] | Transport | Pointer to transport unit |
| 0x6B6 | -- | IsStuck | Flag for stuck state |
| 0x6C4 | [0x1B1] | TechnoTypeClass* | (Alternate path to type class, used for field lookups) |

### TechnoTypeClass

| Byte Offset | INI Key | Purpose |
|-------------|---------|---------|
| +0x5B4 | -- | LocomotorType (12=Drive, etc.) |
| +0x67C | `SpeedType=` | SpeedType enum for terrain table lookup |
| +0xC94 | `JumpJet=` (or IsJumpJet) | Can jump over obstacles |
| +0xD28 | `Crusher=` | Can crush infantry/walls |
| +0xDFC | -- | TunnelLocomotor index (-1 = none) |
| +0xE18 | `IsTrain=` | Train units pass through each other |

### CellClass

| Byte Offset | Purpose |
|-------------|---------|
| +0x24 | Cell coordinates (packed short X, short Y) |
| +0x38 | TileType index (into IsometricTileType array) |
| +0x44 | OverlayIndex (-1 = none) |
| +0x50 | OwnerHouse index (for wall ownership) |
| +0x54 | Ground-level data (saved in local) |
| +0x58 | Bridge-level house/occupant index |
| +0xE4 | FirstObject pointer (ground-level linked list head) |
| +0xE8 | FirstObject pointer (bridge-level linked list head) |
| +0xEC | LandType (int, 0-11) |
| +0x11A | Height (byte) — DUAL-SEMANTIC: terrain tile-sub-type byte for normal cells (passed to `TMP_ReadSlopeType` for slope lookup); direction sub-type byte for tube cells (`LandType==10`, values 2 and 6 valid). NOT a terrain height. |
| +0x11B | Level (signed i8) — the actual terrain height level (each level = 15 pixels of world Z). Read by all bridge-height arithmetic via `MOVSX`. **This is what older drafts of this doc called `Height_0x11B`** — `Level` is the more precise label per [BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md §2](../bridges/00-system-models/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md). |
| +0x11C | SlopeIndex (byte) — terrain slope (0=cliff/fallback, 1-20=canonical ramp). Read by `CheckBridgeTraversal` diff-1 gate. |
| +0x124 | GroundOccupancy bitfield (bits 0-3=infantry, bit 5=vehicle) |
| +0x128 | BridgeOccupancy bitfield (same format) |
| +0x140 | CellFlags (bit 8=HasBridge, bit 10=BridgeRamp, bit 11=NSBridge) |

### BuildingTypeClass

| Byte Offset | INI Key | Purpose |
|-------------|---------|---------|
| +0x1570 | `HasBib=` | Large foundation bib |
| +0x1620 | `NumberImpassableRows=` | Building footprint rows |
| +0x16A9 | `IsRepairDepot=` | Repair depot flag |
| +0x16AB | `UnitRepair=` | Unit repair flag |
| +0x16AD | `Capturable=` | Can be captured by engineer |
| +0x16AE | `CanUnitEnter=` | Unit can enter (refinery) |
| +0x16B6 | `Invisible=` | Invisible building |
| +0x16B7 | `CanBeGarrisoned=` / HasActiveAnim | Garrisonable building |
| +0x16BF | `IsGate=` | Gate building (opens/closes) |
| +0x16C0 | `IsLaserFence=` | Laser fence post |
| +0x1701 | `ImmuneToRadiation=` | Radiation immune |

### OverlayTypeClass

| Byte Offset | INI Key | Purpose |
|-------------|---------|---------|
| +0x9C | -- | Armor type |
| +0x22D | -- | Crushable/passable flag (allows crush-through) |
| +0x2A8 | `Wall=` | Is a wall overlay |
| +0x2AA | -- | Special overlay flag (crate pickup restriction) |

### IsometricTileTypeClass

| Byte Offset | Purpose |
|-------------|---------|
| +0x2E4 | RampType (3,4,5 for different slopes) |
| +0x2E8 | RampDirection (3,4,5 for different orientations) |

---

## Helper Functions Called

| Address | Name | Purpose |
|---------|------|---------|
| 0x00484F20 | CellClass__GetTubeAtCell | Returns TubeClass* or NULL |
| 0x00481810 | MapCoord_StepByDir_GetCell | Step from a cell by `g_DirectionOffsets[dir]` and call `MapClass__Get_CellClass` (corrected 2026-06-01: was `Pathfinding_update_continued`; verified via `decompile_function 0x00481810` — RTTI_LABEL_DRIFT) |
| 0x005657A0 | MapClass__Get_CellClass | Convert cell coords to CellClass* |
| 0x0047C520 | Look_up_building_in_cell | Find first BuildingClass in cell's object list |
| 0x0047EBA0 | CellClass__FindFirstUnit | Find first `WhatAmI()==1` object from ground (`+0xE4`) or bridge (`+0xE8`) list (corrected 2026-06-01 and rechecked 2026-06-06: old Ghidra label `CellClass__FindFirstBuilding` was misleading; interim `CellClass__FindFirstInfantry` was also wrong because InfantryClass returns `0xF`; body returns UnitClass `1`, via `decompile_function 0x0047EBA0` and `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md` RTTI mapping — RTTI_LABEL_DRIFT) |
| 0x00578540 | TechnoClass__IsOnScreen | Check if cell/object is revealed to player |
| 0x004D9C10 | FootClass__LocomotorPassabilityCheck | Locomotor-type vs terrain passability check (corrected 2026-05-28: was FUN_004d9c10; Ghidra label confirmed via get_function_by_address 0x004D9C10 — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x0050B730 | HouseClass__IsPlayerControl | Returns whether the house is player-controlled; used in Phase 9a crate gate (corrected 2026-05-28: was FUN_0050b730 "crate pickup check"; actual Ghidra label is HouseClass__IsPlayerControl — ROOT_CAUSE: RTTI_LABEL_DRIFT; verified via get_function_by_address 0x0050B730 + decompile_function 0x0050B730) |
| 0x0070D0D0 | TechnoClass__HasWeaponAbility | Checks if unit's veterancy (veteran/elite) grants a specific weapon ability (param_2=ability index, 0x11=Crusher ability). NOT a terrain height/range check. (corrected 2026-05-28: was FUN_0070d0d0 "Crusher height-range check (can unit crush at this terrain height?)"; actual Ghidra label is TechnoClass__HasWeaponAbility — ROOT_CAUSE: RTTI_LABEL_DRIFT; verified via get_function_by_address 0x0070D0D0 + decompile_function 0x0070D0D0) |
| 0x005F6CD0 | TechnoClass__CanCrushCheck | Determines if self can crush the target: checks target's Crushable flag, self's Crusher/IsOnMap flags, alliance, IronCurtain status. Returns 1 if crush is permitted. (corrected 2026-05-28: was "FUN_005f6cd0 — Cloak/crush interaction (can unit see/crush cloaked target?)"; binary label is TechnoClass__CanCrushCheck, no cloaking logic inside — ROOT_CAUSE: RTTI_LABEL_DRIFT; verified via get_function_by_address 0x005F6CD0 + decompile_function 0x005F6CD0) |
| 0x004F9A10 | HouseClass__Is_Ally_ByIndex | Check own/allied status by house index (`HouseClass+0x30`, alliance bitmask `+0x5788`) (corrected 2026-06-01: was `FUN_004f9a10`; verified via `decompile_function 0x004F9A10` — RTTI_LABEL_DRIFT) |
| 0x004F9A50 | HouseClass__IsAlliedWith | Full alliance check (HouseClass method) |
| 0x004F9A90 | HouseClass__Is_Ally_ByObject | Check if two objects are allied (corrected 2026-05-28: was HouseClass__Is_Ally; Ghidra label is HouseClass__Is_Ally_ByObject — ROOT_CAUSE: RTTI_LABEL_DRIFT; verified via get_function_by_address 0x004F9A90) |
| 0x0040DD20 | FUN_0040dd20 | Filter object: returns non-NULL only for Infantry(1), Vehicle(2), Building(6), Aircraft(0xF) |
| 0x00458A00 | FUN_00458a00 | Check if building can be bypassed (crush/foundation check) |
| 0x004525F0 | BuildingClass__CanGarrison | Can infantry garrison this building? |
| 0x0065AD50 | DynamicVectorClass__Contains | Check if object is in a vector |
| 0x004C9480 | CDTimerClass__Remaining | Check if timer has expired |
| 0x004C93D0 | RateTimer__Current | Get current game tick timer value |
| 0x004CAE30 | Math__atan2 | atan2 for facing calculation |
| 0x004CAC40 | Sqrt_Approx | Fast approximate square root |
| 0x007C5F00 | Math__ftol | Float-to-long conversion |
| 0x007DC720 | GameDebugLog__Assert | Debug assertion |

---

## Global Data References

| Address | Name | Purpose |
|---------|------|---------|
| 0x0089EA40 | g_SpeedType_LandType_Table | 12x9 float array: terrain speed multipliers |
| 0x00A8ED2C | g_TileTypeArray | IsometricTileTypeClass* array |
| 0x00A83D84 | g_OverlayTypeArray | OverlayTypeClass* array |
| 0x00A8E7AC | g_MapEditorMode | Nonzero if in map editor (skips fog check) |
| 0x00A8B238 | g_GameMode | Game mode (0=single, nonzero=multiplayer) |
| 0x0089F690 | g_BibOffset | Bib foundation offset (short X, short Y) |
| 0x007E1748 | FLOAT_0.0 | Float constant 0.0 (used for speed table comparison) |
| 0x0087F7E8 | g_MapClass | MapClass singleton (for Get_CellClass calls) |

---

## VTable Offsets Used

| Offset | Method | Called On |
|--------|--------|-----------|
| +0x2C | WhatAmI() / GetAbsType() | Various objects |
| +0x48 | GetCoords(out) | self, blocker |
| +0x84 | GetTechnoType() | (indirect) |
| +0x160 | IsIronCurtained() | blocker |
| +0x184 | GetMission() | self |
| +0x1B0 | `CheckBridgeTraversal` (bridge sub-check — NOT a parent virtual; see top-of-doc correction) | self |
| +0x1AC | `UnitClass::Can_Enter_Cell` (THIS function — the A* entry; dispatched from `AStar_main_loop @ 0x429F54`) | self |
| +0x1B8 | GetDestCell() | self, blocker (transport dest match) |
| +0x1D4 | IsBeingWarped() | blocker building |
| +0x2AC | CanFire() | self |
| +0x2E4 | GetThreatRating(target) | self (for weapon selection vs anim) |
| +0x320 | FUN_004DA1D0 (fog-passability gate — checks field_0x3D5, IsJumpJet, mission; NOT IsPlayerControlled) | self (fog of war bypass) (corrected 2026-05-28: was "IsPlayerControlled()" — INFERENCE_HARDENED; vtable slot 0x7F5F90=0x4DA1D0 confirmed via read_memory + decompile_function) |
| +0x3F8 | GetWeapon(index) | self |

---

## Crusher Flag Logic

The `Crusher` flag at `TechnoTypeClass+0xD28` affects passability in several places:

1. **Wall overlay check (Phase 9b):** If the overlay is crushable (`OverlayTypeClass+0x22D`) AND either `Crusher`
   is set OR `TechnoClass__HasWeaponAbility(self, 0x11)` returns true, the unit can pass through walls without
   needing a weapon that destroys walls. (corrected 2026-06-01: was `Crusher AND HasWeaponAbility` and omitted
   the required overlay-crushable gate; binary uses `OverlayCrushable && (Crusher || HasWeaponAbility)` via
   `decompile_function 0x0073F0A0` — OPERATOR_OR_ORDER_DRIFT)

2. **Infantry on cell (Phase 12):** If `Crusher` is set OR `TechnoClass__HasWeaponAbility(self, 0x11)` returns true,
   enemy infantry occupying the cell do not force code 5/7; the branch falls through with the current result code
   (normally 0). Code 2 is assigned for allied/owned occupancy or the vehicle-bit path. (corrected 2026-06-01:
   was "temporarily blocked (code 2)" for can-crush infantry; verified via `decompile_function 0x0073F0A0` —
   OPERATOR_OR_ORDER_DRIFT)

3. **Enemy object loop (Phase 10f):** The crusher check (`Crusher_0xD28 != 0 OR TechnoClass__HasWeaponAbility(0x11)`)
   is combined with `TechnoClass__CanCrushCheck(blocker, self)` (checks target is crushable, non-allied,
   non-IronCurtained). If both pass, the unit marks `crushCandidate=true` rather than treating the enemy
   as a code-5 obstacle. (corrected 2026-05-28: was "FUN_005f6cd0 cloak interaction" — ROOT_CAUSE: RTTI_LABEL_DRIFT)

4. **`IsJumpJet` at +0xC94** works similarly to `Crusher` in some places: JumpJet units
   can pathfind through infantry-occupied cells even without weapons.

---

## Key Behavioral Notes

1. **Bridge handling:** The function reads occupancy from `+0x124` (ground) or `+0x128`
   (bridge) depending on the `isBridgeCell` flag. Bridge transitions are detected by
   checking if `targetHeight == cell->Level + 4`.

2. **Tunnel system:** Tunnel entry uses facing 8 (a special sentinel). Tube direction
   compatibility requires the movement direction to be within 2 octants of the tube's
   direction.

3. **Train units:** Units with `IsTrain=yes` in both self and blocker (both
   `TechnoTypeClass+0xE18 != 0`) can pass through each other freely (return 0).

4. **Gate buildings:** Gate buildings (`IsGate_0x16BF`) are passable when their mission
   data (`+0x618`) indicates open state (0xC or 8).

5. **Laser fences:** `IsLaserFence_0x16C0` buildings are impassable when their owner's
   `LegalTarget_0x1FA` flag is set (i.e., fence is active/powered).

6. **Deadlock prevention:** When two allied moving units face each other within 0x200
   leptons (~2 cells) and are in the same facing octant in the same tick phase, one
   returns code 7 to force a repath, breaking the deadlock.

7. **Building entry:** Buildings that the unit is navigating to (NavTarget match) with
   specific missions (Enter=7, Capture=9) return code 0 even though the building
   occupies the cell.

8. **Code priority:** The function tracks a running `resultCode` and only upgrades it
   (never downgrades). This means the worst passability code encountered across all
   objects in the cell is returned.
