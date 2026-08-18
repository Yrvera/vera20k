# BulletClass::AI — Per-Tick Projectile Update

**Address:** `0x004666E0` (gamemd.exe)
**Size:** ~6422 bytes, 1794 instructions, 251 cyclomatic complexity, 115 calls
**Confidence:** High (verified from binary decompilation)

## 1. Parameters

```c
void __fastcall BulletClass__AI(int *this)  // this = BulletClass*
```

Takes only `this` (ECX, __fastcall). The `int *` type means all `this[N]` offsets
are byte offset `N * 4`.

## 2. Flow Overview

Each tick, BulletClass::AI performs the following in order:

1. **Early exit** — if `!IsAlive` (offset 0x90), return immediately.
2. **Limbo cleanup** — if bullet is in limbo (offset 0x158 != 0) and its associated
   anim (offset 0x154) is null, remove from global array and call `UnInit`.
3. **Animation frame update** — tick the bullet sprite animation (AnimLow/AnimHigh/AnimRate).
4. **Save old position** — snapshot current Location (0x9C/0xA0/0xA4).
5. **Trailer spawning** — if BulletTypeClass has a Trailer anim, spawn it on schedule.
6. **Movement update** — the core logic, branching on `ROT` (Rate of Turn):
   - **ROT <= 0 (arcing/straight):** ballistic or straight-line flight
   - **ROT > 0 (homing):** guided missile with turning
7. **Cell occupancy update** — call vtable+0x124 (occupy/unoccupy map cells).
8. **Position commit** — call `SetCoords` (vtable+0x1B4) with new position.
9. **Out-of-bounds check** — if bullet left the map, force detonation.
10. **Detonation** — if the bullet should explode, call detonation logic.
11. **Degenerates** — if Degenerates=yes and speed > 5, decrement speed each tick.
12. **Store last cell** — save new cell coords to offset 0x14C.

## 3. Movement Types

The primary branch is on `BulletTypeClass.ROT` at offset 0x2DC:

### ROT <= 0: Ballistic / Straight-Line

> **Correction (verified 2026-07-18):** the two sub-cases below are NOT gated by
> `BulletTypeClass.Arcing` (offset 0x29B). A full read of `BulletClass::AI`
> (`decompile_function 0x004666E0`) shows zero references to offset 0x29B anywhere
> in the 6421-byte function body. The only branch in this path tests
> `BulletTypeClass.Vertical` (offset 0x2C0, `*(char *)(param_1[0x2b] + 0x2c0)`):
> Vertical=no takes the gravity/bounce/impact sub-path (mislabeled "Arcing" below),
> Vertical=yes takes the speed-ramped sub-path (mislabeled "Straight" below) — and
> item 6 ("If Vertical") nested inside the "Arcing" heading and the entire
> "Straight" section are two partial descriptions of that SAME Vertical=yes branch,
> not two different cases. `Arcing`'s real effect (if any — plausibly confined to
> initial trajectory setup in `BulletClass::Fire`, unverified this session) is not
> checked anywhere in `BulletClass::AI`. ROOT_CAUSE: INFERENCE_HARDENED — the
> "Arcing=yes/no" framing in the two headers below is informal legacy phrasing,
> not a binary-verified condition.

#### Vertical = no (mislabeled "Arcing" below, ROT=0)

For lobbed projectiles (e.g., tank shells, V3 warheads in ballistic phase):

1. Read current velocity vector from offsets 0xE8-0xFC (three doubles: VelX, VelY, VelZ).
2. Compute velocity magnitude via `sqrt(VelX^2 + VelY^2 + VelZ^2)`.
3. If magnitude < 8.0, flag as arrived (detonation pending).
4. Apply **gravity**: `VelZ -= Gravity` where Gravity comes from either:
   - `RulesClass + 0x16B8` (the global `[General] Gravity=` value, default 6), OR
   - `FUN_0048ACF0()` if `BulletTypeClass.Floater` (offset 0x295) is set.
5. If NOT `Vertical` (offset 0x2C0):
   - Apply `Elasticity` (offset 0x2C8, double) bounce factor.
   - Convert velocity to position delta, move the bullet.
   - Check for bridge collision: if bullet crosses a bridge plane (cell flag 0x100),
     detonate.
   - Check for ground impact: if new Z <= ground height, check for building hit.
   - On building collision with a non-allied, non-ally, non-immune building, detonate
     at impact point.
   - On ground impact without building, adjust final position to ground level and
     handle bounce or detonate.
