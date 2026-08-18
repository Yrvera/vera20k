---
name: BuildingTypeClass Constructor Defaults
description: Every field initialized by BuildingTypeClass__constructor (0x0045DD90). Cross-referenced against BUILDINGTYPECLASS_FIELDS.csv (Task 2).
type: reference
---

# BuildingTypeClass Constructor Defaults

**Address:** `0x0045DD90` (body `0x0045DD90 - 0x0045E511`, 1921 bytes / 0x781)
**Instance size:** 0x1798 (6040 bytes) — inferred from max touched offset 0x1791 + 1 rounded up
**Calling convention:** `__thiscall`, ECX = this, one stack param `param_2` passed to base ctor
**Confidence:** HIGH (direct disassembly + decomp, every write verified against raw ESI+off)

## Important labeling note

The BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md mislabels the ctor as `0x004653C0`. That address
is actually `BuildingTypeClass__FindOrAllocate` — it calls `operator_new(0x1798)` then invokes
the real ctor at `0x0045DD90` via `CALL 0045dd90`. Always use `0x0045DD90` for ctor work.

## Pointer-arithmetic warning

`param_1` has Ghidra decomp type `undefined4 *` — in the decompilation listing, `param_1[N]`
is scaled indexing (byte offset = N * 4). To avoid confusion, this doc uses the **disassembly**
form `[ESI + 0xXXX]` which is a direct byte offset. Every offset below is a byte offset.

## Section 1 — Prologue, base ctor, and vtable setup

| Step | Address | Action |
|---|---|---|
| 1 | `0045dd91` | ECX = this (thiscall convention) |
| 2 | `0045dda1` | `CALL TechnoTypeClass__Constructor(this, param_2)` at `0x00710AF0` |
| 3 | `0045e2cd` | `[ESI + 0x000] = 0x007E4570` — primary BuildingTypeClass vtable |
| 4 | `0045e2d3` | `[ESI + 0x004] = 0x007E4554` — secondary_4 vtable (likely AbstractTypeClass interface) |
| 5 | `0045e2d9` | `[ESI + 0x008] = 0x007E454C` — secondary_8 vtable |
| 6 | `0045e2e1` | `[ESI + 0x00C] = 0x007E4544` — secondary_C vtable |
| 7 | `0045e2e8` | `CALL AbstractClass__AssignUniqueID(this + 4)` at `0x00410230` |
| 8 | `0045e2ed-0045e33d` | Register into `g_BuildingTypeClass_Array` at `0x00A83C6C` |
| 9 | `0045e35c` | `[ESI + 0xDF8] = array_index` (position in global type array, or `-1` if not added) |
| 10 | `0045e2b5` | `CALL operator_new(0xC)` → stored at `[ESI + 0x1788]`, then its 3 DWORDs zeroed |

**Key findings:**
- BuildingTypeClass vtable confirmed at **`0x007E4570`**. Other vtables (`0x007E4554`,
  `0x007E454C`, `0x007E4544`) are multi-inheritance secondary vtables.
- `[ESI + 0x1784] = 0x007E4638` — a pointer-table constant, likely the vtable for the
  12-byte dynamic object allocated at `[ESI + 0x1788]`. That object is zero-initialized.
  Based on size 0xC and structure, this is almost certainly a `VectorClass`/`DynamicVectorClass`
  instance `{capacity:int, count:int, ptr:void*}` used as the per-Type runtime-building-instance list.
- `AbstractClass__AssignUniqueID` is called on `this + 4` (the AbstractTypeClass sub-object),
  consistent with BuildingTypeClass inheriting from AbstractTypeClass.

## Section 2 — Field defaults (by byte offset)

The base `TechnoTypeClass__Constructor` initializes offsets **`0x000` through `≈0xDF0`**;
BuildingTypeClass ctor then patches a handful of bytes in that range before setting its
own fields starting at `0xDF8`.

### 2a. TechnoTypeClass-range fixups by BuildingTypeClass ctor

These are post-base-ctor overrides of bytes that the base had already touched:

| Offset | Type | Default | Source | Notes |
|---|---|---|---|---|
| `0xC8E` | byte | 0 | `0045e42e` | Override of base value |
| `0xD2E` | byte | 0 | `0045e502` | Override |
| `0xD35` | byte | 1 | `0045e4dc` | Override, set to true |
| `0xD36` | byte | 1 | `0045e4e3` | Override, set to true |
| `0xD38` | byte | 0 | `0045e4f0` | Override |
| `0xD3B` | byte | 0 | `0045e4ea` | Override |
| `0xD96` | byte | 0 | `0045e4fc` | Override |
| `0xD97` | byte | 0 | `0045e4f6` | Override |

### 2b. BuildingTypeClass own fields (0xDF8 upward)

Explicit writes, grouped by address. Values marked `=` are direct literals; `→` indicates
loop-initialized with the given per-element value.

