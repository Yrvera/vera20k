# ChronoSphere Superweapon & Chrono Warp System -- Ghidra Research Report

## Overview

This report covers the complete ChronoSphere superweapon handler, the ChronoWarp event
in SuperClass::Launch, the chrono miner instant teleport special case, chrono delay/lock
mechanics, cell occupation during warp, multi-unit warp handling, and all associated
sound/animation events.

All addresses are from gamemd.exe (YR 1.001). Confidence levels are noted per section.

This report complements the existing `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` which
covers the 8-phase TeleportLocomotionClass state machine in detail.

---

## 1. SuperWeapon Type Enum

The SuperWeaponTypeClass stores its type at offset +0xB4. The type name table is at
0x007E4CE0 (for trigger actions) and 0x008425C0 (for SuperWeaponTypeClass::ReadINI).

From memory inspection of the string table at 0x007E4CE0:

| Index | Name | Address of String |
|-------|------|-------------------|
| 0 | Nuke | 0x0081BE60+ |
| 1 | IronCurtain | 0x0081BE54 |
| 2 | LightningStorm | 0x0081BE44 |
| 3 | ChronoSphere | 0x0081BE34 |
| 4 | ChronoWarp | 0x0081BE28 |
| 5 | ParaDrop | 0x0081BE1C |
| 6 | PlaceWaypoint | 0x0081BE0C |
| 7 | TibSunBug | 0x0081BE00 |
| 8+ | (more types follow) | ... |

**Key point:** `ChronoSphere` (index 3) is the initial cursor/selection phase.
`ChronoWarp` (index 4) is the execution phase that actually warps units.

**Confidence: 95%** -- Verified from memory dumps and SuperWeaponTypeClass::ReadINI
(0x006CEA20) which iterates the table at 0x008425C0.

---

## 2. SuperClass::Launch -- ChronoSphere Handler (Case 3)

**Function: 0x006CC200 (SuperClass::Launch)**

The launch function uses a switch on `this->Type->SuperWeaponType` (at Type+0xB4).

### Case 3: ChronoSphere (Initial Activation)

```c
// param_1 = SuperClass*, param_2 = target CellStruct*
case 3: // ChronoSphere
    // Store the target cell for later use by ChronoWarp
    *(int*)(this + 0x62) = *param_2;  // SuperClass+0x62 = TargetCell

    // Get cell coordinates
    CellClass* cell = MapClass::GetCellAt(param_2);
    cell->vtable->GetCoords(...);

    // Create navigation command
    FUN_00437090(0);  // Calculate offset vector
    CoordStruct* adjusted = CoordStruct::VecAdd(...);

    // Start the ChronoSphere building animation
    FUN_006CB3A0(*adjusted);

    if (param_3) {  // isPlayer
        DAT_008809a0 = 4;  // cursor state = ChronoWarp select dest
        return;
    }
    // For AI: handle animation directly
    ...
```

**What this does:** When the player activates the ChronoSphere, it stores the SOURCE
cell (where units are) and sets the cursor state to 4 (select destination). The actual
warp happens in Case 4 (ChronoWarp) when the player clicks the destination.

**Confidence: 85%** -- Case 3 is relatively simple; the cursor state transition is clear.

---

## 3. SuperClass::Launch -- ChronoWarp Handler (Case 4)

**This is the most critical function for ChronoSphere gameplay.**

### Case 4: ChronoWarp (0x006CC4D0 approx.)

The ChronoWarp handler iterates through a 3x3 cell grid (9 cells) around both the
source and destination, processing each unit found.

