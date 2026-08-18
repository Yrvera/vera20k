# TechnoClass Target Fields - Ghidra RE Report

## Overview

This report maps all target-related fields in TechnoClass and FootClass in gamemd.exe,
documenting how they interact with each other, what sets/reads them, and their purpose.

**All offsets verified from Ghidra decompilation of gamemd.exe.** param_1 is typed as
`int*` in these functions, so indexed offsets must be multiplied by 4 to get byte offsets.

---

## Target Fields Map

### TechnoClass Fields (present on all units, infantry, buildings, aircraft)

| Byte Offset | int* Index | Field Name | Type | Purpose | Confidence |
|-------------|-----------|------------|------|---------|------------|
| 0x274 | 0x9d | TemporalPtr | ptr | Pointer to this unit's TemporalClass (chrono weapon). Created in init when weapon has Temporal=yes. | 95% |
| 0x294 | 0xa5 | AirstrikePtr | ptr | Pointer to this unit's AirstrikeClass (airstrike designator). Checked in Set_ArchiveTarget to clear designations on retarget. | 95% |
| 0x2A8 | 0xaa | EMP/DisableTarget? | ptr/target | Checked in What_Action_OnObject alongside TechnoType+0x692. Non-zero prevents certain actions. Init: 0. | 60% |
| 0x2AC | 0xab | ActiveParticleBeamTarget? | ptr/target | Checked in GetFireError: if non-null and different from fire target, returns FIRE_BUSY. Related to persistent beam weapons. | 70% |
| 0x2B4 | 0xad | **ArchiveTarget** | target | The RESOLVED combat target. This is what the unit is actively shooting at. Set by Set_ArchiveTarget (vtable+0x3c8 = 0x006fcdb0). Printed as "TarCom" in sync dump. Used by DrawActionLines to draw attack line. | 99% |
| 0x2B8 | 0xae | ? | dword | Unknown. Init: 0. Near ArchiveTarget. | 30% |
| 0x2BC | 0xaf | CaptureManagerPtr | ptr | Pointer to this unit's CaptureManagerClass (mind control). Created when primary weapon has MindControl=yes. | 95% |
| 0x2C0 | 0xb0 | MindControlledBy | ptr | Pointer to the TechnoClass that is mind-controlling this unit. Set by CaptureManagerClass::CaptureUnit. | 95% |
| 0x2C4 | 0xb1 | ? | byte | Unknown. Init: 0. Between MindControlledBy and MindControlAnim. | 30% |
| 0x2C8 | 0xb2 | MindControlAnim | ptr | Pointer to the AnimClass showing the mind control "ring" on this unit. Created by CaptureManagerClass::CaptureUnit. | 90% |
| 0x2CC | 0xb3 | ? | dword | Unknown. Init: 0. | 30% |
| 0x2D0 | 0xb4 | SpawnManagerPtr | ptr | Pointer to this unit's SpawnManagerClass (aircraft carrier spawn logic). When ArchiveTarget clears, SpawnManager->SetTarget(0) is called. | 95% |
| 0x2D8 | 0xb6 | SlaveManagerPtr | ptr | Pointer to this unit's SlaveManagerClass (slave miner logic). Created in init function. | 95% |
| 0x2FC | 0xbf | IFVMode | int | IFV turret mode index. Init: -1 (0xFFFFFFFF). Cleared to 0 in Set_ArchiveTarget for infantry when certain conditions met. | 85% |
| 0x304 | 0xc1 | WeaponParticleSystem1 | ptr | Pointer to active ParticleSystemClass created during Fire_At when weapon has IsLaser=yes (offset 0x129). Destroyed when ArchiveTarget changes and new target can't be attacked with same weapon. | 90% |
| 0x308 | 0xc2 | WeaponParticleSystem2 | ptr | Another active particle beam pointer (weapon flag 0x12a). | 85% |
| 0x314 | 0xc5 | WeaponParticleSystem3 | ptr | Another active particle beam pointer (weapon flag 0x12d). | 85% |
| 0x3B8 | 0xee | CurrentBurstIndex | int | Number of shots fired in current burst. Incremented in Fire_At. Modulo'd by weapon Burst count. Reset to 0 when ArchiveTarget is cleared. | 95% |
| 0x50C | 0x143 | IsNewTarget | byte | Flag set to 1 when passive target scanning finds a target DIFFERENT from current ArchiveTarget. Cleared to 0 at start of Set_ArchiveTarget. In AI_Update, if set and unit is in idle mission, triggers Set_ArchiveTarget(0) to force re-evaluation. | 90% |

### FootClass Fields (present on units, infantry, aircraft - NOT buildings)