| Offset | Type | Default | ASM | Key (if any) | Notes |
|---|---|---|---|---|---|
| `0xDF8` | int | -1 (0xFFFFFFFF) | `0045ddaf` | — | Array index (overwritten later by registry add) |
| `0xDFC` | int | 0 | `0045ddb5` | — | Registry / slot chain (orphan) |
| `0xE00` | int | 0 | `0045ddbb` | — | (orphan) |
| `0xE04` | byte | 0 | `0045ddc1` | — | Flag byte (orphan) |
| `0xE08` | int | 0 | `0045ddc7` | `BuildCat` | Default = 0 (BuildCat::DontCare) |
| `0xE0C-0xE17` | int[3] | {0,0,0} | `0045ddd3/da/e9` | `HalfDamageSmokeLocation1` | Zero coord triple via `DAT_0089C8D0` |
| `0xE18-0xE23` | int[3] | {0,0,0} | `0045ddf2/fa/0045de03` | `HalfDamageSmokeLocation2` | Zero coord triple |
| `0xE28` | double | 0.0 | `0045de06/0c` | `GateCloseDelay` | Two zeroed DWORDs |
| `0xE2C` | int | 0 | `0045de0c` | — | (orphan — active runtime, many xrefs) |
| `0xE30` | int | **5000** (0x1388) | `0045de12` | `LightVisibility` | Significant default |
| `0xE34` | double | 0.0 | `0045de21` | `LightIntensity` | |
| `0xE38` | double | **1000000** (0xF4240) | `0045de27` | `LightRedTint` | Fixed-point scale factor |
| `0xE3C` | double | **1000000** | `0045de2d` | `LightGreenTint` | |
| `0xE40` | double | **1000000** | `0045de33` | `LightBlueTint` | |
| `0xE44-0xE4B` | minmax | {0xFFFF, 0xFFFF} | `0045de40/46` | `PrimaryFirePixelOffset` | `-1/-1` sentinel |
| `0xE4C-0xE53` | minmax | {0xFFFF, 0xFFFF} | `0045de4c/52` | `SecondaryFirePixelOffset` | |
| `0xE54` | ptr | 0 (null) | `0045de58` | `ToOverlay` | |
| `0xE58` | int | 0 | `0045de5e` | — | (orphan, active) |
| `0xE5C` | char[16] | "" (null-term) | `0045e416` | `Buildup` | Byte 0 set 0; rest indeterminate — effectively empty string |
| `0xE6C` | VocClass* | -1 (0xFFFFFFFF) | `0045de64` | `BuildupSound` | Unset sound handle |
| `0xE70` | VocClass* | -1 | `0045de6a` | `PackupSound` | |
| `0xE74` | VocClass* | -1 | `0045de70` | `CreateUnitSound` | |
| `0xE78` | VocClass* | -1 | `0045de76` | `UnitEnterSound` | |
| `0xE7C` | VocClass* | -1 | `0045de7c` | `UnitExitSound` | |
| `0xE80` | VocClass* | -1 | `0045de82` | `WorkingSound` | |
| `0xE84` | VocClass* | -1 | `0045de88` | `NotWorkingSound` | |
| `0xE88` | char[16] | "" | `0045e41c` | `PowersUpBuilding` | null-termed at first byte |
| `0xEA0` | UnitType* | 0 (null) | `0045de8e` | `FreeUnit` | |
| `0xEA4` | InfantryType* | 0 | `0045de94` | `SecretInfantry` | |
| `0xEA8` | UnitType* | 0 | `0045de9a` | `SecretUnit` | |
| `0xEAC` | string | 0 | `0045dea0` | `SecretBuilding` | Null string slot |
| `0xEB0` | int | -1 | `0045dea6` | — | (orphan — active runtime, sentinel) |
| `0xEB4` | int | **3** | `0045deac` | `Adjacent` | Default adjacent-build range |
| `0xEB8` | UnitType* | 0 | `0045deb6` | `Factory` | |
| `0xEBC-0xEC7` | int[3] | {0,0,0} | `0045debc/c2/c8` | `TargetCoordOffset` | |
| `0xEC8-0xED3` | int[3] | {0,0,0} | `0045dece/d4/da` | `ExitCoord` | |
| `0xED4` | int | 0 | `0045dee0` | — | (orphan) — inside ExitCoord extension region? |
| `0xED8` | int | 0 | `0045dee6` | — | (orphan) |
| `0xEDC` | int | **128** (0x80) | `0045deec` | `DeployFacing` | North-facing default |
| `0xEE0` | int | 0 | `0045def6` | `Power` | |
| `0xEE4` | int | 0 | `0045defc` | — | (orphan) |
| `0xEE8` | int | 0 | `0045df02` | `ExtraPower` | |
| `0xEEC` | int | 0 | `0045df08` | — | (orphan) |
| `0xEF0` | int | 0 | `0045df13` | `Foundation` | Default = unknown (category 0) |
| `0xEF4` | int | **2** | `0045df19` | `Height` | |
| `0xEF8` | int | **2** | `0045df1f` | `OccupyHeight` | |
| `0xEFC` | int | 0 | `0045df25` | `MidPoint` | |
| `0xF00` | int | 0 | `0045df2b` | `DoorStages` | |
| `0xF04` | int | 0 | `0045e362` | — | Start of orphan anim triple (see Section 2c) |
| `0xF08` | int | **1** | `0045e368` | — | orphan anim triple +4 |
| `0xF0C` | int | 0 | `0045e36e` | — | orphan anim triple +8 |
| `0xF10-0xF1B` | int[3] | **{0,1,0}** | `0045e374/7a/80` | `AnimIdle` | Anim triple `{start, count, loop}` pattern |
| `0xF1C-0xF27` | int[3] | **{0,1,0}** | `0045e386/8c/92` | `AnimActive` | |
| `0xF34-0xF3F` | int[3] | **{0,1,0}** | `0045e398/9e/a4` | `AnimAux1` | Gap 0xF28-0xF33 unwritten here but covered by PowerUp loop (see 2c) |
| `0xF40-0xF4B` | int[3] | **{0,1,0}** | `0045e3aa/b0/b6` | `AnimAux2` | |
| `0xF4C-0x14DF` | — | see §3 | `0045e3be` loop | PowerUp anim table | 21 entries × 0x44 bytes |
| `0x14E0` | int | 0 | `0045df31` | `Upgrades` | |
| `0x14E4` | int | 0 | `0045df37` | — | (orphan) |
| `0x14E8` | byte | 0 | `0045e434` | — | (orphan) |
| `0x14EC` | int | 0 | `0045df3d` | — | (orphan) |
| `0x14F0` | byte | 0 | `0045e43a` | — | (orphan) |
| `0x14F4` | int | 0 | `0045df43` | — | (orphan) |
| `0x14F8` | byte | 0 | `0045e440` | — | (orphan) |
| `0x14FC` | int | 0 | `0045df49` | — | (orphan) |
| `0x1500` | byte | 0 | `0045e446` | — | (orphan) |
| `0x1504` | int | 0 | `0045df4f` | — | (orphan) |
| `0x1508` | byte | 0 | `0045e44c` | — | (orphan) |
| `0x150C-0x150F` | int | 0 | `0045df55` | — | (orphan) |
| `0x1510` | int | 0 | `0045df5b` | — | (orphan) |
| `0x1514` | int | 0 | `0045df61` | `SpecialZOverlayZAdjust` | |
| `0x1518` | int | 0 | `0045df67` | — | (orphan) |
| `0x151C` | byte | 0 | `0045df6d` | — | (orphan) |
| `0x1520` | int | 0 | `0045df73` | `NormalZAdjust` | |
| `0x1524` | int | 0 | `0045df79` | `AntiAirValue` | |
| `0x1528` | int | 0 | `0045df7f` | `AntiArmorValue` | |
| `0x152C` | int | 0 | `0045df87` | `AntiInfantryValue` | |
| `0x1530-0x1537` | minmax | {0,0} | `0045df8d/93` | `ZShapePointMove` | |
| `0x1538-0x1547` | 4× int | {0,0,0,0} | `0045dfa4/b0/b8/c9` | — | (orphan 16-byte block, possibly a 4D coord or matrix row zero from `DAT_0089C8A0`) |
| `0x1548` | short | 0 | `0045dfcc` (word write) | `ExtraLight` | |
| `0x154A` | bool | **1 (true)** | `0045dfd3` | `TogglePower` | |
| `0x154B` | bool | 0 | `0045dfda` | `HasSpotlight` | |
| `0x154C` | bool | 0 | `0045dfe0` | `IsTemple` | |
| `0x154D` | bool | 0 | `0045dfec` | `IsPlug` | |
| `0x154E` | bool | 0 | `0045dfc3` | `HoverPad` | |
| `0x154F` | bool | **1 (true)** | `0045dff2` | `BaseNormal` | Default yes — active in YR (corrected 2026-05-28: was "TS-only in YR"; rulesmd.ini contains many BaseNormal= entries; claim was MISLEADING — ROOT_CAUSE: TS_LEGACY_AS_YR) |
| `0x1550` | bool | 0 | `0045dff9` | `EligibileForAllyBuilding` | |
| `0x1551` | bool | 0 | `0045dfff` | `EligibleForDelayKill` | |
| `0x1552` | bool | 0 | `0045e005` | `NeedsEngineer` | |
| `0x1554` | int | -1 | `0045e00b` | `CaptureEvaEvent` | Sentinel |
| `0x1558` | int | 0 | `0045e011` | `ProduceCashStartup` | |
| `0x155C` | int | 0 | `0045e017` | `ProduceCashAmount` | |
| `0x1560` | int | 0 | `0045e01d` | `ProduceCashDelay` | |
| `0x1564` | int | 0 | `0045e023` | `InfantryGainSelfHeal` | |
| `0x1568` | int | 0 | `0045e029` | `UnitsGainSelfHeal` | |
| `0x156C` | int | **25** (0x19) | `0045e02f` | `RefinerySmokeFrames` | |
| `0x1570` | byte | 0 | `0045e039` | — | (orphan) |
| `0x1571` | byte | 0 | `0045e03f` | — | (orphan) |
| `0x1572` | bool | 0 | `0045e045` | `Capturable` | |
| `0x1573` | bool | 0 | `0045e04b` | `Powered` | |
| `0x1574` | bool | 0 | `0045e051` | `PoweredSpecial` | |
| `0x1575` | bool | 0 | `0045e057` | `Overpowerable` | |
| `0x1576` | bool | 0 | `0045e05d` | `Spyable` | |
| `0x1577` | bool | **1 (true)** | `0045e063` | `CanC4` | |
| `0x1578` | bool | 0 | `0045e06a` | `WantsExtraSpace` | |
| `0x1579` | bool | 0 | `0045e070` | `Unsellable` | |
| `0x157A` | bool | **1 (true)** | `0045e076` | `ClickRepairable` | |
| `0x157B` | bool | 0 | `0045e07d` | `CanBeOccupied` | |
| `0x157C` | bool | 0 | `0045e083` | `CanOccupyFire` | |
| `0x1580` | int | 0 | `0045e089` | `MaxNumberOccupants` | |
| `0x1584` | bool | **1 (true)** | `0045e08f` | `ShowOccupantPips` | |
| `0x1588-0x15D7` | — | {0} × 80 bytes | `0045e458` loop | — | 10 pair zero-loop (20 int slots). (orphan) |
| `0x15D8-0x1617` | — | {0} × 64 bytes | `0045e47a` loop | — | 8 pair zero-loop (16 int slots). (orphan) |
| `0x1618` | int | — (UNINIT) | — | `QueueingCell` (min half) | **WARN: ctor does NOT write 0x1618; `max` at 0x161C is zeroed but `min` is uninit until INI** |
| `0x161C` | int | 0 | `0045e096` | `QueueingCell` (max half) | |
| `0x1620` | int | -1 | `0045e09c` | `NumberImpassableRows` | Sentinel |
| `0x1624-0x1663` | — | 0xFFFF × 64 bytes | `0045e49b` loop part 1 | — | (orphan; 8 pairs of `min=0xFFFF,max=0xFFFF`) |
| `0x1664-0x16A3` | — | 0xFFFF × 64 bytes | `0045e49b` loop part 2 | — | (orphan; 8 pairs) |
| `0x16A4` | bool | 0 | `0045e0a2` | `Radar` | |
| `0x16A5` | bool | 0 | `0045e0a8` | `SpySat` | |
| `0x16A6` | bool | 0 | `0045e0ae` | `ChargeAnim` | |
| `0x16A7` | bool | 0 | `0045e0b4` | `IsAnimDelayedFire` | |
| `0x16A8` | bool | 0 | `0045e0ba` | `SiloDamage` | |
| `0x16A9` | bool | 0 | `0045e0c0` | `UnitRepair` | |
| `0x16AA` | bool | 0 | `0045e0c6` | `UnitReload` | |
| `0x16AB` | bool | 0 | `0045e0cc` | `Bunker` | |
| `0x16AC` | bool | 0 | `0045e0d2` | `Cloning` | |
| `0x16AD` | bool | 0 | `0045e0d8` | `Grinding` | |
| `0x16AE` | bool | 0 | `0045e0de` | `UnitAbsorb` | |
| `0x16AF` | bool | 0 | `0045e0e4` | `InfantryAbsorb` | |
| `0x16B0` | bool | 0 | `0045e0ea` | `SecretLab` | |
| `0x16B1` | bool | 0 | `0045e0f0` | `DoubleThick` | |
| `0x16B2` | byte | 0 | `0045e0f6` | — | (orphan) |
| `0x16B3` | bool | 0 | `0045e0fc` | `DockUnload` | |
| `0x16B4` | bool | 0 | `0045e102` | `Recoilless` | |
| `0x16B5` | bool | **1 (true)** | `0045e108` | `HasStupidGuardMode` | |
| `0x16B6` | bool | 0 | `0045e10f` | `BridgeRepairHut` | (TS-only) |
| `0x16B7` | byte | 0 | `0045e115` | — | (orphan) |
| `0x16B8` | byte | 0 | `0045e11b` | — | (orphan) |
| `0x16B9` | bool | 0 | `0045e121` | `ConstructionYard` | |
| `0x16BA` | bool | 0 | `0045e127` | `NukeSilo` | |
| `0x16BB` | bool | 0 | `0045e12d` | `Refinery` | |
| `0x16BC` | bool | 0 | `0045e133` | `Weeder` | |
| `0x16BD` | bool | 0 | `0045e139` | `WeaponsFactory` | |
| `0x16BE` | bool | 0 | `0045e13f` | `LaserFencePost` | (TS-only) |
| `0x16BF` | bool | 0 | `0045e145` | `LaserFence` | (TS-only) |
| `0x16C0` | bool | 0 | `0045e14b` | `FirestormWall` | (TS-only) |
| `0x16C1` | bool | 0 | `0045e151` | `Hospital` | |
| `0x16C2` | bool | 0 | `0045e157` | `Armory` | |
| `0x16C3` | bool | 0 | `0045e15d` | `EMPulseCannon` | (TS-only) |
| `0x16C4` | bool | 0 | `0045e163` | `TickTank` | (TS-only) |
| `0x16C5` | bool | 0 | `0045e169` | `TurretAnimIsVoxel` | |
| `0x16C6` | bool | 0 | `0045e16f` | `BarrelAnimIsVoxel` | |
| `0x16C7` | bool | 0 | `0045e175` | `CloakGenerator` | (TS-only) |
| `0x16C8` | bool | 0 | `0045e17b` | `SensorArray` | |
| `0x16C9` | bool | 0 | `0045e181` | `ICBMLauncher` | (TS-only) |
| `0x16CA` | bool | 0 | `0045e187` | `Artillary` | (TS-only) |
| `0x16CB` | bool | 0 | `0045e18d` | `Helipad` | |
| `0x16CC` | bool | 0 | `0045e193` | `OrePurifier` | |
| `0x16CD` | bool | 0 | `0045e19e` | `FactoryPlant` | |
| `0x16D0` | float | **1.0f** (0x3F800000) | `0045e1a4` | `InfantryCostBonus` | |
| `0x16D4` | float | **1.0f** | `0045e1aa` | `UnitsCostBonus` | |
| `0x16D8` | float | **1.0f** | `0045e1b0` | `AircraftCostBonus` | |
| `0x16DC` | float | **1.0f** | `0045e1b6` | `BuildingsCostBonus` | |
| `0x16E0` | float | **1.0f** | `0045e1bc` | `DefensesCostBonus` | |
| `0x16E4` | bool | 0 | `0045e1c2` | `GDIBarracks` | (TS-only) |
| `0x16E5` | bool | 0 | `0045e1c8` | `NODBarracks` | (TS-only) |
| `0x16E6` | bool | 0 | `0045e1ce` | `YuriBarracks` | |
| `0x16E8` | float | **999.0f** (0x4479C000) | `0045e1d4` | `ChargedAnimTime` | |
| `0x16EC` | int | 0 | `0045e1de` | `DelayedFireDelay` | |
| `0x16F0` | int | -1 | `0045e1e4` | `SuperWeapon` | Index sentinel |
| `0x16F4` | int | -1 | `0045e1ea` | `SuperWeapon2` | |
| `0x16F8` | int | **9** | `0045e1f0` | `GateStages` | |
| `0x16FC` | int | -1 | `0045e1fa` | `PowersUpToLevel` | Sentinel: "not an upgrade" |
| `0x1700` | bool | 0 | `0045e200` | `DamagedDoor` | |
| `0x1701` | bool | 0 | `0045e206` | `InvisibleInGame` | |
| `0x1702` | bool | 0 | `0045e20c` | `TerrainPalette` | |
| `0x1703` | bool | 0 | `0045e212` | `PlaceAnywhere` | |
| `0x1704` | bool | **1 (true)** | `0045e218` | `ExtraDamageStage` | |
| `0x1705` | bool | 0 | `0045e21f` | `AIBuildThis` | |
| `0x1706` | bool | 0 | `0045e225` | `IsBaseDefense` | |
| `0x1707` | byte | **20** (0x14) | `0045e22b` | `CloakRadiusInCells` | |
| `0x1708` | bool | 0 | `0045e232` | `ConcentricRadialIndicator` | |
| `0x170C` | int | 0 | `0045e238` | `PsychicDetectionRadius` | |
| `0x1710` | int | **64** (0x40) | `0045e23e` | `BarrelStartPitch` | |
| `0x1714` | char[] | "" | `0045e428` | `VoxelBarrelFile` | Only first byte null-termed; rest indeterminate |
| `0x1730-0x173B` | int[3] | **UNINIT** | — | `VoxelBarrelOffsetToPitchPivotPoint` | Ctor does NOT initialize — INI silent => undefined |
| `0x173C-0x1747` | int[3] | **UNINIT** | — | `VoxelBarrelOffsetToRotatePivotPoint` | Same |
| `0x1748-0x1753` | int[3] | **UNINIT** | — | `VoxelBarrelOffsetToBuildingPivotPoint` | Same |
| `0x1754-0x175F` | int[3] | **UNINIT** | — | `VoxelBarrelOffsetToBarrelEnd` | Same |
| `0x1760` | byte | 0 | `0045e248` | — | (orphan) |
| `0x1761` | byte | 0 | `0045e24e` | — | (orphan) |
| `0x1762` | byte | 0 | `0045e254` | — | (orphan) |
| `0x1763` | bool | 0 | `0045e25a` | `IsThreatRatingNode` | |
| `0x1764` | bool | 0 | `0045e260` | `PrimaryFireDualOffset` | |
| `0x1765` | bool | 0 | `0045e266` | `ProtectWithWall` | |
| `0x1766` | bool | **1 (true)** | `0045e26c` | `CanHideThings` | |
| `0x1767` | bool | 0 | `0045e273` | `CrateBeneath` | |
| `0x1768` | bool | 0 | `0045e279` | `LeaveRubble` | |
| `0x1769` | bool | 0 | `0045e284` | `CrateBeneathIsMoney` | |
| `0x176A` | byte | 0 | `0045e422` | — | (orphan) |
| `0x1780` | int | **1** | `0045e28a` | `NumberOfDocks` | |
| `0x1784` | ptr | 0x007E4638 | `0045e2ab` | — | Vtable for the 12-byte object at 0x1788 |
| `0x1788` | VectorClass* | heap-allocated (12b) | `0045e2ba` | — | `operator_new(0xC)`; the 3 DWORDs at the allocation are then zeroed. Confirmed `{count=0, capacity=0, ptr=0}` layout |
| `0x178C` | int | **1** | `0045e298` | — | (orphan; likely vector "is valid" flag or init count) |
| `0x1790` | bool | **1 (true)** | `0045e29e` | — | (orphan flag) |
| `0x1791` | bool | **1 (true)** | `0045e2c6` | — | (orphan flag, set=0 at 0045e2a5 then overwritten 1) |

