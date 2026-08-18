# Convoy/Formation System - Ghidra Research Report

**Date:** 2026-03-23
**Binary:** gamemd.exe
**Confidence:** High (verified from binary, cross-referenced across multiple functions)

## Overview

The convoy/formation system allows ground vehicles to move in linked chains where
a leader unit sets the pace and followers synchronize their speed and destination.
The system has two distinct subsystems:

1. **Unit-level convoy chain** (FootClass offsets 0x6C8-0x6D2) - a linked list of
   units that share speed and stop propagation via DriveLocomotionClass
2. **Team-level convoy movement** (TeamClass functions at 0x6E9050+) - AI team
   scripts that coordinate team member movement toward objectives

## FootClass Convoy Chain Layout

All offsets are relative to TechnoClass/FootClass `this` pointer:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0x578  | 4    | formation_speed | Speed value propagated from leader to followers |
| 0x5A4  | 4    | NavCom | Navigation target (destination pointer) |
| 0x5D4  | 4    | convoy_related_field | Checked before calling Clear_Convoy_Chain; if nonzero, convoy is cleared on pathfind failure |
| 0x5D8  | 4    | team_next_member | Linked list pointer to next member in TeamClass (NOT convoy chain) |
| 0x5E0  | 96   | path_queue[24] | 24-entry path direction queue |
| 0x6C4  | 4    | type_pointer | Pointer to TechnoTypeClass/UnitTypeClass |
| 0x6C8  | 4    | next_in_convoy | Pointer to next FootClass in convoy chain (linked list) |
| 0x6CC  | 4    | convoy_data | Serialized with convoy state; purpose TBD |
| 0x6D0  | 1    | is_convoy_follower | Set to 1 on units that are following a leader |
| 0x6D1  | 1    | convoy_stopped | Set to 1 in Stop_Moving check; prevents re-propagation |
| 0x6D2  | 1    | convoy_flag_2 | Additional convoy flag, serialized |
| 0x687  | 1    | convoy_flag_3 | Another convoy-related flag, serialized |
| 0x688  | 1    | convoy_disbanded | Set to 1 in Clear_Convoy_Chain for each member |
| 0x689  | 1    | convoy_arrived | Used by team movement to track arrived members |

## TechnoTypeClass Fields

| Offset | INI Key | Description |
|--------|---------|-------------|
| 0xC94  | IsTrain | Enables mutual pass-through in Can_Enter_Cell |
| 0xDBD  | Accelerates | Formation leader uses this for speed ramping logic |
| 0xE18  | IsTrain (from UnitClass vptr) | Same field accessed via UnitTypeClass pointer |

**Note on IsTrain offsets:** TechnoTypeClass+0xC94 and UnitTypeClass+0xE18 both
refer to the IsTrain field. The offset difference (0x184 bytes) represents additional
fields in the UnitTypeClass inheritance chain before TechnoTypeClass begins. Both
are confirmed via ReadINI at 0x712277 (writes to [EBP+0xC94]) and Can_Enter_Cell
at 0x73F0A0 (reads from [type+0xE18]).

## Key Functions

### TechnoClass__Clear_Convoy_Chain (0x6EC3A0)

Walks the TeamClass member list (via +0x5D8 links) and for each member:
1. Calls vtable+0x3C8 (Set_Target with NULL) to clear their attack target
2. Sets +0x688 = 1 (convoy_disbanded flag)

Called from:
- DriveLocomotionClass__Process_Movement (0x4B2EB6) - on pathfinding failure
  when techno+0x5D4 is nonzero
- ShipLocomotionClass__Process_Movement (0x6A2506)
- FUN_005164d0 (0x516861) - related movement processing

### TeamClass__Set_Convoy_Target (0x6E9050)

**Address:** 0x6E9050
**Signature:** `void __thiscall(TeamClass *this, TechnoClass *new_target)`

The central convoy coordination function. Called by nearly all team script actions.

Logic:
1. If `new_target` differs from current target (team+0x3C):
   - Walk all team members (via +0x54 first_member, iterated via member+0x5D8)
   - For each member whose nav target (member[0xAD] = techno+0x2B4) or
     destination (member[0x169] = techno+0x5A4) matches the OLD target:
     - Assign mission 5 (Guard) via vtable+0x1E8
     - Clear destination if target was destination
     - Clear nav target if target was nav target
2. Update team+0x40 (secondary target) if it matches old target
3. Set team+0x3C = new_target
4. If new_target is a building (AbstractType == 0xB):
   - Check if it's on screen; set team+0x82 accordingly
   - If off-screen, force all members to re-path via vtable+0x480