```c
case 4: // ChronoWarp
    // Create radar events at both source and destination
    CreateRadarEvent(source_cell);
    CreateRadarEvent(dest_cell);

    // Get world coordinates of source cell center
    CellClass* srcCellObj = MapClass::GetCellAt(source_cell);
    CoordStruct srcCoord = srcCellObj->GetCenterCoord();

    // Get world coordinates of destination cell center
    CellClass* dstCellObj = MapClass::GetCellAt(dest_cell);
    CoordStruct dstCoord = dstCellObj->GetCenterCoord();

    // Handle bridge Z offsets
    if (srcCellObj->Flags & 0x100) srcCoord.Z += g_BridgeZOffset;
    if (dstCellObj->Flags & 0x100) dstCoord.Z += g_BridgeZOffset;

    // Clear any existing ChronoSphere building animation
    if (this->field_0x1A != 0) {
        *(byte*)(this->field_0x1A + 0x195) = 0;
        this->field_0x1A = 0;
        // Remove from sound system
    }

    // Spawn ChronoBlast anim at SOURCE
    AnimClass::Constructor(
        Rules->ChronoBlast,     // Rules+0x32C
        &srcCoord, 0, 1, 0x600, 0, 0
    );

    // Spawn ChronoBlastDest anim at DESTINATION
    AnimClass::Constructor(
        Rules->ChronoBlastDest, // Rules+0x328
        &dstCoord, 0, 1, 0x600, 0, 0
    );

    // === MAIN LOOP: iterate 3x3 cell grid ===
    // g_CellSpreadOffsets at 0x00B0C038 contains 9 CellStruct offsets for 3x3 grid
    CellStruct* cellOffsets = &DAT_00B0C038;  // array of 9 {dx, dy} pairs

    for (int i = 0; i < 9; i++) {
        CellStruct currentSrcCell = {
            source_cell.X + cellOffsets[i].X,
            source_cell.Y + cellOffsets[i].Y
        };

        // Get the occupant list for this cell
        CellClass* cell = MapClass::GetCellAt(currentSrcCell);
        TechnoClass* occupant;
        if (cell->Flags & 0x100) {
            occupant = cell->AltOccupant;  // bridge occupant at +0xE8
        } else {
            occupant = cell->FirstOccupant; // ground occupant at +0xE4
        }

        // Process each unit in the cell's linked list
        for (; occupant != NULL; occupant = occupant->NextInCell) {
            // --- FILTER: Skip non-warpable units ---

            // Must be on map (IsAlive flag bit 2 set)
            if (!(occupant->Flags & 0x04)) continue;

            // Must not be cloaked
            if (occupant->vtable->IsCloaked()) continue;  // vtable+0x54

            // --- SPECIAL CASE: Chrono miner returning to refinery ---
            bool skipKill = false;
            if (occupant->NavTarget != NULL) {  // FootClass+0x5A4
                int navType = occupant->NavTarget->vtable->WhatAmI();
                if (navType == 1) {  // Building
                    // Check if NavTarget is at the current cell
                    CoordStruct navCoord = occupant->GetCoord();
                    CellClass::Get_Cell_At(&navCoord);
                    BuildingClass* bldg = Look_up_building_in_cell();
                    if (bldg == occupant->NavTarget) {
                        skipKill = true;  // unit is docked -> skip kill/warp
                    }
                }
            }
            // Also check NavTarget types 2 (Vehicle) and 6 (Building) similarly

            // --- CHECK: Chronoshiftable type exclusion ---
            TypeClass* unitType = occupant->vtable->GetType();

            // Units with Chronoshiftable=no (TypeClass+0xD97) are killed instantly
            if (unitType->Chronoshiftable (+0xD97) == 0
                && unitType->field_0xCD4 == 0   // additional filter
                && !occupant->vtable->IsCloaked()) {

                // Kill non-chronoshiftable units
                // Check for: not IsInAir, not ChronoInTransit, not BeingWarped,
                // not InLimbo, and not docked
                if (!occupant->vtable->IsInAir()         // vtable+0x160
                    && occupant->ChronoInTransit == 0     // +0x27C byte -> [+0x9F*4]
                    && !occupant->vtable->IsBeingWarped() // vtable+0x1D4
                    && !occupant->vtable->IsInLimbo()     // vtable+0x1D8
                    && !skipKill) {

                    // If unit has a "limbo" building, detach it
                    if (occupant->LimboBuilding != NULL) {  // +0x2E4
                        if (occupant->LimboBuilding->vtable->WhatAmI() == 6) {
                            // Detach from building
                        }
                        FUN_00459470();
                    }

                    // === WARP THE UNIT ===

                    // 1. Save existing piggyback locomotor
                    IPiggyback* existingPiggyback = NULL;
                    if (occupant->Locomotor != 0) {
                        QueryInterface(IID_IPiggyback, &existingPiggyback);
                    }

                    // 2. Create TeleportLocomotion via COM
                    ILocomotion* newLoco = NULL;
                    CoCreateInstance(&CLSID_TeleportLocomotion, 0, 7, &newLoco);

                    // 3. Link new locomotor to the unit
                    newLoco->Link_To_Object(occupant);

                    // 4. Set up piggyback chain
                    IPiggyback* newPiggyback = NULL;
                    QueryInterface(newLoco, IID_IPiggyback, &newPiggyback);
                    if (existingPiggyback != newPiggyback) {
                        // Store the old locomotor as piggybacked
                    }
                    if (newPiggyback != NULL) {
                        newPiggyback->Begin_Piggyback(occupant->Locomotor);
                        // Replace unit's active locomotor
                        occupant->Locomotor = newLoco;
                    }

                    // 5. Calculate destination coordinates
                    // Destination = srcUnitOffset + dstCellCenter
                    CellStruct destCellCoord = {
                        dest_cell.X + cellOffsets[i].X,
                        dest_cell.Y + cellOffsets[i].Y
                    };
                    CellClass* destCellObj = MapClass::GetCellAt(destCellCoord);
                    CoordStruct destCellCenter = destCellObj->GetCenterCoord();

                    // The unit's offset from its source cell center is preserved
                    CoordStruct unitPos = occupant->GetCoord();
                    CoordStruct srcCellCenter = srcCellObj->GetCenterCoord();
                    CoordStruct offset = unitPos - srcCellCenter;
                    CoordStruct finalDest = destCellCenter + offset;

                    // Handle bridge Z at destination
                    if (destCellObj->Flags & 0x100) {
                        finalDest.Z += g_BridgeZOffset;
                    }

                    // 6. Set chrono destination on TechnoClass
                    occupant->ChronoInTransit (+0x27C) = 1;  // SET via [+0x9F] = 1
                    occupant->ChronoDestCoord_X (+0x288) = finalDest.X;
                    occupant->ChronoDestCoord_Y (+0x28C) = finalDest.Y;
                    occupant->ChronoDestCoord_Z (+0x290) = finalDest.Z;

                    // 7. Set PendingWarpPhase = 3 (jump to Phase 3 in state machine)
                    occupant->vtable->SetMission(3);  // vtable+0x280  PendingWarpPhase

                    // 8. Store source building/house for credit
                    occupant->ChronoSourceHouse (+0x42C) = this->OwnerHouse; // SuperClass+0xB
                    occupant->field_6B6 = 1;  // mark as chronoshifted

                    // 9. Set occupation at destination cell
                    CellStruct destCombined = destCellCoord + psVar4_offset;
                    CellClass* finalCell = MapClass::GetCellAt(destCombined);
                    occupant->vtable->SetOccupation(finalCell, 1);

                } // end warpable check
            } else {
                // Non-chronoshiftable: KILL the unit
                int maxHP = unitType->Strength;
                occupant->vtable->ReceiveDamage(
                    &maxHP, 0, Rules->C4Warhead, 0, 1, 0, 0
                );
            }
        } // end occupant loop
    } // end cell grid loop

    // Play EVA announcement
    if (DAT_00a8b538 == 0) {
        VoxClass::PlayEVA("EVA_ChronosphereActivated");
    }

    // Clean up cursor state
    if (param_3) {
        DAT_008809a0 = -1;
        FUN_00753250();  // Reset cursor
        VoxClass::RemoveFromQueues();
    }
```

