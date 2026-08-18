# TeleportLocomotionClass::Update_Position — 0x00718260

**Proposed Ghidra label:** TeleportLocomotionClass__Update_Position (existing name is authoritative — labeler skip rename, add plate comment only)

## Summary

Updates the warp destination position cache in the locomotor and, depending on the call mode, either:
- **(mode A, `param_5 != 0`)**: Synchronizes dest-cache-1 (`+0x28..+0x30`) from the owner TechnoClass, applies bridge Z-offset, and calls `vtable+0xf4`/`+0xf0` to push/pull the object position.
- **(mode B, `param_5 == 0`)**: Validates cell occupants at the destination, handles infantry un-limbo collisions via `Rules+0xfa8`, and — if `param_4 != 0` — finds a nearby passable cell via `FootClass__Find_Nearby_Passable_Cell` and writes the adjusted coords into TechnoClass `+0x288/+0x28c/+0x290`.

Called exclusively from `TeleportLocomotionClass__StateMachineTick` at three call sites (0x0071991d, 0x00719967, 0x007199b8), verified via `get_xrefs_to 0x00718260`.

## Active in YR

**Yes.** Single caller `TeleportLocomotionClass__StateMachineTick` (0x007192f0), confirmed via `get_function_callers 0x00718260`. StateMachineTick is the locomotor's per-tick state driver, which is unambiguously YR-live.

## Decompilation Excerpt

Source: `decompile_function 0x00718260`

```c
undefined4 __thiscall
TeleportLocomotionClass__Update_Position
    (int param_1, int param_2, int param_3, uint param_4, char param_5)
{
  // param_1: TeleportLocomotionClass* (this, int = direct byte offsets)
  // param_2: dest X (leptons, GetCoords frame)
  // param_3: dest Y (leptons, GetCoords frame)
  // param_4: dest Z / flags
  // param_5: mode flag — non-zero = "commit mode", zero = "validate/adjust mode"

  if (param_5 == '\0') {
    // BRANCH B: validate cell occupants at destination
    iVar6 = CellClass__Get_Cell_At(&param_2);
    if ((*(uint *)(iVar6 + 0x140) & 0x100) == 0) {
      piVar8 = *(int **)(iVar6 + 0xe4);  // non-bridge: first object in cell
    } else {
      piVar8 = *(int **)(iVar6 + 0xe8);  // bridge cell: use bridge-layer list
    }
    // iterate object list; try Unlimbo at dest for blocking infantry
    for (; piVar8 != 0; piVar8 = piVar8[0xc]) {
      // vtable +0x160: Is_Infantry_Type check
      // vtable +0x2c: What_Am_I (returns 0xf for infantry)
      // vtable +0x48: GetCoords
      // vtable +0x84: GetOwner / GetHouse
      // vtable +0x16c: Unlimbo(coord, facing=0, Rules+0xfa8, 0, 1, 0, 0)
      ...
    }
    // re-check bridge overlay flags after loop
    if (bridge_overlay && !bridge_high) param_5 = '\x01';

    // if param_4 != 0: find nearby passable cell and adjust TechnoClass dest
    if ((char)param_4 != '\0') {
      // look up speed type from TechnoType+0x5b4, map 9→0, 2→0, 3→5
      FootClass__Find_Nearby_Passable_Cell(...);
      // write adjusted dest into TechnoClass +0x288/+0x28c/+0x290
      *(iVar1 + 0x288) = adjusted_X;
      *(iVar1 + 0x28c) = adjusted_Y;
      *(iVar1 + 0x290) = adjusted_Z;
      return 0;
    }
  } else {
    // BRANCH A: commit — sync dest-cache-1 from TechnoClass location
    piVar8 = (int *)(param_1 + 0x28);   // dest-cache-1 base
    if (cache1 == NullCoord) {
      // no cached dest → use TechnoClass current location
      local = TechnoClass[0x27..0x29];  // TechnoClass+0x9c/+0xa0/+0xa4 (Location)
    }
    // call vtable+0xf4 on owner: Move_To(dest)
    (**(vtable+0xf4))(piVar7);
    // read back resulting coords from TechnoClass +0x288/+0x28c/+0x290 into cache-1
    *piVar8 = *(iVar6 + 0x288);
    *(param_1 + 0x2c) = *(iVar6 + 0x28c);
    *(param_1 + 0x30) = *(iVar6 + 0x290);
    // get ground height for dest cell
    uVar11 = CellClass__GetGroundHeight(piVar8);
    *(param_1 + 0x30) = uVar11;         // overwrite Z with ground height
    // bridge overlay Z lift
    if (bridge_overlay && !owner_bridge_flag) {
      *(TechnoClass + 0x8c) = 1;        // set bridge-on-bridge flag
      *(param_1 + 0x30) += g_BridgeZOffset_Teleport;  // add bridge Z offset (0 at runtime)
    } else {
      *(TechnoClass + 0x8c) = 0;        // clear bridge-on-bridge flag
    }
    // call vtable+0xf0 on owner
    (**(vtable+0xf0))(piVar8);
  }

  // TAIL: final position application using dest-cache-1 (or TechnoClass location as fallback)
  if (cache1 == NullCoord) {
    // fallback to TechnoClass current location
    piVar8 = &TechnoClass[0x27];
  }
  (**(vtable+0xf0))(piVar8);
  return 1;
}
```

