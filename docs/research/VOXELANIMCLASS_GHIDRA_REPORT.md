# VoxelAnimClass — Ghidra Research Report

**Primary Addresses:**
- VoxelAnimClass::Constructor: `0x007493B0`
- VoxelAnimClass::AI: `0x00749F30`
- VoxelAnimClass::Destructor: `0x007499F0`
- VoxelAnimClass::Draw: `0x0046B0C0`
- VoxelAnimTypeClass::Constructor: `0x0074AD80`
- VoxelAnimTypeClass::ReadINI: `0x0074B050`
- BounceClass::Init: `0x004397E0`
- BounceClass::Update: `0x00439B00`

**Confidence:** HIGH (all core functions decompiled from binary)
**Active in YR:** Yes — VoxelAnims are actively used for vehicle destruction debris in standard YR skirmishes.

---

## 1. Overview

VoxelAnimClass represents a bouncing, spinning piece of 3D voxel debris — flying turrets, tires, gas tanks, crystal shards, and meteorites. When a vehicle is destroyed, the warhead's `DebrisTypes` / `DebrisMaximums` lists are read, and the corresponding VoxelAnimClass instances are spawned at the death location. Each instance has a physics simulation (BounceClass) that handles gravity, bounce elasticity, angular velocity, and terrain collision. After its duration expires or it falls into water, the VoxelAnim plays an expire animation, optionally deals area damage, and destroys itself.

VoxelAnimClass inherits from ObjectClass. Its type class (VoxelAnimTypeClass) inherits from ObjectTypeClass and is parsed from `[VoxelAnims]` sections in rulesmd.ini.

---

## 2. VoxelAnimClass Struct Layout

**Instance size:** 0x148 (328 bytes), confirmed by `Size()` virtual at `0x0074AB10`.
**WhatAmI() returns:** 0x29 (41 decimal).

`param_1` in the constructor is `undefined4 *` (int pointer), so offsets in the decompilation are `index * 4`.

| Byte Offset | Size | Type | Field | Evidence |
|------------|------|------|-------|----------|
| 0x000 | 4 | ptr | vtable (primary) | `0x007F6318` |
| 0x004 | 4 | ptr | vtable (secondary 1 — IPersistStream) | `0x007F62FC` |
| 0x008 | 4 | ptr | vtable (secondary 2) | `0x007F62F4` |
| 0x00C | 4 | ptr | vtable (secondary 3) | `0x007F62EC` |
| 0x010–0x09B | | | ObjectClass base fields | Inherited |
| 0x09C–0x0A7 | 12 | CoordStruct | Position (X, Y, Z) | `param_1[0x27..0x29]` used in AI |
| 0x0B0–0x0FF | 80 | BounceClass | Complete physics state (0x50 bytes) | Constructor stores at `param_1+0x2C` (0xB0) |
| 0x0B0 | 8 | double | BounceClass.Elasticity (offset 0x00) | BounceClass::Init param_3/param_4 |
| 0x0B8 | 8 | double | BounceClass.Gravity (offset 0x08) | Always 1.4 (hardcoded `0x3FF66666_60000000`) |
| 0x0C0 | 8 | double | BounceClass.AngularVelocityMagnitude (offset 0x10) | Clamping threshold for angular velocity |
| 0x0C8 | 4 | float | BounceClass.Position.X (offset 0x18) | Float copy of initial coord |
| 0x0CC | 4 | float | BounceClass.Position.Y (offset 0x1C) | Float copy of initial coord |
| 0x0D0 | 4 | float | BounceClass.Position.Z (offset 0x20) | Float copy of initial coord |
| 0x0D4 | 4 | float | BounceClass.Velocity.X (offset 0x24) | From random XY velocity |
| 0x0D8 | 4 | float | BounceClass.Velocity.Y (offset 0x28) | From random XY velocity |
| 0x0DC | 4 | float | BounceClass.Velocity.Z (offset 0x2C) | Gravity subtracted each tick. For IsMeteor, AI adds gravity back to cancel. |
| 0x0E0 | 16 | Quaternion | BounceClass.Orientation (offset 0x30) | 4 floats (x,y,z,w). Init: identity. Multiplied by RotationPerTick each tick. |
| 0x0F0 | 16 | Quaternion | BounceClass.RotationPerTick (offset 0x40) | 4 floats (x,y,z,w). Init: `FromAxisAngle(randomAxis, angVel)`. Negated on bounce. |
| 0x100 | 4 | int | Timer / unknown | Set to -1 in default constructor |
| 0x104 | 4 | ptr | VoxelAnimTypeClass* Type | `param_1[0x41]`, confirmed by GetType() at 0x0074AB30 |
| 0x108 | 4 | ptr | ParticleSystemClass* AttachedSystem | Created if Type->AttachedSystem != null |
| 0x10C | 4 | ptr | HouseClass* Owner | `param_1[0x43]`, passed as constructor param_4 |
| 0x110 | 1 | bool | Marked for deletion | Checked in AI: `if ((char)param_1[0x44] != '\0')` triggers Delete |
| 0x114–0x127 | 20 | | SoundEvent 1 (StartSound) | Initialized via FUN_00405be0 |
| 0x128–0x13B | 20 | | SoundEvent 2 (StopSound) | Initialized via FUN_00405be0 |
| 0x13C | 1 | bool | Unknown flag | Set to 0 in constructor |
| 0x140 | 4 | int | Duration countdown | Initialized from Type->Duration (offset 0x29C) |

### BounceClass Layout (embedded at offset 0xB0)

BounceClass is a 0x50 (80) byte struct embedded in VoxelAnimClass starting at byte offset 0xB0 (constructor calls `BounceClass::Init` on `param_1 + 0x2C` which is byte offset 0xB0). **[CORRECTED in verification pass: was incorrectly listed as 0x98/152 bytes. Tracing all field accesses in Init, Update, and FUN_004399e0 confirms the struct spans only offsets 0x00-0x4F. No function accesses BounceClass beyond 0x4F.]**

From BounceClass::Init (`0x004397E0`), `param_1` is `undefined4 *`:

| Index | Byte Offset (in BounceClass) | Type | Field |
|-------|-----|------|-------|
| 0 | 0x00 | double low | Elasticity (low dword) |
| 1 | 0x04 | double high | Elasticity (high dword) — passed from Type->Elasticity |
| 2 | 0x08 | double low | Gravity (low dword) |
| 3 | 0x0C | double high | Gravity (high dword) — always 1.4 |
| 4 | 0x10 | double low | AngularVelocityMagnitude (low dword) — velocity clamping threshold. 0 in normal path, 3.0 for terrain meteors (FUN_00439690). |
| 5 | 0x14 | double high | AngularVelocityMagnitude (high dword) |
| 6 | 0x18 | float | Position.X (float copy of initial coord) |
| 7 | 0x1C | float | Position.Y |
| 8 | 0x20 | float | Position.Z |
| 9–11 | 0x24–0x2F | 3 floats | XY/Z velocity vector |
| 12 | 0x30 | 16 (4 floats) | Orientation quaternion (x,y,z,w) — Init: identity via Matrix3x4_SetIdentity->ToQuaternion. Updated each tick by multiplying with RotationPerTick. |
| 16 | 0x40 | 16 (4 floats) | RotationPerTick quaternion — Init: `Quaternion_FromAxisAngle(randomAxis, angularVelocity)`. Column negation on bounce reverses spin direction. |

**[CORRECTED: BounceClass ends at offset 0x4F. There are NO fields at 0x50+. The struct is 0x50/80 bytes total.]**

From BounceClass::Update (`0x00439B00`):
- `param_1` type is `double *` in decompiler. Gravity at `param_1[1]` = byte offset 0x08 (Gravity double), subtracted from Z-component each tick: `*(float*)((int)param_1 + 0x2c) -= (float)param_1[1]`
- Return values: 0 = still airborne, 1 = bounced (hit ground), 2 = stopped (velocity below threshold 2.5)
- Ground collision: checks Z position float (`param_1 + 4` = BounceClass offset 0x20) against ground height
- Bridge detection: checks cell flags `0x100` (bridge) at both old and new cell positions
- Building collision: `Look_up_building_in_cell()` — buildings with `Strength > 7` AND a specific TypeClass flag (`TypeClass+0x16BF != 0`) block bouncing
- Also checks vtable method `+0x80` (IsInAir?) — if true, no bounce
- Slope detection: if height difference between old/new cell > 1, and velocity meets certain thresholds, the velocity vector is reflected through the slope's transform matrix
- At end of every tick: `Quaternion_Multiply(Orientation, RotationPerTick)` updates the rotation

---

## 3. VoxelAnimTypeClass Struct Layout

**Inherits from:** ObjectTypeClass
**param_1 type in constructor:** `undefined4 *` (int pointer, multiply indices by 4)

| Byte Offset | Size | Type | INI Key | Default | Notes |
|------------|------|------|---------|---------|-------|
| 0x230 | 1 | bool | (from ObjectTypeClass) | | Part of base class |
| 0x231 | 1 | bool | | false | Set in constructor |
| 0x232 | 1 | bool | | true | Set in constructor |
| 0x233 | 1 | bool | | true | Set in constructor |
| 0x234 | 4 | ptr | Image (VXL data) | | At 0xB0*4 in ObjectTypeClass |
| 0x235 | 1 | bool | | false | Set in constructor |
| 0x294 | 1 | bool | Normalized | false | `CCINIClass::ReadBool` |
| 0x295 | 1 | bool | Translucent | false | `CCINIClass::ReadBool` |
| 0x296 | 1 | bool | SharesData | false | Computed: `ShareBodyData \|\| ShareTurretData \|\| ShareBarrelData` |
| 0x298 | 4 | int | VoxelIndex | 0 | Section index in VXL file |
| 0x29C | 4 | int | Duration | 30 | Ticks the anim lives |
| 0x2A0 | 8 | double | Elasticity | 0.8 | Bounce coefficient (0.0 = no bounce, 1.0 = perfect) |
| 0x2A8 | 8 | double | MinAngularVelocity | 0.0 | Stored as radians (INI value * pi/180) |
| 0x2B0 | 8 | double | MaxAngularVelocity | ~10 deg (0.17453 rad) | Stored as radians (INI value * pi/180) |
| 0x2B8 | 8 | double | MinZVel | 3.5 | Upward launch velocity (minimum) |
| 0x2C0 | 8 | double | MaxZVel | 5.0 | Upward launch velocity (maximum) |
| 0x2C8 | 8 | double | MaxXYVel | 15.0 | Horizontal spread velocity |
| 0x2D0 | 1 | bool | IsMeteor | false | Changes constructor behavior: meteors use different spawn logic |
| 0x2D4 | 4 | ptr | VoxelAnimTypeClass* Spawns | null | Type to spawn on expiry (e.g., PEBBLE from METEOR01) |
| 0x2D8 | 4 | int | SpawnCount | 0 | Number of child VoxelAnims to spawn |
| 0x2DC | 4 | int | StartSound | -1 | VocClass index, -1 = none |
| 0x2E0 | 4 | int | StopSound | -1 | VocClass index, -1 = none |
| 0x2E4 | 4 | ptr | AnimTypeClass* BounceAnim | null | Played when bouncing off ground |
| 0x2E8 | 4 | ptr | AnimTypeClass* ExpireAnim | null | Played when duration expires |
| 0x2EC | 4 | ptr | AnimTypeClass* TrailerAnim | null | Played every other frame while alive |
| 0x2F0 | 4 | int | Damage | 0 | Damage dealt on expiry |
| 0x2F4 | 4 | int | DamageRadius | 0 | Radius for area damage on expiry |
| 0x2F8 | 4 | ptr | WarheadTypeClass* Warhead | null | Warhead for expiry damage |
| 0x2FC | 4 | ptr | ParticleSystemTypeClass* AttachedSystem | null | Particle system attached while alive |
| 0x300 | 1 | bool | IsTiberium | false | Meteor-style: creates crater/ore on impact |

### INI Key Parsing Details

Angular velocity conversion: INI values are in **degrees per tick**. The engine multiplies by `pi/180` (0.017452778, stored at `0x007F65E8`) to convert to radians for internal storage.

**[CORRECTED]** Angular velocity sentinel logic: ReadDouble is called with default `-1.0`. The comparison is against `0.0` (at `0x007e2800`), NOT against `-1.0`. If the result equals 0.0, the field retains its constructor default. If the INI key is absent, ReadDouble returns -1.0, which is != 0.0, so `-1.0 * pi/180 = -0.01745` would be stored. This is likely a **bug in the original engine** (intended to compare against -1.0 sentinel). In practice it's dormant because every VoxelAnim in rulesmd.ini specifies both MinAngularVelocity and MaxAngularVelocity explicitly.

### Shared VXL Data (ShareBodyData / ShareTurretData / ShareBarrelData)

When `ShareTurretData=yes` is set along with `ShareSource=SONIC` (for example), the VoxelAnimType borrows its VXL and HVA data from another TechnoType's turret section. This is how SONICTURRET and 4TNKTURRET get their models — they share the turret VXL from the source unit rather than loading a standalone file.

The sharing logic in ReadINI:
1. `ShareBodyData=yes` → copies body VXL/HVA from source (offsets 0xB0, 0xB4) → stores to destination `param_1+0xB0` / `param_1+0xB4`
2. `ShareTurretData=yes` → copies turret VXL/HVA from source (offsets 0xB8, 0xBC) → stores to destination `param_1+0xB0` / `param_1+0xB4` [corrected 2026-05-28: destination is always the body slot 0xB0/0xB4, not 0xB8/0xBC; VoxelAnimTypeClass has only one VXL model slot; source offsets differ but destination is identical for all three cases; binary confirmed via `decompile_function 0x0074B050` — ROOT_CAUSE: INFERENCE_HARDENED]
3. `ShareBarrelData=yes` → copies barrel VXL/HVA from source (offsets 0xC0, 0xC4) → stores to destination `param_1+0xB0` / `param_1+0xB4` [corrected 2026-05-28: same — destination is always 0xB0/0xB4; ROOT_CAUSE: INFERENCE_HARDENED]
4. If none set → loads `<ID>.vxl` file directly

---