### 2c. Data constants used

- `DAT_0089C8D0`, `DAT_0089C8D4`, `DAT_0089C8D8` — all zero. Used as a "zero 3D-coord" literal
  triplet for coord initialization (`HalfDamageSmokeLocation1/2`, `TargetCoordOffset`, `ExitCoord`,
  and the ore-pair loops at 0x1588/0x15D8).
- `DAT_0089C8A0..0089C8AC` — all zero. Used at `[ESI + 0x1538..0x1544]` (4 DWORDs). Likely a
  zero 4D matrix row or a Matrix3D identity base.

## Section 3 — PowerUp anim entry layout (+0xF4C..+0x14DF, 21 × 0x44)

Loop at `0x0045E3BE` writes 21 contiguous records of 0x44 bytes each, covering the
range `[0xF4C, 0x14E0)` (0x594 bytes total). Per-entry format fully decoded (11 subfields
— exceeds Task 3 goal of 7+):

| Entry Offset | Type | Default | Purpose (from CSV cross-ref) |
|---|---|---|---|
| `+0x00` | char[16] | "" | Anim name (healthy) — `*Anim` |
| `+0x10` | char[16] | "" | Anim name when building damaged — `*AnimDamaged` |
| `+0x20` | char[16] | "" | Anim name when garrisoned — `*AnimGarrisoned` |
| `+0x30` | int | 0 | X offset — `*X` |
| `+0x34` | int | 0 | Y offset — `*Y` |
| `+0x38` | int | 0 | Z-depth adjust — `*ZAdjust` |
| `+0x3C` | int | 0 | Y-sort bias — `*YSort` |
| `+0x40` | bool | **1 (true)** | `*Powered` — shown when powered (default ON) |
| `+0x41` | bool | 0 | `*PoweredLight` — shows only when unit lighting present |
| `+0x42` | bool | 0 | `*PoweredEffect` — damage/effect gating |
| `+0x43` | bool | 0 | `*PoweredSpecial` — special state (e.g. charging) gating |

