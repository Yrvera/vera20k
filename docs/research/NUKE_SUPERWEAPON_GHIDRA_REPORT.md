# Nuke Superweapon (MultiMissile) — Ghidra Research Report

**Primary Addresses:**
- `SuperClass::Launch` case 0 — `0x006CC390` (switch on SWType+0xB4)
- `FUN_0046b050` (BulletClass::Allocate) — `0x0046B050`
- `FUN_004664c0` (BulletClass::Init) — `0x004664C0`
- `FUN_0046b260` (BulletClass::SetOwner) — `0x0046B260`
- `FUN_00468670` (BulletClass::Fire) — `0x00468670` (BulletClass vtable offset 0x1F0)
- `BuildingClass::CreateFireAnim` — `0x0043B5E0`
- `BulletClass::AI` — `0x004666E0`
- `FUN_00468d80` (BulletClass::BulletDetonation) — `0x00468D80`
- `WarheadTypeClass::Detonate` — `0x004690B0`
- `FUN_0046b310` (NukeMaker::SpawnDownwardNuke) — `0x0046B310`
- `FUN_006e3410` (alternate nuke-down launcher) — `0x006E3410`
- `FUN_0053ab70` (ScreenNukeFlash) — `0x0053AB70`
- `FUN_004251f0` (NukeGroundZero — applies NukeWarhead area damage) — `0x004251F0`
- `RulesClass::ReadSpecialWeapons` — `0x00669060`
- `WarheadTypeClass::ReadINI` — `0x0075D590`
- `WeaponTypeClass::ReadINI` — `0x00772080`

**Confidence:** HIGH (all functions decompiled from binary, cross-referenced with INI)
**Active in YR:** Yes — core gameplay superweapon

---

## 1. Overview: Complete Nuke Chain

The nuclear missile is a two-phase superweapon with a complex chain of events. Two
INI weapons drive the sequence:

```ini
[NukeCarrier]           ; Phase 1: upward missile
Projectile=GiantNukeUp
Speed=100
Warhead=NukeMaker       ; NukeMaker=yes triggers the downward phase

[NukePayload]           ; Phase 2: downward missile + detonation
Damage=600
Range=30
Projectile=GiantNukeDown
Speed=10
RadLevel=500
Warhead=NUKE            ; The actual damage warhead
```

### Full Event Chain

```
Player clicks target cell
    |
    v
SuperClass::Launch (case 0: MultiMissile)
    |
    +-- [Has AuxBuilding (NukeSilo)?]
    |       |
    |   YES (Path B): Find NukeSilo building
    |       -> Open silo door animation
    |       -> Store target cell at HouseClass+0x5784
    |       -> Building AI fires missile when door anim completes
    |       |
    |   NO (Path A): Direct launch
    |
    v
Create BulletClass with:
    - Projectile = GiantNukeUp (from NukeCarrier weapon, found via "NukePayload" lookup)
    - Warhead = NukeMaker
    - Source = {targetX, targetY, groundZ}
    - Target = {targetX, targetY, groundZ + 20000}
    - Velocity = nearly vertical (spherical angle ~pi/2, speed = -100)
    |
    +-- Play NukeTakeOff anim (NUKETO) at building
    +-- Play EVA voice + launch sound
    |
    v
BulletClass::AI per-tick movement
    - GiantNukeUp has Vertical=yes, DetonationAltitude=20000
    - Missile flies upward until reaching altitude
    |
    v
Carrier missile detonation
    -> FUN_00468d80 (BulletDetonation)
        -> WarheadTypeClass::Detonate
            -> Checks NukeMaker flag (WarheadTypeClass+0x176)
            -> NukeMaker=yes: calls FUN_0046b310
    |
    v
FUN_0046b310 (SpawnDownwardNuke):
    - Looks up "NukePayload" weapon from global WeaponTypeClass array
    - Creates NEW BulletClass with:
        - Projectile = GiantNukeDown (NukePayload.Projectile)
        - Warhead = NUKE (NukePayload.Warhead)
        - Speed = 10 (NukePayload.Speed)
        - Damage = 600 (NukePayload.Damage)
        - Target = original player-selected cell (from HouseClass nuke target)
    - Velocity = nearly vertical downward (spherical angle ~pi/2, speed = -100)
    |
    v
Downward nuke missile (GiantNukeDown) flies down
    - GiantNukeDown has Vertical=yes, DetonationAltitude=30000
    |
    v
BulletClass::AI detonation
    - FUN_00410a40 checks: does warhead name == "NUKE"?
    - YES: Special nuke detonation path
        |
        v
    1. FUN_0053ab70 — Screen nuke flash (white screen, 30 frames)
    2. CreateRadarEvent at impact point
    3. Create NUKEBALL anim at impact coordinates
    4. Store bullet in global nuke-detonation tracking array
    5. Skip normal BulletDetonation
        |
        v
    6. FUN_004251f0 (NukeGroundZero):
       Apply_area_damage(cell, Rules.NukeWarhead, 0, 0)
       -> Uses the "Nuke" warhead from [SpecialWeapons] NukeWarhead=
       -> Applies area damage + radiation at ground zero
```

