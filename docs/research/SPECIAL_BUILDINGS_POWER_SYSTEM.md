# Special Buildings & Power System Interactions

> **Companion doc.** The authoritative power system reference is
> [POWER_SYSTEM_GHIDRA_REPORT.md](POWER_SYSTEM_GHIDRA_REPORT.md).
> If any detail here conflicts with the main report, the main report is correct.

Research from Ghidra decompilation of `gamemd.exe`. All offsets verified live.

## Key BuildingTypeClass Offsets (from INI parsing at 0x460A93)

| Offset | Field | Type | INI Key |
|--------|-------|------|---------|
| 0x1573 | Powered | bool | `Powered=` |
| 0x1574 | PoweredSpecial | bool | `PoweredSpecial=` |
| 0x16A5 | SpySat | bool | `SpySat=` |
| 0x16BE | LaserFencePost | bool | `LaserFencePost=` |
| 0x16BF | LaserFence | bool | `LaserFence=` |
| 0x16C7 | CloakGenerator | bool | `CloakGenerator=` |
| 0x16C8 | SensorArray | bool | `SensorArray=` |
| 0x0CD1 | GapGenerator | bool | `GapGenerator=` (TechnoType @ 0x713F99) |
| 0x0CD2 | GapRadiusInCells | byte | `GapRadiusInCells=` (TechnoType) |
| 0x0CD3 | SuperGapRadiusInCells | byte | `SuperGapRadiusInCells=` (TechnoType) |
| 0x1707 | CloakRadiusInCells | byte | `CloakRadiusInCells=` |
| 0x170C | PsychicDetectionRadius | int | `PsychicDetectionRadius=` |
| 0x0EE0 | Power (output) | int | `Power=` (positive) |
| 0x0EE4 | Power (drain) | int | `Power=` (negative stored as positive drain) |

## HouseClass Power State Fields

| Offset | Field | Notes |
|--------|-------|-------|
| 0x53A4 | PowerOutput | Total power produced by all buildings |
| 0x53A8 | PowerDrain | Total power consumed by all buildings |
| 0x577A | HasLowPower | Flag set when power output < drain |
| 0x577B | HasPoweredCenter | Whether any power-producing building exists |
| 0x5778 | NeedsRecalc | Flag to trigger power recalculation |
| 0x5779 | NeedsEffectsUpdate | Flag to update powered effects |

## 1. Power Ratio and the IsOperational Check

### The Central Power Ratio (FUN_004fce30 @ 0x4FCE30)
The engine calculates a **power ratio** = `PowerOutput / PowerDrain`:
- If `PowerOutput >= PowerDrain` or `PowerDrain == 0`: ratio = **1.0** (full power)
- If `PowerOutput == 0` but `PowerDrain > 0`: ratio = **0.0** (no power)
- Otherwise: ratio = `(double)PowerOutput / (double)PowerDrain`

The threshold constant at `0x7E1718` is **1.0** (double). A building is considered "powered" if the ratio >= 1.0, meaning exact equality counts as powered.

### BuildingClass::IsOperational (0x4555D0)
**Confidence: HIGH** -- fully decompiled.

This is the **master check** used throughout the engine (vtable offset 0x350). Returns true only if ALL conditions are met:

```c
bool BuildingClass::IsOperational() {
    // 1. Must have completed deployment
    if (!this->HasPower && this->UpgradeLevel < 2) return false;

    // 2. Must not be under EMP effect
    if (this->EMPLockRemaining > 0) return false;

    // 3. Must have health > 0
    if (this->Health == 0) return false;

    // 4. IsPowered check: if BuildingType->Powered(0x1573) is set
    //    AND BuildingType->PowerDrain(0xEE4) > 0,
    //    then check house power ratio < 1.0 → NOT operational
    if (this->Type->Powered && this->Type->PowerDrain > 0) {
        float ratio = House->GetPowerRatio();
        if (ratio < 1.0 && this->UpgradeLevel < 2) return false;
    }

    // 5. PoweredSpecial check: if BuildingType->PoweredSpecial(0x1574)
    //    Check the house timer -- if timer still running, not operational
    //    Also checks House->HasPoweredCenter (0x577B)
    if (this->Type->PoweredSpecial) {
        if (house_timer_still_active) return false;
        if (House->HasPoweredCenter) return false;
    }

    // 6. NeedsEngineer: must have engineer inside
    if (this->Type->NeedsEngineer(0x1552) && !this->HasEngineer) return false;

    // 7. Not in limbo or being sold
    if (GetMission() == SELLING || GetMission() == DECONSTRUCTING) return false;

    return true;
}
```