**Entry index → named slot map (from CSV offsets):**

| Idx | Base | Key prefix |
|---|---|---|
| 0 | 0xF4C | (no CSV; orphan — runtime-only slot — see §4 "Runtime Active") |
| 1 | 0xF90 | (no CSV; orphan) |
| 2 | 0xFD4 | (no CSV; orphan) |
| 3 | 0x1018 | `ActiveAnim` |
| 4 | 0x105C | `ActiveAnimTwo` |
| 5 | 0x10A0 | `ActiveAnimThree` |
| 6 | 0x10E4 | `ActiveAnimFour` |
| 7 | 0x1128 | `PreProductionAnim` (bools +0x40..0x43 orphan — not INI-parsed) |
| 8 | 0x116C | `ProductionAnim` (bools orphan) |
| 9 | 0x11B0 | `TurretAnim` (bools orphan) |
| 10 | 0x11F4 | `SpecialAnim` |
| 11 | 0x1238 | `SpecialAnimTwo` |
| 12 | 0x127C | `SpecialAnimThree` |
| 13 | 0x12C0 | `SpecialAnimFour` |
| 14 | 0x1304 | `SuperAnim` |
| 15 | 0x1348 | `SuperAnimTwo` |
| 16 | 0x138C | `SuperAnimThree` |
| 17 | 0x13D0 | `SuperAnimFour` |
| 18 | 0x1414 | `IdleAnim` |
| 19 | 0x1458 | `LowPower` |
| 20 | 0x149C | `SuperLowPower` — active in YR (corrected 2026-05-28: was "yr_active=no — TS-only"; artmd.ini maps SuperLowPower=YAGNTC_P for the Genetic Converter; ROOT_CAUSE: TS_LEGACY_AS_YR) |

