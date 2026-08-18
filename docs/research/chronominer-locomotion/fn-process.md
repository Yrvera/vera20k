# TeleportLocomotionClass__Process — 0x00718b70

**Proposed Ghidra label:** TeleportLocomotionClass__Process (existing label is authoritative — plate comment only needed)

**Active in YR:** Yes — dispatched each logic tick via ILocomotion vtable slot 0x44 (HeadToCoord → Process chain). Confirmed live caller: `FootClass::Set_Destination_Internal` @ 0x004D94B0 → `TeleportLocomotionClass__HeadToCoord` @ 0x00718100 → `TeleportLocomotionClass__Process` @ 0x00718b70. (verified via decompile_function 0x00718100 plate comment and decompile_function 0x004D94B0)

---

## Summary

Per-tick entry point for the teleport locomotor, called from `HeadToCoord` each time a teleport destination is being validated or processed. The function has three major branches:

1. **No destination cached** (sentinel check): falls through to current-coords path, drives normally.
2. **Destination cached, mission ≠ ENTER (0xf)**: validates cell, drives unit to cached destination; if infantry-in-transit flag is clear, attempts infantry placement via `CellClass__PlaceInfantryInCell` + `FootClass__Find_Nearby_Passable_Cell` fallback; otherwise calls locomotion vtable move.
3. **Destination cached, mission = ENTER (0xf) OR certain secondary missions**: dispatches to the radio-link/warp-dock gate at LAB_00718ce9, checking radio states 1, 2, or 6 to decide if the unit should commit the teleport to a building dock.

The function writes the validated destination back to the locomotor's destination cache (+0x28/+0x2C/+0x30) and returns 1 if a valid destination was committed, 0 if no destination.

---

## Caller chain (Active in YR: Yes)

```
FootClass::Set_Destination_Internal (0x004D94B0)   [live in YR — core mission dispatch]
  └─ TeleportLocomotionClass__HeadToCoord (0x00718100) [ILocomotion vtable slot 0x44]
       └─ TeleportLocomotionClass__Process (0x00718b70)
```
Verified via decompile_function 0x00718100 (plate comment: "Called by FootClass::Set_Destination_Internal (0x4D94B0)") and decompile_function 0x004D94B0.

---

## Decompilation excerpt

Source: `decompile_function 0x00718b70`