6. If `Vertical` (e.g., V3 rocket descending) — **this is the same branch as the
   "Vertical = yes" section immediately below**, described here only for the
   height-threshold half of it; see that section for the speed-ramp half:
   - Simple linear movement, check `DetonationAltitude` (offset 0x2BC, stored at
     0x2BC as int, note: actually at byte offset 700 = 0x2BC) for height threshold.
   - Check if bullet has gone below ground: detonate.
   - Check bridge crossing: detonate.

#### Vertical = yes (mislabeled "Straight" below, ROT=0)

The other half of item 6 above — same `Vertical=yes` branch, not a third case.
Velocity is integrated directly each tick. The bullet flies in a straight line.
Speed is controlled by `Acceleration` (offset 0x2D0). The velocity vector is
normalized to the current speed each tick.

Collision checks:
- If new Z > `DetonationAltitude` (offset 0x2BC): detonate.
- If `GetHeight() < 0`: bullet went underground, detonate.
- Bridge crossing check: detonate if crossing bridge plane.

### Cell-arrival detection (ROT <= 0)

After movement, the code converts the new position and old position to cell
coordinates. If both are in the same cell AND the bullet is not Vertical AND
`GetHeight() < 2 * DAT_0089de70` (likely 2 * cell_size_leptons = 512):
**detonate immediately** — the bullet has reached its target cell.

Additionally, buildings in the destination cell are checked: if the bullet enters
a cell containing the same building it was in before (pass-through), it detonates.

### Proximity detection (ROT <= 0)

After cell checks, `CellClass::Find_Nearest_Object` is called to find any object
near the bullet's current cell. If an object is found within 127 leptons (distance
check `<= 0x7F`, i.e., detonation fires at distance <= 127 inclusive — binary gates
skip with `0x7f < iVar5`, verified via `decompile_function 0x004666E0`) and is NOT
the bullet's own target and NOT an ally, the bullet detonates on that nearby object
(proximity fuse).

If the target is found in the same cell (exact match), it detonates on target.

If no valid nearby target is found, the bullet continues to the next frame.

### Velocity clamping (ROT <= 0)

After all movement checks, the velocity magnitude is compared to 10.0
(`_DAT_007e44a8`). If speed < 10.0 AND `GetHeight() <= 9`, force detonation
(the bullet has essentially stopped and is near ground).

## 4. Tracking / Homing (ROT > 0)

When `BulletTypeClass.ROT` (offset 0x2DC) > 0, the bullet uses guided flight:

### Speed Ramping

The bullet maintains a current speed (magnitude of velocity vector) and a target
speed (`this[0x44]` = offset 0x110). Each tick:

1. Compute current speed = `sqrt(VelX^2 + VelY^2 + VelZ^2)`.
2. **CourseLockDuration** (offset 0x2E0): if nonzero, the bullet's initial heading
   is locked for this many ticks. A counter at `this[0x42]` (offset 0x108) increments
   each tick. While locked, `IsCourseLocked` (offset 0x105) stays true.
3. If no CourseLockDuration, course lock clears after the first tick where
   `target_speed <= current_speed + 0.5` (verified via `decompile_function 0x004666E0`: `(double)(int)local_1a0 <= local_130[0] + _DAT_007e1738` where `_DAT_007e1738` = 0.5), OR immediately if target_speed > 39
   (`0x27`).
4. **Acceleration**: `Acceleration` (offset 0x2D0) controls speed change per tick.
   - If current < target: `speed = min(current + accel, target)`.
   - If current > target: `speed = max(current - accel/2, 0)`.
   - If velocity is completely zero, set a default VelX=100.0 (`0x40590000`).
5. Velocity vector is normalized and scaled to the new speed magnitude.

### Homing Logic (FUN_005b20f0)

After speed is resolved, the bullet calls `FUN_005b20f0` at `0x005B20F0` — the
core homing/tracking function. Parameters:

```c
FUN_005b20f0(
    CoordStruct *out_new_position,  // output
    double *velocity,               // velocity vector (3 doubles) — modified in place
    int *rot_param,                 // ROT value shifted to facing units
    byte  airburst,                 // BulletTypeClass.Airburst (0x294)
    byte  composite,               // CONCAT11(wobble_byte, VeryHigh)
    byte  level                     // BulletTypeClass.Level (0x29D)
)
```

