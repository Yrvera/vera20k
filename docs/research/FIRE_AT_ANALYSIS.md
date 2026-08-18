# TechnoClass::Fire_At — Full Decompilation Analysis

**Address:** 0x006FDD50
**Size:** ~7167 bytes / 919 decompiled lines
**Research date:** 2026-03-22
**Source:** Ghidra MCP decompilation of gamemd.exe
**Confidence:** HIGH (directly decompiled, cross-referenced against READINI_FIELD_MAPS.md and TECHNOCLASS_STRUCT_LAYOUT.md)

---

## 1. Function Signature & Parameters

```c
int* __thiscall TechnoClass::Fire_At(TechnoClass* this)
```

Ghidra shows `this` as the ECX register (thiscall). The two stack parameters are:

| Parameter | Stack Var | Description |
|-----------|-----------|-------------|
| `target` | `in_stack_00000004` | `AbstractClass*` — the target being fired upon (cell, techno, etc.) |
| `weapon_index` | `in_stack_00000008` | `int` — 0 = primary weapon, 1 = secondary weapon |

**Return value:** `int*` — pointer to the created `BulletClass` (projectile), or NULL (0) if fire was aborted.

---

## 2. Flow Overview — Step-by-Step Pipeline

The function is large (~919 decompiled lines). Here is the logical flow:

### Phase 1: Get Weapon & Early Bail-Out Checks (lines ~60-200)

1. **Get the weapon:** Calls `vtable+0x3F8` (GetWeapon) with the weapon_index parameter. Returns a `WeaponTypeClass*` pointer. If the weapon pointer is NULL, return immediately.

2. **Target validation:** If `target == NULL`, return NULL.

3. **Target extraction:** If the target has flag bit 1 set (is a TechnoClass), extract its pointer for further checks. Stores this as `piStack_38` (the "techno target").

4. **Get the weapon's BulletTypeClass:** Reads `weapon+0xA0` (the `Projectile` pointer). Stored as `iStack_54`.

5. **Map editor check:** If `g_IsMapEditor != 0` (`DAT_00a8ed6b`), return NULL. (corrected 2026-05-28: was "game is paused/frozen"; binary label is `g_IsMapEditor` via `decompile_function 0x006FDD50` — ROOT_CAUSE: RTTI_LABEL_DRIFT)

6. **Target-is-limbo check:** If the techno target exists and `target+0x81 != 0` (in limbo), return NULL.

### Phase 2: Special Weapon Type Handling — Short-Circuit Returns (lines ~100-200)

The function checks for several special weapon types that have their own firing logic and return early:

7. **Suicide weapon** (`weapon+0x144`): If `Suicide=yes`, calls `vtable+0x84` (GetTechnoType), then calls `vtable+0x16C` (SetTarget with rules value) to cause self-destruction. Returns NULL.

