# FlyLocomotionClass — Ghidra Research Report

**Primary Address:** `0x004CD600` (Process), `0x004CC9A0` (Constructor)
**ILocomotion VTable:** `0x7E89F4`
**Confidence:** HIGH (all findings decompiled and cross-referenced in binary)
**Active in YR:** Yes — used by all Fly-locomotor aircraft (Harrier, Black Eagle, Kirov, Nighthawk, etc.)

## 1. Overview

FlyLocomotionClass is the COM locomotion controller for all fixed-wing/airship aircraft
in gamemd.exe. It implements `ILocomotion` and manages: horizontal flight along the
current **facing direction** (NOT directly toward the destination), a two-speed ramping
system (TargetSpeed/CurrentSpeed), altitude state management, approach deceleration
zones, crash/destruction sequences, and bridge height compensation.

**Critical architectural difference from our implementation:** The original moves aircraft
in the direction they are **currently facing**, not directly toward the goal. Turning
is gradual (controlled by ROT via FacingClass). This creates curved flight paths.
Our code moves directly toward the goal with instant facing changes.

**Facing-based movement verified from assembly** at `0x4CDA62`–`0x4CDAF5`:
```asm
LEA ECX,[EAX + 0x388]        ; body FacingClass at linkedObject+0x388
CALL RateTimer__Current       ; → current facing (16-bit) in EBX
CALL [ILoco_vtable + 0x84]   ; speed = ftol(ScaledSpeed * CurrentSpeed)
MOVSX ECX, BX                ; facing to signed
SUB ECX, 0x3FFF              ; shift range
FILD / FMUL [0x7E2810]       ; × (-2π/65536) → radians
CALL Cos_lookup               ; cos(angle)
FMUL [speed]                 ; × speed
FSUBR [pos.Y]                ; new_Y = pos.Y - cos*speed
CALL Sin_lookup               ; sin(angle)
FMUL [speed]                 ; × speed
FIADD [pos.X]                ; new_X = pos.X + sin*speed
CALL CoordStruct__Set         ; update position
```
The destination direction is ONLY used in `Horizontal_Step` to set the **desired**
facing on the FacingClass, which then gradually turns toward it at the ROT rate.

## 2. FlyLocomotionClass Struct Layout

**Total size:** 0x60 bytes (96 bytes). Constructor at `0x004CC9A0`.

### Base Fields (from LocomotionClass)

| Offset | Size | Type | Field | Init | Evidence |
|--------|------|------|-------|------|----------|
| +0x00 | 4 | ptr | IUnknown_vtable | vtable | Constructor |
| +0x04 | 4 | ptr | ILocomotion_vtable | vtable | Constructor |
| +0x08 | 4 | FootClass* | Owner (ILoco-relative) | — | Link_To_Object |
| +0x0C | 4 | FootClass* | Owner (primary) | — | All functions read this |
| +0x10 | 1 | bool | IsPowered | true | Base constructor |
| +0x11 | 1 | bool | IsLockedDown | true | Base constructor |

### FlyLocomotionClass-Specific Fields

| Offset | Size | Type | Field | Init | Evidence |
|--------|------|------|-------|------|----------|
| +0x18 | 1 | bool | IsLandingAtDock | false | Descent_Step reads; Move_To_Coord reads |
| +0x1C | 12 | CoordStruct | Destination (X,Y,Z) | NullCoord | Move_To_Coord sets; Process reads |
| +0x28 | 12 | CoordStruct | HeadTo (X,Y,Z) | NullCoord | Secondary heading target |
| +0x34 | 1 | bool | HasMoveDestination | false | Move_To_Coord sets to 1; Descent_Step clears |
| +0x38 | 4 | int | FlightLevel | 0 | Begin_Takeoff sets from GetSpeed(); Begin_Landing sets to 0 |
| +0x3C | 4 | — | (padding) | 0 | 8-byte alignment for double |
| +0x40 | 8 | double | TargetSpeed | 0.0 | Horizontal_Step sets; Process reads |
| +0x48 | 8 | double | CurrentSpeed | 0.0 | Process ramps toward TargetSpeed |
| +0x50 | 1 | bool | IsAscending | false | Begin_Takeoff sets; Ascent_Step clears |
| +0x51 | 1 | bool | IsDescending | false | Begin_Landing sets; Descent_Step clears |
| +0x52 | 1 | bool | HasPlayedLandingFX | false | Descent_Step sets at alt<300; Begin_Landing resets |
| +0x53 | 1 | bool | IsTurning | false | FUN_004CE5A0 checks; controls strafing turn anim |
| +0x54 | 4 | int | TurnCounter | 0 | FUN_004CE5A0 decrements toward 0 each tick |
| +0x58 | 4 | int | CrashCounter | 0 | Process increments by 1 or 3 per tick when crashing |
| +0x5C | 1 | bool | IsStrafe | false | Process reads; controls ground-hugging flight level |