### Key observations:

1. **3x3 cell grid**: The ChronoSphere processes ALL 9 cells in a 3x3 area, not just the center.
2. **Per-unit processing**: Each unit in each cell is individually checked and warped.
3. **Position offset preservation**: Units keep their offset relative to cell center during warp.
4. **Chronoshiftable filter**: Units with `Chronoshiftable=no` in their TypeClass are killed (C4 warhead damage equal to full HP).
5. **PendingWarpPhase = 3**: Units are set to enter the state machine at Phase 3 (IN_TRANSIT_CONTINUE), skipping the self-warp delay phases 0-2.
6. **No explicit unit count limit**: The loop processes ALL units in the 3x3 area without a cap.

**Confidence: 85%** -- The core loop and filtering logic is verified. Some variable
names in the offset calculation are approximated due to complex stack manipulation.

---

## 4. ChronoSphere::WarpUnitsAtCell (0x0065EC30)

**Called from: TriggerAction handler (0x006DD8B0, case 7 -> FUN_0065D8E0)**

This is a SEPARATE warp function used by map triggers (not the player ChronoSphere).
It has slightly different behavior:

```c
// param_1 = some context struct
// param_2 = dest waypoint index

bool WarpUnitsAtCell(int context, int destWaypoint) {
    // Early exit if context or its arrays are null
    if (context == 0 || *(context + 0xE4)->count == 0) return false;

    // Get unit linked list from context
    // For each unit in the list:
    //   - Save piggyback locomotor
    //   - Create TeleportLocomotion via CoCreateInstance
    //   - Link to unit, set up piggyback chain
    //   - Calculate destination from waypoint
    //   - Set chrono fields:
    //       techno->BeingWarped (+0x271) = 1
    //       techno->ChronoLockDuration (+0x284) = Rules->ChronoReinfDelay (+0xBF0)
    //       techno->ChronoDestCoord = calculated dest
    //   - Set PendingWarpPhase (+0x280) = 3
    //   - Call techno->Unmark(0) then techno->vtable(0x1EC)

    // After loop: play radar event if allied
    return true;
}
```

**Key difference from Case 4:** This uses `Rules->ChronoReinfDelay (+0xBF0)` as the lock
duration, not `Rules->ChronoDelay (+0xBEC)`. This is for chrono reinforcement (scripted
warps in campaigns).

**Confidence: 75%** -- Complex function with heavy register/stack manipulation. Core
field writes are verified but control flow has some decompilation artifacts.

---

## 5. Chrono Miner Instant Teleport Special Case

### The Check (in StateMachineTick Phase 0, at 0x719573-0x71958A)

```c
// After calculating the chrono delay timer:
int abstractType = techno->vtable->WhatAmI();  // vtable+0x2C
if (abstractType == 1  /* UnitClass */
    && *(char*)(techno->Type (+0x6C4) + 0xE0E) != 0) {
    // TypeClass+0xE0E = Harvester flag (Harvester=yes in rules.ini)

    // INSTANT TELEPORT: zero the timer
    timer.StartFrame = g_CurrentFrameCounter;
    timer.Duration = 0;

    // Clear BeingWarped
    techno->BeingWarped (+0x271) = 0;
}
```

### What this means:

When a unit with `Harvester=yes` (like the Chrono Miner) initiates a self-teleport:
1. The delay timer is set to 0 (instant)
2. BeingWarped is cleared immediately
3. The unit teleports instantly with no warp-out visual delay

This is why the Chrono Miner appears to "blink" instead of doing the full warp-out/
fade-in sequence that the Chrono Legionnaire does.

### How the Chrono Miner Gets TeleportLocomotion

The chrono miner has `Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}` in rulesmd.ini.
This CLSID is resolved via `COM::CoCreateInstance_Locomotor` (0x0041C250) to create
a TeleportLocomotionClass instance.

**CLSID_TeleportLocomotion** at address **0x007E9A90**:
```
{4A582747-9839-11d1-B709-00A024DDAFD1}
```
Hex bytes: `47 27 58 4A 39 98 D1 11 B7 09 00 A0 24 DD AF D1`

