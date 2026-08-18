# RulesClass — Ghidra Research Report

**Primary addresses:**
- `g_RulesClass_Instance` pointer — `0x008871E0` (stores `RulesClass*`)
- Constructor (sets defaults) — `0x00665650` (body ends `0x00667A26`, ~9174 B)
- Destructor — `0x00667A30`
- Outer orchestrator `RulesClass::Process` (reads INI + allocates type arrays) — `0x006686C0` (mis-labelled `CDFileClass__Constructor` in Ghidra)
- Inner orchestrator `RulesClass::Read_INI` (section dispatch) — `0x00668BF0`
- RulesClass instance size — `0x18C0` bytes (6336 decimal)

**Confidence (overall):** HIGH for structural claims, singleton, section ownership, field offsets within the smaller Read methods that were fully decompiled; MEDIUM for internal `[General]`/`[AudioVisual]` field offsets (methods too large to decompile end-to-end in a single pass; documented from confirmed INI-key strings + sample reads).

**Active in YR:** Yes — RulesClass is loaded every time a game starts (skirmish, campaign, multiplayer). Several individual fields it populates are Tiberian Sun legacy and inert in YR; those are called out per section below.

---

## 1. Overview

`RulesClass` is the global singleton that holds all *game-wide tuning data* read from the top-level sections of `rulesmd.ini` (with `rules.ini` as the base fallback). It is NOT per-type data (no weapons, infantry, buildings, warheads fields live on it) — those live in `WeaponTypeClass`, `InfantryTypeClass`, etc. RulesClass owns:

- gameplay globals (veterancy, repair, build speed, survivor divisors, harvester scans, chrono timers)
- combat globals (Iron Curtain duration, occupy/bunker multipliers, overload damage, deathweapon)
- AI parameters (build lists, ratios/limits, power emergency, base spacing)
- IQ thresholds (SuperWeapons, GuardArea, Scatter, etc.)
- difficulty multipliers (three embedded copies: Easy, Normal, Difficult — via `DifficultyClass::Read_INI`)
- speed×land-type movement table (hover/foot/track/wheel/float/amphibious/subterranean × each LandType)
- special-weapon warhead/projectile bindings (Nuke, EMP, Mutate)
- crate rules, radiation, elevation model, wall model
- multiplayer dialog defaults (money, unit count, tech level, FogOfWar, Bases, etc.)
- color-add remap (16-entry RGB table), radar event params
- animation/sound bindings used globally (warp anims, debris, scorches, etc.)

Lifetime: allocated once via `operator new(0x18C0)` in `Init_Game` (`0x0052BAD8`), populated by `ScenarioClass::Full_Init` (`0x00686B20`) before the tick loop starts, freed on shutdown in `Game_Shutdown` (`0x006BE1C0`).

**Vtable:** NONE. Confirmed by reading the first instructions of the constructor at `0x00665650`:

```
55              PUSH   EBP
8B EC           MOV    EBP, ESP
83 E4 F8        AND    ESP, 0xFFFFFFF8     ; stack align
83 EC 08        SUB    ESP, 8
53 55 56        PUSH   EBX, EBP, ESI
8B F1           MOV    ESI, ECX             ; ESI = this
B8 05 00 00 00  MOV    EAX, 5
33 DB           XOR    EBX, EBX
C7 06 0F 00 00 00   MOV DWORD PTR [ESI], 0x0F       ; *(this+0) = 15  (NOT a vtable pointer)
C7 46 04 14 00 00 00 MOV DWORD PTR [ESI+4], 0x14
89 46 08        MOV DWORD PTR [ESI+8], EAX          ; = 5
...
```

The first write to `this+0` is the immediate literal `0x0000000F` (= 15), not a pointer into the `0x0088xxxx`/`0x007Exxxx` vtable region. A global search for `vtable__Rules*` symbols also returns zero hits. So `RulesClass` is a plain (non-polymorphic) singleton — its first 4 bytes are a regular int field (default 15), and a Rust port should model it as a flat `struct Rules { ... }` with no leading vtable slot. This matches the fact that `RulesClass::Process` / `RulesClass::Read_INI` are called by direct address from `ScenarioClass::Full_Init`, not through any dispatch table.

---

## 2. Class Layout — verified key offsets

All offsets are **direct byte offsets** from the `RulesClass*` (the function prototypes pass it as `int`, not `int*`, so `*(T*)(this + N)` in the decompile is literal bytes).

### `[CrateRules]` (0066B900)
| INI Key | Type | Offset |
|---|---|---|
| FreeMCV | bool | `0x40` |
| WoodCrateImg | string→OverlayType* | `0xF8` |
| CrateImg | string→OverlayType* | `0xFC` |
| WaterCrateImg | string→OverlayType* | `0x100` |
| HealCrateSound | sound idx | `0x718` |
| CrateMinimum | int | `0x1470` |
| CrateMaximum | int | `0x1474` |
| CrateRadius | range | `0x172C` |
| CrateRegen | double | `0x1678` |
| UnitCrateType | string→UnitType* | `0x1148` |
| SoloCrateMoney | int | `0x1140` |
| SilverCrate | enum | `0x1464` |
| WoodCrate | enum | `0x1468` |
| WaterCrate | enum | `0x146C` |

### `[Radiation]` (0066CF70)
| INI Key | Type | Offset |
|---|---|---|
| RadDurationMultiple | int | `0x1804` |
| RadApplicationDelay | int | `0x1808` |
| RadLevelMax | int | `0x180C` |
| RadLevelDelay | int | `0x1810` |
| RadLightDelay | int | `0x1814` |
| RadLevelFactor | double | `0x1818` |
| RadLightFactor | double | `0x1820` |
| RadTintFactor | double | `0x1828` |
| RadColor | RGB (3B) | `0x1830` |
| RadSiteWarhead | string→WarheadType* | `0x1834` |

### `[ElevationModel]` (0066D150)
| INI Key | Type | Offset |
|---|---|---|
| ElevationIncrement | int | `0x1838` |
| ElevationIncrementBonus | double | `0x1840` |
| ElevationBonusCap | double | `0x1848` |

### `[WallModel]` (0066D1F0)
| INI Key | Type | Offset |
|---|---|---|
| AlliedWallTransparency | bool | `0x1850` |
| WallPenetratorThreshold | double | `0x1858` |

### `[SpecialWeapons]` (00668FB0)
| INI Key | Type | Offset |
|---|---|---|
| NukeWarhead | WarheadType* | `0xF8C` |
| NukeProjectile | BulletType* | `0xF90` |
| NukeDown | BulletType* | `0xF94` |
| MutateWarhead | WarheadType* | `0xF98` |
| MutateExplosionWarhead | WarheadType* | `0xF9C` |
| EMPulseWarhead | WarheadType* | `0xFA0` |
| EMPulseProjectile | BulletType* | `0xFA4` |

After reading, the method iterates every `SuperWeaponTypeClass` in `g_SuperWeaponTypeClass_Array` and calls its vtable+0x64 (`Read_INI`).

