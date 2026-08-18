# BuildingClass__GetDockCoord — Decode Doc

## Summary

`BuildingClass__GetDockCoord` (0x00447b20) returns the world-space lepton
coordinate where a docking unit should park against this building. It is a
multi-branch function that selects among five distinct docking coordinate
strategies based on BuildingTypeClass boolean flags and a dock-slot array:

1. **Weeder branch** (`BuildingTypeClass+0x16bc != 0`): calls `Get_Cell_Packed`
   (vtable+0x1B8 on `this`), shifts cell (X+2, Y+1) by 0x100 leptons + 0x80
   centering. Produces pad cell (NW+2, NW+1) center-of-cell.
2. **Stock refinery branch** (`BuildingTypeClass+0x16bb != 0`): returns the
   building coordinate helper result with X shifted by +0x80. For stock 4x3
   GAREFN/NAREFN this converts to cell NW+2,NW+1, but it is distinct from the
   accepted CAN_DOCK move target at NW+3,NW+1.
3. **Approach-angle branch** (`BuildingTypeClass+0x16ab != 0` and `param_3 != null`):
   computes `atan2` angle from building to requester and picks one of four
   approach-side offsets (±0x80 in X/Y) relative to building center (vtable+0x48).
4. **Type-defined dock slot branch** (`BuildingTypeClass+0x1780 > 0`): reads a
   struct array at `BuildingTypeClass+0x1788` indexed by `RadioClass__FindDockSlot`;
   adds slot offset to building center.
5. **Default fallback**: returns building center (vtable+0x48 result).

Correction from the 2026-05-24 refinery-pad re-swarm: branch 1 is not the stock
refinery branch. `BuildingTypeClass+0x16BC` is parsed from `Weeder=`, while stock
GAREFN/NAREFN use `DockUnload=yes` at +0x16B3 and `Refinery=yes` at +0x16BB.
Stock `GetDockCoord` reaches branch 2 and, for a 4x3 refinery, still resolves to
cell NW+2,NW+1. That is a later dock-arrival coordinate used by
`UnitClass::PerCellProcess`, not the accepted `BuildingClass::Receive_Radio(0x0E)`
move target, which remains NW+3,NW+1. The `-0x80 / +0x80` lepton offsets seen in
the approach-angle branch are a general approach-side half-cell shift, not a
stock-refinery-only constant.

Follow-up correction from the 2026-05-24 radio/timer re-swarm: there is no
physical accepted-cell-to-GetDockCoord bridge in `gamemd`. Drive arrival can
leave the miner stopped at accepted NW+3,NW+1 while its logical refinery
destination remains active. `UnitClass::Receive_Radio(0x16)` does not call
`GetDockCoord` and is not a move command; first ordinary `0x16` can synchronize
locomotor/facing rate and return, while a later/already-synchronized `0x16` can
send `0x15` directly from the stopped accepted-cell state. `PerCellProcess`
still has a `GetDockCoord` equality branch, but that branch is only one possible
`0x15` source and accepted NW+3,NW+1 does not equal stock `GetDockCoord`
NW+2,NW+1.

Function body: 0x00447b20
(verified via `decompile_function 0x00447b20`)

## Active in YR

**Yes.** Bound to vtable slot 0xa8 in the BuildingClass vtable at 0x007e3ebc
(verified by `read_memory 0x007e3f64` → `20 7b 44 00` = 0x00447b20; offset =
0x007e3f64 − 0x007e3ebc = 0xa8). Dispatched whenever a docking unit (harvester,
repaired vehicle, etc.) requests a dock anchor from a building. Fires during every
harvester deposit, repair-depot approach, and war-factory exit in normal YR gameplay.
(verified via `get_xrefs_to 0x00447b20` and `read_memory 0x007e3f64`)

## Decompilation excerpt