This function:
1. Gets the target's current position via `AbstractClass::GetCoords` (vtable+0x58).
2. If target position == invalid sentinel `{0,0,0}` (no target / target died):
   - Compute current heading from velocity vector using `atan2`.
   - Attempt to turn toward a random direction using ROT as max turn rate.
   - Apply pitch based on elevation.
3. If target is valid:
   - Compute desired heading = `atan2(target - current_pos)`.
   - Compute current heading = `atan2(velocity)`.
   - Call `FUN_005b2990` to check if desired heading is within ROT of current heading.
   - If within ROT: snap to desired heading.
   - If NOT within ROT: turn by ROT amount toward desired (via `FUN_005b2950`
     which determines turn direction — clockwise or counter-clockwise).
   - Compute desired pitch toward target.
   - If NOT Airburst AND (Inaccurate OR scatter conditions):
     - **Altitude avoidance**: checks if bullet would collide with terrain below.
       If bullet altitude is too low relative to ground, pitch up. If too high,
       pitch down. Uses half-ROT for pitch adjustments.
   - Otherwise, pitch adjusts to track target normally.
4. Output: the velocity vector is rotated to the new heading/pitch, position is
   updated by adding velocity.

### Wobble Effect

For homing bullets, a sinusoidal wobble is applied:
```
Cos_lookup((frame_counter % 15) * (2*PI/15))
```
This creates a 15-frame periodic wobble in the bullet's facing, making guided
missiles weave slightly. This wobble is suppressed when `IsCourseLocked` is active.

### Proximity Check (Homing)

After movement, the bullet checks proximity to target:
- Computes distance from new position to target position.
- If `distance <= speed * 0.5` (verified via `decompile_function 0x004666E0`: `local_160 <= fVar24 * _DAT_007e1738` where `_DAT_007e1738` = 0.5, verified via `read_memory 0x007E1730 len=24`), OR
  `GetHeight() < 1`: flag for detonation.
- If target location is the invalid sentinel AND `GetHeight() >= Rules.FlightLevel`
  (at `RulesClass + 0x5A0`): force detonation (lost target, too high up).

### Approach Detection (Homing)

The bullet maintains a running average of "approach rate" — how much closer it gets
to the target each tick:

1. `old_dist - new_dist` = approach delta this tick.
2. For the first 60 ticks (`this[0x46]` < 60, offset 0x118):
   - Accumulate delta into `ApproachSum` (offset 0x120, double).
   - Increment sample count.
3. After 60 ticks:
   - `ApproachSum = ApproachSum * 0.983 + delta` (exponential moving average,
     decay factor `_DAT_007e48e8`).
   - If `0 <= ApproachSum < 60.0` (`_DAT_007e1728`) AND NOT Airburst AND NOT VeryHigh:
     **force detonation** — bullet has stopped closing on target (fly-by / orbit detected).

### Bridge Collision (Homing)

After approach detection, if the bullet's current or previous cell has the bridge
flag (cell flags & 0x100), and the bullet's altitude crossed the bridge plane,
force detonation.

## 5. Height / Altitude / Gravity

- **Location.Z** is stored at `this[0x29]` (byte offset 0xA4), in leptons.
- **GetHeight()** (vtable+0x1C8 at `0x005F5F40`): `Location.Z - GroundHeight - (OnBridge ? BridgeHeight : 0)`.
- **Gravity** for arcing bullets comes from `Rules.Gravity` (RulesClass+0x16B8), default=6.
  Applied each tick as `VelZ -= Gravity`.
- **Floater** bullets use a different gravity from `FUN_0048ACF0()`.
- **DetonationAltitude** (BulletTypeClass offset 0x2BC): for Vertical bullets, detonation
  triggers when bullet Z exceeds this value.
- Bullets below ground (`GetHeight() < 0`) are repositioned to ground level via
  `SetHeight(0)` (vtable+0x1CC).

## 6. Proximity Detection

Two proximity systems:

### Cell-based (for ROT <= 0 bullets)
- `CellClass::Find_Nearest_Object` at the bullet's cell finds the closest object.
- If distance <= 127 leptons to a non-allied, non-same-as-target object: detonate. (binary gates skip with `0x7f < iVar5`, so detonation fires at distance <= 127 inclusive; verified via `decompile_function 0x004666E0`)
- If the object IS the target: detonate directly on it (unless Inaccurate).

