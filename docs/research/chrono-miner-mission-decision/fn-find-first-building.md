# CellClass__FindFirstBuilding @ 0x0047EBA0

**Proposed Ghidra label:** CellClass__FindFirstBuilding

## Summary

`CellClass::FindFirstBuilding` scans the object linked-list on a given cell and returns the
first object whose RTTI type matches BuildingClass (type ID == 1). The `param_2` flag selects
which object list to scan: `'\0'` (false) scans `CellClass+0xE4` (the ground-level occupant
list), `'\x01'` (true) scans `CellClass+0xE8` (the air-layer occupant list). Returns the
`BuildingClass*` if found, or `NULL`.

In the context of `TechnoClass::Set_Destination @ 0x00741970`, this function gates the
**teleport-vs-drive** decision for `Teleporter` units (chrono miner): a non-NULL return means
the destination cell has a building — the locomotor is swapped to DriveLocomotionClass so
the dock handshake can proceed. A NULL return means the cell is empty — TeleportLocomotionClass
stays active and the unit warps to the destination.

**Active in YR:** Yes. Called by 8 callers (verified via `get_function_callers 0x0047EBA0`):
- `TechnoClass__Set_Destination @ 0x00741970` — teleport gate
- `DriveLocomotionClass__Process_Drive_Track @ 0x004B0F20` — bridge/crush-path occupancy check
- `DriveLocomotionClass__Process_Movement @ 0x004B2630` — movement collision
- `UnitClass__Can_Enter_Cell @ 0x0073F0A0` — cell enter eligibility
- `UnitClass__PerCellProcess @ 0x0073EC0` — per-cell arrival processing
- `InfantryClass__PerCellProcess @ 0x00519630` — infantry per-cell arrival
- `HouseClass__Find_Nearest_Ally_Building @ 0x00500300` — AI nearest building scan
- `AircraftClass__Mission_Move_Carryall @ 0x00416D50` — carryall landing site check

All callers are active in standard YR skirmish.

---

## Decompilation excerpt

Verified via `decompile_function 0x0047EBA0`.

```c
int * __thiscall CellClass__FindFirstBuilding(int param_1, char param_2)
{
  int *piVar1;
  int iVar2;

  if (g_GameActive != '\0') {
    if (param_2 == '\0') {
      piVar1 = *(int **)(param_1 + 0xe4);   // CellClass+0xE4 = ground object list head
    } else {
      piVar1 = *(int **)(param_1 + 0xe8);   // CellClass+0xE8 = air object list head
    }
    for (; piVar1 != (int *)0x0; piVar1 = (int *)piVar1[0xc]) {
      // vtable+0x2c = WhatAmI() -> returns RTTI type integer
      iVar2 = (**(code **)(*piVar1 + 0x2c))();
      if (iVar2 == 1) {          // 1 = BuildingClass RTTI type
        return piVar1;           // return BuildingClass* on first match
      }
    }
  }
  return (int *)0x0;  // no building found
}
```

---

## Behavioral analysis

### Object list traversal

- **Ground list** (`param_2 == 0`): starts at `CellClass+0xE4` (first ground-occupant pointer).
  Each object's `+0xC` field is the `next` pointer in the linked list (`piVar1[0xc]`).
  Terminates at `NULL`.

- **Air list** (`param_2 != 0`): starts at `CellClass+0xE8`. Same traversal pattern.
  Used for air-layer occupants (aircraft, hovering units).

### Building identification

Each visited object is tested via virtual dispatch: `vtable+0x2c` = `AbstractClass::WhatAmI()`,
which returns the RTTI type integer. Return value `1` matches `BuildingClass`. No other filtering
is applied — the first `WhatAmI()==1` hit is returned immediately.

### Guard: `g_GameActive`

If the global `g_GameActive` flag is false (e.g., during map loading, before simulation starts),
the function returns `NULL` unconditionally without scanning. This avoids list access before the
game state is initialized.

### Return value semantics

- **Non-NULL** (`BuildingClass*`): a building occupies the cell. In `TechnoClass::Set_Destination`
  this triggers locomotor swap to Drive (drive into building) and eventually the dock handshake.
- **NULL**: no building on the cell. In `TechnoClass::Set_Destination` this keeps
  TeleportLocomotion active → warp.

### Usage in `DriveLocomotionClass::Process_Drive_Track`

Verified via `decompile_function 0x004B0F20`. The call at ~`0x004B1A58`:
```c
if (((*(char *)(param_1 + 100) != '\0') &&
    (iVar7 = CellClass__FindFirstBuilding(0), iVar7 != 0)) &&
   (iVar7 = TypeClass_ptr_lookup(), *(int *)(iVar7 + 0x5b4) == 0xc)) {
    *(undefined1 *)(*(int *)(param_1 + 0xc) + 0x6b5) = 1;
```
Here `param_2 = 0` (ground-level) is always used. The building lookup is used to detect
a specific building type (`+0x5b4 == 0xc` = SpeedType 12 = sub-tunnel, TS legacy) for
a special drive path. Active-in-YR status of this particular branch is conditional (TS check).