```c
undefined4 __fastcall TeleportLocomotionClass__Process(int param_1)
{
  // param_1 is int (direct byte offsets), TeleportLocomotionClass instance

  // BRANCH 1: Sentinel check — is destination cache empty?
  piVar1 = (int *)(param_1 + 0x28);   // dest cache X
  if (param_1+0x28 == g_NullCoord_Teleport_X &&
      param_1+0x2c == g_NullCoord_Teleport_Y &&
      param_1+0x30 == g_NullCoord_Teleport_Z) {
    // No destination: read TechnoClass current location
    piVar10 = *(int **)(param_1 + 0xc);   // TechnoClass ptr
    local_24 = piVar10[0x27];             // TechnoClass+0x9C (Location.X, leptons, NW frame)
    local_20 = piVar10[0x28];             // TechnoClass+0xA0 (Location.Y)
    local_1c[0] = piVar10[0x29];          // TechnoClass+0xA4 (Location.Z)
    iVar6 = *piVar10;                     // TechnoClass vtable ptr
    piStack_58 = &local_24;
  } else {
    iVar6 = **(int **)(param_1 + 0xc);   // vtable from TechnoClass
    piStack_58 = piVar1;                 // use cached dest
  }

  // vtable+0xf4 call: locomotion Head_To_Coord (stores destination in return area)
  (**(code **)(iVar6 + 0xf4))();

  // Post-call: if result is sentinel, copy sentinel to cache and goto exit
  if (result == NullCoord sentinel) {
    *piVar1 = sentinel_X; +0x2c = Y; +0x30 = Z;
    goto LAB_00719249;
  }

  // Get CellClass for destination
  iVar6 = CellClass__Get_Cell_At(&iStack_34);   // 0x00565730

  // BRIDGE CHECK: CellClass+0x140 & 0x100 = bridge overlay bit
  if ((*(uint *)(iVar6 + 0x140) & 0x100) == 0) {
    uVar11 = 0;  // not on bridge
  } else {
    // Check if already above bridge height: TechnoClass+0x9C/0xA0/0xA4 = Location leptons
    iVar6 = *(int *)(param_1 + 0xc);
    iStack_28 = *(int *)(iVar6 + 0x9c);    // Location.X (NW frame, leptons)
    local_24 = *(int *)(iVar6 + 0xa0);     // Location.Y
    iVar6 = *(int *)(iVar6 + 0xa4);        // Location.Z
    iVar7 = CellClass__GetGroundHeight(&iStack_34);  // 0x00578080
    uVar11 = 1;
    if (iVar6 <= iVar7 + DAT_00b0ec38 * 3) goto LAB_00718c70;  // DAT_00b0ec38 = bridge Z offset
  }

  // BRANCH 2: Mission check — vtable+0x2c on TechnoClass = Get_Mission()
  iVar6 = (**(code **)(**(int **)(param_1 + 0xc) + 0x2c))();
  if (iVar6 != 0xf) {
    // Not MISSION_ENTER — drive path
    piVar10 = *(int **)(param_1 + 0xc);
    if ((char)piVar10[0x9f] == '\0') {
      // piVar10[0x9f] = TechnoClass[0x9f] as int* = offset 0x9f*4 = 0x27C = ChronoInTransit flag
      // ChronoInTransit is clear: do infantry placement + find passable cell
      iVar6 = (**(code **)(*piVar10 + 0x84))();  // vtable+0x84 = GetOwningHouse
      uVar16 = *(undefined4 *)(iVar6 + 0x5b4);   // HouseClass+0x5b4 = zone info
      // ... convert destination to cell coords (sign-correct arithmetic shift)
      // Check if cell != sentinel
      if (cell != null_sentinel_cell) {
        // Get zone + find passable: FootClass__Find_Nearby_Passable_Cell (0x0056dc20)
        // ... calls MapClass__Get_CellClass (0x005657a0), MapClass__GetZoneID (0x0056d230)
        // ... FootClass__Find_Nearby_Passable_Cell result → updated cell coords
      }
      // If result cell != sentinel: update dest cache X/Y/Z with center leptons + ground height
      (**(code **)(**(int **)(param_1 + 0xc) + 0xf4))(piVar1);  // Head_To_Coord with valid dest
      piVar10 = MapClass__Get_CellClass(...);
      // Convert NW cell to center leptons: cell_x*256+128, cell_y*256+128
      *piVar1 = (short)(cell_x + ...) * 0x100 + 0x80;
      *(int*)(param_1+0x2c) = (short)(cell_y+...) * 0x100 + 0x80;
      *(unsigned4*)(param_1+0x30) = 0;
      uVar11 = CellClass__GetGroundHeight(piVar1);
      *(undefined4*)(param_1+0x30) = uVar11;
      (**(code **)(**(int **)(param_1+0xc) + 0xf0))(piVar1);  // vtable+0xf0 = SetDestination
    } else {
      // ChronoInTransit set: skip placement, just move
      (**(code **)(*piVar10 + 0xf0))(piVar1);
    }
    goto LAB_00719249;
  }

  // BRANCH 3: Mission == 0xf (MISSION_ENTER) — warp dispatch path
  uVar16 = 0;  // dock-commit flag
  // Infantry scatter: vtable+0x200 = IsInfantry(), vtable+0x1ec = Scatter()
  cVar3 = (**(code **)(**(int **)(param_1 + 0xc) + 0x200))();
  if (cVar3 != '\0') {
    (**(code **)(**(int **)(param_1 + 0xc) + 0x1ec))();  // scatter infantry at dest
  }

  // vtable+0x184 = GetCurrentMission() or radio state — secondary mission check
  iVar6 = (**(code **)(**(int **)(param_1 + 0xc) + 0x184))();
  if (iVar6 == 8 || iVar6 == 9 || iVar6 == 7 || iVar6 == 0x19) goto LAB_00718ce9;
  // Note: Ghidra shows iVar6==8 directly goes to LAB_00718ce9; 9/7/0x19 fall through
  // additional checks

LAB_00718ce9: // RADIO-STATE GATE for dock commit
  apiStack_44[0] = *(int **)(*(int *)(param_1 + 0xc) + 0x5a4);  // TechnoClass+0x5a4 = radio link
  if (apiStack_44[0] != NULL) {
    // Radio state 1 check: vtable+0x2c on radio target = Get_Mission()
    iVar6 = (**(code **)(*apiStack_44[0] + 0x2c))();
    if (iVar6 == 1) {
      // Check if destination cell matches radio target's NW cell
      // vtable+0x1b8 on radio target = Get_Cell_Packed (returns NW cell)
      psVar8 = (**(code **)(*apiStack_44[0] + 0x1b8))(auStack_3c);
      if (dest_cell_X == psVar8[0] && dest_cell_Y == psVar8[1]) {
        uVar16 = 1;  // commit dock
      }
    }
    // Radio state 2 check (same pattern)
    iVar6 = (**(code **)(*apiStack_44[0] + 0x2c))();
    if (iVar6 == 2) { /* same cell match → uVar16 = 1 */ }
    // Radio state 6 check: building lookup
    iVar6 = (**(code **)(*piVar10 + 0x2c))();
    if (iVar6 == 6) {
      CellClass__Get_Cell_At(unaff_retaddr);
      piVar9 = Look_up_building_in_cell();  // 0x0047c520
      if (piVar9 == piVar10) {  // building at dest == radio target
        uVar16 = 1;
      }
    }
  }

  // Infantry placement using dock-commit flag (uVar16)
  CellClass__Get_Cell_At(&iStack_34);
  piVar10 = CellClass__PlaceInfantryInCell(local_1c, &iStack_34, uVar16, uVar11, 0);  // 0x00481180
  *piVar1 = *piVar10;       // update dest X
  *(param_1+0x2c) = piVar10[1];  // update dest Y
  *(param_1+0x30) = piVar10[2];  // update dest Z

  // Try to find passable cell for the placed result
  // MapClass__Get_CellClass → check occupancy (vtable+0x1ac)
  if (cell occupied) {
    // Reset dest to sentinel
    *piVar1 = g_NullCoord_Teleport_X;
    *(param_1+0x2c) = g_NullCoord_Teleport_Y;
    *(param_1+0x30) = g_NullCoord_Teleport_Z;
  }
  // If still null and TechnoClass[0x169] (NavCom) exists with flag bit 2 set:
  //   FootClass__Find_Nearby_Passable_Cell fallback → CellClass__PlaceInfantryInCell again

LAB_00719249:  // FINAL sentinel check
  if (dest == NullCoord sentinel) {
    // Read TechnoClass location again, call vtable+0xf0 (stop locomotion)
    return 0;  // no valid destination
  }
  (**(code **)(**(int **)(param_1 + 0xc) + 0xf0))(piVar1);
  return 1;  // valid destination committed
}
```