### Approach detector: `FUN_004E11F0` (0x004E11F0)

Used by both ROT<=0 (if `Ranged=yes`) and ROT>0 bullets. Returns:
- **0** = not close enough / timer not expired — continue flying.
- **1** = within **64** leptons of target — very close, detonate. (binary halves distance first: `iVar4 = distance/2; if (iVar4 < 0x20)`, effective threshold = 64 leptons; verified via `decompile_function 0x004E11F0`)
- **2** = within **512** leptons AND distance is increasing — overshot, detonate. (binary: `if (iVar4 < 0x100)`, effective threshold = 512 leptons; verified via `decompile_function 0x004E11F0`)

Maintains a "closest distance" watermark at its own offset 0x24. A timer (offset
0x0C/0x14) delays activation until a configurable number of ticks after launch.

### Distance-based (for ROT > 0 bullets)
- `distance <= speed * 0.5`: detonate (verified via `decompile_function 0x004666E0`; `_DAT_007e1738` = 0.5 confirmed via `read_memory 0x007E1730 len=24`).
- Approach-rate averaging over 60+ frames detects if bullet is no longer closing.

### Ranged flag (offset 0x2A0)

When `Ranged=no` AND `ROT <= 0`, the approach detector is skipped entirely — the
bullet relies solely on cell-based proximity and ground collision.

## 7. Detonation Triggers

The bullet detonates (calls `FUN_00468D80`) when ANY of these conditions is true:

| Condition | Context |
|-----------|---------|
| Velocity magnitude < 8.0 | Arcing bullet stopped |
| Z <= ground height | Hit the ground |
| Crossed bridge plane | Bridge collision |
| Hit a building in cell | Ground impact on structure |
| Same cell as target, GetHeight < 2*cell_size | Arrived at target cell |
| Building overlap with target cell | Entered target building's cell |
| Nearby object within 127 leptons (inclusive, `<= 0x7F`) | Proximity fuse (non-homing) |
| Out of map bounds | `FUN_00568350` returns false |
| Speed < 10 AND GetHeight <= 9 | Slow + near ground (non-homing) |
| `distance <= speed * 0.5` to target | Close enough (homing) |
| GetHeight < 0 AND target is valid | Below ground (homing) |
| Target lost + height >= FlightLevel | Lost target, too high (homing) |
| ApproachSum stable near 0 for 60+ ticks | No longer closing (homing) |
| Bridge crossing (homing) | Altitude crosses bridge plane |

### Detonation Function: `FUN_00468D80` (0x00468D80)

1. If NOT Inaccurate (0x2A2):
   - Check if target object is close (< 32 leptons) — if so, detonate at target's
     exact position.
   - Check if target is alive and not airborne — spread damage to nearby objects.
2. If NOT Airburst (0x294):
   - Loop `Cluster` times (offset 0x2AC): call `WarheadTypeClass::Detonate` for each
     sub-munition, with random scatter (256-512 leptons).

### Bounce Behavior: `FUN_00468BB0` (0x00468BB0)

Called before detonation to check if the bullet should bounce instead:

1. If `SubjectToCliffs` (0x296) or `SubjectToWalls` (0x298): check for cliff/wall
   collision via `FUN_004CC360`. If hit, return true (deflect).
2. If `GetHeight() <= -4 * cell_size`: return true (deeply underground).
3. If `FlakScatter` (0x2A3) AND target exists: if bullet is BELOW target's Z, return
   true (flak burst below target).
4. If `Level` (0x29D): check if cell blocks passage, return true.
5. If `AA` (0x2A4) AND target exists AND is alive AND distance < 128: return true.

## 8. Miss / Scatter / Inaccurate

### Inaccurate (offset 0x2A2)

When `Inaccurate=yes`:
- The bullet does NOT home onto the target's exact position for detonation.
- On proximity trigger, the bullet detonates at its current position rather than
  snapping to the target.
- In `FUN_00468D80`, skip the "detonate at target coords" logic.
- The scatter is inherent in the bullet's trajectory — it aims at the original
  target cell but doesn't course-correct.

### FlakScatter (offset 0x2A3)