---

## 2. SuperClass::Launch — Case 0 (MultiMissile)

**Address:** `0x006CC390`, case 0 of switch on `*(int*)(SuperWeaponTypeClass + 0xB4)`

### Preconditions

SuperClass has three state bytes that determine the launch path:
- `SuperClass+0x6F` — overall readiness flag (must be nonzero to launch at all)
- `SuperClass+0x6E` — secondary readiness flag
- `SuperClass+0x6D` — tertiary readiness flag

### Path A: Direct Launch (all three flags set)

**Condition:** `+0x6F != 0 && +0x6E != 0 && +0x6D != 0`

This executes when there is no NukeSilo building requirement, or when the silo
has already opened its door and is now actually firing the missile.

1. **Parse target coordinates from param_2:**
   ```c
   packed_cell = *param_2;  // packed as (cellY << 16) | cellX
   cellX = (short)(packed_cell & 0xFFFF);
   cellY = (short)(packed_cell >> 16);
   targetX = cellX * 256 + 128;   // center of cell in leptons
   targetY = cellY * 256 + 128;
   ```

2. **Get ground height:**
   ```c
   groundZ = CellClass::GetGroundHeight(targetX, targetY);
   cell = MapClass::Get_CellClass(packed_cell);
   if (cell->flags & 0x100)  // bridge flag
       groundZ += g_BridgeHeight;  // DAT_00b0c07c
   ```

3. **Look up "NukePayload" weapon:**
   ```c
   idx = FUN_00773030("NukePayload");  // search WeaponTypeClass array by name
   weapon = WeaponTypeClass_Array[idx];
   projectile = weapon+0xA0;  // BulletTypeClass* (GiantNukeUp... actually this is
                               // the NukePayload weapon's projectile = GiantNukeDown)
   ```
   
   **Important correction:** The code actually finds the NukePayload weapon and reads
   its Projectile field. But for the carrier missile, the code uses the
   `SuperWeaponTypeClass+0x9C` WeaponType reference (NukeCarrier), not NukePayload.
   The "NukePayload" lookup string at `0x0081AFA0` is referenced from `SuperClass::Launch`
   at address `0x006CDAF9`.

4. **Set source and target coords:**
   ```c
   source = {targetX, targetY, groundZ};
   target = {targetX, targetY, groundZ + 20000};  // 20000 leptons above ground
   ```

5. **Create carrier missile:**
   ```c
   bullet = FUN_0046b050(projectile, source, target, warhead, speed, flags);
   FUN_0046b260(bullet, housePtr);  // set owner at bullet+0x130
   ```

6. **Play NukeTakeOff anim:**
   ```c
   BuildingClass::CreateFireAnim();  // creates NUKETO anim at building location
   ```

7. **Calculate velocity and fire:** (see Section 4)

8. **Post-launch:**
   ```c
   if (!is_multiplayer)
       VoxClass::PlayEVA();  // "Nuclear missile launched"
   VocClass::PlayAtCoord();  // launch sound effect
   
   HouseClass+0x1FC = 1;  // mark superweapon as having been launched
   ```

