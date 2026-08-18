# ReadINI Field Maps — gamemd.exe

Struct field maps extracted from decompiled `ReadINI` functions via Ghidra MCP.
Every INI key, its read type, and the struct byte offset where it's stored.

**All offsets are direct byte offsets** (param_1 type is `int` in all cases).

---

## WeaponTypeClass::ReadINI (0x00772080) — 63 keys

Inherits from AbstractTypeClass (Name, UIName, etc.).

| # | INI Key | Type | Offset | Notes |
|---|---------|------|--------|-------|
| 1 | `AmbientDamage` | int | 0x98 | |
| 2 | `Burst` | int | 0x9C | |
| 3 | `Projectile` | string→BulletType* | 0xA0 | Lookup via FindBulletType |
| 4 | `Damage` | int | 0xA4 | |
| 5 | `Speed` | int (0-100→0-255) | 0xA8 | Percentage converted to byte range |
| 6 | `Warhead` | string→WarheadType* | 0xAC | Lookup via FindWarheadType |
| 7 | `ROF` | int | 0xB0 | Rate of Fire (frames) |
| 8 | `Range` | double→leptons | 0xB4 | Cell distance → lepton int |
| 9 | `MinimumRange` | double→leptons | 0xB8 | Cell distance → lepton int |
| 10 | `Report` | sound list | 0xCC-0xD4 | 3 dwords, parsed from comma-separated |
| 11 | `DownReport` | sound list | 0xE8-0xF0 | 3 dwords |
| 12 | `Anim` | anim list | 0xF4+ | Comma-separated AnimType names |
| 13 | `OccupantAnim` | string→AnimType* | 0x110 | |
| 14 | `AssaultAnim` | string→AnimType* | 0x114 | |
| 15 | `OpenToppedAnim` | string→AnimType* | 0x118 | |
| 16 | `AttachedParticleSystem` | string→ParticleSys* | 0x11C | |
| 17 | `LaserInnerColor` | RGB (3 bytes) | 0x120-0x122 | `%d,%d,%d` format |
| 18 | `LaserOuterColor` | RGB (3 bytes) | 0x123-0x125 | |
| 19 | `LaserOuterSpread` | RGB (3 bytes) | 0x126-0x128 | |
| 20 | `UseFireParticles` | bool | 0x129 | |
| 21 | `UseSparkParticles` | bool | 0x12A | |
| 22 | `OmniFire` | bool | 0x12B | |
| 23 | `DistributedWeaponFire` | bool | 0x12C | |
| 24 | `IsRailgun` | bool | 0x12D | |
| 25 | `Lobber` | bool | 0x12E | |
| 26 | `Bright` | bool | 0x12F | |
| 27 | `IsSonic` | bool | 0x130 | |
| 28 | `Spawner` | bool | 0x131 | |
| 29 | `LimboLaunch` | bool | 0x132 | |
| 30 | `DecloakToFire` | bool | 0x133 | |
| 31 | `CellRangefinding` | bool | 0x134 | |
| 32 | `FireOnce` | bool | 0x135 | |
| 33 | `NeverUse` | bool | 0x136 | |
| 34 | `RevealOnFire` | bool | 0x137 | |
| 35 | `TerrainFire` | bool | 0x138 | |
| 36 | `SabotageCursor` | bool | 0x139 | |
| 37 | `MigAttackCursor` | bool | 0x13A | |
| 38 | `DisguiseFireOnly` | bool | 0x13B | |
| 39 | `DisguiseFakeBlinkTime` | int | 0x13C | |
| 40 | `InfiniteMindControl` | bool | 0x140 | |
| 41 | `FireWhileMoving` | bool | 0x141 | |
| 42 | `DrainWeapon` | bool | 0x142 | |
| 43 | `FireInTransport` | bool | 0x143 | |
| 44 | `Suicide` | bool | 0x144 | |
| 45 | `TurboBoost` | bool | 0x145 | |
| 46 | `Supress` | bool | 0x146 | Typo in original: "Supress" not "Suppress" |
| 47 | `Camera` | bool | 0x147 | |
| 48 | `Charges` | bool | 0x148 | |
| 49 | `IsLaser` | bool | 0x149 | |
| 50 | `DiskLaser` | bool | 0x14A | |
| 51 | `IsLine` | bool | 0x14B | |
| 52 | `IsBigLaser` | bool | 0x14C | |
| 53 | `IsHouseColor` | bool | 0x14D | |
| 54 | `LaserDuration` | int→byte | 0x14E | Stored as single byte |
| 55 | `IonSensitive` | bool | 0x14F | |
| 56 | `AreaFire` | bool | 0x150 | |
| 57 | `IsElectricBolt` | bool | 0x151 | |
| 58 | `DrawBoltAsLaser` | bool | 0x152 | |
| 59 | `IsAlternateColor` | bool | 0x153 | |
| 60 | `IsRadBeam` | bool | 0x154 | |
| 61 | `IsRadEruption` | bool | 0x155 | |
| 62 | `RadLevel` | int | 0x158 | |
| 63 | `IsMagBeam` | bool | 0x15C | |