### `[IQ]` (00674240)
| INI Key | Type | Offset |
|---|---|---|
| MaxIQLevels | int | `0x1434` |
| SuperWeapons | int | `0x1438` |
| Production | int | `0x143C` |
| GuardArea | int | `0x1440` |
| RepairSell | int | `0x1444` |
| AutoCrush | int | `0x1448` |
| Scatter | int | `0x144C` |
| ContentScan | int | `0x1450` |
| Aircraft | int | `0x1454` |
| Harvester | int | `0x1458` |
| SellBack | int | `0x145C` |

### `[JumpjetControls]` (006743D0)
| INI Key | Type | Offset |
|---|---|---|
| TurnRate | int | `0x40C` |
| Speed | int | `0x410` |
| Climb | double | `0x418` |
| CruiseHeight | int | `0x420` |
| Acceleration | double | `0x428` |
| WobblesPerSecond | double | `0x430` |
| WobbleDeviation | int | `0x438` |

These are *defaults*; each JumpJet unit can override them on its own type record.

### `[MultiplayerDialogSettings]` (00671EA0)
| INI Key | Type | Offset |
|---|---|---|
| MinMoney | int | `0x1480` |
| Money | int | `0x1484` |
| MaxMoney | int | `0x1488` |
| MoneyIncrement | int | `0x148C` |
| MinUnitCount | int | `0x1490` |
| UnitCount | int | `0x1494` |
| MaxUnitCount | int | `0x1498` |
| TechLevel | int | `0x149C` |
| GameSpeed | int | `0x14A0` |
| AIDifficulty | int | `0x14A4` |
| AIPlayers | int | `0x14A8` |
| BridgeDestruction | bool | `0x14AC` |
| ShadowGrow | bool | `0x14AD` |
| Shroud | bool | `0x14AE` |
| Bases | bool | `0x14AF` |
| TiberiumGrows | bool | `0x14B0` |
| Crates | bool | `0x14B1` |
| CaptureTheFlag | bool | `0x14B2` |
| HarvesterTruce | bool | `0x14B3` |
| MultiEngineer | bool | `0x14B4` |
| AlliesAllowed | bool | `0x14B5` |
| ShortGame | bool | `0x14B6` |
| FogOfWar | bool | `0x14B7` |
| MCVRedeploys | bool | `0x14B8` |
| SuperWeaponsAllowed | bool | `0x14B9` |
| BuildOffAlly | bool | `0x14BA` |
| AllyChangeAllowed | bool | `0x14BB` |

These are **defaults** that the MP dialog later writes back before `ScenarioClass::Full_Init` — so the actual runtime value is whatever the host chose, not the INI value.

### `[Maximums]` (read inside outer orchestrator at `0x006689F3`)
| INI Key | Type | Offset |
|---|---|---|
| Players | int | `0x14D0` |

This single value is used as the player-count cap. Also written into `DAT_00A8B548` during `Init_Game`.

### `[CombatDamage]` (0066BBB0) — partial confirmed offsets
| INI Key | Type | Offset |
|---|---|---|
| BerzerkAllowed | bool | `0x41` |
| AmmoCrateDamage | int | `0xC` |
| IonCannonDamage | int | `0x490` |
| RailgunDamageRadius | int | `0x494` |
| TiberiumExplosionDamage | int | `0x568` |
| TiberiumStrength | int | `0x56C` |
| Scorches (list) | DynVec AnimType* | `0x7C4` |
| Scorches1 (list) | DynVec AnimType* | `0x7E0` |
| Scorches2 (list) | DynVec AnimType* | `0x7FC` |
| Scorches3 (list) | DynVec AnimType* | `0x818` |
| Scorches4 (list) | DynVec AnimType* | `0x834` |
| SplashList | DynVec AnimType* | `0xBC0` |
| MindControlAttackLineFrames | int | `0x310` |
| DrainMoneyFrameDelay | int | `0x314` |
| DrainMoneyAmount | int | `0x318` |
| ControlledAnimationType | AnimType* | `0x320` |
| DrainAnimationType | AnimType* | `0x31C` |
| PermaControlledAnimationType | AnimType* | `0x324` |
| FlameDamage | WarheadType* | `0xF84` |
| FlameDamage2 | WarheadType* | `0xF88` |
| C4Warhead | WarheadType* | `0xFA8` |
| CrushWarhead | WarheadType* | `0xFAC` |
| V3Warhead | WarheadType* | `0xFB0` |
| DMislWarhead | WarheadType* | `0xFB4` |
| V3EliteWarhead | WarheadType* | `0xFB8` |
| DMislEliteWarhead | WarheadType* | `0xFBC` |
| CMislWarhead | WarheadType* | `0xFC0` |
| CMislEliteWarhead | WarheadType* | `0xFC4` |
| IvanWarhead | WarheadType* | `0xFC8` |
| IvanDamage | int | `0xFCC` |
| IvanTimedDelay | int | `0xFD0` |
| CanDetonateTimeBomb | bool | `0xFD4` |
| CanDetonateDeathBomb | bool | `0xFD5` |
| IvanIconFlickerRate | int | `0xFD8` |
| DeathWeapon | WeaponType* | `0xFDC` |
| IvanIconName file refs | (LoadFileFromMIX) | `0xFE0`, `0xFE4` |
| IronCurtainDuration | int | `0xFE8` |
| PsychicRevealRadius | int | `0xFEC` |
| IonCannonWarhead | WarheadType* | `0xFF0` |
| OccupyDamageMultiplier | float | `0xF40` |
| OccupyROFMultiplier | float | `0xF44` |
| OccupyWeaponRange | int | `0xF48` |
| BunkerDamageMultiplier | float | `0xF4C` |
| BunkerROFMultiplier | float | `0xF50` |
| BunkerWeaponRangeBonus | int | `0xF54` |
| OpenToppedDamageMultiplier | float | `0xF58` |
| OpenToppedRangeBonus | int | `0xF5C` |
| OpenToppedWarpDistance | int | `0xF60` |
| OverloadCount (3 ints) | int[3] | `0xEF8, 0xEFC, 0xF00` (DynVec at `0xEE8`) |
| OverloadDamage (3 ints) | int[3] | `0xF14, 0xF18, 0xF1C` (DynVec at `0xF04`) |
| OverloadFrames (3 ints) | int[3] | `0xF30, 0xF34, 0xF38` (DynVec at `0xF20`) |
| FallingDamageMultiplier | float | `0xF64` |
| CurrentStrengthDamage | bool | `0xF68` |
| DefaultLargeGreySmokeSystem | ParticleSys* | `0x1018` |
| DefaultSmallGreySmokeSystem | ParticleSys* | `0x101C` |
| DefaultSparkSystem | ParticleSys* | `0x1020` |
| DefaultLargeRedSmokeSystem | ParticleSys* | `0x1024` |
| DefaultSmallRedSmokeSystem | ParticleSys* | `0x1028` |
| DefaultDebrisSmokeSystem | ParticleSys* | `0x102C` |
| DefaultFireStreamSystem | ParticleSys* | `0x1030` |
| DefaultTestParticleSystem | ParticleSys* | `0x1034` |
| DefaultRepairParticleSystem | ParticleSys* | `0x1038` |
| TurboBoost | double | `0x1098` |
| AtomDamage | int | `0x1530` |
| BallisticScatter | range | `0x1734` |
| BridgeStrength | int | `0x1740` |
| C4Delay | double | `0x1750` |
| Crush | range | `0x1728` |
| ExpSpread | double | `0x1428` |
| FireSupress | range | `0x1430` |
| HomingScatter | range | `0x1730` |
| MaxDamage | int | `0x16C8` |
| MinDamage | int | `0x16C4` |
| TiberiumExplosive | bool | `0x17E5` |
| PlayerAutoCrush | bool | `0x17EB` |
| PlayerReturnFire | bool | `0x17EC` |
| PlayerScatter | bool | `0x17ED` |
| TreeTargeting | bool | `0x17E9` |
| Incoming | speed | `0x16C0` |
| CollapseChance | int | `0x17CC` |

