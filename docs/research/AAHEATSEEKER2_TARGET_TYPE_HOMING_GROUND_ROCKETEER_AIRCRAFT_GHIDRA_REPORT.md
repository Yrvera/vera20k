# AAHeatSeeker2 Target Type Predicates - Ghidra Research Report

**Address(es):** `0x006FC0B0`, `0x00468670`, `0x004666E0`, `0x005B20F0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Target-type predicates for `MissileLauncher` / `AAHeatSeeker2` against a ground vehicle, an airborne Rocketeer infantry target, and a true `AircraftClass` target.  
**Non-Scope:** Full target acquisition, all `GetFireError` return-code causes, all `ConsideredAircraft` consumers outside this target-type slice, target invalidation, damage presentation.  
**Confidence:** High for the four named binary functions and parser anchor; Medium for broader AI-threat meaning of `ConsideredAircraft` because this slice did not exhaust target acquisition.  
**Active in YR:** Yes. The checked functions are in the normal YR weapon-fire and bullet AI path used by deployed Guardian GI `MissileLauncher` firing `AAHeatSeeker2`.

## 1. Overview

The same target can be classified by three different predicates in this path. `TechnoClass::GetFireError` uses the target's high-flying/ground status plus projectile `AA` / `AG` flags to decide fire legality. `BulletClass::Fire` and `BulletClass::AI` use the target object's `WhatAmI()` RTTI value and treat only `WhatAmI()==2` as the `AircraftClass` special case. `ConsideredAircraft=yes` on Rocketeer is active type data, but the four functions in this slice do not read `TechnoTypeClass+0xD96` directly.

## 2. Key Offsets / Predicates

| Predicate / field | Evidence | Meaning in this slice | Active in YR |
|---|---|---|---|
| `BulletTypeClass+0x2A4` | `TechnoClass::GetFireError @ 0x006FC0B0`; `WeaponType+0xA0` is projectile | Projectile `AA=` gate when the target is high-flying / air-layered | Yes: normal fire legality path |
| `BulletTypeClass+0x2A5` | `TechnoClass::GetFireError @ 0x006FC0B0`; `WeaponType+0xA0` is projectile | Projectile `AG=` gate when the target is not in the air predicate branch | Yes: normal fire legality path |
| `ObjectClass::IsHighFlying` vtable `+0x54` | `0x005F6B90` | Returns true only when object is marked and height is at least `DAT_00AC13C8 * 2` | Yes: called by `GetFireError` |
| `ObjectClass::IsLowFlying` vtable `+0x50` | `0x005F6B60` | Complementary marked-height check below `DAT_00AC13C8 * 2` | Yes: adjacent active object predicate |
| target `WhatAmI()==2` | `BulletClass::Fire @ 0x00468A4B/0x00468A4E`; `BulletClass::AI @ 0x00466CD8/0x00466CDB`; `AircraftClass::WhatAmI @ 0x0041C180` assembly returns `2` | True `AircraftClass` special case for arming override and homing flag | Yes: normal bullet launch and AI path |
| `InfantryClass::WhatAmI()==0xF` | `InfantryClass::What_Am_I @ 0x00523340` | Rocketeer remains InfantryClass for the `WhatAmI()==2` checks | Yes: class RTTI method |
| `TechnoTypeClass+0xD96` | parse write at `0x00714FE9..0x00715003`; `rulesmd.ini [JUMPJET] ConsideredAircraft=yes` | `ConsideredAircraft=` type flag; not read by the four named functions | Yes as parsed type data; not a direct predicate in this slice |

## 3. Core Logic

### 3.1 MissileLauncher fire legality

**Binary fact.** `TechnoClass::GetFireError @ 0x006FC0B0` resolves the selected `WeaponType`, then uses `WeaponType+0xA0` as the projectile pointer. It checks projectile bytes `+0x2A4` and `+0x2A5`, matching `AA` and `AG` from `BulletTypeClass::ReadINI`.

**Binary fact.** The air legality check uses object virtuals and layer/height predicates, not `WhatAmI()==2`. A high-flying target requires projectile `AA` to be true; a non-air/ground-style target requires projectile `AG` to be true. `AAHeatSeeker2` has both true in `rulesmd.ini`, so this specific projectile passes both gates.

**Active in YR:** Yes. This is the normal `GetFireError` path reached before deployed GGI fire. No TS-only feature gate was observed for these target-type branches.

### 3.2 BulletClass::Fire arming override

**Binary fact.** `BulletClass::Fire @ 0x00468670` checks the stored target pointer at `BulletClass+0x10C`. If the target is non-null and `target->WhatAmI()` returns `2`, the value passed as the arming delay to `ProximityDetector::Set @ 0x004E1130` is forced to `0`; otherwise it passes `BulletTypeClass+0x2F0` (`Arm=`).

**Binary fact.** `AAHeatSeeker2` has `Arm=2` in `rulesmd.ini`. Therefore ground vehicles and Rocketeer infantry keep `Arm=2`; true `AircraftClass` targets use arm delay `0`.

**Active in YR:** Yes. `BulletClass::Fire` is called by the live projectile launch path from `TechnoClass::Fire_At`.

### 3.3 BulletClass::AI and HomingTrack aircraft flag

**Binary fact.** In the ROT>0 homing branch, `BulletClass::AI @ 0x004666E0` re-resolves the target coordinate each tick when `BulletClass+0x10C` is non-null. It then computes an aircraft flag from the same strict class predicate: target non-null and `target->WhatAmI()==2`.

**Binary fact.** The call to `BulletClass::HomingTrack @ 0x005B20F0` pushes that `WhatAmI()==2` result as a stack argument. Assembly at `0x00466CD8..0x00466D31` shows the compare against `2`, the `ESI=1` aircraft flag path, and the later call to `0x005B20F0`.

**Binary fact.** Inside `HomingTrack`, that aircraft flag controls altitude/pitch handling. When the flag is false, the function enters the ground-target missile altitude correction branch: it samples cell ground height, applies bridge adjustment when cell flags require it, and adjusts projectile Z/pitch against a safety-altitude style target. When the flag is true, that ground-target altitude correction block is skipped and the function proceeds through the alternate pitch application path.

**Active in YR:** Yes. This is the normal homing branch for projectiles with `ROT > 0`; `AAHeatSeeker2 ROT=60`.

### 3.4 Rocketeer / ConsideredAircraft behavior

**Binary fact.** `rulesmd.ini [JUMPJET]` sets `JumpJet=yes`, `MovementZone=Fly`, and `ConsideredAircraft=yes`. The `ConsideredAircraft` parser writes `TechnoTypeClass+0xD96` at `0x00714FE9..0x00715003`; prior audited Rocketeer docs also verify this parse site.

**Binary fact.** `InfantryClass::What_Am_I @ 0x00523340` returns `0xF`, so Rocketeer does not satisfy the `WhatAmI()==2` checks in `BulletClass::Fire` or `BulletClass::AI`.

**Binary fact.** `ObjectClass::IsHighFlying @ 0x005F6B90` does not read `ConsideredAircraft`; it checks the marked object flag and current height against `DAT_00AC13C8 * 2`. A normal airborne Rocketeer can therefore be an air target for fire legality while still remaining InfantryClass for the arming/homing aircraft branch.

**Inference.** In normal play, Rocketeer is usually airborne because its JumpJet/BalloonHover movement data keep it at cruise height; that makes the high-flying `AA` gate relevant. This report did not exhaust every jumpjet state transition, so grounded/transition edge cases remain outside scope.

**Active in YR:** Yes for the parser and normal Rocketeer data. Direct `ConsideredAircraft` consumers for broader threat routing are active per prior audited docs, but they are not direct reads in the four functions this slot verified.

## 4. Three-Target Comparison

| Target | Fire legality predicate | Arming delay in `Fire` | `AI` / `HomingTrack` aircraft flag | Active in YR |
|---|---|---|---|---|
| Ground vehicle | Non-high-flying / ground branch; requires projectile `AG=yes` | `Arm=2` from `AAHeatSeeker2+0x2F0` | false (`WhatAmI()!=2`) | Yes |
| Rocketeer infantry, airborne, `ConsideredAircraft=yes` | High-flying branch; requires projectile `AA=yes` | `Arm=2` because InfantryClass `WhatAmI()==0xF` | false because not `AircraftClass` | Yes, conditional on being high-flying for the AA legality branch |
| True `AircraftClass` target | High-flying branch when airborne; requires projectile `AA=yes` | forced `0` because `WhatAmI()==2` | true | Yes |

For `AAHeatSeeker2`, both `AA=yes` and `AG=yes` are set, so the legality result is the same for these three normal targets unless another generic `GetFireError` condition fails. The projectile behavior after launch is not the same: only true `AircraftClass` targets get the arming override and the homing aircraft flag.

## 5. INI Keys

| INI key | Location | Verified effect in this slice | Active in YR |
|---|---|---|---|
| `[MissileLauncher] Projectile=AAHeatSeeker2` | `rulesmd.ini:22569..22575` | Selects projectile read through `WeaponType+0xA0` | Yes |
| `[MissileLauncher] Speed=30` | `rulesmd.ini:22575` | Not a target predicate; carried into bullet motion per parent report | Yes |
| `[AAHeatSeeker2] Arm=2` | `rulesmd.ini:25678..25679` | Default arming delay, overridden to zero only for `WhatAmI()==2` target | Yes |
| `[AAHeatSeeker2] AA=yes` | `rulesmd.ini:25684` | Projectile `+0x2A4`; allows high-flying target legality | Yes |
| `[AAHeatSeeker2] AG=yes` | `rulesmd.ini:25685` | Projectile `+0x2A5`; allows ground/non-air target legality | Yes |
| `[AAHeatSeeker2] ROT=60` | `rulesmd.ini:25687` | Selects ROT>0 homing AI branch | Yes |
| `[JUMPJET] ConsideredAircraft=yes` | `rulesmd.ini:3951` | Parsed to `TechnoTypeClass+0xD96`; not directly read by the four functions | Yes |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::GetFireError` AA/AG target legality | verified | `0x006FC0B0`, projectile pointer `WeaponType+0xA0`, projectile bytes `+0x2A4/+0x2A5` | none for this slice |
| `ObjectClass::IsHighFlying` | verified | `0x005F6B90` height check against `DAT_00AC13C8 * 2` | exact runtime init of `DAT_00AC13C8` belongs to spatial primitive docs |
| `BulletClass::Fire` arming override | verified | `0x00468670`, assembly around `0x00468A3F..0x00468A6E` | none |
| `ProximityDetector::Set` / `Check` arming meaning | verified | `0x004E1130`, `0x004E11F0` | none for target-type slice |
| `BulletClass::AI` homing aircraft flag | verified | `0x004666E0`, assembly around `0x00466CD8..0x00466D31` | none |
| `BulletClass::HomingTrack` aircraft flag effect | verified | `0x005B20F0` | exact global altitude constant initialization not rederived here |
| `InfantryClass::WhatAmI` | verified | `0x00523340` returns `0xF` | none |
| `AircraftClass::WhatAmI` | verified | `0x0041C180` assembly `MOV EAX,0x2; RET` | none |
| `ConsideredAircraft` parser | verified | `0x00714FE9..0x00715003`, write to `+0xD96` | full threat-routing consumers out of scope |
| Rocketeer normal airborne state | touched-not-exhausted | `rulesmd.ini [JUMPJET]`; prior audited Rocketeer docs | full jumpjet state-machine edge cases |