8. **UseFireParticles** (`weapon+0x129`): If set and `this+0x304 != 0` (particle system already exists), return NULL (don't fire again until particles finish).

9. **IsRailgun** (`weapon+0x12D`): If set and `this+0x314 != 0` (railgun particle system exists), return NULL.

10. **UseSparkParticles** (`weapon+0x12A`): If set and `this+0x308 != 0`, return NULL.

11. **IsSonic** (`weapon+0x130`): If set and `this->Wave != NULL` (sonic wave exists), return NULL.

12. **Spawner** (`weapon+0x131`): Handles spawner weapons (e.g. Aircraft Carrier). Calls `FUN_006b7b90` to set spawn target. Performs shroud/fog reveal logic if needed. Updates fog borders. Returns NULL (spawner weapons don't create bullets).

13. **DrainWeapon** (`weapon+0x142`): Handles drain/capture weapons. Checks target is a TechnoClass, checks target's type has `Drainable` flag (`typeClass+0x5EF`). Calls `BuildingClass::EnterTransport` to initiate drain. Returns NULL.

### Phase 3: Coordinate Calculations (lines ~200-280)

14. **Fire Location Table Init:** On first call, initializes a static 8-entry coordinate offset table at `DAT_00b0eaa8` for random scatter patterns. Each entry is 12 bytes (X, Y, Z).

15. **Target coordinate resolution:**
    - If the weapon type has `IsGattling` flag (`technoType+0x691`): Uses a rotating index into the 8-entry scatter table, adding offsets to `this->Location`. The scatter index is randomized on first burst (`CurrentBurstIndex == 0`) then incremented modulo `8 / Burst`.
    - If `AreaFire` (`weapon+0x150`): Gets the firing unit's own cell coordinates and targets that cell (fires at own position — area effect).
    - Otherwise: Gets the target's coordinates via `vtable+0xA4` (GetCoords for techno) or `vtable+0x58` (GetCoords for abstract).

16. **FLH (Fire Location Height) calculation:** Calls `vtable+0xB0` (GetFLH) with weapon_index to get the muzzle position (Fire Location + Height offset from art.ini). Stored as `iStack_8c, iStack_88, iStack_84` (X, Y, Z).

17. **Facing calculation:** If the bullet is not `Dropping` (0x29C) and ROT is 0 (0x2DC), calculates the facing angle via `atan2` from muzzle to target. Otherwise calls `vtable+0x308` (GetTurretFacing).

### Phase 4: Damage Calculation & Veterancy (lines ~280-340)

18. **Base damage:** Reads `weapon+0xA4` (Damage value). If weapon is NOT `IsSonic` and NOT `UseFireParticles`, the damage is scaled:

19. **Veterancy damage multiplier:** Checks if the unit is veteran (`FUN_0074ff90`) or elite (`FUN_00750010`). If so, reads type flags:
    - Veteran: If `type+0x29E` (VeteranAbilities includes firepower), applies multiplier via `Math::ftol`.
    - Elite: If `type+0x29E` or `type+0x2B0` (EliteAbilities includes firepower), applies multiplier.

20. **Naval unit check:** If `vtable+0x400` returns true (IsNaval check), applies another damage modifier.

21. **IronCurtain modifier:** If `this+0x2E4 != 0` (has IronCurtain or ForceShield) AND unit type is not type 6 (building), applies damage reduction multiplier.

22. **Airstrike modifier:** If `this+0x82 != 0` (airstrike flag), applies yet another damage multiplier.

### Phase 5: DiskLaser Special Case (lines ~340-360)

23. **DiskLaser** (`weapon+0x14A`): If set, allocates a `DiskLaserClass` object (0x40 bytes), constructs it. Then:
    - Increments `CurrentBurstIndex`
    - Sets ROF timer fields (0x2F8, 0x2EC, 0x2F0, 0x2F4)
    - Modulos burst index by `Burst` count (`weapon+0x9C`)
    - Calls `BulletAnimTracker__Register` (0x004a71a0) (corrected 2026-05-28: was "FUN_004a71a0 (DiskLaser registration)"; binary label is `BulletAnimTracker__Register` via `get_function_by_address 0x004a71a0` — ROOT_CAUSE: RTTI_LABEL_DRIFT)
    - Returns NULL (DiskLaser is not a normal bullet)

### Phase 6: Projectile (Bullet) Creation (lines ~360-400)

24. **Range calculation:** Computes distance from muzzle to target via `sqrt(dx^2 + dy^2)`.

25. **BulletClass creation:** Calls `FUN_0046b050` (BulletClass factory). Parameters include:
    - `this` (the firer)
    - damage amount
    - `weapon+0xAC` (WarheadType pointer)
    - computed range
    - `weapon+0x12F` (Bright flag)

    Returns a `BulletClass*`. If NULL, jumps to cleanup at `LAB_006ff751`.

26. **Bullet owner assignment:** Calls `BulletClass__SetOwner` (0x0046b260) then `vtable+0xD4` on the bullet. (corrected 2026-05-28: was "FUN_0046b260 (bullet init)"; binary label is `BulletClass__SetOwner` via `get_function_by_address 0x0046b260` — ROOT_CAUSE: RTTI_LABEL_DRIFT)

27. **Bright flag propagation:** If the firer is on the map (flag bit 2) and has a LocomotorClass, and the type does NOT have a specific flag (`type+0xD94`), sets `bullet+0xB4 = 1`.

### Phase 7: Pre-fire Damage Application (lines ~400-420)

28. **Inaccurate bullet pre-damage:** If the bullet is NOT bright AND the BulletType does NOT have `Inaccurate` flag (`0x2A2`), checks if the firer has an existing target (`this->Target`). If so, calls `FUN_006fdb80` to compute preliminary damage and decrements the target's health: `target+0x70 -= damage`.

    This is the "damage subtraction on fire" mechanic — the engine pre-subtracts damage from the target's HP when the bullet is fired, not when it hits. This prevents overkill from multiple units firing at the same target.

### Phase 8: Bullet Velocity & Trajectory (lines ~420-600)

29. **Get bullet target coordinates:** Calls `TechnoClass__Resolve_ArchiveTarget_Coords` (0x0070bcb0) to get the bullet's actual target point (accounting for spawn offsets, barrel positions, etc.). (corrected 2026-05-28: was "FUN_0070bcb0 / GetBulletTargetPoint"; binary label is `TechnoClass__Resolve_ArchiveTarget_Coords` via `get_function_by_address 0x0070bcb0` — ROOT_CAUSE: RTTI_LABEL_DRIFT)

30. **Inaccurate scattering:** If `BulletType+0x2A2` (Inaccurate) is set AND `BulletType+0x29B` (Arcing) is set:
    - If NOT `FlakScatter` (`0x2A3`) OR `Inviso` (`0x29E`): Applies random inaccuracy using `RulesClass+0x1734` (BallisticScatter value). Uses random angle and distance.
    - If `FlakScatter` AND NOT `Inviso`: Applies proportional inaccuracy based on distance, using weapon's `vtable+0x168` call.

31. **Projectile pitch calculation:** For Arcing bullets with no ROT (non-homing), calculates the vertical pitch angle using the distance and Z-difference:
    - If `abs(dZ) > 200`: Computes pitch angle via `atan(dZ / horizontalDist)`
    - Special handling for aircraft targets (type == 6): Uses aircraft altitude to compute steep angles
    - For `Level` bullets (`0x29D`): Uses `SubjectToGravity` calculation

32. **For Arcing (lobbed) projectiles:** Calculates a gravity-based arc trajectory. Uses `RulesClass+0x16B8` (Gravity constant). If the BulletType has `Floater` flag, calls `FUN_0048acf0` for floating trajectory math. Then calls `FUN_0048a8d0` for full arc calculation.

33. **Velocity vector normalization:** The velocity vector `(vX, vY, vZ)` is normalized and scaled to the weapon's Speed value. Sin/Cos lookup tables are used for angular decomposition.

34. **Bullet launch:** Calls `bullet->vtable+0x1F0` (MoveTo/Launch) with the source coordinates and velocity vector. If launch fails, the bullet is destroyed (`vtable+8`, destructor) and NULL is returned.

### Phase 9: Post-Fire Effects — Burst Rate System (lines ~640-680)

35. **Building multi-barrel cycling:** If the firer is a building (type == 6), increments `this+0x69C` (MultiBarrelIndex) and modulos by `vtable+0x408` (GetBarrelCount).

36. **Bright bullet + target InfDeath:** If `BulletType+0x29E` (Inviso) is set AND target has `BulletType+0x8C` check, sets a flag on the bullet.

37. **Burst rate system (for IFVs and multi-weapon):** If `vtable+0x3FC` returns true AND `type+0xCA2` check passes:
    - **Primary weapon burst data (0x3D8-0x3F4):** If `this+0x3D8 != 0` (has burst data), sets `this+0x3F0 = 1` (active), copies divisor from `this+0x3DC`, clamps >= 1, computes ratio `this+0x3E8 = total / divisor`.
    - **Secondary weapon burst data (0x3F8-0x414):** Same pattern for secondary weapon fields.

### Phase 10: Particle System Creation (lines ~680-730)

38. **UseFireParticles** (`weapon+0x129`): If set and `this+0x304 == 0`, creates a `ParticleSystemClass` using `weapon+0x11C` (AttachedParticleSystem).

39. **UseSparkParticles** (`weapon+0x12A`): Same pattern, stores at `this+0x308`.

40. **IsRailgun** (`weapon+0x12D`): Creates a railgun particle system. First computes the railgun endpoint via `FUN_0070c690`, then creates the particle system at `this+0x314`.

### Phase 11: ROF Timer & Burst Index Update (lines ~730-780)

41. **Increment burst index:** `this->CurrentBurstIndex++`

42. **Get ROF:** Calls `vtable+0x318` (GetROF). If `this+0x298 != 0` (some modifier flag), halves the ROF.

43. **Set fire timer:**
    ```
    this+0x2F8 = ROF_value        // ROF countdown
    this+0x2EC = g_CurrentFrameCounter  // start frame
    this+0x2F0 = computed_value   // duration
    this+0x2F4 = ROF_value        // initial value
    ```

44. **Modulo burst index:** `this->CurrentBurstIndex %= weapon+0x9C` (Burst count).

### Phase 12: Muzzle Flash Animation (lines ~780-810)

45. **Weapon Anim selection:** Based on `weapon+0x104` (Anim count/type):
    - If count == 8: Selects anim by turret facing direction (8-directional muzzle flash)
    - If count > 0 but != 8: Uses the first anim in the list
    - If naval unit (`vtable+0x400`): Uses `weapon+0x110` (OccupantAnim)
    - If airstrike (`this+0x82`) and `weapon+0x118 != 0`: Uses OpenToppedAnim

46. **Weapon fire report sound:** If `weapon+0xCC` has a sound count > 0, plays the fire report sound via `FUN_007509E0`.

47. **AnimClass creation:** If an anim was selected, allocates `AnimClass` (0x1C8 bytes) and constructs it at the muzzle position. For buildings, adjusts the anim's Z-offset. For non-buildings, calls `FUN_00424b50` to attach the anim to the unit.

### Phase 13: Special Weapon Visual Effects (lines ~810-870)

48. **IsSonic** (`weapon+0x130`): Creates a `WaveClass` (0x240 bytes) and stores at `this->Wave`.

49. **IsLaser** (`weapon+0x149`): Calls `TechnoClass__SpawnLaser` (0x006fd210) which creates a laser draw object, calculates endpoints, and registers it. For infantry with specific flags, stores the laser pointer for persistent beam. (corrected 2026-05-28: was "Calls FUN_006fd570"; both building and non-building paths go through 0x006fd210 — ROOT_CAUSE: RTTI_LABEL_DRIFT)

50. **IsElectricBolt** (`weapon+0x151`): Calls `TechnoClass__SpawnElectricBoltEffect` (0x006fd570) which calls `TechnoClass__CreateElectricBolt` (0x006fd460). Uses `RulesClass+0x1830/0x1866/0x1869` for bolt colors. (corrected 2026-05-28: was "Calls FUN_006fd620"; electric bolt is 0x006fd570, not 0x006fd620 — ROOT_CAUSE: RTTI_LABEL_DRIFT)

51. **IsRadBeam** (`weapon+0x154`): Calls `TechnoClass__SpawnRadBeam` (0x006fd620) with parameter derived from warhead's Temporal flag (`weapon+0xAC -> warhead+0x15A`). (corrected 2026-05-28: was "Calls FUN_006fd620 — CreateElectricBolt/RadBeam"; 0x006fd620 is SpawnRadBeam not ElectricBolt — ROOT_CAUSE: RTTI_LABEL_DRIFT)

52. **IsRadEruption** (`weapon+0x155`): Calls `FUN_006fd800` which creates a multi-cell radiation eruption effect. Iterates a 3x3 grid around the firer, placing radiation particles with random offsets (+-128 leptons) and random intensities (5-20).

53. **IsMagBeam** (`weapon+0x15C`): Creates a `WaveClass` with type 3 (magnetron beam). Only created if `this->Wave == NULL` and target is not a building.

### Phase 14: Laser Drawing (from TechnoClass__SpawnLaser / 0x006fd210) (lines ~800-830)

54. **Laser beam creation** (`weapon+0x149` / IsLaser path): Calls `TechnoClass__SpawnLaser` (0x006fd210) which handles BOTH building and non-building cases: (corrected 2026-05-28: was described as "for buildings only via FUN_006fd210, non-buildings via FUN_006fd570"; binary shows single function at 0x006fd210 handles all — ROOT_CAUSE: RTTI_LABEL_DRIFT)
    - Gets source position from `vtable+0xB0` (GetFLH)
    - For buildings (`GetAbsType==6`): uses `vtable+0xAC` for render coords to calculate Z-offset adjustment
    - For `IsHouseColor` weapons: adjusts color from house color table (`param_1[0x87] + 0x56fc/0x56fd/0x56fe`)
    - Creates a `LaserDrawClass` (0x5C bytes) via `LaserDrawClass__Constructor` (0x0054fe60)
    - If `IsHouseColor` (`weapon+0x14D`), sets `laser+0x20 = 1` (byte, confirmed via `decompile_function 0x006fd210`)
    - Returns the laser object pointer

### Phase 15: Post-Fire Cleanup (lines ~830-919)

55. **Fire animation trigger:** Calls `vtable+0x390` (Mark/notify fire event).

56. **Voice/sound trigger:** Calls `vtable+0x124` with param 2 (fire voice event).

57. **Cloaked unit decloak check:** If the unit has decloak flags (`0x41A` or `0x41B`), checks visibility. If visible, proceeds to RevealOnFire logic.

58. **RevealOnFire** (`weapon+0x137`): If set and target is a TechnoClass with a valid pointer, AND firer is player-controlled, reveals shroud at the target location and updates fog borders.

59. **Set LastFireFrame:** `this+0x120 = g_CurrentFrameCounter`.

60. **LimboLaunch** (`weapon+0x132`): Special handling for weapons that fire from limbo (off-map spawned units). If set:
    - Checks `type+0xD3C` (IsLimboDelivery): If firer has airstrike flag, kills the unit (`vtable+0x3C`). If target is infantry (type 0xF), sets suicide flag (`this+0x432 = 1`).
    - Checks `type+0xD3D` (IsLimboKill): If firer is a building with factory, stores building animation handle.
    - Calls `vtable+0xD4` (detach). If warhead has `Temporal` flag (`weapon->warhead+0x159`), applies temporal damage to target, sets `target+0x698 = g_CurrentFrameCounter + 0x14`, and calls `FUN_004664c0` for the temporal warp effect.

61. **Spawner multi-target (type+0x6B0):** If type has multi-target spawner flag, manages a target list at `this+0x470..0x484`. Adds the current target to the list if capacity allows. Calls `vtable+0x3C8` (StopFiring).

62. **FireOnce** (`weapon+0x135`): If set, calls `TeamClass__Set_Convoy_Target` (0x006e9050) to redirect convoy team targeting, then sets `*(this[1].field_0xb4 + 0x80) = 1` (convoy delivery flag), then calls `vtable+0x3C8` (StopFiring). (corrected 2026-05-28: was "FUN_006e9050 for infantry to handle one-shot behavior / sets weapon handle"; binary shows `TeamClass__Set_Convoy_Target` via `get_function_by_address 0x006e9050` and decompile confirms convoy logic — ROOT_CAUSE: RTTI_LABEL_DRIFT)

63. **Return:** Returns the BulletClass pointer (`piStack_94`).

---

## 3. Weapon Selection

`Fire_At` does NOT select which weapon to use. That decision is made upstream by `TechnoClass::SelectWeaponAgainst` (which stores the result in `this+0x140` / CurrentWeaponIndex). `Fire_At` receives the weapon index as its second parameter.

The weapon pointer is obtained via `vtable+0x3F8` (GetWeapon), which returns the WeaponTypeClass for the given index. This virtual function is overridden by subclasses (BuildingClass, InfantryClass, etc.) to handle IFV weapons, garrison weapons, etc.

---

## 4. Projectile (Bullet) Creation

The bullet is created by `FUN_0046b050` at address 0x0046b050:

```
FUN_0046b050(firer, damage, warhead_ptr, range, bright_flag)
```

This function:
1. Calls `CoCreateInstance` with a CLSID — this creates a COM object (BulletClass)
2. Calls `FUN_004664c0` to initialize the bullet with its parameters
3. Returns the BulletClass pointer

After creation, the bullet's velocity vector is calculated (see Phase 8) and then `bullet->vtable+0x1F0` (MoveTo) is called to actually launch it.

If bullet creation fails (returns NULL), the function skips all post-fire effects and jumps directly to cleanup at `LAB_006ff751`.

---

## 5. Ammo Handling

**Ammo is NOT decremented in Fire_At.** There is no explicit ammo counter modification in this function. Ammo management is handled elsewhere — likely in `TechnoClass::GetROF` (vtable+0x318) or in the mission/AI layer that decides whether the unit can fire.

The function does check `this+0x298` as a modifier (if set, ROF is halved), but this appears to be a reloading/ammo-related flag rather than a direct ammo count.

---

## 6. Rate of Fire (ROF) Timer

After a successful fire, the ROF timer is set:

```c
int rof = vtable+0x318();  // GetROF virtual call
if (this->field_0x298 != 0) {
    rof = rof / 2;          // Halve ROF if modifier flag set
}
this->field_0x2F8 = rof;            // ROF value (countdown timer)
this->field_0x2EC = g_CurrentFrameCounter;  // Frame when fire started
this->field_0x2F0 = computed_value;  // Duration
this->field_0x2F4 = rof;            // Initial ROF value (for timer ratio)
```

The `CurrentBurstIndex` is then modulo'd by `weapon+0x9C` (Burst count), so multi-burst weapons cycle through their burst sequence.

### Veterancy ROF Bonus

The ROF itself is returned by the virtual `GetROF` call (vtable+0x318), which presumably applies veterancy multipliers internally. There is no direct veterancy ROF modifier visible in Fire_At itself — the modifier is encapsulated in the GetROF virtual.

---

## 7. Special Weapon Types — Summary Table

| Weapon Flag | Offset | Behavior in Fire_At |
|-------------|--------|---------------------|
| `Suicide` | 0x144 | Sets target to self, triggers self-destruct via vtable. Returns NULL. |
| `Spawner` | 0x131 | Updates spawn target (`FUN_006b7b90`). Handles shroud reveal. Returns NULL. |
| `DrainWeapon` | 0x142 | Calls EnterTransport on target. Returns NULL. |
| `DiskLaser` | 0x14A | Creates DiskLaserClass (0x40 bytes). Updates burst/ROF. Calls `BulletAnimTracker__Register` (0x004a71a0). Returns NULL. (corrected 2026-05-28: was "FUN_004a71a0"; ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| `IsLaser` | 0x149 | Creates LaserDrawClass via `TechnoClass__SpawnLaser` (0x006fd210) for all unit types. (corrected 2026-05-28: was "FUN_006fd210/FUN_006fd570"; 006fd570 is ElectricBolt, not laser — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| `IsBigLaser` | 0x14C | Handled via the same laser path. |
| `IsHouseColor` | 0x14D | Laser uses house color; sets `laser+0x20 = 1`. |
| `IsElectricBolt` | 0x151 | Creates electric bolt via `TechnoClass__SpawnElectricBoltEffect` (0x006fd570) → `TechnoClass__CreateElectricBolt` (0x006fd460). (corrected 2026-05-28: was "FUN_006fd620"; ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| `IsRadBeam` | 0x154 | Creates radiation beam via `TechnoClass__SpawnRadBeam` (0x006fd620). (corrected 2026-05-28: was "FUN_006fd620 (same as ElectricBolt)"; 0x006fd620 is SpawnRadBeam; ElectricBolt is 0x006fd570 — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| `IsRadEruption` | 0x155 | Creates 3x3 grid radiation effect via `FUN_006fd800`. |
| `IsSonic` | 0x130 | Creates WaveClass (0x240 bytes). Only one wave per unit. |
| `IsMagBeam` | 0x15C | Creates WaveClass type 3 (magnetron). Only if no existing wave and target is not building. |
| `UseFireParticles` | 0x129 | Creates ParticleSystemClass at `this+0x304`. Blocks re-fire until system finishes. |
| `UseSparkParticles` | 0x12A | Creates ParticleSystemClass at `this+0x308`. Same blocking behavior. |
| `IsRailgun` | 0x12D | Creates railgun particle system at `this+0x314`. Same blocking behavior. |
| `LimboLaunch` | 0x132 | Special off-map delivery. May kill firer or set suicide flag. |
| `FireOnce` | 0x135 | Stops firing after one shot. Handles infantry one-shot deploy. |
| `RevealOnFire` | 0x137 | Reveals shroud at target position for player-controlled units. |
| `AreaFire` | 0x150 | Fires at own cell position (area effect). |

---

## 8. Veterancy Bonuses

### Damage Modifier
Applied in Phase 4 (lines ~280-340). The logic:

```
base_damage = weapon+0xA4
if (is_veteran AND type+0x29E != 0):   // VeteranAbilities firepower
    damage *= veteran_multiplier
if (is_elite AND (type+0x29E != 0 OR type+0x2B0 != 0)):  // EliteAbilities firepower
    damage *= elite_multiplier
```

The actual multiplier values come from `Math::ftol()` calls (floating-point scaling), likely reading from RulesClass veteran/elite damage multiplier constants.

### ROF Modifier
Not directly in Fire_At — encapsulated in the `GetROF` virtual call (vtable+0x318).

### Additional Damage Modifiers in Fire_At
- **Naval units** (`vtable+0x400`): Additional damage scaling
- **IronCurtain/ForceShield** (`this+0x2E4 != 0`): Reduces damage unless unit is a building (type 6)
- **Airstrike flag** (`this+0x82 != 0`): Additional damage modifier

---

## 9. Key Struct Offsets Referenced

### TechnoClass (this pointer)

| Offset | Type | Purpose | Confidence |
|--------|------|---------|------------|
| 0x082 | byte | Airstrike flag — if set, applies damage modifier | MED |
| 0x083 | byte | Checked in LimboLaunch self-destruct path | MED |
| 0x0AC | int | House owner index/type — checked != 1 for weapon select | MED |
| 0x120 | int | **LastFireFrame** — set to `g_CurrentFrameCounter` after firing | HIGH |
| 0x298 | byte | ROF halving flag — if set, ROF is divided by 2 | MED |
| 0x2A0 | int | **GattlingScatterIndex** — random index for gattling scatter | HIGH |
| 0x2E4 | int | **IronCurtain/ForceShield timer** — if != 0, damage reduced | MED |
| 0x2EC | int | **FireTimer.StartFrame** — `g_CurrentFrameCounter` when fired | HIGH |
| 0x2F0 | int | **FireTimer.Duration** | HIGH |
| 0x2F4 | int | **FireTimer.InitialValue** | HIGH |
| 0x2F8 | int | **FireTimer.ROF** — the ROF countdown value | HIGH |
| 0x304 | int | **FireParticleSystem ptr** — for UseFireParticles weapons | HIGH |
| 0x308 | int | **SparkParticleSystem ptr** — for UseSparkParticles weapons | HIGH |
| 0x314 | int | **RailgunParticleSystem ptr** — for IsRailgun weapons | HIGH |
| 0x3B8 | int | **CurrentBurstIndex** — incremented per shot, modulo'd by Burst | HIGH |
| 0x3BC | int | **MuzzleFlashTimer.Start** | HIGH |
| 0x3C0 | int | **MuzzleFlashTimer.Duration** | HIGH |
| 0x3D8 | int | **PrimaryBurstRate.Total** — for IFV/multi-weapon | HIGH |
| 0x3DC | int | **PrimaryBurstRate.Count** | HIGH |
| 0x3E8 | float | **PrimaryBurstRate.Ratio** (computed: Total / Divisor) | HIGH |
| 0x3F0 | int | **PrimaryBurstRate.Active** — set to 1 | HIGH |
| 0x3F4 | int | **PrimaryBurstRate.Divisor** — clamped >= 1 | HIGH |
| 0x3F8 | int | **SecondaryBurstRate.Total** | HIGH |
| 0x3FC | int | **SecondaryBurstRate.Count** | HIGH |
| 0x408 | float | **SecondaryBurstRate.Ratio** | HIGH |
| 0x410 | int | **SecondaryBurstRate.Active** | HIGH |
| 0x414 | int | **SecondaryBurstRate.Divisor** | HIGH |
| 0x41A | byte | **CloakState1** — checked for decloak/reveal logic | MED |
| 0x41B | byte | **CloakState2** | MED |
| 0x432 | byte | **SuicideFlag** — set to 1 for LimboLaunch self-destruct | MED |
| 0x434 | int | **BuildingAnimHandle** — stored after LimboLaunch fire | MED |
| 0x43C | int | **BarrelRotationIndex** — cycled for multi-barrel weapons | HIGH |
| 0x470 | ptr | **TargetList vtable** — for multi-target spawner weapons | HIGH |
| 0x474 | ptr | **TargetList.Data** — array of target pointers | HIGH |
| 0x478 | int | **TargetList.Count** | HIGH |
| 0x480 | int | **TargetList.CurrentIndex** | HIGH |
| 0x484 | int | **TargetList.Capacity** | HIGH |
| 0x510 | ptr | **EBolt ptr** — electric bolt object for infantry | MED |
| 0x698 | int | **TempInvulTimer** — set to `g_CurrentFrameCounter + 0x14` | MED |
| 0x69C | int | **MultiBarrelIndex** — for building multi-barrel cycling | HIGH |
| Wave | ptr | **WaveClass ptr** — sonic/magnetron wave object | HIGH |
| Target | ptr | **Current target** — AbstractClass pointer | HIGH |

### WeaponTypeClass (weapon pointer)

| Offset | Type | INI Key | Confidence |
|--------|------|---------|------------|
| 0x98 | int | `AmbientDamage` | HIGH |
| 0x9C | int | `Burst` | HIGH |
| 0xA0 | ptr | `Projectile` (BulletTypeClass*) | HIGH |
| 0xA4 | int | `Damage` | HIGH |
| 0xA8 | int | `Speed` | HIGH |
| 0xAC | ptr | `Warhead` (WarheadTypeClass*) | HIGH |
| 0xB0 | int | `ROF` (frames) | HIGH |
| 0xCC | int | `Report` sound count | HIGH |
| 0xF4+ | ptr[] | `Anim` list (AnimType ptrs) | HIGH |
| 0xF8 | ptr | `Anim` list data pointer | HIGH |
| 0x104 | int | `Anim` list count | HIGH |
| 0x110 | ptr | `OccupantAnim` | HIGH |
| 0x118 | ptr | `OpenToppedAnim` | HIGH |
| 0x11C | ptr | `AttachedParticleSystem` | HIGH |
| 0x120-0x128 | bytes | `LaserInnerColor`, `LaserOuterColor`, `LaserOuterSpread` | HIGH |
| 0x129 | bool | `UseFireParticles` | HIGH |
| 0x12A | bool | `UseSparkParticles` | HIGH |
| 0x12D | bool | `IsRailgun` | HIGH |
| 0x12F | bool | `Bright` | HIGH |
| 0x130 | bool | `IsSonic` | HIGH |
| 0x131 | bool | `Spawner` | HIGH |
| 0x132 | bool | `LimboLaunch` | HIGH |
| 0x135 | bool | `FireOnce` | HIGH |
| 0x137 | bool | `RevealOnFire` | HIGH |
| 0x142 | bool | `DrainWeapon` | HIGH |
| 0x144 | bool | `Suicide` | HIGH |
| 0x149 | bool | `IsLaser` | HIGH |
| 0x14A | bool | `DiskLaser` | HIGH |
| 0x14D | bool | `IsHouseColor` | HIGH |
| 0x14E | byte | `LaserDuration` | HIGH |
| 0x150 | bool | `AreaFire` | HIGH |
| 0x151 | bool | `IsElectricBolt` | HIGH |
| 0x154 | bool | `IsRadBeam` | HIGH |
| 0x155 | bool | `IsRadEruption` | HIGH |
| 0x15C | bool | `IsMagBeam` | HIGH |

### BulletTypeClass (projectile pointer at weapon+0xA0)

| Offset | Type | INI Key | Used For |
|--------|------|---------|----------|
| 0x29B | bool | `Arcing` | Trajectory calculation, inaccuracy scatter |
| 0x29C | bool | `Dropping` | Skips facing calc, uses special trajectory |
| 0x29E | bool | `Inviso` | Skips inaccuracy, damage pre-subtraction |
| 0x2A2 | bool | `Inaccurate` | Enables scatter calculation |
| 0x2A3 | bool | `FlakScatter` | Proportional inaccuracy mode |
| 0x236 | bool | `Level` | Flat trajectory (used for pitch calc) |
| 0x295 | bool | `Floater` | Alternative gravity calc |
| 0x2DC | int | `ROT` | Rate of turn — 0 = unguided |

---

## 10. Functions Called — Address Map

| Address | Name/Purpose | Called When |
|---------|-------------|------------|
| `vtable+0x3F8` | **GetWeapon(index)** — returns WeaponTypeClass* | Always (first call) |
| `vtable+0x84` | **GetTechnoType()** — returns TechnoTypeClass* | Multiple checks |
| `vtable+0x2C` | **GetAbsType()** — returns RTTIType enum (6=building, 1=infantry, etc.) | Type-specific branching |
| `vtable+0x48` | **GetCoords(out)** — returns 3D coordinates of this object | Muzzle/target pos |
| `vtable+0xB0` | **GetFLH(weapon_idx)** — returns Fire Location + Height coords | Muzzle position |
| `vtable+0x308` | **GetTurretFacing()** — returns facing direction | For turret-aimed weapons |
| `vtable+0x318` | **GetROF()** — returns rate of fire value | Setting reload timer |
| `vtable+0x300` | **GetTargetCoords(out, weapon_idx)** — target coordinates with offsets | For aircraft targets |
| `vtable+0x390` | **MarkFired()** — notification that unit fired | Post-fire event |
| `vtable+0x124` | **PlayVoice(event)** — plays voice/sound event | Fire sound |
| `vtable+0x16C` | **SetTarget(target, ...)** — sets unit's target | Suicide weapons |
| `vtable+0x3C8` | **StopFiring()** — stops the fire cycle | FireOnce, Spawner |
| `vtable+0x3FC` | **HasBurstRate()** — checks if burst rate applies | IFV burst rate |
| `vtable+0x400` | **IsNaval()** — checks if unit is naval | Damage/anim modifiers |
| `vtable+0x408` | **GetBarrelCount()** — number of barrels on building | Multi-barrel cycling |
| `vtable+0x3F4` | **GetBarrelOffset()** — barrel position offset | Laser endpoint |
| `vtable+0xAC` | **GetRenderCoords()** — visual render position | Z-offset calc |
| `vtable+0xA4` | **GetCenterCoords()** (on target) | Target position |
| 0x0046b050 | **BulletClassAllocate** — allocates BulletClass | Projectile creation |
| 0x0046b260 | **BulletClass__SetOwner** — assigns owner/firer to bullet | After creation (corrected 2026-05-28: was "BulletClass::Init"; `get_function_by_address 0x0046b260` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x0054fe60 | **LaserDrawClass::Constructor** — creates laser draw object (0x5C bytes) | IsLaser weapons |
| 0x006fd210 | **TechnoClass__SpawnLaser** — full laser creation for all unit types (building and non-building) | IsLaser weapons (corrected 2026-05-28: was "CreateLaserBeam — for buildings only"; binary handles both paths; `get_function_by_address 0x006fd210` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x006fd460 | **TechnoClass__CreateElectricBolt** — creates electric bolt effect object | Called by SpawnElectricBoltEffect (corrected 2026-05-28: was "CreateLaserFromFLH"; `get_function_by_address 0x006fd460` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x006fd570 | **TechnoClass__SpawnElectricBoltEffect** — creates electric bolt visual, stores ptr for infantry | IsElectricBolt weapons (corrected 2026-05-28: was "CreateLaserForUnit — IsLaser for units"; `get_function_by_address 0x006fd570` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x006fd620 | **TechnoClass__SpawnRadBeam** — creates RadBeam object with colors from Rules, sets duration/amplitude | IsRadBeam weapons (corrected 2026-05-28: was "CreateElectricBolt/RadBeam"; `get_function_by_address 0x006fd620` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x006fd800 | **TechnoClass__SpawnRadEruption** — 3x3 grid radiation effect | IsRadEruption |
| 0x006fdb80 | **FUN_006fdb80** — calculates damage to pre-subtract from target HP | Accurate bullets |
| 0x0070bcb0 | **TechnoClass__Resolve_ArchiveTarget_Coords** — computes bullet's actual target coordinates | Velocity calculation (corrected 2026-05-28: was "GetBulletTargetPoint"; `get_function_by_address 0x0070bcb0` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x004a71a0 | **BulletAnimTracker__Register** — registers in bullet anim tracker | DiskLaser weapons (corrected 2026-05-28: was "DiskLaser::Register — registers disk laser in global draw array"; `get_function_by_address 0x004a71a0` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x006b7b90 | **SpawnManagerClass__SetTarget** — updates spawn target for spawner weapons | Spawner weapons |
| 0x006e9050 | **TeamClass__Set_Convoy_Target** — redirects convoy team targeting (called in FireOnce path) | FireOnce weapons (corrected 2026-05-28: was "InfantryClass::HandleFireOnce"; `get_function_by_address 0x006e9050` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x00424b50 | **AnimClass__SetOwnerObject** — attaches muzzle flash anim to unit | Muzzle flash (corrected 2026-05-28: was "AttachAnimToUnit"; `get_function_by_address 0x00424b50` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x007509e0 | **VocClass__PlayAt** — plays weapon fire report sound at position | Fire sound (corrected 2026-05-28: was "PlayFireSound"; `get_function_by_address 0x007509e0` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x0074ff90 | **IsVeteran()** — checks if unit has veteran status | Damage modifier |
| 0x00750010 | **IsElite()** — checks if unit has elite status | Damage modifier |
| 0x0048a8d0 | **CalcArcTrajectory** — calculates arcing ballistic trajectory | Arcing bullets |
| 0x0048acf0 | **CalcFloaterGravity** — floater gravity override | Floater bullets |
| 0x00773070 | **GetWeaponRange** — returns weapon range in leptons | Range calculation |

---

## Key Observations

1. **Pre-damage subtraction:** The engine subtracts damage from `target+0x70` (`EstimatedHealth`, NOT `Health` at +0x6C) at fire time, not on bullet impact. This is the RA2 "damage reservation" system that prevents overkill — multiple units check EstimatedHealth to avoid wasting shots on a target that will already die. Only applies to non-Inaccurate, non-Inviso, non-ElectricBolt bullets.

2. **Spawner weapons never create bullets:** They update the spawn target and return NULL. The actual spawned units are managed separately.

3. **DiskLaser bypasses normal bullet creation:** Gets its own object type (DiskLaserClass, 0x40 bytes) and returns NULL instead of a BulletClass.

4. **Visual effects are fire-time, not impact-time:** Lasers, electric bolts, rad beams, sonic waves, and magnetron beams are all created in Fire_At when the weapon fires, not when the projectile hits. The visual beam connects firer to target instantly.

5. **Multiple blocking mechanisms:** Sonic, fire particles, spark particles, and railgun weapons all use a "one at a time" pattern — they store a pointer to their active effect and refuse to fire again until the effect finishes (pointer goes NULL).

6. **Burst cycling:** `CurrentBurstIndex` increments per shot and wraps via `% Burst`. The FLH system uses this index to offset muzzle position for multi-barrel effects.

7. **Building-specific multi-barrel:** Buildings have a separate `MultiBarrelIndex` at `this+0x69C` that cycles independently from `CurrentBurstIndex`, using `GetBarrelCount()` for the modulo.

8. **The 8-entry gattling scatter table** at 0x00B0EAA8 contains pre-computed XYZ offsets for gattling weapons, initialized on first call. Values include (256,0,0), (180,180,0), (0,256,0), (-180,180,0), (-256,0,0), (-180,-180,0), (0,-256,0), (180,-180,0) — an octagonal pattern.