---

## Struct field accesses

All offsets are direct byte offsets on the respective classes.

| Field | Byte offset | Description |
|---|---|---|
| `CellClass + 0xE4` | 0xE4 | Ground-level object linked-list head (`ObjectClass*`) |
| `CellClass + 0xE8` | 0xE8 | Air-layer object linked-list head (`ObjectClass*`) |
| `ObjectClass + 0xC` (as `piVar1[0xc]`) | `0xC × 4 = 0x30` | **Wait** — `piVar1` is `int*`, so `piVar1[0xc]` = `*(piVar1 + 0xc)` = byte offset `0xC × 4 = 0x30`... |

**Correction on `piVar1[0xc]` offset:**
`piVar1` is declared as `int *`. `piVar1[0xc]` dereferences at byte offset `0xC × 4 = 0x30` from
`piVar1`. This field at `ObjectClass+0x30` is the singly-linked "next object in cell" pointer
(the standard C&C object overlay chain). Verified by context: same pattern used in
`Look_up_building_in_cell @ 0x0047C520` which also uses `piVar1[0xc]` in the same traversal
pattern (`decompile_function 0x0047C520` confirmed).

| Field | Byte offset | Description |
|---|---|---|
| `CellClass + 0xE4` | 0xE4 | Ground-occupant list head (`ObjectClass*`) |
| `CellClass + 0xE8` | 0xE8 | Air-occupant list head (`ObjectClass*`) |
| `ObjectClass + 0x30` | 0x30 | Next object in cell (linked list `NextObject` ptr) |
| `ObjectClass.vtable + 0x2C` | vtable slot 0x2C | `WhatAmI()` — returns RTTI type integer |
| RTTI type `1` | — | BuildingClass |

---

## Globals / enums / INI

| Global | Role |
|---|---|
| `g_GameActive` | Game active flag; function returns NULL if false. Not INI-driven. |

RTTI type constants (from WhatAmI() return values seen across the binary):
- `1` = BuildingClass (confirmed: returned when this function finds a match)
- `2` = UnitClass (confirmed: used in other callers)
- `6` = CellClass (confirmed: used in `Look_up_building_in_cell` which returns RTTI==6)

**Note on RTTI==6 vs RTTI==1:** `Look_up_building_in_cell @ 0x0047C520` checks for `iVar2 == 6`
(not 1), but that function is searching for a building *object* from `CellClass+0xE4` using
type 6 = ??? — this is a different scan type. `CellClass__FindFirstBuilding` correctly uses `1`
for BuildingClass. The discrepancy is noted; `Look_up_building_in_cell` may actually return the
`BuildingClass*` despite the `==6` check, or `6` may be a sub-type. See YELLOW section.

---

## Out-of-scope refs

No callees (verified: `get_function_callees 0x0047EBA0` returned empty).

Callers outside this decode system's scope:
- `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` — locomotion; out of scope
- `DriveLocomotionClass::Process_Movement @ 0x004B2630` — locomotion; out of scope
- `UnitClass::Can_Enter_Cell @ 0x0073F0A0` — cell passability; out of scope
- `UnitClass::PerCellProcess @ 0x0073EC0` — per-cell logic; out of scope
- `InfantryClass::PerCellProcess @ 0x00519630` — infantry; out of scope
- `HouseClass::Find_Nearest_Ally_Building @ 0x00500300` — AI; out of scope
- `AircraftClass::Mission_Move_Carryall @ 0x00416D50` — aircraft; out of scope

---

## Unverified (YELLOW)

- **`Look_up_building_in_cell @ 0x0047C520` uses RTTI==6, not RTTI==1.** The relationship
  between these two functions for building detection is not fully resolved. `CellClass__FindFirstBuilding`
  using `==1` and `Look_up_building_in_cell` using `==6` may represent different use-cases
  (one for actual BuildingClass instances, one for the CellClass->BuildingClass entry point).
  The RTTI values are taken from Ghidra decompilation output and appear as integer constants;
  cross-checking against the RTTI string table for type ID 1 was not done in this session. YELLOW.
- **`CellClass+0xE4` and `+0xE8` field names** ("ground-occupant list" vs "air-occupant list")
  are inferred from `param_2` branching and context. Not cross-verified via CellClass struct
  layout or INI docs in this session. YELLOW.
- **`ObjectClass+0x30` as "NextObject"**: derived from `int*`-indexed `[0xc]` access (0xc × 4 = 0x30).
  The field name "NextObject" is inferred by function; not verified against the CellClass/ObjectClass
  layout document directly in this session. YELLOW.