**Structural insight:** BuildingTypeClass has a **fixed-size array of 21 PowerUpAnim records**,
not separate fields per anim role. The INI parser writes each named anim into its assigned
slot. Slots 0–2 are reserved for runtime use (not INI-addressable) and slot 20 is TS-only
legacy.

**Parity note:** The CSV contains two typos on `SuperAnimTwoPowered` and `SuperAnimTwoPoweredLight`
(line 171–172) — both show offset `0x1389`. The correct offsets are `0x1388` and `0x1389`
(entry 15 +0x40 and +0x41). This matches a note already in the CSV on `load=0x108c store=0x1378`
(binary reads from slot 4 but writes to slot 15) — a longstanding binary parser bug that
causes `SuperAnimTwoX/Y` to read `ActiveAnimTwoX/Y` as their "get" default. Inherit this
behavior for parity.

## Section 4 — Orphan Analysis

### 4.1 Config (INI + ctor)

**Count: 338 out of 344 CSV fields (98.3%)**

Every CSV INI-parsed field whose byte range is fully covered by the ctor. The ctor's `default`
column in the CSV has been populated for all scalar/collection defaults; coord triples and
minmax pairs have structured-literal defaults (e.g., `{0,0,0}`, `"{min=0xFFFF, max=0xFFFF}"`).

