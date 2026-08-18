# UnitClass::Scatter — Deep Dive Ghidra Report

**Function:** `UnitClass::Scatter`
**Address:** `0x00743A50`
**Size:** 494 instructions, 75 basic blocks, cyclomatic complexity 70
**Calling convention:** `__thiscall` (ECX = this), 3 stack params (RET 0xC)
**Similar function:** `InfantryClass::Scatter` at `0x0051D0D0` (similarity 0.897)

## Signature

```c
void __thiscall UnitClass::Scatter(
    UnitClass* this,           // ECX
    CoordStruct* threat_coord, // [ESP+4]  — source of threat (or NullCoord {0,0,0} = no direction)
    int threat,                // [ESP+8]  — nonzero = threat-based scatter
    int forced                 // [ESP+C]  — nonzero = force scatter unconditionally
);
```

**VTable slot:** `+0x174` (address `0x7F5DE4` in UnitClass vtable at `0x7F5C70`)

Called by `CellClass::Scatter_Objects` (0x481670) which iterates cell occupants
and calls `vtable+0x174` on each.

---

## Stack Frame Layout

```
ESP+0x00..0x47  — local variables (0x48 bytes)
ESP+0x48..0x4B  — saved EBX
ESP+0x4C..0x4F  — saved EBP
ESP+0x50..0x53  — saved ESI
ESP+0x54..0x57  — saved EDI
ESP+0x58..0x5B  — return address
ESP+0x5C..0x5F  — threat_coord (CoordStruct* / pushed as ptr to 12-byte struct)
ESP+0x60        — threat (low byte used as bool in many places)
ESP+0x64        — forced (low byte used as bool)
```

Key register assignments:
- **EBP** = `this` (UnitClass pointer)
- **EBX** = `&this->Locomotor` (at `this+0x674 = FootClass::Locomotor`)
- **ESI** = `threat_coord` pointer (loaded from stack at `0x743BAC`)
- **EDI** = various uses (direction index, loop counter)

---

## Complete Flow — Pseudocode

