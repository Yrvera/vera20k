# BuildingClass::Receive_Radio — Decode

**Proposed Ghidra label:** `BuildingClass__Receive_Radio` (already labeled)

---

## Summary

`BuildingClass::Receive_Radio` at `0x0043C2D0` is the radio dispatch handler for buildings. It implements a large switch on the radio message code. For the chrono-miner dock system the critical cases are:

- **Case `0x02` (HELLO):** no dedicated BuildingClass case; falls through to `TechnoClass__Receive_Radio`. Sent by the building itself during the `0x0E` contact-establishment step.
- **Case `0x0E` (CAN_DOCK):** the refinery admission gate. Checks HasPower, runs contact management, sends `NEED_TO_MOVE(0x13)` and then `MOVE_TO_CELL(0x12)` with payload cell `NW+(3,1)`. Only if `0x12` returns `0x14` (already there) does the building send `ENTER_DOCK(0x18)` and then directed `TIMING_SYNC(0x16)`.
- **Case `0x0F` (CANENTER/dock query):** alliance/power/capacity eligibility check; returns `1` (accept) or `10` (reject). Used internally by the dock-bay eligibility checker (`FUN_004DEE80`) as the final radio gate before a building is returned as a valid dock candidate.
- **Case `0x15` (PAD_ARRIVED):** for `DockUnload=yes` buildings (stock GAREFN/NAREFN), queues sender mission `0x10` (Mission_Unload) with arg `0`. No reciprocal `+0x2E4` link written.

Function is invoked via vtable — no direct callers. Vtable DATA ref at `0x007E4050` (confirmed `get_xrefs_to 0x0043C2D0`). Already labeled in Ghidra.

---

## Active in YR

**Yes.** Stock `GAREFN` and `NAREFN` both set `DockUnload=yes` and `Refinery=yes`. `CMIN` and `HARV` both have `Dock=NAREFN,GAREFN` in `rulesmd.ini`. All cases listed above are live in a normal YR skirmish with a chrono miner.

---

## Decompilation excerpt

Verified via `decompile_function 0x0043C2D0`. The function already carries a verified Ghidra plate comment. Key cases for the chrono-miner system:

### Case `0x02` (HELLO) — no explicit case

No `case 2:` in the switch. Falls through to the default at bottom:
```c
iVar10 = TechnoClass__Receive_Radio(param_2, param_3, param_4);
return iVar10;
```
The building sends `0x02` to the miner during case `0x0E` contact setup via `vtable+0x278`.

### Case `0x08` (ABANDON)

```c
case 8:
  if (Type[0x16a9] || Type[0x16ab]) {     // UnitRepair or Bunker
    // distance check: if < 0x180 leptons, return 1
    return 1;
  }
  TechnoClass__Receive_Radio(param_2, 8, param_4);
  if (!Type[0x16bd] && !Type[0x16a9] && !Type[0x16ab])
    return 1;
  return 0x17;   // QUEUED — only for factory/repair/bunker types
```
For stock GAREFN/NAREFN: returns `1` (not `0x17`), because none of the three flags are set.

### Case `0x0E` (CAN_DOCK) — admission gate

