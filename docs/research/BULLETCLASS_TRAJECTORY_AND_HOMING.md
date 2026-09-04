# BulletClass: Trajectory Calculation, Homing Behavior, and Air-Burst Logic

**Program:** gamemd.exe
**Date:** 2026-04-06
**Confidence:** High (verified from binary decompilation of all key functions)
**Complements:** `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`, `BULLET_CLASS_AI_GHIDRA_REPORT.md`

This report fills the gaps in the existing BulletClass documentation by providing
detailed analysis of trajectory math, homing algorithms, air-burst mechanics, and
shrapnel spawning, with actual formulas extracted from decompilation.

---

## 1. Trajectory Calculation

### 1.1 Arcing Trajectory (Arcing=yes, ROT<=0)

**Ballistic projectiles** (tank shells, lobbed warheads). No course correction.

**Per-tick update:**
```
1. Read velocity: VelX (0xE8), VelY (0xF0), VelZ (0xF8) — three doubles
2. speed = sqrt(VelX^2 + VelY^2 + VelZ^2)
3. If speed < 8.0: flag as arrived, detonate
4. Apply gravity: VelZ -= Gravity
   - Gravity = Rules.Gravity (RulesClass+0x16B8, default 6)
   - If Floater=yes: Gravity = FUN_0048ACF0() (variable gravity)
5. Convert velocity to position delta (ftol each component)
6. new_pos = old_pos + delta
```

**Ground collision (non-Vertical):**
```
ground_height = CellClass::GetGroundHeight(new_pos) + DAT_0089de64
if new_pos.Z < ground_height:
    Check for building in cell:
        - Skip if building is firer's target (pass-through)
        - Skip if building has 7+ occupants and "GarrisonImmune" flag
        - Skip if building.IsImmune()
        - Skip if building is ally of firer
        - Otherwise: detonate on building
    If no building hit:
        Adjust Z to ground_height - 100 (non-bridge) or bridge_height - 20
        Perform bounce reflection (see section 5)
```

**Bridge collision:**
```
bridge_height = ground_height + DAT_0089de64  (bridge plane altitude)
if (cell_flags & 0x100) != 0:  // cell has bridge
    if old_Z was above bridge_height and new_Z is at/below: crossed bridge -> detonate
    if old_Z was below bridge_height and new_Z is at/above: crossed bridge -> detonate
```

**Bounce reflection (arcing, no firer target):**
Uses rotation matrices to reflect velocity off terrain normal:
```
1. facing_matrix = VXL_GetFacingMatrix(cell)      // 0x007559B0
2. inverse_matrix = FUN_005AFC20(facing_matrix)    // matrix inverse
3. Transform velocity to terrain-local space via FUN_0043A0B0
4. local_vel = matrix_multiply(inverse, velocity)  // FUN_005AF4D0
5. local_vel.Z = -local_vel.Z * Elasticity         // FUN_0043A0D0
6. velocity = matrix_multiply(facing_matrix, local_vel)
7. Continue with reflected velocity
```

### 1.2 Straight Trajectory (Arcing=no, ROT<=0)

**Straight-line projectiles.** Velocity is integrated directly.

**Per-tick speed ramping:**
```
current_speed = sqrt(VelX^2 + VelY^2 + VelZ^2)
if current_speed < target_speed:
    new_speed = current_speed + Acceleration  (BulletTypeClass offset 0x2D0)
    // No cap check; acceleration adds until >= target
if velocity == (0,0,0):
    VelX = 100.0  // prevent stuck bullet (0x40590000 IEEE)
// Normalize and scale:
scale = new_speed / current_speed
VelX *= scale; VelY *= scale; VelZ *= scale
```

**Position update:**
```
delta.X = ftol(VelX)
delta.Y = ftol(VelY)
delta.Z = ftol(VelZ)
new_pos = old_pos + delta
```

**Detonation checks (straight):**
```
if new_pos.Z > DetonationAltitude (BulletTypeClass offset 0x2BC): detonate
if GetHeight() < 0: detonate
Bridge crossing check: same as arcing
```

### 1.3 Vertical Trajectory (Vertical=yes)

For V3 rocket descending phase. Subset of arcing path:
```
1. Apply gravity: VelZ -= Gravity
2. Simple linear movement
3. if new_pos.Z > DetonationAltitude: detonate
4. if new_pos.Z < ground_height: detonate
5. Bridge crossing: detonate
```

### 1.4 Inaccurate Projectiles

When `Inaccurate=yes` (BulletTypeClass offset 0x2A2):
- The bullet aims at the original target coordinates set during Fire_At
- No course correction toward the target's current position
- On proximity trigger, detonates at bullet's current position, NOT at target
- Skips the "snap to target coords" logic in BulletClass::Detonate

The scatter is **not** applied as random offset — it's inherent in the trajectory.
The bullet was aimed at the target's position at fire time; any miss comes from
the target moving after the bullet was fired.

---

## 2. Homing Behavior (ROT > 0)

### 2.1 Speed Ramping

```c
current_speed = sqrt(VelX^2 + VelY^2 + VelZ^2)
target_speed = this->TargetSpeed  // offset 0x110, from WeaponType.Speed
accel = BulletTypeClass.Acceleration  // offset 0x2D0

if current_speed < target_speed:
    new_speed = min(current_speed + accel, target_speed)
elif current_speed > target_speed:
    new_speed = max(current_speed - accel/2, 0)
// Note: deceleration is at HALF the acceleration rate

if velocity == (0,0,0):
    VelX = 100.0  // prevent stuck bullet

// Normalize and scale velocity to new_speed
scale = new_speed / sqrt(VelX^2 + VelY^2 + VelZ^2)
VelX *= scale; VelY *= scale; VelZ *= scale
```