---

## WarheadTypeClass::ReadINI (0x0075D590) — 43 keys

| # | INI Key | Type | Offset | Notes |
|---|---------|------|--------|-------|
| 1 | `Deform` | double | 0x98 | Terrain deformation intensity |
| 2 | `Verses` | 11×double | 0xA0-0xF7 | 11 armor multipliers, `%` or float format |
| 3 | `ProneDamage` | double | 0xF8 | Prone infantry damage modifier |
| 4 | `DeformThreshhold` | int | 0x100 | Typo in original: double 'h' |
| 5 | `AnimList` | anim list | 0x104 | DynVec of AnimType*, comma-separated |
| 6 | `InfDeath` | int | 0x120 | Infantry death anim index |
| 7 | `Sonic` | bool | 0x14B | |
| 8 | `Fire` | bool | 0x14C | |
| 9 | `Rocker` | bool | 0x14E | |
| 10 | `DirectRocker` | bool | 0x14F | |
| 11 | `Bright` | bool | 0x150 | |
| 12 | `CLDisableRed` | bool | 0x151 | |
| 13 | `CLDisableGreen` | bool | 0x152 | |
| 14 | `CLDisableBlue` | bool | 0x153 | |
| 15 | `EMEffect` | bool | 0x154 | |
| 16 | `MindControl` | bool | 0x155 | |
| 17 | `Poison` | bool | 0x156 | |
| 18 | `IvanBomb` | bool | 0x157 | |
| 19 | `ElectricAssault` | bool | 0x158 | |
| 20 | `Parasite` | bool | 0x159 | |
| 21 | `Temporal` | bool | 0x15A | |
| 22 | `IsLocomotor` | bool | 0x15B | |
| 23 | `Locomotor` | CLSID (16B) | 0x15C | GUID for locomotor class |
| 24 | `Airstrike` | bool | 0x16C | |
| 25 | `Psychedelic` | bool | 0x16D | |
| 26 | `BombDisarm` | bool | 0x16E | |
| 27 | `Paralyzes` | int | 0x170 | Duration in frames |
| 28 | `Culling` | bool | 0x174 | |
| 29 | `MakesDisguise` | bool | 0x175 | |
| 30 | `NukeMaker` | bool | 0x176 | |
| 31 | `Radiation` | bool | 0x177 | |
| 32 | `PsychicDamage` | bool | 0x178 | |
| 33 | `AffectsAllies` | bool | 0x179 | |
| 34 | `Bullets` | bool | 0x17A | |
| 35 | `Veinhole` | bool | 0x17B | |
| 36 | `ShakeXlo` | int | 0x17C | |
| 37 | `ShakeXhi` | int | 0x180 | |
| 38 | `ShakeYlo` | int | 0x184 | |
| 39 | `ShakeYhi` | int | 0x188 | |
| 40 | `DebrisTypes` | string list | 0x18C | DynVec of VoxelAnimType names |
| 41 | `DebrisMaximums` | int list | 0x1A8 | DynVec, parallel to DebrisTypes |
| 42 | `MaxDebris` | int | 0x1C4 | Clamped ≥ MinDebris |
| 43 | `MinDebris` | int | 0x1C8 | Clamped ≥ 0 |