The `Locomotor=` INI key is read in `TechnoTypeClass::ReadINI` (0x00710490) at the
string reference at 0x0084444C ("Locomotor"). The CLSID string is parsed and stored
in the TypeClass for later COM instantiation.

### ChronoHarvTooFarDistance

**INI Key:** `ChronoHarvTooFarDistance` in `[General]`
**Rules Offset:** +0xD7C (default: 50)
**Read at:** RulesClass::ReadGeneral, address 0x00670003

```c
// In RulesClass::ReadGeneral:
Rules->ChronoHarvTooFarDistance = CCINIClass::ReadInt(
    "General", "ChronoHarvTooFarDistance", Rules->ChronoHarvTooFarDistance
);
// Stored at Rules+0xD7C
```

This value is used in `TechnoClass::Set_Destination` (0x00741970) to determine when a
chrono miner should teleport vs drive. If the distance to the destination (in cells)
exceeds `ChronoHarvTooFarDistance`, the miner uses its TeleportLocomotion (teleports).
If the distance is within range, it piggybacks a DriveLocomotion and drives normally.

The related `HarvesterTooFarDistance` at Rules+0xD78 (default: 5) controls when a
NORMAL harvester gives up looking for ore and returns to the refinery.

**Confidence: 90%** -- Field offsets verified from RulesClass::ReadGeneral decompilation.
The usage in Set_Destination is documented in the companion report.

---

## 6. ChronoDelay and Lock Mechanics

### INI Keys and Rules Offsets

| Rules Offset | INI Key | Type | Default | Section | Purpose |
|-------------|---------|------|---------|---------|---------|
| +0xBEC | ChronoDelay | int | 60 | [General] | Frames of post-warp "chrono lock" |
| +0xBF0 | ChronoReinfDelay | int | 200 | [General] | Lock duration for scripted/reinforcement warps |
| +0xBF4 | ChronoDistanceFactor | int | 32 | [General] | delay = distance / this value |
| +0xBF8 | ChronoTrigger | bool | yes | [General] | Enable distance-based delay calculation |
| +0xBFC | ChronoMinimumDelay | int | 0 | [General] | Minimum delay floor (frames) |
| +0xC00 | ChronoRangeMinimum | int | 0 | [General] | If distance < this, force minimum delay |
| +0xD7C | ChronoHarvTooFarDistance | int | 50 | [General] | Cell distance threshold for chrono miner teleport |

All parsed at `RulesClass::ReadGeneral` (0x0066D750), addresses 0x0066FAD5-0x0066FB9C.

### TechnoClass Chrono Fields

| Offset | Size | Field | Set By | Purpose |
|--------|------|-------|--------|---------|
| +0x270 | 1 | WarpingOut | Phase 0 (ChronoInTransit) | Unit is in warp-out phase |
| +0x271 | 1 | BeingWarped | Multiple phases | Master warp visual flag |
| +0x27C | 1 | ChronoInTransit | ChronoSphere handler | Set by ChronoSphere case 4 |
| +0x280 | 4 | PendingWarpPhase | ChronoSphere / state machine | Phase to enter (3 = from ChronoSphere) |
| +0x284 | 4 | ChronoLockDuration | Phase 3 / WarpUnitsAtCell | Duration of post-warp lock |
| +0x288 | 4 | ChronoDestCoord.X | ChronoSphere handler | Destination X coordinate |
| +0x28C | 4 | ChronoDestCoord.Y | ChronoSphere handler | Destination Y coordinate |
| +0x290 | 4 | ChronoDestCoord.Z | ChronoSphere handler | Destination Z coordinate |
| +0x428 | 4 | ChronoSourceBuilding | Phase 5 / DeployUnit | Ptr to ChronoSphere building |
| +0x42C | 4 | ChronoSourceHouse | Phase 5 / DeployUnit | Ptr to source house |

### How Chrono Lock Works

1. **Phase 3** of the state machine writes `Rules->ChronoDelay` into `techno->ChronoLockDuration (+0x284)`:
   ```c
   techno->ChronoLockDuration = Rules->ChronoDelay;  // Rules+0xBEC
   ```

2. **Phase 5** reads that value and sets the locomotor timer:
   ```c
   int lockDuration = techno->ChronoLockDuration;  // +0x284
   timer.StartFrame = g_CurrentFrameCounter;
   timer.Duration = lockDuration;
   ```

3. **Phase 6** waits for this timer to expire by calling `TimerCheck` (0x719BF0) every tick.

4. **While locked** (phases 5-6), the unit has `BeingWarped = 1`, which:
   - Prevents the unit from accepting movement orders (checked in movement command handlers)
   - Causes the rendering system to draw the unit with a "chrono shimmer" visual effect
   - The shimmer intensity fades linearly as the timer counts down

5. **Phase 7** clears `BeingWarped = 0` and resets all warp state. The unit can move again.

### What Prevents Movement During Lock

The `BeingWarped (+0x271)` flag is checked in the state machine pre-phase logic:
```c
// In StateMachineTick, before any phase processing:
if (techno->BeingWarped && phase == 0 && techno->PendingWarpPhase == 0) {
    // Just run the timer check -- don't process any movement
    this->TimerCheck(this);
    return;
}
```

Additionally, the TeleportLocomotionClass::Process function (0x718B70) will not initiate
new warps while the state machine is in phases 5-7.