### 2.2 Course Lock

Two modes:

**Mode A: CourseLockDuration > 0** (BulletTypeClass offset 0x2E0)
```
CourseLockCounter++  // offset 0x108, increments each tick
if CourseLockCounter >= CourseLockDuration:
    IsCourseLocked = false  // offset 0x105
```

**Mode B: CourseLockDuration == 0** (default)
```
if target_speed > 39:
    IsCourseLocked = false  // immediate unlock for fast missiles
// corrected 2026-05-28: was "+ 90.0"; binary constant _DAT_007e1738 at 0x007e1738 = 0.5;
// verified via read_memory 0x007e1738 + decompile_function 0x004666E0 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT
elif target_speed <= current_speed + 0.5:
    IsCourseLocked = false  // unlock once near target speed
// Also: when CourseLockDuration==0 and IsCourseLocked:
//   acceleration is set to (g_CurrentFrame & 1) — alternates 0/1
//   This creates a slow, wobbly launch phase
```

### 2.3 Target Tracking

The bullet gets the target's position each tick:
```c
target_pos = Target->GetCoords()  // vtable+0x58 on Target (offset 0x10C)
if Target != NULL && (Target.AbstractFlags & 0x2) != 0:
    // Target is on map: use ReceiveDamage coords (vtable+0xA4) instead
    target_pos = Target->GetTargetCoords()
```

### 2.4 Wobble Effect

For homing bullets, a sinusoidal wobble perturbs the facing:
```c
network_id = this->vtable_secondary[4](this+4)  // get unique network ID
wobble_index = (network_id + g_CurrentFrameCounter) % 15
wobble_angle = Cos_lookup(wobble_index * (2*PI / 15))
wobble_byte = ftol(wobble_angle)
```

The wobble is **suppressed when IsCourseLocked is true:**
```c
wobble_byte = wobble_byte & ~(-(IsCourseLocked != 0))
// When locked: wobble_byte = 0 (no wobble)
```

The wobble byte is then scaled into a facing delta:
```c
wobble_facing = wobble_byte << 8  // shift to facing units
```

### 2.5 Homing Turn Logic (BulletClass__HomingTrack, 0x005B20F0)

**Parameters:**
```c
void HomingTrack(
    CoordStruct* out_new_pos,  // output: new position after move
    double* velocity,          // 3 doubles (VelX, VelY, VelZ) — modified in place
    CoordStruct* target_pos,   // target position {X, Y, Z}
    uint* rot_param,           // ROT in facing units (shifted)
    byte airburst,             // BulletTypeClass.Airburst
    byte composite,            // CONCAT11(wobble_byte, VeryHigh)
    byte level                 // BulletTypeClass.Level
)
```

**Case 1: Target position is invalid sentinel {0,0,0} (target lost)**
```
1. Compute current heading from velocity: atan2(VelX, -VelY) -> current_facing
2. Pick random target facing: 0x2000 (45 degrees)
3. Check if random facing is within ROT of current facing
4. If yes: snap to it
5. If no: turn toward it by ROT amount (CW or CCW per shortest path)
6. Compute pitch from velocity's vertical component
7. Apply new heading/pitch to velocity vector
```

**Case 2: Target position is valid**
```
1. Convert velocity to position delta, compute tentative new position:
   new_pos = current_pos + ftol(velocity)

2. Compute vector from new_pos to target_pos:
   delta = target_pos - new_pos
   distance = sqrt(delta.X^2 + delta.Y^2 + delta.Z^2)

3. Compute DESIRED heading: atan2(delta.X, -delta.Y) -> desired_facing
4. Compute CURRENT heading: atan2(VelX, -VelY) -> current_facing

5. Check if desired_facing is within ROT of current_facing:
   Call Facing__IsWithinROT(desired_facing, current_facing, ROT)

6. If within ROT: snap heading to desired_facing
7. If NOT within ROT:
   a. Get turn delta: Facing__GetTurnDelta(current_facing, desired_facing)
   b. If delta < 0: new_heading = current_facing - ROT (turn CW)
   c. If delta >= 0: new_heading = current_facing + ROT (turn CCW)

8. Compute desired pitch: atan2(delta.Z, horizontal_magnitude)

9. TERRAIN AVOIDANCE (non-Airburst, non-Level):
   Only applies when (ROT/128 + 1) / 2 >= 2 (i.e., ROT >= 128):
   AND (Inaccurate OR (VeryHigh==false AND distance > threshold)):

   half_rot = ROT / 2

   a. Project velocity forward, get ground height at projected cell
   b. Check bridge presence: add bridge height if bridge flag set
   c. Compute altitude_clearance:
      clearance_cells = clamp(distance/256, 0..5)  // or 10 if Inaccurate/VeryHigh
      required_altitude = ground_height + clearance_cells * cell_size

   d. altitude_margin = bullet.Z - required_altitude
   e. If margin < -20: pitch UP by +18 leptons each tick
      If margin > +20: pitch DOWN by -18 leptons each tick
   f. If margin < -(cell_size/2):
      Force pitch toward UP (0x2000 facing), using half_rot
   g. If margin > +(cell_size/2):
      Force pitch toward DOWN (0x4800 facing), using half_rot
   h. Otherwise: pitch toward level (0x4000 facing), using half_rot

10. If NOT terrain avoidance path (Airburst or Level or small ROT):
    Simple pitch tracking:
    a. Compute pitch delta = 0x100 (small step)
    b. Get random pitch jitter via FUN_005B2970
    c. Clamp pitch using Facing__ClampToROT

11. Apply final pitch to velocity via Velocity__ApplyPitch:
    - Decompose speed into horizontal_mag and vertical
    - horizontal_components = VelX, VelY (divided by sin(old_pitch) to remove old pitch)
    - Apply new pitch: VelX *= sin(new_pitch), VelY *= sin(new_pitch)
    - VelZ = total_speed * cos(new_pitch)
```