**Derived:** `AffectsNothing` (0x149) = true if Verses[4] and Verses[6] both == 0.0

---

## BulletTypeClass::ReadINI (0x0046BEE0) — 42 keys

Inherits from ObjectTypeClass (Name, Image, etc.).

Two section contexts: most keys from `[TypeName]`, but keys marked **Image** read from the Image section.

| # | INI Key | Type | Offset | Section | Notes |
|---|---------|------|--------|---------|-------|
| 1 | `Airburst` | bool | 0x294 | TypeName | |
| 2 | `Floater` | bool | 0x295 | TypeName | |
| 3 | `SubjectToCliffs` | bool | 0x296 | TypeName | |
| 4 | `SubjectToElevation` | bool | 0x297 | TypeName | |
| 5 | `SubjectToWalls` | bool | 0x298 | TypeName | |
| 6 | `VeryHigh` | bool | 0x299 | TypeName | |
| 7 | `Shadow` | bool | 0x29A | TypeName | |
| 8 | `Arcing` | bool | 0x29B | TypeName | |
| 9 | `Dropping` | bool | 0x29C | TypeName | |
| 10 | `Level` | bool | 0x29D | TypeName | |
| 11 | `Inviso` | bool | 0x29E | TypeName | |
| 12 | `Proximity` | bool | 0x29F | TypeName | |
| 13 | `Ranged` | bool | 0x2A0 | TypeName | |
| 14 | `Rotates` | bool (inverted) | 0x2A1 | **Image** | Stored as !value |
| 15 | `Inaccurate` | bool | 0x2A2 | TypeName | |
| 16 | `FlakScatter` | bool | 0x2A3 | TypeName | |
| 17 | `AA` | bool | 0x2A4 | TypeName | Anti-air |
| 18 | `AG` | bool | 0x2A5 | TypeName | Anti-ground |
| 19 | `Degenerates` | bool | 0x2A6 | TypeName | |
| 20 | `Bouncy` | bool | 0x2A7 | TypeName | |
| 21 | `AnimPalette` | bool | 0x2A8 | **Image** | |
| 22 | `FirersPalette` | bool | 0x2A9 | TypeName | |
| 23 | `Cluster` | int | 0x2AC | TypeName | |
| 24 | `AirburstWeapon` | string→WeaponType* | 0x2B0 | TypeName | |
| 25 | `ShrapnelWeapon` | string→WeaponType* | 0x2B4 | TypeName | |
| 26 | `ShrapnelCount` | int | 0x2B8 | TypeName | |
| 27 | `DetonationAltitude` | int | 0x2BC | TypeName | Default 700 |
| 28 | `Vertical` | bool | 0x2C0 | TypeName | |
| 29 | `Elasticity` | double | 0x2C8 | TypeName | |
| 30 | `Acceleration` | int | 0x2D0 | TypeName | |
| 31 | `Color` | string→scheme idx | 0x2D4 | TypeName | Color scheme lookup |
| 32 | `Trailer` | string→AnimType* | 0x2D8 | **Image** | Only if Image set |
| 33 | `ROT` | int | 0x2DC | TypeName | Rate of turn |
| 34 | `CourseLockDuration` | int | 0x2E0 | TypeName | |
| 35 | `SpawnDelay` | int | 0x2E4 | **Image** | Only if Image set |
| 36 | `Scalable` | bool | 0x2EC | TypeName | |
| 37 | `Arm` | int | 0x2F0 | TypeName | Arming delay |
| 38 | `AnimLow` | int→byte | 0x2F4 | **Image** | |
| 39 | `AnimHigh` | int→byte | 0x2F5 | **Image** | |
| 40 | `AnimRate` | int→byte | 0x2F6 | **Image** | |
| 41 | `Flat` | bool | 0x2F7 | **Image** | Only if Image set |
| 42 | `Image` | string (25 chars) | 0x1F8 | TypeName | Art image name |

