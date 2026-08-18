# FootClass Complete Vtable Map

**Source:** Ghidra decompilation of gamemd.exe  
**Vtable address:** `0x007E8C94` (primary), `0x007E8C78` (+4), `0x007E8C70` (+8), `0x007E8C68` (+12)  
**Constructor:** `0x004D31E0`  
**Total entries:** 310 (indices 0-309), spanning offsets `0x000`-`0x4D4`  
**Parent vtable:** TechnoClass at `0x007F4960` (309 entries, 0-308)  
**New virtuals added by FootClass:** 1 (index 309, offset 0x4D4)  
**Overridden entries:** 67 slots overridden from TechnoClass  
**Confidence:** High - addresses read directly from vtable memory, cross-checked against TechnoClass vtable and constructor.  
**Date:** 2026-04-06

## Inheritance Hierarchy

```
AbstractClass          (entries ~0-22)
  ObjectClass          (entries ~23-68)
    MissionClass       (entries ~69-156, includes mission handlers)
      RadioClass       (entries ~157-160)
        TechnoClass    (entries ~161-308)
          FootClass    (entry 309 = new; 67 overrides)
```

## Overview

FootClass is the base class for all **mobile** game objects: InfantryClass, UnitClass, AircraftClass.
It overrides 67 TechnoClass vtable slots and adds 1 new virtual method. The overrides concentrate
on: serialization, movement/pathfinding, mission handlers, damage processing, and locomotion.

FootClass object size through its fields: **0x6C0 = 1728 bytes** (subclass fields start at 0x6C0).

---

## Complete Vtable — FootClass Overrides Only

The following table lists ONLY the 67 slots where FootClass provides a different implementation
from TechnoClass, plus the 1 new virtual. For all other slots, see TECHNOCLASS_VTABLE_COMPLETE.md.

Legend:
- **OVERRIDE** = FootClass replaces a TechnoClass/parent implementation
- **NEW** = FootClass adds a new virtual method not present in TechnoClass

### IPersistStream Overrides

| Idx | Offset | Address    | Name | Type | Description |
|-----|--------|------------|------|------|-------------|
| 5   | 0x014  | 0x004DB3C0 | FootClass::Load | OVERRIDE | Deserialize from stream — loads path vectors, locomotion COM object via OleLoadFromStream, swizzle pointers |
| 6   | 0x018  | 0x004DB690 | FootClass::Save | OVERRIDE | Serialize to stream — saves path vectors, locomotion via OleSaveToStream |

### AbstractClass Overrides

| Idx | Offset | Address    | Name | Type | Description |
|-----|--------|------------|------|------|-------------|
| 8   | 0x020  | 0x004E0170 | FootClass::ScalarDeletingDestructor | OVERRIDE | Destructor dispatch |
| 10  | 0x028  | 0x004D9960 | FootClass::PointerExpired | OVERRIDE | Nullify dangling pointers — handles team leader, NavCom target, path queue entries, passenger list cleanup |
| 13  | 0x034  | 0x004DBAD0 | FootClass::ComputeChecksum | OVERRIDE | Compute state hash for lockstep sync — includes heading, speed, destination coords, locomotion state |

### ObjectClass Overrides