### Path B: AuxBuilding Door-Open Phase

**Condition:** `+0x6F != 0` but either `+0x6E == 0` or `+0x6D == 0`

This path searches for the NukeSilo building and initiates the door-open sequence:

1. **Iterate all BuildingTypeClass entries:**
   ```c
   for (i = 0; i < BuildingTypeClass_Count; i++) {
       bldgType = BuildingTypeClass_Array[i];
       if (bldgType+0x16BA == 0)  continue;  // skip if no HasSuperWeapon flag
       
       // Check if this building type's super weapon matches
       swWeaponType = SuperWeaponTypeClass+0x98;  // WeaponType from SW definition
       if (bldgType+0x16F0 == swWeaponType || bldgType+0x16F4 == swWeaponType) {
           building = HouseClass::Find_Building_Of_Type(bldgType);
           if (building != NULL) {
               // ... proceed with door open
           }
           break;
       }
   }
   ```

2. **Open silo door:**
   ```c
   building->vtable[0x1E8]();  // Open door / start special anim
   building->vtable[0x1EC]();  // Set mission state / begin sequence
   ```

3. **Store nuke target info:**
   ```c
   HouseClass+0x5784 = *param_2;  // store target cell (packed coords)
   building[0x17E] = SuperWeaponTypeClass.Type;  // store SW type index (0 = MultiMissile)
   ```
   Building offset 0x17E (dword-indexed) = byte offset 0x5F8.

4. **Play sounds:**
   ```c
   VocClass::PlayAtCoord();  // launch sound
   if (!is_multiplayer)
       VoxClass::PlayEVA();  // EVA announcement
   ```

After the silo door animation completes, the building's AI eventually calls back
into the direct launch path, creating the actual carrier missile.

---

## 3. BulletClass Internals

### BulletClass::Allocate — FUN_0046b050
**Address:** `0x0046B050`

Uses COM to create a BulletClass instance:
```c
HRESULT hr = CoCreateInstance(
    &CLSID_BulletClass,  // {0E2D2DC9-9C0F-11D1-B709-00A024DDAFD1} at 0x007E96E0
    NULL, CLSCTX_ALL,
    &IID_BulletClass,     // at 0x007F7C90
    &bullet
);
if (FAILED(hr)) return NULL;
FUN_004664c0(bullet, ...);  // Initialize
return bullet;
```

### BulletClass::Init — FUN_004664c0
**Address:** `0x004664C0`

Initializes bullet fields (param_1 type is `int`, so offsets are direct bytes):

| Offset | Field | Source |
|--------|-------|--------|
| 0x10C | Source coords? | param_3 |
| 0x110 | Flags | param_7 |
| 0x128 | WeaponTypeClass* | param_6 |
| 0x0E0 | Extra flag | param_8 |
| 0x06C | Speed | param_5 |
| 0x0AC | BulletTypeClass* | param_2 |
| 0x0B0 | Target (AbstractClass*) | param_4 |
| 0x12C | (cleared to 0) | — |
| 0x12D | Flag from BulletType+0x2F6 | — |
| 0x114 | House ID from target's owner | -1 if no target |
| 0x150 | 0x100 | (constant) |
| 0x154 | 0 | — |
| 0x158 | 0 | — |

### BulletClass::SetOwner — FUN_0046b260
**Address:** `0x0046B260`

Trivial setter: `bullet+0x130 = ownerHousePtr;`

### BulletClass Primary Vtable
**Address:** `0x007E46E4` (set in BulletClass::Constructor at `0x00466425`)

Secondary vtables:
- `bullet+0x04` = `0x007E46C8`
- `bullet+0x08` = `0x007E46C0`
- `bullet+0x0C` = `0x007E46B8`

Key virtual functions:
- vtable[0x1EC/4] = AI function at `0x004666E0` (BulletClass::AI)
- **vtable[0x1F0/4] = `0x00468670`** (BulletClass::Fire)

### BulletClass::Fire — FUN_00468670
**Address:** `0x00468670`

