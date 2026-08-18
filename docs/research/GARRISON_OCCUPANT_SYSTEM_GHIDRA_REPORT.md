# Garrison Occupant System - Ghidra Research Report

## Summary

BuildingClass stores garrison occupants in a **DynamicVectorClass<InfantryClass*>** at byte offset **0x684**. This is a standard Westwood DynamicVectorClass with vtable, Items buffer pointer, Capacity, IsAllocated flag, Count, and GrowStep fields. The Count field at offset 0x694 is directly returned by the virtual function at vtable+0x408 (`BuildingClass__GetOccupantCount`).

Confidence: **HIGH** - verified from constructor initialization, garrison entry function, sell/evacuation function, and the GetOccupantCount getter.

## BuildingClass Garrison-Related Layout

### BuildingClass Instance (byte offsets from `this`)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0x520 | 4 | Type | Pointer to BuildingTypeClass |
| 0x55C | 84 (21x4) | Anims[21] | Building animation slot pointers (AnimClass*) |
| 0x5C8 | 32 (8x4) | DamageFireAnims[8] | Damage fire animation pointers |
| 0x5E8 | 1 | IsDamaged | Current damage state flag |
| 0x664 | 4 | CurrentFiringOccupantIdx | (probable) Index for garrison fire rotation |
| 0x66C | 24 | DynamicVectorClass #1 | Purpose uncertain - possibly bunker passengers (YR) |
| 0x684 | 24 | **Occupants (DVec)** | **DynamicVectorClass\<InfantryClass*\> - garrison occupants** |
| 0x69C | 4 | (unknown) | Zeroed in constructor |

### DynamicVectorClass Layout at 0x684 (Occupants)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0x684 | 4 | VTable | Pointer to DynamicVectorClass vtable (0x007E43C8) |
| 0x688 | 4 | Items | Pointer to heap-allocated array of InfantryClass* |
| 0x68C | 4 | Capacity | Current allocated capacity |
| 0x690 | 1 | IsAllocated | Whether Items buffer is heap-allocated |
| 0x691 | 1 | (unknown) | |
| 0x694 | 4 | Count | **Number of current occupants** |
| 0x698 | 4 | GrowStep | Growth increment (initialized to 10) |

### DynamicVectorClass Layout at 0x66C (Unknown - possibly Bunker)

Same structure as above but at offset 0x66C. Also initialized with vtable 0x007E43C8 and GrowStep=10. Two DVecs are created side by side in the constructor. The first one (0x66C) may be for YR's Bunker system (`Bunker=yes` in BuildingTypeClass), distinct from the garrison system (`CanBeOccupied=yes`).

### BuildingTypeClass Garrison-Related Fields

| Offset | Size | Field | INI Key |
|--------|------|-------|---------|
| 0x157B | 1 | CanBeOccupied | `CanBeOccupied` (bool) |
| 0x157C | 1 | CanOccupyFire | `CanOccupyFire` (bool) |
| 0x1580 | 4 | MaxNumberOccupants | `MaxNumberOccupants` (int) |
| 0x1584 | 1 | ShowOccupantPips | `ShowOccupantPips` (bool) |
| 0x1588 | 8*N | MuzzleFlash[N] | `MuzzleFlashN` (2 ints per position, N = MaxNumberOccupants) |
| 0x15D8 | ... | DamageFireOffset | `DamageFireOffsetN` positions (starts after MuzzleFlash) |
| 0x1664 | 8*8 | AddOccupy[8] | `AddOccupyN` - cell offsets for adding occupy flags (2 shorts each) |
| 0x16A4+ | ... | RemoveOccupy[8] | `RemoveOccupyN` - cell offsets for removing occupy flags |
| 0x16AB | 1 | Bunker | `Bunker` (bool) - separate from garrison |

## Key Functions

### BuildingClass__GetOccupantCount (0x004581F0)
- **vtable offset**: 0x408
- **Signature**: `int __fastcall GetOccupantCount(BuildingClass* this)`
- **Implementation**: Simply returns `*(int*)(this + 0x694)` - the Count field of the Occupants DynamicVectorClass
- **Confidence**: HIGH

### BuildingClass__AddGarrisonOccupant (0x00522910)
- **Signature**: `void __thiscall AddGarrisonOccupant(InfantryClass* infantry, BuildingClass* building)`
- Note: `this` = the infantry entering, `building` = parameter
- **Logic**:
  1. Checks infantry type's `Occupier` flag at InfantryTypeClass+0xEB4
  2. If Occupier=false, checks `C4` flag at +0xEB5 (engineer path - separate logic)
  3. For Occupier=true infantry:
     - Calls infantry's `Limbo()` (vtable+0xD4) to remove from map
     - Adds infantry pointer to building's DynamicVectorClass at 0x684:
       ```c
       count = building->Occupants.Count;  // offset 0x694
       building->Occupants.Count = count + 1;
       building->Occupants.Items[count] = infantry;  // offset 0x688
       ```
     - Calls `FUN_0070f6e0` to update building threat value
     - If this is the first occupant (count becomes 1), calls vtable+0x124 (SetMission to Guard)
     - Plays EVA "Structure Garrisoned" event if player-controlled