```c
void UnitClass::Scatter(CoordStruct* threat_coord, int threat, int forced)
{
    // ============================================================
    // PHASE 1: Early-exit checks
    // ============================================================

    // 1a. Can this unit scatter at all?
    //     vtable+0x28C = FUN_006f3280 "CanScatter"
    //     Checks: mission != None(0), mission != 6, mission != 16(0x10),
    //             and if IsTrain (TypeClass+0xC94) is set, returns false
    if (!this->CanScatter())        // vtable+0x28C → 0x6F3280
        return;

    // 1b. Query locomotor for IPiggyback interface
    //     If locomotor supports piggyback (e.g. TeleportLoco on top of DriveLoco),
    //     get the underlying locomotor's CLSID
    ILocomotion* loco = this->Locomotor;  // +0x674
    IPiggyback* piggy = NULL;
    if (loco != NULL) {
        HRESULT hr = LocomotionClass::QueryInterface_IPiggyback(&loco, &piggy);
        if (FAILED(hr) && hr != E_NOINTERFACE)
            DebugAssert(hr);
    }
    if (piggy == NULL)
        DebugAssert(E_POINTER);

    // 1c. Get the locomotion CLSID via IPiggyback::GetLocomotorID
    CLSID loco_clsid;
    piggy->lpVtbl->GetLocomotorID(piggy, &loco_clsid);  // IUnknown vtable+0xC

    // 1d. If locomotor is TeleportLocomotion, skip scatter entirely
    //     CLSID_TeleportLocomotion = {4A582747-9839-11D1-B709-00A024DDAFD1}
    if (loco_clsid == CLSID_TeleportLocomotion)
        goto cleanup;

    // ============================================================
    // PHASE 2: Mission/state gate checks
    // ============================================================

    // Get mission timer entry for current mission
    MissionTimerEntry* mte = MissionClass::GetMissionTimerEntry(this);
    //   Returns &g_MissionTimerTable[this->CurrentMission]
    //   (offset 0xAC = current mission enum)

    // 2a. Check if mission timer allows scatter
    //     MissionTimerEntry[mission].byte_9 = "allows scatter" flag
    //     If this flag is false, only threat-based scatter (threat!=0) proceeds
    if (mte->allows_scatter == false && threat == false) {
        // Mission doesn't allow passive scatter and there's no threat
        goto cleanup;  // release piggyback ref and return
    }

    // ============================================================
    // PHASE 3: Split into two major branches
    // ============================================================

    // Check if threat_coord is the null/invalid coordinate {0,0,0}
    bool has_threat_source = !(threat_coord->X == 0
                            && threat_coord->Y == 0
                            && threat_coord->Z == 0);

    if (has_threat_source) {
        // ========================================================
        // BRANCH B: Scatter AWAY from a specific threat source
        // ========================================================
        goto branch_b;
    }

    // ========================================================
    // BRANCH A: Scatter to random direction (no threat source)
    // ========================================================

    // --- Additional state checks for random scatter ---

    // A1. Check scatter cooldown timer
    //     this+0x388 = CDTimerClass (scatter/rate timer on FootClass)
    if (CDTimerClass::Remaining(&this->ScatterTimer) != 0)
        goto cleanup;  // still on cooldown

    // A2. If unit has a NavQueue target and not forced, don't scatter
    //     this+0x5A4 = NavQueue destination pointer
    if (this->NavTarget != NULL && forced == false)
        goto cleanup;

    // A3. Block scatter if deploying/deployed
    //     this+0x6E0 = UnitClass::DeployedFlag (IsFalling per one doc)
    //     this+0x6E1 = deploying/undeploying state
    //     this+0x6E2 = another deploy-related flag
    if (this->DeployedFlag || this->IsDeploying_6E1 || this->IsDeploying_6E2)
        goto cleanup;

    // A4. Check locomotor is currently moving
    //     ILocomotion vtable+0x60 = Is_Moving()
    if (loco == NULL) DebugAssert(E_POINTER);
    if (!loco->lpVtbl->Is_Moving(loco))
        goto cleanup;  // only scatter if locomotor reports moving

    // --- Compute random scatter destination (BRANCH A) ---

    // A5. Get unit's current position as cell coordinates
    //     vtable+0x4C = GetActionCoords → 0x4DBDF0
    CoordStruct pos;
    this->GetActionCoords(&pos, 0);  // vtable+0x4C

    // Convert lepton coords to cell coords:  cell = (coord + sign_extend) >> 8
    CellStruct current_cell;
    current_cell.X = (short)((pos.X + (pos.X >> 31 & 0xFF)) >> 8);
    current_cell.Y = (short)((pos.Y + (pos.Y >> 31 & 0xFF)) >> 8);

    // A6. Call FootClass::Find_Nearby_Passable_Cell
    //     with speed_type from UnitTypeClass+0x67C (SpeedType)
    //     and OnBridge flag from this+0x8C
    CellStruct found_cell;
    SpeedType speed = this->Type->SpeedType;  // *(this+0x6C4)->0x67C
    found_cell = FootClass::Find_Nearby_Passable_Cell(
        &current_cell,            // start cell
        speed,                    // speed type
        -1,                       // max_distance = unlimited
        0,                        // movement_zone param
        (char)this->OnBridge,     // on_bridge flag (this+0x8C)
        1, 1, 0, 1, 0, 1,        // various search flags
        NULL,                     // no exclude coord
        0, 0                      // extra params
    );

    // A7. Check if found cell is valid (not the sentinel {0,0})
    if (found_cell == g_InvalidCell)  // g_InvalidCell at 0x00B1CFB8 = {0,0}
        goto cleanup;

    // A8. Issue movement order
    //     vtable+0x480 = TechnoClass::Set_Destination (0x741970)
    CellClass* dest = MapClass::Get_CellClass(&found_cell, true);
    this->Set_Destination(dest);  // vtable+0x480
    goto cleanup;

branch_b:
    // ========================================================
    // BRANCH B: Scatter away from a specific threat source
    // ========================================================

    // B1. Check mission timer byte_7 — "blocks directional scatter"
    //     Different field from byte_9 checked in Phase 2
    MissionTimerEntry* mte2 = MissionClass::GetMissionTimerEntry(this);
    if (mte2->blocks_scatter != 0)
        goto cleanup;  // mission explicitly blocks directional scatter

    // B2. Additional bail-out conditions
    int rtti = this->WhatAmI();  // vtable+0x2C → returns 1 (RTTI_Unit)
    if (rtti == 1 && this->DeployFlag_6D1 != 0)
        goto cleanup;  // unit with deploy flag set (e.g. MCV deploying)

    // B2b. NavTarget gate
    //   If unit has a nav destination:
    //     - If scatter is NOT forced → bail (don't interrupt movement)
    //     - If scatter IS forced but +0x6AF flag is set → bail
    //     - Only if forced AND +0x6AF is clear → allow scatter
    if (this->NavTarget != NULL) {       // +0x5A4
        if (forced == false)
            goto cleanup;
        if (this->field_6AF != 0)        // +0x6AF
            goto cleanup;
    }

    // B3. Random 1-in-4 chance to skip scatter for towed units
    //     this+0x2B4 = TowTarget (tow/link pointer)
    if (this->TowTarget != NULL && threat == false) {
        int roll = ScenarioRandom.RandomRanged(1, 4);
        if (roll != 1)
            goto cleanup;  // 75% chance to NOT scatter when towed
    }

    // ========================================================
    // B4. Compute escape direction
    // ========================================================

    int direction_index;  // 0-7, indexing g_DirectionOffsets

    if (threat_coord->X == 0 && threat_coord->Y == 0 && threat_coord->Z == 0) {
        // Threat source is null coord — use locomotor's current facing
        //     RateTimer::Current reads the facing/heading timer at this+0x388
        uint facing_raw;
        RateTimer::Current(&this->FacingTimer, &facing_raw);
        // Convert 16-bit facing to 0-7 direction index:
        //   (facing >> 12) gives 0-15
        //   (+1) >> 1 gives 0-7
        //   & 7 ensures wrap
        direction_index = ((facing_raw >> 12) + 1) >> 1;
        direction_index &= 7;

        // Add random jitter: RandomRanged(0,2) - 1 = {-1, 0, +1}
        int jitter = ScenarioRandom.RandomRanged(0, 2);
        direction_index = (direction_index + jitter - 1) & 7;
    }
    else {
        // Compute direction AWAY from threat source using atan2
        CoordStruct my_pos;
        my_pos.X = this->Location.X;      // this+0x9C
        my_pos.Y = this->Location.Y;      // this+0xA0
        my_pos.Z = this->Location.Z;      // this+0xA4

        // atan2(threat_Y - my_Y, my_X - threat_X)
        //   Note: arguments are swapped vs standard — this gives
        //   "direction FROM threat TO self" in screen coordinates
        //   (positive X = east, positive Y = south in isometric)
        double dy = (double)(int)threat_coord->Y - (double)my_pos.Y;
        double dx = (double)my_pos.X - (double)(int)threat_coord->X;
        double angle = Math::atan2(dy, dx);  // custom atan2 at 0x4CAE30

        // Convert radians to WW 16-bit direction:
        //   direction = (angle - pi/2) * (-32768/pi)
        //   This rotates coordinate system so North=0, clockwise increases
        angle = angle - 1.5707963267948966;   // subtract pi/2 (0x7E2820)
        angle = angle * (-10430.06);          // multiply by ~-32768/pi (0x7E2818)
        uint raw_direction = Math::ftol(angle);  // float-to-long

        // Convert 16-bit direction to 0-7 index:
        //   (raw >> 12) + 1) >> 1 = 0-7
        direction_index = ((raw_direction >> 12) + 1) >> 1;

        // Add random jitter: RandomRanged(0,2) - 1 = {-1, 0, +1}
        int jitter = ScenarioRandom.RandomRanged(0, 2);
        direction_index = (direction_index + jitter - 1) & 7;
    }

    // ========================================================
    // B5. Search for a passable adjacent cell
    // ========================================================

    // Get current position and convert to cell
    CellStruct origin_cell;
    {
        CoordStruct pos2;
        this->GetActionCoords(&pos2, 0);  // vtable+0x4C
        origin_cell.X = (short)((pos2.X + (pos2.X >> 31 & 0xFF)) >> 8);
        origin_cell.Y = (short)((pos2.Y + (pos2.Y >> 31 & 0xFF)) >> 8);
    }

    // Get the CellClass for origin, check height/bridge
    CellClass* origin_cc = MapClass::Get_CellClass(&origin_cell);
    bool on_bridge = this->IsOnBridge(0);  // vtable+0xBC → 0x4DDC40
    int cell_height_level = (int)(signed char)origin_cc->HeightLevel;  // +0x11B

    // Get coords again for the height-aware cell search
    this->GetActionCoords(&origin_pos, 0);  // vtable+0x4C

    // Compute Z offset for bridge: if on_bridge + height != 0, multiply by bridge constant
    //   bridge_z = g_BridgeHeightOffset * (on_bridge + cell_height != 0 ? 4 : 0)
    //   g_BridgeHeightOffset at 0x00B1D0B8 (runtime-computed)
    int bridge_z = 0;
    if ((on_bridge & 0xFF) + cell_height_level != 0)
        bridge_z = g_BridgeHeightOffset * 4;

    CellStruct best_cell = g_InvalidCell;   // {0, 0} — fallback if no ideal found
    CellStruct ideal_cell = g_InvalidCell;  // {0, 0} — ideal cell (height-validated)

    // B5a. Loop through 8 directions starting from computed escape direction
    for (int i = 0; i < 8; i++) {
        int dir = (i + direction_index) & 7;

        // Look up cell offset from g_DirectionOffsets table at 0x89F688
        //   Table format: short[16] = { dx0, dy0, dx1, dy1, ... dx7, dy7 }
        //   Standard C&C directions:
        //     dir 0=N:  (0,-1)    dir 1=NE: (1,-1)
        //     dir 2=E:  (1, 0)    dir 3=SE: (1, 1)
        //     dir 4=S:  (0, 1)    dir 5=SW: (-1, 1)
        //     dir 6=W:  (-1, 0)   dir 7=NW: (-1,-1)
        CellStruct candidate;
        candidate.X = origin_cell.X + g_DirectionOffsets[dir * 2];
        candidate.Y = origin_cell.Y + g_DirectionOffsets[dir * 2 + 1];

        // B5b. Get CellClass for candidate
        CellClass* cand_cc = MapClass::Get_CellClass(&candidate);

        // B5c. Check if candidate cell is in playfield
        if (!MapClass::Is_Cell_In_Playfield(&candidate, 1))
            continue;  // out of bounds

        // B5d. Check if unit can enter this cell
        //     vtable+0x1AC = UnitClass::Can_Enter_Cell (0x73F0A0)
        //     Args: cell_class, direction, height_value
        int height_val = CellClass::Get_Effective_Height(cand_cc, 0);
        int move_result = this->Can_Enter_Cell(cand_cc, dir, height_val);
        //   vtable+0x1AC
        if (move_result != 0)  // MOVE_OK = 0
            continue;  // cell not passable

        // B5e. Track as fallback if no ideal cell found yet
        if (best_cell == g_InvalidCell)
            best_cell = candidate;

        // B5f. Height-aware validation (only when on flat ground)
        //     g_InvalidCell check: if cell sentinel is {0,0}, this always triggers
        if (g_InvalidCell.X == 0 && g_InvalidCell.Y == 0) {
            // Build a 3D coordinate for the candidate cell center
            CoordStruct cand_coord;
            cand_coord.X = (int)(short)candidate.X * 256 + 128;  // cell center X
            cand_coord.Y = (int)(short)candidate.Y * 256 + 128;  // cell center Y
            cand_coord.Z = bridge_z;                               // bridge Z offset

            // FUN_006d6410: "Coord_Snap_To_Cell_With_Height"
            //   Given a lepton coordinate, converts to cell while accounting for
            //   height level differences. Walks the coord downhill through height
            //   steps, returning the actual cell that can be reached given terrain
            //   elevation. Uses global map pointer at 0x887324 (DisplayClass/MapClass).
            CellStruct snapped;
            FUN_006d6410(&snapped, &cand_coord);

            // Check if snapped cell matches candidate (no height barrier)
            if (snapped.X != candidate.X || snapped.Y != candidate.Y)
                continue;  // height mismatch — can't actually reach this cell

            // Also check if destination cell has bridge overlay (bit 0x100 in flags)
            CellClass* snap_cc = MapClass::Get_CellClass(&candidate);
            if (snap_cc->Flags_140 & 0x100)
                continue;  // bridge cell — skip (can cause stuck units)

            // This cell passes all checks — it's the ideal destination
            ideal_cell = candidate;
            break;  // found ideal, stop searching
        }
    }

    // ========================================================
    // B6. Select final destination
    // ========================================================

    CellStruct final_dest;

    // Prefer ideal_cell; fall back to best_cell
    if (ideal_cell == g_InvalidCell) {
        final_dest = best_cell;
    } else {
        final_dest = ideal_cell;
    }

    // If no passable cell found at all, give up
    if (final_dest == g_InvalidCell)
        goto cleanup;

    // ========================================================
    // B7. Issue movement order
    // ========================================================

    // Queue mission MOVE (mission 2)
    //   vtable+0x1E8 = MissionClass::Queue_Mission (0x5B35E0)
    this->Queue_Mission(MISSION_MOVE /*2*/, 0 /*not queued*/);

    // Set destination to the found cell
    //   vtable+0x480 = TechnoClass::Set_Destination (0x741970)
    CellClass* dest_cc = MapClass::Get_CellClass(&final_dest, true);
    this->Set_Destination(dest_cc);

cleanup:
    // Release IPiggyback COM reference if acquired
    if (piggy != NULL)
        piggy->lpVtbl->Release(piggy);
    return;
}
```