When `FlakScatter=yes`:
- In `FUN_00468BB0`, if the bullet is below the target's altitude AND `GetHeight() < 0`,
  force a bounce/redirect.
- This creates the "flak burst" effect where the bullet detonates near the target
  aircraft's altitude rather than hitting ground.

### Cluster (offset 0x2AC)

When `Cluster > 0`:
- The detonation function fires `WarheadTypeClass::Detonate` N times.
- Each detonation applies random scatter of 256-512 leptons via
  `Random::RandomRanged(0x100, 0x200)`.

## 9. Bounce (Bouncy)

`Bouncy` flag is at BulletTypeClass offset 0x2A7. The bounce is handled in the
arcing path:

When an arcing bullet hits the ground:
- If `FUN_00468BB0` returns true (bounce conditions met):
  - The bullet's velocity vector is reflected.
  - `Elasticity` (offset 0x2C8, double) controls how much energy is retained.
  - The bullet continues flying with reflected velocity.
- The arcing code uses rotation matrices (`VXL_GetFacingMatrix`, `FUN_005AFC20`)
  to properly reflect the velocity vector in 3D, accounting for terrain slope.

The reflection process:
1. Get the terrain normal via facing matrices.
2. Transform velocity into terrain-local space.
3. Negate the Z component (bounce off surface).
4. Scale by Elasticity.
5. Transform back to world space.
6. Continue movement with new velocity.

## 10. Trail / Trailer

### Trailer Spawning

If `BulletTypeClass.Trailer` (offset 0x2D8) is non-null (an AnimTypeClass pointer):

```
if (Trailer != null) {
    if (BulletType_field_0x2E8 != 0) {  // dead branch in standard YR
        if (g_CurrentFrameCounter % BulletType_field_0x2E8 == 0) {
            spawn AnimClass(Trailer, bullet.Location)
        }
    } else {
        if (g_CurrentFrameCounter % SpawnDelay == 0) {  // offset 0x2E4
            spawn AnimClass(Trailer, bullet.Location)
        }
    }
}
```

> **Note (verified 2026-04-24):** BulletType+0x2E8 is **never written by any
> ReadINI path**. Constructor zeros it and no INI key reaches it. The first
> branch above is therefore permanently dead; every BulletType uses the
> SpawnDelay (+0x2E4) cadence. The previously-claimed `TrailerSeperation=`
> reader at +0x2E8 belonged to a phantom function that does not exist — see
> `BULLETTYPECLASS_GHIDRA_REPORT.md` §5 for evidence.

The anim is spawned at the bullet's current position each interval. The AnimClass
is allocated with `operator_new(0x1C8)` (AnimClass size = 456 bytes) and constructed
via `AnimClass::Constructor`.

### Animation Frames (AnimLow / AnimHigh / AnimRate)

For the bullet's own sprite animation:
- `AnimLow` (offset 0x2F4, byte): first frame of animation range.
- `AnimHigh` (offset 0x2F5, byte): last frame of animation range.
- `AnimRate` (offset 0x2F6, byte): ticks per frame.
- Current frame stored at `this + 0x12C` (byte).
- Timer at `this + 0x12D` (byte): decremented each tick, on zero advances frame.
- Frame wraps from AnimHigh back to AnimLow.

## 11. Key Struct Offsets

### BulletClass Instance (param_1 is `int *`, multiply index by 4)