```c
// verified via decompile_function 0x00447b20
int * __thiscall BuildingClass__GetDockCoord(int *param_1, int *param_2, int *param_3)
{
  // param_1 = this (BuildingClass*) — int*, so [N] = byte offset N*4
  // param_2 = output CoordStruct buffer
  // param_3 = requesting unit (or null)

  // param_1[0x148] = byte offset 0x520 = BuildingTypeClass* pointer
  iVar13 = param_1[0x148];   // BuildingTypeClass*

  // Branch 1: Weeder dock pad
  if (*(char *)(iVar13 + 0x16bc) != '\0') {
    // Get_Cell_Packed via vtable+0x1B8 on this
    psVar7 = (short *)(**(code **)(*param_1 + 0x1b8))(&param_3);
    sVar1 = psVar7[1];  // cell_Y
    iVar13 = param_1[0x29];  // Z field (byte offset 0xa4)
    *unaff_retaddr = (short)(*psVar7 + 2) * 0x100 + 0x80;  // (cell_X+2)*256 + 128
    unaff_retaddr[1] = (short)(sVar1 + 1) * 0x100 + 0x80;  // (cell_Y+1)*256 + 128
    unaff_retaddr[2] = iVar13;
    return unaff_retaddr;
  }

  // Branch 2: Refinery=yes dock coordinate
  if (*(char *)(iVar13 + 0x16bb) != '\0') {
    piVar8 = FUN_005f6c80(local_30, param_3);  // GetCoords of requester
    iVar13 = piVar8[1];
    iVar12 = *piVar8 + 0x80;
    goto LAB_00447ce1;  // write X+0x80, Y, Z to param_2
  }

  // Branch 3: Approach-angle dependent offset
  if ((*(char *)(iVar13 + 0x16ab) != '\0') && (param_3 != null)) {
    // atan2(requester_Y - this_Y, this_X - requester_X)
    // Then pick ±0x80 offset in X or Y based on 4 quadrants (facing = 0x00..0xff)
    uVar6 = Math__ftol();
    uVar10 = (uVar6 >> 7 + 1) >> 1 & 0xff;  // 8-bit facing direction
    if (uVar10 < 0x40)      { iVar12 = *piVar8 + 0x80;  iVar13 = piVar8[1] - 0x80; }
    else if (uVar10 < 0x80) { iVar12 = *piVar8 + 0x80;  iVar13 = piVar8[1] + 0x80; }
    else if (uVar10 < 0xc0) { iVar12 = *piVar8 - 0x80;  iVar13 = piVar8[1] + 0x80; }
    else                    { iVar12 = *piVar8 - 0x80;  iVar13 = piVar8[1] - 0x80; }
    // write to param_2 and return
  }

  // Branch 4: Type-defined dock slots
  if (*(char *)(iVar13 + 0x16cb) != '\0') || (*(char *)(iVar13 + 0x16a9) != '\0') {
    if (*(int *)(iVar13 + 0x1780) == 0) goto fallback;
    if (*(int *)(iVar13 + 0x1780) == 1) {
      piVar8 = *(int **)(iVar13 + 0x1788);       // first (only) slot entry
      piVar9 = GetCoords(this, local_18);
      // result = GetCoords + slot_offset
    } else {
      iVar13 = RadioClass__FindDockSlot(param_3);
      piVar8 = (int *)(*(int *)(type + 0x1788) + iVar13 * 0xc);  // slot[N] (12-byte struct)
      piVar9 = GetCoords(this, local_24);
      // result = GetCoords + slot_offset
    }
    *unaff_retaddr = *piVar8 + *piVar9;
    unaff_retaddr[1] = piVar8[1] + piVar9[1];
    unaff_retaddr[2] = piVar8[2] + piVar9[2];
  }

  // Branch 5: Fallback — return building center (GetCoords result)
  piVar8 = FUN_005f6c80(local_24, param_3);
  *param_2 = *piVar8; param_2[1] = piVar8[1]; param_2[2] = piVar8[2];
  return param_2;
}
```

(edited for readability; full unedited body in the Decompilation section above)

## Behavioral analysis

### Branch 1 - Weeder pad

Triggered when `BuildingTypeClass+0x16bc != 0`. This flag corresponds to the
`Weeder=` BuildingType flag, not the stock refinery flag. Stock GAREFN/NAREFN do
not set this flag.

The formula:
```
pad_X = (cell_X + 2) * 256 + 128   // cell NW_X + 2, centered (128 = half cell)
pad_Y = (cell_Y + 1) * 256 + 128   // cell NW_Y + 1, centered
pad_Z = this.Z (Location altitude)
```

For a Weeder building at NW cell (10, 10):
- `cell_X = 10`, `cell_Y = 10` (from `Get_Cell_Packed` vtable+0x1B8)
- `pad_X = (10+2)*256 + 128 = 3072 + 128 = 3200 = cell 12.5`
- `pad_Y = (10+1)*256 + 128 = 2816 + 128 = 2944 = cell 11.5`

This places the dock coordinate at cell NW+2,NW+1 (absolute cell 12,11 for
NW 10,10). This branch is real, but it is not the standard stock refinery branch.

The `Get_Cell_Packed` vtable dispatch (`*param_1 + 0x1b8`) uses vtable slot 0x1B8
— the same slot decoded in task #1 (`ObjectClass__Get_Cell_Packed`). This returns
the NW-corner cell index in CONCAT22(cell_Y, cell_X) format.
(verified via `decompile_function 0x00447b20`)

### Branch 2 - Stock Refinery=yes dock coordinate