---

## InfantryTypeClass::ReadINI (0x005240A0) — 36 keys + 42 sequences

Inherits from TechnoTypeClass (hundreds of keys). Rules.ini section = `this+0x24`.

### Rules.ini keys

| # | INI Key | Type | Offset | Notes |
|---|---------|------|--------|-------|
| 1 | `Pip` | pip color enum | 0xDFC | green/yellow/white/red/blue |
| 2 | `OccupyPip` | pip color enum | 0xE00 | |
| 3 | `OccupyWeapon` | string→WeaponType* | 0xE04 | |
| 4 | `EliteOccupyWeapon` | string→WeaponType* | 0xE20 | |
| 5 | `VoiceComment` | sound list | 0xE88+ | Voice comment structure |
| 6 | `DeadBodies` | anim list | 0xE50 | DynVec of AnimType* |
| 7 | `DeathAnims` | anim list | 0xE6C | DynVec of AnimType* |
| 8 | `Cyborg` | bool | 0xEAC | Also sets 0xC8F=1 |
| 9 | `NotHuman` | bool | 0xEAD | |
| 10 | `EnterWaterSound` | string→VocType | 0xEA4 | |
| 11 | `LeaveWaterSound` | string→VocType | 0xEA8 | |
| 12 | `DetectionDistance` | int | 0xEB0 | |
| 13 | `Occupier` | bool | 0xEB4 | |
| 14 | `Assaulter` | bool | 0xEB5 | |
| 15 | `HarvestRate` | int | 0xEB8 | |
| 16 | `Fearless` | bool | 0xEBC | |
| 17 | `Crawls` | bool | 0xEBD | Read from art.ini |
| 18 | `Infiltrate` | bool | 0xEBE | |
| 19 | `Fraidycat` | bool | 0xEBF | |
| 20 | `TiberiumProof` | bool | 0xEC0 | |
| 21 | `Civilian` | bool | 0xEC1 | |
| 22 | `C4` | bool | 0xEC2 | Forces Infiltrate=1 |
| 23 | `Engineer` | bool | 0xEC3 | Forces Infiltrate=1 |
| 24 | `Agent` | bool | 0xEC4 | Forces Infiltrate=1 |
| 25 | `Thief` | bool | 0xEC5 | |
| 26 | `VehicleThief` | bool | 0xEC6 | |
| 27 | `Doggie` | bool | 0xEC7 | |
| 28 | `Deployer` | bool | 0xEC8 | |
| 29 | `DeployedCrushable` | bool | 0xEC9 | |
| 30 | `UseOwnName` | bool | 0xECA | |
| 31 | `JumpJetTurn` | bool | 0xECB | |
| 32 | `Ivan` | bool | 0xEAE | |

### Art.ini keys (section = Image name at 0x1F8)

| # | INI Key | Type | Offset |
|---|---------|------|--------|
| 33 | `FireUp` | int | 0xE40 |
| 34 | `FireProne` | int | 0xE44 |
| 35 | `SecondaryFire` | int | 0xE48 |
| 36 | `SecondaryProne` | int | 0xE4C |

### Sequence system (art.ini, via sub-function 0x00523D00)

Reads `Sequence=` key → uses result as sub-section name.
Then reads 42 sequence entries, each with format `startFrame,frameCount,facingMult,shadowFlag`:

Ready, Guard, Prone, Walk, FireUp, Down, Crawl, Up, FireProne,
Idle1, Idle2, Die1-Die5, Tread, Swim, WetIdle1-2, WetDie1-2,
WetAttack, Tumble, FireFly, Deployed, Deploy, Hover, Fly,
DeployedFire, DeployedIdle, Undeploy, Cheer, Paradrop,
AirDeathStart, AirDeathFalling, AirDeathFinish, Panic,
Shovel, Carry, SecondaryFire, SecondaryProne

Each also reads `<Name>Sounds` for per-frame sound effects.

---

## AircraftTypeClass::ReadINI (0x0041CC20) — 10 keys

Inherits from TechnoTypeClass. Compact extension.