### 2.6 Facing Helper Functions

**Facing__IsWithinROT (0x005B2990):**
```c
bool IsWithinROT(short* desired, short* current, short* rot) {
    short delta = abs((short)(*desired - *current));
    short max_turn = abs(*rot);
    return delta <= max_turn;
}
```

**Facing__GetTurnDelta (0x005B2950):**
```c
short GetTurnDelta(short* current, short* desired) {
    return *current - *desired;
    // Negative = turn CW, Positive = turn CCW
}
```

**Facing__ClampToROT (0x005B29C0):**
```c
void ClampToROT(short* current, short* desired, short* rot) {
    short delta = abs(*current - *desired);
    if (delta <= abs(*rot)):
        *current = *desired  // close enough, snap
    elif (*desired - *current) < 0:
        *current = *current - *rot  // turn CW by ROT
    else:
        *current = *current + *rot  // turn CCW by ROT
}
```

**Velocity__ApplyPitch (0x005B2A30):**
```c
void ApplyPitch(double* velocity, short* pitch_facing) {
    // Get current pitch from velocity
    horiz_mag = sqrt(VelX^2 + VelY^2)
    current_pitch = atan2(VelZ, horiz_mag)
    total_speed = sqrt(VelX^2 + VelY^2 + VelZ^2)

    // Remove old pitch from horizontal components
    // corrected 2026-05-28: was "(current_pitch - 0x3FFF) * (2*PI / 65536)";
    // binary constant _LAB_007e2810 at 0x007e2810 = -2*PI/65536 (negative);
    // verified via read_memory 0x007e2810 + decompile_function 0x005B2A30 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT
    if current_pitch != 0:
        pitch_rad = (current_pitch - 0x3FFF) * (-2*PI / 65536)
        VelX /= sin(pitch_rad)
        VelY /= sin(pitch_rad)

    // Apply new pitch
    new_pitch_rad = (*pitch_facing - 0x3FFF) * (-2*PI / 65536)
    VelX *= sin(new_pitch_rad)
    VelY *= sin(new_pitch_rad)
    VelZ = total_speed * cos(new_pitch_rad)
}
```

Note: Facings use a 16-bit unsigned integer space where 0x0000 = North, 0x4000 = East,
0x8000 = South, 0xC000 = West. Pitch uses a different convention: 0x3FFF is level
(horizontal), 0x0000 is straight up, 0x7FFF is straight down.

### 2.7 Proximity Check (Homing Path)

After movement, homing bullets check detonation conditions:

```c
// Distance-based proximity
distance_to_target = sqrt((new_pos - target_pos)^2)
// corrected 2026-05-28: was "speed * 90.0 >= distance_to_target"; binary shows
// condition is distance_to_target <= speed * 0.5 (constant at 0x007e1738 = 0.5,
// not 90.0); verified via decompile_function 0x004666E0 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT
if distance_to_target <= speed * 0.5:
    detonate
if GetHeight() < 1:
    detonate

// Target-snap: if detonating and height > 0 and !Airburst:
if target_pos != invalid_sentinel:
    snap bullet position to target_pos
```

### 2.8 Approach-Rate Averaging

Detects "fly-by" (bullet orbiting or unable to close):

```c
old_dist = distance(old_pos, target_pos)
new_dist = distance(new_pos, target_pos)
approach_delta = old_dist - new_dist  // positive = closing

if IsCourseLocked: skip approach tracking

if ApproachSampleCount < 60:
    ApproachSampleCount++
    ApproachSum += approach_delta
else:
    // Exponential moving average with decay 0.983
    ApproachSum = ApproachSum * 0.983 + approach_delta

    if 0 <= ApproachSum < 60.0:
        if !Airburst && !VeryHigh:
            force detonate  // bullet has stopped closing
```

### 2.9 Lost Target Handling and Chrono-Warp Retargeting

There are **two distinct mechanisms** that change a homing bullet's Target — frequently
confused because both end with Target being changed or detonating, but they have
completely different callers and triggers:

**(a) Lost-target detonation in `BulletClass::AI`** — for homing missiles whose target
just died or moved off-map. Inline in AI, no helper call:

```c
// In BulletClass::AI, after homing proximity:
if target_pos == invalid_sentinel:
    if GetHeight() >= Rules.FlightLevel (RulesClass+0x5A0):
        force detonate  // lost target, too high up
```

**(b) Chrono-warp retargeting via `BulletClass::UpdateTarget` (`0x00468430`)** —
called **only** from `TeleportLocomotionClass::StateMachineTick` (`0x007192F0`)
when the bullet's target is mid-chrono-warp. **NOT called from `BulletClass::AI`.**

```c
// BulletClass::UpdateTarget (0x00468430), sole caller TeleportLocomotionClass::StateMachineTick:
target_coords = Target->GetCoords()
if !g_MapEditorMode AND !Target->IsOnMap() AND coords != off-map-sentinel:
    Target = CellClass at target's position   // retarget to the ground cell
else:
    Target = NULL                              // no valid retarget; bullet detonates inline
```