| Idx | Offset | Address    | Name | Type | Description |
|-----|--------|------------|------|------|-------------|
| 19  | 0x04C  | 0x004DBDF0 | FootClass::GetDestinationCoords | OVERRIDE | Returns locomotion destination coords; checks tube index first, then queries ILocomotion::Destination() |
| 21  | 0x054  | 0x004DE620 | FootClass::IsHighFlying | OVERRIDE | JMP thunk to ObjectClass__IsHighFlying at 0x005F6B90 — checks locomotion height via ILocomotion vtable; returns 1 if height >= DAT_00ac13c8*2, else 0 (corrected 2026-05-28: was "returns false stub"; binary at 0x004DE620 is a 5-byte JMP to 0x005F6B90 via read_memory 0x004DE620 + get_function_by_address 0x005F6B90 — ROOT_CAUSE: INFERENCE_HARDENED) |
| 23  | 0x05C  | 0x004DA530 | FootClass::AI | OVERRIDE | Main per-tick AI — locomotion AI, path following, team management, speed calculation, scatter, zone checking |
| 26  | 0x068  | 0x004DA4E0 | FootClass::GetVisualState | OVERRIDE | Get visual/animation state — checks ILocomotion for current visual state |
| 27  | 0x06C  | 0x004DED70 | FootClass::GetCursorForCell | OVERRIDE | Determine cursor when hovering over cell — checks shroud, allied status, CloakStop |
| 28  | 0x070  | 0x004DDDE0 | FootClass::What_Action_OnCell | OVERRIDE | Determine action for cell — wraps parent with shroud/CloakStop visibility check |
| 29  | 0x074  | 0x004DDED0 | FootClass::What_Action_OnObject | OVERRIDE | Determine action for object — wraps parent with shroud visibility check |
| 30  | 0x078  | 0x004DB7E0 | FootClass::GetThreatLevel | OVERRIDE | Returns threat level — queries locomotion interface for threat value |
| 38  | 0x098  | 0x004D9E70 | FootClass::IsSurfacing | OVERRIDE | Checks if unit is surfacing from underwater (locomotion state check) |
| 47  | 0x0BC  | 0x004DDC40 | FootClass::ShouldBeOnBridge | OVERRIDE | Bridge height check — returns false if in a tunnel (tube index >= 0), otherwise defers to parent |
| 48  | 0x0C0  | 0x0041C070 | FootClass::GetFoundation | OVERRIDE | Returns 0 (no foundation for mobile units) |
| 53  | 0x0D4  | 0x004DB260 | FootClass::Limbo | OVERRIDE | Remove from map — decrements adjacent cell counters, stops locomotion, clears scatter flag |
| 54  | 0x0D8  | 0x004D7170 | FootClass::Unlimbo | OVERRIDE | Place on map — increments adjacent cell counters, initializes locomotion, sets speed from TypeClass |
| 55  | 0x0DC  | 0x004D9720 | FootClass::Destroy | OVERRIDE | Handle destruction — releases team members if leader, sets scatter direction, calls parent |
| 62  | 0x0F8  | 0x004DE5D0 | FootClass::UnInit | OVERRIDE | Final cleanup — frees CaptureManager, chrono warp, team membership, then parent UnInit |
| 69  | 0x114  | 0x004DB250 | FootClass::OnLoadNotify | OVERRIDE | Post-load notification — returns 0 (stub) |
| 73  | 0x124  | 0x004D3780 | FootClass::ProcessCloakAndNotify | OVERRIDE | Cloak processing for moving units — calls DoCloak on self |
| 78  | 0x138  | 0x004DFA50 | FootClass::CanBeSelected | OVERRIDE | Selection check — returns false if unit is "intransit" (being paradropped), otherwise defers to parent |
| 80  | 0x140  | 0x004D7D50 | FootClass::ClickedAction_Cell | OVERRIDE | Handle player click on cell — large switch on action type (move, attack, harvest, deploy, enter tunnel, C4, etc.) |
| 81  | 0x144  | 0x004D74E0 | FootClass::ClickedAction_Object | OVERRIDE | Handle player click on object — large switch on action type (attack, enter transport, capture, repair, etc.) |
| 85  | 0x154  | 0x004DEAE0 | FootClass::IronCurtain | OVERRIDE | Apply invulnerability — detaches active chrono warp (WarpAttachClass::Detach), records frame counter in fields 0x1a8-0x1aa, then delegates to TechnoClass::IronCurtain; NOT a stub (corrected 2026-05-28: was "returns false (stub, 4 bytes)"; binary at 0x004DEAE0 has sub-esp prologue and calls TechnoClass__IronCurtain via decompile_function 0x004DEAE0 — ROOT_CAUSE: INFERENCE_HARDENED) |
| 91  | 0x16C  | 0x004D7330 | FootClass::ReceiveDamage | OVERRIDE | Damage processing — handles team scatter on damage, passenger damage forwarding |
| 99  | 0x18C  | 0x004D85D0 | FootClass::PerCellProcess | OVERRIDE | Per-cell movement events — zone tracking, crush enemies, enter buildings, tunnel logic, bridge height |
| 101 | 0x194  | 0x004D8FB0 | FootClass::Receive_Radio | OVERRIDE | Radio message handling — handles docking, loading passengers, NavCom coordination |
| 104 | 0x1A0  | 0x004D9F70 | FootClass::OnSold | OVERRIDE | Handle sell action — play EVA, refund credits, destroy unit |
| 105 | 0x1A4  | 0x004DC810 | FootClass::SetPathIndex | OVERRIDE | Set current path waypoint index — stores path cell and bridge flag |
| 107 | 0x1AC  | 0x004D9C10 | FootClass::LocomotorPassabilityCheck | OVERRIDE | Check if locomotor can traverse cell — queries ILocomotion::Can_Enter_Cell |
| 108 | 0x1B0  | 0x004D9C60 | FootClass::CheckBridgeTraversal | OVERRIDE | Check bridge traversal legality — validates bridge crossing based on speed type |