---

## Decision Tree Summary

```
UnitClass::Scatter(threat_coord, threat, forced)
│
├─ CanScatter() == false?  → RETURN
│   (mission==None or mission==6 or mission==0x10 or IsTrain)
│
├─ Locomotor is TeleportLocomotion?  → RETURN
│   (CLSID check against {4A582747-9839-11D1-B709-00A024DDAFD1})
│
├─ MissionTimer.allows_scatter==false AND threat==false?  → RETURN
│
├─ threat_coord == {0,0,0}?  (BRANCH A: random scatter)
│   │
│   ├─ ScatterTimer still active?  → RETURN
│   ├─ Has NavTarget AND not forced?  → RETURN
│   ├─ DeployedFlag OR IsDeploying?  → RETURN
│   ├─ Locomotor NOT Is_Moving?  → RETURN
│   │
│   ├─ Call Find_Nearby_Passable_Cell(current_cell, SpeedType, -1, ...)
│   ├─ If found == InvalidCell → RETURN
│   └─ Set_Destination(found_cell)
│
└─ threat_coord != {0,0,0}?  (BRANCH B: directional scatter)
    │
    ├─ MissionTimer.byte_7 != 0?  → RETURN
    ├─ WhatAmI()==1 AND DeployFlag?  → RETURN  (MCV deploying)
    ├─ Has NavTarget AND (!forced OR field_6AF)?  → RETURN
    ├─ Has TowTarget AND !threat?  → 75% chance RETURN (Random 1-4 != 1)
    │
    ├─ COMPUTE ESCAPE DIRECTION:
    │   ├─ If threat_coord=={0,0,0}: use current facing + jitter
    │   └─ Else: atan2(from threat to self) + jitter
    │       direction = ((atan2_result >> 12) + 1) >> 1
    │       jitter = RandomRanged(0,2) - 1  →  {-1, 0, +1}
    │       final_dir = (direction + jitter) & 7
    │
    ├─ SEARCH 8 ADJACENT CELLS (starting at escape direction):
    │   for i in 0..8:
    │     dir = (i + final_dir) & 7
    │     candidate = origin + g_DirectionOffsets[dir]
    │     ├─ Not in playfield?  → skip
    │     ├─ Can_Enter_Cell != MOVE_OK?  → skip
    │     ├─ Set best_cell if first passable
    │     └─ Height validation via FUN_006d6410:
    │         ├─ Snapped cell != candidate?  → skip
    │         ├─ Cell has bridge overlay (0x100)?  → skip
    │         └─ Set ideal_cell, BREAK
    │
    ├─ final = ideal_cell or best_cell or InvalidCell
    ├─ If InvalidCell → RETURN
    │
    └─ Queue_Mission(MOVE)
       Set_Destination(final_cell)
```