## 3. Core Algorithm: Process (0x004CD600)

Process is called once per game tick. It orchestrates all flight behavior.

### 3.1 Overall Tick Flow

```
1. Position sync check (radio update if aircraft displaced)
2. Guard mission check
3. Is_Moving / Is_Powered gate
4. IF NOT moving AND height > 0:
   → Crash sequence (height decreases by 1 or 3/tick → explosion at ground)
5. IF health > 0 AND has destination:
   → Guard mission auto-assign if applicable
6. Is_Powered gate
7. Get aircraft TypeClass
8. Disable display culling
9. Get current position (X, Y, Z leptons)
10. Get current facing from FacingClass/RateTimer
11. Compute per-tick movement speed:
    iVar7 = ftol(TypeClass.ScaledSpeed * CurrentSpeed)
12. IF speed > 0:
    → Compute angle from facing (16-bit → radians)
    → dx = cos(angle) * speed; dy = sin(angle) * speed
    → Update position to (X+dx, Y+dy)
    → Bounds check; if out of bounds: FlyBack wrap or random relocation
13. Bridge height compensation (if on bridge cell, subtract bridge height)
14. Compute distance to destination (Euclidean via sqrt)
15. IF height < FlightLevel:
    → Ascent logic (altitude steps, dynamic step size)
16. IF height >= FlightLevel:
    → Descent logic or cruise maintenance
17. Speed computation based on distance/strafing/ConsideredAircraft
18. Landing check (at destination + speed zero → Begin_Landing)
19. Speed ramping: CurrentSpeed → TargetSpeed by ±0.1/tick
20. Re-enable display culling
```

### 3.2 Movement Direction = FACING, Not Goal

**Address:** Process at `0x004CD600`, lines ~220-240.
**Assembly at 0x004CFE20 (ILocomotion vtable entry 0x21):**

```asm
MOV ESI, [ESP+0x8]         ; this = ILocomotion*
MOV ECX, [ESI+0x8]         ; linkedObject
MOV EAX, [ECX]             ; vtable
CALL [EAX+0x84]            ; GetType() → TypeClass* in EAX
FILD dword ptr [EAX+0x678] ; Load ScaledSpeed (int→float)
FMUL double ptr [ESI+0x44] ; Multiply by CurrentSpeed (double)
CALL Math__ftol             ; Convert to int
```

**Formula:**
```
movement_per_tick = floor(TypeClass.ScaledSpeed * CurrentSpeed)
```

Where:
- `ScaledSpeed` (TypeClass+0x678) = `Speed_from_INI * 256 / 100`, capped at 255
- `CurrentSpeed` (FlyLoco+0x48) = smooth-ramped fraction (0.0 to 1.0)

**Position update:**
```
angle = (facing_16bit - 0x3FFF) * (-2π / 65536)
new_X = pos_X + floor(cos(angle) * movement_per_tick)
new_Y = pos_Y + floor(sin(angle) * movement_per_tick)
```

The aircraft moves in the direction it is **facing**, not toward the destination.
Facing is updated gradually by the FacingClass (ROT-controlled turn rate).
This creates curved approach paths.

### 3.3 Speed Scaling Examples

| Aircraft | INI Speed= | ScaledSpeed (×256/100) | Full speed lep/tick | At 15fps: cells/sec |
|----------|-----------|------------------------|--------------------|--------------------|
| ORCA (Harrier) | 14 | 35 | 35 | 2.05 |
| BEAG (Black Eagle) | 14 | 35 | 35 | 2.05 |
| HORNET | 12 | 30 | 30 | 1.76 |
| CARGOPLANE | 15 | 38 | 38 | 2.23 |
| BPLN (MIG) | 16 | 40 | 40 | 2.34 |
| Kirov | 5 | 12 | 12 | 0.70 |