**Signature:** `bool __thiscall BulletClass::Fire(CoordStruct* target, double velocity[3])`

1. Calls `ObjectClass::Reveal()` — if fails, returns false
2. Copies velocity vector (3 doubles = 24 bytes) into bullet fields:
   - `bullet+0xE8..0xFF` (offsets 0x3A-0x3F in dword indexing)
   - This is the velocity as {vx: double, vy: double, vz: double}
3. Sets target coordinates:
   - `bullet+0x134` = target.X (offset 0x4D in dword idx)
   - `bullet+0x138` = target.Y (offset 0x4E)
   - `bullet+0x13C` = target.Z (offset 0x4F)
4. Computes cell from target, stores at `bullet+0x14C` (offset 0x53)
5. If `BulletTypeClass+0x2A3` (scatter) and `+0x29E` (inaccuracy) are set:
   - Applies random scatter to the target position
6. If `BulletTypeClass+0x29E` (Vertical) is set:
   - Handles vertical trajectory setup
   - Computes ground height at target for approach
7. Normalizes velocity vector to unit length
8. Submits bullet to display system via `DisplayClass::Submit_Object()`
9. Returns true

---

## 4. Trajectory Math — Spherical Velocity Calculation

### Constants

| Constant | Address | Value | Meaning |
|----------|---------|-------|---------|
| Angle (theta = phi) | stack push `{0x0FE8FBDA, 0x3FF921C9}` | 1.5707483884 | Approximately pi/2 |
| Speed | `0x007EDA88` | -100.0 | Velocity magnitude (negative) |
| Steep angle (downward nuke) | stack push `{0x1049EE22, 0x4012D989}` | 4.7124369187 | Approximately 3*pi/2 |

### Upward Nuke (Carrier Missile) Velocity — in SuperClass::Launch

**Address:** `0x006CDB9B` and surrounding

All six trig calls use the same angle: `theta = phi = pi/2`

```
vel_x = sin(theta) * cos(phi) * speed
      = sin(pi/2) * cos(pi/2) * (-100)
      = 1.0 * ~0.0 * (-100)
      ~= 0.0

vel_y = sin(phi) * sin(theta) * speed  
      = sin(pi/2) * sin(pi/2) * (-100)
      = 1.0 * 1.0 * (-100)
      = -100.0

vel_z = cos(theta) * speed
      = cos(pi/2) * (-100)
      ~= 0.0
```

**Initial velocity: approximately (0, -100, 0) leptons/frame.**

However, this initial velocity is overridden by the `Vertical=yes` flag in
`GiantNukeUp`'s BulletTypeClass, which causes BulletClass::AI to fly the
missile straight upward toward the target Z coordinate (groundZ + 20000).
The bullet speed from the INI (`Speed=100`) controls the actual movement rate.

### Downward Nuke Velocity — in FUN_0046b310 and FUN_006e3410

**Address:** `0x006E3410`

Uses `theta = phi = 3*pi/2 = 4.712...`:
```
vel_x = sin(3pi/2) * cos(3pi/2) * (-100) ~= (-1) * 0 * (-100) ~= 0
vel_y = sin(3pi/2) * sin(3pi/2) * (-100) ~= (-1) * (-1) * (-100) = -100
vel_z = cos(3pi/2) * (-100) ~= 0 * (-100) = 0
```

Again, the `Vertical=yes` flag in `GiantNukeDown` BulletTypeClass overrides
this to fly the missile straight downward. The `DetonationAltitude=30000`
(from GiantNukeDown INI) controls when it detonates.

---

## 5. NukeMaker Warhead Handler

### WarheadTypeClass Field Layout (nuke-relevant)