---

## Key Data Structures & Globals

| Address | Name | Type | Description |
|---------|------|------|-------------|
| `0x00B1CFB8` | `g_InvalidCell` | `CellStruct {short,short}` | Sentinel `{0,0}` meaning "no cell" |
| `0x00B1CFE8` | `g_InvalidCoord` | `CoordStruct {int,int,int}` | Sentinel `{0,0,0}` meaning "no coordinate" |
| `0x0089F688` | `g_DirectionOffsets` | `short[16]` | 8 pairs of (dx,dy) cell offsets, indexed by direction 0-7 |
| `0x00B1D0B8` | `g_BridgeHeightOffset` | `int` | Runtime-computed bridge Z offset constant |
| `0x00A8B230` | `g_Scenario` | `ScenarioClass*` | Scenario singleton |
| `g_Scenario+0x218` | `ScenarioRandom` | `Random` | Deterministic RNG for sim (lockstep-safe) |
| `0x0087F7E8` | `g_Map` | `MapClass` | Map singleton (used for Get_CellClass, Is_Cell_In_Playfield, Find_Nearby_Passable_Cell) |
| `0x00887324` | `g_Map2` | `MapClass*` | Secondary map pointer (used by FUN_006d6410 height snap) |
| `0x007E9A90` | `CLSID_TeleportLocomotion` | `GUID` | `{4A582747-9839-11D1-B709-00A024DDAFD1}` |
| `0x007E2818` | `k_DirConvertScale` | `double` | `-10430.06` (approximately `-32768/pi`) |
| `0x007E2820` | `k_HalfPi` | `double` | `1.5707963...` = `pi/2` |
| `0x00A8E3A8` | `g_MissionTimerTable` | `MissionTimerEntry[]` | Indexed by mission enum, 8 bytes per entry |