---

## Behavioral analysis

### Path A — No cached destination (sentinel match at entry)
When `+0x28/+0x2C/+0x30` all equal the sentinel triple (g_NullCoord_Teleport_X/Y/Z), the locomotor has no pending warp destination. It reads the TechnoClass current Location (`+0x9C/+0xA0/+0xA4`, leptons, NW-corner frame for buildings) and uses that as the working coordinate. This path falls through to the final sentinel check and returns 0 — no action this tick.

### Path B — Mission ≠ ENTER (drive path)
The unit has a cached destination but is not in MISSION_ENTER (0xf). This is the normal drive case. The function:
1. Checks `TechnoClass+0x27C` (ChronoInTransit flag, verified: `piVar10[0x9f]` with int* arithmetic → 0x9f×4=0x27C).
2. If ChronoInTransit is clear: converts destination to cell coords, calls `FootClass__Find_Nearby_Passable_Cell` (0x0056dc20) as fallback if primary cell is blocked, then updates dest cache with cell center leptons (cell×256+128) plus ground height from `CellClass__GetGroundHeight` (0x00578080).
3. If ChronoInTransit is set: skips placement, calls vtable+0xf0 (locomotion move) directly.
4. Returns at LAB_00719249.

### Path C — Mission = ENTER (warp dispatch)
The unit is in MISSION_ENTER (value 0xf from vtable+0x2c = `Get_Mission()`). This is the warp state. The function:
1. If infantry (vtable+0x200 = IsInfantry check), scatters infantry at destination (vtable+0x1ec = Scatter).
2. Reads secondary mission state via vtable+0x184. If the result is 8, 9, 7, or 0x19, jumps to LAB_00718ce9 (radio-gate check). Value 8 is the direct dispatch; 9, 7, 0x19 are fallthrough matches.
3. At LAB_00718ce9: reads the radio-linked object pointer from `TechnoClass+0x5a4`. Checks three radio states (1, 2, 6) against the linked building's mission/state via vtable+0x2c. States 1 and 2 additionally verify the destination cell matches the building's NW cell (`vtable+0x1b8 = Get_Cell_Packed`). State 6 uses `Look_up_building_in_cell` (0x0047c520) to confirm the building at the destination is the radio target.
4. Sets `uVar16` (dock-commit flag) to 1 if any radio-state gate passes.
5. Calls `CellClass__PlaceInfantryInCell` (0x00481180) with the dock-commit flag and bridge flag.
6. If the resulting cell is occupied (MapClass vtable+0x1ac occupancy check), resets dest to sentinel.
7. If sentinel and NavCom (TechnoClass+0x5a4 via int* 0x169 = offset 0x5A4) has bit 2 set, runs `FootClass__Find_Nearby_Passable_Cell` (0x0056dc20) fallback.