> Note: `TiberiumExplosive` is TS legacy (ore barrels explode on impact). In YR the INI defaults it to `no` and no standard content sets it. See section 7.

### `[AI]` (FUN_00672AE0 — *not renamed in Ghidra yet*) — confirmed offsets
`[AI]` is the largest AI-side section; offsets are split across `0x8AC–0xB1C`, `0x10A0–0x10C0`, `0x1100–0x1768`, `0x17E0–0x17E3`, `0x1460`.

| INI Key | Type | Offset |
|---|---|---|
| BuildConst | DynVec BuildingType* | `0x8AC` |
| BuildPower | DynVec BuildingType* | `0x8C8` |
| BuildRefinery | DynVec BuildingType* | `0x8E4` |
| BuildBarracks | DynVec BuildingType* | `0x900` |
| BuildTech | DynVec BuildingType* | `0x91C` |
| BuildWeapons | DynVec BuildingType* | `0x938` |
| AlliedBaseDefenses | DynVec BuildingType* | `0x954` |
| SovietBaseDefenses | DynVec BuildingType* | `0x970` |
| ThirdBaseDefenses | DynVec BuildingType* | `0x98C` |
| AIForcePredictionFudge (3 ints) | int[3] | `0x9A8`+ |
| BuildDefense | DynVec BuildingType* | `0x9C4` |
| BuildPDefense | DynVec BuildingType* | `0x9E0` |
| BuildAA | DynVec BuildingType* | `0x9FC` |
| BuildHelipad | DynVec BuildingType* | `0xA18` |
| BuildRadar | DynVec BuildingType* | `0xA34` |
| ConcreteWalls | DynVec BuildingType* | `0xA50` |
| NSGates | BuildingType** | `0xA6C` |
| EWGates | BuildingType** | `0xA88` |
| BuildNavalYard | DynVec BuildingType* | `0xAA4` |
| BuildDummy | DynVec BuildingType* | `0xAC0` |
| NeutralTechBuildings | DynVec BuildingType* | `0xADC` |
| AttackInterval | double | `0x10A0` |
| AttackDelay | double | `0x10A8` |
| PatrolScan | double | `0x1150` |
| CreditReserve | int | `0x1758` |
| PathDelay | double | `0x1760` |
| BlockagePathDelay | int | `0x1768` |
| AutocreateTime | double | `0x1510` |
| InfantryReserve | int | `0x1138` |
| InfantryBaseMult | int | `0x113C` |
| PowerSurplus | int | `0x1134` |
| BaseSizeAdd | int | `0x1130` |
| RefineryRatio | double | `0x1128` |
| RefineryLimit | int | `0x1124` |
| BarracksRatio | double | `0x1118` |
| BarracksLimit | int | `0x1120` |
| WarRatio | double | `0x1108` |
| WarLimit | int | `0x1110` |
| DefenseRatio | double | `0x10F8` |
| DefenseLimit | int | `0x1100` |
| AARatio | double | `0x10E8` |
| AALimit | int | `0x10F0` |
| TeslaRatio | double | `0x10D8` |
| TeslaLimit | int | `0x10E0` |
| HelipadRatio | double | `0x10C8` |
| HelipadLimit | int | `0x10D0` |
| AirstripRatio | double | `0x10B8` |
| AirstripLimit | int | `0x10C0` |
| CompEasyBonus | bool | `0x17E3` |
| Paranoid | bool | `0x17E0` |
| PowerEmergency | double | `0x10B0` |
| AIBaseSpacing | int | `0x1460` |
| GDIWallDefense | double | `0xAF8` |
| GDIWallDefenseCoefficient | double | `0xB00` |
| NodBaseDefenseCoefficient | double | `0xB08` |
| GDIBaseDefenseCoefficient | double | `0xB10` |
| MaximumBaseDefenseValue | int | `0xB1C` |
| ComputerBaseDefenseResponse | int | `0xB18` |

### `[General]` (0066D530) — offset map (MEDIUM confidence on individual fields)

`ReadGeneral` is ~18.8 KB of compiled code (≈200+ keys). Individual offsets were NOT exhaustively decompiled in this pass, because the method is too large for a single MCP call and the INI key set is already fully enumerated in `ini/rulesmd.ini`. What IS confirmed:

- The function takes the `RulesClass*` as its first param.
- It uses the standard pattern: `CCINIClass::ReadInt/ReadBool/ReadDouble/ReadString/ReadRange/ReadSpeed/ReadColorRGB/ReadPercent/ReadPercentFloat` each into a direct offset `*(T*)(this + N)`.
- The DifficultyClass *ordering constants* come from vectors of 3 (easy/normal/hard) — e.g. `AIIonCannonConYardValue=100,100,100` parses with `DifficultyClass__ReadINI_IntVector` into three consecutive ints.
- Animation/sound list keys (WarpIn, WarpOut, MetallicDebris, DropPod, DeadBodies, BridgeExplosions, etc.) are parsed the same way as in `[CombatDamage]` — string-tokenized and resolved via `AnimTypeClass::FindByName`.
- String-to-country/infantry/unit lookups (Pilot=E1, Technician=CTECH, etc.) use `FindOrAllocate` on the appropriate type class.

See `ini/rulesmd.ini` `[General]` for the authoritative key list (≈260 keys in rulesmd). Verified structural notes:

- `TunnelSpeed` is stored as speed-units (not float). Read via `ReadSpeed`.
- `FlightLevel` is int leptons.
- `WarpIn/WarpOut/WarpAway/ChronoSparkle1` are single AnimType*.
- `MetallicDebris`/`BridgeExplosions`/`Scorches*` are DynVec<AnimType*>.
- `PrerequisiteXxx` lists are DynVec<BuildingType*> resolved via `BuildingTypeClass::Find_Or_Allocate`.
- Lightning storm parameters (LightningDamage, LightningDeferment, LightningHitDelay, LightningScatterDelay, LightningCellSpread, LightningSeparation, LightningStormDuration) are all `[General]` keys that `LightningStorm::*` later reads from `g_RulesClass_Instance` directly.
- `ParadropRadius`, `FogOfWar`, `Visceroids`, `Meteorites`, `IonStorms`, `MutateExplosion`, `Pilot`, `*Crew`, `Technician`, `Engineer`, `PParatrooper`, `AmerParaDropInf`, `*ParaDropInf/Num`, `AnimToInfantry`, `SecretInfantry/Units/Buildings`, `SpyPowerBlackout`, `SpyMoneyStealPercent`, `AttackCursorOnDisguise`, `DefaultMirageDisguises`, `AlliedBaseDefenseCounts`, `SovietBaseDefenseCounts`, `ThirdBaseDefenseCounts` are all `[General]` — not `[AI]` — keys despite the name theme.
- V3/DMisl/CMisl ballistic-missile globals (`V3RocketPauseFrames` … `CMislBodyLength`, `CMislLazyCurve`, `CMislType`) all live in `[General]` and control the generic ballistic-missile pathing used by the respective unit types.