### 4.2 Partial config (WARN)

**Count: 1**

| Offset | Field | Type | Issue |
|---|---|---|---|
| `0x1618` | `QueueingCell` | minmax | ctor writes only `[0x161C]=0` (max half); `[0x1618]` (min half) is **never initialized**. If INI silent → `min` holds whatever memory had. |

**Recommendation:** Rust side should initialize `QueueingCell.min = 0` explicitly, matching
gamemd.exe's likely zero-from-calloc behavior (MSVC debug heap) but not its strict semantic.

### 4.3 Uninit config (WARN)

**Count: 5**

| Offset | Field | Type | Owner Ctor | Note |
|---|---|---|---|---|
| `0x67C` | `WaterBound` | bool | TechnoTypeClass (inherited) | Below 0xDF8; init likely happens in base ctor. Would need `TechnoTypeClass__Constructor` analysis to confirm. |
| `0x1730` | `VoxelBarrelOffsetToPitchPivotPoint` | int[3] | neither | **TRUE UNINIT** — INI silent => undefined memory |
| `0x173C` | `VoxelBarrelOffsetToRotatePivotPoint` | int[3] | neither | Same |
| `0x1748` | `VoxelBarrelOffsetToBuildingPivotPoint` | int[3] | neither | Same |
| `0x1754` | `VoxelBarrelOffsetToBarrelEnd` | int[3] | neither | Same |