## Behavioral Analysis

### Call sites in StateMachineTick

Three call sites (verified `get_xrefs_to 0x00718260`):
- `0x0071991d`: mode B (`param_5=0`) — validate/check occupants
- `0x00719967`: mode B (`param_5=0`) — validate/adjust with fallback cell search
- `0x007199b8`: mode A (`param_5≠0`) — commit position

### Mode A — Position Commit

1. Reads dest-cache-1 from locomotor (`+0x28..+0x30`); falls back to TechnoClass current location if cache is null-sentinel.
2. Calls `TechnoClass vtable+0xf4` (Move_To or equivalent position setter) with the destination coord.
3. Reads the applied coords back from TechnoClass `+0x288/+0x28c/+0x290` into dest-cache-1.
4. Overrides Z with `CellClass__GetGroundHeight` result (ground height takes precedence over lepton Z).
5. Bridge overlay check: if dest cell has bridge overlay (`CellClass+0x140 & 0x100`) and owner is not already on a bridge (`TechnoClass+0x8c == 0`):
   - Sets `TechnoClass+0x8c = 1` (bridge-on-bridge flag).
   - Adds `g_BridgeZOffset_Teleport` (currently `0x00000000`, verified `read_memory 0x00b0ec2c`) to Z.
6. Calls `TechnoClass vtable+0xf0` (position finalization / Un_Mark_Occupation).
7. Returns 1 (success).

### Mode B — Validate / Adjust

1. Walks the object list in the destination cell (either `CellClass+0xe4` for normal cells or `CellClass+0xe8` for bridge-layer cells — bridge flag = `CellClass+0x140 & 0x100`).
2. For each object: checks if infantry (`vtable+0x160`) with type `0xf` (`What_Am_I` = infantry), same owner type as locomotor's TechnoClass. If coord match → calls `Unlimbo` (`vtable+0x16c`) at that coord with `Rules+0xfa8` (scatter radius).
3. After loop: if dest cell has bridge overlay but NOT bridge-high (`CellClass+0x140 & 0x200 == 0`), forces `param_5 = 1` (mode switch to commit).
4. If `param_4 != 0`: looks up speed type from `TechnoType+0x5b4`, maps values (9→0, 2→0, 3→5), calls `FootClass__Find_Nearby_Passable_Cell` to find an open cell, writes result to TechnoClass `+0x288/+0x28c/+0x290`. Returns 0.
5. Falls through to tail `vtable+0xf0` call with dest-cache-1 (or TechnoClass fallback). Returns 1.

### Bridge Z offset

`g_BridgeZOffset_Teleport` at `0x00b0ec2c` is `0x00000000` at runtime (verified `read_memory 0x00b0ec2c`). The logic to add it still executes when the bridge overlay flag fires — but the added value is zero. This means the bridge Z path is architecturally present but produces no Z delta in stock YR. The bridge-on-bridge flag (`TechnoClass+0x8c`) is still set/cleared correctly regardless.

### Coord frame annotation

- `param_2/param_3/param_4`: input coords in **GetCoords (foundation center, leptons)** frame — these are the dest coords passed in by StateMachineTick.
- `TechnoClass+0x9c/+0xa0/+0xa4` (`piVar7[0x27..0x29]` when `piVar7` is `int *`): **Location frame (leptons)** — current unit position.
- `TechnoClass+0x288/+0x28c/+0x290`: **GetCoords (leptons)** — warp destination coords set by `InitiateWarp`, read back here after vtable call.
- Locomotor cache `+0x28..+0x30`: mirrors TechnoClass dest coords in **GetCoords (leptons)** frame.

## Struct Field Accesses

`param_1` is `int` (direct byte offsets, verified by decompile syntax `*(int *)(param_1 + N)`).

### TeleportLocomotionClass fields (via param_1 direct):