| Index | Byte Offset | Field | Type | Notes |
|-------|-------------|-------|------|-------|
| 0x00 | 0x00 | vtable_ptr | ptr | BulletClass vtable at 0x7E46E4 |
| 0x01 | 0x04 | vtable_secondary_4 | ptr | |
| 0x02 | 0x08 | vtable_secondary_8 | ptr | |
| 0x03 | 0x0C | vtable_secondary_12 | ptr | |
| 0x1B | 0x6C | DegradeSpeed | int | Speed counter, decremented if Degenerates=yes & > 5 |
| 0x24 | 0x90 | IsAlive | byte | False = dead, skip AI |
| — | 0x8D | HasDropped | byte | Dropping flag |
| 0x27 | 0x9C | Location.X | int | Leptons |
| 0x28 | 0xA0 | Location.Y | int | Leptons |
| 0x29 | 0xA4 | Location.Z | int | Leptons |
| 0x2B | 0xAC | pType | ptr | -> BulletTypeClass |
| 0x2C | 0xB0 | pTarget | ptr | -> AbstractClass (firing target) |
| 0x2D | 0xB4 | (unknown) | byte | |
| 0x38 | 0xE0 | (unknown flag) | byte | |
| 0x3A-0x3B | 0xE8-0xEF | Velocity.X | double | Leptons/tick |
| 0x3C-0x3D | 0xF0-0xF7 | Velocity.Y | double | Leptons/tick |
| 0x3E-0x3F | 0xF8-0xFF | Velocity.Z | double | Leptons/tick |
| 0x40 | 0x100 | (unknown) | int | |
| 0x41 | 0x104 | (unknown flag) | byte | Set to 1 in constructor |
| — | 0x105 | IsCourseLocked | byte | True while course lock active |
| 0x42 | 0x108 | CourseLockCounter | int | Ticks since launch |
| 0x43 | 0x10C | pTargetTechno | ptr | -> TechnoClass (tracked target) |
| 0x44 | 0x110 | TargetSpeed | int | Desired speed in leptons/tick |
| 0x45 | 0x114 | (unknown, init -1) | int | |
| 0x46 | 0x118 | ApproachSampleCount | int | Counts up to 60 |
| 0x48-0x49 | 0x120-0x127 | ApproachSum | double | Running approach-rate accumulator |
| 0x4A | 0x128 | (unknown) | int | |
| 0x4B | 0x12C | AnimFrame | byte | Current sprite frame |
| — | 0x12D | AnimTimer | byte | Ticks until next frame |
| 0x4C | 0x130 | (unknown) | int | |
| 0x4D | 0x134 | SourceCoord.X | int | Where bullet was fired from |
| 0x4E | 0x138 | SourceCoord.Y | int | |
| 0x4F | 0x13C | SourceCoord.Z | int | |
| 0x50 | 0x140 | TargetCoord.X | int | Original target coord |
| 0x51 | 0x144 | TargetCoord.Y | int | |
| 0x52 | 0x148 | TargetCoord.Z | int | |
| 0x53 | 0x14C | LastCellXY | short[2] | Packed cell X,Y |
| — | 0x14E | (unknown) | short[2] | |
| 0x55 | 0x154 | pBounceAnim | ptr | -> AnimClass (bounce/impact anim) |
| 0x56 | 0x158 | IsInLimbo | byte | True = waiting for anim to finish |

### BulletTypeClass Offsets (direct byte offsets from BulletTypeClass pointer)

| Offset | Field | Type | INI Key |
|--------|-------|------|---------|
| 0x294 | Airburst | bool | `Airburst` |
| 0x295 | Floater | bool | `Floater` |
| 0x296 | SubjectToCliffs | bool | `SubjectToCliffs` |
| 0x297 | SubjectToElevation | bool | `SubjectToElevation` |
| 0x298 | SubjectToWalls | bool | `SubjectToWalls` |
| 0x299 | VeryHigh | bool | `VeryHigh` |
| 0x29A | Shadow | bool | `Shadow` |
| 0x29B | Arcing | bool | `Arcing` |
| 0x29C | Dropping | bool | `Dropping` |
| 0x29D | Level | bool | `Level` |
| 0x29E | Inviso | bool | `Inviso` |
| 0x29F | Proximity | bool | `Proximity` |
| 0x2A0 | Ranged | bool | `Ranged` |
| 0x2A1 | Rotates (inverted) | bool | `Rotates` (read inverted) |
| 0x2A2 | Inaccurate | bool | `Inaccurate` |
| 0x2A3 | FlakScatter | bool | `FlakScatter` |
| 0x2A4 | AA | bool | `AA` |
| 0x2A5 | AG | bool | `AG` |
| 0x2A6 | Degenerates | bool | `Degenerates` |
| 0x2A7 | Bouncy | bool | `Bouncy` |
| 0x2A8 | AnimPalette | bool | `AnimPalette` (art) |
| 0x2A9 | FirersPalette | bool | `FirersPalette` |
| 0x2AC | Cluster | int | `Cluster` |
| 0x2B0 | AirburstWeapon | ptr | `AirburstWeapon` |
| 0x2B4 | ShrapnelWeapon | ptr | `ShrapnelWeapon` |
| 0x2B8 | ShrapnelCount | int | `ShrapnelCount` |
| 0x2BC | DetonationAltitude | int | `DetonationAltitude` |
| 0x2C0 | Vertical | bool | `Vertical` |
| 0x2C8 | Elasticity | double | `Elasticity` |
| 0x2D0 | Acceleration | int | `Acceleration` |
| 0x2D4 | Color | int | `Color` |
| 0x2D8 | Trailer | ptr | `Trailer` (AnimType, art) |
| 0x2DC | ROT | int | `ROT` |
| 0x2E0 | CourseLockDuration | int | `CourseLockDuration` |
| 0x2E4 | SpawnDelay | int | `SpawnDelay` (art) |
| 0x2E8 | (uninit by ReadINI) | int | constructor-zeroed; no INI writer; AI's "max" trailer-cadence branch is permanently dead |
| 0x2EC | Scalable | bool | `Scalable` |
| 0x2F0 | Arm | int | `Arm=` — proximity arming delay (ticks); wired in `BulletClass::Fire` → `ProximityDetector::Set` |
| 0x2F4 | AnimLow | byte | `AnimLow` (art) |
| 0x2F5 | AnimHigh | byte | `AnimHigh` (art) |
| 0x2F6 | AnimRate | byte | `AnimRate` (art) |
| 0x2F7 | Flat | bool | `Flat=` (art) — flat-to-ground render flag |