### Bridge Z-offset logic
When the destination cell has `CellClass+0x140 & 0x100` set (bridge overlay), the function checks whether the unit's current Z coordinate (`TechnoClass+0xA4`) exceeds `CellClass__GetGroundHeight + DAT_00b0ec38×3`. This gate decides whether to treat the destination as at bridge height (uVar11=1) or ground level (uVar11=0). `DAT_00b0ec38` is `g_BridgeZOffset_Teleport` (verified via manifest note and decompile_function 0x00718260).

### Return value
- Returns 1 when a valid destination is committed to `+0x28/+0x2C/+0x30`.
- Returns 0 when the destination ends up as the sentinel (no valid placement found).
- The caller (HeadToCoord @ 0x00718100) gates `IsMoving = 1` on the non-sentinel return.

---

## Struct field accesses

All `param_1` offsets are direct byte offsets (param_1 is `int`, not `int*`). All `piVar10` (TechnoClass) accesses are `int*` → multiply by 4 for byte offset.

| Field | Source | Offset | Unit | Frame | Notes |
|---|---|---|---|---|---|
| `param_1 + 0x0C` | TeleportLocomotionClass | +0x0C | ptr | — | Pointer to owning TechnoClass |
| `param_1 + 0x28` | TeleportLocomotionClass | +0x28 | leptons | abs-coord | Cached destination X |
| `param_1 + 0x2C` | TeleportLocomotionClass | +0x2C | leptons | abs-coord | Cached destination Y |
| `param_1 + 0x30` | TeleportLocomotionClass | +0x30 | leptons | abs-coord | Cached destination Z |
| `piVar10[0x27]` = TechnoClass+0x9C | TechnoClass | +0x9C | leptons | NW-corner | Location.X (current position) |
| `piVar10[0x28]` = TechnoClass+0xA0 | TechnoClass | +0xA0 | leptons | NW-corner | Location.Y |
| `piVar10[0x29]` = TechnoClass+0xA4 | TechnoClass | +0xA4 | leptons | NW-corner | Location.Z |
| `piVar10[0x9f]` = TechnoClass+0x27C | TechnoClass | +0x27C | bool | — | ChronoInTransit flag (verified: 0x9f×4=0x27C per CLAUDE.md) |
| TechnoClass+0x5A4 | TechnoClass | +0x5A4 | ptr | — | Radio-linked object pointer |
| TechnoClass+0x5B4 | HouseClass (from vtable+0x84) | +0x5B4 | uint | — | Zone bitmap (via owning house) |
| `(int*)[0x169]×4` = TechnoClass+0x5A4 | TechnoClass | +0x5A4 | ptr | — | NavCom pointer (same as radio link, different path) |
| CellClass+0x140 | CellClass | +0x140 | uint | — | Cell overlay flags; bit 0x100 = bridge overlay |

---

## Vtable slots resolved

All vtable calls are on the owning TechnoClass (`**(int**)(param_1+0xc)`), or on the radio-link target (`*apiStack_44[0]`).

| Offset | Call site | Resolved meaning | Evidence |
|---|---|---|---|
| vtable+0x2C | TechnoClass | `Get_Mission()` — returns current mission enum value | decompile_function 0x004D94B0 uses same slot; HeadToCoord plate comment cross-refs |
| vtable+0x84 | TechnoClass | Get owning `HouseClass*` | result used as HouseClass (accesses +0x5B4 zone field) |
| vtable+0xF0 | TechnoClass/Loco | Locomotion set-destination / stop-moving | called with dest coords or empty; final path |
| vtable+0xF4 | TechnoClass/Loco | Locomotion Head_To_Coord | called to forward validated coords to locomotor |
| vtable+0x184 | TechnoClass | Secondary mission/state query (returns 8/9/7/0x19) | distinct from vtable+0x2c; gates LAB_00718ce9 |
| vtable+0x1AC | TechnoClass | Occupancy / passability check | result gated: if ≠0 → dest reset to sentinel |
| vtable+0x1EC | TechnoClass | Scatter() — scatter infantry at destination | called only when IsInfantry()=true |
| vtable+0x200 | TechnoClass | `IsInfantry()` — returns bool | guards Scatter call |
| vtable+0x1B8 | Radio target | `Get_Cell_Packed()` — returns NW cell (cell index) | per CLAUDE.md: ObjectClass::Get_Cell_Packed @ 0x0041BEA0 |
| vtable+0x2C | Radio target | Same `Get_Mission()` — on the linked building | checks radio states 1, 2, 6 |
| vtable+0x4C | TechnoClass | `GetCoords()` — foundation center leptons | inner passable-cell search path |