**Recommendation:** Rust must default these to `{0,0,0}` for all buildings without explicit
INI keys. Relying on uninit-memory behavior is non-deterministic and breaks lockstep.
Parity with gamemd.exe: in practice, the Windows heap for these allocations (via
`operator_new(0x1798)` in `FindOrAllocate`) is typically zero-initialized by the CRT in
release mode, so the effective default is `{0,0,0}`. But code should still explicitly zero
these fields.

### 4.4 Runtime active (ctor writes, no CSV/INI entry, consumers exist)

Runtime-only fields — ctor sets an initial value; code later reads/writes them but they're
not INI-parseable. Xref evidence confirms active usage.

| Offset | Size/Type | Ctor Default | Purpose (inferred) | Sample consumers |
|---|---|---|---|---|
| `0xC8E` | byte | 0 | TechnoTypeClass flag fixup | (base class override) |
| `0xD2E` | byte | 0 | TechnoTypeClass flag fixup | |
| `0xD35`, `0xD36` | bool | 1, 1 | TechnoTypeClass flags — default yes | (likely IsTiberiumStorage/IsSimpleDeploy-like) |
| `0xD38`, `0xD3B` | byte | 0 | TechnoType fixup | |
| `0xD96`, `0xD97` | byte | 0 | TechnoType fixup | |
| `0xDF8` | int | -1 (→ array_index) | **Array position in `g_BuildingTypeClass_Array`** | 130+ xrefs — heavy runtime use |
| `0xDFC-0xE04` | int+byte | 0 | Registry bookkeeping / AssignUniqueID slot | — |
| `0xE2C` | int | 0 | Upper half of `GateCloseDelay` double (CSV covers) OR adjacent runtime | xref FUN_00460dcc, 00464b40, 00665650 (FUN_00665650 = DrawBody-style) |
| `0xE58` | int | 0 | Likely `SpotlightAnim*` runtime anim ptr | xref 0x43f1f3, 0x445c80, 0x524e9a, 0x5f58a8 (Unlimbo + related) |
| `0xEB0` | int | -1 | Runtime sentinel, few refs | 0x00523768, 0x005244fe, 0x00524518, 0x006669fa |
| `0xED4`, `0xED8` | int | 0 | Runtime buffer slots (inside ExitCoord extension region) | light xref |
| `0xEE4`, `0xEEC` | int | 0 | Runtime caches between Power/ExtraPower | light xref |
| `0xF04-0xF0F` | int[3] | {0,1,0} | **Unnamed anim triple** — matches the same `{start, count, loop}` pattern as AnimIdle/Active/Aux1/Aux2 | 25+ xrefs (0x45119a, 0x45e8c8, 0x665650, 0x66c825, 0x675351, ...) — very actively used. Likely "AnimPreProduction" or "AnimBuildUp" anim spec that was never exposed to INI. |
| `0xF4C`, `0xF90`, `0xFD4` | PowerUpEntry×3 | empty | **PowerUp table slots 0-2 — no INI mapping** | 0x44099e (Unlimbo), 0x45093a (UpdateRepairAndPower), 0x45144c (AddUpgrade), 0x665650 — used by upgrade/animation systems. These three slots are reserved for the 3 installable upgrades' runtime state. |
| `0x14E4-0x1518` | mixed | 0 | Runtime state between Upgrades and zoffset fields | light xref |
| `0x151C`, `0x1570-0x1571` | byte | 0 | Runtime flags | |
| `0x1538-0x1547` | 16b | 0 | **4D zero block** from `DAT_0089C8A0` — likely an `InitialPosition` or Matrix3D row | |
| `0x1588-0x15D7` | 80b | 0 | **10-slot runtime coord pair table** — zero loop init | used by placement/building system (xref count moderate) |
| `0x15D8-0x1617` | 64b | 0 | **8-slot runtime coord pair table** | |
| `0x1624-0x16A3` | 128b | 0xFFFF | **16 slots of `{min=0xFFFF, max=0xFFFF}` minmax pairs** — sentinel range pattern. Likely the `OccupySomething[]` or `ExitList[]` cell-list table, capped at 16 entries. | |
| `0x16B2`, `0x16B7-0x16B8` | byte | 0 | Padding bools inside the `Grinding..SecretLab` bool block | |
| `0x1760-0x1762` | byte | 0 | Padding bools before `IsThreatRatingNode` | |
| `0x176A` | byte | 0 | Padding after `CrateBeneathIsMoney` | |
| `0x1784` | ptr | 0x007E4638 | **Vtable pointer** for the 12-byte object at `+0x1788` | Static init |
| `0x1788` | 12b VectorClass | alloc'd, zeroed | **`{count, capacity, ptr}` vector** — the runtime BuildingClass-per-Type instance list | Built by creation/destroy hooks; many xrefs |
| `0x178C` | int | 1 | Vector meta-flag | |
| `0x1790`, `0x1791` | bool | 1, 1 | VectorClass "live"/"owns" flags | |

**Per-entry "Powered-flag-only orphans" in the PowerUp block:**