**Key insight**: `Powered=yes` buildings fail IsOperational when `PowerOutput < PowerDrain`. `PoweredSpecial=yes` buildings have a different check using a timed mechanism with a powered center flag.

## 2. Gap Generator (0x454DB0 -- UpdateGapGeneratorTick, 2076 bytes)

**Confidence: HIGH** -- fully decompiled both pages.

### Power Check
The gap generator does NOT check power independently -- it relies on `BuildingClass::Update` (0x43FB20) which calls `IsOperational()` (vtable 0x350) at the very top:

```c
void BuildingClass::Update() {
    bool isActive = this->IsOperational();
    // Also excludes selling/deconstructing states
    if (!isActive || mission == SELLING || mission == DECONSTRUCTING)
        isActive = false;

    // If state changed from last tick:
    if (isActive != this->PrevActiveState) {
        BuildingClass::UpdateGapAndSpecialEffects();  // 0x4549B0
        this->PrevActiveState = isActive;
    }
    ...
}
```

### Gap State Machine (offset 0x220 = field at param_1[0x88])
The gap generator uses a 4-state machine:
- **State 0**: Inactive/Off -- gap is disabled
- **State 1**: Expanding -- gap is growing outward (shroud radius growing)
- **State 2**: Fully active -- gap at max radius
- **State 3**: Contracting -- gap is shrinking (turning off)

### Gap Expansion/Contraction (offset 0x6ED = animation counter)
- **Expanding (state 1→2)**: Counter increments from 0 to 15. At frames 1, 6, 11 the building marks `NeedsRedraw=1`. When counter reaches 15, transitions to state 2 (fully active). Propagates the counter value to all 21 gap generator overlay cells (offsets 0x55C through 0x5B0).
- **Contracting (state 3→0)**: Counter decrements from current value down to 0. Same redraw triggers at 0, 5, 10. When counter reaches 0, transitions to state 0 (inactive) and creates a new shroud map regeneration object.

### Radius
The gap generator's shroud radius comes from the **shroud map object** created at offset 0xC3 (param_1[0xC3]). The radius calculation uses `BuildingType->offset_0x764` which is the pre-computed gap map data. The gap map dimensions are stored in the shroud object.

### CloakRadiusInCells Usage (in the gap tick, second half)
At offset 0x6EB (gap tick cloaking state) and offset 0x1BB (current cloak radius), the gap generator ALSO manages the **cloak generator** functionality within the same tick function:
- Uses `CloakRadiusInCells` (BuildingType offset 0x1707)
- Checks `CloakGenerator` flag (BuildingType offset 0x16C7)
- Gets the shroud map size via virtual call `*DAT_0089ddc0->vtable[0x7C]()` (divided by 2 for radius)
- Calls `FUN_007bb920` to update the cloak shroud bitmap
- Iterates cells within cloak radius, calling `FUN_004870b0` (check cell visibility) and `FUN_00487130`/`FUN_00487110` (reveal/hide cell) for each
- Nearby CloakGenerator buildings within `(CloakRadiusInCells + 2)^2 * 4` distance squared also get their cloak state synchronized

### What Happens When Gap Turns Off (Low Power)
1. `IsOperational()` returns false → `BuildingClass::Update` detects state change
2. `UpdateGapAndSpecialEffects()` (0x4549B0) is called with `isActive=false`
3. Gap generator state transitions to **State 3** (contracting)
4. Each tick, the gap overlay counter decrements by 1
5. The 21 overlay cell objects get their translucency updated (offset 0x178 of each)
6. At count 0/5/10, building forces redraw
7. When fully contracted (count=0), the gap shroud map object is released
8. **Visually**: the fog-of-war gradually recedes over 15 frames, revealing terrain underneath

## 3. UpdateGapAndSpecialEffects (0x4549B0)

**Confidence: HIGH** -- fully decompiled.

This function handles the **transition** when a building's operational state changes. It covers MULTIPLE special building types in a single dispatcher.