```c
case 0xe:
  TechnoClass__Receive_Radio(param_2, 0x0e, param_4);
  if (!building.HasPower) return 10;

  // UnitRepair/Bunker hard-reject gates (not GAREFN/NAREFN)
  if (Type[0x16a9] && Contains(sender) && radio_0x22_returns_10) return 10;
  if (Type[0x16ab] && Contains(sender) && !CanAutoDeployHere(sender)) return 10;

  // Hospital/Armory branch (not GAREFN/NAREFN)
  if (!Type[0x16c1] && !Type[0x16c2]) {
    // Standard non-helipad path:
    if (!Contains(sender) && FreeSlotOrSameSender(sender)) {
      radio 0x02 to sender;   // HELLO — add to contacts
      re-check Contains(sender);
    }
    if (Contains(sender) && (DockUnload || Weeder)) {
      // GetDockCoord side-check: compute dock coord, compare to sender.NavCom (+0x5A4)
      // Sets local sentinel if sender has a different non-null destination
      GetDockCoord(out_coord, narrowed_sender_if_foot);
      Get_CellClass(out_coord) -> side_check_cell;
      // (sentinel used later to gate early return on non-ROGER 0x13 reply)
    }
    // NEED_TO_MOVE check
    iVar10 = radio 0x13 to sender;
    if (iVar10 != 1 && local_sentinel == 0) return 1;  // not ready, not sentinel → defer

    *param_4 = this;  // write building ptr to out-param

    if (!DockUnload && !Weeder) {
      // Helipad branch (Type[0x16cb])
      if (Type[0x16cb]) {
        *param_4 = this;
        send 0x12 to sender;
        if (reply != 0x14) return 1;
        radio 0x18 to first contact;
        return 1;
      }
      return 1;
    }

    // GAREFN/NAREFN: DockUnload branch:
    packed_nw = vtable+0x1B8()  // Get_Cell_Packed — NW cell
    cell.x = packed_nw.x + 3
    cell.y = packed_nw.y + 1
    *param_4 = MapClass::Get_CellClass(cell)
    iVar10 = radio 0x12 (MOVE_TO_CELL) to sender with *param_4;
    if (iVar10 != 0x14) return 1;   // not yet at target cell → miner drives
    radio 0x18 (ENTER_DOCK) directed to sender;
    iVar10 = radio 0x16 (TIMING_SYNC) directed to sender;
    if (iVar10 == 1) return 1;
    // fallthrough: TIMING_SYNC not ROGER — play approaching anim
    vtable+0x174 on sender (animation trigger);
    return 1;
  }
  // Hospital/Armory path and contact-iteration path omitted (not GAREFN/NAREFN)
```

### Case `0x0F` (CANENTER) — dock eligibility check

```c
case 0xf:
  TechnoClass__Receive_Radio(param_2, 0xf, param_4);
  if (!IsAlly(sender)) return 0;
  if (GetMission() == 0x12 || GetMission() == 0x13) return 10;
  if (field_0x534 == 0) return 10;    // contact capacity (NumberOfDocks)
  if (g_MapEditorMode == 0 && !FreeSlotOrSameSender(sender) &&
      !Type[0x16ae] && !Type[0x16af]) return 10;
  // unit-type checks (zone, Teleporter, mind-control, veteran)
  if (DockUnload && sender.GetMission()==1 && sender.UnitType.Teleporter &&
      g_MapEditorMode==0 && building.field_0x118==0) return 1;
  // ... (further sub-checks by building type)
  return 0;  // default reject
```

This is the admission gate called by `FUN_004DEE80` (eligibility checker in `Find_Docking_Bay`) via `vtable+0x278` with `0x0F`. Returns `1` (accept) to indicate the building has capacity and the unit may dock. Returns `0` or `10` to reject.

Key field: `building.field_0x534` = contact capacity (tied to `NumberOfDocks`). Stock GAREFN/NAREFN have `NumberOfDocks=1`.

For chrono miner (Teleporter=yes, GetMission()==1 = MISSION_ENTER): case `0x0F` reaches the `DockUnload && sender.GetMission()==1 && sender.UnitType.Teleporter` branch and returns `1` if the building has no pending occupant (`field_0x118==0`).

### Case `0x15` (PAD_ARRIVED) — unload handoff

```c
case 0x15:
  if (GetMission() == 0x13) return 10;  // building in wrong mission state
  if (Type[0x16ae] || Type[0x16af]) return 1;   // Armory/alternate
  if (Type[0x16a9] || Type[0x16aa] || Type[0x16c1] || Type[0x16c2]) {
    field_0x6dd = 1;
    Queue_Mission(0x14, 0);
    piStack_4->Queue_Mission(0, 0);
    return 1;
  }
  if (Type[0x16ab]) {         // Bunker
    field_0x6dd = 1;
    Queue_Mission(0x14, 0);
    return 1;
  }
  if (Type[0x16b3]) {         // DockUnload — stock GAREFN/NAREFN path
    sender->Queue_Mission(0x10, 0);   // queue Mission_Deploy (unload)
    return 1;
  }
  // fallthrough to TechnoClass::Receive_Radio
```

For stock GAREFN/NAREFN: queues `Mission_Deploy(0x10)` on the arriving harvester. No `+0x2E4` reciprocal link is written.

---

## Behavioral analysis

### Dock admission flow for CMIN → GAREFN (state 2 close return)