See `BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md` §2.3 for the
full body decompile, the sentinel-check explanation, and the sole-caller analysis.

---

## 3. Proximity Detector Sub-Object

### 3.1 ProximityDetector Layout (embedded at BulletClass+0xB8)

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| +0x00 | 4 | int | CreationFrame | g_CurrentFrameCounter at init |
| +0x04 | 4 | int | (unused/padding) | Not written by Init |
| +0x08 | 4 | int | field_08 | Set to 0 |
| +0x0C | 4 | int | ArmingFrame | g_CurrentFrameCounter at init; -1 = no timer |
| +0x10 | 4 | int | (unused) | Not written by Init |
| +0x14 | 4 | int | ArmingDelay | Ticks to delay before proximity activates |
| +0x18 | 4 | int | ReferenceX | Target X coord for distance calc |
| +0x1C | 4 | int | ReferenceY | Target Y coord |
| +0x20 | 4 | int | ReferenceZ | Target Z coord |
| +0x24 | 4 | int | ClosestDistance | Minimum half-distance seen (watermark) |

### 3.2 ProximityDetector::Init (0x004E1100)

```c
void Init(ProximityDetector* this) {
    this->CreationFrame = g_CurrentFrameCounter;
    this->field_08 = 0;
    this->ArmingFrame = g_CurrentFrameCounter;
    this->ArmingDelay = 0;
    this->ReferenceX = 0;
    this->ReferenceY = 0;
    this->ReferenceZ = 0;
    this->ClosestDistance = 0;
}
```

### 3.3 ProximityDetector::Check (0x004E11F0)

```c
int Check(ProximityDetector* this, CoordStruct* current_pos) {
    int delay = this->ArmingDelay;  // offset +0x14

    if (this->ArmingFrame != -1) {  // offset +0x0C
        int elapsed = g_CurrentFrameCounter - this->ArmingFrame;
        if (delay > elapsed) {
            delay = delay - elapsed;
        } else {
            goto check_distance;
        }
    }

    if (delay != 0)
        return 0;  // still arming

check_distance:
    dx = current_pos->X - this->ReferenceX;  // +0x18
    dy = current_pos->Y - this->ReferenceY;  // +0x1C
    dz = current_pos->Z - this->ReferenceZ;  // +0x20
    dist = sqrt(dx*dx + dy*dy + dz*dz);
    half_dist = ftol(dist) / 2;

    if (half_dist < 32):
        return 1;  // VERY CLOSE: within 32 leptons (half-distance)

    if (half_dist < 256 && this->ClosestDistance < half_dist):
        return 2;  // OVERSHOT: within 256 leptons but distance increasing

    this->ClosestDistance = half_dist;  // update watermark
    return 0;  // continue flying
}
```

**Return values:**
- **0** = not close enough or still arming
- **1** = within 64 leptons of reference (32 * 2 half-distance) -- very close
- **2** = within 512 leptons (256 * 2) AND moving away -- overshot target

### 3.4 Where ProximityDetector is Used

In BulletClass::AI, after position update:
```c
// Skip proximity check if ROT <= 0 AND Ranged=no
if (BulletTypeClass.ROT < 1 && !BulletTypeClass.Ranged):
    prox_result = 0
else:
    prox_result = ProximityDetector::Check(&this->ProxDetector, &new_pos)

// Special case: if the LIVE firer's type has JumpJet (+0xD94) and prox_result == 2:
//   Override to prox_result = 1 (treat overshoot as close-hit)
```

### 3.5 Homing ground admission and final coordinate (2026-09-05)

Fresh retail body inspection corrects the ground-contact interpretation. At
`0x00466DB1..0x00466E6B`, AI admits an impact when the returned homing distance
is at most half the **double velocity vector's magnitude**, or the **old**
ObjectClass height is nonpositive. Bullet vtable `0x007E46E4 + 0x1C8` binds
`ObjectClass::GetHeight @ 0x005F5F40`; it reads the object's still-uncommitted
location, ground from `0x00578080`, and the object's own OnBridge byte.
The arm copies its cached target coordinate into the candidate only when height
is positive, Airburst is false, and that target coordinate is nonzero.
Inaccurate does not gate this first snap.

At `0x00467BF0..0x00467C06`, an admitted impact whose **committed** height is
negative invokes `+0x1CC(0)` (`0x005F5FA0`). The setter performs another ground
lookup and changes ObjectClass location; the stack coordinate passed to the
proximity detector remains unchanged. At `0x00467C3C..0x00467C66`, a live firer
at Bullet `+0xB0` supplies type `+0x84`; its JumpJet byte `+0xD94` changes
detector mode 2 to mode 1. ReadINI `0x007151EC..0x00715200` binds that byte to
the string `JumpJet` at `0x00843640`. This is the firer, not the target.

The mode-1 tail at `0x00467CA9..0x00467E4D` requires a non-null target and
neither Airburst nor Inaccurate; it finally copies target `+0x48` through
Bullet's setter `+0x1B4 @ 0x005F6940`. A CellClass aim point (`+0x58 @ 0x00486890`)
includes a structural bridge offset, but its location (`+0x48 @ 0x00486840`)
is the cell center at ground height. A retained shared dummy must be read again
after the intervening ground lookups, which may stamp its coordinate.