Triggered when `BuildingTypeClass+0x16BB != 0`, which the binary reader maps to
`Refinery=`. Stock GAREFN/NAREFN set this flag. The branch returns a coordinate
derived from the building coordinate helper with X shifted by +0x80 leptons. For
a 4x3 building at NW cell (10,10), `BuildingClass__GetCoords` yields foundation
center `(2944,2816)`, and the +0x80 X shift yields `(3072,2816)`, which converts
to cell `(12,11)` = NW+2,NW+1.

This is the later dock-arrival coordinate checked by `UnitClass::PerCellProcess`
before radio 0x15. It is not the receiver-side `CAN_DOCK(0x0E)` move target.
That accepted move target is computed inline by `BuildingClass::Receive_Radio`
as building packed/NW cell + (3,1), i.e. `(13,11)` for a refinery at `(10,10)`.

The verified stock handoff is staged. `FootClass::Mission_Enter` sends one
`CAN_DOCK(0x0E)` per mission dispatch and stock `[Enter] Rate=.016` yields a
14-16 frame retry cadence. If the building's `MOVE_TO_CELL(0x12)` reply is
ROGER, the building sends only the accepted NW+3,NW+1 move in that pass. If a
later retry sees the unit already at the accepted cell (`0x12 == 0x14`), the
building sends `0x18` then `0x16` synchronously. The first ordinary `0x16` may
only set locomotor/facing rate; a later/aligned `0x16` can send `0x15` without
requiring a physical move to this `GetDockCoord` cell.

### Branch 3 — Approach-angle offsets

The approach-angle branch uses `Math__atan2` on the delta between the building
center (vtable+0x48) and the requesting unit center (vtable+0x48 on param_3).
The result is quantized to a facing byte (0–255) and mapped to four 90° quadrants
(0x00–0x3F, 0x40–0x7F, 0x80–0xBF, 0xC0–0xFF). Each quadrant applies ±0x80
(= ±128 leptons = ±0.5 cells) to X or Y from the building center.

The ±0x80 offsets documented in CLAUDE.md Frame #5 ("Force_Track 0x47 shifts
center by (-0x80, +0x80)") correspond to this branch — the approach side gets
a ±0.5-cell offset from the building center, selecting the edge of the foundation
closest to the approaching unit. This is NOT refinery-specific; it applies to any
building type with `BuildingTypeClass+0x16ab != 0`.

### Branch 4 — Type-defined dock slots

Uses `RadioClass__FindDockSlot` to look up which dock slot the requesting unit
holds, then reads the lepton offset from a 12-byte-stride array at
`BuildingTypeClass+0x1788`. The result is `GetCoords(this) + slot_offset`.
This branch enables buildings with multiple named dock positions (e.g., multiple
landing pads, bay doors).

### FUN_005f6c80

A GetCoords forwarder: dispatches vtable slot 0x48 on its first argument (an
object pointer) and copies the result. Its only role here is to call GetCoords
on `param_3` (the requesting unit) for branches 2 and the fallback.
(verified via `decompile_function 0x005f6c80`)

### Coordinate reference frame

- **Input**: Branch 1 uses Frame #2 (NW cell, from vtable+0x1B8). The stock
  `Refinery=yes` branch uses the building coordinate helper/foundation-center
  frame plus a +0x80 X shift. Dock-slot and approach branches use building
  coordinates plus their own offsets.
- **Output**: Frame #1 (leptons) — the returned CoordStruct is in absolute
  lepton coordinates.
- **Weeder branch output**: center of the cell at (NW_X+2, NW_Y+1) in leptons.
- **Stock 4x3 refinery `GetDockCoord` output**: cell (NW_X+2, NW_Y+1), produced
  through the `Refinery=yes` branch, not through the `Weeder` branch.
- **Stock accepted `CAN_DOCK(0x0E)` move target**: cell (NW_X+3, NW_Y+1).
- **Stock art `QueueingCell=4,1` fallback/wait target**: cell (NW_X+4, NW_Y+1).
- **Stock `0x16` unload path**: can send `0x15` from stopped accepted
  (NW_X+3, NW_Y+1) after idle/destination/mission/facing-rate gates; it does not
  physically move the unit to `GetDockCoord`.

### Vtable slot

GetDockCoord is at vtable slot 0xa8 in the BuildingClass vtable (0x007e3ebc).
`read_memory 0x007e3f64` → `20 7b 44 00` = 0x00447b20.
`0x007e3f64 − 0x007e3ebc = 0xa8`. Confirmed.
(verified via `read_memory 0x007e3f64`)

### INI keys / enums

The branch-select flags at `BuildingTypeClass+0x16bc`, `+0x16bb`, `+0x16ab`,
`+0x16cb`, `+0x16a9` correspond to INI keys that control building dock behavior.
The 2026-05-24 re-swarm verified these scoped names: `+0x16B3 = DockUnload`,
`+0x16BB = Refinery`, `+0x16BC = Weeder`. Stock GAREFN/NAREFN set DockUnload and
Refinery, not Weeder.