## Object Field Offsets Used

| Offset | Size | Class | Field | Description |
|--------|------|-------|-------|-------------|
| `+0x008C` | byte | ObjectClass | `OnBridge` | Unit is on bridge surface |
| `+0x009C` | int×3 | ObjectClass | `Location` | X, Y, Z coordinates (leptons) |
| `+0x00AC` | int | MissionClass | `CurrentMission` | Mission enum value |
| `+0x02B4` | ptr | TechnoClass | `TowTarget` | Towed/linked unit target |
| `+0x0388` | timer | FootClass | `ScatterTimer` | Cooldown timer (CDTimerClass) |
| `+0x05A4` | ptr | FootClass | `NavTarget` | Navigation queue destination |
| `+0x0674` | ptr | FootClass | `Locomotor` | Active ILocomotion COM pointer |
| `+0x06AF` | byte | FootClass | `field_6AF` | Blocks forced scatter when NavTarget set (exact semantics unclear) |
| `+0x06C4` | ptr | TechnoClass | `Type` | UnitTypeClass pointer |
| `+0x06D1` | byte | UnitClass | `DeployFlag` | Deploy-in-progress flag |
| `+0x06E0` | byte | UnitClass | `DeployedFlag` | Unit is deployed (IsFalling?) |
| `+0x06E1` | byte | UnitClass | `DeployState1` | Deploy/undeploy state byte 1 |
| `+0x06E2` | byte | UnitClass | `DeployState2` | Deploy/undeploy state byte 2 |