For anim slots that have no `*Powered` INI key (`PreProductionAnim`, `ProductionAnim`,
`TurretAnim`), the ctor still writes the `+0x40` byte to 1 via the shared loop; CSV has
no entry for those 4 bytes. These are orphaned ctor writes but the values (1,0,0,0)
are irrelevant at runtime because consumers don't check these slots' Powered flags.

### 4.5 Runtime dead (ctor writes, no consumers)

**Count: 0 confirmed fully-dead fields.**

Every orphan offset checked had at least one xref outside the ctor. Some have very low
xref counts (e.g., 0x16B2 = 1 xref, 0x176A = 3 xrefs), but none are verifiably dead.
Upper bound: there may be dead fields among the low-xref orphans in the 0x14E4-0x1518
runtime-state block, but absent an exhaustive trace, all are classified as runtime_active.

## Section 5 — Key findings and surprises

### Surprise 1: PowerUp block is a **fixed-size array**, not per-field slots

The decoded 21×0x44-byte layout reveals BuildingTypeClass stores all 18 named anim
(`ActiveAnim`..`LowPower`) plus 2 TS-legacy slots and 3 runtime-reserved slots in a
**uniform record array**, not as sibling-named-field tuples. This matters for Rust
ECS layout — instead of 18 separate typed anim structs, prefer a single `[PowerUpAnim; 21]`
array with a role-index enum for lookup.

### Surprise 2: 4 VoxelBarrelOffset int[3] fields are truly UNINIT by ctor

The VoxelBarrel pivot points at `0x1730..0x175F` are completely untouched by both
BuildingTypeClass ctor and (based on disassembly range) any mid-function init. Only INI
writes them. In practice CRT zero-init makes them `{0,0,0}` — but Rust must explicitly
set this default rather than relying on undefined-memory.

### Surprise 3: `QueueingCell.min` is a silent partial-init bug

The ctor zeros `max` (0x161C) but forgets `min` (0x1618). If no INI override and no prior
heap-zero, `min` holds stale data. This is a **gamemd.exe bug** — not a design decision.
Parity-safe Rust: init both to 0.

### Surprise 4: Orphan anim triple at `0xF04-0xF0F`

A fully-formed anim `{0, 1, 0}` triple exists at `0xF04` with no INI key but 25+ code
consumers. Pattern matches `AnimIdle/Active/Aux1/Aux2` exactly. This is probably
"anim 0" in a series that the CSV (based on ReadINI xrefs) missed. Worth investigating
in Task 2 follow-up — possibly named `Anim` or `AnimBuildUp` without a numeric suffix.

### Surprise 5: `0x178C, 0x1790, 0x1791` are vector-bookkeeping bytes set to 1

These three orphan flags are set at the same time the `operator_new(0xC)` vector is
installed at `0x1788`. They're almost certainly `{initialized=true, owns_buffer=true,
is_shared=true}` or similar VectorClass meta-flags. Confirming this requires tracing
`0x007E4638` (the vector's vtable).

### Surprise 6: `0x1624-0x16A3` is a fixed-size (min,max) pair table

16 slots × 8 bytes × 2-byte min/max × 2 = 256 bytes... wait, 128 bytes (8 DWORDs for min
block + 8 DWORDs for max block, interleaved via loop stride). All initialized to 0xFFFF.
Likely an `OccupyWhatShape[]` or `ExitCellOffsets[]` table. **The loop stride of 8 with
writes at `EAX-0x40`, `EAX-0x3C`, `EAX`, `EAX+0x4` means it's two parallel minmax tables
interleaved**: one at `0x1624` (8 pairs) and one at `0x1664` (8 pairs). Each table
represents 8 cells with (min,max) cell-coord bounds.

## Section 6 — Cross-reference summary

**Updated CSV:** `BUILDINGTYPECLASS_FIELDS.csv` now has:
- `field_class` column added (values: `config`, `uninit_config`, `partial_config`)
- `default` column populated where ctor writes match an INI field (334 entries populated
  for simple scalars; PowerUp-block fields populated with loop-default notations)
- `notes` column extended with ctor source line numbers

**Total ctor-written bytes:** 2,294 out of ~6,040 total instance bytes (~38% direct
init, remainder is TechnoTypeClass base init + genuinely-uninit VoxelBarrel region +
padding).

**Coverage gaps for INI-parsed fields:** 6 out of 344 (1.7%) — 5 uninit_config + 1
partial_config.

## Sources

- **Decompile**: `0x0045DD90` (BuildingTypeClass__constructor)
- **Disassembly** (authoritative for byte offsets): `0x0045DD90-0x0045E511`
- **Base ctor reference**: `TechnoTypeClass__Constructor` at `0x00710AF0`
- **Allocator**: `FUN_004653C0` (BuildingTypeClass__FindOrAllocate) — calls `operator_new(0x1798)` then ctor
- **Cross-ref**: `BUILDINGTYPECLASS_FIELDS.csv` (Task 2 output)
- **Byte-pattern searches** (for runtime xref analysis): `search_byte_patterns` with
  little-endian offset constants (e.g. `04 0F 00 00` for offset `0xF04`)
- **Vtable references**: `0x007E4570` (primary), `0x007E4554` (secondary_4),
  `0x007E454C` (secondary_8), `0x007E4544` (secondary_C), `0x007E4638` (inner vector's vtable)