1. `Mission_Harvest` state 2 sends `radio 0x02` to refinery (HELLO contact).
2. `Find_Docking_Bay` (vtable `0x528`) calls eligibility checker (vtable `0x52c` = `FUN_004DEE80`) which sends `radio 0x0F` to check capacity → must return `1`.
3. `Mission_Harvest` accepts the dock and miner drives toward refinery.
4. `FootClass::Mission_Enter` sends `radio 0x0E` (CAN_DOCK) on arrival approach.
5. Building: 
   - Sends `HELLO(0x02)` if not in contacts, adds to contacts.
   - GetDockCoord side-check: computes dock cell, sets allowance sentinel if miner has different destination.
   - Sends `NEED_TO_MOVE(0x13)` → miner updates NavCom and returns `1`.
   - Computes `NW+(3,1)` cell, sends `MOVE_TO_CELL(0x12)`.
   - If miner is already there (returns `0x14`): sends `ENTER_DOCK(0x18)` + `TIMING_SYNC(0x16)`.
6. On physical arrival at dock pad: miner sends `PAD_ARRIVED(0x15)`.
7. Building queues `Mission_Deploy(0x10)` on miner.
8. Miner executes `Mission_Deploy` → unloads ore → sends departure radio.

### `0x0E` accepted cell: NW+(3,1)

For a refinery at NW cell `(cx, cy)`:
- Accepted `0x12` destination cell = `(cx+3, cy+1)`.
- For stock GAREFN 4×3 at NW `(10,10)`: cell `(13,11)`.

This is computed from `vtable+0x1B8` (= `ObjectClass::Get_Cell_Packed` at `0x0041BEA0`) which returns the NW-corner packed cell (cell index, units of cells), then adds the hardcoded `(+3, +1)` offset to produce a `(short x, short y)` pair passed to `MapClass::Get_CellClass`.

**This path does NOT read:** `QueueingCell` (`BuildingType+0x1618/+0x161C`), `DockingOffset[]` (`BuildingType+0x1788`), `GetDockCoord` (`vtable+0xA8`) for the movement target, or foundation width/height. (Confirmed by live decompile and prior research docs.)

### Veteran dock override
The `0x0F` (CANENTER) gate does not check veteran status on the building — veteran status affects selection priority in the outer `Find_Docking_Bay` loop (via `Building+0x3D3`), not the building's own admit/reject decision.

---

## Struct field accesses

### BuildingClass (param_1, receiver building)

| Expression | Byte offset | Field | Notes |
|---|---|---|---|
| `param_1->HasPower` | direct field | HasPower bool | Used in `0x0E` hard-reject and `0x0F` check |
| `param_1->field_0x534` | `+0x534` | contact capacity / NumberOfDocks slot count | Used in `0x0F` to gate reject; `NumberOfDocks=1` for stock GAREFN/NAREFN |
| `param_1->field_0x118` | `+0x118` | pending-occupant count (0 = no harvester docked/entering) | Used in `0x0F` DockUnload+Teleporter branch to confirm free slot |
| `param_1->field_0x6dd` | `+0x6DD` | unload-active latch | Set to 1 in `0x15` for UnitRepair/Bunker/Hospital types |
| `param_1->field_0xe4/0xe8` | `RadioClass+0xE4`/`+0xE8` | contacts array ptr / capacity | Used in contact contains/free-slot checks |

### BuildingTypeClass (param_1->Type)

| Offset | Flag | Stock GAREFN/NAREFN | Notes |
|---|---|---|---|
| `+0x16A9` | `UnitRepair=` | No | Activates special `0x08`/`0x0E`/`0x0F` branches |
| `+0x16AA` | (UnitRepair adjacent) | No | Used in `0x15` group check |
| `+0x16AB` | `Bunker=` | No | Bunker dock path |
| `+0x16AE`, `+0x16AF` | `Armory=`/alt | No | Used in `0x0F` and `0x15` type gates |
| `+0x16B3` | `DockUnload=` | **Yes** | Main stock refinery branch gate in `0x0E`, `0x0F`, `0x15` |
| `+0x16B9` | (repair anim) | No | Used in case `0x0C` |
| `+0x16BB` | `Refinery=` | **Yes** | Used in `0x10` and `GetDockCoord` branch |
| `+0x16BC` | `Weeder=` | No | Shares DockUnload `0x0E` branch |
| `+0x16BD` | `WeaponsFactory=` | No | Gates `0x08 → 0x17` response |
| `+0x16C1`, `+0x16C2` | `Hospital=`/`Armory=` | No | Alternative dock-unload branch |
| `+0x16CB` | `Helipad=` | No | Helipad dock path in `0x0E` |