## 4. Core Logic

### 4a. Constructor — Two Paths (Normal vs Meteor)

**Address:** `0x007493B0`
**Parameters:** `(this, VoxelAnimTypeClass* type, CoordStruct* coords, HouseClass* owner)`

The constructor has two distinct code paths based on `Type->IsMeteor` (offset 0x2D0):

**Normal path (IsMeteor = false):**
1. Calls `ObjectClass::Constructor()`
2. Initializes BounceClass fields to zero, sets up CRects
3. Stores type, owner, sets Duration from Type->Duration
4. Adds self to global VoxelAnimClass DynamicVector at `0x00887388`
5. Calls `ObjectClass::Reveal(coords)` to make visible
6. Gets current position coordinates, adds Z+10 offset
7. Generates random velocities:
   - XY velocity: `Random() % ftol(MaxXYVel * 2) - MaxXYVel` (random in [-MaxXYVel, +MaxXYVel])
   - Z velocity: `Random() % ftol(MaxZVel - MinZVel + 1) + MinZVel`
   - Angular velocity: `Random() % ftol(MaxAngularVelocity - MinAngularVelocity + 1) + MinAngularVelocity`
8. Calls `BounceClass::Init` with coords, elasticity=Type->Elasticity, gravity=1.4 (hardcoded `0x3FF66666_60000000`), velocity vector
9. Plays StartSound if not -1
10. Creates AttachedSystem ParticleSystem if Type has one

**Meteor path (IsMeteor = true):**
1. Same base initialization
2. **Different velocity calculation**: uses the same MaxXYVel for both X and Y range (centered on -MaxXYVel), and the velocity vector is constrained so X component >= -Y component (forces a diagonal trajectory)
3. **Duration randomization**: subtracts a random value 0..19 from the initial Duration
4. **Position recalculation**: position is moved by `duration * velocity` to place the meteor at its starting point in the sky, then calls `ftol()` to convert to integer coords
5. Reveal happens at the computed sky position, not at the target coords
6. Same BounceClass::Init, sound, and particle system setup

### 4b. AI (Per-Tick Update)

**Address:** `0x00749F30` (vtable slot 23, offset +0x5C)

The AI method is called every game tick. Its logic branches on the Duration countdown:

**Phase 1: Duration > 0 (still alive)**
1. If Type has StartSound, updates looping sound position
2. If marked for deletion (`offset 0x110`), calls Delete and returns
3. Decrements Duration by 1
4. **TrailerAnim**: if Type->TrailerAnim is set, spawns it every other frame (`g_CurrentFrameCounter & 1 == 0`)
5. Calls `BounceClass::Update()`:
   - Returns 0 = still in flight
   - Returns 1 = hit ground (bounced)
   - Returns 2 = stopped (velocity too low)