## 4. Speed System: Two-Speed Ramping

### 4.1 TargetSpeed and CurrentSpeed

The locomotor has TWO speed values:
- **TargetSpeed** (+0x40, double): The desired speed fraction. Set by Horizontal_Step
  and Process based on distance, altitude, and strafing state.
- **CurrentSpeed** (+0x48, double): The actual speed used for movement. Smoothly
  approaches TargetSpeed each tick.

### 4.2 Speed Ramping (end of Process)

**Acceleration/deceleration step:** `_DAT_007e3860` = **0.1** per tick
(verified: `0x3FB9999999999999A` = IEEE 754 double 0.1)

```
if CurrentSpeed < TargetSpeed:
    CurrentSpeed = min(CurrentSpeed + 0.1, TargetSpeed)
if CurrentSpeed > TargetSpeed:
    CurrentSpeed = max(CurrentSpeed - 0.1, TargetSpeed)
```

This means a full acceleration (0→1) takes **10 ticks**, and a full deceleration
(1→0) also takes **10 ticks**.

### 4.3 Approach Speed Zones (Horizontal_Step at 0x4CEFB0)

For non-missile aircraft when approaching destination (FUN_004D0180 returns true):

| Distance to target | TargetSpeed | Constant verified |
|---------------------|-------------|-------------------|
| ≥ 768 leptons (3 cells) | 1.0 | `0x3FF00000_00000000` |
| < 768, ≥ 512 leptons | 0.75 | `0x3FE80000_00000000` |
| < 512, ≥ 128 leptons | 0.5 | `0x3FE00000_00000000` |
| < 128 leptons | 0.0 + begin landing | `0x00000000_00000000` |

### 4.4 Fine Approach Speed (Process)

When aircraft is NOT strafing and NOT a ConsideredAircraft (missile):

```
TargetSpeed = min(distance / TypeClass.Speed, 1.0)

if TargetSpeed < 0.1:          ; _DAT_007e3860 = 0.1
    if distance < 86 leptons:  ; 0x56
        TargetSpeed = 0.0
        CurrentSpeed *= 0.5    ; _DAT_007e1738 = 0.5 (rapid decel)
    else:
        TargetSpeed = 0.1      ; 0x3FB9999999999999A

if distance < CurrentSpeed:
    CurrentSpeed = distance    ; don't overshoot

if TargetSpeed == 0 AND CurrentSpeed == 0 AND distance > 0:
    CurrentSpeed = 0.05        ; _DAT_007e8ae8 = 0.05 (minimum creep)
```

**Constants verified from binary:**
| Address | Value | Purpose |
|---------|-------|---------|
| 0x7E3860 | 0.1 | Acceleration step per tick |
| 0x7E1738 | 0.5 | Rapid deceleration multiplier (halve speed) |
| 0x7E8AE8 | 0.05 | Minimum creep speed |

### 4.5 ConsideredAircraft (Missile) Speed

For units with TypeClass+0xD27 set (ConsideredAircraft flag):
- If IsAscending is clear AND has TarCom (target): TargetSpeed = 1.0 (always full speed)
- Else: TargetSpeed = 0.0

### 4.6 Strafing Speed

During strafing (IsStrafe set, +0x5C), from Horizontal_Step:

```
if distance < TypeClass.Speed:
    decel_ratio = 1.0 - (distance / TypeClass.Speed)
    FlightLevel = GetSpeed() * decel_ratio / 3
```

And from Process, strafing speed uses constants:
- `_DAT_007e3558` = **0.6** (inner boundary factor)
- `_DAT_007e3550` = **0.4** (range divisor)

```
speed_fraction = 1.0 - (distance - Speed*0.6) / (Speed*0.4)
```

## 5. Flight Level (Altitude)

### 5.1 FlightLevel INI Key

**Two sources, verified from binary xrefs to string at 0x0083C854:**

| Source | Struct Offset | INI Section | Default |
|--------|---------------|-------------|---------|
| RulesClass (global) | +0x7B4 | `[General]` | Self (constructor init) |
| TechnoTypeClass (per-type) | +0x618 | `[TypeName]` | -1 (sentinel → use global) |

**INI value:** `[General] FlightLevel=1500` (raw leptons, no scaling)