Active path: LogicClass `0x0055AFB0` calls object vtable `+0x5C`, which binds
Bullet AI at `0x007E4740`. Retail `[HoverMissile]` uses `[AAHeatSeeker2]`
(`ROT=60`, `Arm=2`, `Ranged=yes`, non-Inviso). Non-Inviso fresh bullets retain
the base constructor's OnBridge=false: the observed FireAt `+0x8C` propagation
at `0x006FF08B..0x006FF0B7` is gated on Inviso. Raw native save-image mutations
are outside this fresh-object proof. Stock JumpJet firers examined use either
Inviso projectiles or non-Ranged, ROT=0 Ballistic projectiles; generic source
mode conversion is implemented, but stock homing reachability of that combination
is not claimed.

Rust production delivery is `SimRuntime::advance_frame -> advance_app_frame ->
advance_master_frame -> object_ai_visit_one -> ProjectileStore::advance_one ->
commit_logic_projectile_detonations`. The runtime regression
`homing_ground_impact_reaches_damage_and_cleanup_through_runtime_frame` checks
old-height timing, impact-cell wall damage, removal, and live firer cleanup.
`homing_mode_one_fuse_snaps_to_cell_location_below_bridge_aim_point` checks the
CellClass receiver distinction. These are Rust regression tests, not native
whole-frame comparisons.

[The executable oracle](../../tools/projectile_oracle/homing_impact.py) emits
[retail-derived vectors](../../tools/projectile_oracle/homing_impact_vectors.json)
for admission, common clamp/final snap, and live-source mode conversion. It runs
the original instruction ranges and ObjectClass getter/setter bodies with
controlled external floor and target-coordinate inputs. Its comparison is bounded
to the recorded inputs; it does not prove the upstream HomingTrack velocity,
target-coordinate production, bridge-flight trajectory, null-target altitude
termination, or downstream warhead implementation. Rust still quantizes velocity
to integer components and lacks native homing pitch integration. GSI-08.07 and
GSI-08.08 remain open; the extra ballistic target-nearness admission and ordinary
ground/source collision discrepancies also remain open.

---

## 4. Detonation Logic

### 4.1 BulletClass Impact Damage (0x00468D80)

<!-- corrected 2026-05-28: was labeled "BulletClass::Detonate"; binary label at 0x00468D80 is
BulletClassBulletDetonationImpactDamage (the impact-damage sub-function, not the full Detonate entry);
verified via get_function_by_address 0x00468D80 — ROOT_CAUSE: RTTI_LABEL_DRIFT -->

**param_1 type: `int` (direct byte offsets)**

```c
void Detonate(BulletClass* this) {
    int cur_x = this->Location.X;  // 0x9C
    int cur_y = this->Location.Y;  // 0xA0
    int cur_z = this->Location.Z;  // 0xA4

    // Get target if valid and on map
    AbstractClass* target = NULL;
    if (this->Target != NULL && this->Target->IsOnMap()):
        target = this->Target

    // --- ACCURATE BULLET PATH ---
    if (!BulletTypeClass->Inaccurate):  // offset 0x2A2
        if (this->Target != NULL):
            target_pos = Target->GetCoords()
            delta = cur_pos - target_pos
            dist = sqrt(delta.X^2 + delta.Y^2 + delta.Z^2)

            if dist < 32 && !Airburst && !Inaccurate:
                // Snap to target's exact coords
                cur_pos = Target->GetCoords()

        // Apply area damage if warhead is not special type
        if (!WarheadType->IsSpecial (offset 0x154))
           && !Airburst:
            // Check if target exists and is not airborne
            if target == NULL || target->GetLayer() == 2 (ground):
                // Find objects near impact
                if Target != NULL && distance < 42:
                    Target->GetCoords() -> snap position
                    // If target is TechnoClass with turret offset:
                    if Target->WhatAmI() == 6 (Building):
                        if building has turret coords (EBC/EC0/EC4 != 0):
                            Target->ReceiveDamage()
            else:
                // Target in air: if distance < 128, apply damage
                if distance < 128:
                    target->ReceiveDamage()

    // --- CLUSTER SUB-MUNITIONS ---
    if (!Airburst):
        count = BulletTypeClass->Cluster  // offset 0x2AC
        for i in 0..count:
            WarheadTypeClass::Detonate(...)
            if !this->IsAlive: break  // bullet destroyed during detonation
            // Random scatter for next sub-munition:
            scatter = Random::RandomRanged(0x100, 0x200)  // 256-512 leptons
            detonation_pos = FUN_0049F420(scatter, 0)  // apply scatter
    else:
        // Airburst: single detonation, then spawn sub-bullets (see section 6)
        WarheadTypeClass::Detonate(...)
```

### 4.2 Detonation Trigger Summary

From BulletClass::AI, detonation is triggered when ANY of:

| Condition | Context | Address Range |
|-----------|---------|---------------|
| speed < 8.0 | Arcing bullet lost momentum | 0x004669xx |
| Z <= ground_height | Ground collision | 0x004672xx |
| Crossed bridge plane | Bridge hit | 0x004670xx |
| Building in cell (non-allied) | Building collision | 0x004673xx |
| Same cell as target + height < 2*cell_size | Target cell arrived | 0x004678xx |
| Building overlap between cells | Entered target building | 0x004678xx |
| Object within 127 leptons | Proximity fuse (non-homing) | 0x004679xx |
| Out of map bounds | FUN_00568350 returns false | 0x00467Axx |
| speed < 10.0 AND height <= 9 | Stalled near ground | 0x00467Bxx |
| vector magnitude * 0.5 >= distance | Close enough (homing); see §3.5 | 0x00466DB1..0x00466E05 |
| old ObjectClass height <= 0 | At/below ground (homing); see §3.5 | 0x00466DF6..0x00466E20 |
| Target lost + height >= FlightLevel | Lost target, too high | 0x00466Fxx |
| ApproachSum in [0, 60) for 60+ ticks | No longer closing (homing) | 0x004670xx |
| Bridge crossing (homing) | Altitude crosses bridge | 0x004671xx |
| ProximityDetector returns 1 or 2 | Embedded proximity check | 0x00467Cxx |
| Dropping=yes + HasDropped | Drop bomb behavior | 0x004668xx |