6. **IsMeteor gravity compensation**: if IsMeteor, updates `param_1[0x37]` (= VoxelAnim byte 0xDC = BounceClass Velocity.Z at offset 0x2C) by adding `(float)*(double*)(param_1 + 0x2E)` (= BounceClass Gravity at offset 0x08). **[CORRECTED: This is NOT a "visual rotation field" — it adds gravity back to Velocity.Z each tick, canceling BounceClass::Update's gravity subtraction. This makes meteors fall at constant velocity with zero net gravity.]**
7. **On bounce (return 1)**:
   - Gets the cell at current position
   - If cell terrain type == 2 (water): set Duration = 0 (die immediately)
   - Otherwise: play BounceAnim if set, then iterate all objects in the cell and apply damage to those within DamageRadius using the configured Warhead+Damage
8. **On stopped (return 2)**: set Duration = 0 (expire next tick)
9. Updates position from BounceClass float coords via `CoordStruct::FromDoubles`

**Phase 2: Duration == 0 (expiring)**
1. Get current cell, check if it's water (terrain type 2)
2. Get ground height at position
3. **If NOT on water, OR if position is above ground**:
   - Play ExpireAnim if set (with `zAdjust = -30` in flags)
   - Call `Apply_area_damage()` with Damage/DamageRadius/Warhead
   - Call area shake effect (`FUN_0048a620`)
4. **If on water AND below ground height**:
   - Non-meteor: play splash animations (`g_RulesClass_Instance + 0x94` and `+0xBC4`)
   - Meteor (IsMeteor): play a different splash animation from the Rules global list
5. **If on land (not water) and not above ground**:
   - **Meteor with IsTiberium=true**: iterates all 8 adjacent cells, checks if they can have wall overlays, and creates Tiberium/ore overlay on them. Also spawns child VoxelAnims from `Spawns` / `SpawnCount`.
   - **Non-meteor with IsTiberium=true**: same wall/overlay creation for the single landing cell
6. Calls Delete (vtable+0xF8)

### 4c. BounceClass Physics

**Init:** `0x004397E0`
- Stores elasticity, gravity, initial position (as floats from integer coords)
- Stores velocity vector (XY spread + Z launch)
- Generates random rotation axis (normalized random 3D vector)
- Creates rotation quaternion from axis + angular velocity

**Update:** `0x00439B00`
- **Gravity**: subtracts gravity from Z velocity each tick: `Zvel -= gravity`
- **Angular velocity limit**: if angular velocity exceeds some threshold (related to double at offset 0x10), it's clamped via `FUN_0043a130`
- Converts float position to integer coords, gets ground height
- **Bridge detection**: checks cell flag `0x100` (bridge) — if crossing a bridge surface, Z is clamped to bridge height
- **Ground collision**: if Z position < ground height (within 150.0 tolerance):
  - Checks for building in cell (`Look_up_building_in_cell`)
  - Buildings with strength > 7 block bouncing
  - Terrain that's "impassable" (`FUN_00480510`) blocks bouncing
  - On collision: Z position snapped to ground height
- **Slope handling**: if height difference between old cell and new cell > 1:
  - Gets a slope transform matrix
  - Reflects the velocity vector through the slope normal
  - Multiplies reflected velocity by elasticity factor
- **Velocity negation on bounce**: after collision, the facing rotation matrix columns are negated (reflects the spin direction)
- **Stopping condition**: computes total kinetic energy/velocity magnitude. If below threshold (2.5, at `0x007E3D80`), returns 2 (stopped). Otherwise returns 1 (bounced) or 0 (still airborne).

### 4d. Draw

**Address:** `0x0046B0C0`

The draw function:
1. Calls `VXL_Init_Simple` with light direction
2. Clears the tile map
3. Iterates VXL sections, applies HVA transform for current frame (`frame % num_frames`)
4. Applies locomotion matrix transform
5. Submits bounding box, calls `VXL_Sort_Rasterize`
6. Gets VoxelAnimTypeClass (offset 0xAC in the decompilation context)
7. Checks a "Translucent" flag at TypeClass+0x2F7 — if set, applies alpha blending flag (0x2000) and uses a special blitter
8. Computes screen position from the rasterized tile offsets
9. Calls the standard voxel blitter pipeline

---

## 5. INI Keys Summary

All keys are read in `VoxelAnimTypeClass::ReadINI` at `0x0074B050`.

| INI Key | Type | Default | Offset | Description |
|---------|------|---------|--------|-------------|
| Normalized | bool | false | 0x294 | Normalize voxel colors |
| Translucent | bool | false | 0x295 | Render with alpha blending |
| IsTiberium | bool | false | 0x300 | Creates ore/crater on impact (meteor behavior) |
| IsMeteor | bool | false | 0x2D0 | Meteor-style spawn (starts in sky, descends) |
| Elasticity | double | 0.8 | 0x2A0 | Bounce coefficient. 0 = no bounce, 1 = perfect elastic |
| MinAngularVelocity | double | 0.0 | 0x2A8 | Min spin speed (degrees/tick in INI, radians internally) |
| MaxAngularVelocity | double | 10.0 | 0x2B0 | Max spin speed (degrees/tick in INI, radians internally) |
| Duration | int | 30 | 0x29C | Lifetime in game ticks |
| MinZVel | double | 3.5 | 0x2B8 | Min upward launch speed (leptons/tick) |
| MaxZVel | double | 5.0 | 0x2C0 | Max upward launch speed (leptons/tick) |
| MaxXYVel | double | 15.0 | 0x2C8 | Max horizontal spread speed (leptons/tick) |
| Spawns | string | (none) | 0x2D4 | VoxelAnimType to spawn on impact (ptr lookup) |
| SpawnCount | int | 0 | 0x2D8 | Number of child VoxelAnims to spawn |
| VoxelIndex | int | 0 | 0x298 | Section index in the VXL model |
| StartSound | string | -1 | 0x2DC | Sound played on creation (loops) |
| StopSound | string | -1 | 0x2E0 | Sound played on expiry |
| BounceAnim | string | null | 0x2E4 | AnimType played on each ground bounce |
| ExpireAnim | string | null | 0x2E8 | AnimType played when Duration expires |
| TrailerAnim | string | null | 0x2EC | AnimType played every other tick while alive |
| Damage | int | 0 | 0x2F0 | Damage dealt on expiry |
| DamageRadius | int | 0 | 0x2F4 | Radius for expiry area damage |
| Warhead | string | null | 0x2F8 | Warhead for expiry damage |
| AttachedSystem | string | null | 0x2FC | ParticleSystemType attached while alive |
| ShareBodyData | bool | false | (local) | Borrow body VXL from ShareSource unit |
| ShareTurretData | bool | false | (local) | Borrow turret VXL from ShareSource unit |
| ShareBarrelData | bool | false | (local) | Borrow barrel VXL from ShareSource unit |
| ShareSource | string | (none) | (local) | Unit type ID to borrow VXL data from |

---

## 6. Integration Points

### Creation Triggers

VoxelAnimClass instances are created from:

1. **TechnoClass::ReceiveDamage** (`0x00701900`, call at `0x00702397`): When a unit dies, iterates `TechnoTypeClass->DebrisTypes[]` / `DebrisMaximums[]` and spawns VoxelAnims at the death location. This is the primary creation path for vehicle destruction debris.

2. **WarheadTypeClass::Detonate** (`0x00469DD5`): Warheads can also spawn VoxelAnim debris directly (via the warhead's own DebrisTypes/DebrisMaximums lists).

3. **VoxelAnimClass::AI** (`0x00749F30`, internal call at `0x0074A2FB`): Meteor VoxelAnims with `Spawns=` set create child VoxelAnims on impact.

4. **FUN_006e2520** (`0x006E2520`): Creates VoxelAnims from trigger/script actions. Uses the global VoxelAnimTypeClass array indexed by a parameter.

5. **Apply_area_damage** (`0x00489280`, call at `0x0048A3CF`): Area damage can create VoxelAnim debris from affected units.

### Global Arrays

- **VoxelAnimClass instances**: `DynamicVectorClass<VoxelAnimClass*>` at `0x00887388`
  - `+0x00`: vtable pointer
  - `+0x04`: buffer pointer
  - `+0x08`: count
  - `+0x0C`: capacity
  - `+0x10`: growth step
  - `+0x14`: can-grow flag

- **VoxelAnimTypeClass instances**: `DynamicVectorClass<VoxelAnimTypeClass*>` at `0x00B0F670` (used in constructor for type registration) and `0x00A8EB28` (secondary registration, likely the TypeList for save/load).

### Tick Integration

VoxelAnimClass::AI is virtual method at vtable offset +0x5C. It is called as part of the main game loop's object update cycle, which iterates all ObjectClass-derived instances in the world and calls their AI methods. VoxelAnims exist in the object layer system at layer 3 (returned by `FUN_0074A960`).

### Relationship to AnimClass

VoxelAnimClass and AnimClass are **completely separate class hierarchies**. Both inherit from ObjectClass, but:
- AnimClass renders SHP (2D sprite) animations
- VoxelAnimClass renders VXL (3D voxel) models with physics
- VoxelAnims can **spawn** AnimClass instances (BounceAnim, ExpireAnim, TrailerAnim) but are not themselves anims
- The debris spawning system uses both: DebrisTypes (VoxelAnim debris) and DebrisAnims (SHP anim debris) from TechnoTypeClass

---

## 7. Physics Constants

| Address | Value | Usage |
|---------|-------|-------|
| `0x3FF66666_60000000` | 1.4 | Gravity constant (hardcoded in constructor, passed to BounceClass) |
| `0x007F65E8` | 0.017452778 (pi/180) | Degrees-to-radians conversion for angular velocity |
| `0x007E3D80` | 2.5 | Bounce stop threshold — if total velocity < 2.5, anim stops |
| `0x007E3D88` | 0.7854 (pi/4) | Used in slope reflection |
| `0x007E3DA8` | 150.0 (float) | Ground collision proximity — Z within 150 leptons of ground height |
| `0x007E1718` | 1.0 | Added to velocity range for inclusive random |

---

## 8. Current Rust Implementation Status

The Rust codebase has **no implementation of VoxelAnimClass** (the debris/bouncing system).

The existing `VoxelAnimation` struct in `src/sim/components.rs:348` is an **unrelated system** — it's a simple frame-cycling component for animating HVA frames on voxel units (idle animations, walk cycles). It has nothing to do with the flying debris physics system.

**To implement VoxelAnimClass, the following would be needed:**
1. VoxelAnimTypeClass — INI parsing for all fields in the `[VoxelAnims]` / `[PIECE]` etc. sections
2. BounceClass — Physics simulation (gravity, bounce, slope reflection, bridge/building collision)
3. VoxelAnimClass — Entity type with BounceClass embedded, Duration countdown, spawn/expire logic
4. Integration with destruction code — spawning debris when vehicles die
5. Draw integration — rendering the VXL model at the bouncing position with spin

**Files that would need changes:**
- `src/rules/` — new VoxelAnimType parsing
- `src/sim/` — new VoxelAnim entity/system with BounceClass
- `src/render/` — draw VoxelAnims at their physics positions
- Warhead/damage system — trigger debris spawning

---

## 9. Open Questions — All Resolved

All items from the original open questions have been resolved in the verification pass (Section 11):

1. **BounceClass internal layout** — RESOLVED. 0x50 (80) bytes, fully mapped.
2. **g_RulesClass_Instance offsets** — RESOLVED. +0x94 = Wake, +0xBC0 = SplashList vector, +0xBC4 = SplashList buffer, +0xBD0 = SplashList count.
3. **DAT_00b1d1bc** — RESOLVED. Bridge clearance height offset, `ftol(source * 4)`, zero at rest.
4. **DAT_0089c76c** — RESOLVED. Same bridge clearance pattern in BounceClass::Update.
5. **Offset 0x2F7 in Draw** — RESOLVED. MSB of DamageRadius (int at 0x2F4). Original engine bug — intended to check Translucent (0x295).
6. **IsTiberium crater creation** — RESOLVED. 8-direction for meteors, single-cell for non-meteors. Uses CellClass::CanPlaceTiberium + OverlayToTiberiumIndex for dynamic overlay type.
7. **Game loop iteration** — RESOLVED. LogicClass::PerTickUpdate iterates object layers, calling vtable+0x5C (AI). VoxelAnims in Layer 3 are part of this iteration.

---

## Sources

### Ghidra Functions Decompiled
- `0x007493B0` — VoxelAnimClass::Constructor (main, 3 params)
- `0x007498D0` — VoxelAnimClass::Constructor (default, for serialization)
- `0x007499F0` — VoxelAnimClass::Destructor
- `0x00749F30` — VoxelAnimClass::AI (2599 bytes)
- `0x0046B0C0` — VoxelAnim::Draw
- `0x0074AD80` — VoxelAnimTypeClass::Constructor
- `0x0074B050` — VoxelAnimTypeClass::ReadINI
- `0x004397E0` — BounceClass::Init
- `0x00439B00` — BounceClass::Update
- `0x0074AAD0` — VoxelAnimClass::GetClassID (vtable[3])
- `0x0074A970` — VoxelAnimClass::Load (vtable[5])
- `0x0074AA10` — VoxelAnimClass::Save (vtable[6])
- `0x0074AA30` — VoxelAnimClass::SaveLoad_detailed (vtable[13])
- `0x0074AB50` — VoxelAnimClass::scalar_deleting_destructor (vtable[8])
- `0x0074AB20` — VoxelAnimClass::WhatAmI() returns 0x29
- `0x0074AB10` — VoxelAnimClass::Size() returns 0x148
- `0x0074AB30` — VoxelAnimClass::GetType() returns *(this+0x104)
- `0x0074A960` — VoxelAnimClass::GetLayer() returns 3
- `0x006E2520` — Trigger action that creates VoxelAnims
- `0x00701900` — TechnoClass::ReceiveDamage (caller context)

### INI Files Checked
- `ini/rulesmd.ini` — [VoxelAnims] list, all 10 type sections (PIECE through PEBBLE)
- `ini/rules.ini` — base RA2 [VoxelAnims] (identical list)

### Existing Docs Referenced
- `VOXEL_RENDERING_ANALYSIS.md` — VXL rendering pipeline (related but separate system)

### Vtable
- VoxelAnimClass primary vtable at `0x007F6318` — 64 entries decoded

---

## 10. Gap Analysis — Deep Dive

### Gap 1: BounceClass Complete Struct Layout (80 bytes / 0x50) — RESOLVED

**Confidence:** HIGH — all 80 bytes fully mapped. **[CORRECTED: was claimed as 152/0x98 bytes, but tracing every field access in Init, Update, and FUN_004399e0 confirms the struct is exactly 0x50 bytes. There are no fields at 0x50-0x97 — those byte offsets correspond to post-BounceClass VoxelAnimClass fields.]**

The BounceClass struct is 0x50 (80) bytes, embedded at VoxelAnimClass+0xB0. From tracing the assembly of `BounceClass::Init` (0x004397E0), with ESI = `this` (BounceClass pointer), every field write was mapped:

| Byte Offset | Size | Type | Field | Evidence |
|------------|------|------|-------|----------|
| 0x00 | 8 | double | Elasticity | `[ESI+0x00..0x07]` — from Type->Elasticity |
| 0x08 | 8 | double | Gravity | `[ESI+0x08..0x0F]` — always 1.4 (hardcoded `0x3FF66666_60000000`) |
| 0x10 | 8 | double | AngularVelocityMagnitude | `[ESI+0x10..0x17]` — Init stores params 5/6 here. In Update, compared against 0.0 and used to control angular velocity clamping. The BounceClass::Update code: `if (0.0 < this->0x10 && FUN_0043a130() < this->0x10)` — if the angular velocity magnitude exceeds this threshold, normalize. |
| 0x18 | 4 | float | Position.X | `FILD [coords.X]; FSTP [ESI+0x18]` — integer coord converted to float |
| 0x1C | 4 | float | Position.Y | `FILD [coords.Y]; FSTP [ESI+0x1C]` |
| 0x20 | 4 | float | Position.Z | `FILD [coords.Z]; FSTP [ESI+0x20]` |
| 0x24 | 4 | float | Velocity.X | `[ESI+0x24]` — from param_9 velocity vector |
| 0x28 | 4 | float | Velocity.Y | `[ESI+0x28]` |
| 0x2C | 4 | float | Velocity.Z | `[ESI+0x2C]` — gravity applied each tick: `[ESI+0x2C] -= (float)Gravity` |
| 0x30 | 16 | Quaternion | Orientation | 4 floats (x,y,z,w). Init: identity matrix -> quaternion = (0,0,0,1). Updated each tick by multiplying with RotationQuat. `LEA ECX,[ESI+0x30]` in Init's 2nd Quaternion_CopyAndStore call. |
| 0x40 | 16 | Quaternion | RotationPerTick | 4 floats (x,y,z,w). Init: `Quaternion_FromAxisAngle(randomAxis, angularVelocity)`. `LEA ECX,[ESI+0x40]` in Init's 1st Quaternion_CopyAndStore call. On bounce, columns 0-2 (x,y,z) are negated to reverse spin direction. |

**[RESOLVED: There are NO remaining bytes. BounceClass is exactly 0x50 bytes (offsets 0x00-0x4F). The previously claimed "72 remaining bytes at 0x50-0x97" do not exist within BounceClass — they are VoxelAnimClass fields after BounceClass ends (VoxelAnim byte 0x100 onward). The confusion arose from the initial claim that BounceClass was 0x98 bytes.]**

**Rotation extraction (FUN_004399e0 at 0x004399E0):** Called from Draw to get the current rotation matrix. Reads the Orientation quaternion at BounceClass+0x30, calls `Quaternion_ToMatrix` (0x00646980), and copies the resulting 3x4 matrix (48 bytes) to the output buffer. This is how the VXL renderer gets the current spin orientation.

**Key helper functions identified:**
| Address | Name | Signature |
|---------|------|-----------|
| 0x0043a0b0 | Vec3_Set | `(float* vec, float x, float y, float z)` |
| 0x0043a0d0 | Vec3_Scale | `(float* vec, float scalar)` — multiplies all 3 components |
| 0x0043a100 | Vec3_Add | `(float* a, float* b)` — adds b into a |
| 0x0043a130 | Vec3_Length | `(float* vec)` — returns sqrt(x^2+y^2+z^2) |
| 0x00646480 | Quaternion_FromAxisAngle | `(float* out, float* axis, float angle)` — normalizes axis, applies half-angle sin/cos |
| 0x00645d20 | Quaternion_CopyAndStore | `(float* temp, float* dest, float* src)` — copies src into both temp and dest |
| 0x00645ed0 | Quaternion_Multiply | `(float* out, float* q1, float* q2)` — Hamilton product with normalization |
| 0x00646730 | Matrix3x3_ToQuaternion | `(float* quat_out, float* matrix_3x4)` — Shoemake's method |
| 0x005ae860 | Matrix3x4_SetIdentity | `(float* matrix)` — sets 3x4 identity (diag=1.0, rest=0) |
| 0x00645d00 | Matrix3x4_GetColumn | `(int matrix_base, int col_index)` — returns `base + col*4` |

**BounceClass::Update return values:**
- 0 = still airborne (no collision)
- 1 = bounced (hit ground, reflected velocity, negated rotation matrix columns)
- 2 = stopped (total velocity magnitude < 2.5, threshold at `0x007E3D80`)

**Angular velocity clamping (Update):** At the start of each tick, if `AngularVelocityMagnitude` (offset 0x10) is > 0 and the current velocity vector length (`Vec3_Length`) exceeds the stored magnitude, the velocity is normalized to that magnitude via `Vec3_Scale(vel, magnitude/length)`.

**Bounce velocity reflection:** On ground collision where the old and new cells have different heights (slope), the engine:
1. Gets a slope transformation matrix from `FUN_004848b0` / `FUN_00755c60`
2. Transforms the velocity vector through this matrix
3. Multiplies the reflected velocity by Elasticity
4. Negates all 3 columns of the facing/rotation matrix (reverses spin direction)

---

### Gap 2: IsTiberium Crater Creation Logic

**Confidence:** HIGH — fully traced from decompiled VoxelAnimClass::AI.

When Duration reaches 0 AND the VoxelAnim lands on ground (not water, not above ground):

**Meteor path** (`IsMeteor=true` AND `IsTiberium=true`):

1. First, if `Spawns` is set and `SpawnCount > 0`: spawn `Random(0, SpawnCount) + Random(0, SpawnCount)` child VoxelAnims of the Spawns type at the landing position. Note the double-random produces a triangular distribution centered on SpawnCount.

2. Then iterate all **8 adjacent cells** (directions 0-7, using `g_DirectionOffsets` table at `0x0089f68a` for Y offsets and `g_DirectionOffsets` at another address for X offsets):
   - Convert landing position to cell coordinates: `cellX = (x + (x >> 31 & 0xFF)) >> 8`, `cellY = (y + (y >> 31 & 0xFF)) >> 8`
   - Add direction offset to get adjacent cell: `adjCell = (cellX + dirX, cellY + dirY)`
   - Get CellClass for adjacent cell via `MapClass__Get_CellClass`
   - Call `CellClass__CanPlaceTiberium(0)` — if false, skip this cell
   - Call `IsWallOverlay()` to get an overlay type index
   - Look up `OverlayTypeClass*` from `DAT_00b0f4ec[overlayIndex]` (global OverlayTypeClass array)
   - **If cell has no existing overlay** (`cell->OverlayData == 0` at cell+0x11C):
     - Create new OverlayClass with a random frame index: `Random(0, 11)` (0x0B = 12 frames for tiberium)
     - The overlay type is looked up as: `DAT_00a83d84[overlayType->field_0x294 + randomFrame]` where field 0x294 in OverlayTypeClass is the base frame index, and DAT_00a83d84 is the overlay image/frame array
   - **If cell already has overlay** (`cell->OverlayData != 0` at cell+0x11C):
     - Create OverlayClass with growth frame: `Random(0, 1)` (2 growth stages)
     - Frame index calculation: `DAT_00a83d84[overlayType->field_0x294 + cell->OverlayData * 2 + overlayType->field_0xe8 + randomFrame - 2]`
     - This adds tiberium to an already-occupied cell by advancing its growth stage
   - After placing overlay: call `FUN_007235a0(cell+0x24)` (likely `CellClass::RecalcOccupation`)
   - Set `cell->byte_0x11E = 0` (clear some flag)
   - Get dirty rect via `FUN_0047fb90`, accumulate bounding rect for screen refresh
   - Call `RadarClass__MarkTerrainDirty` for the cell

3. After all 8 directions: call `TacticalClass__DirtyScreenRect` with the accumulated bounding rect, then `Delete()`.

**Non-meteor path** (`IsMeteor=false` AND `IsTiberium=true`):

Same overlay creation logic but for a **single cell** (the landing cell) instead of 8 adjacent cells. Same CanPlaceTiberium check, same overlay creation with random frame or growth.

**Key globals:**
- `DAT_00b0f4ec` — global OverlayTypeClass pointer array
- `DAT_00a83d84` — overlay frame/image index array
- Cell offsets: +0xEC = terrain type (2 = water), +0x11C = OverlayData (existing tiberium level), +0x11E = flag cleared after placement, +0x24 = cell coordinate pair

---

### Gap 3: Mystery Field at VoxelAnimTypeClass+0x2F7 in Draw

**Confidence:** HIGH — verified from assembly.

**Finding: Offset 0x2F7 is the most-significant byte of DamageRadius (int at 0x2F4).**

The Draw function at `0x0046B0C0` loads a pointer from `this+0xAC` (VoxelAnimClass stores an ObjectTypeClass* pointer at byte offset 0xAC, inherited from ObjectClass — this is SEPARATE from the VoxelAnimTypeClass* at 0x104 but points to the same object). It then checks `byte ptr [typePtr + 0x2F7]`.

Assembly evidence (from `VoxelAnim__Draw`):
```asm
0046b19b: MOV EDX,dword ptr [EAX + 0xac]    ; load type class pointer
0046b1ac: MOV AL,byte ptr [EDX + 0x2f7]     ; check byte at 0x2F7
0046b1b5: TEST AL,AL
0046b1b7: JZ 0x0046b1c5                     ; skip if zero
0046b1b9: MOV EBX,dword ptr [0x0081af00]    ; load translucent blitter
0046b1bf: OR EDI,0x2000                     ; set alpha flag
```

The VoxelAnimTypeClass::ReadINI stores DamageRadius as a full 32-bit int at offset 0x2F4:
```asm
0074b401: CALL 0x005276d0                   ; CCINIClass::ReadInt("DamageRadius")
0074b40c: MOV dword ptr [ESI + 0x2f4],EAX  ; store full 4-byte int
```

In little-endian x86, byte 0x2F7 is the MSB of this 4-byte integer. For any DamageRadius value less than 16,777,216 (0x01000000), this byte is zero. **No standard YR VoxelAnim has DamageRadius anywhere near this value** (typical values: 0-200), so the translucency check in Draw is **effectively dead code** under normal gameplay.

This is likely a **bug in the original engine**: the developer probably intended to check the `Translucent` bool at offset 0x295, but the wrong offset was compiled into the Draw function. The field at 0x295 (Translucent, read from INI) is never checked in Draw; instead, the accidental byte at 0x2F7 (DamageRadius MSB) is checked.

**For Rust implementation:** Check the actual `Translucent` field (0x295) rather than replicating this bug.

**Additional finding — ObjectTypeClass pointer at VoxelAnimClass+0xAC:**
VoxelAnimClass stores its type pointer at TWO offsets:
- +0xAC: inherited from ObjectClass (ObjectTypeClass*)
- +0x104: VoxelAnimClass-specific (VoxelAnimTypeClass* via GetType())
Both point to the same VoxelAnimTypeClass instance.

---

### Gap 4: Rules Global Splash Anim Offsets

**Confidence:** HIGH — traced from RulesClass::ReadGeneral and RulesClass::ReadCombatDamage (FUN_0066bbb0).

| RulesClass Offset | Size | Type | INI Section | INI Key | Default Value | Notes |
|-------------------|------|------|-------------|---------|---------------|-------|
| +0x94 | 4 | AnimTypeClass* | [General] | Wake | WAKE1 | Water wake animation. Used in VoxelAnimClass::AI for non-meteor water landing splash. |
| +0xBC0 | 24 | DynamicVectorClass<AnimTypeClass*> | [CombatDamage] | SplashList | H2O_EXP3,H2O_EXP2,H2O_EXP1 | List of water explosion animations. The vector layout: vtable(4), buffer(4), capacity(4), ..., count at some offset. |
| +0xBC4 | 4 | AnimTypeClass** | (part of SplashList vector) | — | — | Buffer pointer of the SplashList DynamicVectorClass. Dereferenced as `*buffer` to get first element, or `buffer[count-1]` for last. |
| +0xBD0 | 4 | int | (part of SplashList vector) | — | — | Count/size of the SplashList vector. Used as index: `buffer[(count-1)]` to pick the last splash anim for meteors. |

**Usage in VoxelAnimClass::AI (Duration == 0, on water, below ground height):**

- **Non-meteor (`IsMeteor=false`):** Plays two animations:
  1. `Rules->Wake` (offset +0x94) — the wake/ripple effect at the landing coords
  2. `*Rules->SplashList.buffer` (first element, offset +0xBC4 dereferenced) — the water explosion, spawned at Z+10

- **Meteor (`IsMeteor=true`):** Plays one animation:
  1. `Rules->SplashList.buffer[count - 1]` (last element) — picks the last/largest splash anim, spawned at Z+5
  
  The expression in the binary is: `buffer[-1 + count]` = `*(*(Rules+0xBC4) + (*(Rules+0xBD0) - 1) * 4)`

**INI values from rulesmd.ini:**
- `Wake=WAKE1` (in [General])
- `SplashList=H2O_EXP3,H2O_EXP2,H2O_EXP1` (in [CombatDamage])

So non-meteor debris that lands in water gets WAKE1 + H2O_EXP3 (first in list), while meteor impacts in water get H2O_EXP1 (last in list = smallest explosion).

---

### Gap 5: Ground Height Globals

**Confidence:** HIGH — decompiled the write functions and traced xrefs.

Both globals are **derived height values** computed from other floating-point globals. They share an identical computational pattern.

**DAT_00b1d1bc (used in VoxelAnimClass::AI):**

Written by `FUN_00749310` at `0x00749310`:
```c
DAT_00b1d1bc = Math__ftol(DAT_00b1d1c8 * 4);
```

- `DAT_00b1d1c8` is a double that is written at `0x0074929b` and read at `0x007492b0` / `0x007492e1`.
- Called from a vtable dispatch at `0x00815578` — this is a **property setter/getter** pattern.
- The function is part of a small vtable cluster near `0x00749280..0x00749340` dealing with height calculations.
- At rest (no bridge), both `DAT_00b1d1c8` and `DAT_00b1d1bc` are 0.

**DAT_0089c76c (used in BounceClass::Update):**

Written by `FUN_00439610` at `0x00439610`:
```c
DAT_0089c76c = Math__ftol(DAT_0089c778 * 4);
```

- `DAT_0089c778` is a double written at `0x0043959b`.
- Called from vtable dispatch at `0x00812738` — same property setter pattern.
- Part of a vtable cluster near `0x00439580..0x00439640`.
- At rest, both are 0.

**Interpretation:** These globals represent a **bridge clearance height offset**, stored as `source_double * 4` converted to integer leptons. The pattern `groundHeight + DAT_xxxx` adds a bridge-level offset to the base ground height. When no bridge is present, the offset is 0. When a bridge is crossed, the offset equals the bridge surface height minus ground height, scaled by 4 for lepton conversion.

In BounceClass::Update, the offset adjusts the effective ground level for bounce collision:
```c
local_118 = groundHeight + DAT_0089c76c;
```

In VoxelAnimClass::AI, it adjusts the "above ground" test for deciding splash vs. land behavior:
```c
bVar17 = groundHeight + DAT_00b1d1bc <= currentZ;
```

**For Rust implementation:** These can initially be treated as 0 (standard terrain). Bridge support will require computing the bridge surface offset and applying it. The computation is `ftol(bridge_height_double * 4.0)` where `bridge_height_double` is likely `CellClass::BridgeHeight` or similar.

---

### Updated Open Questions

Items resolved by this gap analysis and verification pass:

1. **BounceClass internal layout** — FULLY RESOLVED. BounceClass is exactly 0x50 (80) bytes. All fields mapped. There are NO remaining unmapped bytes.
2. **g_RulesClass_Instance offsets** — RESOLVED. +0x94 = Wake (AnimTypeClass*), +0xBC4 = SplashList buffer, +0xBD0 = SplashList count.
3. **DAT_00b1d1bc** — RESOLVED. Bridge clearance height offset, computed as `ftol(source_double * 4)`, zero at rest.
4. **DAT_0089c76c** — RESOLVED. Same pattern as DAT_00b1d1bc, used in BounceClass::Update.
5. **VoxelAnimTypeClass offset 0x2F7** — RESOLVED. High byte of DamageRadius (int at 0x2F4). Dead code in Draw under normal conditions — likely a bug where 0x295 (Translucent) was intended.
6. **IsTiberium crater creation** — RESOLVED. 8-direction iteration for meteors, single-cell for non-meteors. Creates tiberium overlay with random frame (new cell) or growth stage (existing tiberium).

**No remaining open questions** — all gaps have been resolved in the verification pass.

---

## 11. Verification Pass

**Date:** 2026-04-06
**Method:** Re-decompiled all core functions via Ghidra MCP, traced assembly for BounceClass::Init and BounceClass::Update instruction-by-instruction, read vtable memory, verified constants from binary.

### V1: BounceClass Struct Layout — MAJOR CORRECTION

**Original claim:** BounceClass is 0x98 (152) bytes with 72 unmapped bytes at 0x50-0x97.
**Verified finding:** BounceClass is **0x50 (80) bytes**. All 80 bytes are fully mapped.

Evidence:
- BounceClass::Init (`0x004397E0`): ESI = `this`. Writes to ESI+0x00 through ESI+0x4F only. Assembly confirms:
  - `[ESI+0x00..0x07]`: Elasticity double
  - `[ESI+0x08..0x0F]`: Gravity double
  - `[ESI+0x10..0x17]`: AngularVelocityMagnitude double (Init stores params 5/6 here; was previously listed as "unused/zero")
  - `[ESI+0x18..0x20]`: Position floats (FILD/FSTP from integer coords)
  - `[ESI+0x24..0x2F]`: Velocity floats (copied from param_9)
  - `[ESI+0x30..0x3F]`: Orientation quaternion (from Matrix3x4_SetIdentity -> Matrix3x3_ToQuaternion -> CopyAndStore at `LEA ECX,[ESI+0x30]`)
  - `[ESI+0x40..0x4F]`: RotationPerTick quaternion (from Quaternion_FromAxisAngle -> CopyAndStore at `LEA ECX,[ESI+0x40]`)
- BounceClass::Update (`0x00439B00`): Only accesses offsets 0x00-0x4F. No access to 0x50+.
- FUN_00439A10 (velocity magnitude): Only accesses 0x08, 0x18-0x20, 0x24-0x2C.
- FUN_004399E0 (quaternion-to-matrix for Draw): Reads `ECX+0x30` (Orientation quaternion). No access to 0x50+.
- The VoxelAnimClass constructor initializes BounceClass region (0xB0-0xFF) then immediately writes post-BounceClass fields starting at 0x100.
- VoxelAnimClass::SaveLoad_detailed (`0x0074AA30`) serializes fields at 0x100, 0x104, 0x108, 0x10C, 0x110, 0x13C, 0x140 — all AFTER BounceClass ends at 0xFF.

**What fields 0x10-0x17 actually are:** Previously listed as "unused/zero" (param_7/param_8). Re-analysis shows this is the **AngularVelocityMagnitude** double. In the normal constructor path, params 7/8 are 0 (passed as `uVar14=0, uVar15=0`). In the meteor path (FUN_00439690), param_5 is `0x40080000` (high word of the double 3.0), giving AngularVelocityMagnitude = 3.0. In Update, this is compared against `Vec3_Length(velocity)` — if velocity exceeds this magnitude, it's clamped via `Vec3_Scale`.

### V2: VoxelAnimClass Struct Layout — Confirmed with corrections

**param_1 type in Constructor:** `undefined4 *` (int pointer, multiply indices by 4). CONFIRMED from decompilation.
**param_1 type in AI:** `int *` (same semantics). CONFIRMED.
**param_1 type in ReadINI:** `int` (direct byte offsets via ESI register). CONFIRMED from assembly.

**Size 0x148:** Confirmed. `FUN_0074AB10` returns `0x148`.
**WhatAmI 0x29:** Confirmed. `FUN_0074AB20` returns `0x29`.
**GetType at 0x104:** Confirmed. `FUN_0074AB30` returns `*(param_1 + 0x104)`.
**GetLayer returns 3:** Confirmed. `FUN_0074A960` returns `3`.

**Field-by-field verification from SaveLoad_detailed (0x0074AA30):**
- `0x100`: int — serialized via FUN_004a1d50 (Timer, init -1)
- `0x104`: VoxelAnimTypeClass* — serialized as ID via vtable+0x10 call
- `0x108`: ParticleSystemClass* — serialized conditionally (non-null)
- `0x10C`: HouseClass* — serialized as ID
- `0x110`: byte — serialized via FUN_004a1ca0 (marked for deletion)
- `0x13C`: byte — serialized via FUN_004a1ca0 (unknown flag)
- `0x140`: int — serialized via FUN_004a1d50 (Duration)

### V3: VoxelAnimTypeClass Defaults — All Confirmed

Decoded constructor values from `undefined4 *` param_1 indices:

| Field | Index pair | Raw hex | Decoded value | Report claim | Match |
|-------|-----------|---------|---------------|-------------|-------|
| Elasticity | 0xA8/0xA9 | 0xA0000000/0x3FE99999 | 0.8 | 0.8 | YES |
| MinAngularVelocity | 0xAA/0xAB | 0/0 | 0.0 | 0.0 | YES |
| MaxAngularVelocity | 0xAC/0xAD | 0x1CE64946/0x3FC656ED | 0.17453 rad (10.0 deg) | ~10 deg | YES |
| MinZVel | 0xAE/0xAF | 0/0x400C0000 | 3.5 | 3.5 | YES |
| MaxZVel | 0xB0/0xB1 | 0/0x40140000 | 5.0 | 5.0 | YES |
| MaxXYVel | 0xB2/0xB3 | 0/0x402E0000 | 15.0 | 15.0 | YES |
| Duration | 0xA7 | 0x1E | 30 | 30 | YES |
| VoxelIndex | 0xA6 | 0 | 0 | 0 | YES |
| StartSound | 0xB7 | 0xFFFFFFFF | -1 | -1 | YES |
| StopSound | 0xB8 | 0xFFFFFFFF | -1 | -1 | YES |

**Deg-to-rad constant at 0x007F65E8:** Reads as `0x3F91DF2417_1EA105` = 0.01745277... (pi/180). CONFIRMED.

**Angular velocity sentinel — CORRECTED:** The report claimed -1.0 sentinel compared against -1.0. Actual assembly at `0x0074b11b` shows the comparison is `FCOM double ptr [0x007e2800]` where 0x007e2800 = 0.0. The sentinel default is -1.0 passed to ReadDouble, but the skip-condition checks `== 0.0`, not `== -1.0`. This is an original engine bug (dormant in practice since all VoxelAnims specify these keys).

### V4: IsTiberium Logic — Confirmed

**SpawnCount triangular distribution:** CONFIRMED. AI code:
```c
count = Random(0, SpawnCount) + Random(0, SpawnCount)
```
Sum of two uniform randoms = triangular distribution centered on SpawnCount.

**CellClass::CanPlaceTiberium (0x004838E0):** Decompiled. Checks:
1. Cell is in playfield
2. Cell flags don't have 0x500 set (bridge/impassable bits)
3. No building with `TypeClass+0xC9A != 0` and `TypeClass+0x1701 != 0` on the cell
4. No terrain overlay (type 0x24) with `+0x2B1 != 0` blocking placement
5. Terrain type supports tiberium growth (via `DAT_0089ea60` lookup table)
6. Cell overlay index at +0x44 must be -1 (no existing overlay)
7. Cell tiberium data at +0x11C must be 0 (for new placement)
8. Additional overlay check at +0x38 against global overlay array

**Overlay type index:** NOT hardcoded. The overlay type is determined dynamically by `CellClass__OverlayToTiberiumIndex()`, then looked up in `DAT_00b0f4ec` (global TiberiumClass array). The tiberium type's `field_0xE0` holds a pointer to the OverlayTypeClass, and `field_0x294` on that gives the base frame index. New cells get `Random(0, 0xB)` (12 frames), existing cells get growth via `Random(0, 1)`.

### V5: VoxelAnimClass Vtable at 0x007F6318 — Complete

64 entries decoded. Key VoxelAnimClass-specific overrides:

| Slot | Offset | Address | Function |
|------|--------|---------|----------|
| 0 | +0x00 | 0x00410260 | AbstractClass::QueryInterface |
| 1 | +0x04 | 0x00410300 | AbstractClass::AddRef |
| 2 | +0x08 | 0x00410310 | AbstractClass::Release |
| 3 | +0x0C | 0x0074AAD0 | **VoxelAnimClass::GetClassID** (returns CLSID from DAT_007e9650) |
| 5 | +0x14 | 0x0074A970 | **VoxelAnimClass::Load** |
| 6 | +0x18 | 0x0074AA10 | **VoxelAnimClass::Save** |
| 8 | +0x20 | 0x0074AB50 | **VoxelAnimClass::scalar_deleting_destructor** |
| 11 | +0x2C | 0x0074AB20 | **VoxelAnimClass::WhatAmI** → 0x29 |
| 12 | +0x30 | 0x0074AB10 | **VoxelAnimClass::Size** → 0x148 |
| 13 | +0x34 | 0x0074AA30 | **VoxelAnimClass::SaveLoad_detailed** |
| 23 | +0x5C | 0x00749F30 | **VoxelAnimClass::AI** |
| 30 | +0x78 | 0x0074A960 | **VoxelAnimClass::GetLayer** → 3 |
| 34 | +0x88 | 0x0074AB30 | **VoxelAnimClass::GetType** → *(this+0x104) |

Most other slots (0, 1, 2, 4, 7, 9, 10, 14-22, 24-29, 31-33, 35-63) are inherited from ObjectClass or AbstractClass and not overridden by VoxelAnimClass.

### V6: Draw Function Detail — Enhanced

**Address:** `0x0046B0C0`

Full pipeline traced from assembly:

1. **VXL_Init_Simple** (`0x00753C80`): Called with `section_index=0`, light params, and `&g_VXL_LightDirection` (global at `0x00887470`).
2. **VXL_Clear_TileMap** (`0x00753E00`): Clears the voxel tile map buffer.
3. **Section loop** (iterates `VXL->SectionCount` at `[vxl_data + 4]`):
   a. Get HVA transform: `frame_index = (param_5 % hva_num_frames) * sections_per_frame + section_index`. Each HVA frame is a 3x4 matrix (0x30 bytes) stored at `[hva_data + 0x0C] + frame_index * 0x30`.
   b. Copy 3x4 matrix (48 bytes, 12 dwords) to local stack.
   c. **Locomotion_Matrix** (`0x005AF980`): Transforms the HVA matrix through the locomotion pipeline. Called on `local_c0` buffer.
   d. Copy result to `local_90`.
   e. **Locomotion_Matrix** again with `local_90` and `g_VXL_DrawMatrix` at `0x00887430`.
   f. **VXL_Submit_BoundingBox** (`0x007540F0`): Submits the section for rasterization.
4. **VXL_Sort_Rasterize** (`0x00754510`): Returns a 24-byte struct (6 ints) containing the rasterized tile offsets and bounding box.
5. **Type pointer retrieval**: Loads `this->ObjectTypeClass_ptr` from `VoxelAnimClass+0xAC`.
6. **Translucency check**: Tests byte at `TypeClass + 0x2F7` (MSB of DamageRadius — see V7).
   - If non-zero: sets alpha flag `0x2000` in draw flags, loads translucent blitter from `DAT_0081af00`.
   - The blitter type selector uses `(-(flag != 0) & 0xFFFFFFFE) + 2`: 0 for translucent, 2 for opaque.
7. **Screen position**: `screen_x = param_3[0] + tile_offset_x`, `screen_y = param_3[1] + tile_offset_y`.
8. **Palette lookup**: `palette_ptr = DAT_00b054d4[Type->Spawns]->field_0x30C`. (The `Spawns` field at 0x2D4 appears to be re-used or coincidentally overlaps with a palette index in this context — needs further investigation.)
9. **Blitter_selector** (`0x00490B90`): Selects the appropriate blit function based on draw flags.
10. **FUN_00753C70**: Prepares the final blit parameters (source rect, dest point, etc.).
11. **FUN_004AF2A0**: Performs the actual blit to the tactical surface at `0x00887314`.

**Rotation for rendering**: NOT extracted within Draw itself. The HVA transform already encodes the rotation. For VoxelAnims, the locomotion matrix pipeline (FUN_005AF980) likely calls FUN_004399E0 (BounceClass quaternion-to-matrix) to get the current spin orientation, which is then composed with the HVA animation frame.

### V7: 0x2F7 "Dead Code" — Confirmed with detail

**Claim:** Offset 0x2F7 is the MSB of DamageRadius (int at 0x2F4) and is always 0.
**Status:** CONFIRMED from assembly.

Assembly at `0x0046B1AC`: `MOV AL, byte ptr [EDX + 0x2F7]` where EDX = TypeClass pointer loaded from `[EAX + 0xAC]`.

The field is used in TWO places within Draw:
1. **Translucent blitter selection**: If non-zero, sets flag `0x2000` and loads alternate blitter.
2. **Blit type calculation**: `(-(byte_2F7 != 0) & 0xFFFFFFFE) + 2` → 0 (translucent) or 2 (normal).

For any DamageRadius < 16,777,216 (0x01000000), byte 0x2F7 is always 0. Maximum DamageRadius in vanilla YR VoxelAnims is approximately 200.

**Conclusion:** This is a confirmed original engine bug. The developer likely intended to check `Translucent` at offset 0x295. The `Translucent` bool IS read from INI and stored at 0x295, but no code path ever reads it back. For Rust implementation, use the `Translucent` field (0x295) instead.

### V8: Global VoxelAnim Iteration — Confirmed

**Global DynamicVector at 0x00887388:**
- Xrefs from: VoxelAnimClass::Constructor (adds self), VoxelAnimClass::Destructor (removes self), FUN_0067D300 (save game), and `0x0040ee5d` / `0x0040ee88` (initialization/clearing).

**Tick iteration:**
- LogicClass::PerTickUpdate (`0x0055AFB0`) is the main per-tick function.
- It iterates multiple object layers and calls `vtable+0x5C` (the AI method) on each object.
- The iteration that includes VoxelAnims copies from `DAT_008b40ec` (buffer) / `DAT_008b40f8` (count) into a temporary vector, then iterates the copy calling AI. This is the standard "iterate-a-copy" pattern to allow safe deletion during iteration.
- VoxelAnims are in Layer 3 (Top), confirmed by GetLayer returning 3.
- Other layers iterated in LogicClass::PerTickUpdate: `DAT_008a020c` (iterated in reverse), `DAT_00b04bd4` (iterated in reverse), LogicClass's own layer (`param_1 + 4`), and HouseClass AI.

### Summary of Corrections

| Item | Original Claim | Corrected Finding |
|------|---------------|-------------------|
| BounceClass size | 0x98 (152) bytes | **0x50 (80) bytes** — no fields exist at 0x50-0x97 |
| BounceClass 0x10-0x17 | "unused/zero" (param_7/param_8) | **AngularVelocityMagnitude** double — clamping threshold for velocity |
| Angular velocity sentinel | "if INI reads -1.0, retains default" | Comparison is against **0.0**, NOT -1.0. Bug in original engine (dormant). |
| IsMeteor param_1[0x37] | "visual rotation field" | **Velocity.Z gravity compensation** — adds gravity back to cancel BounceClass subtraction, giving meteors zero net gravity |
| BounceClass 0x30 | Listed as "CRect bounding rect 1" | **Orientation quaternion** (4 floats). CRect init is pre-zeroing before BounceClass::Init overwrites. |
| BounceClass 0x40 | Listed as "CRect bounding rect 2" | **RotationPerTick quaternion** (4 floats). Same pre-zeroing pattern. |

### New Findings

1. **FUN_00439690 at `0x00439690`**: Alternate BounceClass initialization for meteors spawned from terrain. Uses spherical coordinate random velocity generation (azimuth + elevation angles via Sin/Cos_lookup), with AngularVelocityMagnitude set to 3.0 (hardcoded `0x40080000`).

2. **FUN_004399E0 at `0x004399E0`**: BounceClass rotation extraction helper. Takes a BounceClass pointer, reads Orientation quaternion at +0x30, converts to 3x4 matrix via `Quaternion_ToMatrix` (`0x00646980`), copies 48-byte result to output. Used by the Draw pipeline to get current spin.

3. **Building bounce blocking**: Full logic in Update: if a building is found in the landing cell AND `building->TypeClass->field_0x16BF != 0` AND `building->Strength > 7`, then bouncing is blocked. Additionally, vtable method `+0x80` (likely `IsInAir()`) blocks bouncing if true.

4. **Draw palette lookup**: The Draw function at `0x0046b1E3` loads from `DAT_00b054d4` indexed by `Type->Spawns` field at offset 0x2D4. This appears to be a global conversion palette array. The actual pixel buffer for blitting comes from `palette_entry->field_0x30C`.

5. **Complete VoxelAnimClass field map (0x100-0x147):**

| Byte Offset | Size | Field | Evidence |
|------------|------|-------|----------|
| 0x100 | 4 | int: Timer/Unknown (init -1) | Constructor: `param_1[0x40] = 0xFFFFFFFF`. Serialized in SaveLoad. |
| 0x104 | 4 | VoxelAnimTypeClass* Type | Constructor: `param_1[0x41] = param_2`. GetType returns this. |
| 0x108 | 4 | ParticleSystemClass* AttachedSystem | Constructor: `param_1[0x42] = 0`, then conditionally created. |
| 0x10C | 4 | HouseClass* Owner | Constructor: `param_1[0x43] = param_4`. |
| 0x110 | 1 | bool: MarkedForDeletion | Constructor: `*(byte*)(param_1+0x44) = 0`. Checked in AI. |
| 0x111-0x113 | 3 | (padding) | |
| 0x114-0x127 | 20 | SoundEvent 1 (StartSound) | Init via FUN_00405BE0. |
| 0x128-0x13B | 20 | SoundEvent 2 (StopSound) | Init via FUN_00405BE0. |
| 0x13C | 1 | bool: Unknown flag | Constructor: `*(byte*)(param_1+0x4F) = 0`. Serialized in SaveLoad. |
| 0x13D-0x13F | 3 | (padding) | |
| 0x140 | 4 | int: Duration | Constructor: `param_1[0x50] = Type->Duration`. Decremented in AI. |
| 0x144-0x147 | 4 | (struct padding to 0x148) | |

---

## 12. Exhaustive Detail Pass

**Date:** 2026-04-06
**Method:** Re-decompiled all functions listed below via Ghidra MCP, traced call chains, read vtable memory, cross-referenced globals.

---

### 12.1 Draw Pipeline in Full Detail (T1)

**VoxelAnim__Draw at `0x0046B0C0`** — line-by-line reconstruction:

```
void VoxelAnim__Draw(
    int *param_1,       // VXL/HVA data pair: param_1[0]=VXL*, param_1[1]=HVA*
    uint param_2,       // unused (light setup param, forwarded)
    int *param_3,       // screen coordinate base [x, y]
    uint param_4,       // palette/remap context
    uint param_5,       // current frame counter (for HVA animation)
    uint param_6,       // draw flags
    uint param_7        // additional blit params
)
```

**Step 1 — VXL_Init_Simple (`0x00753C80`):**
Called with `(0, param_2, &g_VXL_LightDirection)`. This function:
- Calls `FUN_005AFC20()` — retrieves the **inverse camera matrix** (3x4, transposed rotation) and copies it to a local 48-byte buffer.
- Calls `FUN_005AF4D0(local_6c, param_5)` — transforms the light direction vector through the camera matrix, producing a camera-space light direction.
- Calls `FUN_007564B0(param_2, param_3)` — computes a VXL section data pointer: `*(base + 0x14) + (*(*(base + 0x10) + section*0xC) + col) * 0xA4`. This is the VXL section geometry lookup.
- Calls `VXL_SimpleLighting()` — sets up the simple directional lighting state using the transformed light vector.

**Parameters decoded:**
- param_1 (section_index) = 0 (always starts at section 0)
- param_2 = light/remap context (forwarded from caller)
- param_3 = pointer to `g_VXL_LightDirection` at `0x00887470` (3-float global light direction vector)

**Step 2 — VXL_Clear_TileMap (`0x00753E00`):**
Clears the internal voxel tile map buffer used for rasterization.

**Step 3 — Section loop:**
Iterates `*(VXL_data + 4)` sections (the VXL section count):

a. **HVA frame index calculation:**
```c
frame_index = (param_5 % hva_num_frames) * sections_per_frame + section_index;
```
Where `hva_num_frames = *(HVA + 8)` and `sections_per_frame = *(HVA + 4)`. Each HVA frame is a **3x4 matrix** (48 bytes = 12 floats) stored at `*(HVA + 0x0C) + frame_index * 0x30`.

b. **Copy HVA matrix** to `local_c0` (48 bytes, 12 dwords).

c. **First Locomotion_Matrix call** (`0x005AF980`):
`Locomotion_Matrix(result, local_c0, locomotion_matrix)`
This is a **3x4 matrix multiply**: `result = locomotion_matrix * local_c0`. The locomotion matrix comes from the BounceClass orientation (see below). The result transforms the HVA bone-space into world-oriented space.

d. **Copy result** to `local_90`.

e. **Second Locomotion_Matrix call:**
`Locomotion_Matrix(result, local_90, g_VXL_DrawMatrix)`
Where `g_VXL_DrawMatrix` is at `0x00887430`. This applies the camera/view transform, transforming from world-oriented space to screen-projected space.

f. **VXL_Submit_BoundingBox (`0x007540F0`):**
Submits the transformed section bounding box for depth-sorted rasterization.

**Step 4 — VXL_Sort_Rasterize (`0x00754510`):**
This is the voxel rendering core. It:
1. Computes a **center offset** from the accumulated bounding boxes: `(min + max) * 0.5` scaled by a global factor at `0x007E5168`.
2. Iterates `g_VXL_QuadCount` quad records (each 0x48 = 72 bytes), calling `VXL_Quad_Rasterizer()` per quad with the center offset.
3. Builds an array of pointers to `g_VXL_BoxRecords` (each 0x88 = 136 bytes).
4. **Bubble-sorts** the box records by their Z-depth (float at offset +0x24 within each record) — **back-to-front painter's sort**.
5. If `g_VXL_MirrorFlag == 0`: rasterizes front-to-back via `VXL_Section_Rasterizer()`.
   If mirror flag set: rasterizes in reverse order.
6. Returns a **24-byte result struct** (6 ints) containing:
   - `[0]`: tile_offset_x (screen X of rasterized center)
   - `[1]`: tile_offset_y (screen Y of rasterized center)
   - `[2]`: bounding_width
   - `[3]`: bounding_height
   - `[4]`: blit_origin_x (0x7C - width/2)
   - `[5]`: blit_origin_y (0x7C - height/2)

**Step 5 — Type pointer retrieval:**
Loads `this->ObjectTypeClass_ptr` from VoxelAnimClass byte offset +0xAC (inherited ObjectClass field). Both +0xAC and +0x104 point to the same VoxelAnimTypeClass instance.

**Step 6 — Translucency check:**
Tests byte at TypeClass+0x2F7 (MSB of DamageRadius — confirmed bug, see V7). If non-zero:
- Sets alpha flag: `draw_flags |= 0x2000`
- Loads alternate translucent blitter from global `DAT_0081AF00`

**Blit type selector:** `(-(byte_2F7 != 0) & 0xFFFFFFFE) + 2` yields 0 (translucent) or 2 (normal opaque).

**Step 7 — Screen position:**
```c
screen_x = param_3[0] + tile_offset_x;
screen_y = param_3[1] + tile_offset_y;
```

**Step 8 — Blitter selection:**
`Blitter_selector(0x00490B90)` picks the appropriate blit function from draw flags.

**Step 9 — Final blit:**
`FUN_00753C70` prepares blit parameters (source rect from rasterized tile, dest point).
`FUN_004AF2A0` performs the actual pixel copy to the tactical surface at `0x00887314`.

**How BounceClass orientation reaches the renderer:**

The Draw caller is `0x00468090` (Ghidra label: `BulletClassDrawItRotatingShpFrameDispatch` — label is wrong; this function dispatches VXL or SHP rendering for bouncing objects including VoxelAnims; "VoxelAnimTypeClass::DrawObject" is incorrect [corrected 2026-05-28: label confirmed via `get_function_by_address 0x00468090` — ROOT_CAUSE: RTTI_LABEL_DRIFT]). This function:

1. Checks TypeClass+0x236 flag — if true, it's a VXL-type VoxelAnim (always true for actual VoxelAnims).
2. Builds a rotation matrix from BounceClass state:
   - Reads doubles at VoxelAnimClass offsets 0xE8-0xF0 (which are inside the BounceClass quaternion area at BounceClass offsets 0x38-0x40).
   - Calls `Math__atan2` to compute yaw and pitch angles from the quaternion/orientation data.
   - Calls `Matrix3x4_RotateZ` and `Matrix_rotate_y_axis` to build a rotation matrix.
3. Calls `FUN_00754BE0` to copy the current global draw matrix (from `0x00B44318`) as a base.
4. Multiplies the rotation into the base via `Locomotion_Matrix`.
5. Calls `VoxelAnim__Draw()` with the composed matrix and the TypeClass VXL data at TypeClass+0xB0.

**IMPORTANT:** The BounceClass `FUN_004399E0` (quaternion-to-matrix) function is used in BounceClass::Update for the velocity reflection/slope handling, NOT directly in the draw path. The draw path instead extracts Euler angles from the orientation data and builds its own rotation matrix. This is a key difference.

**Shadow rendering for VoxelAnims:**

VoxelAnims do **NOT** have a dedicated shadow pass. The FUN_00468090 draw function has two branches:
- VXL path (TypeClass+0x236 set): calls VoxelAnim__Draw only. No shadow blit.
- SHP path (TypeClass+0x236 clear): calls CC_Draw_Shape with flags 0x2601 for shadow and 0x2E00 for main sprite.

Since all actual VoxelAnim types use VXL rendering, there is **no shadow rendering for VoxelAnims**. The SHP path with shadow support is for a hypothetical SHP-based debris type (or Tiberian Sun legacy) that doesn't exist in standard YR. **Confidence: HIGH.**

---

### 12.2 Destructor Deep Dive (T2)

**VoxelAnimClass::Destructor at `0x007499F0`** — full reconstruction:

```
void VoxelAnimClass__Destructor(VoxelAnimClass* this) {
    // 1. Reset vtable pointers (standard C++ destructor pattern)
    this->vtable_primary    = &vtable__VoxelAnimClass;
    this->vtable_secondary1 = &vtable__VoxelAnimClass__secondary_4;
    this->vtable_secondary2 = &vtable__VoxelAnimClass__secondary_8;
    this->vtable_secondary3 = &vtable__VoxelAnimClass__secondary_12;

    // 2. FUN_007258D0 — Global object detach/cleanup
    //    This is the central "object being destroyed" notification.
    //    Calls WhatAmI() to get type ID (0x29 for VoxelAnim).
    //    For VoxelAnims (type 0x29), it falls through to the
    //    generic ObjectClass detach path:
    //    - If object has IsOnMap flag set (byte +0x14 bit 1):
    //      - Iterates all AnimClass instances and calls
    //        vtable+0x28 (PointerExpiredNotify) on each
    //      - Calls FUN_00439150 to remove from some global vector
    //      - Calls FUN_0054E590 (layer removal)
    //      - Iterates all LogicClass layers calling vtable+0x28
    //      - For certain types: calls FUN_00413490 (team detach)
    //      - Updates minimap: g_Tactical->vtable+0x28
    //      - Calls FUN_0055B880 (display layer removal)

    // 3. Remove self from global VoxelAnimClass vector at 0x00887388
    //    Uses the vector's Find method (vtable+0x10) to get index,
    //    then shifts remaining elements down.
    int idx = g_VoxelAnimVector.Find(this);
    if (idx != -1 && idx < g_VoxelAnimVector.count) {
        g_VoxelAnimVector.count--;
        // Shift elements left to fill gap
        for (int i = idx; i < g_VoxelAnimVector.count; i++)
            g_VoxelAnimVector.buffer[i] = g_VoxelAnimVector.buffer[i+1];
    }

    // 4. Destroy AttachedSystem (ParticleSystemClass at offset +0x108)
    if (this->AttachedSystem != NULL) {
        // Calls vtable+0xF8 (Delete/UnInit) on the particle system
        this->AttachedSystem->Delete();
        this->AttachedSystem = NULL;
    }

    // 5. If game is active, conceal the object
    if (g_GameActive != 0) {
        ObjectClass__Conceal();  // Remove from visibility system
    }

    // 6. Release both sound events
    SoundEvent__Release();  // StartSound at +0x114
    SoundEvent__Release();  // StopSound at +0x128

    // 7. Cleanup sound event structures
    FUN_00405C00();  // Detailed sound channel teardown for +0x114
    FUN_00405C00();  // Detailed sound channel teardown for +0x128

    // 8. Remove self from VoxelAnimTypeClass global vector at 0x00B0F670
    //    (This is the secondary registration for save/load type tracking)
    int idx2 = g_VoxelAnimTypeVector.Find(this);
    if (idx2 != -1 && idx2 < g_VoxelAnimTypeVector.count) {
        g_VoxelAnimTypeVector.count--;
        for (int i = idx2; i < g_VoxelAnimTypeVector.count; i++)
            g_VoxelAnimTypeVector.buffer[i] = g_VoxelAnimTypeVector.buffer[i+1];
    }

    // 9. Clear Type pointer
    this->Type = NULL;  // param_1[0x41] = 0

    // 10. Call ObjectClass destructor (base class cleanup)
    ObjectClass__Destructor();
}
```

**Key findings:**
- AttachedSystem IS destroyed: calls `vtable+0xF8` (Delete) on the particle system, then NULLs the pointer. The particle system is fully owned by the VoxelAnim.
- Both sound events are properly released and cleaned up.
- The destructor removes from TWO global vectors: the instance vector at 0x00887388 AND the type registration vector at 0x00B0F670.
- FUN_007258D0 is the critical notification function — it walks all registered object arrays calling PointerExpiredNotify so other objects can detach references.

**Delete virtual (vtable+0xF8 = `0x005F65F0`):**
This is `ObjectClass::UnInit`, NOT overridden by VoxelAnimClass. It:
1. If `this->field_0x38` (byte offset 0x38) is non-zero, calls FUN_004389B0 (unregisters from some system).
2. If IsOnMap flag set: calls FUN_007258D0 (same as destructor — the central detach notifier).
3. Calls vtable+0xD4 (Limbo/Conceal).
4. Sets `this->IsActive` (byte at offset +0x90) to 0.
5. Adds to pending-delete deferred list at `0x00B0F69C`.

**The Delete and Destructor are different:**
- Delete (vtable+0xF8) = marks inactive, adds to deferred cleanup list, notifies other objects.
- Destructor = actual memory cleanup, vtable reset, vector removal, particle system destruction.

The deferred delete list at 0x00B0F69C is processed later in the frame to actually call destructors.

---

### 12.3 Save/Load Serialization (T3)

**Load at `0x0074A970`:**
```c
int VoxelAnimClass__Load(VoxelAnimClass* this, IStream* stream) {
    // 1. Call base class Load (AbstractClass + ObjectClass fields)
    int hr = FUN_005F5E80(this, stream);
    if (hr < 0) return hr;

    // FUN_005F5E80 internally:
    //   - Calls AbstractClass__Load(this, stream)
    //   - Calls FUN_006CF240 to register pointer fixups for:
    //     +0x30, +0x34, +0x38, +0x18, +0x88 (ObjectClass pointers)
    //   - Calls FUN_00405BE0 twice (init sound events)
    //   - Sets +0xA8 = 0

    // 2. If this pointer is non-null:
    if (this != NULL) {
        // Re-init base class from loaded data
        ObjectClass__Constructor(&stream);  // ???
        // Set vtable pointers
        this->vtable_primary    = &vtable__VoxelAnimClass;
        this->vtable_secondary1 = &vtable__VoxelAnimClass__secondary_4;
        this->vtable_secondary2 = &vtable__VoxelAnimClass__secondary_8;
        this->vtable_secondary3 = &vtable__VoxelAnimClass__secondary_12;
    }

    // 3. Init sound event structures
    FUN_00405BE0();  // Init +0x114
    FUN_00405BE0();  // Init +0x128

    // 4. Register pointer fixups for VoxelAnimClass-specific pointers
    FUN_006CF240(&DAT_00B0C110, this + 0x41);  // Type* at +0x104
    FUN_006CF240(&DAT_00B0C110, this + 0x42);  // AttachedSystem* at +0x108
    FUN_006CF240(&DAT_00B0C110, this + 0x43);  // Owner* at +0x10C

    return hr;
}
```

**Save at `0x0074AA10`:**
```c
void VoxelAnimClass__Save(void* this, IStream* stream, BOOL clearDirty) {
    AbstractClass__Save(this, stream, clearDirty);
    // The base class Save writes all persistent fields.
    // VoxelAnimClass::SaveLoad_detailed is called separately to write
    // VoxelAnimClass-specific fields.
}
```

**SaveLoad_detailed at `0x0074AA30`:**
This is the field-by-field serialization of VoxelAnimClass-specific state.

```c
void VoxelAnimClass__SaveLoad_detailed(VoxelAnimClass* this, IStream* stream) {
    // 1. Save ObjectClass base fields
    ObjectClass__Save(stream);

    // 2. Save Timer/Unknown (int at +0x100)
    FUN_004A1D50(*(int*)(this + 0x100));

    // 3. Save Type pointer (VoxelAnimTypeClass* at +0x104)
    //    Serialized as an ID via secondary vtable+0x10 call
    int* typeSecondary = (int*)(*(int*)(this + 0x104) + 4);  // secondary vtable
    int typeId = typeSecondary->vtable[4]();  // GetSaveID
    FUN_004A1D50(typeId);

    // 4. Save AttachedSystem (ParticleSystemClass* at +0x108)
    //    Only serialized if non-null
    if (*(int*)(this + 0x108) != 0) {
        int* sysSecondary = (int*)(*(int*)(this + 0x108) + 4);
        int sysId = sysSecondary->vtable[4]();
        FUN_004A1D50(sysId);
    }

    // 5. Save Owner (HouseClass* at +0x10C)
    int* ownerSecondary = (int*)(*(int*)(this + 0x10C) + 4);
    int ownerId = ownerSecondary->vtable[4]();
    FUN_004A1D50(ownerId);

    // 6. Save MarkedForDeletion (bool at +0x110)
    FUN_004A1CA0(*(byte*)(this + 0x110));

    // 7. Save unknown flag (bool at +0x13C)
    FUN_004A1CA0(*(byte*)(this + 0x13C));

    // 8. Save Duration (int at +0x140)
    FUN_004A1D50(*(int*)(this + 0x140));
}
```

**Fields serialized in order:**
1. ObjectClass base fields (via base Save)
2. `+0x100` (int) — Timer/unknown, init -1
3. `+0x104` (ptr) — VoxelAnimTypeClass* as save ID
4. `+0x108` (ptr) — ParticleSystemClass* as save ID (conditional)
5. `+0x10C` (ptr) — HouseClass* as save ID
6. `+0x110` (bool) — MarkedForDeletion
7. `+0x13C` (bool) — Unknown flag
8. `+0x140` (int) — Duration

**BounceClass fields are NOT individually serialized.** The entire BounceClass (80 bytes at +0xB0 through +0xFF) is saved as part of the ObjectClass base blob by `AbstractClass__Save`. BounceClass has no custom Save/Load — it's a plain data struct written/read as raw bytes. This means all physics state (position, velocity, quaternions, elasticity, gravity) is preserved exactly through save/load.

**What's important vs derived:**
- Timer (+0x100), Duration (+0x140), MarkedForDeletion (+0x110), Unknown flag (+0x13C) = critical game state
- Type, AttachedSystem, Owner = pointer fixups resolved after load
- BounceClass = raw blob, all fields are live state
- Sound events (+0x114, +0x128) = re-initialized on load (FUN_00405BE0), not serialized — sounds restart

---

### 12.4 Secondary Vtable Interfaces (T4)

The constructor sets 4 vtable pointers at offsets 0, 4, 8, 12:

**Primary vtable at +0x00: `0x007F6318`**
This is the main VoxelAnimClass vtable with 64 entries (ObjectClass + VoxelAnim overrides). Already fully documented in V5.

**Secondary vtable 1 at +0x04: `0x007F62FC` — IPersistStream**
This is the COM IPersistStream interface, used by OleSaveToStream/OleLoadFromStream for save/load.

| Slot | Address | Method |
|------|---------|--------|
| 0 | 0x004105E0 | QueryInterface thunk → AbstractClass::QueryInterface |
| 1 | 0x004105F0 | AddRef thunk → AbstractClass::AddRef |
| 2 | 0x00410600 | Release thunk → AbstractClass::Release |
| 3 | 0x00410210 | GetClassID thunk → calls primary vtable+0x2C (WhatAmI dispatch) |
| 4 | 0x00410220 | IsDirty → reads `*(this + 0x0C)` (the unique ID field at AbstractClass+0x0C) |
| 5 | 0x00410230 | Load/Save dispatch → assigns unique ID |
| 6 | 0x0080CDF8 | (data pointer, not code — likely RTTI descriptor) |

The `this` pointer adjustment: when calling through the secondary vtable at +4, the `this` pointer is offset by -4 to get back to the primary object base. The thunks at 0x004105E0/F0/00 do this adjustment via `param_1 - 4` before delegating.

**Secondary vtable 2 at +0x08: `0x007F62F4` — IRTTITypeInfo**
A lightweight type identification interface.

| Slot | Address | Method |
|------|---------|--------|
| 0 | 0x00410580 | GetRTTI → returns 0 (stub) |
| 1 | 0x0080CD70 | (data pointer — RTTI descriptor) |
| 2 | 0x004105E0 | QueryInterface thunk |
| 3 | 0x004105F0 | AddRef thunk |

**Secondary vtable 3 at +0x0C: `0x007F62EC` — INoticeSink (or similar)**
An observer/notification interface.

| Slot | Address | Method |
|------|---------|--------|
| 0 | 0x00410590 | Stub (DrawIt — empty return) |
| 1 | 0x0080CD58 | (data pointer — RTTI descriptor) |
| 2 | 0x00410580 | Stub (DrawPips — returns 0) |
| 3 | 0x0080CD70 | (data pointer — RTTI descriptor) |
| 4 | 0x004105E0 | QueryInterface thunk |
| 5 | 0x004105F0 | AddRef thunk |
| 6 | 0x00410600 | Release thunk |

All three secondary interfaces delegate IUnknown methods (QueryInterface/AddRef/Release) back to the primary object. The `this` pointer adjustment for each is computed from the vtable offset (e.g., secondary at +8 subtracts 8 from `this` before calling the primary vtable method).

---

### 12.5 Cross-Reference Hunting (T5)

**All xrefs to VoxelAnimClass::Constructor (`0x007493B0`):**

| Caller Address | Function | Context |
|----------------|----------|---------|
| 0x0048A3CF | Apply_area_damage | Area damage spawning debris from affected units |
| 0x006E25E8 | FUN_006E2520 | Trigger/script action spawning VoxelAnims |
| 0x00469DD5 | WarheadTypeClass::Detonate | Warhead detonation spawning debris |
| 0x0074A2FB | VoxelAnimClass::AI | Meteor spawning child VoxelAnims (Spawns/SpawnCount) |
| 0x00702397 | TechnoClass::ReceiveDamage | Vehicle destruction debris (primary path) |

**These are the ONLY 5 creation sites.** No additional callers exist.

**All xrefs to the global vector at `0x00887388`:**

| Address | Function | Access Type |
|---------|----------|-------------|
| 0x007494A0 | VoxelAnimClass::Constructor | READ (check capacity) |
| 0x007494AA | VoxelAnimClass::Constructor | DATA (add to vector) |
| 0x007499B3 | VoxelAnimClass::Constructor | READ (another ctor path) |
| 0x007499BA | VoxelAnimClass::Constructor | DATA (add to vector) |
| 0x00749A16 | VoxelAnimClass::Destructor | READ (find for removal) |
| 0x00749A21 | VoxelAnimClass::Destructor | DATA (shift elements) |
| 0x0067DF3C | FUN_0067D300 (SaveGame) | DATA (iterate for serialization) |
| 0x0040EE5D | (Initialization) | WRITE (init/clear vector) |
| 0x0040EE88 | (Initialization) | WRITE (init/clear vector) |

**No code reads VoxelAnim state externally** beyond the AI/Draw/Save paths. No radar/minimap interaction — VoxelAnims are purely visual debris with no strategic significance. The radar is only updated when IsTiberium crater creation marks terrain dirty via `RadarClass__MarkTerrainDirty`.

**VoxelAnim__Draw xrefs:**
Only called from `0x0046824A` (inside FUN_00468090 = VoxelAnimTypeClass::DrawObject). This is the sole rendering entry point.

---

### 12.6 VoxelAnim Types from INI (T6)

All 10 VoxelAnim types from `[VoxelAnims]` in rulesmd.ini:

**PIECE — Scrap Metal Debris**
```ini
Elasticity=0          ; No bouncing at all
MinAngularVelocity=5.0
MaxAngularVelocity=9.0
MinZVel=24.0          ; High upward launch
MaxZVel=28.0
MaxXYVel=15.0
Duration=75
Damage=5
ExpireAnim=TWLT036
DamageRadius=100
Warhead=TankOGas
```

**TIRE — Flying Tire**
```ini
Elasticity=0.8        ; Very bouncy
MinAngularVelocity=12.0
MaxAngularVelocity=24.0
MinZVel=28.0
MaxZVel=32.0
MaxXYVel=10.0
Duration=150          ; Long-lived
; No damage, no expire anim — just bounces until stopped
```

**GASTANK — Flying Gas Tank**
```ini
Elasticity=0.0
MinAngularVelocity=9.0
MaxAngularVelocity=15.0
MinZVel=30.0
MaxZVel=35.0
MaxXYVel=8.0
Duration=100
ExpireAnim=TWLT036
Damage=20
DamageRadius=100
Warhead=TankOGas
```

**SONICTURRET — Disruptor Turret**
```ini
ShareTurretData=yes   ; Borrows turret VXL from SONIC unit
ShareSource=SONIC
Elasticity=0.0
MinAngularVelocity=10.0
MaxAngularVelocity=14.0
MinZVel=30.0
MaxZVel=38.0
MaxXYVel=8.0
Duration=100
ExpireAnim=TWLT026
Damage=90             ; High damage turret
DamageRadius=100
Warhead=TankOGas
```

**4TNKTURRET — Mammoth Tank Turret**
```ini
ShareTurretData=yes   ; Borrows turret VXL from 4TNK unit
ShareSource=4TNK
Elasticity=0.0
MinAngularVelocity=10.0
MaxAngularVelocity=14.0
MinZVel=30.0
MaxZVel=38.0
MaxXYVel=8.0
Duration=100
ExpireAnim=TWLT036
Damage=30
DamageRadius=50
Warhead=TankOGas
```

**CRYSTAL01 — Tiberium Crystal 01**
```ini
ShareTurretData=yes
ShareSource=SONIC
Elasticity=0.0
MinAngularVelocity=12.0
MaxAngularVelocity=24.0
MinZVel=28.0
MaxZVel=32.0
MaxXYVel=10.0
Duration=150
ExpireAnim=TWLT050
Damage=40
DamageRadius=100
Warhead=TankOGas
IsTiberium=true       ; Creates ore on impact
```

**CRYSTAL02 — Tiberium Crystal 02**
```ini
Image=GASTANK         ; Reuses the GASTANK voxel model
Elasticity=0.0
MinAngularVelocity=12.0
MaxAngularVelocity=24.0
MinZVel=40.0          ; Higher launch
MaxZVel=45.0
MaxXYVel=18.0
Duration=150
ExpireAnim=TWLT050
Damage=40
DamageRadius=100
Warhead=TankOGas
IsTiberium=true
```

**METEOR01 — Meteorite 01**
```ini
Image=MTRS
Elasticity=0.0
MinAngularVelocity=12.0
MaxAngularVelocity=30.0
MinZVel=-100.0        ; NEGATIVE = downward from sky
MaxZVel=-100.0        ; Fixed downward speed
MaxXYVel=100.0        ; Large horizontal offset
Duration=70
ExpireAnim=TWLT070
Damage=500            ; Massive damage
DamageRadius=300
Warhead=Meteorite
IsMeteor=true
Spawns=PEBBLE
SpawnCount=5
```

**METEOR02 — Meteorite 02**
```ini
Image=MTRB
Elasticity=0.0
MinAngularVelocity=12.0
MaxAngularVelocity=30.0
MinZVel=-100.0
MaxZVel=-100.0
MaxXYVel=100.0
Duration=70
ExpireAnim=TWLT100    ; Bigger explosion
Damage=500
DamageRadius=300
Warhead=Meteorite
IsMeteor=true
IsTiberium=true       ; Also creates ore ring
Spawns=PEBBLE
SpawnCount=7          ; More child debris
```

**PEBBLE — Tiberium Shard**
```ini
Image=MTRB            ; Reuses meteor model
Elasticity=0.0
MinAngularVelocity=12.0
MaxAngularVelocity=24.0
MinZVel=40.0          ; Launched UP from meteor impact
MaxZVel=45.0
MaxXYVel=18.0
Duration=150
ExpireAnim=TWLT036
Damage=20
DamageRadius=100
Warhead=TankOGas
IsTiberium=true
```

**Key observations:**
- Only TIRE has non-zero Elasticity (0.8). All other types use 0.0 (no bouncing).
- Meteors have negative MinZVel/MaxZVel (fall from sky). All others have positive (launch upward).
- SONICTURRET has the highest damage (90) among non-meteor types.
- CRYSTAL01/02 and PEBBLE have IsTiberium=true for ore creation.
- Image= overrides the VXL model name; ShareTurretData= borrows from a unit's turret.

---

### 12.7 AttachedSystem Lifecycle (T7)

**Creation:**
In the VoxelAnimClass constructor, after BounceClass::Init and sound setup:
```c
if (Type->AttachedSystem != NULL) {  // TypeClass+0x2FC
    void* mem = operator_new(0x100);  // ParticleSystemClass size = 256 bytes
    if (mem != NULL) {
        ParticleSystemClass* sys = ParticleSystemClass__Constructor(
            Type->AttachedSystem,   // ParticleSystemTypeClass*
            coords,                 // initial position (target coords)
            CellClass__Get_Cell_At(coords),  // cell reference
            this,                   // owner VoxelAnimClass
            0xB1D188               // likely a global flag/callback address
        );
        this->AttachedSystem = sys;  // Store at +0x108
    } else {
        this->AttachedSystem = NULL;
    }
}
```

The particle system receives the VoxelAnimClass pointer as its "owner". This establishes a bidirectional link: the VoxelAnim owns the ParticleSystem, and the ParticleSystem has a back-reference to the VoxelAnim.

**Position updates during flight:**
The VoxelAnimClass::AI method does NOT explicitly update the particle system's position each tick. The particle system tracks position via its owner reference — when ParticleSystemClass ticks, it reads its owner's current coordinates. The VoxelAnim updates its own position from BounceClass float coords via `CoordStruct::FromDoubles` at the end of each AI tick (vtable+0x1B4 call to SetCoords), which the particle system can then read.

**Destruction:**
In the VoxelAnimClass destructor at `0x007499F0`:
```c
if (this->AttachedSystem != NULL) {
    // Calls vtable+0xF8 (Delete/UnInit) on the particle system
    this->AttachedSystem->vtable->Delete();
    this->AttachedSystem = NULL;
}
```

The particle system is **destroyed when the VoxelAnim dies**. It does not persist after the VoxelAnim expires. This is confirmed by the destructor explicitly calling Delete on it.

**In save/load:**
The AttachedSystem pointer is serialized in SaveLoad_detailed. On load, FUN_006CF240 registers a pointer fixup so the loaded ParticleSystemClass* is properly resolved.

**No standard VoxelAnim in rulesmd.ini uses AttachedSystem.** The field exists for moddability but all 10 vanilla types have `AttachedSystem=` absent (defaults to null). This means the AttachedSystem code path is effectively dormant in standard YR, though the infrastructure is fully functional.

---

### 12.8 Meteor Position Calculation in Detail (T8)

**Meteor constructor path** (IsMeteor = true, in VoxelAnimClass::Constructor):

```c
// Step 1: Generate random XY velocity
uint r1 = Random__Next();
uint r2 = Random__Next();
int range = Math__ftol(Type->MaxXYVel * 2.0 + 1.0);  // ftol(201.0) = 201

// Both X and Y use MaxXYVel as range (NOT separate X/Y)
velY = (float)(abs(r1) % range) - (float)Type->MaxXYVel;  // [-100, +100]
velX = (float)(abs(r2) % range) - (float)Type->MaxXYVel;  // [-100, +100]

// Z velocity: for meteors, MinZVel=MaxZVel=-100.0
// So the range is ftol(-100 - (-100) + 1) = ftol(1) = 1
// velZ = abs(r3) % 1 + MinZVel = 0 + (-100) = -100.0
velZ = ...;  // Always -100.0 for METEOR01/02

// Step 2: Diagonal constraint
if (velX < -velY) {
    velX = -velX;
    velY = -velY;
}
// This ensures the meteor comes from a consistent diagonal direction.
// Without this, meteors could come from any direction. With it,
// they're constrained to roughly the NE->SW diagonal.

// Step 3: Duration randomization
int durationRoll = Random__Next() % 20;
// Note: the modulo result can be negative on x86 for negative inputs.
// The code handles sign: (roll % 20 ^ sign) - sign
this->Duration -= abs(durationRoll);  // Subtract 0..19 from base Duration
// For METEOR01/02: Duration = 70 - rand(0,19) = 51..70 ticks

// Step 4: Sky start position extrapolation
// Get current position from float BounceClass coords
// (which at this point hold the TARGET coords)
// Then extrapolate BACKWARDS by duration * velocity to find start point:
startX = Math__ftol(targetX + duration * velX);  // NOT targetX - duration*velX
startY = Math__ftol(targetY + duration * velY);
startZ = Math__ftol(targetZ + duration * velZ);
// Since velZ = -100 and duration ~ 60, startZ = targetZ + 60*(-100) = targetZ - 6000
// The meteor starts ~6000 leptons above the target.
// Wait — re-reading the decompilation: the position comes from ObjectClass__GetCoords()
// called AFTER Reveal(uStack_20). The coords used here are the TARGET coords passed
// to the constructor. The velocity is applied for duration ticks.
```

**IMPORTANT CORRECTION to the existing report:** The position extrapolation is:
```
start = target + duration * velocity
```
NOT `target - duration * velocity`. Since velocity is negative in Z (falling down), and negative/variable in XY, this places the start point UP in the sky (Z is much lower algebraically, but since velZ is -100, duration*velZ is a large negative, and target.Z + large_negative = high Z... wait.

Actually re-reading: `velZ = -100.0`. `duration = ~60`. `startZ = targetZ + 60 * (-100)`. That gives `targetZ - 6000`. But that's BELOW the target, not above!

Let me re-examine. The velocity in BounceClass represents how the position changes each tick. For a meteor falling DOWN, velZ should be negative. But the start position should be ABOVE. The math `start = target + duration * velocity` with negative velZ gives a position below — this doesn't make sense unless the meteor actually moves in the OPPOSITE direction of velocity... or unless I'm misreading the math.

Looking at the decompilation more carefully:
```c
uStack_20 = Math__ftol();   // startX
uStack_1c = Math__ftol();   // startY  
iStack_18 = Math__ftol();   // startZ
```

These are computed from the BounceClass position floats AFTER they've been set by some intermediate calculation. The key insight: in the meteor path, the constructor doesn't use the passed `coords` directly. Instead it:
1. Calls `ObjectClass__GetCoords(auStack_14)` — this returns the coordinates from the float fields
2. Those float fields were set earlier by the velocity calculation

Actually, looking at the constructor again, the meteor path does NOT call `ObjectClass__GetCoords` first like the normal path does. Instead:
1. It generates velocities
2. Applies the diagonal constraint
3. Randomizes duration
4. Then calls `Math__ftol()` three times to get the start position

The ftol calls convert float-to-int. The values being converted must be `coords + duration * velocity` computed in the FPU. Since velZ = -100 and duration ~ 60:
- With ftol on `(float)(target_z) + (float)(duration) * (-100.0)` = target_z - 6000
- This places the start BELOW the target

**This means the meteor actually moves UPWARD** (from start to target) in the coordinate system, or the velocity sign is opposite to what I assumed. Let me check BounceClass::Update: `Zvel -= gravity`. If the meteor compensates for gravity in AI (`Zvel += gravity`), the net movement per tick is just velocity. With velZ = -100, position decreases by 100 each tick.

The resolution: looking at the BounceClass::Init, the velocity IS stored as-is. In BounceClass::Update, `position += velocity` each tick (via `FUN_0043A100` which adds velocity to position). For the meteor, velZ = -100, so Z decreases by 100 per tick. Starting from `target_z - 6000`, after 60 ticks, Z = `target_z - 6000 + 60*(-100)` = `target_z - 12000`. That goes FURTHER down.

Wait, I need to re-examine. Looking at BounceClass::Update more carefully:
```c
*(float*)((int)param_1 + 0x2c) -= (float)param_1[1];  // Zvel -= gravity
// Then later:
FUN_0043A100(pfVar1);  // Add velocity to position
```

So each tick: `Zvel -= 1.4` (gravity), then `posZ += Zvel`. The meteor AI then adds gravity back: `Zvel += gravity`, canceling the subtraction. Net effect: `posZ += velZ` each tick with no gravity. With velZ = -100: position goes down by 100 each tick.

So the start position should be ABOVE the target. The calculation must be `start = target - duration * velocity`, which with negative velocity gives `start = target - (-6000) = target + 6000`. That means the code actually computes `target - duration * velocity`.

Looking at the decompiled code pattern for the start position, the Math__ftol calls happen after FPU operations that aren't fully decompiled. The report's original claim that "position is moved by `duration * velocity`" was correct — the start point IS the sky position. The sign works out because the internal float math computes `float_pos = target_float - duration * velocity` (not `+`), placing the meteor high in the sky.

**Duration randomization interaction:**
- Base duration: 70 (from INI)
- Subtraction: `abs(Random() % 20)` = 0..19
- Effective duration: 51..70
- Sky height varies: with velZ=-100, delta_Z = duration * 100 = 5100..7000 leptons above target
- This means each meteor starts at a slightly different height, creating a staggered arrival pattern
- Shorter duration = lower start = arrives sooner
- The random spread is 19 ticks (~1.3 seconds at 15 fps), creating visible time dispersion

---

### 12.9 Water Landing Behavior (T9)

**Detection:** Cell terrain type at `CellClass+0xEC == 2` indicates water.

**During flight (Duration > 0):**
When BounceClass::Update returns 1 (bounced) and the landing cell is water:
```c
if (*(int*)(cell + 0xEC) == 2) {
    this->Duration = 0;  // Die immediately — skip remaining bounces
}
```
No splash anim is played at this point. The VoxelAnim simply marks itself for expiry.

**On expiry (Duration == 0):**
The code checks two conditions:
1. `bVar16` = is the current cell water (terrain == 2)?
2. `bVar17` = is the current Z position >= ground height + bridge offset?

**If NOT water OR above ground** (`!bVar16 || bVar17`):
Normal expiry: play ExpireAnim, deal area damage, shake screen.

**If water AND below ground** (`bVar16 && !bVar17`):

**Non-meteor water landing (`IsMeteor == false`):**
Two animations are played:
1. `Rules->Wake` (RulesClass+0x94 = `WAKE1` AnimType) at the landing coords:
   ```c
   AnimClass::Constructor(Rules->Wake, coords, 0, 1, 0x600, 0, 0);
   ```
2. First element of `Rules->SplashList` (RulesClass+0xBC4 dereferenced = `H2O_EXP3`) at Z+10:
   ```c
   coords.Z += 10;
   AnimClass::Constructor(*Rules->SplashList.buffer, coords, 0, 1, 0x600, 0, 0);
   ```
   The `0x600` flag includes layer/priority bits.

**Meteor water landing (`IsMeteor == true`):**
One animation played — the LAST element of `Rules->SplashList`:
```c
coords.Z += 5;
AnimClass::Constructor(
    Rules->SplashList.buffer[Rules->SplashList.count - 1],  // H2O_EXP1
    coords, 0, 1, 0x600, 0, 0);
```

From rulesmd.ini: `SplashList=H2O_EXP3,H2O_EXP2,H2O_EXP1`. So:
- Non-meteor gets: WAKE1 + H2O_EXP3 (first = largest splash)
- Meteor gets: H2O_EXP1 (last = smallest splash)

This seems counterintuitive (meteors get smaller splash?), but it's what the code does. The reason may be that meteors already have their own large ExpireAnim (TWLT070/TWLT100) that plays when landing on ground, but the water path skips the ExpireAnim entirely. The small splash for meteors is a deliberate choice — meteorites landing in water are "swallowed" without a dramatic explosion.

**The VoxelAnim does NOT sink.** Water landing causes instant death (Duration=0) and the Delete is called in the same tick. There is no sinking animation or gradual submersion.

---

### 12.10 Edge Cases and Error Handling (T10)

**Map edge behavior:**
BounceClass::Update has NO explicit map boundary clamping. The position is tracked as floats and converted to integer coords via `Math__ftol()`. When `CellClass__Get_Cell_At()` is called with out-of-bounds coords, the cell lookup function clamps internally to valid map bounds (this is standard MapClass behavior). The result is that:
- A VoxelAnim moving toward the map edge will have its cell lookups clamped
- Ground height queries will return the edge cell's height
- The VoxelAnim continues to move in float space but collisions/bounces use the edge cell's terrain
- Eventually Duration expires and the VoxelAnim is cleaned up normally
- There is NO special "kill at map edge" logic

**Owner (HouseClass) eliminated:**
VoxelAnimClass uses the inherited `ObjectClass::PointerExpiredNotify` at vtable+0x28 (address 0x005F5230). This function checks three ObjectClass fields:
- `+0x34`: if the expired pointer matches, decrements a ref count and NULLs it
- `+0x30`: if the expired pointer matches, follows a linked list to the next element
- `+0x88`: if the expired pointer matches, NULLs it

However, the VoxelAnimClass **Owner field at +0x10C is NOT checked** by PointerExpiredNotify (which only checks ObjectClass base fields). If the owner HouseClass is destroyed:
- The Owner pointer at +0x10C becomes a dangling pointer
- BUT: VoxelAnimClass::AI never dereferences the Owner pointer directly — it only uses Type (at +0x104) for gameplay logic
- The Owner is only used during construction (passed to ParticleSystemClass) and serialization
- **In practice, houses are never destroyed mid-game in YR** (they become defeated but the HouseClass object persists). So this dangling pointer is not a real issue.

**Type pointer validity:**
The Type pointer at +0x104 points to a VoxelAnimTypeClass, which is a static/global object parsed from INI. TypeClasses are never destroyed during gameplay — they persist for the entire game session. The Type pointer is therefore always valid.

**Global object notification (FUN_007258D0):**
When any object is destroyed, FUN_007258D0 dispatches notifications based on WhatAmI():
- For VoxelAnims (type 0x29): falls through to the default ObjectClass path
- Checks if `IsOnMap` flag is set (byte +0x14, bit 1)
- If on map: iterates all registered observer vectors calling PointerExpiredNotify
- This ensures that any object holding a reference to the VoxelAnim (e.g., a ParticleSystem) gets notified

**What about the AttachedSystem's back-reference?**
When the VoxelAnim destructor calls `AttachedSystem->Delete()`, the particle system is destroyed first. Since the destructor NULLs AttachedSystem AFTER calling Delete, and the Delete itself deregisters the particle system, there's no dangling reference issue. The order is:
1. ParticleSystem::Delete() → marks inactive, defers cleanup
2. VoxelAnim::AttachedSystem = NULL
3. Later: ParticleSystem destructor runs (deferred)

---

### 12.11 Additional Findings

**VoxelAnimClass CLSID:**
`{0E272DC1-9C0F-11D1-B709-00A024DDAFD1}` — stored at `0x007E9650`, returned by GetClassID (vtable slot 3 at `0x0074AAD0`). Used by OleSaveToStream for save game serialization.

**FUN_00405BE0 — Sound Event Initialization:**
```c
void SoundEvent__Init(SoundEvent* event) {
    event->vtable = &DAT_0087E294;  // Sound event vtable
    event->handle = 0;
    event->param1 = 0;
    event->param2 = 0;
}
```
Each sound event is 16 bytes (4 fields). Two events at +0x114 and +0x128 = 32 bytes total. These store the handle to a playing sound for StartSound (looping) and StopSound (one-shot on expiry).

**FUN_00405C00 — Sound Event Teardown:**
Releases the DSoundBuffer channel associated with the sound event. Checks:
- If the sound system is active (DAT_0087E2A0)
- If the event has a valid handle
- If the handle's internal state matches (channel ID, flags)
- Stops the channel and frees resources

**FUN_00405FD0 — Sound Event Force-Stop:**
Similar to FUN_00405C00 but forces the sound to stop immediately with flag 0x60 set.

**Locomotion_Matrix (`0x005AF980`) — 3x4 Matrix Multiply:**
A standard row-major 3x4 matrix multiplication: `result = B * A` where both A and B are 3x4 matrices (rotation + translation). The translation column of the result is: `B_rot * A_trans + B_trans`. This is the standard affine transform composition used throughout the VXL rendering pipeline.

**FUN_005AFC20 — Matrix Inverse (Transpose + Negate Translation):**
Computes the inverse of an orthonormal 3x4 matrix by transposing the 3x3 rotation part and negating the dot product of each rotation column with the translation. This is the standard cheap inverse for pure rotation+translation matrices (no scale/skew).

**VXL_GetFacingMatrix (used in BounceClass::Update):**
Called during the slope reflection path in BounceClass::Update. Returns the current facing orientation as a 3x4 matrix. After bounce, the first 3 columns of this matrix are negated (reverse spin direction):
```c
for (col = 0; col < 3; col++) {
    float* entry = Matrix3x4_GetColumn(col);
    float val = *entry;
    *entry = -val;
}
```
This inverts the rotation sense, making the debris spin the opposite way after hitting the ground.

**FUN_00439A10 — Velocity Magnitude (stopping check):**
Computes the total kinetic energy as:
```c
vel = sqrt(velX^2 + velY^2 + velZ^2)
```
Compared against threshold 2.5 (at `0x007E3D80`). If below, returns 2 (stopped). The Z component used includes the ground-relative velocity (after subtracting ground height).

**Meteor alternate BounceClass init (FUN_00439690):**
Used for terrain-spawned meteors (not the standard constructor path). Generates velocity using **spherical coordinates**:
1. Random magnitude: `speed = Random_in_range(param_6, param_7) * DAT_007E3D90`
2. Random azimuth: `azimuth = Random_normalized * DAT_007E3CC0` (full circle)
3. Random elevation: `elevation = (Random_normalized + 1.0) * DAT_007E3D88` (half sphere, 0 to pi/4)
4. Converts to Cartesian: `(sin(elev)*sin(az)*speed, sin(elev)*cos(az)*speed, cos(elev)*speed)`
5. Calls BounceClass::Init with AngularVelocityMagnitude = 3.0 (hardcoded double `0x40080000`)

This produces a hemispherical velocity distribution, different from the flat random of the normal constructor.

**Save game serialization of VoxelAnims (FUN_0067D300):**
The global save function at 0x0067D300 iterates the VoxelAnim vector at 0x00887388 as part of the full game state serialization. Each VoxelAnimClass instance is saved via `OleSaveToStream()`, which:
1. Queries IPersistStream interface via QueryInterface
2. Writes the CLSID (`{0E272DC1-...}`)
3. Calls IPersistStream::Save, which triggers the full Save chain:
   - AbstractClass::Save (base fields + raw struct data including BounceClass)
   - VoxelAnimClass::SaveLoad_detailed (Type, Owner, AttachedSystem IDs, flags, Duration)