### When Building Becomes Active (isActive == true):
1. **Gap Generator** (`BuildingType->offset_0x40C != 0`): If not already gapping, sets gap flag (`offset 0x662 = 1`) and calls `FUN_0050E010` (which increments the house's gap generator count and starts cloaking all technos of the required type)
2. **Cloak Generator** (`BuildingType->CloakGenerator 0x16C7`): If cloak state `offset_0x6EB < 1`, initiates cloaking expansion. Sets `NeedsRedraw`.
3. **Sensor Array** (`BuildingType->offset_0xCD1` / GapGenerator flag): Calls `vtable[0x414]()` to activate sensor
4. **Laser Fence Posts** (`BuildingType->offset_0x1573` / Powered AND `PowerDrain > 0`): Calls `BuildingClass::OnPowerOff()` -- note: this is counterintuitive but the check is "if Powered and has drain, call OnPowerOff" when becoming active (it's actually toggling the powered animation state)
5. **PoweredSpecial** (`BuildingType->offset_0x1574`): Clears anim slot 0x13, then iterates all 20 anim slots (0x594/0x44 = ~20 slots). For each slot with a "PowerOff" flag, creates the appropriate anim (healthy vs damaged version based on health ratio threshold at `RulesClass+0x1700`).

### When Building Becomes Inactive (isActive == false):
1. **Gap Generator**: Clears gap flag (`offset 0x662 = 0`), calls `FUN_0050E0E0` (decrements house gap count, uncloaks technos)
2. **Laser Fence Beams** (offset 0x2BC = param_1[0xAF]): Calls `FUN_00472140` -- **destroys all laser fence beams** connected to this post
3. **Laser Fence Linked Object** (offset 0x2AC = param_1[0xAB]): Calls `FUN_0070FEE0(1)` -- releases the linked laser fence and triggers fence destruction animation
4. **Cloak Generator**: Sets cloak state to 0xFF (-1 signed), marks `NeedsRedraw`
5. **Sensor Array / GapGenerator**: Calls `vtable[0x418]()` to deactivate sensor; if was cloaked, uncloaks and triggers rebuild
6. **Laser Fence (Powered)**: Calls `BuildingClass::OnPowerOn()` (again, counterintuitive naming but correct -- restores powered anims)
7. **PoweredSpecial**: Creates the "powered special" anim for slot 0x13 using health-based selection, and clears all other powered slot anims

## 4. Cloak Generator Power Behavior

**Confidence: HIGH** -- verified in UpdateGapGeneratorTick and UpdateGapAndSpecialEffects.

The cloak generator uses `IsOperational()` (vtable 0x350) as its power check -- the SAME mechanism as all other powered buildings. This is checked via `BuildingClass::Update` at the top level.

### Cloak expansion/contraction:
- **CloakGenerator** flag at BuildingType offset 0x16C7
- Cloak radius stored at BuildingType offset 0x1707 (`CloakRadiusInCells`)
- Building instance field at offset 0x6EB tracks cloak state:
  - `< 1` (0 or negative): cloaking is inactive
  - `>= 1`: cloaking is active, value represents current cloak radius
- Building instance field at offset 0x1BB (param_1[0x6EC/4]) tracks the current animated radius (grows/shrinks incrementally)

### When power is lost:
1. `UpdateGapAndSpecialEffects` sets cloak state (0x6EB) to 0xFF
2. In `UpdateGapGeneratorTick`, the cloak radius decrements each tick
3. Each frame, `FUN_007BB920` is called to update the cloak bitmap with the shrinking radius
4. Units that were cloaked within the radius gradually lose cloak as the radius contracts
5. At radius 0, the cloak generator iterates ALL other CloakGenerator buildings nearby and triggers them to expand their radius if they're still active (mutual coverage)

## 5. Laser Fence Posts During Low Power

**Confidence: HIGH** -- verified in UpdateGapAndSpecialEffects.

Laser fence posts are affected by power through TWO mechanisms:

1. **Direct beam destruction**: When `UpdateGapAndSpecialEffects` is called with `isActive=false`:
   - Offset 0x2BC (laser fence beam list): `FUN_00472140` iterates all connected beams and destroys each one via `FUN_00471FF0`. This function:
     - Removes the beam animation (offset 0x2C8 of the beam)
     - Plays a sound effect based on BuildingType offset 0x5B0 (or rules default 0x264)
     - Calls `vtable[0x3D4]` to notify the connected post
     - Calls `FUN_004723B0` to clean up beam data
   - Offset 0x2AC (linked laser fence): `FUN_0070FEE0` releases the linked object, which if it's a bridging laser fence:
     - Checks if linked object has passengers
     - If yes, ejects them and sets them hostile
     - Removes the object from the map

2. **Animation toggle**: `BuildingClass::OnPowerOn/OnPowerOff` (0x4547C0 / 0x4545D0) cycle through all 20+ anim slots and toggle between powered/unpowered animation sets. Specifically for slot 10 (index 0xA), there's special handling for `IsAnimDelayedFire` (offset 0x16A7) buildings.

**Result**: When power drops, laser fences between posts **disappear** -- the beams are destroyed and the fence line goes down. When power returns, `BuildingClass::Update` detects the state change, calls `UpdateGapAndSpecialEffects` with `isActive=true`, and the building will re-establish its laser fence connections.

## 6. Psychic Sensor / Sensor Array

**Confidence: MEDIUM** -- offset confirmed, but exact sensor logic is in deeper virtual calls.

- **SensorArray** flag at BuildingType offset 0x16C8
- **PsychicDetectionRadius** at BuildingType offset 0x170C
- Sensor activation/deactivation happens through `vtable[0x414]` (activate) and `vtable[0x418]` (deactivate) in `UpdateGapAndSpecialEffects`
- The sensor is gated by `IsOperational()` like all other powered buildings
- When the building has `GapGenerator` flag (`offset 0xCD1`) set, the sensor check is triggered from `UpdateGapAndSpecialEffects` directly

**Note**: The "PsychicSensor" is NOT a separate INI flag -- it uses the `SensorArray=yes` flag combined with `PsychicDetectionRadius`. The string "PsychicSensorDetectSound" at 0x83A600 is a RulesClass sound key, not a BuildingType flag.

## 7. Super Weapons During Low Power

**Confidence: HIGH** -- FUN_0050AF10 and FUN_006CB4D0 fully decompiled.

### Super Weapon Charging Pause (FUN_006CB4D0)
The super weapon timer system uses a **pause/resume mechanism** based on power:

```c
// Called from FUN_0050AF10 (house super weapon update)
bool SuperClass::SetSuspended(bool suspend) {  // FUN_006CB4D0
    if (!this->IsChargeable) return false;
    if (this->IsReady && !SuperType->IsPowered) return false;  // already ready
    if (this->IsSuspended) return false;  // already suspended

    if (suspend != this->IsSuspended) {
        if (suspend == false) {  // RESUMING
            if (SuperType->UseChargeDrain) {  // offset 0xF5
                // Start fresh recharge from current frame
                this->StartFrame = g_CurrentFrameCounter;
                this->IsSuspended = false;
                return true;
            }
        } else {  // SUSPENDING
            // Save remaining charge time
            int elapsed = g_CurrentFrameCounter - this->StartFrame;
            if (elapsed < this->Duration) {
                this->Duration -= elapsed;
            } else {
                this->Duration = 0;
            }
            this->StartFrame = -1;  // Mark as paused
        }
        this->IsSuspended = suspend;
        return true;
    }
    return false;
}
```

### SuperWeaponTypeClass->IsPowered (offset 0xE6)
- Parsed from `IsPowered=` in the super weapon's INI section (at 0x6CEA20)
- Checked via `FUN_006CC2A0`: returns `SuperWeaponType->offset_0xE6`

### How Power State Reaches Super Weapons (FUN_0050AF10 @ 0x50AF10)
This is called from `HouseClass::Update` when power state changes (`NeedsEffectsUpdate` flag):

1. Iterates all super weapons for this house
2. For each super weapon that is `IsChargeable` and `HasChargeTarget`:
   - Checks if the owning building still exists and is operational
   - Checks if the building's `LaserFence` connections exist
   - **Calculates power ratio**: `PowerOutput / PowerDrain`
   - **If ratio < 1.0** (low power): sets `cVar5 = false` (cannot charge)
   - Calls the super weapon type's `vtable[0x40]` to get sidebar tab
   - If the super weapon CAN charge AND `SuperType->IsPowered`:
     - Calls `FUN_006CC2A0()` to check if super weapon requires power
     - Calls `FUN_006CB4D0(true)` to **suspend** the super weapon (power is insufficient)
   - If the super weapon CANNOT charge due to power:
     - Calls `FUN_006CB7B0()` to **resume/cancel** the super weapon

**Result**: Super weapons with `IsPowered=yes` **pause their charging timer** during low power. The remaining time is saved and charging resumes from where it left off when power is restored. The timer does NOT continue counting during low power.

## 8. EVA Voice Lines for Power Transitions

**Confidence: HIGH** -- traced through decompilation.

### Where EVA is Triggered
The EVA voice for power transitions is triggered from `FUN_00508F60` (called from `HouseClass::Update` when `NeedsEffectsUpdate` flag is set):

```c
// FUN_00508F60 - Power state assessment and EVA trigger
void HouseClass::AssessPowerState() {
    this->NeedsEffectsUpdate = false;

    // Iterate all buildings belonging to this house
    for each building {
        // Skip non-SpySat buildings, dead buildings, limbo buildings
        if (!building->Type->SpySat || building->IsDead || !building->IsAlive)
            continue;

        // Skip observer/spectator buildings
        if (!IsPlayerControlled()) continue;

        // Check if building is NOT in selling/deconstructing
        if (GetMission() != SELLING && GetMission() != DECONSTRUCTING) {
            bool wasLowPower = building->vtable[0x1D4]();  // IsPowerOnline?

            if (wasLowPower == false) {  // Power went DOWN
                if (this->HadLowPower == false) {  // First detection
                    FUN_00577D90(this);  // Apply shroud (gap effect for all)
                    this->HadLowPower = true;

                    if (this == PlayerHouse) {
                        // EVA voice: power lost
                        // ECX = RulesClass->EVA_PowerLost (offset 0x220)
                        FUN_00750920(1.0f_priority, 0);
                    }
                }
                return;
            }
            break;  // Found a powered SpySat building -- power is OK
        }
    }

    // If we get here, all SpySat buildings are powered (power restored)
    if (this->HadLowPower) {
        FUN_00577AB0(this);  // Remove shroud (restore vision)
        this->HadLowPower = false;

        if (this == PlayerHouse) {
            // EVA voice: power restored
            // ECX = RulesClass->EVA_PowerRestored (offset 0x224)
            FUN_00750920(1.0f_priority, 0);
        }
    }
}
```

### Two EVA Voice Systems

The engine has TWO ways to trigger EVA voices:

**1. FUN_00752700 -- EVA by String Name (0x752700)**
- `this` (ECX) = pointer to EVA event name string (e.g. `"EVA_LowPower"`)
- `param_2` = priority (-1 = use default from EVA entry)
- Iterates the global EVA event array at `DAT_00B1D4A4`, doing `strcmp` against each entry
- When found, calls `FUN_00752480(index, priority)` to play it

**2. FUN_00750920 -- EVA by Index (0x750920)**
- `this` (ECX) = pointer to an EVA data structure (from RulesClass offset)
- `param_2` = 0x3F800000 (1.0f as int -- priority)
- `param_3` = 0 (secondary param)
- Looks up the event in the EVA array directly by index

### EVA_LowPower String (0x82473C)
The string `"EVA_LowPower"` at 0x82473C is referenced from `HouseClass::Update` at `0x4F8D0F`. The disassembly shows:
```asm
004f8d0f:  PUSH 0xFF                   ; priority = -1 (use default)
004f8d11:  OR EDX, 0xFFFFFFFF          ; param = -1
004f8d14:  MOV ECX, 0x0082473C         ; "EVA_LowPower" string
004f8d19:  CALL FUN_00752700           ; EVA by string name
```
This call is made from the **power calculation comparison** section of `HouseClass::Update`. It triggers when `PowerOutput < PowerDrain` AND the player has power-producing buildings (line ratio check).

### SpySat Power EVA (FUN_00508F60)
The SpySat-related EVA voice uses the **by-index** method:
```asm
; Power LOST (shroud applied):
0050907e:  PUSH 0x0
00509085:  PUSH 0x3F800000             ; 1.0f priority
0050908a:  MOV ECX, [RulesClass+0x220] ; EVA event for power lost
00509090:  CALL FUN_00750920

; Power RESTORED (shroud removed):
0050902d:  PUSH 0x0
00509034:  PUSH 0x3F800000             ; 1.0f priority
00509039:  MOV ECX, [RulesClass+0x224] ; EVA event for power restored
0050903f:  CALL FUN_00750920
```
The EVA events are stored at RulesClass offsets 0x220 (power lost) and 0x224 (power restored).

### EVA_PowerSabotaged (0x8191B0)
Referenced from `BuildingClass::OnSpyInfiltrate` at 0x457309 -- this is the voice line when a spy cuts power to the enemy base.

### Power Transition Detection
The power state change is detected in `FUN_00508C30` (called when `NeedsRecalc` flag is set):
1. Saves the old power state (was drain > output?)
2. Recalculates `PowerOutput` and `PowerDrain` by iterating all buildings
3. Compares new power state with old
4. If changed: calls `FUN_0050AF10` (update super weapons) and sets `NeedsEffectsUpdate`
5. The effects update then triggers the EVA voice in `FUN_00508F60`

### Spy Satellite Power Loss (Shroud)
`FUN_00577D90` (power lost → apply shroud):
- Sets `HouseClass->offset_0x241` = 1 in the House's side data
- Resets the entire shroud map for the player
- Iterates all map cells and sets shroud flags (0x18 = both shroud bits)
- Forces radar rebuild and map redraw
- Calls `FUN_004F42F0(2)` to update fog state

`FUN_00577AB0` (power restored → remove shroud):
- Clears `HouseClass->offset_0x241` = 0
- Resets shroud map
- Iterates all cells and clears shroud bits (& 0xFFFFFFE7)
- Rebuilds radar and redraws map

## 9. FUN_0050E010 / FUN_0050E0E0 -- Gap Generator House-Level Tracking

These functions manage the **house-level gap generator count** (offset 0x2D8):

### FUN_0050E010 (Gap Enabled):
- Increments `House->GapGeneratorCount` (offset 0x2D8)
- If count goes from 0→1 (first gap generator), iterates ALL technos belonging to this house
- For each techno matching the required type: calls `FUN_0070FBE0` (cloak the unit)
- If any units were cloaked AND this is the player's house: plays EVA voice `FUN_00752700(-1)`

### FUN_0050E0E0 (Gap Disabled):
- Decrements `House->GapGeneratorCount`
- If count goes from 1→0 (last gap generator lost), iterates ALL technos
- For each techno matching the type: calls `FUN_0070FC90` (uncloak the unit)
- If any units were uncloaked AND this is the player's house: plays EVA voice

## Summary Table: How Each Special Building Checks Power

| Building | INI Flag | Power Check | Effect When Unpowered |
|----------|----------|-------------|----------------------|
| Gap Generator | `GapGenerator=yes` | `IsOperational()` via BuildingClass::Update | Gap shroud contracts over 15 frames, then releases |
| Cloak Generator | `CloakGenerator=yes` | `IsOperational()` via BuildingClass::Update | Cloak radius contracts, units gradually uncloak |
| Laser Fence Post | `LaserFencePost=yes` | `IsOperational()` via BuildingClass::Update | Fence beams destroyed, fence lines go down |
| Psychic Sensor | `SensorArray=yes` | `IsOperational()` via BuildingClass::Update | Sensor deactivated via vtable[0x418] |
| Spy Satellite | `SpySat=yes` | `IsOperational()` via FUN_00508F60 | Shroud restored (all revealed terrain re-fogged) |
| Super Weapons | `IsPowered=yes` (SuperWeaponType) | Power ratio check in FUN_0050AF10 | Charging timer PAUSED (saved remaining time) |
| Powered buildings | `Powered=yes` | `IsOperational()` power ratio < 1.0 | Not operational, anims switch to unpowered set |
| PoweredSpecial | `PoweredSpecial=yes` | `IsOperational()` timed + HasPoweredCenter | Not operational until powered center exists |

All power checks ultimately flow through the same mechanism:
1. `FUN_00508C30` recalculates `PowerOutput/PowerDrain` each tick
2. `IsOperational()` checks the ratio against 1.0
3. `BuildingClass::Update` detects state changes and calls `UpdateGapAndSpecialEffects`
4. `FUN_00508F60` handles SpySat shroud and EVA voice
5. `FUN_0050AF10` handles super weapon pause/resume