| Byte Offset | int* Index | Field Name | Type | Purpose | Confidence |
|-------------|-----------|------------|------|---------|------------|
| 0x5A0 | 0x168 | PathStepsRemaining? | int | Init: 0. Referenced near NavCom. | 50% |
| 0x5A4 | 0x169 | **NavCom** | target | The current navigation destination (move target). Printed as "NavCom" in sync dump. Used by DrawActionLines to draw move line (green) when no ArchiveTarget. | 99% |
| 0x5A8 | 0x16a | **SuspendedNavCom** | target | Saved NavCom before overwriting. FUN_004d8f40 saves NavCom->SuspendedNavCom before setting new nav. Referenced in FUN_004d8f80 (vtable+0x1f8). | 90% |
| 0x5C4 | 0x171 | QueuedAttackMission | int | The mission ID queued with the attack command. Init: -1 (0xFFFFFFFF). Set to 0x1D (Attack mission) in the command handler. Cleared by FUN_004df1a0. | 85% |
| 0x5C8 | 0x172 | **SuspendedTarCom** | target | Suspended combat target. Set by command handler when `param_2+0x17` flag is set (force-fire / queued attack). In arrival handler: if non-zero, unit finishes moving first, then processes target. | 95% |
| 0x5CC | 0x173 | **TarCom** | target | The commanded combat target (Target Computer). Set by command handler from player input or AI orders. In arrival handler: if SuspendedTarCom==0, TarCom is checked and copied to ArchiveTarget. | 95% |
| 0x5D1 | (byte) | IsReturningFire | byte | Flag set to 1 in arrival handler when unit begins pursuing a target. Checked to determine if unit should keep chasing ArchiveTarget after arrival. Cleared by FUN_004df1a0. | 80% |

---

## Key Functions

### Set_ArchiveTarget (vtable+0x3c8 = 0x006fcdb0)
The primary function for assigning/clearing the active combat target.

**Logic flow:**
1. Clear IsNewTarget flag (0x50C = 0)
2. If new target == current ArchiveTarget, return (no change)
3. If unit has AirstrikeClass and is the airstrike owner, clear the airstrike designation
4. If unit is Infantry (RTTI==2), handle IFV mode changes
5. Resolve the target: if target is an object on a cell, may redirect to the cell's occupant, handle passengers, temporal immunity, etc.
6. If target points to self, resolve to own cell
7. If target is null and on a warp-linked building, redirect to null
8. Store resolved target into ArchiveTarget (0x2B4)
9. If SpawnManager exists (0x2D0) and new target is null, call SpawnManager->SetTarget(0)
10. If ArchiveTarget is now null, reset CurrentBurstIndex (0x3B8) to 0
11. If WeaponParticleSystem (0x304) exists and new target fails range check, destroy the particle system

### Arrival Handler (FUN_004DF3A0 = 0x004DF3A0)
Called when a unit reaches its movement destination. Transfers TarCom/SuspendedTarCom to ArchiveTarget.

**Logic flow:**
```
if SuspendedTarCom (0x5C8) == 0:
    if TarCom (0x5CC) != 0:
        if Can_Attack(TarCom): return  // will attack TarCom later
    if IsReturningFire (0x5D1):
        if Can_Attack(ArchiveTarget): return  // keep chasing
    ArchiveTarget = 0  // clear target
    coords = this->Location
    if Scan_For_Target(coords, 1) == false:
        ArchiveTarget = TarCom  // copy TarCom to ArchiveTarget
        return
else:  // has SuspendedTarCom
    if IsReturningFire (0x5D1):
        if Can_Attack(ArchiveTarget): return
        ArchiveTarget = 0
    coords = this->Location
    if Scan_For_Target(coords, 1) == false:
        return  // no target found, don't override
    // Target found by scanning
Queue_Mission(ATTACK, 1)
IsReturningFire = 1
```

### Clear_All_TarCom (FUN_004df1a0 = 0x004DF1A0)
Clears all targeting state for a FootClass unit.
```
QueuedAttackMission (0x5C4) = -1
SuspendedTarCom (0x5C8) = 0
TarCom (0x5CC) = 0
IsReturningFire (0x5D1) = 0
```

### Assign_Target Command Handler (FUN_004df0e0 = 0x004DF0E0)
Processes an attack command (command type 0x1D).
- If `param_2+0x17` (suspended flag): writes to SuspendedTarCom (0x5C8)
- If `param_2+0x12` (normal attack flag): writes to TarCom (0x5CC)
- Also sets QueuedAttackMission (0x5C4) = 0x1D and clears IsReturningFire

### StopAllTargeting (0x0070D4A0)
Global function iterating all TechnoClass objects. For each unit whose ArchiveTarget matches the given object:
1. Calls stop (vtable+0x1F8)
2. If infantry in specific mission, clears mission and flag
3. Calls Set_ArchiveTarget(0)
Also iterates bullets/projectiles clearing target references.

### Passive Target Acquisition (FUN_00709480)
Called periodically. Runs Scan_For_Target. If a new target is found that differs from current ArchiveTarget, sets IsNewTarget (0x50C) = 1. AI_Update then checks this flag and calls Set_ArchiveTarget(0) to force re-evaluation.

### TechnoClass::AI_Update (0x006F9E50)
Relevant target logic:
- If unit has ArchiveTarget and target is now allied, calls Set_ArchiveTarget(0)
- Periodically validates ArchiveTarget is still attackable; clears if not
- If ArchiveTarget is set and IsNewTarget flag is set, and unit is in an idle/guard mission, clears ArchiveTarget to trigger re-acquisition