### Navigation & Targeting Overrides

| Idx | Offset | Address    | Name | Type | Description |
|-----|--------|------------|------|------|-------------|
| 109 | 0x1B4  | 0x004DB810 | FootClass::SetCoordsWithCloak | OVERRIDE | Set coordinates with cloak update — wraps parent with bridge height adjustment |
| 125 | 0x1F4  | 0x004D8F40 | FootClass::Assign_Target | OVERRIDE | Assign NavCom target with mission suspend — stores target + previous NavCom |
| 126 | 0x1F8  | 0x004D8F80 | FootClass::Assign_Destination | OVERRIDE | Assign movement destination — sets NavCom destination cell |

### Mission Handler Overrides

**CORRECTED 2026-04-06:** Previous version had 8 wrong mission-to-slot mappings.
Verified from `MissionClass::Mission_Dispatch` switch at `0x005B3060`.
See FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md for full details.

| Idx | Offset | Address    | Mission Enum | Name | Type | Description |
|-----|--------|------------|--------------|------|------|-------------|
| 132 | 0x210  | 0x004D4DC0 | 1 (Attack)    | FootClass::Mission_Attack | OVERRIDE | Attack mission — approach target, chase, engage, patrol back to base between attacks |
| 133 | 0x214  | 0x004D4B20 | 8 (Capture)   | FootClass::Mission_Capture | OVERRIDE | Capture/Sabotage — engineer/spy/C4 logic, pathfind to building, also handles mission 17 (Sabotage) |
| 134 | 0x218  | 0x004D4CB0 | 9 (Eaten)     | FootClass::Mission_Eaten | OVERRIDE | Eaten/mind-control state — similar to Capture but for controlled units |
| 135 | 0x21C  | 0x004D5070 | 5 (Guard)     | FootClass::Mission_Guard | OVERRIDE | Guard (idle) — repair/dock/weed sub-states, garrison scan, AI auto-hunt. Also handles mission 6 (Sticky) |
| 136 | 0x220  | 0x004D6AA0 | 11 (AreaGuard) | FootClass::Mission_AreaGuard | OVERRIDE | Area guard — full harvester AI + patrol + garrison scan + ore collection (~900 bytes) |
| 138 | 0x228  | 0x004D5350 | 15 (Hunt)     | FootClass::Mission_Hunt | OVERRIDE | Hunt — seek-and-destroy with engineer/spy/C4 special handling |
| 139 | 0x22C  | 0x004D4200 | 2 (Move)      | FootClass::Mission_Move | OVERRIDE | Move — monitor locomotion completion, call OnArrival when stopped |
| 140 | 0x230  | 0x004DA2C0 | 4 (Retreat)   | FootClass::Mission_Retreat | OVERRIDE | Retreat — 2-state: find nearby passable cell, move there |
| 143 | 0x23C  | 0x004DA2B0 | 16 (Unload)   | (stub, returns 450) | OVERRIDE | Unload — stub at FootClass level; UnitClass overrides with real implementation |
| 144 | 0x240  | 0x004D9290 | 7 (Enter)     | FootClass::Mission_Enter | OVERRIDE | Enter transport/building — waypoint queue dequeue, locomotion swap, dock approach |
| 150 | 0x258  | 0x004DDF90 | 21 (Rescue)   | FootClass::Mission_Rescue | OVERRIDE | Rescue mission handler |
| 151 | 0x25C  | 0x004D4280 | 25 (Patrol)   | FootClass::Mission_Patrol | OVERRIDE | Patrol — 4-state machine: find target, engage, return to route, re-patrol |

### TechnoClass-Specific Overrides