### `[AudioVisual]` (006691E0) — offset map (MEDIUM confidence)

ReadAudioVisual is ~10 KB of compiled code. Reads a mix of:
- Sound references (VocClass::FindByName → sound index) — ~50 keys (GUI sounds, unit-build sounds, ambient, etc.).
- Animation references (AnimTypeClass::FindByName) — small set.
- Ints/doubles/bools for UI behavior (ShakeScreen, ScrollMultiplier, ShroudRate, FogRate, MessageDelay, etc.).
- Colors: `LineTrailColorOverride`, `ChronoBeamColor`, `MagnaBeamColor`, `LocalRadarColor` as RGB bytes.

Keys like `ConditionRed`, `ConditionYellow`, `DropZoneRadius`, `EnemyHealth`, `NamedCivilians`, `AllyReveal`, `Gravity`, `AmbientChangeRate`, `AmbientChangeStep`, `ExtraUnitLight`, `ExtraInfantryLight`, `ExtraAircraftLight`, `SpeakDelay`, `TimerWarning`, `IceGrowthRate`, `IceSolidifyFrameTime`, `EliteFlashTimer`, `PoseDir`, `DeployDir`, `CloakSound`, `SellSound`, `BuildingDieSound`, `TeslaCharge`, `TeslaZap`, `StormSound`, `LightningSounds`, `PsychicDominatorActivateSound`, `GeneticMutatorActivateSound`, `PsychicRevealActivateSound`, `EnterBioReactorSound`, `EnterGrinderSound`, `SlavesFreeSound` … are all in this method (not verified individually — see INI file).

---

## 3. Speed × LandType Table (`ReadSpeedTypeLandTypeTable` at 0x00674000)

Not stored on RulesClass — stored in a **separate global** at `0x0089EA44` (length 0x180 bytes, ~12 LandType rows × 36 bytes).

Each LandType section (`[Road]`, `[Rough]`, `[Sand]`, `[Beach]`, `[Ice]`, `[Tiberium]`, `[Rock]`, `[Water]`, etc.) has per-SpeedType multipliers parsed:

| Row slot | SpeedType | Default |
|---|---|---|
| `+0x00` | Track | 1.0 |
| `+0x04` | Wheel | 1.0 |
| `+0x08` | Hover | 1.0 |
| `+0x0C` | Winged/Flying | **hardcoded 1.0** (INI ignored) |
| `+0x10` | Float | 1.0 |
| `+0x14` | Amphibious | 1.0 |
| `+0x18` | Subterranean | 1.0 |
| `-0x04` | Foot | 1.0 |
| `+0x1C` | Buildable (bool) | true |

**Every multiplier is clamped to ≤ 1.0 on read.** This is the key behavioral detail: INI can only *reduce* a unit's speed on a terrain (e.g. 0.8 for slow mud) — it can never *boost* speed above the type-class base. Speeds > 1.0 in the INI silently saturate to 1.0.

---

## 4. DifficultyClass embedded instances (`FUN_0066D270`)

RulesClass embeds **three `DifficultyClass` instances** (Easy/Normal/Difficult). Each one is populated by `FUN_0066D270` — a helper that takes a `DifficultyClass*` + section name and reads 12 keys:

| INI Key | Type | Offset in DifficultyClass |
|---|---|---|
| FirePower | double | `+0x00` |
| Groundspeed | double | `+0x08` |
| Airspeed | double | `+0x10` |
| Armor | double | `+0x18` |
| ROF | double | `+0x20` |
| Cost | double | `+0x28` |
| BuildTime | double | `+0x30` |
| RepairDelay | double | `+0x38` (default 0.02) |
| BuildDelay | double | `+0x40` (default 0.03) |
| BuildSlowdown | bool | `+0x48` |
| DestroyWalls | bool | `+0x49` (default true) |
| ContentScan | bool | `+0x4A` |

`sizeof(DifficultyClass) = 0x4C` bytes (0x50 with 4 bytes of tail padding when embedded in RulesClass). The outer orchestrator calls:
```
FUN_0066D270(Rules + 0x1538, "Easy"     );  // Rules->Easy
FUN_0066D270(Rules + 0x1588, "Normal"   );  // Rules->Normal
FUN_0066D270(Rules + 0x15D8, "Difficult");  // Rules->Difficult
```
The three slots are embedded contiguously in RulesClass at offsets
`0x1538`, `0x1588`, `0x15D8` (each `0x50 B`, total `0xF0` for the three).
Confirmed from the raw `LEA EDX, [Rules + slot]` context at each call
site. Full evidence in
[RULESCLASS_DIFFICULTY_SLOTS.md](RULESCLASS_DIFFICULTY_SLOTS.md).

Also: several scalar keys like `AIIonCannonConYardValue=100,100,100` in `[General]` and `OverloadCount/Damage/Frames` in `[CombatDamage]` use a companion helper `DifficultyClass::ReadINI_IntVector` to parse 3 comma-separated ints into three consecutive DWORDs.

---

## 5. Master orchestrators

### Outer — `RulesClass::Process` at `0x006686C0` (signature: `(RulesClass*, CCINIClass*)`)

This is called by `ScenarioClass::Full_Init`. It:

1. **Clears every type-class array** (Infantry, Unit, Aircraft, Building, Anim, Overlay, Warhead, Weapon, Bullet, Particle, ParticleSystem, VoxelAnim, Smudge, Terrain, SuperWeapon, House). Every entry is released via vtable+0x20 (destructor dispatch).
2. Reads `[Maximums] Players` into `this + 0x14D0`.
3. Calls `FUN_00668BF0(this, ini)` — the inner dispatch (see below).
4. Zeroes 16 RGB triples at `this + 0x1874` (the ColorAdd remap table).
5. Reads `LANGRULE.INI` and merges it (localized strings patch).
6. Re-reads `[ColorAdd]` — up to 16 entries of `Name=R,G,B` into the remap table.
7. If `g_GameMode ∈ {in-game, skirmish-loaded}`, opens the current map file and calls `FUN_00668BF0` **again** with the map's `[Rules]`-style overrides (how maps can override global rules).

### Inner — `RulesClass::Read_INI` at `0x00668BF0`

This is the section-by-section reader. Fixed ordering:

| Step | Call | What |
|---|---|---|
| 1 | `RulesClass::ReadColors` (FUN_0066D3A0) | `[Colors]` — per-house palette colour-scheme registration. Populates the global colour table at `0x00886380`/`0x00885780`, **not** a RulesClass field. See the checkpoint-1 CSV for per-key details. |
| 2 | loop `[ColorAdd]` | RGB table at `+0x1874` (physical 16 slots × 3 B = 48 B; stock YR fills 14 slots, remaining 2 are zeroed by the outer orchestrator). Reader is `FUN_0066D480` (`RulesClass::ReadColorAdd`) — see [RULESCLASS_COLORADD_TABLE.md](RULESCLASS_COLORADD_TABLE.md). |
| 3 | loop `[Countries]` | allocate HouseTypeClass per country |
| 4 | `FUN_00672440` | `[Sides]` |
| 5 | loops `[OverlayTypes]`, `[SuperWeaponTypes]`, `[Warheads]`, `[SmudgeTypes]`, `[TerrainTypes]` | allocate each type record |
| 6 | `FUN_00672660` | `[VehicleTypes]` (derived) |
| 7 | `FUN_00672360` | `[AircraftTypes]` |
| 8 | `FUN_006723D0` | `[InfantryTypes]` |
| 9 | `FUN_00672280` | `[BuildingTypes]` |
| 10 | `FUN_006728B0` | `[Animations]` |
| 11 | `FUN_00672920` | `[VoxelAnims]` |
| 12 | `FUN_00672A00` | `[Particles]` |
| 13 | `FUN_00672A70` | `[ParticleSystems]` |
| 14 | `RulesClass::ReadJumpjetControls` | `[JumpjetControls]` |
| 15 | `RulesClass::ReadMultiplayerDialogSettings` | `[MultiplayerDialogSettings]` |
| 16 | `FUN_00672AE0` (ReadAI) | `[AI]` |
| 17 | `FUN_00673E80` (ReadPowerups) | `[Powerups]` — crate bonus-types table, 19 fixed slots × 4 parallel globals (`DAT_0081DA8C` weight, `DAT_0081DAD8` anim, `DAT_0089ECC0` enabled, `DAT_0089EC28` value). Does **not** write RulesClass. Full schema + slot-index table in [RULESCLASS_POWERUPS_TABLE.md](RULESCLASS_POWERUPS_TABLE.md). |
| 18 | `RulesClass::ReadSpeedTypeLandTypeTable` | per-LandType sections → global at `0x0089EA44` |
| 19 | `RulesClass::ReadIQ` | `[IQ]` |
| 20 | `RulesClass::ReadGeneral` | `[General]` |
| 21 | `FUN_00679A10` | **Type_Read_INI_All** — iterates every type-class array and calls vtable+0x64 (`ReadINI`) on each instance, then runs MissionClass::Read_INI for each map mission |
| 22 | `FUN_0066D270(&Rules->Easy, "Easy")` | DifficultyClass fill |
| 23 | `FUN_0066D270(&Rules->Normal, "Normal")` | |
| 24 | `FUN_0066D270(&Rules->Difficult, "Difficult")` | |
| 25 | `RulesClass::ReadCrateRules` | `[CrateRules]` |
| 26 | `RulesClass::ReadCombatDamage` | `[CombatDamage]` |
| 27 | `RulesClass::ReadRadiation` | `[Radiation]` |
| 28 | `RulesClass::ReadElevationModel` (FUN_0066D150) | `[ElevationModel]` |
| 29 | `RulesClass::ReadWallModel` (FUN_0066D1F0) | `[WallModel]` |
| 30 | `RulesClass::ReadAudioVisual` | `[AudioVisual]` |
| 31 | `RulesClass::ReadSpecialWeapons` | `[SpecialWeapons]` |
| 32 | `TiberiumClass::ReadINI_All` | ore classes |
| 33 | `FUN_00674650(unused, is_multiplayer)` | **Misidentified in earlier drafts.** This is *not* the AI-team / TaskForces loader — it is the advanced-command-bar button-list reader. Reads `[MultiplayerAdvancedCommandBar]` if `g_GameMode ∉ {0, 5}`, else `[AdvancedCommandBar]`. Populates the global button array at `DAT_00B0CB78` (25 slots). **No stock YR INI ships the section**, so the helper returns 0 on retail and writes nothing — effectively a no-op unless a mod adds the section. Details in [RULESCLASS_HELPER_FUN_00674650.md](RULESCLASS_HELPER_FUN_00674650.md). AI-team / ScriptType / TaskForce loading happens elsewhere (part of the `Type_Read_INI_All` pass at step 21, which dispatches each type's own `ReadINI` via vtable). |

**Critical ordering detail:** TypeClass *allocation* loops (steps 5–13) run BEFORE any Rules* method. This means by the time `ReadGeneral` tries to resolve `Pilot=E1`, the `E1` InfantryType exists but its fields are still at constructor defaults. The actual per-type field reads happen in step 21 (`FUN_00679A10`), AFTER most Rules sections. So RulesClass's type* pointers resolve correctly, but the typed data behind them is not yet populated at the time ReadGeneral runs. Any hand-rolled code that accesses a *type* pointer too early will see defaults.

**Map rules override (caller attribution corrected 2026-07-24):**
`ScenarioClass::Full_Init @ 0x00686B20` invokes the outer reset/main-rules path
and, after the full main read, passes the active map's `CCINIClass` to the same
inner reader `FUN_00668BF0`. The second call belongs to `Full_Init`, not to the
outer function body itself. Individual maps can therefore override every one
of the sections above via `[General]`/`[CombatDamage]`/etc. in the map INI, not
just `[Rules]`.

---

## 6. Integration Points

**When populated:** `ScenarioClass::Full_Init` at `0x00686B20`, once per game session, before the tick loop starts. There is no per-tick re-read.

**Who reads it:** `g_RulesClass_Instance` (0x008871E0) is dereferenced by a long list of subsystems at runtime. Verified callers include:
- `LogicClass::PerTickUpdate` — sim tick loop (likely `DamageDelay`, lightning/storms)
- `LightningStorm::*` — lightning-storm SW (all fields in `[General]`)
- `AnimClass::Constructor`, `AnimClass::Middle` — damage-fire types, meta anim selection
- `CellClass::RecalcAttributes`, `CellClass::BlowUpBridge`, `CellClass::IsWallConnectableInDirection` — terrain/bridge rules
- `Apply_area_damage`, `Warhead::SelectExplosionAnim` — combat damage resolution
- `MapClass::RevealAroundCell`, `MapClass::ParanoidRevealAll`, `MapClass::ParanoidUnrevealAll` — sight/paranoid vision
- `BuildingClass::GetCurrentFrame` — damage-state → anim frame decision
- `HouseClass::Recalculate_Alliances`, `HouseClass::MakeAlly`, `HouseClass::BreakAlliance`, `HouseClass::ComputerTakeover` — diplomacy, AI-takeover rules
- `CrateSlot::*` (Place/Validate/Remove) — crate-regen cadence
- `Wave_splash_forces` — water impact
- `ObjectClass::Reveal` — sight range bonuses
- `BSurface::Constructor` — display setup

This is consistent with RulesClass being the root source-of-truth for tuning data across the engine.

---

## 7. TS-legacy vs YR-active (CRITICAL)

Many RulesClass fields exist in the binary but are not exercised in standard YR content. Where the default is off, implementing them as if they were live is a common mistake.

| Field (section) | YR default | Notes |
|---|---|---|
| `FogOfWar` (`[General]` and `[MultiplayerDialogSettings]`) | `no` | TS legacy. Both copies default off. A separate SpecialFlags bit must also be set. YR skirmishes do NOT use fog (shroud only). |
| `IonStorms` (`[General]`) | `no` | TS legacy. The lightning-storm code runs via SW invocation in YR, not ambient ion storms. |
| `Meteorites` (`[General]`) | `no` | TS legacy. Dormant. |
| `Visceroids` (`[General]`, `LargeVisceroid`, `SmallVisceroid`) | `no` + names default to visceroid units that don't exist in YR skirmish | TS legacy. |
| `TiberiumExplosive` (`[CombatDamage]` offset 0x17E5) | `no` | Misleading name. Controls barrel-on-ore explosions; TS-era behavior. |
| `TiberiumHeal` (`[General]`) | 0.010 | Unit self-heal on tiberium — TS legacy. In YR ore is non-interactive for infantry/units. |
| `CrewEscape`, `Pilot`, `AlliedCrew`, `SovietCrew`, `ThirdCrew` (`[General]`) | used | YR *does* use these for unit-destruction infantry spawn. Active. |
| `CurleyShuffle` (`[General]`) | `yes` | Infantry path-wobble. Active. |
| `MutateExplosion`, `MutateWarhead`, `MutateExplosionWarhead` | active | YR-specific — drives Genetic Mutator SW. |
| `MultiplayerAICM` (`[General]`) | active | YR MP AI money multipliers. |
| `NodAIBuildsWalls` / `AIBuildsWalls` | no | Defaults both off — AI does not build walls in standard YR content. |
| `BerzerkAllowed` (`[CombatDamage]`) | `no` | TS legacy — cybernetic infantry berserk after C4. Inert in YR skirmish. |
| `SeparateAircraft` | `yes` | Air-unit handling mode — active in YR. |
| `ParadropRadius`, `*ParaDropInf/Num` | active | Paratrooper SW — YR. |
| `ChronoHarvTooFarDistance`, `ChronoDelay`, `ChronoMinimumDelay`, `ChronoRangeMinimum`, `ChronoDistanceFactor`, `ChronoTrigger` | active | Chronosphere SW + chrono miner. |
| `PurifierBonus` | 0.25 | Active — Allied Industrial Plant / Yuri bonus. |
| `HunterSeeker*` | TS legacy, but YR still ships with parameters | Hunter-Seeker SW exists in YR art but not wired to any standard superweapon in stock content — flag as optional. |
| `DominatorDamage`, `DominatorCaptureRange`, `DominatorFireAtPercentage`, `DominatorWarhead`, `DominatorFirstAnim`, `DominatorSecondAnim` | active | Psychic Dominator SW (YR). |
| `ForceShield*` | active | Iron Curtain-adjacent SW. |
| `PrismSupport*` | active | Prism Tower chain. |
| `V3Rocket*`, `DMisl*`, `CMisl*` (`[General]`) | active | V3 / Dreadnought / Boomer missile pathing globals. |

**Rule when implementing:** default-false booleans and type-class refs whose INI value is a TS-era unit/building name (e.g. `LargeVisceroid=VISC_LRG`) can be skipped unless a map/mod turns them on. Default-true and default-nonzero fields with standard YR content references are almost always live.

---

## 8. Current Rust Implementation Status

Detailed status from the Rust scan (see Agent report earlier). Table refreshed
2026-06-10 for the ScenarioClass/RulesClass substrate slices (SC-1/RC-1/SC-2)
and the earlier radiation slice — three rows that read "Missing" at first scan
now have implementations.

| Section | Status | Struct / file |
|---|---|---|
| `[General]` | **Partial (~80 of ~260 keys)** | [src/rules/ruleset.rs](../src/rules/ruleset.rs) `GeneralRules` (lines 114–359) |
| `[AudioVisual]` | **Stub** (only ConditionYellow/ConditionRed) | [src/rules/ruleset.rs](../src/rules/ruleset.rs) |
| `[CombatDamage]` | **Partial** (Iron Curtain, occupy/bunker/opentopped, bridge strength, mutate warheads) | [src/rules/ruleset.rs](../src/rules/ruleset.rs) `GarrisonRules`, `BridgeRules` |
| `[SpecialWeapons]` | **Missing** | — |
| `[CrateRules]` | **Missing** | — |
| `[Radiation]` | **Implemented** (RadLevel/Light/Tint/Color/Site warhead) | [src/rules/ruleset.rs](../src/rules/ruleset.rs) `RadiationRules` |
| `[ElevationModel]` | **Missing** | — |
| `[WallModel]` | **Missing** | — |
| `[JumpjetControls]` | **Missing** | — |
| `[MultiplayerDialogSettings]` | **Implemented** (per-match options + dialog trackbar bounds) | [src/sim/game_options.rs](../src/sim/game_options.rs) `GameOptions`; `SkirmishTrackbarBounds` in [src/ui/skirmish_shell/mod.rs](../src/ui/skirmish_shell/mod.rs) |
| `[Maximums]` | **Missing** | — |
| `[AI]` | **Missing** | — |
| `[AIGenerals]` | **Missing** (empty in stock anyway) | — |
| `[IQ]` | **Missing** | — |
| `[Easy]`/`[Normal]`/`[Difficult]` | **Missing** (no DifficultyClass equivalent) | — |
| Speed×LandType table | **Partial** via TerrainRules | [src/rules/terrain_rules.rs](../src/rules/terrain_rules.rs) |
| `[Colors]` (named color schemes) | **Implemented** (parsed into schemes → house colour ramps) | [src/rules/color_scheme.rs](../src/rules/color_scheme.rs) `parse_color_schemes`; `house_color_ramps` on `RuleSet` |
| Color Add 16-entry remap (`[ColorAdd]`) | **Missing** | — |

Runtime consumers (verified): ore growth, production speed, chrono SW, miner purifier, garrison combat, power damage-delay, pathfinding retry cadence, aircraft reload/flight-level, survivor divisor on building sell. Full map is in the parallel Rust-scan report.

---

## 9. Open Questions

Most entries in this list have been resolved during the full-decode plan
tasks (see `docs/plans/2026-04-24-rulesclass-full-decode-plan.md`).
Resolved items have been promoted into the appropriate structural
sections above. Remaining open items below.

1. **ColorAdd per-slot runtime consumers.** The reader, offset, and slot
   layout are now fully mapped ([RULESCLASS_COLORADD_TABLE.md](RULESCLASS_COLORADD_TABLE.md)),
   but *which* engine subsystem indexes each of the 14 slots is not yet
   traced. Finding them requires a scan for the two-step access pattern
   `MOV reg, [0x008871E0]; ... + 0x1874 + i*3` across the binary.
   Candidate consumers (not verified): Iron Curtain flash, Psychic
   Dominator tint, Chrono warp blend, health-bar colour mix.
2. **Exact `FUN_00679A10` per-type-class dispatch order** (step 21 of the
   inner dispatcher). Known to iterate every type-class array and call
   each entry's vtable+0x64 (`ReadINI`), plus run
   `MissionClass::Read_INI` for mission scripts — but the sub-sequence
   inside the helper was not decomposed this pass. This is the actual
   loader for `[AITriggerTypes]`, `[ScriptTypes]`, `[TeamTypes]`,
   `[TaskForces]`, and per-type data (weapon stats, warhead verses,
   etc.). Out of scope for RulesClass; queue a separate investigation
   for `FUN_00679A10`.
3. **Map-rules override semantics.** ✅ RESOLVED 2026-06-10 (substrate RC-1
   sub-question). The second rules pass over the map INI runs inside
   `ScenarioClass::Full_Init` (`decompile_function 0x00686b20`), which calls
   the master section processor `0x00668BF0` with the map's `CCINIClass`
   (built by `Read_Scenario_INI`, `decompile_function 0x00686730`) — the
   **same** processor used for rules+rulesmd, so all its registry semantics
   apply to the map.
   - **TypeClass allocation CAN happen from a map.** The type-list loops use
     find-OR-allocate, not find-only: `UnitTypeClass::FindOrAllocate`
     (`decompile_function 0x007480d0`, called from `ReadVehicleTypes`
     `0x00672360`) does `operator_new(0xe78)` + constructor on a name miss;
     the sibling factories (Infantry/Aircraft/Building/Warhead/SuperWeapon/
     Smudge/Terrain/Anim) follow the same Westwood pattern. The only
     non-allocating names are the `<none>`/`<noname>` sentinels.
   - **`[Colors]` also allocates per new key**: `Init_Color_Schemes_INI`
     (`decompile_function 0x0066d3a0`) → `0x00626ab0` does `operator_new(0x100)`
     and appends a scheme per key. (`[ColorAdd]`, by contrast, writes a fixed
     16-slot array — override-into-slots, not allocation.)
   - **Empty-value entries are "key absent."** `INIClass::Put_String`
     (`0x00528660`, the sole entry-creation site — `get_function_callers` on
     its node ctor `0x0052af00` returns only this caller) gates out NULL/empty
     values (`disassemble_function 0x00528660`, value check at
     `0x005288cc–0x005288e2`): an empty `Key=` stores **no entry** (and removes
     any prior one), so the readers (`ReadDouble 0x005283d0`, `ReadInt`,
     `ReadString`) fall through to the live/default value. A previously-merged
     value therefore **survives** an empty map entry — it is never reset to 0.
   - **Rust port status (corrected 2026-07-24):** the current
     value-override-only `merge_rules_overrides` keeps map-side
     TypeClass/`[Colors]` allocation OFF. This DRIFT is stock-live:
     `XEB2.MAP` from `multimd.mix` (and loose `EB2.mmx`'s `EB2.MAP`) defines
     map-only warhead section `[SpazWH]` with `Tiberium=yes`, and a map weapon
     names `Warhead=SpazWH`; `SpazWH` is absent from merged retail
     `rules.ini`/`rulesmd.ini`. Therefore the bounded Rust merge drops the new
     section instead of allocating and reading the warhead as native does.
     This is not an overlay-type classification override. Retail evidence:
     `MISSIONSMD.PKT` row `70=XEB2`; `XEB2.MAP` `[SpazWH]` lines 90–99,
     `Tiberium=yes` line 98; `EB2.MAP` `[SpazWH]` lines 86–95,
     `Tiberium=yes` line 94. The empty-value drift is fixed:
     `merge_rules_overrides` skips empty-valued keys so the merged value
     survives.
4. **TS-legacy flagging in the CSV.** Task 2/3/5 extraction tagged
   `yr_active={yes,no,conditional}` per row, but a handful of
   `SpecialFlags`-gated keys remain as `conditional`; resolving each
   against the actual gate default would upgrade them to `no` or `yes`.

### Resolved during the full-decode plan

- ✅ *Full `[General]` offset map (~260 keys)* — extracted via
  `RulesReadINI_Extractor` (Task 2); see [RULESCLASS_FIELDS.csv](RULESCLASS_FIELDS.csv).
- ✅ *Full `[AudioVisual]` offset map (~100+ keys)* — extracted via
  the same script (Task 3); see [RULESCLASS_FIELDS.csv](RULESCLASS_FIELDS.csv).
- ✅ *Remaining small Read_* methods + `[AI]`* — Task 5, all in CSV.
- ✅ *`ReadCombatDamage` validation* — Task 4; ≥95% agreement between
  script and prior manual extraction.
- ✅ *Constructor defaults for every field* — Task 6; see
  [RULESCLASS_CONSTRUCTOR_DEFAULTS.md](RULESCLASS_CONSTRUCTOR_DEFAULTS.md)
  and [RULESCLASS_CONSTRUCTOR_DEFAULTS.csv](RULESCLASS_CONSTRUCTOR_DEFAULTS.csv).
- ✅ *Ctor ↔ INI cross-reference, orphan analysis* — Task 7; see
  [RULESCLASS_DEFAULTS_CROSSREF.md](RULESCLASS_DEFAULTS_CROSSREF.md)
  (723 matched, 5 INI-only, 240 runtime-only).
- ✅ *Three embedded DifficultyClass slot offsets* — Task 8; Easy=`0x1538`,
  Normal=`0x1588`, Difficult=`0x15D8`, each `0x50 B`; see
  [RULESCLASS_DIFFICULTY_SLOTS.md](RULESCLASS_DIFFICULTY_SLOTS.md). §4
  of this report updated to remove the old "not fully nailed down"
  caveat.
- ✅ *Vtable presence* — Task 9. **None.** First ctor instruction writes
  the immediate literal `0x0000000F` to `this+0`, not a vtable pointer;
  no `vtable__Rules*` symbol exists. See §1 for the raw-bytes evidence.
- ✅ *`FUN_0066D3A0` purpose* — Task 10. Reader for the `[Colors]`
  palette-remap section (per-house colour-scheme names). Populates
  global palette table at `0x00886380`/`0x00885780`, not a RulesClass
  field. §5 step 1 updated.
- ✅ *`FUN_00674650` purpose* — Task 11. **Not** AI-teams. Reads
  `[AdvancedCommandBar]` or `[MultiplayerAdvancedCommandBar]` and
  populates the global UI button array `DAT_00B0CB78`. Dormant in
  stock YR (neither section ships in any `ini/*.ini`). §5 step 33
  updated; full write-up in
  [RULESCLASS_HELPER_FUN_00674650.md](RULESCLASS_HELPER_FUN_00674650.md).
- ✅ *`[Powerups]` schema* — Task 12. 19 fixed slots, 4 parallel
  globals, field-by-field format documented in
  [RULESCLASS_POWERUPS_TABLE.md](RULESCLASS_POWERUPS_TABLE.md).
- ✅ *ColorAdd reader + layout* — Task 13. Reader is `FUN_0066D480`
  (`RulesClass::ReadColorAdd`); table at `RulesClass + 0x1874`; 14
  populated slots in stock YR; write-up in
  [RULESCLASS_COLORADD_TABLE.md](RULESCLASS_COLORADD_TABLE.md).
  Runtime-consumer question remains open (see above).

---

## 10. Complete field map (CSV references)

The authoritative per-key extraction lives in
[RULESCLASS_FIELDS.csv](RULESCLASS_FIELDS.csv). Format:
`function_addr, section, key, offset, type, section_match, source_line`.

Approximate per-section row counts from the canonical CSV:

| Section | Reader | CSV rows |
|---|---|---:|
| `[General]` | `RulesClass::ReadGeneral @ 0x0066D530` | ~240 |
| `[AudioVisual]` | `RulesClass::ReadAudioVisual @ 0x006691E0` | ~100 |
| `[CombatDamage]` | `RulesClass::ReadCombatDamage @ 0x0066BBB0` | ~60 |
| `[AI]` | `RulesClass::ReadAI @ 0x00672AE0` | ~45 |
| `[IQ]` | `RulesClass::ReadIQ @ 0x00674240` | ~12 |
| `[CrateRules]` | `RulesClass::ReadCrateRules @ 0x0066B900` | ~20 |
| `[Radiation]` | `RulesClass::ReadRadiation @ 0x0066CF70` | ~10 |
| `[ElevationModel]` | `RulesClass::ReadElevationModel @ 0x0066D150` | 3 |
| `[WallModel]` | `RulesClass::ReadWallModel @ 0x0066D1F0` | 2 |
| `[JumpjetControls]` | `RulesClass::ReadJumpjetControls @ 0x006743D0` | 7 |
| `[MultiplayerDialogSettings]` | `RulesClass::ReadMultiplayerDialogSettings @ 0x00671EA0` | ~25 |
| `[SpecialWeapons]` | `RulesClass::ReadSpecialWeapons @ 0x00668FB0` | 7 |
| `[SpeedType × LandType]` | `RulesClass::ReadSpeedTypeLandTypeTable @ 0x00674000` | (table → global `0x0089EA44`, not RulesClass) |
| `[Colors]` (per-house scheme) | `FUN_0066D3A0` | (→ global palette, not RulesClass) |
| `[ColorAdd]` | `RulesClass::ReadColorAdd @ 0x0066D480` | 14 rows → `+0x1874` |
| `[Powerups]` | `FUN_00673E80` | 19 slots → four parallel globals |
| `[Difficulty.Easy/Normal/Difficult]` | `DifficultyClass::Read_INI @ 0x0066D270` | 3× `0x50 B` embedded slots at `0x1538`/`0x1588`/`0x15D8` |

**Total CSV rows:** 728 INI-reader offsets across all sections.

## 11. Constructor defaults reference

Every field set in `RulesClass::Constructor @ 0x00665650` is enumerated in
[RULESCLASS_CONSTRUCTOR_DEFAULTS.md](RULESCLASS_CONSTRUCTOR_DEFAULTS.md)
(1,085 store-offsets). The parallel CSV is
[RULESCLASS_CONSTRUCTOR_DEFAULTS.csv](RULESCLASS_CONSTRUCTOR_DEFAULTS.csv).
Cross-reference against the INI-reader CSV is in
[RULESCLASS_DEFAULTS_CROSSREF.md](RULESCLASS_DEFAULTS_CROSSREF.md):

- **723** fields have both a ctor default and an INI reader (happy case).
- **5** fields have an INI reader but no ctor default — flagged WARN
  (potentially uninitialised if INI is silent). See cross-ref §1.
- **240** fields have a ctor default but no INI reader — runtime-only
  caches, TS-legacy slots, or live fields populated by non-INI code.
  See cross-ref §2.

## 12. Helper / non-RulesClass companion docs

Dispatcher calls that produced data outside the `RulesClass*` instance:

| Call | Doc |
|---|---|
| `FUN_00673E80` (step 17, `[Powerups]`) | [RULESCLASS_POWERUPS_TABLE.md](RULESCLASS_POWERUPS_TABLE.md) |
| `FUN_0066D480` (step 2, `[ColorAdd]`) | [RULESCLASS_COLORADD_TABLE.md](RULESCLASS_COLORADD_TABLE.md) |
| `FUN_00674650` (step 33, `[AdvancedCommandBar]`) | [RULESCLASS_HELPER_FUN_00674650.md](RULESCLASS_HELPER_FUN_00674650.md) |
| `FUN_0066D270` (steps 22–24, Difficulty) | [RULESCLASS_DIFFICULTY_SLOTS.md](RULESCLASS_DIFFICULTY_SLOTS.md) + `AI_DIFFICULTY_SYSTEM.md` |

---

## Sources

### Ghidra addresses decompiled in full
- `RulesClass::ReadSpecialWeapons` — `0x00668FB0`
- `RulesClass::ReadCrateRules` — `0x0066B900`
- `RulesClass::ReadCombatDamage` — `0x0066BBB0`
- `RulesClass::ReadRadiation` — `0x0066CF70`
- `RulesClass::ReadElevationModel` — `0x0066D150` (was `FUN_0066D150`)
- `RulesClass::ReadWallModel` — `0x0066D1F0` (was `FUN_0066D1F0`)
- `RulesClass::ReadIQ` — `0x00674240`
- `RulesClass::ReadJumpjetControls` — `0x006743D0`
- `RulesClass::ReadMultiplayerDialogSettings` — `0x00671EA0`
- `RulesClass::ReadSpeedTypeLandTypeTable` — `0x00674000`
- `RulesClass::ReadAI` (FUN_00672AE0) — `0x00672AE0`
- `DifficultyClass::Read_INI` (FUN_0066D270) — `0x0066D270`
- `RulesClass::Process` (outer) — `0x006686C0`
- `RulesClass::Read_INI` (inner) — `0x00668BF0`
- `Init_Game` (ctor site) — `0x0052BAD8` (shows `operator_new(0x18C0)`)
- `Game_Shutdown` (dtor site) — `0x006BE1C0`

### Ghidra addresses located but not fully decompiled (too large)
- `RulesClass::ReadGeneral` — `0x0066D530` (body → `0x00671E98`)
- `RulesClass::ReadAudioVisual` — `0x006691E0` (body → `0x0066B8FF`)
- `RulesClass::Constructor` (defaults) — `0x00665650` (body → `0x00667A26`)
- `RulesClass::Destructor` — `0x00667A30`
- `Type_Read_INI_All` (FUN_00679A10) — iterates all type arrays

### Docs consulted
- `ra2-rust-game-docs/ADDRESS_MAP.md` — confirmed `g_RulesClass_Instance` at `0x008871E0`, `ReadGeneral` at `0x0066D530`
- `ra2-rust-game-docs/READINI_FIELD_MAPS.md` — per-type ReadINI patterns (WeaponType, WarheadType etc.) — structural reference, not RulesClass itself
- `ra2-rust-game-docs/CRATE_SYSTEM_GHIDRA_REPORT.md`, `AI_DIFFICULTY_SYSTEM.md`, `ANIMATION_SOUNDS_GHIDRA_REPORT.md`, `BOMB_CLASS_GHIDRA_REPORT.md`, `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` — partial coverage of individual section key sets
- `ra2-rust-game-docs/GAME_START_INITIALIZATION.md`, `SCENARIO_INIT_DEEP_DIVE.md` — for lifetime context

### INI files cross-checked
- `ini/rulesmd.ini` — all `[General]`, `[JumpjetControls]`, `[SpecialWeapons]`, `[AudioVisual]`, `[CrateRules]`, `[CombatDamage]`, `[Radiation]`, `[ElevationModel]`, `[WallModel]`, `[MultiplayerDialogSettings]`, `[Maximums]`, `[AI]`, `[AIGenerals]`, `[IQ]`, `[Easy]`, `[Normal]`, `[Difficult]` sections enumerated
- `ini/rules.ini` — base RA2 deltas noted (MutateExplosion added, third-side variants added, etc.)