---

## Target Flow Diagram

```
Player Attack Command
        |
        v
  [Assign_Target Handler]
        |
  +-----+------+
  |             |
  v             v
TarCom       SuspendedTarCom
(0x5CC)      (0x5C8)
  |             |
  |  [Unit arrives at destination]
  |             |
  v             v
  [Arrival Handler FUN_004DF3A0]
        |
        v
  ArchiveTarget (0x2B4)  <----  [Passive Scan also sets this via Set_ArchiveTarget]
        |
        +---> DrawActionLines (red attack line)
        +---> Fire_At / GetFireError (combat)
        +---> SpawnManager->SetTarget (carrier spawns)
        +---> ParticleSystem cleanup (beam weapons)
        +---> CurrentBurstIndex reset on clear
```

---

## Mind Control System

### Architecture
Mind control is entirely separate from ArchiveTarget. It uses the CaptureManagerClass system.

**Key structures:**
- `TechnoClass+0x2BC` (CaptureManagerPtr): owned by the controller unit
- `TechnoClass+0x2C0` (MindControlledBy): set on the controlled unit, pointing to controller
- `TechnoClass+0x2C8` (MindControlAnim): the visual ring anim on the controlled unit

### CaptureManagerClass Layout
- `+0x28`: Vector of CaptureNode pointers (controlled units list)
- `+0x34`: Count of controlled units
- `+0x3C`: Max capture count
- `+0x40`: byte flag (mind control damage active)
- `+0x44`: damage timer countdown
- `+0x48`: Owner TechnoClass pointer
- `+0x4C`: timer for periodic damage

### CaptureNode Layout (0x14 bytes)
- `+0x00`: Controlled unit pointer
- `+0x04`: Original owner (HouseClass) pointer
- `+0x08`: Capture frame (-1 = permanent)
- `+0x0C`: Unknown
- `+0x10`: MindControlAttackLineFrames duration (from [CombatDamage] in rules.ini)

### Mind Control Line Drawing
CaptureManagerClass::DrawLinks (0x00472160) iterates all CaptureNodes and draws lines:
1. For each controlled unit, checks if the link should be visible through `ShouldDrawLinks @ 0x00472640`
2. A link draws when the controller/host/victim selected-state gate passes, OR when `g_CurrentFrameCounter - capture_frame < MindControlAttackLineFrames`
3. The timer starts at capture time and uses the per-node capture frame plus node duration
4. After the timer expires, visibility is selected-state gated; viewport clipping happens later inside the line helper, not as the main visibility rule
5. Line color comes from the controller's house color scheme (`HouseClass+0x56F9..+0x56FB`)
6. `FUN_00704E40` draws the actual line from controller FLH to controlled unit coords plus height offset from `TechnoType+0x3DC`

### Key difference from ArchiveTarget
- ArchiveTarget is for active combat targeting (who to shoot at)
- Mind control links are persistent ownership relationships maintained by CaptureManagerClass
- ArchiveTarget is set/cleared frequently during combat; mind control links persist until freed
- The mind control "attack" itself goes through normal Fire_At -> BulletClass -> CaptureUnit path
- After capture completes, the link is maintained independently of any target field

### MindControlAttackLineFrames
- INI key in [CombatDamage] section
- Stored at RulesClass+0x310 (verified from 0x0066c921)
- Controls how long the "attack animation" line persists after mind control capture
- After this duration, lines only show when the selected-state gate in `ShouldDrawLinks` passes
- Default value loaded from rulesmd.ini

---

## DrawActionLines (0x004DC060)
This function draws the colored lines from selected units to their targets/destinations:
- If ArchiveTarget (0x2B4) is set: draws an **attack line** (red, using house color) from unit to target
- Else if NavCom (0x5A4) is set: draws a **move line** (green) from unit to destination
- Line visibility controlled by `g_ActionLines_Duration` and `g_ActionLines_StartFrame`
- Mind control lines are drawn separately by CaptureManagerClass::DrawLinks

---

## Verified Addresses
- Set_ArchiveTarget: 0x006FCDB0 (vtable+0x3C8)
- Arrival Handler: 0x004DF3A0
- Assign_Target Command: 0x004DF0E0
- Clear_All_TarCom: 0x004DF1A0
- StopAllTargeting: 0x0070D4A0
- DrawActionLines: 0x004DC060
- Passive Target Acquisition: 0x00709480
- AI_Update: 0x006F9E50
- Fire_At: 0x006FDD50
- GetFireError: 0x006FC0B0
- CaptureManagerClass::DrawLinks: 0x00472160
- CaptureManagerClass::ShouldDrawLinks: 0x00472640
- CaptureManagerClass::CaptureUnit: 0x00471D40
- CaptureManagerClass::Update: 0x00471A50
- CaptureManagerClass::FreeAll: 0x00472140
- SpawnManager::SetTarget: 0x006B7B90