| Idx | Offset | Address    | Name | Type | Description |
|-----|--------|------------|------|------|-------------|
| 162 | 0x288  | 0x004DBDA0 | FootClass::IsCloakable | OVERRIDE | Check cloak capability — adds locomotion IsMoving check (some units can only cloak when stopped) |
| 164 | 0x290  | 0x0041C050 | FootClass::ReturnFalse_290 | OVERRIDE | Stub |
| 177 | 0x2C4  | 0x004DBA50 | FootClass::CanAutoCloak_2C4 | OVERRIDE | Auto-cloak eligibility for moving units |
| 179 | 0x2CC  | 0x004D3810 | FootClass::CanReachDestination | OVERRIDE | Check if unit can path to destination — uses zone connectivity |
| 187 | 0x2EC  | 0x004DAFC0 | FootClass::GetZAdjust | OVERRIDE | Z-coordinate draw adjustment — accounts for locomotion height, ramp position |
| 188 | 0x2F0  | 0x004DB0A0 | FootClass::GetZAdjust_Alt | OVERRIDE | Z-adjust variant for specific rendering contexts |
| 200 | 0x320  | 0x004DA1D0 | FootClass::IsMovingToTarget | OVERRIDE | Check if actively moving toward attack target |
| 205 | 0x334  | 0x004DE580 | FootClass::OnTeamDisband | OVERRIDE | Called when team is disbanded — clears team pointer |
| 206 | 0x338  | 0x004DD0A0 | FootClass::Scan_For_Tiberium | OVERRIDE | Find nearest harvestable ore cell — zone-aware spiral search |
| 207 | 0x33C  | 0x004DFA70 | FootClass::FindDockingBay | OVERRIDE | Find available docking bay (refinery, repair depot, etc.) — checks accessibility |
| 208 | 0x340  | 0x004DFB70 | FootClass::FindDockingBay_Alt | OVERRIDE | Alternative docking bay search with different criteria |
| 209 | 0x344  | 0x004DFF40 | FootClass::FindNearestDock_Weighted | OVERRIDE | Find nearest dock with distance weighting |
| 210 | 0x348  | 0x004DFCB0 | FootClass::Find_Nearest_Dock | OVERRIDE | Find nearest accessible dock of required type |
| 211 | 0x34C  | 0x004DFE00 | FootClass::FindDock_Variant | OVERRIDE | Dock finding variant |
| 224 | 0x380  | 0x004DE770 | FootClass::IsInGarrison | OVERRIDE | Check if unit is garrisoned in a building |
| 229 | 0x394  | 0x004DE630 | FootClass::FreeMindControlledChain | OVERRIDE | Release mind-controlled units chain for FootClass |
| 232 | 0x3A0  | 0x004D5660 | FootClass::StopFiring | OVERRIDE | Stop firing — clears targets and stops locomotion movement |
| 238 | 0x3B8  | 0x004D98C0 | FootClass::PostFire | OVERRIDE | Post-fire processing (TechnoClass has stub returning 0) |
| 241 | 0x3C4  | 0x004D9920 | FootClass::Greatest_Threat | OVERRIDE | Threat scanning for mobile units — uses FootClass::Greatest_Threat_Scan |
| 245 | 0x3D4  | 0x004DBED0 | FootClass::ChangeOwner | OVERRIDE | Transfer ownership — handles team reassignment, NavCom clearing |
| 247 | 0x3DC  | 0x004DEBB0 | FootClass::ReceiveEMP | OVERRIDE | Handle EMP hit — stops locomotion, clears movement |
| 267 | 0x42C  | 0x004D8560 | FootClass::DrawSHP_42C | OVERRIDE | Draw SHP variant for mobile units |
| 270 | 0x438  | 0x004DC060 | FootClass::DrawActionLines | OVERRIDE | Draw target/move order lines to destination |
| 287 | 0x47C  | 0x004D94A0 | FootClass::GetNavComTarget | OVERRIDE | Get NavCom (navigation computer) target — returns first entry from path queue |
| 288 | 0x480  | 0x004D94B0 | FootClass::Set_Destination | OVERRIDE | Set movement destination — full implementation with path validation, zone check, bridge awareness |
| 289 | 0x484  | 0x004D82B0 | FootClass::OnArrival | OVERRIDE | Arrival handler — processes arrival at destination, garrison entry, dock coordination |
| 292 | 0x490  | 0x004DF510 | FootClass::PostMoveProcess | OVERRIDE | Post-movement processing — handles path completion, next waypoint |

### FootClass EMP/TarCom Overrides

