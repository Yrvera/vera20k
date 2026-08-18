# AnimClass & AnimTypeClass — Ghidra Research Report

Reverse-engineered from `gamemd.exe`. All offsets verified from binary.

## Key Addresses

| Entity | Address | Notes |
|--------|---------|-------|
| AnimClass::Constructor (full) | `0x00421EA0` | 7 params + this |
| AnimClass::Constructor (load) | `0x00422720` | No params (deserialization) |
| AnimClass::AI | `0x00423AC0` | Per-tick update, vtable[23] offset 0x5C (corrected 2026-05-28: was vtable[24]/0x60; binary vtable read_memory at 0x007E3354 shows AI at slot 23/0x5C — ROOT_CAUSE: OFFSET_RETYPED_WRONG) |
| AnimClass::DrawIt | `0x00422CA0` | Rendering, vtable[69] offset 0x114 |
| AnimClass::Destroy | `0x004255B0` | Self-removal, vtable offset 0xF8 |
| AnimClass::Start | `0x00424F00` | Sound/particle/scorch on start |
| AnimClass::Middle | `0x00424CE0` | Called when delay expires, begins play |
| AnimClass::SetOwnerObject | `0x00424B50` | Attach/detach from TechnoClass |
| AnimTypeClass::Constructor | `0x00427530` | |
| AnimTypeClass::ReadINI | `0x00427D00` | Parses art.ini |
| AnimTypeClass::FindOrAllocate | `0x00428B80` | Name lookup over `g_AnimTypes_Array`; allocates new type on miss (see §ref) (corrected 2026-05-28: was labeled FindByName; binary labels it AnimTypeClass__FindOrAllocate — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| AnimTypeClass::FindByIndex | `0x00427CB0` | |
| AnimTypeClass::FindOrCreate | `0x00428F70` | Generic variant taking a `DynamicVectorClass*` in param_2 |
| AnimClass vtable | `0x007E3354` | |
| AnimTypeClass vtable (primary) | `0x007E3608` | Set at ctor offset `param_1[0]` |
| AnimTypeClass vtable (secondary_4) | `0x007E35EC` | Set at ctor offset `param_1[1]` (IUnknown bridge) |
| AnimTypeClass vtable (secondary_8) | `0x007E35E4` | Set at ctor offset `param_1[2]` |
| AnimTypeClass vtable (secondary_12) | `0x007E35DC` | Set at ctor offset `param_1[3]` |
| ObjectTypeClass::ResolveImageForTheater | `0x005F9070` | Loads theater-specific SHP for any ObjectType; called from AnimTypeClass ReadINI flow |
| g_AnimClass_Array | `0x00A8E9AC` | DynamicVectorClass<AnimClass*> data ptr |
| g_AnimClass_Array_Count | `0x00A8E9B8` | |
| g_AnimTypes_Array | `0x008B4154` | AnimTypeClass* array |
| g_AnimTypeClass_Count | `0x008B4160` | |
| ObjectClass::UnInit | `0x005F65F0` | Adds to pending-delete list |
| CC_Draw_Shape | `0x004AED70` | Core SHP drawing function |
| Blitter_selector | `0x00490B90` | Selects blitter based on draw flags |

## AnimClass Struct Layout (size = 0x1C8 = 456 bytes)

AnimClass inherits from ObjectClass. The `this` pointer type in Ghidra is `int*`,
so all indices below are multiplied by 4 to get byte offsets.

### Inherited from ObjectClass (first ~0xA0 bytes)

| Byte Offset | Index | Field | Notes |
|-------------|-------|-------|-------|
| 0x000 | [0] | vtable ptr | points to 0x7E3354 |
| 0x004 | [1] | IUnknown vtable | |
| 0x008 | [2] | IRTTITypeInfo vtable | |
| 0x00C | [3] | INoticeSink vtable | |
| 0x090 | [0x24] | IsActive | bool, set to 1 in constructor |
| 0x09C | [0x27] | Location.X | world coords |
| 0x0A0 | [0x28] | Location.Y | |
| 0x0A4 | [0x29] | Location.Z | |

### AnimClass-specific fields

| Byte Offset | Index | Field | Notes |
|-------------|-------|-------|-------|
| 0x0AC | [0x2B] | CurrentFrame | Current animation frame index |
| 0x0B0 | [0x2C] | FrameAdvanced | bool, set to 1 when frame just ticked |
| 0x0B4 | [0x2D] | LastFrameTime | g_CurrentFrameCounter when last frame advanced |
| 0x0B8 | [0x2E] | (unused/saved) | |
| 0x0BC | [0x2F] | FrameDelay | Current delay countdown (ticks until next frame) |
| 0x0C0 | [0x30] | FrameDelayReload | Rate value (reloaded into 0x2F each frame) |
| 0x0C4 | [0x31] | FrameStep | +1 or -1 (direction of frame advance) |
| 0x0C8 | [0x32] | Type | AnimTypeClass* pointer |
| 0x0CC | [0x33] | OwnerObject | ObjectClass* this anim is attached to |
| 0x0D0 | [0x34] | (field) | -1 default |
| 0x0D4 | [0x35] | Palette | Remap palette override, 0 = default |
| 0x0D8 | [0x36] | (field) | -1 default |
| 0x0FC | [0x3F] | Strength | 1000 default (health) |
| 0x100 | [0x40] | ZAdjust | Drawing Z-adjustment |
| 0x104 | [0x41] | (field) | Copied from AnimType+0x340 |
| 0x108-0x110 | [0x42-0x44] | (saved coords) | 3 ints, from DAT_0089a178 |
| 0x114 | [0x45] | (field) | |
| 0x118 | [0x46] | (bool) | |
| 0x119 | +0x119 | HasShadowOverride | bool |
| 0x11A | +0x11A | Paused | bool |
| 0x11B | +0x11B | (bool) | Cleared when 0x47 == CurrentFrame |
| 0x11C | [0x47] | (field) | Some frame comparison value |
| 0x120 | [0x48] | Reverse | bool, from constructor param_8 |
| 0x128 | [0x4A] | (field) | |
| 0x130 | [0x4C] | (field) | |
| 0x138 | [0x4E] | (field) | |
| 0x178 | [0x5E] | TranslucencyStage | char, used for progressive translucency |
| 0x179 | +0x179 | IsMarkedForDeletion | set to 1 before Destroy() call |
| 0x17C | [0x5F] | (field) | |
| 0x180 | [0x60] | OwnerHouse | HouseClass* |
| 0x184 | [0x61] | Delay | Countdown before anim starts playing |
| 0x188-0x18B | [0x62-0x63] | AccumulatedDamage | double (damage accumulator) |
| 0x18C | [99] | (double high) | Part of AccumulatedDamage double |
| 0x190 | [100] | DrawFlags | Passed to CC_Draw_Shape as flags |
| 0x194 | [0x65] | IsBouncer | bool |
| 0x195 | +0x195 | LoopCountRemaining | byte, loops left |
| 0x196 | +0x196 | UseCellDrawer | bool; terrain-tile producer sets after construction |
| 0x197 | +0x197 | TerrainAttached | bool; `MapClass::InitCellAttributes` destroys/recreates marked instances |
| 0x198 | [0x66] | (bool) | Related to spawn behavior |
| 0x199 | +0x199 | (bool) | |
| 0x19A | +0x19A | (bool) | |
| 0x19B | +0x19B | IsInactive | bool, suppresses drawing and AI |
| 0x19C | [0x67] | (bool) | Set to 1 in constructor |
| 0x19D | +0x19D | IsInvisible | bool, suppresses drawing |
| 0x19E | +0x19E | (bool) | |
| 0x1A0-0x1B3 | [0x68-0x6C] | (fields) | Sound/particle state |
| 0x1B4 | [0x6D] | SparkleCoords | 3 ints (X,Y,Z) for sparkle/expire anim |

## AnimTypeClass Struct Layout (size = 0x378 bytes)

Inherits from ObjectTypeClass. Since `param_1` in ReadINI is `int*`, all
`param_1[N]` indices mean byte offset `N * 4`.

Total AnimTypeClass-specific fields: 57 (all verified from ReadINI at 0x427D00
and constructor at 0x427530). Every field from 0x294 to 0x374 is accounted for.

### Inherited ObjectTypeClass Fields (referenced by AnimTypeClass)

These are inherited from ObjectTypeClass but overridden or used in AnimTypeClass.

| Byte Offset | Index | Field | Default (AnimType) | Notes |
|-------------|-------|-------|---------------------|-------|
| 0x024 | [0x09] | ID (Name string) | from INI section | 25-byte string, used as INI section key |
| 0x0A4 | [0x29] | SHPFileData | 0 | SHP image pointer, checked before Theater/NewTheater |
| 0x1F8 | [0x7E] | HasImage | (inherited) | bool, gates Theater/NewTheater reading |
| 0x22C | [0x8B] byte | Theater | 0 | bool, read from INI "Theater" |
| 0x22F | +0x22F | (Crushable) | 1 (overridden) | ObjectTypeClass default=0, AnimType sets 1 |
| 0x230 | +0x230 | (Selectable) | 0 (overridden) | ObjectTypeClass default=1 in ctor |
| 0x231 | +0x231 | (LegalTarget) | 0 (overridden) | ObjectTypeClass default=1 in ctor |
| 0x232 | +0x232 | (Insignificant) | 1 (overridden) | ObjectTypeClass default=0 |
| 0x233 | +0x233 | (Immune) | 1 (overridden) | ObjectTypeClass default=0 |
| 0x234 | [0x8D] byte | (LogicVisible) | 1 (overridden) | ObjectTypeClass default=0 |
| 0x235 | +0x235 | (AllowShroudedDraw) | 0 (overridden) | ObjectTypeClass default=1 |
| 0x237 | +0x237 | NewTheater | 0 | bool, read from INI "NewTheater" |

### AnimTypeClass-Specific Fields (Complete)

All 57 fields from ReadINI (0x427D00) + constructor (0x427530), verified from
disassembly. Fields are listed in byte offset order.

#### Internal / Non-INI Fields

| Byte Offset | Index | Field | Default | Notes |
|-------------|-------|-------|---------|-------|
| 0x294 | [0xA5] | ArrayIndex | -1 | Position in g_AnimTypes_Array |
| 0x298 | [0xA6] | SHPData | 0 | Pointer to loaded SHP image data |
| 0x29C | [0xA7] | SHPWidth | 0 | Cached from SHP header |
| 0x2A0 | [0xA8] | SHPHeight | 0 | Cached from SHP header |
| 0x2A4 | [0xA9] | (byte) | 0 | byte-sized field, not read from INI |
| 0x320-0x327 | [0xC8-0xC9] | MaxZVel | 3.5 | double, bounce physics internal (not from INI) |
| 0x35E | +0x35E | (internal) | 0 | bool, not read from INI |
| 0x35F | +0x35F | (internal) | 0 | bool, not read from INI |
| 0x363 | +0x363 | (padding) | — | Padding byte between Normalized and Layer |

#### INI-Configurable Fields (read in AnimTypeClass::ReadINI)

Fields listed in the order they appear in the ReadINI function.

| Byte Offset | Index | INI Key | Type | Default | Lookup Function |
|-------------|-------|---------|------|---------|-----------------|
| 0x372 | +0x372 | Shadow | bool | false | — |
| 0x364 | [0xD9] | Layer | enum | 3 (Ground) | CCINIClass__ReadLayer (0x477050) |
| 0x361 | +0x361 | AltPalette | bool | false | — |
| 0x368 | +0x368 | DoubleThick | bool | false | — |
| 0x369 | +0x369 | Flat | bool | false | — |
| 0x36C | +0x36C | Flamer | bool | false | — |
| 0x362 | +0x362 | Normalized | bool | false | — |
| 0x36A | +0x36A | Translucent | bool | false | — |
| 0x36B | +0x36B | Scorch | bool | false | — |
| 0x36D | +0x36D | Crater | bool | false | — |
| 0x36E | +0x36E | ForceBigCraters | bool | false | — |
| 0x36F | +0x36F | Sticky | bool | false | — |
| 0x370 | +0x370 | PingPong | bool | false | — |
| 0x371 | +0x371 | Reverse | bool | false | — |
| 0x373 | +0x373 | PsiWarning | bool | false | — |
| 0x357 | +0x357 | TiberiumChainReaction | bool | false | — |
| 0x2B0 | [0xAC] | Rate | int | 1 | `internal = 900 / INI_Rate` (0 if Rate<=0) |
| 0x2A8-0x2AF | [0xAA-0xAB] | Damage | double | 0.0 | CCINIClass__ReadDouble (0x5283D0) |
| 0x2B4 | [0xAD] | Start | int | 0 | — |
| 0x2C0 | [0xB0] | End | int | 0 | Auto-detected from SHP if 0; halved if Shadow=yes |
| 0x2B8 | [0xAE] | LoopStart | int | 0 | — |
| 0x2BC | [0xAF] | LoopEnd | int | 0 | Clamped to End if > End |
| 0x2C4 | [0xB1] | LoopCount | int | 0 | — |
| 0x2C8 | [0xB2] | Next | AnimType* | NULL | AnimTypeClass__FindOrCreate (0x428F70) |
| 0x2D4 | [0xB5] | DetailLevel | int | 0 | — |
| 0x2D8 | [0xB6] | TranslucencyDetailLevel | int | 0 | — |
| 0x2DC-0x2E3 | [0xB7-0xB8] | RandomLoopDelay | int[2] | {0, 0} | CCINIClass__ReadMinMax (0x529880) |
| 0x2EC | [0xBB] | Translucency | int | 0 | Value 25, 50, or 75 for translucency level |
| 0x358 | +0x358 | IsTiberium | bool | false | — |
| 0x359 | +0x359 | HideIfNoOre | bool | false | — |
| 0x340 | [0xD0] | YSortAdjust | int | 0 | — |
| 0x310-0x317 | [0xC4-0xC5] | Elasticity | double | 0.8 | CCINIClass__ReadDouble |
| 0x328-0x32F | [0xCA-0xCB] | MaxXYVel | double | 15.0 | CCINIClass__ReadDouble |
| 0x318-0x31F | [0xC6-0xC7] | MinZVel | double | 3.5 | CCINIClass__ReadDouble |
| 0x34C | [0xD3] | MakeInfantry | int | -1 | Infantry type index |
| 0x2F0 | [0xBC] | Spawns | AnimType* | NULL | AnimTypeClass__FindOrCreate (0x428F70) |
| 0x2F4 | [0xBD] | SpawnCount | int | 0 | — |
| 0x356 | +0x356 | IsMeteor | bool | false | — |
| 0x355 | +0x355 | IsVeins | bool | false | — |
| 0x33C | [0xCF] | TiberiumSpreadRadius | int | 0 | — |
| 0x338 | [0xCE] | TiberiumSpawnType | OverlayType* | NULL | OverlayTypeClass__FindOrCreate (0x5FEC70) |
| 0x360 | +0x360 | IsAnimatedTiberium | bool | false | — |
| 0x374 | +0x374 | ShouldFogRemove | bool | true (1) | — |
| 0x354 | +0x354 | IsFlamingGuy | bool | false | — |
| 0x350 | [0xD4] | RunningFrames | int | 0 | — |
| 0x344 | [0xD1] | YDrawOffset | int | 0 | — |
| 0x348 | [0xD2] | ZAdjust | int | 0 | — |
| 0x2F8 | [0xBE] | StartSound / Report | int | -1 | VocClass__FindByName (0x7514D0); tries StartSound first, then Report as fallback |
| 0x2FC | [0xBF] | StopSound | int | -1 | VocClass__FindByName (0x7514D0) |
| 0x300 | [0xC0] | BounceAnim | AnimType* | NULL | AnimTypeClass__FindOrCreate; inline in ReadINI |
| 0x304 | [0xC1] | ExpireAnim | AnimType* | NULL | AnimTypeClass__FindOrCreate; inline in ReadINI |
| 0x308 | [0xC2] | TrailerAnim | AnimType* | NULL | AnimTypeClass__FindOrCreate; inline in ReadINI |
| 0x30C | [0xC3] | TrailerSeperation | int | 0 | Note: original typo "Seperation" preserved |
| 0x334 | [0xCD] | DamageRadius | int | 0 | — |
| 0x330 | [0xCC] | Warhead | WarheadType* | NULL | WarheadTypeClass__FindOrCreate (0x75E3B0) |
| 0x35A | +0x35A | Bouncer | bool | false | — |
| 0x35B | +0x35B | Tiled | bool | false | — |
| 0x35C | +0x35C | ShouldUseCellDrawer | bool | true (1) | — |
| 0x35D | +0x35D | UseNormalLight | bool | false | — |
| 0x2CC | [0xB3] | SpawnsParticle | int | -1 | ParticleTypeClass__FindOrCreate (0x645430) (corrected 2026-05-28: was ParticleSystemTypeClass; binary labels it ParticleTypeClass__FindOrCreate — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x2D0 | [0xB4] | NumParticles | int | 0 | — |
| 0x2E4-0x2EB | [0xB9-0xBA] | RandomRate | int[2] | {0, 0} | CCINIClass__ReadMinMax; converted via 900/x; clamped >=0; min<=max. ReadINI uses local -1,-1 as no-value sentinel to skip conversion, but stored constructor defaults are {0,0} (corrected 2026-05-28: was {-1,-1}; constructor at 0x427530 sets both to 0 — ROOT_CAUSE: INFERENCE_HARDENED) |

### Field Summary by Byte Offset (sorted)

| Byte Offset | Size | INI Key | Type | Default |
|-------------|------|---------|------|---------|
| 0x294 | 4 | — | int | -1 (ArrayIndex) |
| 0x298 | 4 | — | ptr | 0 (SHPData) |
| 0x29C | 4 | — | int | 0 (SHPWidth) |
| 0x2A0 | 4 | — | int | 0 (SHPHeight) |
| 0x2A4 | 1 | — | byte | 0 (internal) |
| 0x2A8 | 8 | Damage | double | 0.0 |
| 0x2B0 | 4 | Rate | int | 1 |
| 0x2B4 | 4 | Start | int | 0 |
| 0x2B8 | 4 | LoopStart | int | 0 |
| 0x2BC | 4 | LoopEnd | int | 0 |
| 0x2C0 | 4 | End | int | 0 |
| 0x2C4 | 4 | LoopCount | int | 0 |
| 0x2C8 | 4 | Next | AnimType* | NULL |
| 0x2CC | 4 | SpawnsParticle | int | -1 |
| 0x2D0 | 4 | NumParticles | int | 0 |
| 0x2D4 | 4 | DetailLevel | int | 0 |
| 0x2D8 | 4 | TranslucencyDetailLevel | int | 0 |
| 0x2DC | 4 | RandomLoopDelay.Min | int | 0 |
| 0x2E0 | 4 | RandomLoopDelay.Max | int | 0 |
| 0x2E4 | 4 | RandomRate.Min | int | 0 (corrected 2026-05-28: was listed as -1; constructor at 0x427530 sets param_1[0xb9]=0 — ROOT_CAUSE: INFERENCE_HARDENED) |
| 0x2E8 | 4 | RandomRate.Max | int | 0 (corrected 2026-05-28: was listed as -1; constructor at 0x427530 sets param_1[0xba]=0 — ROOT_CAUSE: INFERENCE_HARDENED) |
| 0x2EC | 4 | Translucency | int | 0 |
| 0x2F0 | 4 | Spawns | AnimType* | NULL |
| 0x2F4 | 4 | SpawnCount | int | 0 |
| 0x2F8 | 4 | StartSound/Report | int | -1 |
| 0x2FC | 4 | StopSound | int | -1 |
| 0x300 | 4 | BounceAnim | AnimType* | NULL |
| 0x304 | 4 | ExpireAnim | AnimType* | NULL |
| 0x308 | 4 | TrailerAnim | AnimType* | NULL |
| 0x30C | 4 | TrailerSeperation | int | 0 |
| 0x310 | 8 | Elasticity | double | 0.8 |
| 0x318 | 8 | MinZVel | double | 3.5 |
| 0x320 | 8 | MaxZVel | double | 3.5 (internal, not from INI) |
| 0x328 | 8 | MaxXYVel | double | 15.0 |
| 0x330 | 4 | Warhead | WarheadType* | NULL |
| 0x334 | 4 | DamageRadius | int | 0 |
| 0x338 | 4 | TiberiumSpawnType | OverlayType* | NULL |
| 0x33C | 4 | TiberiumSpreadRadius | int | 0 |
| 0x340 | 4 | YSortAdjust | int | 0 |
| 0x344 | 4 | YDrawOffset | int | 0 |
| 0x348 | 4 | ZAdjust | int | 0 |
| 0x34C | 4 | MakeInfantry | int | -1 |
| 0x350 | 4 | RunningFrames | int | 0 |
| 0x354 | 1 | IsFlamingGuy | bool | false |
| 0x355 | 1 | IsVeins | bool | false |
| 0x356 | 1 | IsMeteor | bool | false |
| 0x357 | 1 | TiberiumChainReaction | bool | false |
| 0x358 | 1 | IsTiberium | bool | false |
| 0x359 | 1 | HideIfNoOre | bool | false |
| 0x35A | 1 | Bouncer | bool | false |
| 0x35B | 1 | Tiled | bool | false |
| 0x35C | 1 | ShouldUseCellDrawer | bool | true |
| 0x35D | 1 | UseNormalLight | bool | false |
| 0x35E | 1 | — | bool | false (internal) |
| 0x35F | 1 | — | bool | false (internal) |
| 0x360 | 1 | IsAnimatedTiberium | bool | false |
| 0x361 | 1 | AltPalette | bool | false |
| 0x362 | 1 | Normalized | bool | false |
| 0x363 | 1 | — | — | padding |
| 0x364 | 4 | Layer | enum | 3 (Ground) |
| 0x368 | 1 | DoubleThick | bool | false |
| 0x369 | 1 | Flat | bool | false |
| 0x36A | 1 | Translucent | bool | false |
| 0x36B | 1 | Scorch | bool | false |
| 0x36C | 1 | Flamer | bool | false |
| 0x36D | 1 | Crater | bool | false |
| 0x36E | 1 | ForceBigCraters | bool | false |
| 0x36F | 1 | Sticky | bool | false |
| 0x370 | 1 | PingPong | bool | false |
| 0x371 | 1 | Reverse | bool | false |
| 0x372 | 1 | Shadow | bool | false |
| 0x373 | 1 | PsiWarning | bool | false |
| 0x374 | 1 | ShouldFogRemove | bool | true |

### Fields NOT in AnimTypeClass (confirmed absent from gamemd.exe)

These INI keys do NOT exist as strings in gamemd.exe and are NOT read by
AnimTypeClass::ReadINI:

- **DoNotSimplify** — not a real INI key (no string found in binary)
- **EndSound** — not a real INI key (no string found; only StartSound/Report/StopSound exist)
- **Report** — exists but is NOT a separate field; it is a fallback for StartSound (same offset 0x2F8)

### StartSound / Report Fallback Logic

The StartSound field at 0x2F8 uses a two-pass read:
1. First tries to read "StartSound" from INI and look up via VocClass__FindByName
2. If StartSound is not set or not found (result == -1), tries "Report" as fallback
3. Both map to the same field at offset 0x2F8

### Shadow Side Effect on End

When Shadow is toggled in ReadINI:
- If Shadow changes from false to true: `End = End / 2` (halve frame count)
- If Shadow changes from true to false: `End = End * 2` (double frame count)
- After adjustment, LoopEnd is clamped: `if LoopEnd > End then LoopEnd = End`

### Rate Conversion

```
internal_rate = 900 / ini_rate    (if ini_rate > 0)
internal_rate = 0                 (if ini_rate <= 0)
```

Same formula applies to RandomRate min/max values. ReadINI uses local sentinel {-1, -1}
to detect "not present in INI" — values of -1 are skipped (no conversion applied).
The stored field defaults (from constructor) are {0, 0}, not {-1, -1}.
After conversion: `RandomRate.Max = max(RandomRate.Max, 0)` and
`RandomRate.Min = min(RandomRate.Min, RandomRate.Max)`.

### Lookup Functions Used in ReadINI

| Address | Name | Returns | Used For |
|---------|------|---------|----------|
| 0x428F70 | AnimTypeClass__FindOrCreate | AnimTypeClass* | Next, Spawns, BounceAnim, ExpireAnim, TrailerAnim |
| 0x7514D0 | VocClass__FindByName | int (index, -1=none) | StartSound, Report, StopSound |
| 0x75E3B0 | WarheadTypeClass__FindOrAllocate | WarheadTypeClass* | Warhead (corrected 2026-05-28: was FindOrCreate; binary labels it FindOrAllocate — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x5FEC70 | OverlayTypeClass__FindOrCreate | OverlayTypeClass* | TiberiumSpawnType |
| 0x645430 | ParticleTypeClass__FindOrCreate | int (index) | SpawnsParticle (corrected 2026-05-28: was ParticleSystemTypeClass__FindOrCreate; binary labels it ParticleTypeClass__FindOrCreate — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x477050 | CCINIClass__ReadLayer | int (enum) | Layer |
| 0x529880 | CCINIClass__ReadMinMax | int[2] | RandomLoopDelay, RandomRate |

## AnimClass Constructor Parameters

```c
AnimClass::Constructor(
    AnimTypeClass* type,      // param_2: the animation type
    CoordStruct*   coords,    // param_3: world position
    int            delay,     // param_4: ticks to wait before playing (0 = immediate)
    int            loopCount, // param_5: multiplied with type's LoopCount
    uint           drawFlags, // param_6: CC_Draw_Shape flags (e.g. 0x600)
    int            zAdjust,   // param_7: Z-order offset (0 = use type default)
    char           reverse    // param_8: play animation in reverse
)
```

### Constructor Behavior

1. Calls `ObjectClass::Constructor()` (base class init)
2. Initializes all AnimClass fields to defaults
3. Sets vtable pointers (4 vtables for multiple inheritance)
4. **Registers in global array**: adds `this` to `g_AnimClass_Array`
5. If `type != NULL`:
   - Auto-detects `End` (total frames) from SHP header if not set in INI
   - If `Shadow=yes`, halves the frame count
   - If `zAdjust == 0`, uses `type->ZAdjust` (offset 0x348)
   - Sets `Rate` from type, with optional random range
   - Computes `LoopCountRemaining = type->LoopCount * loopCount`
   - If `delay == 0`, calls `AnimClass::Middle` to begin immediately
   - If `Flat=yes` or `Bouncer=yes`, sets up bouncing physics

## The 0x600 Draw Flag

`0x600 = 0x200 | 0x400`

- **Bit 0x200** = **Center sprite**: In `CC_Draw_Shape`, subtracts half the sprite
  width and height from the draw position, centering the sprite on the given coords.
- **Bit 0x400** = **No effect** (not checked by CC_Draw_Shape, Blitter_selector, or
  Blitter_selector_extended). Appears to be a reserved/unused bit.

So `0x600` effectively means "center the sprite on the world coordinates."

### Full Draw Flag Reference (CC_Draw_Shape / Blitter_selector)

| Flag | Meaning |
|------|---------|
| 0x001 | Shadow/predator blitter |
| 0x002 | 25% translucent |
| 0x004 | 50% translucent |
| 0x006 | 75% translucent |
| 0x008 | Extra remap (with translucency) |
| 0x010 | Has Z-buffer data |
| 0x020 | Alternative drawing mode |
| 0x040 | Special mode |
| 0x080 | Special mode |
| 0x100 | Special mode |
| 0x200 | **Center sprite** (subtract half w/h from position) |
| 0x400 | Unused/reserved |
| 0x800 | Has remap/key color |
| 0x1000-0x2000 | Checked via `g_BlitterFlagMask_0x3000` |
| 0x4000 | Alternative blitter set |
| 0x8000 | Special mode |

In AnimClass::DrawIt, the stored flags get `| 0x2000` added before passing to
CC_Draw_Shape. If bit 0 is not set, `| 0x800` is also added.

## AnimClass::AI — Frame Advancement Logic

Called every game tick. The frame advancement works as follows:

1. **Delay countdown**: If `Delay > 0`, decrement by 1 each tick. When it reaches 0,
   call `AnimClass::Middle` to begin and return.

2. **Timer-based frame advance**: Uses a CDTimerClass countdown timer. The timer is
   set to `FrameDelayReload` (= Rate = `900 / INI_Rate`). When the timer reaches 0:
   - `CurrentFrame += FrameStep` (either +1 or -1)
   - Timer is reloaded with `FrameDelayReload`
   - `LastFrameTime = g_CurrentFrameCounter`

3. **Damage application**: If `type->Damage > 0.0` and the anim is not a bouncer,
   accumulates damage each frame. When accumulated >= 1.0, applies area damage.

4. **TrailerAnim spawning**: If `type->TrailerAnim != NULL`, spawns a copy of the
   trailer anim at the same position every `TrailerSeperation` frames.

5. **Loop/End detection**:
   - **PingPong**: When reaching end or start, reverses `FrameStep` direction.
   - **Normal**: When `CurrentFrame >= End`:
     - If `LoopCountRemaining > 1` (and not 0xFF = infinite): decrement loop count,
       reset `CurrentFrame` to `LoopStart`, apply `RandomLoopDelay` if set.
     - If `LoopCountRemaining == 1`: check for `Next` anim type. If set, replace
       `Type` pointer and restart. Otherwise, mark for deletion.
   - **Reverse**: When `CurrentFrame <= 0`, same loop/end logic applies.

6. **Self-destruction**: Sets `IsMarkedForDeletion = 1` then calls virtual function
   at vtable offset 0xF8 (`AnimClass::Destroy`).

### Rate Conversion Formula

```
internal_rate = 900 / ini_rate
```

- `Rate=120` (WARPIN/WARPOUT): internal = 7 ticks between frames
- `Rate=150` (CHRONOSK): internal = 6 ticks between frames
- `Rate=300` (WARPAWAY): internal = 3 ticks between frames

At 15 FPS game speed, Rate=120 means ~0.47s per frame, Rate=300 means ~0.2s per frame.

## AnimClass::DrawIt — Rendering

The DrawIt function handles several rendering paths:

### 1. RING1 Special Case (Warp Ring)
If the anim's name matches "RING1" and Z-buffering is available, renders a
**special expanding/fading ring effect**:
- Computes ring size from `End * CurrentFrame`
- Applies progressive alpha (fades in quickly, fades out slowly)
- Draws as a textured quad with Z-buffer integration
- This is how the chrono warp ring visual works

### 2. Translucency Handling
Based on `type->Translucent` and `type->Translucency`:
- If `Translucent=yes` with no explicit level: progressive translucency based on
  frame position (25% in first third, 50% in second, 75% in last)
- If explicit `Translucency=25/50/75`: fixed level via flags 0x2/0x4/0x6

### 3. Flat Drawing (Flat=yes)
For flat anims like warp rings, uses `type->Flat` flag. When set, the anim is drawn
as if lying on the ground plane. Uses `YDrawOffset` for vertical positioning.

### 4. Shadow Drawing (Shadow=yes)
If `Shadow=yes`, the SHP has double the frames — the second half are shadow frames.
Draws the shadow with flag `0x601` (shadow blitter + centered).

### 5. Tiled Drawing (Tiled=yes)
For tiled anims, draws the SHP frame repeatedly to fill the area, tiling vertically.

### 6. Standard Drawing
Calls `CC_Draw_Shape` with:
- The SHP frame data
- Computed screen position (world-to-screen transform + YDrawOffset + ZAdjust)
- Draw flags from `AnimClass+0x190` | `0x2000`
- Palette from `AnimClass+0xD4` or auto-detected from cell/theater

## AnimClass Lifecycle

### Creation
1. `operator_new(0x1C8)` allocates 456 bytes
2. Constructor initializes fields, sets vtables
3. Registers in `g_AnimClass_Array` (global dynamic vector)
4. If `delay == 0`, immediately calls `Middle` to begin play
5. If `Flat=yes` or `Bouncer=yes`, sets up physics simulation

### Per-Tick Update (AnimClass::AI)
1. Handles special behaviors (PsiWarning, HideIfNoOre, tiberium, etc.)
2. Processes bouncer physics (meteor impact, IsMeteor)
3. Decrements delay countdown
4. Advances frame when timer expires
5. Spawns trailer anims, applies damage
6. Handles loop/end/next transitions
7. Self-destructs when animation completes

### Destruction (AnimClass::Destroy, vtable offset 0xF8)
1. Detaches from owner object (`OwnerObject->DetachAnim`)
2. Calls `SetOwnerObject(NULL)` to clean up attachment
3. If `type->ExpireAnim != NULL`, spawns expire animation at same location
4. Calls `ObjectClass::UnInit` which adds to pending-delete list

### Attachment to TechnoClass
- `AnimClass::SetOwnerObject(obj)` attaches the anim to move with a unit
- `AnimClass+0xCC` (index 0x33) stores the ObjectClass* owner
- When attached, the anim's position follows the owner's position
- Multiple anims can share the same owner
- Detachment scans the global array to check if any other anim still references the owner

## Rules.ini Offsets for Warp Animations

All offsets relative to `g_RulesClass_Instance`:

| Offset | Key | Default Anim |
|--------|-----|-------------|
| +0x328 | ChronoBlast | |
| +0x32C | ChronoBlastDest | |
| +0x330 | ChronoPlacement | |
| +0x334 | ChronoBeam | |
| +0x338 | **WarpIn** | WARPIN |
| +0x33C | **WarpOut** | WARPOUT |
| +0x340 | **WarpAway** | WARPAWAY |
| +0x344 | **ChronoSparkle1** | CHRONOSK |
| +0x348 | IronCurtainInvokeAnim | |
| +0x34C | ForceShieldInvokeAnim | |
| +0x350 | WeaponNullifyAnim | |

## Warp Animation Properties (from art.ini/artmd.ini)

### [WARPIN] — Chrono warp-in effect
- Flat=yes, Layer=ground, Translucent=yes, Rate=120 (7 ticks/frame)
- YSortAdjust=-64 (draws behind units)
- Flat ground ring effect

### [WARPOUT] — Chrono warp-out effect
- Flat=yes, Layer=ground, Translucent=yes, Rate=120 (7 ticks/frame)
- YSortAdjust=-64
- Same visual as WARPIN

### [WARPAWAY] — Chrono erasure effect (Chrono Legionnaire kill)
- Flat=true, Layer=ground, Translucent=yes, Rate=300 (3 ticks/frame)
- TranslucencyDetailLevel=1
- Report=ChronoLegionKill (plays sound)
- Faster animation than WARPIN/WARPOUT

### [CHRONOSK] — Chrono sparkle effect
- Flat=true, Rate=150 (6 ticks/frame)
- LoopStart=0, LoopEnd=2, LoopCount=1
- ZAdjust=-124 (draws deep behind other objects)
- YSortAdjust=100 (sorts in front for visibility)
- Short looping sparkle animation

## Layer Enum (AnimTypeClass+0x364)

| Value | Name | Description |
|-------|------|-------------|
| 0 | UNCHECKED here | Older name not re-verified in the tile-animation pass |
| 1 | UNCHECKED here | Older name not re-verified in the tile-animation pass |
| 2 | Ground | Sorted DisplayClass layer; owner-bound AnimClass also forces this value |
| 3 | Top | AnimType constructor default; appended in registration order |
| 4 | UNCHECKED here | Older name not re-verified in the tile-animation pass |
| 5 | UNCHECKED here | Older name not re-verified in the tile-animation pass |

`AnimClass::GetLayer @ 0x00424CB0` is the primary-vtable `+0x78` receiver: an
owner at `+0xCC` forces Ground (2), otherwise the AnimType `+0x364` value is used,
with Top (3) as the no-type/default path. `AnimClass::GetYSort @ 0x00422BC0`
(primary vtable `+0xB8`) returns `ObjectClass::GetYSort + Anim+0x104`; the
constructor copies `+0x104` from AnimType `YSortAdjust +0x340`.

Terrain-tile creation is a post-constructor producer contract.
`CellClass::RecalcAttributes @ 0x0047D2B0` constructs with delay 0, signed loop -1, draw flags 0x1600, and
constructor ZAdjust 0. The constructor calls `AnimClass::Middle` immediately,
which may play StartSound. Only after it returns does the producer write
UseCellDrawer `+0x196`, TileAnimZAdjust `+0x100`, and TerrainAttached `+0x197`.
`MapClass::InitCellAttributes @ 0x00568BB0` deletes marked instances and recreates
the final set after map objects in the active Full_Init path.

## Global Registration

AnimClass instances are stored in a `DynamicVectorClass<AnimClass*>`:
- Data pointer: `g_AnimClass_Array` at `0x00A8E9AC`
- Count: `g_AnimClass_Array_Count` at `0x00A8E9B8`
- Capacity: `g_AnimClass_Array_Capacity` at `0x00A8E9B0`

AnimTypeClass instances are stored similarly:
- Data pointer: `g_AnimTypes_Array` at `0x008B4154`
- Count: `g_AnimTypeClass_Count` at `0x008B4160`

When an AnimClass is constructed, it adds itself to `g_AnimClass_Array`.
When `ObjectClass::UnInit` is called, the object is added to a separate
pending-delete list at `0x00B0F69C` for deferred cleanup.

## AnimTypeClass::FindOrAllocate (0x00428B80)

(corrected 2026-05-28: section previously titled FindByName; binary labels it
AnimTypeClass__FindOrAllocate via get_function_by_address — ROOT_CAUSE: RTTI_LABEL_DRIFT)

This function is a **find-or-allocate** lookup, matching the
pattern used by `WarheadTypeClass::FindOrAllocate` and siblings. The companion
`AnimTypeClass::FindOrCreate` at `0x00428F70` is a more generic variant that
takes a `DynamicVectorClass*` in `param_2`; `FindOrAllocate` is the specialized
form hardcoded against `g_AnimTypes_Array`.

### Behavior

```
if name == "<none>"  → return NULL
if name == "none"    → return NULL

for i in 0 .. g_AnimTypeClass_Count:
    existing = g_AnimTypes_Array[i]
    if stricmp(existing->ID, name) == 0:     // ID is at byte offset 0x24
        return existing

// Not found: allocate new instance and construct it
ptr = operator_new(0x378)                     // 0x378 = AnimTypeClass instance size
if ptr != NULL:
    return AnimTypeClass::Constructor(name)   // ctor at 0x00427530

return NULL
```

### Key observations

- **Name comparison uses string compare at ID offset 0x24**. This confirms the
  inherited `ID (Name string)` field at `0x024 / [0x09]` is the INI section
  key used for lookup.
- **Sentinel strings are `<none>` and `none`**. Stored at `0x00817474` and
  `0x00817694` respectively. Both return NULL without scanning the array. Any
  INI key that allows an AnimType reference can use either spelling to mean
  "no animation".
- **String compare function**: `FUN_007c8d20` at `0x007C8D20` (case-insensitive
  string compare; returns 0 on match, non-zero otherwise).
- **Construction-on-miss**: the allocated pointer from `operator_new` is
  discarded — the constructor (0x00427530) itself registers the new instance
  into `g_AnimTypes_Array` and returns it. The `ptr` returned by `operator_new`
  is used only as an allocation check; the actual pointer returned to the
  caller is the constructor's result.
- **The constructor self-registers** via `g_AnimTypes_Array` append (see
  constructor decompile lines near `0x00427842`), so a freshly-created type
  is immediately findable by subsequent calls.

### Usage pattern

All INI fields that reference an AnimType (Next, Spawns, BounceAnim,
ExpireAnim, TrailerAnim) call through `FindOrCreate` (0x00428F70), which has
the same body as FindOrAllocate but against a caller-supplied vector. Game code
outside of INI loading (e.g. scripted anim spawns) uses `FindOrAllocate` directly.

## FUN_005F9070 — ObjectTypeClass Theater Image Loader

Resolves and loads the theater-appropriate SHP for any `ObjectTypeClass`-derived
type (AnimType, BulletType, OverlayType, etc.). Called from
`ObjectTypeClass::ReadINI` at `0x005F964D`, from `AnimTypeClass::ReadINI`-region
code at `0x00427B71` and `0x00428855`, from `BulletTypeClass::ReadINI` at
`0x0046C406`, from `OverlayTypeClass::Load` at `0x005FEB45`, and from several
other type loaders.

This is **inherited ObjectTypeClass logic**, not AnimTypeClass-specific, but is
on the AnimTypeClass image-resolution path and is worth having on-record.

### Signature and inputs

```c
void __fastcall FUN_005F9070(int *this)   // this = ObjectTypeClass*
```

`this` is typed `int *` in Ghidra, so `this[N]` is byte offset `N*4`.

Fields touched (all relative to `ObjectTypeClass`):

| Byte Offset | Access | Purpose (observed) |
|-------------|--------|---------------------|
| `0x0A4` (`this[0x29]`) | read/write | **SHPData** — cached loaded SHP pointer. Freed via `FUN_007C8B3D` before reload. |
| `0x0A8` (`this[0x2A]`) | read/write | Byte flag, cleared to 0 after freeing SHPData. Semantics: "SHPData was externally provided, don't free". |
| `0x1EC` (`this[0x7B]`) | write | **Cached SHP dimension** (max of width/height from SHP header, clamped to ≥8). |
| `0x1F8` (`this[0x7E]`) | read/write as **25-byte string** | **Image filename buffer** — NOT a bool. Read by `FUN_007C9FF0` with a format extracting the first two chars; rewritten in-place by `_strncpy` with a 24-byte theater-transformed filename. |
| `0x211` | read, byte | Gate bool. When 0 OR theater != 1 (non-snow) OR `0xA8` != 0 → clear `0x212`. Probable "Image= was set in INI". |
| `0x212` | write, byte | "Snow-theater suffix applied" marker. Set to 1 after the `%sA` transform so it isn't applied twice. |
| `0x22C` (`this[0x8B]`) | read, byte | **Theater** flag (already documented). If set, uses full theater-image template instead of 2nd-char substitution. |
| `0x237` | read, byte | **NewTheater** flag (already documented). Gates the 2nd-char substitution branch. |
| vtable + 0x2C | call | Returns an integer RTTI/asset-type discriminator (values 0x05/0x15/0x1E/0x25 observed, selecting MIX filetype). |

### Theater transform logic (the interesting part)

RA2/YR uses two mutually-exclusive filename-transform schemes to resolve
per-theater assets:

**1. `Theater=yes` branch** (via `FUN_007C9FF0`, a printf-like helper):
```
filename = sprintf(template_for_theater, Image_field_at_0x1F8)
```
Template is looked up at `&DAT_007E1BC6 + theater_index * 0x70` — each theater
occupies a 0x70-byte row. This produces a fully-templated per-theater
filename.

**2. `NewTheater=yes` branch** (the classic RA2 convention):
```
if Image[0] ∈ {'g','n','c','y'} && Image[1] ∈ {'a','t'}:
    Image[1] = theater_letter_at (&DAT_007E1BCE + theater_index * 0x70)
```
Replaces the second character of the filename with the theater-specific letter
(T, A, U, L, D, etc.). This is the well-known "gatech → gttech" pattern.

**3. Snow-theater special-case** (top of function, separate from the above):
```
if HasImage_override (0x211) && theater == 1 (snow) && !SHPData_external (0x2A):
    if (!already_applied_flag_0x212):
        filename = sprintf("%sA", Image_field_at_0x1F8)
        strncpy(Image_field_at_0x1F8, filename, 0x18)   // 24 bytes
        already_applied_flag_0x212 = 1
```
This tacks an 'A' suffix onto the Image filename for snow theater when a
custom Image override is set — a one-shot rewrite cached via the marker byte.
Format string `"%sA"` lives at `0x00832AE8`.

### MIX loading

After the filename transform:

1. Dispatch on `vtable[0x2C]()` — the ObjectTypeClass asset-type getter:
   - Returns `0x15` or `0x05` → `LoadFileFromMIX` with default extension
   - Returns `0x1E` or `0x25` → skip to `LAB_005F928C` (no MIX load — e.g. voxel)
   - Otherwise → `LoadFileFromMIX` with default extension
2. If load returned NULL, set `Image[1] = 'G'` (fallback character) and retry
   `LoadFileFromMIX`.
3. On success, store pointer in `this[0x29]` (SHPData at 0x0A4).
4. Cache the larger of width/height from the SHP header into `this[0x7B]`
   (minimum 8).

Extension format `.SHP` lives at `0x0081834C`.

### Discrepancy flagged — ObjectTypeClass 0x1F8

The current inherited-fields table in this document lists:

> `0x1F8 | [0x7E] | HasImage | (inherited) | bool, gates Theater/NewTheater reading`

That is **inconsistent with this decompile**. FUN_005F9070 treats `0x1F8` as
the start of a 25-byte **Image filename string**, not a bool. The actual
"image override was set" gate appears to live at `0x211` (one byte past the
end of a 25-byte string starting at `0x1F8`), and the "snow suffix applied"
marker is the adjacent byte at `0x212`.

This is noted here as a follow-up for when ObjectTypeClass itself is fully
mapped — it does not affect any field at offset `≥ 0x294` (the AnimTypeClass
region), so the rest of the layout stands.

### TS-legacy check

This function is alive in a normal YR skirmish: `ObjectTypeClass::ReadINI`
calls it unconditionally during INI parsing, and every theater-dependent SHP
(ore overlays, anims like `ELECTRIC`/`CRATE`, etc.) goes through this path.
Not TS-dead.