### 4.3 Collision probe and detonation admission

At `0x00467BD1`, AI calls `0x00468BB0` only while its impact flag is clear.
It commits the returned coordinate buffer through `+0x1B4` and copies the
boolean result into the impact flag. A true result proceeds through the
negative committed-height clamp (`+0x1C8 < 0 -> +0x1CC(0)`) and admits the
detonation tail at `0x00467C70`, independently of the proximity fuse. A false
result does not itself admit detonation; the fuse can still do so when
Dropping is false.

```c
bool BounceCheck(BulletClass* this, CoordStruct* impact_pos) {
    *impact_pos = this->Location;
    cell = CellClass::Get_Cell_At(impact_pos);

    // 1. Cliff/Wall collision
    if SubjectToCliffs || SubjectToWalls:
        src_cell = Get_Cell_At(this->SourceCoord)
        tgt_cell = Get_Cell_At(this->TargetCoord)
        cur_cell = MapClass::Get_CellClass(LastCell)
        firer_house = (this->Owner != 0) ? Owner->HouseClass : 0
        if FUN_004CC360(cur_cell, pos, BulletType, firer_house):
            return true  // admit impact

    // 2. Deeply underground
    if GetHeight() <= -4 * cell_size:
        return true

    // 3. FlakScatter
    if FlakScatter && Target != NULL:
        target_pos = Target->GetCoords()
        if bullet.Z < target_pos.Z && GetHeight() < 0:
            return true  // burst below target altitude

    // 4. Level
    if Level:
        if cell->BlocksPassage():  // vtable+0x50
            return true

    // 5. AA
    if AA && Target != NULL && Target->IsOnMap():
        if distance(bullet, target) < 128:
            return true

    return false  // no impact admitted by this probe
}
```


---

## 5. Air-Burst Logic

### 5.1 Airburst=yes Effect

When `Airburst=yes` (BulletTypeClass offset 0x294), the bullet behaves differently
in multiple places:

1. **Detonation**: Only calls WarheadTypeClass::Detonate ONCE (no cluster loop)
2. **Target snap disabled**: Does not snap to target's exact position on detonation
3. **Approach-rate fly-by disabled**: The approach-sum detonation trigger is skipped
4. **VeryHigh interaction**: Airburst + VeryHigh exempts from approach detection
5. **Sub-bullets spawned**: After single detonation, spawns AirburstWeapon sub-bullets

### 5.2 AirburstWeapon Spawning (in WarheadTypeClass::Detonate, 0x004690B0)

<!-- corrected 2026-05-28: was 0x00469790; binary shows entry at 0x004690B0 (body 0x004690B0–0x0046A303);
verified via get_function_by_address 0x00469790 → returns WarheadTypeClass__Detonate entry at 0x004690B0
— ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->

When `Airburst=yes`, at the END of WarheadTypeClass::Detonate:

```c
// WarheadTypeClass::Detonate entry is at 0x004690B0 (corrected 2026-05-28 from 0x00469790)
if (BulletTypeClass->Airburst):  // offset 0x294
    WeaponTypeClass* abw = BulletTypeClass->AirburstWeapon  // offset 0x2B0
    BulletTypeClass* abw_bullet = abw->Projectile     // WeaponType offset 0xA0
    WarheadTypeClass* abw_wh = abw->Warhead            // WeaponType offset 0xA4
    int abw_damage = abw->Damage                       // WeaponType offset 0xAC
    int abw_speed = abw->Speed                         // WeaponType offset 0xA8

    AbstractClass* target_cell = this->GetTargetCell()  // vtable+0x1BC

    // Spawn 8 sub-bullets in cardinal + diagonal directions
    facing_index = 0
    for i in 0..8:
        cell_at_facing = Pathfinding_update_continued(facing_index)
        // (gets adjacent cell in direction facing_index)

        // Create new BulletClass via COM
        new_bullet = CoCreateInstance(BulletClass CLSID)
        new_bullet->Init(abw_bullet, cell_at_facing, this->Owner, abw_wh, abw_damage, 50, false)

        // Set velocity: radial outward with random scatter
        random_facing = Random::RandomRanged(0, 32) << 8  // random facing
        speed_fraction = abw_speed / 10  // 1/10th of weapon speed

        angle_rad = (random_facing - 0x3FFF) * (2*PI / 65536)
        // Cone angle: sin(~75 degrees) for lateral spread
        VelX = cos(angle_rad) * sin(75deg) * speed_fraction
        VelY = sin(angle_rad) * sin(75deg) * speed_fraction
        VelZ = cos(75deg) * speed_fraction  // upward component

        new_bullet->SetCoords(this->Location)
        new_bullet->Fire(this->Location, velocity_vector)  // vtable+0x1F0

        facing_index = (facing_index + 1) & 7  // cycle through 8 directions

    // Spawn 1 additional sub-bullet aimed at the original target cell
    new_bullet = CoCreateInstance(BulletClass CLSID)
    new_bullet->Init(abw_bullet, target_cell, this->Owner, abw_wh, abw_damage, 50, false)
    // Same random velocity setup as above
    new_bullet->Fire(this->Location, velocity_vector)
```