- **Confidence**: HIGH
- **Called from**: `InfantryClass__PerCellProcess` (0x00519630) directly (mission-state-8 garrison branch); `FUN_00519710` (0x00519710) is a separate trampoline that also calls this function but is not the primary path (corrected 2026-05-29: was "InfantryClass__Mission_Enter (0x005196A0) via FUN_00519710"; 0x005196A0 falls inside PerCellProcess body (entry 0x519630), no InfantryClass__Mission_Enter label exists in gamemd.exe, direct call confirmed via decompile_function 0x00519630 — RTTI_LABEL_DRIFT)

### BuildingClass__SellBuilding (0x00457DE0) - Occupant Evacuation
- When building is sold with occupants, iterates the Occupants vector **backwards**:
  ```c
  count = *(int*)(this + 0x694);  // occupant count
  while (--count >= 0) {
      infantry = *(InfantryClass**)(*(int*)(this + 0x688) + count * 4);
      // Try to Unlimbo (place) the infantry near the building
      // If unlimbo fails, delete the infantry
      // If infantry type has Assaulter flag, scatter them
  }
  // Clear the DynamicVectorClass
  DVec_Clear(this + 0x684);
  DVec_Resize(this + 0x684, old_capacity, 0);
  ```
- Also called from `BuildingClass__ReceiveDamage` (0x00442230) when building is destroyed (case 4)
- **Confidence**: HIGH

### BuildingClass__CheckAutoSellOrCivilian (0x00458200)
- Handles civilian building auto-sell and ownership transfer based on occupants
- When occupant count == 0 and building belongs to non-civilian: auto-sells
- When occupant count > 0 and building belongs to civilian: transfers ownership to first occupant's owner
  - Reads first occupant: `*(InfantryClass**)(this->field_0x688)` then accesses `+0x21C` (Owner field)
  - Calls vtable+0x3D4 (ChangeOwner) with that house
- **Confidence**: HIGH

### BuildingClass__CanDock / CanGarrison (0x00457CE0 / 0x004525F0)
- `CanDock` checks `BuildingTypeClass+0x157B` (CanBeOccupied) and calls `GetOccupantCount` (vtable+0x408) to compare against `MaxNumberOccupants` (BuildingTypeClass+0x1580)
- Infantry must have `Occupier=yes` (InfantryTypeClass+0xEB4) to garrison
- Building must not be at red health, and garrison cannot be full
- **Confidence**: HIGH

### BuildingClass__UpdateGarrisonFire (0x0043E7B0)
- Renders muzzle flash effects for garrison fire
- Called during building draw/update
- **Confidence**: MEDIUM (rendering function, not fully traced)

### BuildingClass__ReceiveDamage (0x00442230)
- Building's override of ReceiveDamage
- On building destruction (switch case 4), calls `SellBuilding` which evacuates occupants
- Does NOT directly manage the occupant vector during normal damage
- **Confidence**: HIGH

## MuzzleFlash Position Usage

In `BuildingClass__Update` (0x0043FB20), the MuzzleFlash positions from BuildingTypeClass are used to spawn fire/smoke animations at garrison windows:

```c
if (MaxNumberOccupants > 0) {
    offset = 0x1588;  // MuzzleFlash array start in BuildingTypeClass
    for (i = 0; i < MaxNumberOccupants; i++) {
        if ((g_CurrentFrameCounter + i) % 0x18 == 0) {
            world_pos = IsometricPixelToWorld(Type + offset);
            // Create fire anim at building coords + world_pos
        }
        offset += 8;  // Each MuzzleFlash = 2 ints (X, Y pixel offset)
    }
}
```

## AddOccupy / RemoveOccupy INI Keys

Read in `BuildingTypeClass_ReadINI_Water` (0x0045FE50; corrected 2026-05-29: was 0x00460000+; actual entry confirmed via get_function_by_address 0x0045FE50 — GHIDRA_ADDRESS_SHIFT). These are cell-relative offsets stored at BuildingTypeClass+0x1664 (AddOccupy) and further on (RemoveOccupy). Each entry is 2 shorts (X, Y cell offset). Up to 8 entries, looping `while (index < 8)`. These mark/unmark cells as "occupied" when infantry enter/leave the garrison, preventing other units from using those cells.

## Ghidra Labels Applied

| Address | Label |
|---------|-------|
| 0x004581F0 | `BuildingClass__GetOccupantCount` |
| 0x00522910 | `BuildingClass__AddGarrisonOccupant` |
| 0x00442230 | `BuildingClass__ReceiveDamage` |
| 0x00519630 | `InfantryClass__PerCellProcess` (corrected 2026-05-29: was 0x005196A0 / InfantryClass__Mission_Enter; 0x005196A0 is inside PerCellProcess body, not a separate Mission_Enter function — RTTI_LABEL_DRIFT verified via get_function_by_address 0x005196A0 + 0x00519630) |
| 0x00701410 | `BuildingClass__EngineerRepair` |

## Open Questions

1. **First DynamicVectorClass at 0x66C**: Most likely for YR's Bunker system (`Bunker=yes`), separate from garrison (`CanBeOccupied=yes`). Not fully verified from decompilation.
2. **Occupant removal on death**: When an occupant inside a garrison is killed (e.g., by `PenetratesBunker` warhead), the removal mechanism was not fully traced. It likely goes through a standard DynamicVectorClass remove operation on the 0x684 vector.
3. **Which occupant fires**: The mechanism for selecting which occupant fires from the garrison (rotating through them) was not fully traced. `BuildingClass+0x664` may be the current firing index.