## 7. Open Questions - Final State

| ID | Final state |
|---|---|
| OQ-AAH-TGT-001 | RESOLVED: Fire legality uses `IsHighFlying`/ground-style predicates plus projectile `AA`/`AG`, not strict `WhatAmI()==2` (`0x006FC0B0`). |
| OQ-AAH-TGT-002 | RESOLVED: `BulletClass::Fire` overrides arm delay only when target `WhatAmI()==2`; ground and InfantryClass Rocketeer targets keep `Arm=2` (`0x00468670`). |
| OQ-AAH-TGT-003 | RESOLVED: `BulletClass::AI` computes the homing aircraft flag with the same `WhatAmI()==2` check and passes it to `HomingTrack` (`0x004666E0`, `0x00466D31`). |
| OQ-AAH-TGT-004 | RESOLVED: `HomingTrack` uses that flag to choose aircraft-vs-ground altitude/pitch behavior; false enters the ground-target safety-altitude correction branch (`0x005B20F0`). |
| OQ-AAH-TGT-005 | RESOLVED: Rocketeer remains InfantryClass for RTTI (`0x00523340` returns `0xF`) even though `ConsideredAircraft=yes` is parsed at `TechnoTypeClass+0xD96`. |
| OQ-AAH-TGT-006 | DEFERRED: Broader `ConsideredAircraft` threat/acquisition consumers are outside this slot; prior Rocketeer docs cover them at audit level. |