**Total: 9 sub-bullets** — 8 radial + 1 aimed at original target.
Each sub-bullet gets:
- Speed = AirburstWeapon.Speed / 10
- Random facing in [0, 32] range (multiplied by 256 for facing units)
- Cone angle of approximately 75 degrees from horizontal
- Damage = AirburstWeapon.Damage
- Warhead = AirburstWeapon.Warhead

### 5.3 Cluster Sub-Munitions

When `Cluster > 0` (BulletTypeClass offset 0x2AC) AND `Airburst=no`:

```c
for i in 0..Cluster:
    WarheadTypeClass::Detonate(warhead, detonation_pos, ...)
    if !this->IsAlive: break

    // Scatter to random nearby position
    scatter_dist = Random::RandomRanged(256, 512)  // 0x100 to 0x200 leptons
    detonation_pos = FUN_0049F420(scatter_dist, 0)
```

Each cluster detonation applies the full warhead effect (damage, animations, etc.)
at a randomly scattered position 256-512 leptons from the impact point.

---

## 6. Shrapnel System

### 6.1 ShrapnelWeapon Spawning (BulletClass__SpawnShrapnel, 0x0046A310)

Called from WarheadTypeClass::Detonate when `BulletTypeClass->ShrapnelWeapon != NULL`
(offset 0x2B4).

```c
void SpawnShrapnel(BulletClass* this) {
    int count = BulletTypeClass->ShrapnelCount  // offset 0x2B8
    WeaponTypeClass* shrapnel_wpn = BulletTypeClass->ShrapnelWeapon  // offset 0x2B4

    // If ShrapnelCount is negative: count = distance_to_firer / 256 + |ShrapnelCount|
    if count < 0:
        if this->Owner == NULL:
            count = 3  // default
        else:
            firer_pos = Owner->GetCoords()
            distance = Distance3D(this->Location, firer_pos)
            count = -(distance / 256) - count
            if count < 1: return

    // Get the cell the bullet is over
    impact_cell = this->GetTargetCell()  // vtable+0x1BC
    if cell has building (offset 0xE4) and building is not type 6:
        // Scan nearby cells in expanding rings for enemy targets
        shrapnel_bullet_type = shrapnel_wpn->Projectile  // offset 0xA0
        search_radius = shrapnel_wpn->Range / 256  // offset 0xB4

        spawned = 0
        for ring_radius in 1..search_radius:
            for each cell_offset in ring:
                cell = Get_CellClass(impact_cell + offset)
                target_obj = cell->first_object  // offset 0xE4

                if target_obj != NULL
                   && target_obj != this->Owner
                   && this->Owner != NULL
                   && !HouseClass::Is_Ally(target_obj):

                    // Create shrapnel bullet aimed at target
                    bullet = CoCreateInstance(BulletClass)
                    bullet->Init(
                        shrapnel_bullet_type,
                        target_obj,
                        this->Owner,
                        shrapnel_wpn->Warhead,
                        shrapnel_wpn->Damage,
                        shrapnel_wpn->Speed,
                        shrapnel_wpn->Bright
                    )

                    // Compute heading to target
                    target_pos = target_obj->GetCoords()
                    heading = atan2(target_pos.X - this.X, -(target_pos.Y - this.Y))
                    // Compute velocity at weapon speed, aimed at target
                    // Apply heading and pitch to velocity
                    // Fire from bullet's location

                    bullet->Fire(this->Location, velocity)

                    spawned++
                    if spawned == count: return

        // If not enough targets found, spawn remaining as random-direction shrapnel
        remaining = count - spawned
        for i in 0..remaining:
            // Get random cell near impact
            random_cell = impact_cell + Random offsets
            bullet = CoCreateInstance(BulletClass)
            bullet->Init(shrapnel_bullet_type, random_cell, this->Owner, ...)

            // Compute heading to random cell
            // Set velocity at weapon speed
            bullet->Fire(this->Location, velocity)
```

**Key behaviors:**
- Shrapnel prioritizes nearby enemy objects in expanding ring search
- If not enough enemies found, remaining shrapnel flies in random directions
- Negative ShrapnelCount = dynamic count based on distance from firer
- Each shrapnel bullet is a full BulletClass instance with its own trajectory

---

## 7. Updated Struct Clarifications

### 7.1 Offset 0x150: RockerScale (DirectRocker force scale)

`+0x150` is the **DirectRocker force scale** in Q8.8 fixed-point (default `0x100` =
1.0×). Set to `0x100` by `BulletClass::Init` (`0x004664C0`); no other code path modifies
it. Read in `WarheadTypeClass::Detonate` (`0x004690B0`) at `0x004697FC`, inside the
DirectRocker branch (warhead+0x14F set, target non-NULL, target is not infantry):

```
force = (BulletClass.RockerScale × BulletClass.Damage) >> 8
      × Rules+0x18b4
      / global_const_at_0x0081aef8
if (force >= constant_at_0x007e3cc8): force = 4.0
target.vtbl+0x3D8(impact_pos, force)   // apply rocker physics
```

Earlier "draw priority / sort key" speculation was incorrect. See
`BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md` §1.1 for the full
write/read site analysis (assembly context included).

### 7.2 ProximityDetector Layout Corrections

The existing doc lists the ProximityDetector with 10 fields. From the decompiled
Init and Check functions, the confirmed layout is:

| ProxDetector Offset | BulletClass Offset | Field | Verified |
|---------------------|--------------------|-------|----------|
| +0x00 | 0xB8 | CreationFrame | Yes (Init writes g_CurrentFrame) |
| +0x04 | 0xBC | (not written by Init) | Uncertain |
| +0x08 | 0xC0 | field_08 = 0 | Yes |
| +0x0C | 0xC4 | ArmingFrame | Yes (Init writes g_CurrentFrame; Check reads) |
| +0x10 | 0xC8 | (not written by Init) | Uncertain |
| +0x14 | 0xCC | ArmingDelay = 0 | Yes (Check reads for timer) |
| +0x18 | 0xD0 | ReferenceX = 0 | Yes (Check reads) |
| +0x1C | 0xD4 | ReferenceY = 0 | Yes (Check reads) |
| +0x20 | 0xD8 | ReferenceZ = 0 | Yes (Check reads) |
| +0x24 | 0xDC | ClosestDistance = 0 | Yes (Check reads/writes) |

### 7.3 Velocity Zero-Guard

Both homing and straight paths have a zero-velocity guard:
```c
if VelX == 0.0 && VelY == 0.0 && VelZ == 0.0:
    VelX = 100.0  // IEEE 0x40590000
    // Prevents division by zero in normalization
```

---

## 8. Key Functions — Complete Index

| Address | Name | Purpose |
|---------|------|---------|
| 0x004666E0 | BulletClass::AI | Main per-tick update |
| 0x004664C0 | BulletClass::Init | Sets Type, Owner, Target, WH, Speed, Bright (label corrected 2026-05-28: binary shows BulletClass__Init, not PostInit; verified via decompile_function 0x004664C0 — RTTI_LABEL_DRIFT) |
| 0x00466380 | BulletClass::Constructor | Field initialization |
| 0x00468D80 | BulletClassBulletDetonationImpactDamage | Impact damage sub-function (not full Detonate entry; corrected 2026-05-28 via get_function_by_address) |
| 0x00468BB0 | BulletClass::BounceCheck | Deflection / bounce conditions |
| 0x00468430 | BulletClass::UpdateTarget | Retarget when target dies |
| 0x0046B310 | NukeMaker::SpawnDownwardNuke | Spawns nuke-down bullet (was mislabeled) |
| 0x0046A310 | BulletClass::SpawnShrapnel | ShrapnelWeapon sub-bullet spawning |
| 0x0046B960 | Velocity3D::Add | velocity += delta (3 doubles) |
| 0x005B20F0 | BulletClass::HomingTrack | Core homing turn/pitch logic |
| 0x005B2990 | Facing::IsWithinROT | Check if two facings are within ROT |
| 0x005B2950 | Facing::GetTurnDelta | Compute signed turn delta |
| 0x005B29C0 | Facing::ClampToROT | Clamp facing change to max ROT |
| 0x005B2A30 | Velocity::ApplyPitch | Apply pitch angle to velocity vector |
| 0x004E1100 | ProximityDetector::Init | Reset detector state |
| 0x004E11F0 | ProximityDetector::Check | Returns 0/1/2 proximity status |
| 0x004690B0 | WarheadTypeClass::Detonate | Full warhead detonation (calls shrapnel, airburst); corrected 2026-05-28 from 0x00469790 via get_function_by_address — GHIDRA_ADDRESS_SHIFT |

---

## 9. TS-Only Code Paths Identified

1. **Floater gravity** (FUN_0048ACF0): Used when `Floater=yes`. Only the Trident
   missile in TS uses this. No YR units have Floater=yes in standard rules. However,
   the code IS reachable if a mod sets Floater=yes — it is not dead code, just unused
   by default YR content.

2. **Dropping behavior** (BulletTypeClass offset 0x29C): The `Dropping=yes` flag
   appears in the AI flow but no standard YR BulletType uses it. It was used by TS
   paratrooper bombs. The code is live (not gated) but dormant.

3. **Scalable** (BulletTypeClass offset 0x2EC): Read during INI parsing but no
   confirmed runtime usage in BulletClass::AI was found. Likely a TS rendering flag.

4. **Level** (BulletTypeClass offset 0x29D): Used by a few specialized YR projectiles
   but the bounce-check path for Level bullets includes terrain passability checks
   that are primarily a TS mechanic.

---

## 10. Confidence Assessment

| Area | Confidence | Notes |
|------|-----------|-------|
| Struct layout (0x00-0x15F) | **High** | Verified from constructor + all access patterns |
| Arcing trajectory math | **High** | Gravity, velocity integration, bounce fully traced |
| Straight trajectory | **High** | Speed ramp and normalization confirmed |
| Homing turn logic | **High** | Full decompilation of HomingTrack + helpers |
| Wobble effect | **High** | 15-frame cos period, suppressed during course lock |
| Course lock | **High** | Both CourseLockDuration and default mode confirmed |
| Terrain avoidance | **Medium-High** | Logic confirmed but threshold constants not 100% certain |
| ProximityDetector | **High** | Init + Check fully decompiled, all fields verified |
| Airburst sub-bullet spawn | **High** | 8+1 pattern confirmed, cone angle ~75 degrees |
| Cluster detonation | **High** | Simple loop with random scatter |
| Shrapnel spawning | **Medium-High** | Ring search + random fallback confirmed; some locals ambiguous |
| Bounce reflection math | **Medium** | Matrix operations identified but float precision needs care |
| Facing unit system | **High** | 16-bit with 0x3FFF offset for pitch confirmed |