| Byte Offset | Access | Purpose |
|---|---|---|
| +0x0c | `*(int **)(param_1 + 0xc)` | Pointer to owner TechnoClass object |
| +0x28 | `*(int *)(param_1 + 0x28)` | dest-cache-1 X (sentinel = g_NullCoord_Teleport_X) |
| +0x2c | `*(param_1 + 0x2c)` | dest-cache-1 Y |
| +0x30 | `*(param_1 + 0x30)` | dest-cache-1 Z (overwritten with ground height in mode A) |

### TechnoClass fields (accessed via `*(int **)(param_1 + 0xc)`):

| TechnoClass Byte Offset | Access Pattern | Coord Frame | Purpose |
|---|---|---|---|
| +0x9c | `piVar7[0x27]` (`int *` × 4) | Location (leptons) | Current X position (NW-corner origin) |
| +0xa0 | `piVar7[0x28]` | Location (leptons) | Current Y position |
| +0xa4 | `piVar7[0x29]` | Location (leptons) | Current Z position |
| +0x8c | `*(char *)(... + 0x8c)` | — | Bridge-on-bridge flag (1 = on bridge, 0 = not) |
| +0x288 | `*(iVar1 + 0x288)` | GetCoords (leptons) | Warp dest X (written by InitiateWarp, read back here) |
| +0x28c | `*(iVar1 + 0x28c)` | GetCoords (leptons) | Warp dest Y |
| +0x290 | `*(iVar1 + 0x290)` | GetCoords (leptons) | Warp dest Z |
| +0x5b4 | `*(iVar6 + 0x5b4)` via vtable+0x84 | — | Speed type enum (for zone lookup in mode B) |

### CellClass fields accessed:

| Field | Offset | Purpose |
|---|---|---|
| Bridge overlay flag | `CellClass+0x140 & 0x100` | Is this a bridge cell? |
| Bridge high flag | `CellClass+0x140 & 0x200` | Is this the upper bridge tier? |
| Normal object list | `CellClass+0xe4` | First object in cell (non-bridge) |
| Bridge object list | `CellClass+0xe8` | First object in bridge layer |

## Globals / Enums / INI Keys Referenced

| Symbol | Address | Value | Role |
|---|---|---|---|
| `g_NullCoord_Teleport_X` | `0x00b0ebf8` | 0x00000000 | Sentinel to detect "no dest cached" in cache-1 — corrected from 0x00b0ebd8 |
| `g_NullCoord_Teleport_Y` | `0x00b0ebfc` | 0x00000000 | Sentinel Y |
| `g_NullCoord_Teleport_Z` | `0x00b0ec00` | 0x00000000 | Sentinel Z |
| `g_BridgeZOffset_Teleport` | `0x00b0ec2c` | 0x00000000 | Z delta applied to dest Z when bridge overlay present; zero in stock YR (verified `read_memory 0x00b0ec2c`) |
| `g_RulesClass_Instance` | (global) | — | Rules object; `+0xfa8` = scatter radius used in Unlimbo call |

## Out-of-Scope Refs

| Symbol | Address | Reason |
|---|---|---|
| `CellClass__GetGroundHeight` | `0x00578080` | General cell height utility; not teleport-specific |
| `CellClass__Get_Cell_At` | `0x00565730` | General cell lookup; not teleport-specific |
| `FootClass__Find_Nearby_Passable_Cell` | `0x0056dc20` | General pathfinding fallback; not teleport-specific |
| `MapClass__GetZoneID` | `0x0056d230` | General zone query; not teleport-specific |
| `MapClass__Get_CellClass` | `0x005657a0` | General map utility; not teleport-specific |

## Unverified (YELLOW)

- **TechnoClass vtable+0xf4 identity**: used in mode A to push the unit to dest position. Likely `Force_Track` or `Move_To`. Identity not directly verified via `decompile_function` on the dispatch target — would require runtime vtable read on TechnoClass instance.
- **TechnoClass vtable+0xf0 identity**: called in both mode A tail and mode B tail. Likely `Un_Mark_Occupation_Bits` or `Mark_All_Occupation_Bits`. Not directly verified.
- **TechnoClass vtable+0x16c (Unlimbo)**: inferred from argument pattern `(coord, 0, Rules+0xfa8, 0, 1, 0, 0)`. Identity consistent with Unlimbo signature but not directly confirmed via decompile.
- **Speed type enum values 9, 2, 3**: the branch `if (iVar6 == 9) iVar6 = 0; else if (iVar6 == 2) iVar6 = 0; else if (iVar6 == 3) iVar6 = 5;` remaps speed types for zone lookup. Enum values not cross-referenced against SpeedType definition.
- **`g_RulesClass_Instance + 0xfa8`**: inferred as scatter radius passed to Unlimbo. Rules offset not independently verified.
