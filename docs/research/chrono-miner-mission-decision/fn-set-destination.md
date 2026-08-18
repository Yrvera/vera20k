# TechnoClass__Set_Destination @ 0x00741970

**Proposed Ghidra label:** TechnoClass__Set_Destination

## Summary

`TechnoClass::Set_Destination` is the per-move-order destination setter for all `TechnoClass`
(and `FootClass`) units. For the **chrono miner**, it contains the critical
**teleport-vs-drive decision**: when the unit is a `Teleporter`-type (the `Teleporter=yes` INI
flag, `TechnoTypeClass+0xCD4`), it inspects the destination cell for a building occupant. If
a building is present the function swaps the active locomotor to **DriveLocomotionClass** (so
the unit drives into the building), otherwise it keeps **TeleportLocomotionClass** active (the
unit warps). The function also handles several other roles: stopping/cancelling pending moves,
refinery-unload animation clearing, bridge collision checks, and the WalkLocomotion destination
rate-limiting. The function ends by calling `FootClass::Set_Destination_Internal @ 0x004D94B0`
which sets `NavCom` (`FootClass+0x5A4`) and fires `ILocomotion::Head_To_Coord`.

**Active in YR:** Yes. `Teleporter=yes` is set on `CMIN` (Chrono Miner) in rulesmd.ini.
The entire teleport-vs-drive branch fires every time a chrono miner receives a move order.

---

## Decompilation excerpt

Verified via `decompile_function 0x00741970`.

Signature (Ghidra, `__thiscall` on `ObjectClass*` used as TechnoClass/FootClass):
```c
void __thiscall TechnoClass__Set_Destination(ObjectClass *param_1, int *param_2, char param_3)
```
- `param_1` = `this` (TechnoClass / FootClass instance)
- `param_2` = new destination pointer (0 = cancel move)
- `param_3` = modifier flag (used in bridge/teleport path selection)

Key code paths (selected excerpts):

**1. Short-circuit: same destination, no force flag**
```c
if ((param_2 == *(int **)&param_1[8].field_0x44) && ((char)param_1[2].Location_Y == '\0')) {
    return;
}
```
`param_1[8].field_0x44` is the current NavCom target. Early return if destination unchanged.

**2. Refinery-unload animation clear (mission == 0x10 or substate 0x10)**
```c
iVar6 = (**(code **)(param_1->vtable + 0x184))();  // GetMission()
if ((iVar6 == 0x10) || (param_1[1].vtable_INoticeSink == (undefined *)0x10)) {
    // look at adjacent cell, find building there, clear its unload anim slot
    MapClass__Get_CellClass();
    this = (BuildingClass *)Look_up_building_in_cell();
    if ((this != (BuildingClass *)0x0) && (...)) {
        BuildingClass__ClearAnimSlot(this);
    }
}
```
`Look_up_building_in_cell @ 0x0047C520` scans `CellClass+0xE4` for an object with RTTI type 6
(BuildingClass). Verified via `decompile_function 0x0047C520`.

**3. Teleporter block — the chrono miner locomotor swap**

Condition: `TechnoTypeClass+0xCD4 != 0` (Teleporter=yes) AND locomotor is not already Drive
AND not currently docked. Located near `0x742390` in the body.

```c
// from TechnoTypeClass (int*) param_1[0x335] = byte-offset 0xCD4. Verified via TechnoTypeClass__ReadINI @ 0x00713FE9
if ((param_1[10].vtable_INoticeSource[0xcd4] != '\0')   // TechnoTypeClass+0xCD4 = Teleporter bool
    && (param_1[3].field_0x78 == '\0')                  // not in limbo / docked
    && (param_1[4].vtable == (undefined *)0x0)           // no current NavCom
    && (*(char *)((int)&param_1[9].Location_Y + 1) == '\0')) {
    // ...
    // Query current locomotor CLSID via IPiggyback::GetClassID
    (**(code **)(*piStack_64 + 0xc))(piStack_64, aiStack_30);  // GetClassID -> aiStack_30

    // Check if current loco CLSID == CLSID_DriveLocomotion
    // if already Drive, skip locomotor swap
    if (!CLSID_match(aiStack_30, &CLSID_DriveLocomotion)) {
        // Current destination is non-null CellClass (building or empty)
        if (iVar6 == 6) {  // destination RTTI type 6 = CellClass
            // key decision:
            iVar6 = CellClass__FindFirstBuilding();  // 0x0047EBA0
            if (iVar6 == 0) {
                // cell is EMPTY: keep TeleportLocomotion → warp
                // check if current loco is TeleportLocomotion
                // if not TeleportLocomotion:
                //   - release existing piggyback
                //   - fire vtable+0x1F0 (stop/reset Teleport countdown)
                //   - set mission-related flags
            } else {
                // cell HAS BUILDING: swap to DriveLocomotionClass
                // - CoCreateInstance(CLSID_DriveLocomotion) -> new Drive loco
                // - Query new Drive for IPiggyback
                // - Piggyback the TeleportLocomotion UNDER Drive
                // - Set new loco as active (FootClass+piggybacked ptr update)
            }
        }
    }
}
```