| # | INI Key | Type | Offset | Section | Notes |
|---|---------|------|--------|---------|-------|
| 1 | `Carryall` | bool | 0xDFC | TypeName | |
| 2 | `Trailer` | string→AnimType* | 0xE00 | **Image** | |
| 3 | `SpawnDelay` | int | 0xE04 | **Image** | |
| 4 | `Rotors` | bool | 0xE08 | **Image** | |
| 5 | `CustomRotor` | bool | 0xE09 | **Image** | |
| 6 | `Landable` | bool | 0xE0A | TypeName | |
| 7 | `FlyBy` | bool | 0xE0B | TypeName | |
| 8 | `FlyBack` | bool | 0xE0C | TypeName | |
| 9 | `AirportBound` | bool | 0xE0D | TypeName | |
| 10 | `Fighter` | bool | 0xE0E | TypeName | |

---

## SuperWeaponTypeClass::ReadINI (0x006CEA20) — 22 keys

Inherits from AbstractTypeClass (Name, UIName).

| # | INI Key | Type | Offset | Notes |
|---|---------|------|--------|-------|
| 1 | `Name` | string | 0x64 | From parent (49 chars) |
| 2 | `UIName` | string→CSF | 0x3D | From parent, resolved via StringTable |
| 3 | `WeaponType` | string→WeaponType* | 0x9C | |
| 4 | `RechargeTime` | double→int | 0xB0 | Frames, only if non-zero |
| 5 | `Type` | string→enum | 0xB4 | SW type enum (12 values) |
| 6 | `Action` | enum | 0xBC | Cursor action enum |
| 7 | `SpecialSound` | string→SoundIdx | 0xC0 | |
| 8 | `StartSound` | string→SoundIdx | 0xC4 | |
| 9 | `AuxBuilding` | string→BuildingType* | 0xC8 | |
| 10 | `SidebarImage` | string (25 chars) | 0xCC | Appends .SHP, loads from MIX |
| 11 | `UseChargeDrain` | bool | 0xE5 | |
| 12 | `IsPowered` | bool | 0xE6 | |
| 13 | `DisableableFromShell` | bool | 0xE7 | |
| 14 | `FlashSidebarTabFrames` | int | 0xE8 | |
| 15 | `AIDefendAgainst` | bool | 0xEC | |
| 16 | `PreClick` | bool | 0xED | |
| 17 | `PostClick` | bool | 0xEE | |
| 18 | `PreDependent` | string→enum | 0xF0 | Same enum as Type |
| 19 | `ShowTimer` | bool | 0xF4 | |
| 20 | `ManualControl` | bool | 0xF5 | |
| 21 | `Range` | double→float | 0xF8 | |
| 22 | `LineMultiplier` | int | 0xFC | |

**SW Type enum values** (from table at 0x008425C0):
0=MultiMissile, 1=IronCurtain, 2=LightningStorm, 3=ChronoSphere,
4=ChronoWarp, 5=ParaDrop, 6=AmerParaDrop, 7=Demolish,
8=AttackMoveTar, 9=AttackMoveNav, 10=PsychicDominator, 11=SpyPlane+

---

## Summary

| Class | Address | Keys | Offset Range |
|-------|---------|------|-------------|
| WeaponTypeClass | 0x00772080 | 63 | 0x98–0x15C |
| WarheadTypeClass | 0x0075D590 | 43 | 0x98–0x1C8 |
| BulletTypeClass | 0x0046BEE0 | 42 | 0x1F8–0x2F7 |
| InfantryTypeClass | 0x005240A0 | 36+42seq | 0xDFC–0xECB |
| AircraftTypeClass | 0x0041CC20 | 10 | 0xDFC–0xE0E |
| SuperWeaponTypeClass | 0x006CEA20 | 22 | 0x3D–0xFC |

**Total: ~216 unique INI keys mapped** (plus 42 infantry sequences).

Notable original-game typos: `Supress` (WeaponType), `DeformThreshhold` (Warhead).

*Generated 2026-03-22 via Ghidra MCP decompilation of gamemd.exe ReadINI vtable slot +0x64.*