**GetSpeed() at 0x00717800:**
```asm
MOV EAX, [ECX + 0x618]    ; TypeClass.FlightLevel
CMP EAX, -1               ; if -1 (unset)
JNZ return_it
MOV EAX, [g_RulesClass]   ; fallback to global
MOV EAX, [EAX + 0x7B4]    ; RulesClass.FlightLevel
return_it:
RET
```

Most aircraft don't set per-type FlightLevel, so they all use the global
**1500 leptons** ≈ 5.86 cells of altitude.

### 5.2 FlightLevel Assignment

In `Begin_Takeoff` (0x4CF950):
```
this->FlightLevel = linkedObject->GetType()->GetSpeed()
```

In `Begin_Landing` (0x4CFA70):
```
this->FlightLevel = 0
this->IsDescending = true
this->HasPlayedLandingFX = false
```

### 5.3 Altitude Stepping (in Process)

Altitude change per tick is NOT a constant rate. It uses dynamic step sizes:

**Ascending (height < FlightLevel):**
```
if IsDescending:     ; recovering from descent
    step = (height - FlightLevel) / 20 + 10
    step = clamp(step, step, 48)
elif NOT IsAscending:
    step = (height - FlightLevel) / 20
    step = clamp(step, 20, 50)
else:               ; normal takeoff
    typeSpeed = GetType()->GetSpeed()
    if FlightLevel == typeSpeed:
        step = min(height, 16)
    else:
        step = min(height, 6)
```

**Descending (height > FlightLevel):**
```
step = computed_similarly_to_ascending
new_height = height - step
```

### 5.4 Altitude-Based Movement Gating (Ascent_Step at 0x4CE680)

Two thresholds control when horizontal movement begins during takeoff:

```
flight_range = FlightLevel - ground_height
height_above_ground = current_height - ground_height

if height_above_ground > flight_range * 2/3:    ; above 67%
    → full horizontal movement + turning enabled
elif height_above_ground > flight_range / 2:     ; above 50%
    → begin turning toward destination (atan2 to dest)
    → TargetSpeed = 1.0
    → but NO horizontal movement yet
else:                                            ; below 50%
    → no horizontal movement, no turning
```

### 5.5 Bridge Height Compensation

**Process, Ascent_Step, Descent_Step** all check:
```
cell = Get_Cell_At(position)
if cell.flags & 0x100:   ; bridge present
    effective_height = height - DAT_008b3cac   ; subtract bridge height
```

## 6. Crash/Destruction Sequence

**Process at 0x004CD600, early section.**

When `Is_Moving` returns false and aircraft has no health:

```
if health > 0:
    CrashCounter += 3    ; fast descent while alive
else:
    CrashCounter += 1    ; slower for dead units

position.Z -= CrashCounter

if position.Z reaches ground (GetHeight() <= 0):
    if owner has weapon:
        Fire weapon at ground
        Create explosion animation (0x2600 Z-offset)
        Apply_area_damage(damage = RulesClass+0xFA8)
        Destroy unit
        return
    else:
        Play crash sound (based on terrain type)
        → water: TypeClass+0x53C (CrashSea sound)
        → land: TypeClass+0x540 (CrashLand sound)
        → fallback to RulesClass+0x200/0x204
        Destroy unit
```

**Landing FX:** When altitude < 300 leptons and `HasPlayedLandingFX` is false:
- Create landing dust animation (ConsideredAircraft: PIFFPIFF; else: Carryall check)
- Play landing sound
- Set HasPlayedLandingFX = true

## 7. Facing & Turning System

### 7.1 FacingClass (0x004C9300)

Aircraft facing is managed by FacingClass, which stores:
- Current facing (16-bit, 0-65535 = full circle)
- Desired facing (16-bit)
- ROT-based turn rate

The turn algorithm uses timer-based interpolation:
```
diff = CurrentFacing - DesiredFacing
step = abs(diff) / ROT
per_frame_change = diff / step    ; ≈ ROT units per frame
CurrentFacing -= per_frame_change * frames_remaining
```

### 7.2 ROT Values for Aircraft

| Aircraft | ROT | Turn behavior |
|----------|-----|---------------|
| ORCA (Harrier) | 3 | Slow, wide turns |
| BEAG (Black Eagle) | 3 | Same as Harrier |
| HORNET | 3 | Same |
| CARGOPLANE | 2 | Even slower turns |
| BPLN (MIG) | 2 | Slow turns |
| Kirov | 5 | Moderate turns |

### 7.3 Facing Update in Ascent_Step