## Sources

- Live Ghidra decompilation / assembly context of `gamemd.exe`:
  - `TechnoClass::GetFireError @ 0x006FC0B0`
  - `BulletClass::Fire @ 0x00468670`
  - `BulletClass::AI @ 0x004666E0`
  - `BulletClass::HomingTrack @ 0x005B20F0`
  - `ObjectClass::IsHighFlying @ 0x005F6B90`
  - `ObjectClass::IsLowFlying @ 0x005F6B60`
  - `ProximityDetector::Set @ 0x004E1130`
  - `ProximityDetector::Check @ 0x004E11F0`
  - `InfantryClass::What_Am_I @ 0x00523340`
  - `AircraftClass::WhatAmI @ 0x0041C180` assembly context
  - `TechnoTypeClass::ReadINI` parse site for `ConsideredAircraft` at `0x00714FE9..0x00715003`
- INI:
  - `ini/rulesmd.ini`
- Prior reports:
  - `GGI_MISSILELAUNCHER_AAHEATSEEKER2_PROJECTILE_LIFECYCLE_GHIDRA_REPORT.md`
  - `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`
  - `BULLETTYPECLASS_GHIDRA_REPORT.md`
  - `OBJECTCLASS_GHIDRA_REPORT.md`
  - `units/allied/JUMPJET.md`