| Byte Offset | Type | INI Key | Notes |
|-------------|------|---------|-------|
| 0x176 | bool | NukeMaker | If true, spawns downward nuke on detonation |
| 0x175 | bool | MakesDisguise | Checked before NukeMaker in priority chain |
| 0x174 | bool | PermaDisguise | |
| 0x177 | bool | KillDriver | |
| 0x178 | bool | WarpOut | |
| 0x179 | bool | WarpIn | |
| 0x17A | bool | WarpAway | |
| 0x17B | bool | Rocker | |
| 0x17C | int | ShakeXhi | RandomRange hi for screen shake X |
| 0x180 | int | ShakeXlo | RandomRange lo for screen shake X |
| 0x184 | int | ShakeYhi | RandomRange hi for screen shake Y |
| 0x188 | int | ShakeYlo | RandomRange lo for screen shake Y |
| 0x1C4 | int | MaxDebris | Max debris count |
| 0x1C8 | int | MinDebris | Min debris count (clamped >= 0, <= MaxDebris) |

The NukeMaker INI key is read at `0x0075D970`:
```asm
push [ESI + 0x176]    ; default value
push 0x847cf4          ; "NukeMaker" string
push EBP               ; section name
call CCINIClass::ReadBool
mov [ESI + 0x176], AL  ; store result
```

### WarheadTypeClass::Detonate — Priority Chain
**Address:** `0x004690B0`

When a bullet detonates, this function checks warhead flags in strict priority order.
Only the first matching flag executes; all subsequent checks are skipped.

| Priority | Offset | Flag | Handler |
|----------|--------|------|---------|
| 1 | +0x155 | MindControl | CaptureManagerClass::CaptureUnit |
| 2 | +0x157 | Temporal | TemporalClass::InitiateWarp |
| 3 | +0x158 | unknown | — |
| 4 | +0x159 | IvanBomb | BombClass::Constructor |
| 5 | +0x15A | unknown | Deploy/garrison check |
| 6 | +0x15B | unknown | Building occupant check |
| 7 | +0x16C | unknown | — |
| 8 | +0x14F | Parasite? | Target attachment (infantry only) |
| 9 | +0x16E | unknown | Building-related handler |
| 10 | +0x175 | MakesDisguise | HouseClass deploy handler |
| **11** | **+0x176** | **NukeMaker** | **FUN_0046b310 → spawn downward nuke** |
| fallthrough | — | (none) | Apply_area_damage (normal damage) |

### FUN_0046b310 — SpawnDownwardNuke
**Address:** `0x0046B310`

Called when NukeMaker=yes warhead detonates. Creates the downward-flying nuke:

```c
void __fastcall SpawnDownwardNuke(BulletClass* carrier) {
    // 1. Get carrier's BulletTypeClass
    BulletTypeClass* carrierType = carrier+0x10C;
    
    // 2. Get target coords from carrier
    CoordStruct* carrierCoords = carrierType->vtable->GetCoords();
    CellStruct cell = {carrierCoords->X >> 8, carrierCoords->Y >> 8};
    int groundZ = CellClass::GetGroundHeight(&cell);
    
    // 3. Look up "NukePayload" weapon from global array
    int idx = FUN_00773030("NukePayload");
    WeaponTypeClass* nukePayload = WeaponTypeClass_Array[idx];
    
    // 4. Read weapon fields:
    //    +0xA0 = Projectile (GiantNukeDown BulletTypeClass*)
    //    +0xA4 = Damage (600)
    //    +0xA8 = Speed (10)
    //    +0xAC = Warhead (NUKE WarheadTypeClass*)
    
    // 5. Create new downward bullet
    BulletClass* downNuke = BulletClass::Allocate(
        nukePayload->Projectile,  // GiantNukeDown
        carrier->Target,           // original target from carrier
        nukePayload->Warhead,      // NUKE warhead
        nukePayload->Speed,        // speed = 10
        ...
    );
    
    // 6. Set owner
    BulletClass::SetOwner(downNuke, nukePayload);  // weapon pointer stored as owner ref
    
    // 7. Set target altitude
    target.Z = groundZ + 20000;  // start from 20000 above
    
    // 8. Calculate downward velocity (same spherical math, angle = ~pi/2)
    // 9. Fire bullet via vtable[0x1F0]
}
```

---

## 6. Nuke Impact — "NUKE" Warhead Special Path

### BulletClass::AI Warhead Name Check
**Address:** `0x00467E53` (within BulletClass::AI at `0x004666E0`)