At >50% altitude, facing begins turning toward destination:
```
angle = atan2(dest.Y - pos.Y, pos.X - dest.X)
desired_facing = angle_to_facing(angle)
FacingClass::Set(desired_facing)
```

At >67% altitude, full horizontal movement and facing update via RateTimer.

### 7.4 Facing Update in Horizontal_Step

When approaching destination:
```
angle = atan2(pos.Y - dest.Y, dest.X - pos.X)
desired_facing = angle_to_facing(angle)
RateTimer::Set(desired_facing)    ; gradual turn via ROT
```

When near destination (< 0x100 leptons) and strafing, facing snaps via
`atan2` directly without gradual turning.

## 8. INI Keys

### Per-Type (TechnoTypeClass)

| INI Key | Offset | Type | Default | Effect on Flight |
|---------|--------|------|---------|------------------|
| Speed | +0x678 (scaled) | int | -1 | Movement per tick = Speed*256/100 * CurrentSpeed |
| FlightLevel | +0x618 | int | -1 (→global) | Target altitude in leptons |
| ROT | varies | int | — | Turn rate (lower = wider turns) |
| ConsideredAircraft | +0xC95 | bool | false | Different speed logic (always full speed) |
| FlyBy | +0xE0B | bool | false | Strafing multi-pass behavior |
| FlyBack | +0xE0C | bool | false | Reverse after fly-by |
| AirportBound | +0xE0D | bool | false | Must dock at helipad |
| Fighter | +0xE0E | bool | false | Fighter classification |
| Landable | +0xE0A | bool | false | Can land on ground |
| PitchSpeed | +0x3B0 (double) | double | — | Visual pitch rate (render only) |

### Global (RulesClass)

| INI Key | Offset | Type | Value | Purpose |
|---------|--------|------|-------|---------|
| FlightLevel | +0x7B4 | int | 1500 | Default cruise altitude (leptons) |

## 9. Integration Points

### Who Calls Process?

Process is called via ILocomotion::Process (vtable entry) from `FootClass::AI()`
on every game tick for each aircraft entity.

### Tick Order

In `World::advance_tick`:
1. Ground movement
2. **Air movement** ← FlyLocomotionClass::Process called here
3. Vision + Power
4. Combat (aircraft fire here)
5. Aircraft missions
6. Docking/reload

### What Process Calls

| Function | Address | Purpose |
|----------|---------|---------|
| Ascent_Step | 0x4CE680 | Two-phase takeoff altitude gating |
| Descent_Step | 0x4CE840 | Landing with passability check + occupancy |
| Horizontal_Step | 0x4CEFB0 | Speed zones + facing + attack cell redirect |
| Begin_Landing | 0x4CFA70 | Initiate descent (FlightLevel→0, IsDescending=true) |
| Begin_Takeoff | 0x4CF950 | Initiate ascent (FlightLevel←GetSpeed(), IsAscending=true) |
| FUN_004CE5A0 | 0x4CE5A0 | Strafing turn animation system |
| FUN_004D0180 | 0x4D0180 | "Is approaching destination?" gate for speed zones |
| FacingClass::UpdateFacing | 0x4C9300 | Gradual ROT-based turning |

## 10. Current Rust Implementation Status

### What's Implemented

| Feature | File | Status |
|---------|------|--------|
| Altitude state machine (5 phases) | air_movement.rs | ✅ Basic version |
| Straight-line Bresenham paths | air_movement.rs | ✅ (differs from original) |
| Lepton-based horizontal movement | air_movement.rs | ✅ (differs from original) |
| Constant climb rate | air_movement.rs | ✅ (should be dynamic) |
| speed_fraction control | locomotor.rs | ✅ (should be ramped) |
| Jumpjet-specific logic | jumpjet_movement.rs | ✅ |
| Attack state machine (11 states) | attack_mission.rs | ✅ |
| Docking/reload system | aircraft_dock.rs | ✅ |

### What's Missing (Verified Gaps)