**Confidence: 90%** -- All offsets cross-referenced between RulesClass::ReadGeneral
and the state machine code.

---

## 7. Visual Effects During Warp

### Animation Types (from RulesClass::ReadGeneral, 0x0066E160-0x0066E230)

| Rules Offset | INI Key | Used For |
|-------------|---------|----------|
| +0x328 | ChronoBlast | Spawned at SOURCE cell during ChronoSphere warp (case 4) |
| +0x32C | ChronoBlastDest | Spawned at DESTINATION cell during ChronoSphere warp |
| +0x330 | ChronoPlacement | Chrono building placement animation |
| +0x334 | ChronoBeam | Chrono beam effect |
| +0x338 | WarpIn | Warp-in effect (spawned at destination on arrival) |
| +0x33C | WarpOut / WarpAway | Warp-out shimmer (spawned at source on departure) |
| +0x340 | WarpAway | Additional warp-away effect |
| +0x344 | ChronoSparkle1 | Sparkle during chrono lock period |

### Animation Spawning Points in the State Machine

| Phase | Animation | Location | Code Address |
|-------|-----------|----------|--------------|
| Phase 0 (start) | Rules->WarpAway (+0x33C) | Unit's current position | 0x71943B |
| Phase 0 (end) | Rules->WarpAway (+0x33C) | Unit's new position (dest) | 0x7196D6 |
| Phase 2 | Rules->WarpAway (+0x33C) | Unit's position (before transit) | 0x7198A0 |
| Phase 5 | Rules->WarpAway (+0x33C) | Unit's position (after arrival) | 0x719A4E |
| ChronoSphere case 4 | Rules->ChronoBlast (+0x32C) | Source cell center | 0x6CC5E8 |
| ChronoSphere case 4 | Rules->ChronoBlastDest (+0x328) | Dest cell center | 0x6CC640 |

### Sparkle During Lock

The `ChronoSparkle1` animation (Rules+0x344) is spawned per-unit during chrono lock:

In Phase 0, after playing the warp-out sound:
```c
TypeClass* type = techno->vtable->GetType();
if (type->ChronoOutSound (+0x578) != -1 || Rules->ChronoOutSound (+0x21C) != -1) {
    AnimClass::SpawnAtCoord(...);  // sparkle anim at unit location
}
```

Similarly in Phase 5, the ChronoInSound triggers sparkle at the arrival position.

The sparkle is NOT a persistent looping animation tied to the lock timer. Instead,
it is a one-shot animation spawned at the moment of warp-out and warp-in. The visual
"shimmer" during the lock period is handled by the renderer checking `BeingWarped (+0x271)`.

**Confidence: 85%** -- Animation spawning points verified. The sparkle vs shimmer
distinction is inferred from the AnimClass::Constructor calls using Rules+0x344
(ChronoSparkle1) in the sound-adjacent code.

---

## 8. Sound Events

### INI Keys (from RulesClass::ReadAudioVisual, 0x00664C10)

| Rules Offset | INI Key | Type | Purpose |
|-------------|---------|------|---------|
| +0x1D0 (param*4=0x74) | DefaultChronoSound | VocClass index | Default chrono sound if per-type not set |
| +0x218 (param*4=0x86) | ChronoInSound | VocClass index | Sound when unit arrives at destination |
| +0x21C (param*4=0x87) | ChronoOutSound | VocClass index | Sound when unit departs from source |

These are read at:
- `DefaultChronoSound` at RulesClass::ReadAudioVisual, around the `param_1[0x74]` write
- `ChronoInSound` at param_1[0x86] (Rules+0x218)
- `ChronoOutSound` at param_1[0x87] (Rules+0x21C)

### Per-Type Sounds (TypeClass fields)

| TypeClass Offset | INI Key | Purpose |
|-----------------|---------|---------|
| +0x574 | ChronoInSound | Per-unit-type arrival sound (overrides Rules default) |
| +0x578 | ChronoOutSound | Per-unit-type departure sound (overrides Rules default) |

### SuperWeaponTypeClass Sounds

| SWType Offset | INI Key | Purpose |
|--------------|---------|---------|
| +0xC0 | SpecialSound | Sound when superweapon activates |
| +0xC4 | StartSound | Sound when superweapon starts charging |

### Sound Play Logic (in StateMachineTick)

**Warp-Out (Phase 0):**
```c
// At address 0x7195E0:
TypeClass* type = techno->vtable->GetType();
if (type->ChronoOutSound (+0x578) != -1 || Rules->ChronoOutSound (+0x21C) != -1) {
    FUN_007509E0(sound_index, 0, &techno->Location);
}
```

**Warp-In (Phase 5):**
```c
// At address 0x719A30:
TypeClass* type = techno->vtable->GetType();
if (type->ChronoInSound (+0x574) != -1 || Rules->ChronoInSound (+0x218) != -1) {
    AnimClass::SpawnAtCoord(0);  // also spawns sparkle
}
```

### LetsDoTheTimeWarp Sounds

These are the THEME/EVA sounds, not per-unit sounds:

| Address | String | Used In |
|---------|--------|---------|
| 0x0083A684 | LetsDoTheTimeWarpInAgain | RulesClass::ReadAudioVisual (Rules+0x?) |
| 0x0083A6A0 | LetsDoTheTimeWarpOutAgain | RulesClass::ReadAudioVisual (Rules+0x?) |