---

## Globals + enums + INI keys

| Symbol | Address | Role |
|---|---|---|
| `g_NullCoord_Teleport_X` | 0x00B0EBF8 | Sentinel X for "no cached destination" (corrected from 0x00B0EBD8 — verified via get_xrefs_to 0x00B0EBF8 returning 13 reads across Constructor/HeadToCoord/Process/Stop_Moving/StateMachineTick/Update_Position; the address 0x00B0EBD8 originally cited only has 3 unrelated reads in Process body) |
| `g_NullCoord_Teleport_Y` | 0x00B0EBFC | Sentinel Y (Y is at +0x04 from X, not the +0x02 short the original doc claimed) |
| `g_NullCoord_Teleport_Z` | see global-null-coord-teleport doc | Sentinel Z |
| `DAT_00b0ec38` | ~0x00B0EC38 | Bridge Z offset (g_BridgeZOffset_Teleport region; see global-bridge-z-offset-teleport doc) |
| Mission 0xF (15) | — | MISSION_ENTER — triggers warp dispatch path |
| Mission 8 | — | Secondary mission value triggering radio-gate check |
| Mission 9 | — | Secondary mission value triggering radio-gate check |
| Mission 7 | — | Secondary mission value triggering radio-gate check |
| Mission 0x19 (25) | — | Secondary mission value triggering radio-gate check |
| Radio state 1 | — | Entering building dock |
| Radio state 2 | — | Docked to building |
| Radio state 6 | — | Building-in-cell match gate |

---

## Out-of-scope refs

The following callees are general infrastructure, not teleport-locomotion-specific:

| Symbol | Address | Reason |
|---|---|---|
| `MapClass__Get_CellClass` | 0x005657A0 | General map utility (excluded per manifest) |
| `CellClass__Get_Cell_At` | 0x00565730 | General cell lookup |
| `CellClass__PlaceInfantryInCell` | 0x00481180 | General infantry placement utility |
| `FootClass__Find_Nearby_Passable_Cell` | 0x0056DC20 | General pathfinding fallback |
| `Look_up_building_in_cell` | 0x0047C520 | General cell→building lookup |
| `MapClass__GetZoneID` | 0x0056D230 | General zone query |
| `CellClass__GetGroundHeight` | 0x00578080 | General height query |

The mission state machine dispatched via vtable+0x184 and vtable+0x2c is out of scope for this decode — those are TechnoClass concerns.

---

## Unverified (YELLOW)

- **vtable+0x184 identity**: The secondary mission/state getter at TechnoClass vtable+0x184 was not directly decompiled in this session. Its identity (exact method name, what values 8/9/7/0x19 mean as an enum) is inferred from context. Values 8/9/7/0x19 look like mission-enum or sub-state values but are unconfirmed without reading the mission enum from Ghidra.
- **vtable+0xF0 / +0xF4 identity**: These appear to be locomotor control calls (move / head-to-coord) but the exact method names were not confirmed by reading their implementations in this session.
- **Radio state semantics** (1=entering, 2=docked, 6=building-at-cell): inferred from the guard patterns (state 1/2 check cell equality vs radio-target NW cell; state 6 does building lookup). Not cross-verified against RadioClass enum.
- **`DAT_00b0ec38`**: Referenced as `g_BridgeZOffset_Teleport` region but exact address vs the manifest's `g_BridgeZOffset_Teleport` at 0x00B0EC2C differs by 0xC. This 3×bridge-Z multiplier check may use a different adjacent global. Needs verification in decode-global-bridge-z-offset-teleport.
- **TechnoClass+0x5A4 dual role**: Used both as `radio link pointer` (in radio-state check) and referenced via `(int*)[0x169]` index (0x169×4=0x5A4, NavCom check). Same physical field, confirmed offset — but the two access patterns suggest this field is both radio link and nav-com pointer. The distinction needs clarification in the struct decode.