**4. Locomotor swap detail (building-present branch, ~0x742450)**

```c
// Create Drive locomotor
iVar6 = COM__CoCreateInstance_Locomotor(&CLSID_DriveLocomotion, 0, 7);
// ...
(**(code **)(*piStack_68 + 0xc))(piStack_68, param_1);  // Loco::Link(unit)
// Query new Drive for IPiggyback interface
FUN_0045a050 / QI for IID_IPiggyback
// Set old TeleportLoco as piggybacked child
(**(code **)(*piStack_78 + 0xc))(piStack_78, *(undefined4 *)pbVar1);  // Piggyback->Link(TeleportLoco)
// Update FootClass loco pointer to new Drive
*(int **)pbVar1 = piStack_68;
```
After this swap the unit's locomotor chain is: **Drive (outer) → Teleport (piggybacked)**. When
Drive finishes navigating into the building, the radio handshake via `BuildingClass::Receive_Radio`
will swap back to TeleportLocomotion as the outer loco.

**5. Terminal call**
```c
FootClass__Set_Destination_Internal(unaff_retaddr, param_2);  // 0x004D94B0
```
Sets `FootClass+0x5A4` (NavCom = `param_2`) and calls `ILocomotion::Head_To_Coord` on the
active locomotor. Verified via `decompile_function 0x004D94B0`.

---

## Behavioral analysis

### Teleport-vs-drive decision (chrono miner)

The branching logic for a `Teleporter` unit receiving a move order to a destination:

```
Teleporter=yes AND not docked AND no current NavCom
└─ Current loco == DriveLocomotion?
   ├─ YES → skip swap (already driving to a building)
   └─ NO  → destination RTTI == CellClass?
            ├─ YES → CellClass::FindFirstBuilding(destination_cell)?
            │        ├─ building present (non-null) → SWAP loco to Drive + piggyback Teleport
            │        │  → unit drives into building normally
            │        └─ empty cell → keep TeleportLocomotion
            │           → (if loco was not already Teleport: reset teleport state machine)
            │           → unit warps to destination
            └─ NO  → (destination is not a cell; handle separately)
```

**Why this matters for the chrono miner:** A CMIN move-order to an empty field cell warps
(TeleportLocomotion stays outer). A move-order to a refinery cell swaps to Drive so the
existing dock-entry handshake (`BuildingClass::Receive_Radio`) can fire. Without this swap
the miner would attempt to teleport INTO the refinery building, bypassing the dock approach
and breaking the unload sequence.

### Active-in-YR verdict

**Active: Yes.** The `Teleporter` flag is read from `TechnoTypeClass+0xCD4` at runtime.
`CMIN` has `Teleporter=yes` in `rulesmd.ini`. This branch fires for every chrono miner
move command in a normal YR skirmish.

### Other roles of this function

- **Refinery-unload animation clear**: on Mission==0x10 (Harvest), clears the refinery's
  unload animation slot when a new destination is set (via
  `Look_up_building_in_cell @ 0x0047C520` + `BuildingClass::ClearAnimSlot @ 0x00451E40`).
- **Hover locomotor path**: when locomotor is HoverLocomotion and the current cell is not a
  bridge, creates a DriveLocomotionClass + piggyback (separate from the Teleporter path).
  Also active in YR for hover vehicles.
- **Walk locomotor rate-limiting**: for WalkLocomotion units, respects a per-unit rate timer.
- **Bridge-crossing check**: detects when source and destination are on the same bridge
  segment within 2 cells, uses `MapClass::FindBridgeRecord @ 0x0056DA10`.
- **Ghost cell / building-target handling**: handles targeting BuildingClass directly
  (RTTI==6, `BuildingTypeClass+0x16a9`, `+0x16ab`, `+0x16b3` flags = `Dock`, `Guard`, etc.)
  via TechnoClass::SetGhostCell and mission-start calls.

---

## Struct field accesses

All offsets as byte offsets from `this` (TechnoClass / FootClass instance).