## 12. Functions Called

| Address | Name / Purpose | Called When |
|---------|---------------|-------------|
| 0x005F3E70 | Frame counter / tick guard | Every tick (entry) |
| 0x00421EA0 | `AnimClass::Constructor` | Spawning trailer anims |
| 0x005F5F40 | `ObjectClass::GetHeight` | vtable+0x1C8, height checks |
| 0x005F5FA0 | `ObjectClass::SetHeight` | vtable+0x1CC, height adjustment |
| 0x005F6940 | `ObjectClass::SetCoords` | vtable+0x1B4, position update |
| 0x005F65A0 | `ObjectClass::GetCoords` | vtable+0x48, read position |
| 0x005F65F0 | `ObjectClass::UnInit` | vtable+0xF8, destroy bullet |
| 0x004666C0 | `BulletClass::MarkOccupancy` | vtable+0x124, cell occupancy |
| 0x005B20F0 | **Homing tracking logic** | ROT > 0 path |
| 0x005B2990 | Check if facing within ROT | Inside homing logic |
| 0x005B2950 | Get turn direction (CW/CCW) | Inside homing logic |
| 0x00468D80 | **Detonation** | When bullet should explode |
| 0x00468BB0 | **Bounce check** | Before detonation — can deflect |
| 0x004E11F0 | **Proximity/approach detector** | Both movement paths |
| 0x00568350 | **Map bounds check** | After movement |
| 0x0047C3D0 | `CellClass::Find_Nearest_Object` | Proximity fuse (non-homing) |
| 0x0047C520 | `Look_up_building_in_cell` | Building collision |
| 0x00578080 | `CellClass::GetGroundHeight` | Ground level queries |
| 0x00565730 | `CellClass::Get_Cell_At` | Coord -> CellClass conversion |
| 0x005657A0 | `MapClass::Get_CellClass` | Cell coord -> CellClass |
| 0x004F9A90 | `HouseClass::Is_Ally` | Alliance checks |
| 0x0041C230 | `CoordStruct::Set` | Coord struct construction |
| 0x004CAC40 | `Math::sqrt` | Distance calculations |
| 0x004CACB0 | `Cos_lookup` | Wobble effect |
| 0x007C5F00 | `Math::ftol` | Float-to-int conversion |
| 0x0048ACF0 | Floater gravity override | Arcing+Floater bullets |
| 0x0048ACE0 | (related helper) | Ground height helper |
| 0x00410A40 | Check game mode / condition | Before radar event |
| 0x0053AB70 | (unknown, pre-detonation) | Before impact anim |
| 0x0065FA70 | `CreateRadarEvent` | Impact radar blip |
| 0x00427CB0 | `FindAnimType` | Find impact animation |
| 0x007559B0 | `VXL_GetFacingMatrix` | Bounce reflection |
| 0x005AFC20 | (matrix helper) | Bounce reflection |
| 0x005AF4D0 | (matrix multiply) | Bounce reflection |
| 0x0043A0B0 | (rotation setup) | Bounce calculation |
| 0x0043A0D0 | (rotation setup) | Bounce calculation |
| 0x00437090 | (coord helper) | Arcing path |
| 0x006D6AD0 | (unknown, arcing) | Arcing path ground impact |
| 0x004CC360 | (cliff/wall collision) | Bounce check |
| 0x005F6360 | (distance/range check) | Detonation proximity |
| 0x0046B960 | (bullet helper) | Arcing path |
| 0x00480510 | (cell/terrain check) | Ground impact detection |
| 0x007C8E17 | `operator_new` | Allocating AnimClass |