| Idx | Offset | Address    | Name | Type | Description |
|-----|--------|------------|------|------|-------------|
| 297 | 0x4A4  | 0x004DF0E0 | FootClass::Assign_Target_Command | OVERRIDE | Assign target from player command — validates target, sets TarCom |
| 298 | 0x4A8  | 0x004DF1A0 | FootClass::Clear_All_TarCom | OVERRIDE | Clear all targeting data — NavCom + TarCom |
| 299 | 0x4AC  | 0x004DF1C0 | FootClass::EMP_Handler_4AC | OVERRIDE | EMP effect handler |
| 300 | 0x4B0  | 0x004DF1D0 | FootClass::EMP_Handler_4B0 | OVERRIDE | EMP effect handler |
| 301 | 0x4B4  | 0x004DF1E0 | FootClass::EMP_Handler_4B4 | OVERRIDE | EMP effect handler |
| 302 | 0x4B8  | 0x004DF1F0 | FootClass::EMP_Handler_4B8 | OVERRIDE | EMP effect handler |
| 305 | 0x4C4  | 0x004DF310 | FootClass::EMP_Handler_4C4 | OVERRIDE | EMP effect handler |
| 306 | 0x4C8  | 0x004DF320 | FootClass::EMP_Handler_4C8 | OVERRIDE | EMP effect handler |
| 307 | 0x4CC  | 0x004DF3A0 | FootClass::Arrival_Target_Handler | OVERRIDE | Handle arrival at target — check if should attack, enter, or stop |
| 308 | 0x4D0  | 0x004DF4B0 | FootClass::EMP_Handler_4D0 | OVERRIDE | EMP effect handler |

### NEW FootClass Virtual

| Idx | Offset | Address    | Name | Type | Description |
|-----|--------|------------|------|------|-------------|
| 309 | 0x4D4  | 0x004DE750 | FootClass::ProcessFidgetTrigger | NEW | Process idle fidget animation trigger — RETN 4 stub; FootClass owns this as new virtual (slot 309 beyond TechnoClass 0-308 range); Ghidra label is TechnoClass__ProcessFidgetTrigger (RTTI_LABEL_DRIFT — labeler attributed FootClass-new stub to TechnoClass; corrected 2026-05-28 via read_memory 0x007E9168 = 0x004DE750, read_memory 0x004DE750 = C2 04 00 RETN 4 — ROOT_CAUSE: RTTI_LABEL_DRIFT) |

---

## Key Non-Virtual Methods

These are important FootClass methods that are NOT in the vtable but are called from virtual methods or directly.

| Address    | Name | Description |
|------------|------|-------------|
| 0x004D31E0 | FootClass::Constructor | Full constructor — initializes all FootClass fields (0x520-0x6BF) |
| 0x004D3540 | FootClass::Constructor (load) | Minimal constructor for deserialization |
| 0x004D3810 | FootClass::CanReachDestination | Zone-based reachability check |
| 0x004D3920 | FootClass::Find_Path | Main A* pathfinding entry point |
| 0x004D5690 | FootClass::Greatest_Threat_Scan | Scan for highest-priority threat in range |
| 0x004D55F0 | FootClass::Head_To_Coord_Dispatch | Dispatch movement toward coordinate |
| 0x004D97A0 | FootClass::Evaluate_Target_Threat | Evaluate threat value of potential target |
| 0x004DA0E0 | FootClass::Enter_Destination | Enter/dock at destination object |
| 0x004DA2A0 | FootClass::Is_Mission_Harvest | Check if current mission is harvesting |
| 0x004DB1A0 | FootClass::GetCurrentSpeed | Get current movement speed |
| 0x004DB9B0 | FootClass::Check_Destination_Is_UnitRepair_Dock | Check if NavCom target is repair facility |
| 0x004DC760 | FootClass::Get_Slope_Speed_Factor | Speed multiplier based on terrain slope |
| 0x004DCE80 | FootClass::Is_Cell_Harvestable | Check if cell has harvestable ore/gems |
| 0x004DD9F0 | FootClass::Is_Cell_Weedable | Check if cell has harvestable weeds |
| 0x004DCFE0 | FootClass::Search_For_Tiberium_And_Move | Find ore and set NavCom destination |
| 0x004DDB90 | FootClass::Search_For_Tiberium_Short_And_Move | Short-range ore search variant |
| 0x004DD890 | FootClass::Scan_For_Tiberium_NoZone | Ore scan ignoring zone connectivity |
| 0x004DEBB0 | FootClass::ReceiveEMP | Handle EMP damage/disable |
| 0x004DF040 | FootClass::Find_Docking_Bay | Find available docking bay |
| 0x004DFCB0 | FootClass::Find_Nearest_Dock | Find nearest dock by type |
| 0x004DF0D0 | FootClass::Stop_Moving | Stop all movement |
| 0x004DF0E0 | FootClass::Assign_Target_Command | Player-issued target assignment |
| 0x004DF1A0 | FootClass::Clear_All_TarCom | Clear all targeting/navigation |
| 0x004DF3A0 | FootClass::Arrival_Target_Handler | Process arrival at NavCom target |
| 0x004CBBA0 | FootClass::Run_AStar | Execute A* pathfinding algorithm |
| 0x0056DC20 | FootClass::Find_Nearby_Passable_Cell | Find passable cell near target |
| 0x00520F40 | FootClass::Locomotion_AI | Locomotion subsystem per-tick update |
| 0x005F5FA0 | FootClass::Set_Height_On_Bridge | Adjust Z-height for bridge level |
| 0x0065AD30 | FootClass::GetDestination | Get current NavCom destination |
| 0x00707CB0 | FootClass::EMPPassengers | Apply EMP to all passengers |
| 0x0070D7E0 | FootClass::TryEnterTransport | Attempt to enter a transport unit |
| 0x00744640 | FootClass::Save_Convoy_State | Save convoy/formation data |
| 0x007446E0 | FootClass::Clear_Convoy_On_Delete | Clear convoy data on destruction |