### DriveLocomotionClass__Stop_Moving (0x4AFE00)

Propagates stop to convoy chain:
1. Only propagates if:
   - Destination is valid (not null)
   - Type has IsTrain set (type+0xC94 != 0)
   - convoy_stopped flag (techno+0x6D0) is clear
2. Walks the convoy chain via techno+0x6C8
3. For each follower: calls ILocomotion::Stop_Moving via vtable+0x48 on their locomotor (+0x674)
4. Guards against infinite loops by checking `next != next->next`

### DriveLocomotionClass__Process_Drive_Track (0x4B0F20)

Formation speed propagation (lines ~180-200 of decompilation):

When the leader has `Accelerates=true` (type+0xDBD) and track step < 0x40:
1. Computes target speed based on distance to destination, deceleration curve,
   and whether the unit is braking
2. Calls vtable+0x544 (Set_Speed_Percent) with the computed speed
3. Stores the speed at techno+0x578 (formation_speed)
4. **If mission == 1 (Guard) AND convoy chain exists (techno+0x6C8 != 0):**
   Walks the chain and calls Set_Speed_Percent on each follower with the
   leader's formation_speed value

The convoy exempt deceleration check:
```
if (mission == 1 && type_of_leader->offset_0xE0C != 0) {
    skip_deceleration = true;
}
```
This prevents convoy followers from decelerating when their leader's type has
a specific flag set (likely IsTrain, needs further verification).

### DriveLocomotionClass__Process (0x4B0500)

Follow mission (0xF) destination sync:

When a unit is between drive track steps and its mission class has Follow mission:
1. Gets the mission's target coordinates via vtable+0x4C
2. If the target coords differ from current head-to destination:
   - Calls ILocomotion::Head_To_Coord (vtable+0x44) with the new coordinates
3. Then calls Process_Movement to continue pathfinding

This ensures Follow-mission units continuously update their destination to
track the moving leader.

## Convoy Chain Construction

### At Map Load (ScenarioClass__Read_Units_Section @ 0x743270)

The `[Units]` section in scenario files specifies convoy links as unit indices.
After all units are created, the loader iterates and:

```c
for each unit i:
    if (convoy_next_index[i] == -1 || convoy_next_index[i] >= total_units)
        unit[i]->next_in_convoy = NULL;  // offset 0x6C8
    else
        unit[i]->next_in_convoy = unit_array[convoy_next_index[i]];
        unit_array[convoy_next_index[i]]->is_convoy_follower = 1;  // offset 0x6D0
```

### On Owner Change (UnitClass__Transfer_Convoy_On_Owner_Change @ 0x7463A0)

When a unit changes owner (e.g., mind control):
1. Clears own is_convoy_follower flag (0x6D0 = 0)
2. If it had a next_in_convoy (0x6C8):
   - Calls Set_Destination on the next unit with the new owner
   - Preserves the chain link
   - Sets the next unit's follower flag

### On Deletion (FootClass__Clear_Convoy_On_Delete @ 0x7446E0)

When any object is deleted, checks if it matches the convoy chain pointer
(offset 0x6C8) and nulls it out if so.

### Save/Load (FootClass__Save_Convoy_State @ 0x744640)

Serializes: type pointer (0x6C4), next_in_convoy (0x6C8), convoy_data (0x6CC),
flags at 0x6D0/0x6D1/0x6D2/0x687.

## IsTrain Mutual Passthrough (UnitClass__Can_Enter_Cell @ 0x73F0A0)

When two UnitClass instances meet in Can_Enter_Cell:
```c
if (this->type->IsTrain && other->type->IsTrain && other is UnitClass) {
    return MOVE_OK;  // allow pass-through
}
```
This allows train units in a convoy to overlap cells, enabling realistic
train-car following behavior.

## Team Movement System (TeamClass)

The TeamClass convoy movement uses a completely separate system from the
unit-level convoy chain. It coordinates AI team members via script actions.

### Core Dispatch Functions

| Address | Name | Role |
|---------|------|------|
| 0x6EB490 | TeamClass__Convoy_Move_With_Target | Move team when target is known; picks best member as pathfinder |
| 0x6EBAD0 | TeamClass__Convoy_Move_Without_Target | Move team toward current target; handles stragglers |
| 0x6EBF50 | TeamClass__Convoy_Guard_Members | Guards team members, sends stragglers to catch up |

### Member Selection for Pathfinding