### No direct code callers

The function is dispatched exclusively via vtable slot 0xa8. No UNCONDITIONAL_CALL
callers found by `get_function_callers`.
(verified via `get_function_callers 0x00447b20`)

## Struct field accesses

| Object | Offset (bytes) | Size | Access | Semantics |
|---|---|---|---|---|
| `this` (param_1, int*) | 0x520 (= [0x148] × 4) | 4 | read | BuildingTypeClass* pointer |
| `this` (param_1, int*) | 0xa4 (= [0x29] × 4) | 4 | read | Location Z (altitude leptons) — Branch 1 only |
| BuildingTypeClass | +0x16bc | 1 (char) | read | Weeder flag |
| BuildingTypeClass | +0x16bb | 1 (char) | read | Refinery flag |
| BuildingTypeClass | +0x16ab | 1 (char) | read | Approach-angle dock flag |
| BuildingTypeClass | +0x16cb | 1 (char) | read | Multi-slot dock flag (A) |
| BuildingTypeClass | +0x16a9 | 1 (char) | read | Multi-slot dock flag (B) |
| BuildingTypeClass | +0x1780 | 4 (int) | read | Dock slot count |
| BuildingTypeClass | +0x1788 | 4 (int*) | read | Dock slot array pointer (12-byte-stride entries) |

(verified via `decompile_function 0x00447b20`)

## Callees

| Callee | Address | Purpose |
|---|---|---|
| `FUN_005f6c80` | 0x005f6c80 | GetCoords forwarder — dispatches vtable+0x48 on an object |
| `Math__atan2` | 0x004cae30 | Angle from unit to building center (branch 3 only) |
| `Math__ftol` | 0x007c5f00 | x87 float10 → integer (branch 3 only) |
| `RadioClass__FindDockSlot` | 0x0065ad90 | Returns dock slot index for requesting unit (branch 4 only) |

(verified via `get_function_callees 0x00447b20`)

## Callers / Lifecycle

No direct UNCONDITIONAL_CALL callers — dispatched exclusively via vtable slot
0xa8 on BuildingClass instances. Fires whenever a docking unit (harvester at
deposit, vehicle approaching repair depot, etc.) requests a dock anchor coordinate.
(verified via `get_function_callers 0x00447b20` and `get_xrefs_to 0x00447b20`)

## Out-of-scope refs

- `RadioClass__FindDockSlot` (0x0065ad90) internals — dock-slot radio reservation; out of scope
- `FUN_005f6c80` callers beyond this function — the GetCoords forwarder may be used elsewhere; out of scope
- BuildingTypeClass field decoding beyond the scoped names verified by the
  refinery-pad re-swarm.
- The 12-byte dock slot entry struct (at `BuildingTypeClass+0x1788[N]`) — layout unknown; out of scope

## Related verification reports

- `../REFINERY_DOCK_0X16_BRIDGE_VERIFICATION_GHIDRA_REPORT.md`
- `../FOOTCLASS_MISSION_ENTER_0X0E_REPEAT_TIMING_GHIDRA_REPORT.md`
- `../UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md`
- `../UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`
- `../BUILDING_RECEIVE_RADIO_0E_GETDOCKCOORD_SIDE_CHECK_GHIDRA_REPORT.md`
- `../DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`

## Unverified claims (YELLOW)

**RESOLVED 2026-05-24 re-swarm**: The scoped INI key names are now verified for
the refinery disagreement: `+0x16B3 = DockUnload`, `+0x16BB = Refinery`,
`+0x16BC = Weeder`. The old "Refinery dock pad flag" label for `+0x16BC` was a
descriptive inference and is wrong for stock GAREFN/NAREFN.

**UNVERIFIED**: Whether `param_1[0x29]` (byte offset 0xa4) used in Branch 1 for
the Z return value is truly the Location.Z field or something else at that offset.
The task #12/15 decode confirmed 0x9C=X, 0xA0=Y, 0xA4=Z — but param_1 here is
`int*`, so [0x29] = byte offset 0x29 × 4 = 0xa4. This matches Location.Z under
the `int*` indexing convention, consistent with the 0x9C/0xA0/0xA4 layout.
The match is structurally sound but was not separately verified in this session
for this specific caller.

**UNVERIFIED**: The exact class name associated with vtable base 0x007e3ebc. The
slot-0 function (`AbstractClass__QueryInterface`) does not uniquely identify the
class. The vtable is attributed to BuildingClass based on the task description
and Ghidra labeling, but was not confirmed via RTTI lookup in this session.