Both are read at `RulesClass::ReadAudioVisual` and are VocClass entries (sound effect names,
not theme music). They are referenced at addresses 0x0066A325 and 0x0066A2E3 respectively.
These likely serve as the global chrono warp sound effects.

**Confidence: 90%** -- Sound field offsets verified from ReadAudioVisual decompilation.

---

## 9. Mark_All_Occupation_Bits (0x007192C0)

```c
void TeleportLocomotionClass::Mark_All_Occupation_Bits(param_1, param_2) {
    RateTimer::Set(&param_2);
    return;
}
```

This function is **trivial** -- it just sets a rate timer. It does NOT actually mark
occupation bits in the cell grid. The name (from RTTI or auto-analysis) is misleading.

Actual cell occupation during chrono warp is handled by:
1. `techno->vtable->Unmark(0)` (vtable+0x124) to remove from source cell
2. `techno->vtable->Mark(1)` (vtable+0x124 with param=1) to place at destination
3. `techno->vtable->SetOccupation(cell, flag)` (vtable+0x480) for warp-specific occupation

### Cell Occupation During Warp

When a unit is being warped:
- **Phase 0**: Unmarked from source, marked at destination (for self-teleport)
- **Phase 2**: Teleported to ChronoDestCoord, Update_Position handles the move
- **Phase 4**: Final placement with full occupation (applyOccupancy=1)
- **Phase 5**: SetOccupation(0, 1) called after validation

If the destination cell is **occupied by another unit**, the `Update_Position` function
(0x718260) handles collision:
```c
// In Update_Position when applyOccupancy == false:
for (each object in destCell) {
    if (object->vtable->IsInAir()) {
        // Apply C4 damage (Rules->C4Warhead) to crush the unit
        TypeClass* type = techno->vtable->GetType();
        int maxHP = type->Strength;
        object->vtable->ReceiveDamage(&maxHP, 0, Rules->C4Warhead, 0, 1, 0);
    }
    else if (object->Flags & FLAG_VEHICLE) {
        // Vehicle at dest: apply C4 damage
        int maxHP = type->Strength;
        object->vtable->ReceiveDamage(&maxHP, 0, Rules->C4Warhead, 0, 1, 0);
    }
    // Infantry: damage or push aside
}
```

Units warped onto occupied cells **kill the occupants** with C4 warhead damage.

**Confidence: 80%** -- The Update_Position collision handling is complex and has some
decompilation ambiguity in the object type checks.

---

## 10. PostWarpValidation (0x007187A0)

Called in Phase 5 when `PendingWarpPhase == 0` (self-teleport only, NOT ChronoSphere).

```c
void PostWarpValidation(TeleportLoco* this, int destX, int destY) {
    CellClass* destCell = CellClass::Get_Cell_At(destX, destY);

    // 1. Check for crushable objects at destination
    for (each object at destCell) {
        if (object->vtable->IsInAir()) {
            // Crush: apply max damage with C4 warhead
            int maxHP = type->Strength;
            object->vtable->ReceiveDamage(&maxHP, 0, Rules->C4Warhead, 0, 1, 0);
        }
    }

    // 2. Check if unit can exist on water
    bool isAmphibious = false;
    TypeClass* type = techno->vtable->GetType();
    if (type->SpeedType (+0x67C) == 3) {  // SpeedType::Float
        isAmphibious = true;
        // Check powered hover: must have power surplus
        if (type->PoweredFloat (+0x410)) {
            if (!HouseClass::HasPowerSurplus(techno->OwnerHouse)) {
                isAmphibious = false;
            }
        }
    }

    // 3. Check land type at destination
    CellStruct destCellCoord = { destX >> 8, destY >> 8 };
    CellClass* cell = MapClass::GetCellAt(destCellCoord);
    if (cell->LandType (+0xEC) == 2  /* WATER */ && !isAmphibious) {
        // Check WaterBound type
        if (type->WaterBound (+0xCCE) == 0) {
            int abstractType = techno->vtable->WhatAmI();
            if (abstractType != 0xF) {  // not infantry
                // Check bridge
                if (!(cell->Flags & 0x100)) {
                    CellClass* cell2 = CellClass::Get_Cell_At(destX, destY);
                    if (cell2->LandType != 1) {  // not CLEAR
                        // UNIT DIES: falls into water
                        techno->IsFalling (+0x3CD) = 1;
                        techno->vtable->Die();   // vtable+0x3A0

                        // Credit kill to ChronoSphere owner
                        if (techno->ChronoSourceBuilding (+0x428) != 0) {
                            FUN_006B0AE0(
                                techno->ChronoSourceBuilding,
                                techno->ChronoSourceHouse
                            );
                            // Detach anim
                            if (techno->WarpAnim (+0x2D8) != NULL) {
                                techno->WarpAnim->vtable->Remove(1);
                                techno->WarpAnim = NULL;
                            }
                        }
                        // Call death handler
                        return;
                    }
                }
            }
        }
    }

    // 4. General passability check
    CellClass* passCell = MapClass::GetCellAt(destCellCoord);
    int canEnter = techno->vtable->CanEnterCell(passCell, -1, -1, 0, 1);
    if (canEnter == 7 /* BLOCKED */ || ...) {
        // Check bridge overlay
        if (CellClass::HasBridgeOverlay() && abstractType != 0xF) {
            // Falls off bridge
            techno->IsFalling = 1;
            techno->vtable->Die();
            FUN_006B0AE0(techno->ChronoSourceBuilding, techno->ChronoSourceHouse);
            ...
        } else {
            // Apply damage (not instant kill)
            int maxHP = type->Strength;
            techno->vtable->ReceiveDamage(&maxHP, 0, Rules->C4Warhead, 0, 1, 0, 0);
        }
    }
}
```