---

## Override Summary by Category

### Serialization (2 overrides)
- Load (idx 5), Save (idx 6) — handles path vectors, locomotion COM object serialization

### Movement & Pathfinding (12 overrides)
- AI (idx 23), PerCellProcess (idx 99), GetDestinationCoords (idx 19)
- LocomotorPassabilityCheck (idx 107), CheckBridgeTraversal (idx 108)
- Assign_Target (idx 125), Assign_Destination (idx 126)
- Set_Destination (idx 288), GetNavComTarget (idx 287), OnArrival (idx 289)
- SetCoordsWithCloak (idx 109), ShouldBeOnBridge (idx 47)

### Mission Handlers (12 overrides)
- Mission_Attack (132), Mission_Capture (133), Mission_Eaten (134)
- Mission_Guard (135), Mission_AreaGuard (136), Mission_Hunt (138)
- Mission_Move (139), Mission_Retreat (140), Unload stub (143)
- Mission_Enter (144), Mission_Rescue (150), Mission_Patrol (151)
- (corrected 2026-05-28: summary was stale pre-CORRECTED-2026-04-06 names; brought in sync with main table — ROOT_CAUSE: STALE_SUMMARY)

### Combat & Targeting (8 overrides)
- ReceiveDamage (idx 91), ClickedAction_Cell (80), ClickedAction_Object (81)
- What_Action_OnCell (28), What_Action_OnObject (29), GetCursorForCell (27)
- StopFiring (232), PostFire (238), Greatest_Threat (241)

### Rendering (4 overrides)
- GetVisualState (26), GetZAdjust (187/188), DrawSHP_42C (267), DrawActionLines (270)

### Lifecycle (5 overrides)
- Limbo (53), Unlimbo (54), Destroy (55), UnInit (62), ChangeOwner (245)

### Cloaking (3 overrides)
- IsCloakable (162), ProcessCloakAndNotify (73), CanAutoCloak_2C4 (177)

### Docking / Dock Finding (5 overrides)
- FindDockingBay (207), FindDockingBay_Alt (208), FindNearestDock_Weighted (209)
- Find_Nearest_Dock (210), FindDock_Variant (211)

### Radio & Notification (3 overrides)
- Receive_Radio (101), OnSold (104), OnLoadNotify (69)

### EMP Handlers (7 overrides)
- ReceiveEMP (247), plus EMP_Handler slots 299-302, 305-306, 308

### Other (6 overrides)
- PointerExpired (10), ComputeChecksum (13), ScalarDeletingDestructor (8)
- IsSurfacing (38), Scan_For_Tiberium (206), OnTeamDisband (205)
- CanBeSelected (78), IronCurtain (85), IsInGarrison (224)
- IsHighFlying (21), GetFoundation (48), GetThreatLevel (30)
- PostMoveProcess (292), IsMovingToTarget (200), CanReachDestination (179)
- FreeMindControlledChain (229), Assign_Target_Command (297), Clear_All_TarCom (298)
- Arrival_Target_Handler (307), ProcessFidgetTrigger (309 - NEW)