### Vtable slots used

| Slot | Byte offset | Purpose |
|---|---|---|
| `vtable+0x48` | `+0x048` | `GetCoords` — get lepton position |
| `vtable+0xA8` | `+0x0A8` | `BuildingClass::GetDockCoord` (side-check only, not `0x12` target) |
| `vtable+0x174` | `+0x174` | Animation trigger on sender (TIMING_SYNC fallback) |
| `vtable+0x184` | `+0x184` | `GetMission` |
| `vtable+0x1B8` | `+0x1B8` | `ObjectClass::Get_Cell_Packed` — NW cell index |
| `vtable+0x1E8` | `+0x1E8` | `Queue_Mission` |
| `vtable+0x274` | `+0x274` | `Transmit_Radio_ToFirst` — send to first contact |
| `vtable+0x278` | `+0x278` | Directed `Transmit_Radio` — send to specific target |
| `vtable+0x27C` | `+0x27C` | `Transmit_Radio_Impl` — directed to explicit target (used for `0x12`) |

---

## Globals / enums / INI

| Symbol | Role | Active in YR |
|---|---|---|
| `g_MapEditorMode` | Non-zero bypasses free-slot check in `0x0F` | Conditional |
| `g_RulesClass_Instance+0x1700` | Health ratio threshold for anim selection in `0x0C` | Yes |

**INI keys (verified from `ini/rulesmd.ini`):**

| Key | Value | Effect |
|---|---|---|
| `[GAREFN] DockUnload=yes` | yes | Stock DockUnload branch in `0x0E`, `0x0F`, `0x15` |
| `[NAREFN] DockUnload=yes` | yes | Same |
| `[GAREFN] Refinery=yes` | yes | `GetDockCoord` Refinery branch |
| `[NAREFN] Refinery=yes` | yes | Same |
| `[GAREFN] NumberOfDocks=1` | 1 | Contact capacity = 1 slot |
| `[NAREFN] NumberOfDocks=1` | 1 | Same |
| `[CMIN] Teleporter=yes` | yes | Activates Teleporter branch in `0x0F` |
| `[CMIN] Dock=NAREFN,GAREFN` | yes | Dock type list used by `Find_Docking_Bay` |

---

## Out-of-scope refs

- `TechnoClass__Receive_Radio` (`0x006F4AB0`): base radio dispatch, called for unhandled cases and as super-call in some cases. Separate decode task.
- `BuildingClass__GrandOpening` (case `0x03`): building animation on placement — not part of harvest radio sequence.
- `HouseClass__Is_Ally_ByObject` (case `0x0F`): alliance check — out of scope.
- `TechnoClass__CanAutoDeployHere` (cases `0x0E`, `0x0F`): Bunker deploy check — out of scope.
- `MapClass__Get_CellClass`: converts `(x,y)` cell coords to `CellClass*` — called in multiple places.
- `ObjectClass__Get_Cell_Packed` (`0x0041BEA0`): NW cell from object world coordinates. Separate decode task if needed.
- `BuildingClass__GetDockCoord` (`vtable+0xA8`, `0x00447B20`): used in GetDockCoord side-check only; not the `0x12` target. Separate doc exists.

---

## Unverified

YELLOW — not verified in this session:

- Case `0x02` (HELLO) behavior in `TechnoClass__Receive_Radio` default path — decompile was not pulled in this session for that specific path. Inferred from the `0x0E` contact-setup code which explicitly sends `vtable+0x278` with `0x02`.
- `building.field_0x534` exact struct name and whether it is the same as `RadioClass::Contacts` capacity (the Ghidra decompile shows `field_0xe8` for capacity inside `RadioClass`; `field_0x534` is a BuildingClass-level field also checked in `0x0F`). These may be two different capacity fields.
- The `building.field_0x118` = 0 condition in `0x0F` for Teleporter case — inferred as "no harvester currently entering/docked" but not cross-referenced against the struct layout.
- vtable slot for `BuildingClass::Receive_Radio` in the BuildingClass vtable. The DATA xref at `0x007E4050` confirms it is a vtable slot, but the slot index was not computed this session.
