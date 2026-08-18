# WeaponTypeClass Full Struct Layout — Ghidra Research Report

**Date:** 2026-04-06
**Binary:** gamemd.exe (Yuri's Revenge 1.001)
**Confidence:** High (~95%) — all field offsets verified from live decompilation of ReadINI and constructor
**Struct Size:** 0x160 (352 bytes) — confirmed from `operator_new(0x160)` in FindOrAllocate

---

## 1. Critical Note: Pointer Arithmetic

`param_1` in the constructor and ReadINI is typed as `undefined4 *` (i.e., `int *`).
Array-style accesses like `param_1[0x26]` mean **byte offset = 0x26 * 4 = 0x98**.
Direct casts like `*(char *)((int)this + 0x130)` are **direct byte offsets**.

All offsets in this document are **direct byte offsets**.

---

## 2. Inherited Layout (AbstractTypeClass — 0x00 to 0x97)

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0x00 | 4 | void* | VTable_Primary | `vtable__WeaponTypeClass` |
| 0x04 | 4 | void* | VTable_Secondary_4 | `vtable__WeaponTypeClass__secondary_4` |
| 0x08 | 4 | void* | VTable_Secondary_8 | `vtable__WeaponTypeClass__secondary_8` |
| 0x0C | 4 | void* | VTable_Secondary_12 | `vtable__WeaponTypeClass__secondary_12` |
| 0x10–0x23 | 20 | — | AbstractClass fields | UniqueID, RTTI, etc. |
| 0x24 | 25 | char[25] | Name | Internal ID string (null-terminated, e.g., "Vulcan") |
| 0x3D | 1 | bool | (unknown flag) | Initialized to 0 |
| 0x60 | 4 | void* | TypeListNode | Pointer into global AbstractTypeClass list |
| 0x64 | 49 | char[49] | UIName | Display name (null-terminated) |
| 0x95–0x97 | 3 | — | (padding) | |

---

## 3. WeaponTypeClass-Specific Fields (0x98 to 0x15F)

### Integer / Pointer Fields

| Offset | Size | Type | INI Key | Default | Confidence | Notes |
|--------|------|------|---------|---------|------------|-------|
| 0x98 | 4 | int | `AmbientDamage=` | 0 | Confirmed | Passive area damage per tick |
| 0x9C | 4 | int | `Burst=` | 1 | Confirmed | Number of shots per firing sequence |
| 0xA0 | 4 | BulletTypeClass* | `Projectile=` | NULL | Confirmed | Resolved via BulletTypeClass::FindOrAllocate |
| 0xA4 | 4 | int | `Damage=` | 0 | Confirmed | Base damage per shot |
| 0xA8 | 4 | int (0-255) | `Speed=` | 0 | Confirmed | Projectile speed; INI value 0-100 mapped to 0-255 via ReadSpeed |
| 0xAC | 4 | WarheadTypeClass* | `Warhead=` | NULL | Confirmed | Resolved via WarheadTypeClass::FindOrCreate |
| 0xB0 | 4 | int | `ROF=` | 0 | Confirmed | Rate of fire in game frames |
| 0xB4 | 4 | int (leptons) | `Range=` | 0 | Confirmed | Max range; INI in cells (double), stored as leptons (×256) via ReadRange |
| 0xB8 | 4 | int (leptons) | `MinimumRange=` | 0 | Confirmed | Min range; same conversion as Range |

### DynamicVectorClass: Report Sound List (0xBC–0xD7, 28 bytes)

| Offset | Size | Type | INI Key | Notes |
|--------|------|------|---------|-------|
| 0xBC | 4 | void* | — | DynamicVectorClass vtable |
| 0xC0 | 4 | void* | — | Buffer pointer |
| 0xC4 | 4 | — | — | Unknown (flags/owner) |
| 0xC8 | 1 | — | — | Padding |
| 0xC9 | 1 | bool | — | Owns buffer flag (corrected 2026-05-28: was 0xC8; binary shows `*(char *)((int)param_1 + 0xc9)` in ReadINI_part1 destructor via `decompile_function 0x00771f50` — ROOT_CAUSE: OFFSET_RETYPED_WRONG) |
| 0xCA–0xCB | 2 | — | — | Padding |
| 0xCC | 4 | int | `Report=` | Sound list data field 1 |
| 0xD0 | 4 | int | — | Sound list data field 2 |
| 0xD4 | 4 | int | — | Sound list data field 3 |

`Report=` is the firing sound effect. Parsed via `CCINIClass::ReadSoundList`.

### DynamicVectorClass: DownReport Sound List (0xD8–0xF3, 28 bytes)

| Offset | Size | Type | INI Key | Notes |
|--------|------|------|---------|-------|
| 0xD8 | 4 | void* | — | DynamicVectorClass vtable |
| 0xDC | 4 | void* | — | Buffer pointer |
| 0xE0 | 4 | — | — | Unknown (flags/owner) |
| 0xE4 | 1 | — | — | Padding |
| 0xE5 | 1 | bool | — | Owns buffer flag (corrected 2026-05-28: was 0xE4; binary shows `*(char *)((int)param_1 + 0xe5)` in ReadINI_part1 destructor via `decompile_function 0x00771f50` — ROOT_CAUSE: OFFSET_RETYPED_WRONG) |
| 0xE6–0xE7 | 2 | — | — | Padding |
| 0xE8 | 4 | int | `DownReport=` | Sound list data field 1 |
| 0xEC | 4 | int | — | Sound list data field 2 |
| 0xF0 | 4 | int | — | Sound list data field 3 |

`DownReport=` is the sound played when firing downward (e.g., from cliffs).

### DynamicVectorClass: Anim List (0xF4–0x10F, 28 bytes)

| Offset | Size | Type | INI Key | Notes |
|--------|------|------|---------|-------|
| 0xF4 | 4 | void* | — | DynamicVectorClass vtable (AnimType* vector) |
| 0xF8 | 4 | void* | — | Buffer pointer |
| 0xFC | 4 | — | — | Unknown (flags/owner) |
| 0x100 | 1 | — | — | Padding |
| 0x101 | 1 | bool | — | Owns buffer flag (corrected 2026-05-28: was 0x100; binary shows `*(char *)((int)param_1 + 0x101)` in ReadINI_part1 destructor via `decompile_function 0x00771f50` — ROOT_CAUSE: OFFSET_RETYPED_WRONG) |
| 0x102–0x103 | 2 | — | — | Padding |
| 0x104 | 4 | int | `Anim=` | Anim list data field 1 |
| 0x108 | 4 | int | — | Anim list data field 2 |
| 0x10C | 4 | int | — | Anim list data field 3 |

`Anim=` is a comma-separated list of AnimTypeClass names (e.g., `MGUN-N,MGUN-NE,...`).
Parsed by tokenizing with `,` and calling `AnimTypeClass::FindByName` for each entry.
Can hold up to 8 directional anims (one per facing).

### AnimType Pointer Fields

| Offset | Size | Type | INI Key | Default | Confidence | Notes |
|--------|------|------|---------|---------|------------|-------|
| 0x110 | 4 | AnimTypeClass* | `OccupantAnim=` | NULL | Confirmed | Anim played when firing from garrisoned building |
| 0x114 | 4 | AnimTypeClass* | `AssaultAnim=` | NULL | Confirmed | Anim played when clearing a garrisoned building |
| 0x118 | 4 | AnimTypeClass* | `OpenToppedAnim=` | NULL | Confirmed | Anim played when firing from open-topped transport |
| 0x11C | 4 | ParticleSystemTypeClass* | `AttachedParticleSystem=` | NULL | Confirmed | Particle system attached to projectile |

### Laser / Beam Color Fields

| Offset | Size | Type | INI Key | Default | Confidence | Notes |
|--------|------|------|---------|---------|------------|-------|
| 0x120 | 3 | RGB (3 bytes) | `LaserInnerColor=` | 0,0,0 | Confirmed | R,G,B for inner laser beam color |
| 0x123 | 3 | RGB (3 bytes) | `LaserOuterColor=` | 0,0,0 | Confirmed | R,G,B for outer laser beam color |
| 0x126 | 3 | RGB (3 bytes) | `LaserOuterSpread=` | 0,0,0 | Confirmed | R,G,B spread for outer laser randomization |

### Boolean Flags (single bytes)

| Offset | Size | Type | INI Key | Default | Confidence | Notes |
|--------|------|------|---------|---------|------------|-------|
| 0x129 | 1 | bool | `UseFireParticles=` | false | Confirmed | Use fire particle effects on impact |
| 0x12A | 1 | bool | `UseSparkParticles=` | false | Confirmed | Use spark particle effects on impact |
| 0x12B | 1 | bool | `OmniFire=` | false | Confirmed | Can fire in any direction without turning |
| 0x12C | 1 | bool | `DistributedWeaponFire=` | false | Confirmed | Distribute fire across multiple targets |
| 0x12D | 1 | bool | `IsRailgun=` | false | Confirmed | Uses railgun visual effect |
| 0x12E | 1 | bool | `Lobber=` | false | Confirmed | Lob projectile in high arc |
| 0x12F | 1 | bool | `Bright=` | false | Confirmed | Weapon flash illuminates area |
| 0x130 | 1 | bool | `IsSonic=` | false | Confirmed | Uses sonic/sound wave visual |
| 0x131 | 1 | bool | `Spawner=` | false | Confirmed | Weapon spawns child units (e.g., aircraft carrier) |
| 0x132 | 1 | bool | `LimboLaunch=` | false | Confirmed | Launch projectile from limbo (off-map) |
| 0x133 | 1 | bool | `DecloakToFire=` | true | Confirmed | Must decloak before firing |
| 0x134 | 1 | bool | `CellRangefinding=` | false | Confirmed | Use cell-center for range calculation |
| 0x135 | 1 | bool | `FireOnce=` | false | Confirmed | Fire only once then stop |
| 0x136 | 1 | bool | `NeverUse=` | false | Confirmed | AI will never select this weapon |
| 0x137 | 1 | bool | `RevealOnFire=` | true | Confirmed | Reveal firing unit on minimap |
| 0x138 | 1 | bool | `TerrainFire=` | false | Confirmed | Can target terrain/ground |
| 0x139 | 1 | bool | `SabotageCursor=` | false | Confirmed | Show sabotage cursor when targeting |
| 0x13A | 1 | bool | `MigAttackCursor=` | false | Confirmed | Show MiG attack cursor |
| 0x13B | 1 | bool | `DisguiseFireOnly=` | false | Confirmed | Only fire while disguised |
| 0x13C | 4 | int | `DisguiseFakeBlinkTime=` | 0 | Confirmed | Blink duration when firing while disguised |
| 0x140 | 1 | bool | `InfiniteMindControl=` | false | Confirmed | No limit on mind-controlled units |
| 0x141 | 1 | bool | `FireWhileMoving=` | true | Confirmed | Can fire without stopping; constructor default=1 |
| 0x142 | 1 | bool | `DrainWeapon=` | false | Confirmed | Drains target (like Yuri Prime drain) |
| 0x143 | 1 | bool | `FireInTransport=` | true | Confirmed | Can fire from inside transport; constructor default=1 |
| 0x144 | 1 | bool | `Suicide=` | false | Confirmed | Unit dies after firing |
| 0x145 | 1 | bool | `TurboBoost=` | false | Confirmed | Projectile has turbo boost (AA missiles) |
| 0x146 | 1 | bool | `Supress=` | false | Confirmed | **Note: typo in original INI key** — suppress friendly fire check |
| 0x147 | 1 | bool | `Camera=` | false | Confirmed | Reveal area around impact |
| 0x148 | 1 | bool | `Charges=` | false | Confirmed | Uses limited charges |
| 0x149 | 1 | bool | `IsLaser=` | false | Confirmed | Draw laser beam visual |
| 0x14A | 1 | bool | `DiskLaser=` | false | Confirmed | Draw disk-shaped laser (Floating Disc) |
| 0x14B | 1 | bool | `IsLine=` | false | Confirmed | Draw line visual |
| 0x14C | 1 | bool | `IsBigLaser=` | false | Confirmed | Use thick laser visual |
| 0x14D | 1 | bool | `IsHouseColor=` | false | Confirmed | Laser uses house/player color |
| 0x14E | 1 | int8 | `LaserDuration=` | 10 | Confirmed | Duration of laser visual in frames; constructor default=10 at offset 0x14E |
| 0x14F | 1 | bool | `IonSensitive=` | false | Confirmed | Affected by Ion Storm |
| 0x150 | 1 | bool | `AreaFire=` | false | Confirmed | Fire at own cell (area effect weapons) |
| 0x151 | 1 | bool | `IsElectricBolt=` | false | Confirmed | Draw electric bolt visual (Tesla) |
| 0x152 | 1 | bool | `DrawBoltAsLaser=` | false | Confirmed | Render bolt using laser renderer |
| 0x153 | 1 | bool | `IsAlternateColor=` | false | Confirmed | Use alternate color for beam |
| 0x154 | 1 | bool | `IsRadBeam=` | false | Confirmed | Draw radiation beam visual |
| 0x155 | 1 | bool | `IsRadEruption=` | false | Confirmed | Radiation eruption effect on detonation |
| 0x156–0x157 | 2 | — | — | — | — | Padding |
| 0x158 | 4 | int | `RadLevel=` | 0 | Confirmed | Radiation level deposited on impact |
| 0x15C | 1 | bool | `IsMagBeam=` | false | Confirmed | Magnetron beam visual |
| 0x15D–0x15F | 3 | — | — | — | — | Padding to 0x160 |

---

## 4. Default Values Summary (from Constructor)

The full constructor at `0x00771c70` initializes via `param_1` as `int *`:

| Constructor Access | Byte Offset | Default | Field |
|--------------------|-------------|---------|-------|
| `param_1[0x26] = 0` | 0x98 | 0 | AmbientDamage |
| `param_1[0x27] = 1` | 0x9C | 1 | Burst |
| `param_1[0x28] = 0` | 0xA0 | NULL | Projectile |
| `param_1[0x29] = 0` | 0xA4 | 0 | Damage |
| `param_1[0x2a] = 0` | 0xA8 | 0 | Speed |
| `param_1[0x2b] = 0` | 0xAC | NULL | Warhead |
| `param_1[0x2c] = 0` | 0xB0 | 0 | ROF |
| `param_1[0x2d] = 0` | 0xB4 | 0 | Range |
| `param_1[0x2e] = 0` | 0xB8 | 0 | MinimumRange |
| `param_1[0x34] = 10` | 0xD0 | — | Report DVC internal |
| `param_1[0x33] = 0` | 0xCC | — | Report DVC internal |
| `param_1[0x3b] = 10` | 0xEC | — | DownReport DVC internal |
| `param_1[0x3a] = 0` | 0xE8 | — | DownReport DVC internal |
| `param_1[0x42] = 10` | 0x108 | — | Anim DVC internal |
| `param_1[0x41] = 0` | 0x104 | — | Anim DVC internal |
| `param_1[0x44] = 0` | 0x110 | NULL | OccupantAnim |
| `param_1[0x45] = 0` | 0x114 | NULL | AssaultAnim |
| `param_1[0x46] = 0` | 0x118 | NULL | OpenToppedAnim |
| `param_1[0x47] = 0` | 0x11C | NULL | AttachedParticleSystem |
| offset 0x120–0x128 | — | 0,0,0 | LaserInnerColor, LaserOuterColor, LaserOuterSpread |
| offset 0x129 | — | false | UseFireParticles |
| offset 0x12A | — | false | UseSparkParticles |
| offset 0x12B | — | false | OmniFire |
| offset 0x12C | — | false | DistributedWeaponFire |
| offset 0x12D | — | false | IsRailgun |
| offset 0x12E | — | false | Lobber |
| offset 0x12F | — | false | Bright |
| offset 0x130 | — | false | IsSonic |
| offset 0x131 | — | false | Spawner |
| offset 0x132 | — | false | LimboLaunch |
| offset 0x133 | — | **true** | DecloakToFire |
| offset 0x134 | — | false | CellRangefinding |
| offset 0x135 | — | false | FireOnce |
| offset 0x136 | — | false | NeverUse |
| offset 0x137 | — | **true** | RevealOnFire |
| offset 0x138 | — | false | TerrainFire |
| offset 0x139 | — | false | SabotageCursor |
| offset 0x13A | — | false | MigAttackCursor |
| offset 0x13B | — | false | DisguiseFireOnly |
| `param_1[0x4f] = 0` | 0x13C | 0 | DisguiseFakeBlinkTime |
| offset 0x140 | — | false | InfiniteMindControl |
| offset 0x141 | — | **true** | FireWhileMoving |
| offset 0x142 | — | false | DrainWeapon |
| offset 0x143 | — | **true** | FireInTransport |
| offset 0x144 | — | false | Suicide |
| offset 0x145 | — | false | TurboBoost |
| offset 0x146 | — | false | Supress |
| offset 0x147 | — | false | Camera |
| offset 0x148 | — | false | Charges |
| offset 0x149 | — | false | IsLaser |
| offset 0x14A | — | false | DiskLaser |
| offset 0x14B | — | false | IsLine |
| offset 0x14C | — | false | IsBigLaser |
| offset 0x14D | — | false | IsHouseColor |
| offset 0x14E | — | 10 | LaserDuration |
| offset 0x14F | — | false | IonSensitive |
| offset 0x150 | — | false | AreaFire |
| offset 0x151 | — | false | IsElectricBolt |
| offset 0x152 | — | false | DrawBoltAsLaser |
| offset 0x153 | — | false | IsAlternateColor |
| offset 0x154 | — | false | IsRadBeam |
| offset 0x155 | — | false | IsRadEruption |
| `param_1[0x56] = 0` | 0x158 | 0 | RadLevel |
| offset 0x15C | — | false | IsMagBeam |

---

## 5. INI Parsing Order (in ReadINI at 0x00772080)

The fields are parsed in the following order within `WeaponTypeClass::ReadINI`:

1. `AmbientDamage=` → int → 0x98
2. `IsSonic=` → bool → 0x130
3. `Spawner=` → bool → 0x131
4. `LimboLaunch=` → bool → 0x132
5. `DecloakToFire=` → bool → 0x133
6. `CellRangefinding=` → bool → 0x134
7. `FireOnce=` → bool → 0x135
8. `NeverUse=` → bool → 0x136
9. `RevealOnFire=` → bool → 0x137
10. `TerrainFire=` → bool → 0x138
11. `SabotageCursor=` → bool → 0x139
12. `MigAttackCursor=` → bool → 0x13A
13. `DisguiseFireOnly=` → bool → 0x13B
14. `InfiniteMindControl=` → bool → 0x140
15. `FireWhileMoving=` → bool → 0x141
16. `DrainWeapon=` → bool → 0x142
17. `FireInTransport=` → bool → 0x143
18. `DisguiseFakeBlinkTime=` → int → 0x13C
19. `Suicide=` → bool → 0x144
20. `Supress=` → bool → 0x146
21. `Burst=` → int → 0x9C
22. `Damage=` → int → 0xA4
23. `Speed=` → ReadSpeed (0–100→0–255) → 0xA8
24. `ROF=` → int → 0xB0
25. `Range=` → ReadRange (cells→leptons) → 0xB4
26. `MinimumRange=` → ReadRange → 0xB8
27. `Report=` → ReadSoundList → 0xBC–0xD7 (DVC)
28. `DownReport=` → ReadSoundList → 0xD8–0xF3 (DVC)
29. `Anim=` → comma-separated AnimTypeClass names → 0xF4–0x10F (DVC)
30. `AssaultAnim=` → AnimTypeClass* → 0x114
31. `OccupantAnim=` → AnimTypeClass* → 0x110
32. `OpenToppedAnim=` → AnimTypeClass* → 0x118
33. `Camera=` → bool → 0x147
34. `IsLaser=` → bool → 0x149
35. `DiskLaser=` → bool → 0x14A
36. `IsLine=` → bool → 0x14B
37. `IsHouseColor=` → bool → 0x14D
38. `Charges=` → bool → 0x148
39. `TurboBoost=` → bool → 0x145
40. `UseFireParticles=` → bool → 0x129
41. `UseSparkParticles=` → bool → 0x12A
42. `OmniFire=` → bool → 0x12B
43. `DistributedWeaponFire=` → bool → 0x12C
44. `IsRailgun=` → bool → 0x12D
45. `Lobber=` → bool → 0x12E
46. `LaserInnerColor=` → RGB → 0x120–0x122
47. `LaserOuterColor=` → RGB → 0x123–0x125
48. `LaserOuterSpread=` → RGB → 0x126–0x128
49. `LaserDuration=` → int8 → 0x14E
50. `IsBigLaser=` → bool → 0x14C
51. `Bright=` → bool → 0x12F
52. `IonSensitive=` → bool → 0x14F
53. `AreaFire=` → bool → 0x150
54. `IsElectricBolt=` → bool → 0x151
55. `DrawBoltAsLaser=` → bool → 0x152
56. `IsAlternateColor=` → bool → 0x153
57. `IsRadBeam=` → bool → 0x154
58. `IsRadEruption=` → bool → 0x155
59. `RadLevel=` → int → 0x158
60. `IsMagBeam=` → bool → 0x15C
61. `AttachedParticleSystem=` → ParticleSystemTypeClass* → 0x11C
62. `Warhead=` → WarheadTypeClass* → 0xAC
63. `Projectile=` → BulletTypeClass* → 0xA0

---

## 6. Methods Found and Addresses

| Address | Name | Notes |
|---------|------|-------|
| `0x00771c70` | `WeaponTypeClass::Constructor` | Full constructor with field initialization |
| `0x00771f00` | `WeaponTypeClass::Constructor` | Minimal/copy constructor variant |
| `0x00771f50` | `WeaponTypeClass::Destructor` | Destructor: detaches from all lists, removes from global array, frees DVC buffers, calls AbstractTypeClass::Constructor (corrected 2026-05-28: was labeled ReadINI_part1; decompile shows Detach_From_All_Lists + array removal + DVC cleanup via `decompile_function 0x00771f50` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| `0x00772080` | `WeaponTypeClass::ReadINI` | Main INI parsing — reads all 63 fields |
| `0x00772fa0` | `WeaponTypeClass::FindOrAllocate` | Find by name or allocate new (0x160 bytes) |
| `0x00773030` | `WeaponTypeClass::FindByName` | Linear search through global array by Name field |

### Global Data

| Address | Description |
|---------|-------------|
| `0x0088756C` | WeaponTypeClass array buffer pointer |
| `0x00887570` | WeaponTypeClass array count (current) |
| `0x00887578` | WeaponTypeClass array capacity |
| `0x00849250` | RTTI string: `.?AVWeaponTypeClass@@` |

---

## 7. Field Cross-Reference with rulesmd.ini

All 63 INI keys found in ReadINI were verified against `rulesmd.ini`. The following keys
are actively used by standard YR weapons:

**Commonly used:** Damage, ROF, Range, Speed, Projectile, Warhead, Report, Burst, Anim,
OmniFire, Suicide, AreaFire, TurboBoost, IsLaser, DiskLaser, IsHouseColor, LaserDuration,
LaserOuterSpread, MinimumRange, FireOnce, IsElectricBolt, IsRadBeam, RadLevel, AssaultAnim,
Bright, Spawner

**Less common but used:** DecloakToFire, RevealOnFire, Camera, IsSonic, DrainWeapon,
InfiniteMindControl, LimboLaunch, FireInTransport, OccupantAnim, OpenToppedAnim,
AttachedParticleSystem, NeverUse, IsMagBeam, CellRangefinding, Lobber, IsRadEruption,
IsBigLaser, FireWhileMoving, AmbientDamage

**Rarely or never used in standard YR ini:** DisguiseFireOnly, DisguiseFakeBlinkTime,
SabotageCursor, MigAttackCursor, TerrainFire, Charges, Supress, DistributedWeaponFire,
IsRailgun, DrawBoltAsLaser, IsAlternateColor, IsLine, IonSensitive, DownReport,
LaserInnerColor, LaserOuterColor, UseFireParticles, UseSparkParticles

---

## 8. TS Legacy / Dead Code Analysis

Most fields in WeaponTypeClass appear to be actively used in YR. However, some are
suspicious:

| Field | Assessment |
|-------|-----------|
| `TerrainFire=` | Parsed but no standard YR weapon uses it. Likely TS holdover. |
| `SabotageCursor=` | No standard YR weapon sets this. Related to TS saboteur mechanic. |
| `MigAttackCursor=` | Named after TS MiG. No YR weapon sets it. Likely TS cursor. |
| `Charges=` | No standard YR weapon uses it. TS limited ammo mechanic. |
| `IonSensitive=` | No standard YR weapon uses it. TS Ion Storm mechanic — but Ion Storms DO exist in YR maps, so this may be live. |
| `IsLine=` | No standard YR weapon uses it. TS visual variant. |
| `IsAlternateColor=` | No standard YR weapon uses it. TS beam color variant. |
| `DownReport=` | No standard YR weapon uses it. TS downward firing sound. |
| `UseFireParticles=` | No standard YR weapon uses it. TS particle effect. |
| `UseSparkParticles=` | No standard YR weapon uses it. TS particle effect. |

**All fields are actively parsed in YR's ReadINI** — none are gated behind feature flags.
They are available for modders even if the base game doesn't use them. For implementation
purposes, all should be parsed but the TS-specific ones can be deprioritized for actual
gameplay effect implementation.

---

## 9. Speed and Range Conversion Details

### Speed (ReadSpeed at 0x00474810)
```
Input: INI integer value 0–100
Conversion: clamp to 0–100, then (value * 256) / 100, clamp to 0–255
Output: internal speed in leptons per frame (0–255)
Special: -1 (or missing) = use existing default
```

### Range (ReadRange at 0x00474620)
```
Input: INI double value in cells (e.g., 5.5)
Conversion: read as double, convert to leptons via Math::ftol (×256 implied)
Output: internal range in leptons
Special: -1.0 (or missing) = use existing default
```

---

## 10. Report / DownReport Sound List Format

The `Report=` and `DownReport=` values are parsed by `CCINIClass::ReadSoundList` which
returns a DynamicVectorClass of sound indices. The data is stored as 3 consecutive dwords
within the DVC structure at offsets +0x10, +0x14, +0x18 from the DVC start.

---

## 11. Anim List Format

`Anim=` accepts a comma-separated list of AnimTypeClass names. Each is resolved via
`AnimTypeClass::FindByName` and stored in a DynamicVectorClass of AnimTypeClass pointers.
Up to 8 entries (one per facing direction: N, NE, E, SE, S, SW, W, NW).

Example: `Anim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW`