### Key takeaways:
- Units warped onto **water** (without being amphibious) **die** and credit goes to ChronoSphere owner
- Units warped onto **impassable terrain** take **full C4 damage**
- The `ChronoSourceBuilding (+0x428)` and `ChronoSourceHouse (+0x42C)` fields are used for **kill credit attribution**
- Infantry (type 0xF) are exempt from the water death check

**Confidence: 85%** -- Core logic verified. Some edge cases in the bridge/overlay
checks may have decompilation artifacts.

---

## 11. Multiple Unit ChronoSphere Warp

### How Multiple Units Are Handled

The ChronoWarp handler (SuperClass::Launch case 4) uses a **3x3 cell grid loop**:

```c
// Cell spread offsets at 0x00B0C038
// 9 entries of CellStruct (2 shorts each):
// {0,0}, {-1,0}, {1,0}, {0,-1}, {0,1}, {-1,-1}, {1,-1}, {-1,1}, {1,1}
CellStruct* offsets = &DAT_00B0C038;

for (int i = 0; i < 9; i++) {
    CellStruct srcCell = source + offsets[i];
    CellStruct dstCell = dest + offsets[i];

    // Get all occupants in srcCell
    for (TechnoClass* unit = cell->FirstOccupant; unit; unit = unit->NextInCell) {
        // Process each unit...
    }
}
```

### Limits

- **Area**: Fixed 3x3 cells (9 cells total). This cannot be changed without code modification.
- **Unit count**: No per-cell or total unit limit. ALL units in all 9 cells are processed.
- **Multiple units per cell**: Infantry can stack in cells (subcells). All are processed.

### Destination Mapping

Each source cell maps to the **corresponding** destination cell:
```
Source grid:          Destination grid:
[-1,-1] [0,-1] [1,-1]    [-1,-1] [0,-1] [1,-1]
[-1, 0] [0, 0] [1, 0] -> [-1, 0] [0, 0] [1, 0]
[-1, 1] [0, 1] [1, 1]    [-1, 1] [0, 1] [1, 1]
```

Within each cell, the unit's **sub-cell offset** (distance from cell center) is
preserved at the destination. This means infantry in different subcells maintain
their relative positions.

**Confidence: 90%** -- The 3x3 grid iteration and offset table are clearly visible
in the decompilation.

---

## 12. BuildingClass::DeployUnit_ChronoWarp (0x0070FEE0)

This function handles the special case where a **deployed** building (like an MCV)
is ChronoSphere'd. It handles the undeploy-and-warp sequence:

```c
void BuildingClass::DeployUnit_ChronoWarp(BuildingClass* this, bool param_2) {
    if (this->DeployedUnit (+0x2AC) != 0) {
        // Clear deploy state
        *(this->DeployedUnit + 0x2B0) = 0;

        TechnoClass* unit = this->DeployedUnit;
        if (unit != NULL && (unit->Flags & 0x04)) {
            int iVar = unit->vtable->SomeCheck();  // vtable+0x1C8
            if (iVar > 0) {
                // Set chrono flags on the deployed unit
                unit->field_425 = 1;
                unit->field_427 = 1;
                unit->ChronoSourceBuilding (+0x428) = this;
                unit->ChronoSourceHouse (+0x42C) = this->OwnerHouse;

                // Start warp sequence
                unit->vtable->Die();          // vtable+0x3A0
                unit->vtable->SetOccupation(0, 1);

                this->DeployedUnit = 0;
                if (param_2) {
                    this->vtable->SellBack(0);  // vtable+0x3C8
                }
                return;
            }
            // Failed deploy: mark unit as limboed
            unit->field_6AE = 1;
            unit->vtable->SetOccupation(0, 1);
        }
        this->DeployedUnit = 0;
        if (param_2) {
            this->vtable->SellBack(0);
        }
    }
}
```

**Confidence: 75%** -- Some field offsets in this function are uncertain due to
the complex index-vs-byte-offset ambiguity (param_1 is `int*` type).

---

## 13. TechnoClass::ClearChronoFields (0x00720440)

Resets all chrono-related state on a TechnoClass:

```c
void TechnoClass::ClearChronoFields(TechnoClass* this) {
    this->PendingWarpPhase (+0x280) = 0;
    this->ChronoLockDuration (+0x284) = 0;
    this->field_0x288 = 1;       // ChronoDestCoord.X = 1 (not NullCoord)
    this->field_0x289 = 0;
    this->field_0x28A = 0;
    this->field_0x28C = -1;      // ChronoDestCoord.Y = -1
    // Plus clears at +0x0, +0x100, +0x200 (relative to some sub-object)
}
```

**Note:** No callers were found via xref -- this may be dead code or called via an
unresolved vtable entry.

**Confidence: 95%** -- Simple function with clear field writes.

---