## Key Constants

| Address | Value | Usage |
|---------|-------|-------|
| 0x007E48E0 | 8.0 | Minimum velocity for arcing (below = arrived) |
| 0x007E44A8 | 10.0 | Minimum speed for straight flight |
| 0x007E1730 | 90.0 | **Corrected 2026-07-18: NOT used by `BulletClass::AI`.** `get_xrefs_to 0x007E1730` shows 100+ read sites spread across unrelated functions (0x00403D96–0x005544E6); none fall inside `BulletClass::AI`'s body (0x004666E0–0x00467FF5). This is a generic shared double literal, not an approach-sum constant. ROOT_CAUSE: INFERENCE_HARDENED (prior patch added this row from a neighbor-memory dump without checking whether the function reads it). The real approach-sum threshold is the next row. |
| 0x007E1738 | 0.5 | Homing proximity multiplier (`distance <= speed * 0.5`) and course-lock clear threshold — verified via `read_memory 0x007E1730 len=24` |
| 0x007E1728 | 60.0 | Approach-sum threshold for fly-by detection |
| 0x007E48E8 | 0.983 | Exponential decay for approach averaging |
| 0x007E48F8 | 1/15 | Wobble period divisor (2*PI/15 per frame) |
| 0x007E3CC0 | 2*PI | Full circle for wobble calculation |
| 0x007E2808 | 128.0 | Max scatter distance for homing |
| 0x007E48D8 | 150.0 | Ground hit tolerance (arcing) |
| RulesClass+0x16B8 | Gravity (default 6) | Gravity constant for arcing |
| RulesClass+0x5A0 | FlightLevel | Max altitude for lost-target detonation |

## Summary Flow Diagram

```
BulletClass::AI(this)
│
├─ if !IsAlive → return
├─ if InLimbo → cleanup & UnInit → return
│
├─ Update animation frame (AnimLow/High/Rate)
├─ Save old position
├─ Spawn Trailer anim if due
│
├─ if ROT <= 0 (ballistic/straight):
│   ├─ Compute velocity magnitude
│   ├─ if Arcing:
│   │   ├─ Apply gravity: VelZ -= Gravity
│   │   ├─ if Vertical: check DetonationAltitude, move linearly
│   │   └─ else: integrate position, check ground/bridge/building
│   ├─ else (straight):
│   │   └─ Integrate position, check altitude/ground/bridge
│   │
│   ├─ Check same-cell-as-target → detonate
│   ├─ Check building overlap → detonate
│   ├─ Find nearest object (proximity fuse) → detonate
│   ├─ Check map bounds → detonate if OOB
│   └─ Check velocity < 10 near ground → detonate
│
├─ if ROT > 0 (homing):
│   ├─ Speed ramp (Acceleration)
│   ├─ Course lock check (CourseLockDuration)
│   ├─ Get target position
│   ├─ Wobble effect (cos, 15-frame period)
│   ├─ Call FUN_005b20f0 (homing turn logic)
│   ├─ Check distance <= speed*0.5 → detonate
│   ├─ Check target lost + too high → detonate
│   ├─ Approach-rate averaging → detonate if not closing
│   └─ Bridge collision → detonate
│
├─ Occupy new cell (vtable+0x124)
├─ Set new position (vtable+0x1B4)
│
├─ if out of bounds → force detonate
│
├─ if should_detonate:
│   ├─ Call FUN_00468BB0 (bounce check)
│   ├─ if bounce: set new position, continue
│   ├─ if GetHeight < 0: SetHeight(0)
│   ├─ Check target retarget (homing, non-Airburst)
│   ├─ Create radar event
│   ├─ Find impact anim → create AnimClass, enter limbo
│   └─ OR call FUN_00468D80 (detonate) + UnInit
│
├─ if Degenerates && speed > 5: speed--
└─ Store last cell coords
```