When a bullet reaches its detonation condition, BulletClass::AI checks if the
warhead's INI section name matches "NUKE":

```c
// FUN_00410a40 compares the warhead's name (at +0x24) with the string "NUKE"
// String "NUKE" is at 0x0081AF98
if (FUN_00410a40(warhead, "NUKE")) {
    // Special nuke detonation
    ScreenNukeFlash();            // FUN_0053ab70
    CreateRadarEvent(impactCell);
    
    // Find NUKEBALL anim type
    int animIdx = AnimTypeClass::FindByIndex("NUKEBALL");  // 0x0081AF8C
    if (animIdx != -1) {
        AnimTypeClass* nukeBallType = g_AnimTypes_Array[animIdx];
        AnimClass* nukeBall = new AnimClass(
            nukeBallType,
            impactCoords,
            0, 1,
            0x2600,    // anim flags
            ownerCell,
            0
        );
        bullet[0x55] = nukeBall;   // store anim reference
        bullet[0x56] = 1;          // mark as having nuke anim
        
        // Add bullet to global nuke-tracking array (DAT_00b0f5bc)
    }
    goto skip_normal_detonation;
}

// Normal detonation path
FUN_00468d80(bullet);  // BulletDetonation -> WarheadTypeClass::Detonate
```

### FUN_0053ab70 — Screen Nuke Flash
**Address:** `0x0053AB70`

Triggers the white screen flash effect on nuke impact:

```c
void ScreenNukeFlash() {
    DAT_00a9fabc = 1;                    // enable nuke flash flag
    DAT_00827fcc = 0x1E;                 // flash duration = 30 frames (hardcoded)
    DAT_00827fc8 = g_CurrentFrameCounter; // flash start frame
    
    // Additional screen effects
    *(g_DisplayClass + 0x1248) = g_CurrentFrameCounter;  // display flash timestamp
    *(g_DisplayClass + 0x1250) = 1;                       // flash active flag
    
    // Apply palette shift for white flash
    FUN_0053ad00((g_DisplayClass->brightness * 1000) / 100, 1);
    FUN_004f42f0(1);  // screen redraw request
}
```

**Key findings:**
- Flash duration is **hardcoded at 30 frames** (0x1E). There is NO `NukeFlashDuration`
  INI key in gamemd.exe — that string does not exist in the binary.
- The flash is a full-screen white flash that fades over 30 frames.

### FUN_004251f0 — NukeGroundZero Area Damage
**Address:** `0x004251F0`

Applies the actual nuke warhead damage at ground zero:

```c
void NukeGroundZero() {
    Apply_area_damage(
        targetCell,                          // ground zero cell
        *(Rules + 0xF8C),                    // NukeWarhead (WarheadTypeClass*)
        0,                                    // no source object
        0                                     // no source house
    );
    
    if (g_GameMode == 0) {  // single player only
        FUN_004a3c30(Network_ServiceLoop);
    }
}
```

This uses the `NukeWarhead` from `[SpecialWeapons]` section (stored at `RulesClass+0xF8C`),
NOT the weapon's own warhead. In standard YR, NukeWarhead=Nuke (the [Nuke] warhead section).

---

## 7. Rules / INI Offsets

### RulesClass Offsets — [SpecialWeapons] Section

From `RulesClass::ReadSpecialWeapons` at `0x00669060`:

| Offset | Type | INI Key | Default Value | Notes |
|--------|------|---------|---------------|-------|
| 0xF8C | WarheadTypeClass* | NukeWarhead | — | "Nuke" — applied at ground zero |
| 0xF90 | BulletTypeClass* | NukeProjectile | — | "NukeUp" — carrier missile projectile |
| 0xF94 | BulletTypeClass* | NukeDown | — | "NukeDown" — falling missile projectile |
| 0xF98 | WarheadTypeClass* | MutateWarhead | — | (genetic mutator) |
| 0xF9C | WarheadTypeClass* | MutateExplosionWarhead | — | |
| 0xFA0 | WarheadTypeClass* | EMPulseWarhead | — | |
| 0xFA4 | BulletTypeClass* | EMPulseProjectile | — | |