All team movement functions select the "best" member for pathfinding using the
same criteria:
1. Member must be alive ((char)member[0x24] != 0), have health (member[0x1B] != 0)
2. Must not be cloaked (*(char *)(member+0x81) == 0) unless in map editor
3. Must have arrived flag set (*(char *)(member+0x689) != 0) or be in Guard mission (2)
4. Highest type->ThreatPosed value (type+0x5FC) wins

The selected member pathfinds to the target, and other members follow.

### Straggler Management

Team movement functions check `RulesClass+0x171C` (CloseEnough distance) and
`RulesClass+0x1720` (Stray distance) to determine:
- If a member is too far, send it to catch up (assign mission Move, set destination)
- If close enough, set arrived flag (member+0x689 = 1)

For Follow mission (0xF) specifically, if a member in Follow mode has a type with
`type+0xEBE` set (likely a team-related flag), it gets reassigned to mission 8 (Capture)
with the team target.

### Team Target Tracking

- team+0x3C: Current primary target (set by Set_Convoy_Target)
- team+0x40: Secondary/previous target
- team+0x54: First member pointer (head of team member list)
- team+0x7F: Movement completion flag
- team+0x80: "step done" flag - set to 1 when all members arrive
- team+0x82: off-screen flag for target building

## Summary of Convoy Chain vs Team Movement

| Feature | Convoy Chain (0x6C8) | Team Movement (0x6E9050+) |
|---------|---------------------|--------------------------|
| Used by | Player-controlled units, map-placed convoys | AI team scripts |
| Link structure | Singly-linked list at FootClass+0x6C8 | TeamClass member list at +0x54, iterated via +0x5D8 |
| Speed sync | Formation_speed at +0x578, propagated in Process_Drive_Track | Implicit via team pathfinding |
| Stop propagation | Stop_Moving walks chain, calls ILocomotion::Stop_Moving | Set_Convoy_Target clears old targets |
| Passthrough | IsTrain enables cell overlap | Not applicable (team members path separately) |
| Construction | Map load (Units section), potentially player commands | AI creates teams via trigger/script |

## Functions Labeled in Ghidra

| Address | Name |
|---------|------|
| 0x6E9050 | TeamClass__Set_Convoy_Target |
| 0x6EB490 | TeamClass__Convoy_Move_With_Target |
| 0x6EBAD0 | TeamClass__Convoy_Move_Without_Target |
| 0x6EBF50 | TeamClass__Convoy_Guard_Members |
| 0x6EC3A0 | TechnoClass__Clear_Convoy_Chain |
| 0x6EC7D0 | TeamClass__Convoy_Script_Move_To_Cell |
| 0x6EC9A0 | TeamClass__Convoy_Script_Attack_Building |
| 0x6ECA70 | TeamClass__Convoy_Script_Attack_Building_v2 |
| 0x6ECCE0 | TeamClass__Convoy_Script_Move |
| 0x6ECE60 | TeamClass__Convoy_Script_Follow_Target |
| 0x6ED090 | TeamClass__Convoy_Script_Patrol |
| 0x6EE310 | TeamClass__Convoy_Script_Attack_Nearest |
| 0x6EE3F0 | TeamClass__Convoy_Script_Attack_Nearest_v2 |
| 0x6EE5C0 | TeamClass__Convoy_Script_Attack_Farthest |
| 0x6EE800 | TeamClass__Convoy_Script_Attack_Production |
| 0x6EEBD0 | TeamClass__Find_Best_Target_Building |
| 0x743270 | ScenarioClass__Read_Units_Section |
| 0x7446E0 | FootClass__Clear_Convoy_On_Delete |
| 0x744640 | FootClass__Save_Convoy_State |
| 0x7463A0 | UnitClass__Transfer_Convoy_On_Owner_Change |

## Open Questions

1. **Runtime convoy chain building:** The only confirmed place convoy chains are
   BUILT (writing to 0x6C8) is during map load from the [Units] section. Whether
   player group-move commands also build convoy chains at runtime was not confirmed.
   The player Follow command may only use NavCom destination sync (mission 0xF)
   without building actual 0x6C8 chains.

2. **Offset 0x5D4 exact purpose:** Checked before Clear_Convoy_Chain in
   Process_Movement. If nonzero, the convoy is dissolved on pathfind failure.
   Likely a "has active convoy path" indicator but needs further verification.

3. **Offset 0x6CC:** Serialized alongside convoy chain data but its runtime
   purpose is unclear.

4. **Type offset 0xE0C from UnitTypeClass:** Used in the deceleration exemption
   check during convoy speed propagation. Likely IsTrain accessed via UnitTypeClass
   pointer, but the offset arithmetic needs confirmation.