## UnitTypeClass Fields Used

| Offset | Field | Used For |
|--------|-------|----------|
| `+0x67C` | `SpeedType` | Passed to Find_Nearby_Passable_Cell |
| `+0xC94` | `IsTrain` | Checked in CanScatter (convoy units don't scatter) |

## VTable Calls Made

| VTable Offset | Function | Address | Purpose |
|---------------|----------|---------|---------|
| `+0x028C` | `CanScatter` | `0x6F3280` | Pre-check: mission/train guard |
| `+0x002C` | `WhatAmI` | `0x746E20` | Returns RTTI type (always 1 for Unit) |
| `+0x004C` | `GetActionCoords` | `0x4DBDF0` | Get unit's navigation coordinates |
| `+0x00BC` | `IsOnBridge` | `0x4DDC40` | Check bridge surface flag (+0x684) |
| `+0x01AC` | `Can_Enter_Cell` | `0x73F0A0` | Passability check for candidate cell |
| `+0x01E8` | `Queue_Mission` | `0x5B35E0` | Queue MISSION_MOVE (2) |
| `+0x0480` | `Set_Destination` | `0x741970` | Set movement target cell |

## Helper Functions Called

| Address | Name | Purpose |
|---------|------|---------|
| `0x45AEA0` | `LocomotionClass::QueryInterface_IPiggyback` | Get piggyback COM interface |
| `0x5B3A00` | `MissionClass::GetMissionTimerEntry` | Lookup in g_MissionTimerTable by mission |
| `0x4C9480` | `CDTimerClass::Remaining` | Check if timer still active |
| `0x4C93D0` | `RateTimer::Current` | Read current value of rate timer |
| `0x56DC20` | `FootClass::Find_Nearby_Passable_Cell` | Search for passable cell from origin |
| `0x5657A0` | `MapClass::Get_CellClass` | Convert CellStruct to CellClass pointer |
| `0x578460` | `MapClass::Is_Cell_In_Playfield` | Bounds check for cell coordinates |
| `0x5F5F00` | `CellClass::Get_Effective_Height` | Cell height + bridge adjustment |
| `0x4CAE30` | `Math::atan2` | Custom atan2 (lookup table based) |
| `0x7C5F00` | `Math::ftol` | Float-to-long with FPU rounding |
| `0x65C7E0` | `Random::RandomRanged` | Deterministic random in range |
| `0x6D6410` | `Coord_Snap_To_Cell_With_Height` | Walk coord through height steps |

---

## Direction Computation Details

### Atan2-based direction (Branch B, directional scatter)

The escape direction is computed as the direction **from the threat TO the unit**
(i.e., directly away from the threat):

```
dx = unit.Location.X - threat_coord.X    // positive = unit is east of threat
dy = threat_coord.Y - unit.Location.Y    // positive = threat is south of unit
                                          // NOTE: Y-axis inverted from math convention
angle = Math::atan2(dy, dx)              // custom atan2, returns radians
```

The angle is then converted to a Westwood 16-bit direction value (0=North, clockwise):

```
raw_direction = ftol((angle - pi/2) * (-32768/pi))
```

And reduced to a 3-bit direction index (0-7):

```
facing_3bit = ((raw_direction >> 12) + 1) >> 1
```

This maps to the standard 8 compass directions:
- 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW

### Random jitter

Both branches apply a jitter of `{-1, 0, +1}` (uniform random) to the direction:

```
jitter = ScenarioRandom.RandomRanged(0, 2) - 1
final_direction = (direction + jitter) & 7
```

This means the unit scatters roughly away from the threat, but can deviate by
one compass direction in either direction for variety.

### Facing-based direction (Branch B, no threat coord)

When `threat_coord == {0,0,0}` but we're in Branch B, the direction is derived
from the locomotor's current facing timer:

```
facing_raw = RateTimer::Current(&this->FacingTimer)
direction = ((facing_raw >> 12) + 1) >> 1
direction = (direction & 7) + jitter
```

---

## g_DirectionOffsets Table (0x89F688)

This is a `short[16]` array (8 pairs of `{dx, dy}`) that maps direction index 0-7
to cell coordinate offsets. The table is **runtime-initialized** (BSS section),
populated during game startup. Expected values:

| Direction | Index | dx | dy | Name |
|-----------|-------|----|----|------|
| North     | 0     |  0 | -1 | N    |
| Northeast | 1     | +1 | -1 | NE   |
| East      | 2     | +1 |  0 | E    |
| Southeast | 3     | +1 | +1 | SE   |
| South     | 4     |  0 | +1 | S    |
| Southwest | 5     | -1 | +1 | SW   |
| West      | 6     | -1 |  0 | W    |
| Northwest | 7     | -1 | -1 | NW   |

Memory layout: `[dx0, dy0, dx1, dy1, ..., dx7, dy7]` as `short` values.

---

## Branch A vs Branch B Summary

| Aspect | Branch A (random) | Branch B (directional) |
|--------|-------------------|----------------------|
| **Trigger** | `threat_coord == {0,0,0}` | `threat_coord != {0,0,0}` |
| **Scatter timer check** | Yes (cooldown must expire) | No |
| **Deploy flag check** | `+0x6E0/6E1/6E2` | `+0x6D1` only |
| **Is_Moving check** | Yes (must be moving) | No |
| **Direction source** | N/A (uses Find_Nearby_Passable_Cell) | atan2 or current facing |
| **Cell search** | Find_Nearby_Passable_Cell (sophisticated) | 8-direction scan with height check |
| **Mission queued** | None (just Set_Destination) | MISSION_MOVE (2) |
| **Height validation** | Handled by Find_Nearby_Passable_Cell | Manual via FUN_006d6410 |
| **Random chance skip** | No | 75% skip if towed & !threat |

---

## Confidence Assessment

- **Overall flow:** 95% — assembly traced instruction-by-instruction
- **VTable offsets:** 95% — verified against UnitClass vtable at 0x7F5C70
- **Field offsets:** 90% — cross-referenced with existing docs
- **Direction math:** 90% — floating-point constants decoded, formula verified
- **Mission enum (MOVE=2):** 95% — confirmed in multiple docs
- **g_DirectionOffsets values:** 80% — standard C&C convention, but table is BSS
  (runtime-filled) so exact values not directly verified from binary
- **FUN_006d6410 purpose:** 75% — decompiled and understood flow, but complex
  height-walking logic; named "Coord_Snap_To_Cell_With_Height" provisionally