### RulesClass Offsets — [General] Section (nuke-related)

| Offset | Type | INI Key | Value in YR |
|--------|------|---------|-------------|
| 0x98 | AnimTypeClass* | NukeTakeOff | NUKETO |

### WeaponTypeClass Offsets

From `WeaponTypeClass::ReadINI` at `0x00772080`:

| Offset | Type | INI Key |
|--------|------|---------|
| 0x98 | int | AmbientDamage |
| 0x9C | int | Burst |
| 0xA0 | BulletTypeClass* | Projectile |
| 0xA4 | int | Damage |
| 0xA8 | int | Speed |
| 0xAC | WarheadTypeClass* | Warhead |
| 0xB0 | int | ROF |
| 0xB4 | int | Range |
| 0xB8 | int | MinimumRange |
| 0x158 | int | RadLevel |

### SuperWeaponTypeClass Offsets (nuke-relevant)

| Offset | Type | INI Key |
|--------|------|---------|
| 0x9C | WeaponTypeClass* | WeaponType |
| 0xB4 | int | Type (0 = MultiMissile) |
| 0xC8 | BuildingTypeClass* | AuxBuilding |

---

## 8. NukeSilo (AuxBuilding) Door Animation

### How the Door Works

The nuke silo building (specified by `AuxBuilding=` in the superweapon's INI section)
has a door-open animation that must complete before the missile launches.

**Launch flow with AuxBuilding:**

1. `SuperClass::Launch` (Path B) finds the NukeSilo building
2. Calls `building->vtable[0x1E8]()` — this initiates the door-opening animation
3. Calls `building->vtable[0x1EC]()` — this sets the building's mission state
4. Stores the nuke target at `HouseClass+0x5784`
5. Stores the SW type at `building+0x5F8` (dword-indexed building[0x17E])
6. The building's AI tick loop detects the pending nuke launch and plays the door anim
7. When the door animation completes, the building fires the missile (calling back into the direct launch path or creating the bullet directly)

### BuildingTypeClass Nuke Fields

| Offset | Type | Purpose |
|--------|------|---------|
| 0x16BA | bool | HasSuperWeapon — building grants a superweapon |
| 0x16F0 | WeaponTypeClass* | SuperWeapon1 weapon type reference |
| 0x16F4 | WeaponTypeClass* | SuperWeapon2 weapon type reference |

### BuildingClass::CreateFireAnim
**Address:** `0x0043B5E0`

Creates the NukeTakeOff (NUKETO) animation at the building's coordinates:

```c
int BuildingClass::CreateFireAnim(AnimTypeClass* animType) {
    int animIdx = AnimTypeClass::FindByIndex(animType);
    AnimTypeClass* type = g_AnimTypes_Array[animIdx];
    CoordStruct coords;
    this->GetCoords(&coords);
    
    AnimClass* anim = new AnimClass(type, &coords, 0, 1, 0x600, 0, 0);
    
    FUN_00424c90(...);  // Associate anim with building
    FUN_00424ca0(...);  // Set building reference
    anim+0x19D = 1;     // Mark as attached to building
    
    return anim;
}
```

---

## 9. Radiation at Impact

From `WarheadTypeClass::Detonate` (beginning of the function, before the flag
priority chain):

```c
// Radiation check — runs for ALL warheads before the flag chain
if (bullet->WarheadType != NULL) {
    int radLevel = *(int*)(bullet->WarheadType + 0x158);  // RadLevel from weapon
    if (radLevel > 0) {
        // Convert impact coords to cell
        CellStruct cell = MapClass::Get_CellClass(impactCoords);
        
        RadSiteClass* existingSite = FindRadSiteAtCell(cell);
        if (existingSite == NULL) {
            RadSiteClass* site = new RadSiteClass();
            site->SetCell(cell);
            site->SetSpread(...);
            site->SetRadLevel(radLevel);
            site->Activate();
            RegisterRadSite(site);
        } else {
            existingSite->AddRadLevel(radLevel);
        }
    }
}
```

For the nuclear missile, `[NukePayload]` has `RadLevel=500`, which creates
significant radiation at the impact point.

### Screen Shake

Also at the start of Detonate, screen shake is applied from warhead fields:

```c
if (warhead->ShakeXhi != 0 || warhead->ShakeXlo != 0) {
    g_ShakeX = Random::Range(warhead->ShakeXhi, warhead->ShakeXlo);
}
if (warhead->ShakeYhi != 0 || warhead->ShakeYlo != 0) {
    g_ShakeY = Random::Range(warhead->ShakeYhi, warhead->ShakeYlo);
}
```

---

## 10. INI Configuration Reference

### Standard YR Nuke Configuration

```ini
; === [SpecialWeapons] section in rules(md).ini ===
NukeWarhead=Nuke          ; RulesClass+0xF8C — warhead for ground zero damage
NukeProjectile=NukeUp     ; RulesClass+0xF90 — carrier missile projectile type
NukeDown=NukeDown          ; RulesClass+0xF94 — falling missile projectile type

; === [General] section ===
NukeTakeOff=NUKETO         ; RulesClass+0x98 — takeoff animation

; === Weapon definitions ===
[NukeCarrier]
Projectile=GiantNukeUp     ; Upward missile bullet type
Speed=100
Warhead=NukeMaker          ; Triggers downward nuke spawn on detonation

[NukePayload]
Damage=600
Range=30
Projectile=GiantNukeDown   ; Downward missile bullet type
Speed=10
RadLevel=500
Warhead=NUKE               ; Actual damage warhead

; === Projectile (BulletType) definitions ===
[GiantNukeUp]
Arm=2
Shadow=no
Image=NKMSLUP
Acceleration=1
Vertical=yes               ; Flies straight up
DetonationAltitude=20000   ; Detonates at this height, triggering NukeMaker
FirersPalette=yes

[GiantNukeDown]
Arm=2
Shadow=no
Image=NKMSLDN
Acceleration=1
Vertical=yes               ; Flies straight down
DetonationAltitude=30000
FirersPalette=yes

; === Warhead definitions ===
[NukeMaker]
NukeMaker=yes              ; WarheadTypeClass+0x176 — triggers downward nuke spawn

[Nuke]                     ; The actual damage warhead (from NukeWarhead= in [SpecialWeapons])
; (standard damage warhead with high CellSpread)

; === Animation ===
[NUKETO]                   ; In art(md).ini
Translucent=yes
Translucency=75
StartSound=NukeLaunch
```

### Key Hardcoded Values

| Value | Location | Description |
|-------|----------|-------------|
| 20000 | SuperClass::Launch, FUN_0046b310 | Height offset for missile target (leptons above ground) |
| -100.0 | `0x007EDA88` | Velocity magnitude for spherical trajectory calculation |
| ~pi/2 (1.5707...) | Stack-pushed double | Angle for nearly-vertical trajectory |
| 30 (0x1E) | FUN_0053ab70 | Nuke flash duration in frames (hardcoded, no INI key) |
| "NUKE" | `0x0081AF98` | Warhead name checked in BulletClass::AI for special detonation path |
| "NUKEBALL" | `0x0081AF8C` | Animation name looked up for nuke impact visual |
| "NukePayload" | `0x0081AFA0` | Weapon name looked up to create the downward nuke |

---

## 11. Global Data Addresses

| Address | Type | Description |
|---------|------|-------------|
| 0x0088756C | ptr | WeaponTypeClass::Array — global array of all weapon types |
| 0x00887578 | int | WeaponTypeClass::Array count |
| 0x00A9FABC | bool | Nuke flash active flag |
| 0x00827FCC | int | Nuke flash remaining frames |
| 0x00827FC8 | int | Nuke flash start frame |
| 0x00B0F5BC | ptr | Nuke bullet tracking array |
| 0x00B0F5C0 | int | Nuke bullet array capacity |
| 0x00B0F5C8 | int | Nuke bullet array count |
| 0x00A8B538 | int | Multiplayer flag (0 = single player) |
| 0x007EDA88 | double | -100.0 constant (velocity magnitude) |