| Gap | Impact | Binary Evidence |
|-----|--------|-----------------|
| **Movement uses direction-to-goal, not facing** | HIGH | Process asm at 0x4CFE20: cos/sin of facing, not atan2 to goal |
| **No speed ramping (TargetSpeed/CurrentSpeed)** | HIGH | Process +0x40/+0x48 doubles, ±0.1 step at 0x7E3860 |
| **No distance-based approach speed zones** | HIGH | Horizontal_Step: 0x80/0x200/0x300 thresholds |
| **FlightLevel hardcoded 600 (should be 1500)** | HIGH | [General] FlightLevel=1500; GetSpeed() at 0x717800 |
| **No ROT-based gradual turning** | HIGH | FacingClass at 0x4C9300; ORCA ROT=3 |
| **Altitude-based movement gating: 50%/67%** | MEDIUM | Ascent_Step: `flight_range/2` and `flight_range*2/3` |
| **Dynamic altitude steps (not constant rate)** | MEDIUM | Process: step clamped 6/16/20/48/50 per context |
| **No crash/destruction on ground impact** | MEDIUM | Process: area damage, explosion, destroy |
| **No bridge height compensation** | MEDIUM | Process/Ascent/Descent: `cell_flags & 0x100` |
| **No landing passability check** | LOW | Descent_Step: Find_Nearby_Passable_Cell |
| **No ConsideredAircraft missile behavior** | LOW | Process: TypeClass+0xD27 gates speed logic |
| **No strafing ground-hugging flight level** | LOW | IsStrafe gates FlightLevel = ground_height + delta |
| **Speed scaling (×256/100) not matched** | LOW | TechnoTypeClass ReadINI: Speed*256/100 at +0x678 |

## 11. Open Questions

1. **RateTimer internals:** The exact relationship between ROT, frame rate, and
   per-tick facing change needs more investigation. The FacingClass uses a CDTimer
   internally, and the turn interpolation depends on timer state.

2. **PitchSpeed integration:** PitchSpeed (TypeClass+0x3B0, double) is used in the
   strafing speed formula in Process but only as a visual multiplier. Need to confirm
   this is render-only vs. affecting movement.

3. **HeadTo coords (+0x28-0x30):** The second coordinate set's exact purpose.
   Possibly used for intermediate waypoints during strafing turns.

4. **GameSpeedBias interaction:** `[General] GameSpeedBias=1.6` — does this affect
   the per-tick speed calculation? Or is it applied at a higher level?

5. **FlyBack wrapping:** When aircraft flies off-map with FlyBack=yes, the exact
   re-entry logic in Process needs more tracing.

## Sources

### Ghidra Functions Decompiled
- FlyLocomotionClass::Process (0x4CD600) — 526 lines, fully analyzed
- FlyLocomotionClass::Ascent_Step (0x4CE680)
- FlyLocomotionClass::Descent_Step (0x4CE840)
- FlyLocomotionClass::Horizontal_Step (0x4CEFB0)
- FlyLocomotionClass::Begin_Landing (0x4CFA70)
- FlyLocomotionClass::Begin_Takeoff (0x4CF950)
- FlyLocomotionClass::Constructor (0x4CC9A0)
- FlyLocomotionClass::Move_To_Coord (0x4CCC80)
- FlyLocomotionClass::Render_Matrix (0x4CFB00)
- FlyLocomotionClass::Is_On_Floor (0x4CFE50)
- FlyLocomotionClass::Is_Moving (0x4CCA90)
- FlyLocomotionClass::Layer (0x4CCB40)
- FlyLocomotionClass::Emergency_Relocate (0x4CCFD0)
- FUN_004CFE20 (ILocomotion vtable 0x21 — speed computation)
- FUN_004D0180 (approach destination check)
- FUN_004CE5A0 (strafing turn animation)
- FacingClass::UpdateFacing (0x4C9300)
- TechnoTypeClass::GetSpeed (0x717800)
- TechnoTypeClass::ReadINI — FlightLevel at 0x71233D
- RulesClass::ReadGeneral — FlightLevel at 0x66F2FB

### Memory Reads Verified
- 0x7E3860: 0.1 (acceleration step)
- 0x7E1738: 0.5 (rapid deceleration factor)
- 0x7E8AE8: 0.05 (minimum creep speed)
- 0x7E3558: 0.6 (strafing inner boundary)
- 0x7E3550: 0.4 (strafing range divisor)
- 0x7E2810: -2π/65536 (facing-to-radians conversion)
- ILocomotion vtable at 0x7E89F4, entry +0x84 → 0x4CFE20

### INI Files Checked
- ini/rulesmd.ini — [General] FlightLevel=1500, [ORCA] Speed=14 ROT=3
- ini/rules.ini — same values