## 14. Summary of All Function Addresses

| Address | Function | Purpose |
|---------|----------|---------|
| 0x006CC200 | SuperClass::Launch | Superweapon dispatch (case 3=ChronoSphere, case 4=ChronoWarp) |
| 0x0065EC30 | ChronoSphere::WarpUnitsAtCell | Trigger-based chrono warp (reinforcements) |
| 0x0070FEE0 | BuildingClass::DeployUnit_ChronoWarp | MCV/deployed building warp handler |
| 0x00720440 | TechnoClass::ClearChronoFields | Reset all chrono state fields |
| 0x007192F0 | TeleportLocomotionClass::StateMachineTick | 8-phase warp state machine |
| 0x00719400 | TeleportLocomotionClass::InitiateWarp | Warp initiation (phase 0 body) |
| 0x007192C0 | TeleportLocomotionClass::Mark_All_Occupation_Bits | Trivial timer set (misleading name) |
| 0x007187A0 | TeleportLocomotionClass::PostWarpValidation | Destination validity check (water/terrain death) |
| 0x00718B70 | TeleportLocomotionClass::Process | Movement request processing |
| 0x00718260 | TeleportLocomotionClass::Update_Position | Teleport unit to new coordinates |
| 0x00719BF0 | TeleportLocomotionClass::TimerCheck | CDTimer expiry check, advances phase |
| 0x00719790 | TeleportLocomotionClass::ClearPendingWarpPhase | Clears PendingWarpPhase to 0 |
| 0x007197D0 | TeleportLocomotion::Phase0_SetWarpingOut | Sets WarpingOut+timer for ChronoInTransit |
| 0x00718000 | TeleportLocomotionClass::Constructor | Initializes all fields |
| 0x00719E90 | TeleportLocomotionClass::Begin_Piggyback | IPiggyback: stores old locomotor |
| 0x00719EE0 | TeleportLocomotionClass::End_Piggyback | IPiggyback: restores old locomotor |
| 0x00719F30 | TeleportLocomotionClass::Is_Ok_To_End | IPiggyback: check if can swap back |
| 0x0066D750 | RulesClass::ReadGeneral | Parses all chrono INI keys |
| 0x00664C10 | RulesClass::ReadAudioVisual | Parses chrono sound INI keys |
| 0x006CEA20 | SuperWeaponTypeClass::ReadINI | Parses SW type, action, sounds |
| 0x00710490 | TechnoTypeClass::ReadINI | Parses Locomotor= CLSID |

---

## 15. Summary of All INI Keys

### [General] Section (Rules Offsets)
```
ChronoDelay=60              ; Rules+0xBEC - Post-warp lock frames
ChronoReinfDelay=200        ; Rules+0xBF0 - Scripted warp lock frames
ChronoDistanceFactor=32     ; Rules+0xBF4 - delay = distance / factor
ChronoTrigger=yes           ; Rules+0xBF8 - Enable distance-based delay
ChronoMinimumDelay=0        ; Rules+0xBFC - Minimum delay floor
ChronoRangeMinimum=0        ; Rules+0xC00 - Force min delay if distance < this
ChronoHarvTooFarDistance=50 ; Rules+0xD7C - Cell threshold for chrono miner teleport
HarvesterTooFarDistance=5   ; Rules+0xD78 - Cell threshold for harvester return
ChronoBlast=                ; Rules+0x32C - AnimType at source (ChronoSphere)
ChronoBlastDest=            ; Rules+0x328 - AnimType at dest (ChronoSphere)
ChronoPlacement=            ; Rules+0x330 - Chrono building placement anim
ChronoBeam=                 ; Rules+0x334 - Chrono beam anim
WarpIn=                     ; Rules+0x338 - Warp-in anim type
WarpOut=                    ; Rules+0x33C - Warp-out anim type
WarpAway=                   ; Rules+0x340 - Warp-away anim type
ChronoSparkle1=             ; Rules+0x344 - Sparkle during warp
```

### [AudioVisual] Section (Rules Offsets)
```
DefaultChronoSound=         ; Rules+0x1D0 - Default chrono sound
ChronoInSound=              ; Rules+0x218 - Arrival sound
ChronoOutSound=             ; Rules+0x21C - Departure sound
LetsDoTheTimeWarpInAgain=   ; (VocClass name, parsed as sound)
LetsDoTheTimeWarpOutAgain=  ; (VocClass name, parsed as sound)
```

### Per-Type (TypeClass offsets)
```
Locomotor={CLSID}           ; Parsed in TechnoTypeClass::ReadINI
Chronoshiftable=yes/no      ; TypeClass+0xD97 - Can survive ChronoSphere warp
Harvester=yes/no            ; TypeClass+0xE0E - Instant teleport (chrono miner)
ChronoInSound=              ; TypeClass+0x574 - Per-type arrival sound
ChronoOutSound=             ; TypeClass+0x578 - Per-type departure sound
```

### Per-SuperWeapon (SuperWeaponTypeClass offsets)
```
Type=ChronoSphere           ; +0xB4 index 3
Type=ChronoWarp             ; +0xB4 index 4
SpecialSound=               ; +0xC0
StartSound=                 ; +0xC4
```

### CLSIDs
```
TeleportLocomotion: {4A582747-9839-11d1-B709-00A024DDAFD1}  ; at 0x007E9A90
```