| Expression in decompile | Byte offset | Field | Notes |
|---|---|---|---|
| `param_1[10].vtable_INoticeSource[0xcd4]` | TechnoTypeClass\* + 0xCD4 | `TechnoTypeClass::Teleporter` (bool) | from `int*` param: `0x335 × 4 = 0xCD4`; verified via `decompile_function 0x00713FE9` (TechnoTypeClass__ReadINI) |
| `param_1[8].field_0x44` | instance + `8×sizeof(ObjectClass) + 0x44` | current NavCom destination ptr | Ghidra objectclass-array indexing; this is `FootClass+0x5A4` area |
| `param_1[3].field_0x78` | instance + `3×sizeof(ObjectClass) + 0x78` | limbo/docked flag | checked to skip locomotor swap when unit is already docked |
| `param_1[9].BombVisible` (used as `ILocomotion*`) | TechnoClass+0x... | Active locomotor pointer (`FootClass+0x19D` area = `FootClass::Locomotor`) | used to query CLSID and IPiggyback |
| `param_1[9].Location_Y+1` | chrono-state byte | chrono in-transit / ChronoInTransit flag | per TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md: `TechnoClass+0x27C` |
| `piVar11[0x148] + 0x16a9` | `BuildingTypeClass+0x16a9` | `Harvester` flag on BuildingTypeClass | gates "is this a dock we can enter?" |
| `piVar11[0x148] + 0x16ab` | `BuildingTypeClass+0x16ab` | `Guard` flag | gates escort behavior |
| `piVar11[0x148] + 0x16b3` | `BuildingTypeClass+0x16b3` | `Dock` flag | gates enter-building send-radio path |
| `piVar11[0x148] + 0x16bd` | `BuildingTypeClass+0x16bd` | unknown building flag | read during destination finalization |

**Coordinate frame note:** the `*psVar8` / `psVar9` used for the adjacent-cell offset look-up
in the refinery-unload anim clear block uses **Get_Cell_Packed** (NW-cell-indexed) result,
shifted by `g_refinery_unload_adjacent_lookup_dx/dy` globals.

---

## Globals / enums / INI

| Global / INI key | Address / offset | Role |
|---|---|---|
| `TechnoTypeClass::Teleporter` | TechnoTypeClass + 0xCD4 (bool) | INI key `Teleporter=`; verified via `decompile_function 0x00713FE9` |
| `CLSID_DriveLocomotion` | data label referenced at ~0x742460 | GUID for DriveLocomotionClass; used with `COM__CoCreateInstance_Locomotor @ 0x0041C250` |
| `CLSID_TeleportLocomotion` | data label referenced at ~0x742530 | GUID for TeleportLocomotionClass; CLSID comparison after locomotor swap |
| `CLSID_HoverLocomotion` | data label referenced at ~0x742820 | GUID for HoverLocomotionClass; Hover path (separate from Teleporter path) |
| `CLSID_WalkLocomotion` | referenced in `FootClass__Set_Destination_Internal @ 0x004D94B0` | rate-limiting path for Walk units |
| `g_refinery_unload_adjacent_lookup_dx/dy` | referenced ~0x741C40 | cell offsets for refinery unload anim clear |
| `DAT_00b1cfb8`, `DAT_00b1cfba` | constants used in passable-cell return check | sentinel "invalid cell" X/Y values |
| `g_RulesClass_Instance + 0x16F8` | double; health threshold for dock vs guard | used in building-target health check (compare with `ObjectClass::GetHealthRatio`) |

---

## Out-of-scope refs

These callees or mechanisms are referenced by this function but belong to other decode tasks:

- `FootClass::Find_Nearby_Passable_Cell @ 0x0056DC20` — out-of-scope (task #4)
- `MapClass::GetZoneID @ 0x0056D230` — zone system, out of scope
- `TechnoClass::SetGhostCell @ 0x0070C610` — ghost/shadow cell setter, out of scope
- `BuildingClass::ClearAnimSlot @ 0x00451E40` — refinery anim, out of scope
- `FUN_007447b0 @ 0x007447B0` — unknown helper called from bridge-crossing param_2 fixup
- `FUN_00746000 @ 0x00746000` — hover locomotor path helper
- `DynamicVectorClass::Contains @ 0x0065AD50` — generic container check
- `Filter_AbstractType_InMap @ 0x0040DD70` — AbstractType map filter (building type lookup)
- `FootClass::Enter_Destination @ 0x004DA0E0` — refinery/dock enter step (task #48)
- `BuildingClass::DeployUnit_ChronoWarp @ 0x0070FEE0` — called via `FootClass::Set_Destination_Internal` (task #53)

---

## Unverified (YELLOW)

- The exact byte offset computed for `param_1[8].field_0x44` (NavCom field) depends on the
  size Ghidra assigns to `ObjectClass`. The access is consistent with `FootClass+0x5A4` based
  on context but the `ObjectClass` element-size is not verified in this session.
- `param_1[3].field_0x78` (docked/limbo flag byte) — byte-offset from TechnoClass not
  directly computed here; identified by context ("not in limbo / docked" gate). YELLOW.
- `BuildingTypeClass+0x16a9 / +0x16ab / +0x16b3` field names (`Harvester`, `Guard`, `Dock`)
  inferred from context (how they gate behavior). Not cross-verified via ReadINI in this session. YELLOW.
- The exact address range of the Teleporter branch (cited as "~0x742390") is approximate;
  the decompilation confirms the logic but the label address is inferred from code flow, not
  from a named Ghidra label at that exact offset.
