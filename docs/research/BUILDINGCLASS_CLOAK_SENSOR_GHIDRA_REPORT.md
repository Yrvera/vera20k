# BuildingClass Cloak Generator, Sensor Array, and Building Cloaking -- Ghidra Report

**Source:** Live Ghidra decompilation of `gamemd.exe`, focused on
`BuildingClass::UpdateGapAndSpecialEffects` (0x004549B0),
`BuildingClass::UpdateGapGenerator_Tick` (0x00454DB0), the four vtable slots for
sensor/disguise cell-counter management, and `TechnoClass::{Update,Remove}CloakShroud`.

**Confidence:** HIGH for all listed offsets, function addresses, and state machine
logic (verified directly from decompiled binary). HIGH for the radius-source
discrepancy (examined assembly and confirmed the two functions read different fields).

**YR retail usage:** Neither `CloakGenerator=yes` nor `Cloakable=yes` is used on any
retail YR building. These code paths are TS-legacy features retained in the binary
but dormant in a standard YR skirmish. `SensorArray=yes` IS used (Psychic Sensor,
Spy Satellite Uplink) and is live. `DetectDisguise=` is used (same buildings +
infantry/vehicles). See "YR-active summary" at the end.

Cross-reference: `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md` covers the global
TechnoClass cloak state machine and unit-level SensorsSight. This report covers the
building-specific subsystems and tick logic in depth.

---

## Table of Contents

1. [Key INI keys and field offsets](#1-key-ini-keys-and-field-offsets)
2. [BuildingClass instance field map (cloak/sensor subset)](#2-buildingclass-instance-field-map)
3. [UpdateGapAndSpecialEffects (power-toggle entry point)](#3-updategapandspecialeffects-0x004549b0)
4. [UpdateGapGenerator_Tick (per-tick radius animation)](#4-updategapgenerator_tick-0x00454db0)
5. [Cell counter add/remove vtable methods](#5-sensor-and-disguise-cell-counter-methods)
6. [The radius discrepancy (real gamemd bug)](#6-the-sensor-radius-discrepancy--real-bug-in-gamemdexe)
7. [Building cloaking itself (CloakGenerator vs Cloakable)](#7-building-self-cloaking-vs-cloak-field-generator)
8. [Power interaction -- full flow](#8-power-interaction-full-flow)
9. [Overlapping cloak/sensor fields -- reference counting](#9-overlapping-fields-reference-counting)
10. [BuildingClass vtable map (cloak/sensor entries)](#10-buildingclass-vtable-map)
11. [Rust implementation notes](#11-rust-implementation-notes)

---

## 1. Key INI keys and field offsets

All parsed in `BuildingTypeClass::ReadINI` (part of the function at 0x00460C00,
section around 0x00460C0C-0x00460C39).

| INI Key | BuildingTypeClass Offset | Type | Ghidra line | Default |
|---------|--------------------------|------|-------------|---------|
| `CloakGenerator=` | +0x16C7 | bool (byte) | `CCINIClass__ReadBool(..., s_CloakGenerator_0081a998, ...)` | previous byte value (0 at construction) |
| `SensorArray=` | +0x16C8 | bool (byte) | `CCINIClass__ReadBool(..., s_SensorArray_0081a98c, ...)` | 0 |
| `CloakRadiusInCells=` | +0x1707 | byte (read as int, stored as byte) | `CCINIClass__ReadInt(..., s_CloakRadiusInCells_0081a978, (int)*(char *)(param_1 + 0x1707))` | 0 (**NOT 20** -- see below) |
| `PsychicDetectionRadius=` | +0x170C | int | `CCINIClass__ReadInt(..., s_PsychicDetectionRadius_0081a960, ...)` | 0 |

Inherited from TechnoTypeClass (apply to BuildingTypeClass too):

| INI Key | TechnoTypeClass Offset | Type | Use |
|---------|------------------------|------|-----|
| `SensorsSight=` | +0x5F0 | int | Range used by `AddSensorArrayAt` and unit-level sensor detection |
| `DetectDisguise=` | +0xD31 | bool | Enables disguise detection (cell counter at CellClass+0xAC) |
| `DetectDisguiseRange=` | +0x5F4 | int | Range for both Add and Remove disguise detection |
| `Cloakable=` | +0xCD0 | bool | Unit can cloak itself (used as BuildingClass innate cloak flag too) |
| `GapGenerator=` | +0xCD1 | bool | Object projects a gap shroud (re-shrouds cells for enemies) |
| `GapRadiusInCells=` | +0xCD2 | byte | Gap shroud radius, used by `TechnoClass::{Update,Remove}CloakShroud` |

### Correction of the task prompt's "default 20 cells"

The task prompt claimed the default for `CloakRadiusInCells` is 20. That is **not**
backed by the binary. `CCINIClass::ReadInt` is called with `(int)*(char *)(param_1 + 0x1707)`
as the fallback -- i.e. the current value before this read, which is zero from the
TechnoTypeClass/BuildingTypeClass constructor. Retail `rulesmd.ini` does not define
`CloakRadiusInCells` anywhere (no building has `CloakGenerator=yes`), so the effective
default is 0. If this were used, the "20 default" claim would need a separate hardcoded
initialization path which does not appear to exist.

---

## 2. BuildingClass instance field map

Fields relevant to cloak field and sensor visualization:

| Offset | Size | Meaning | Notes |
|--------|------|---------|-------|
| +0x80 | 1 | Redraw dirty flag | Set to 1 when cloak stage changes so sidebar/map redraws |
| +0x220 | 4 | CloakState (TechnoClass inherited) | 0=Uncloaked, 1=Cloaking, 2=Cloaked, 3=Uncloaking. Also used for building self-cloaking via `Cloakable=yes`. |
| +0x269 | 1 | CloakShroudActive | Set by `UpdateCloakShroud`, cleared by `RemoveCloakShroud`. Prevents double-apply. |
| +0x26C | 4 | CloakShroudRadius cached | Copied from `TechnoTypeClass+0xCD2` (GapRadiusInCells). Only cached on first call. |
| +0x6EB | 1 | CloakFieldDirection | signed char: `1`=expanding, `-1` (0xFF)=contracting, `0`=idle |
| +0x6EC | 1 | CloakFieldCurrentRadius | byte: current expanded radius step (0 .. `BuildingTypeClass+0x1707`) |
| +0x6ED | 1 | BuildingCloakStage | byte: 0-15 visual animation stage; 16 = fully cloaked + invisible |
| +0x660 | 1 | (animation direction, per cloak report) | Set by StartCloaking/StartUncloaking wrappers |
| +0x662 | 1 | RobotTanksOnline flag | Set/cleared by `RobotTanksBackOnline` / `RobotTanksOffline` in the same power-toggle function |
| +0x268 | 1 | (power-related latch) | Cleared during unpowered + gap-generator shutdown to force cloak shroud recalc |

Field offset 0x6EC in int-stride decomp: `param_1[0x1BB]` with `(char)` cast
reads byte at `0x1BB * 4 = 0x6EC`. Verified against the raw assembly of
`UpdateGapGenerator_Tick`.

---

## 3. UpdateGapAndSpecialEffects (0x004549B0)

**Signature:** `void __fastcall(BuildingClass *this)`

**Called from:**
- `BuildingClass::Update` (0x0043fbea) -- main per-tick AI (path that runs when power state changes or when poweredness needs re-evaluation)
- `FUN_006e0b60` (0x006e0c7a) -- house-level power refresh (called on power plant destruction, etc.)

**Branch:** The entire function splits on `vtable+0x350` which returns true when the
building is "fully operational" (has power, not EMP-locked, has health, and, if
`Powered=yes`, house has enough power ratio). Ghidra labels this slot
`BuildingClass__CanSellOrUndeploy` but the decompilation body shows it is the
general "is this building currently active" predicate.

### Powered branch (`vtable+0x350` returned true)

Sequential blocks, in order:

1. **Robot tanks online:** If `BuildingTypeClass+0x40C != 0` (a RobotTankType pointer) and `this+0x662 == 0`, set `this+0x662 = 1` and call `HouseClass::RobotTanksBackOnline`.

2. **CloakGenerator field expansion trigger:** If `BuildingTypeClass+0x16C7` (CloakGenerator) AND `this+0x6EB <= 0` (not currently expanding) AND `this+0x6EC != BuildingTypeClass+0x1707` (current radius != target):
   - Set `this+0x6EB = 1` (start expanding)
   - If `this+0x6EC == BuildingTypeClass+0x1707` (edge case), zero `this+0x6EC`
   - Mark dirty `this+0x80 = 1`

3. **GapGenerator (techno-level) shroud apply:** If `BuildingTypeClass+0xCD1` (GapGenerator) AND `this+0x269 == 0` (not already shroud-active), call `vtable+0x414 = TechnoClass::UpdateCloakShroud`. This is the cell-level shroud re-apply.

4. **Overpowerable bonus / PoweredSpecial:** If `Type+0x1573 (Powered)` AND `Type+0xEE4 > 0`, call `BuildingClass::OnPowerOff` (sic -- see note).
   - Note: This call to `OnPowerOff` inside the powered branch looks reversed; this is likely an internal dispatcher and the name is misleading. It corresponds to "re-apply overpower effect after power restoration". Not directly cloak-related.

5. **PoweredSpecial animation slots:** If `Type+0x1574` (PoweredSpecial), iterate `Type+0xF8F` animation slot flags and recreate animations via `BuildingClass::CreateAnimForSlot`. Not cloak-related.

### Unpowered branch (`vtable+0x350` returned false)

1. **Robot tanks offline:** Symmetric to powered case.

2. **Capture manager free:** If `this+0x2BC != 0` (has CaptureManager), call `CaptureManagerClass::FreeAll`. Drops all mind control from this building.

3. **Chrono warp cancel:** If `this+0x2AC != 0` (ChronoSphere/ChronoWarp state), call `BuildingClass::DeployUnit_ChronoWarp(1)`.

4. **CloakGenerator field contraction trigger:** If `BuildingTypeClass+0x16C7` (CloakGenerator) AND `(signed char)this+0x6EB >= 0` (not already contracting) AND `this+0x6EC != 0`:
   - Set `this+0x6EB = 0xFF` (-1, start contracting)
   - Mark dirty `this+0x80 = 1`

5. **GapGenerator shroud remove:** If `BuildingTypeClass+0xCD1` AND `this+0x269 != 0`:
   - Call `vtable+0x418 = TechnoClass::RemoveCloakShroud`
   - If `this+0x268 != 0`, clear it, clear `HasExtraPowerDrain`, mark `Owner+0x5778 = 1` (house-dirty), reset `this+0x26C = GapRadiusInCells` (recache radius), call `vtable+0x414 = UpdateCloakShroud` again to reapply (unclear why -- perhaps for shroud transition animation; confirmed from disasm).

6. **Power-off animation recreation:** Same as powered branch, different animation slot offset (+0x1458 / +0x1468 for ConditionYellow).

### Net cloak/sensor effect of this function

| Event | Consequence for cloak field | Consequence for sensor array |
|-------|------------------------------|------------------------------|
| Power restored | `+0x6EB = 1` (start expanding) if `CloakGenerator=yes` | (No direct action here -- sensor array is re-applied when `AddSensorArrayAt` is invoked elsewhere, e.g. on construction complete) |
| Power lost | `+0x6EB = -1` (start contracting) if `CloakGenerator=yes` | (Sensor array is removed when the building is destroyed/sold/captured, or when the house's power state changes via the callers of vtable+0x4F8) |

**Important:** Sensor array add/remove is **NOT** toggled by this function directly.
Sensor array cell coverage persists as long as the building exists. The
`AddSensorArrayAt` itself (see section 5) internally gates on `vtable+0x350`, so if
called while unpowered the call is a no-op. The remove function does NOT gate on
power -- it always decrements. This asymmetry matters for the radius bug below.

---

## 4. UpdateGapGenerator_Tick (0x00454DB0)

**Signature:** `void __thiscall(BuildingClass *this)` (prototype Ghidra inferred as `int *param_1`)

**Called from:** `BuildingClass` per-tick update chain (enters via vtable slot `+0x410` from `BuildingClass::Update`).

**Guard:** Early return if `this+0x520 == 0` (Type pointer is null / building placeholder).

This function has two distinct responsibilities running in sequence every tick:

### Part A -- Building self-cloak visual animation (driven by CloakState at `+0x220`)

Runs only when BuildingCloakStage (`+0x6ED`) needs to advance:

- **CloakState == 1 (Cloaking) && BuildingCloakStage < 15:**
  - `BuildingCloakStage++`
  - At stages 1, 6, 11 set dirty flag (`+0x80 = 1`)
  - At stage 15: if `GetVisualState(0,0) == 5` (fully invisible), bump to 16
  - Iterate 21 animation slots at `this+0x55C` (int array), write `BuildingCloakStage` to each anim's `+0x178` field
  - If BuildingCloakStage hits 15: set `CloakState = 2` (Cloaked), destroy attached ParticleSystem at `this+0x30C` (int-stride `this[0xc3]`)

- **CloakState == 3 (Uncloaking) && BuildingCloakStage >= 0:**
  - If stage > 0: `BuildingCloakStage--`
  - At stages 0, 5, 10 set dirty flag
  - (Same "stage 15 + vs == 5 -> 16" idiom, but in uncloaking direction -- used for visual consistency)
  - Iterate the same 21 animation slots, write BuildingCloakStage to each
  - If BuildingCloakStage hits 0 AND no particle system attached AND type has CloakAttachOffset (at BuildingTypeClass+0x768/0x76c/0x770 -- a 3D coord, compared against `DAT_0089c848`): create a new ParticleSystem from `BuildingTypeClass+0x764` (ParticleSystemType pointer) at world coord = building pos + attach offset. Store in `this[0xc3]`.
  - Then set `CloakState = 0` (Uncloaked)

- **CloakState == 2 (Cloaked):** Call `vtable+0x45C = StartUncloaking(0)` if `vtable+0x2A4 = ShouldUncloak()` returns true (this is the `BuildingClass::ShouldUncloak` override that also checks for nearby enemy `Sensors=yes` units across the foundation).

- **CloakState == 0 (Uncloaked):** Call `vtable+0x460 = StartCloaking(0)` if `vtable+0x2A0 = CanAutoCloak()` returns true (this is the `BuildingClass::CanCloak` override that ALSO checks foundation for enemy `Sensors=yes` units).

This is the TechnoClass-derived state machine, plus foundation-wide enemy sensor
check. The `BuildingClass::CanCloak` (0x00457770) / `BuildingClass::ShouldUncloak`
(0x004578C0) overrides walk the building's foundation footprint (width/height from
`BuildingTypeClass::GetFoundationWidth/Height`) and call
`CellClass::Find_Nearest_Object` to locate any enemy object with
`TechnoTypeClass+0xC9D (Sensors=yes)`; if one exists, cloaking is prevented /
uncloaking is forced.

### Part B -- Cloak FIELD radius expansion/contraction (driven by +0x6EB / +0x6EC)

Runs when `this+0x6EB != 0` AND `Type+0x16C7 (CloakGenerator)`.

The expanding side iterates over a square of cells at the building's foundation
center using `MapClass::Get_CellClass` + a radius-mask from the sprite system
(`DAT_0089ddc0` which is the `TheMap` / `MapClass`; its vtable slot `+0x5C` returns
a circular bitmap mask for a given radius). For each cell in the mask:

- **+0x6EB == 1 (expanding path, reached when signed char > 0):**
  - If `this+0x6EC == Type[0x1707]`: set `this+0x6EB = 0` (done expanding), return
  - Otherwise `this+0x6EC++`, get radius mask for new radius
  - For each cell in the mask that is visible to the player:
    - Call `FUN_00487110(this+0x30 = Owner house index)` which is a sibling of `CellClass::IncrementSensorCount`/`IncrementDisguiseDetectCount` -- it actually applies the CLOAK field to the cell (see below)
    - For each object in the cell with RTTI = Unit (1) / Aircraft (15) / Infantry (2), call `vtable+0x420 = TechnoClass::DoUncloak` ... wait, the code does `ProcessCloakMode` equivalent. Actually this hits the SAME pattern as `AddSensorArrayAt`. The `FUN_00487130` is the *cloak apply* sibling; see "Cell counter family" below.
  - If a building exists at the cell that belongs to the same owner AND matches the expansion center coord -- skip/return (covers a reentrancy edge case)

- **+0x6EB == -1 (contracting path):**
  - If `this+0x6EC == 0`: set `+0x6EB = 0`, return
  - Otherwise `this+0x6EC--`, get mask for previous radius
  - For each cell in (previous_radius mask - new_radius mask), call `FUN_00487130(house_idx)` which is the *cloak remove* sibling
  - For each removed cell: iterate all units/aircraft/infantry in the cell, call `vtable+0x420 = DoUncloak` on them (to force re-check of visibility since their cell-cloak coverage just ended)
  - If `+0x6EC == 0` at the end, walk all buildings in `g_BuildingClass_Array`: if another CloakGenerator building is powered AND has `+0x6EB == 0` AND is within `Type[0x1707]+2` cells (with 2x slop factor, `<` comparison against `(radius+2)^2 * 4`), set its `+0x6EB = 1` to restart expansion. This **handshake/chain** behavior ensures that if two cloak generators are adjacent and one goes offline, the other's field still covers what it can -- a "hand off" mechanic.

**Bottom line:** `UpdateGapGenerator_Tick` handles TWO independent concerns:
1. Building self-cloak visual animation (if Cloakable=yes applied to a building -- unused in YR)
2. CloakGenerator field radius animation (expanding/contracting the cloak coverage over time one cell radius per tick)

Note the animation is **one cell per tick** (not instantaneous). At 15 fps, a
`CloakRadiusInCells=10` field takes 10 ticks ≈ 0.67 seconds to fully expand.

---

## 5. Sensor and disguise cell-counter methods

These are all the `BuildingClass` vtable overrides for cell-level counter management.

### BuildingClass vtable base

The real main `BuildingClass` vtable is at **`0x007e3EBC`**. (Note: Ghidra naming
for some slots is off; the authoritative mapping below is taken from data xrefs
to the actual implementation functions.)

### Vtable slot to function

| Vtable offset | Function addr | Name | Range source | Power-gated? |
|---------------|---------------|------|--------------|--------------|
| +0x4F4 | 0x00455820 | `BuildingClass::AddSensorArrayAt` | `TechnoTypeClass+0x5F0` (SensorsSight, int) | **YES** (early return if `vtable+0x350` = 0) |
| +0x4F8 | 0x004556D0 | `BuildingClass::RemoveSensorArrayAt` | `BuildingTypeClass+0x1707` (CloakRadiusInCells, byte) | **NO** |
| +0x4FC | 0x00455A80 | `BuildingClass::AddDetectDisguiseAt` | `TechnoTypeClass+0x5F4` (DetectDisguiseRange, int) | **YES** |
| +0x500 | 0x00455980 | `BuildingClass::RemoveDetectDisguiseAt` | `TechnoTypeClass+0x5F4` (DetectDisguiseRange, int) | **NO** |

### Cell-level counter storage

| Cell offset | Width | Purpose | Inc helper | Dec helper |
|-------------|-------|---------|------------|------------|
| +0x7C | short\[MaxHouses] | SensorCount per house | `CellClass::IncrementSensorCount` (0x00487150) | `CellClass::DecrementSensorCount` (0x00487160) |
| +0xAC | short\[MaxHouses] | DisguiseDetectCount per house | `CellClass::IncrementDisguiseDetectCount` | `CellClass::DecrementDisguiseDetectCount` |

A cell with `SensorCount[h] > 0` makes all cloaked enemies of house `h` visible.
Check: `CellClass::SensorCountForHouse` (0x004870D0) = `0 < SensorCount[h]`.

### Algorithm

Both `Add*` and `Remove*` functions use the identical iteration pattern:

```
r = range_source               // either SensorsSight or CloakRadiusInCells
h = this.Owner.ArrayIndex       // this+0x87 dereferenced +0x30
center = this.GetCoords cell    // from vtable+0x48
for dy in -r..r:
    for dx in -r..r:
        if dx*dx + dy*dy < r*r:            // circular, strict-less
            cell = MapClass::Get_CellClass(center + (dx, dy))
            cell.SensorCount[h] += delta    // +1 for Add, -1 for Remove
            // For Add/Remove SensorArray (NOT DisguiseDetect):
            //   for each object in cell linked-list at cell+0xE4:
            //     if object.RTTI in (Unit=1, Aircraft=15, Infantry=2):
            //       object.DoUncloak()     // vtable+0x420 on TechnoClass
```

`AddDetectDisguiseAt` / `RemoveDetectDisguiseAt` do NOT iterate cell objects --
they only update the counter. Disguise detection is passive (queried on render
/ targeting).

`AddSensorArrayAt` / `RemoveSensorArrayAt` DO iterate cell objects and force
`DoUncloak` on all units/aircraft/infantry. This is how sensor arrays cause
cloaked units to become visible at the moment coverage starts.

### Called from

- `AddSensorArrayAt` (+0x4F4): `BuildingClass::OnConstructionComplete` (0x004467A1) and other construction/unlimbo paths
- `RemoveSensorArrayAt` (+0x4F8): House power-off / building-removal paths (e.g. 0x005B0D1C inside what appears to be a house-level sensor sweep reset)
- `AddDetectDisguiseAt` / `RemoveDetectDisguiseAt`: Construction/destruction and power toggles on buildings with `DetectDisguise=yes`

---

## 6. The sensor radius discrepancy -- real bug in gamemd.exe

The task question asked whether AddSensorArrayAt using `Type+0x5F0` and
RemoveSensorArrayAt using `Type+0x1707` is intentional. **Verified: this is a
genuine gamemd.exe bug.** Disassembly of both functions confirmed:

### AddSensorArrayAt (0x00455820)

```c
iVar1 = *(int *)(param_1[0x148] + 0x5f0);   // SensorsSight, int
...
if (iVar6 * iVar6 + iVar5 * iVar5 < iVar1 * iVar1) { ... }
```

### RemoveSensorArrayAt (0x004556D0)

```c
iVar2 = (int)*(char *)(param_1[0x148] + 0x1707);  // CloakRadiusInCells, byte
...
if (iVar5 * iVar5 + iVar4 * iVar4 < iVar3) { ... }  // iVar3 = iVar2*iVar2
```

### Why it is a bug

1. **Different INI fields.** `SensorsSight` (TechnoTypeClass `ReadInt`) and
   `CloakRadiusInCells` (BuildingTypeClass `ReadInt` truncated to byte) are read
   at different code paths in `ReadINI` and have distinct purposes. They are
   never auto-synchronized.
2. **Unit-level siblings are symmetric.** `TechnoClass::AddSensorsAt` (0x004DE7B0)
   and `TechnoClass::RemoveSensorsAt` (0x004DE940) both read `Type+0x5F0`. Only
   the BuildingClass override deviates.
3. **Semantic breakage.** In retail YR, Psychic Sensor sets `SensorsSight=15` but
   does NOT set `CloakRadiusInCells`. So `Type+0x1707` is 0. `RemoveSensorArrayAt`
   therefore iterates a 0-cell circle (loop body never executes because
   `dx*dx + dy*dy < 0` is impossible). Net effect: **sensor counts added on
   construction are never decremented on removal.** The counter leak is silent
   because sensor arrays rarely come down in practice (destroying the Psychic
   Sensor or capturing it are the only triggers).
4. **Comparison with DisguiseDetect symmetry.** Both `AddDetectDisguiseAt` and
   `RemoveDetectDisguiseAt` consistently use `Type+0x5F4`. The bug is isolated
   to the SensorArray remove.

### Consequence in retail YR

Because no retail YR building has `CloakRadiusInCells` set AND the sensor array
is rarely removed (Psychic Sensor destruction is uncommon), this bug is mostly
cosmetic -- the sensor coverage "ghost-persists" on cells where a destroyed sensor
array used to be. However: if a mod sets both keys, the radii may differ and
remove would under-clean. If a mod sets only `SensorsSight` without
`CloakRadiusInCells` (like retail YR), remove is a full no-op.

### Rust implementation

Do NOT replicate the bug. Use `SensorsSight` for both add and remove.
**Recommendation:** add an assertion or debug log if the two radii differ to
catch mod configurations that rely on the buggy asymmetry.

---

## 7. Building self-cloaking vs cloak-field generator

These are two distinct systems that happen to share BuildingClass state fields.

### System A -- Building-as-a-cloaked-object (`Cloakable=yes`)

Driven by `TechnoTypeClass+0xCD0` copied through the normal `CloakState` state
machine at `TechnoClass+0x220`. Transitions are driven by `TechnoClass::CloakingTick`
(0x006FB740), but BuildingClass overrides `CanAutoCloak` and `ShouldUncloak` to
also consult the foundation for nearby enemy `Sensors=yes` units.

**Key overrides:**
- `BuildingClass::CanCloak` (0x00457770, vtable+0x2A0): calls base `CanAutoCloak` first; if true, walks the building's foundation footprint (-1 .. width/height) calling `CellClass::Find_Nearest_Object`; returns 0 if any enemy has `TechnoTypeClass+0xC9D (Sensors=yes)`.
- `BuildingClass::ShouldUncloak` (0x004578C0, vtable+0x2A4): calls base `ShouldUncloak`; returns 1 if true or if any enemy with `Sensors=yes` exists in the foundation.
- `BuildingClass::GetVisualState` (0x004544A0, vtable+0x68): see dedicated section in `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md`. Uses `BuildingCloakStage` (+0x6ED) as the visual index, NOT `CloakProgress`.

**Not overridden:** `StartCloaking` / `StartUncloaking`. Buildings reuse the
TechnoClass implementations unchanged; the visual animation is driven purely by
`UpdateGapGenerator_Tick` reading `CloakState` and updating `BuildingCloakStage`.

**Retail YR usage:** No retail YR building has `Cloakable=yes`. This code path
is dormant.

### System B -- Building-as-a-cloak-field-projector (`CloakGenerator=yes`)

A completely separate system from above. Driven by the trio of `BuildingClass`
fields `+0x6EB` (direction), `+0x6EC` (current radius), `+0x6ED` (visual stage)
plus `+0x269` (shroud-applied latch) and `+0x26C` (cached gap radius).

`CloakGenerator` tells the building to **project a cloak field** that:
1. Re-shrouds cells for enemies (via `TechnoClass::UpdateCloakShroud` -- the
   TechnoClass gap generator machinery, activated here through
   `Type+0xCD1 (GapGenerator=yes)` OR directly through the `+0x6EB` cycle)
2. Grows the field radius one cell per tick up to `CloakRadiusInCells`
3. Shrinks symmetrically when power is lost

Units inside the field are NOT individually cloaked -- they are simply hidden
under re-shrouded cells. `DoUncloak()` is called on units when the field
**shrinks past them** (triggers their visibility re-check). Units inside the
steady-state field remain in their normal CloakState.

**Retail YR usage:** No retail YR building has `CloakGenerator=yes`. This is a
TS-era feature (the TS Mobile Stealth Generator). Dormant in YR.

### Does a building ever call `StartCloaking` on units inside its field?

**No.** Neither `UpdateGapGenerator_Tick` nor the CloakGenerator path calls
`StartCloaking` on the units covered. The cloak field is purely a cell-level
shroud effect; unit-level cloak state is untouched.

---

## 8. Power interaction -- full flow

### Summary table

| Event | Method called | Consequence |
|-------|---------------|-------------|
| Building construction completes | `BuildingClass::OnConstructionComplete` | Calls `AddSensorArrayAt` / `AddDetectDisguiseAt` (gated on `vtable+0x350`). Does NOT initiate CloakGenerator expansion directly -- waits for tick. |
| Power restored to building | `UpdateGapAndSpecialEffects` (powered branch) | Sets `+0x6EB = 1` to start cloak field expansion. Calls `UpdateCloakShroud` if `GapGenerator=yes`. |
| Power lost from building | `UpdateGapAndSpecialEffects` (unpowered branch) | Sets `+0x6EB = -1` to start cloak field contraction. Calls `RemoveCloakShroud` if `GapGenerator=yes` and shroud was active. |
| Per-tick update | `UpdateGapGenerator_Tick` | Steps `+0x6EC` by 1 toward target (`Type+0x1707` or 0) each tick; applies/removes cloak cells via cell-sibling helpers; calls `DoUncloak` on units in removed cells. |
| Building sold / destroyed / captured | `BuildingClass::Limbo` / change-owner | Calls `RemoveSensorArrayAt` / `RemoveDetectDisguiseAt`. **See bug in section 6.** |
| House-wide power recompute (e.g. power plant destroyed) | `FUN_006e0b60` iterates buildings | Calls `UpdateGapAndSpecialEffects` on each building. |

### Gotchas

- **Sensor array does NOT toggle on power loss.** Examining the
  `UpdateGapAndSpecialEffects` code paths shows no direct call to add/remove
  sensor array when power flips. Sensor arrays are added on construction
  completion and removed only when the building leaves the map (or via the
  house-level sensor reset at 0x005B01C0 on global events). **Confirm this is
  the intent of gamemd:** a Psychic Sensor that loses power KEEPS its cell
  coverage.
  - **Counter-evidence:** `AddSensorArrayAt` itself gates on `vtable+0x350`
    (power check) and returns early if unpowered. So if a sensor array is somehow
    placed while unpowered, no coverage is applied. But it is never actively
    torn down on power loss in the `UpdateGapAndSpecialEffects` path.
  - **Implication:** In practice the game must be calling sensor remove somewhere
    else on power loss, OR the sensor array is intended to persist through
    power loss. Track xrefs to `vtable+0x4F8` to map the full remove-triggering
    set. At minimum we see 0x005B01C0 (possibly tied to spysat / radar-style
    events) and any `Limbo` / `ChangeOwner` flows.

- **CloakGenerator field expansion is reentrant-safe.** The `+0x6EB` state field
  prevents double-starting. Each tick it steps by one cell, so transitions are
  smooth.

- **The "cloak generator handshake" at contraction end** (section 4 Part B):
  when a CloakGenerator fully contracts (`+0x6EC` hits 0), it scans all other
  CloakGenerator buildings on the map and if any are nearby (within
  `Type[0x1707]+2` cells with 2x slop radius) AND powered AND idle (`+0x6EB ==
  0`), it signals them to re-expand. This allows adjacent cloak generators to
  cover the void left behind without player intervention. In retail YR this
  never triggers since no buildings have CloakGenerator.

---

## 9. Overlapping fields: reference counting

### Sensor and disguise cell counters

Both `SensorCount` (+0x7C) and `DisguiseDetectCount` (+0xAC) are `short[MaxHouses]`
arrays. Each `Add*` call increments; each `Remove*` call decrements. **Overlapping
sensor arrays DO accumulate.** A cell covered by two `SensorArray=yes` buildings
owned by the same house has `SensorCount[h] = 2`. When one is destroyed, the
counter drops to 1 and the cell still reveals cloaked units.

Visibility check is `> 0`, so any non-zero count means "detected". The counter
width is `short` (signed 16-bit); in practice won't overflow even with many
overlapping sensors since the range limit keeps coverage small.

### Cloak shroud cells (gap generator)

`CellClass+0x134` = `GapOverlayCount` (int). Incremented by `UpdateCloakShroud`,
decremented by `RemoveCloakShroud`. **Overlapping gap generators accumulate:**
a cell covered by two gap generators has GapOverlayCount=2.

Only when `GapOverlayCount` drops to 0 does the secondary counter `+0x130`
(`GapShroudLevel`) decrement. And only when `GapShroudLevel` drops to 0 do the
cell's ShroudFlags (+0x12C) get bit `0x08` (mark-for-reveal) and `0x10`
(mark-for-redraw) set. This double-counter structure prevents premature reveal
when multiple generators cover the same cell and one is removed.

**Allied exclusion** (`+0x13C`) counts separately -- allied players always see
through their own teammates' gap generators.

### Answer to the task prompt's question about two Yuri Cloning Vats overlapping

Cloning Vat (`YACOW` building) does NOT have `CloakGenerator=yes` or
`SensorArray=yes`. The hypothetical applies to any two `CloakGenerator=yes`
buildings: yes, reference counting works correctly. If either is destroyed
first, cells under both remain shrouded; only after both are removed do cells
reveal.

---

## 10. BuildingClass vtable map

Relevant slots only. Vtable base: **0x007E3EBC**.

| Offset | Address | Name | Override of |
|--------|---------|------|-------------|
| +0x068 | 0x004544A0 | `BuildingClass::GetVisualState` | TechnoClass |
| +0x2A0 | 0x00457770 | `BuildingClass::CanCloak` | `TechnoClass::CanAutoCloak` |
| +0x2A4 | 0x004578C0 | `BuildingClass::ShouldUncloak` | TechnoClass |
| +0x350 | 0x004555D0 | `BuildingClass::IsActive` (Ghidra labeled `CanSellOrUndeploy`) | (not a cloak method but referenced in every gate) |
| +0x410 | 0x00454DB0 | `BuildingClass::UpdateGapGenerator_Tick` | (BuildingClass-specific, no parent) |
| +0x414 | 0x006FB170 | `TechnoClass::UpdateCloakShroud` | inherited |
| +0x418 | 0x006FB470 | `TechnoClass::RemoveCloakShroud` | inherited |
| +0x420 | 0x006F4EB0 | `TechnoClass::DoUncloak` | inherited |
| +0x45C | 0x007036C0 | `TechnoClass::StartUncloaking` | inherited |
| +0x460 | 0x00703770 | `TechnoClass::StartCloaking` | inherited |
| +0x4F4 | 0x00455820 | `BuildingClass::AddSensorArrayAt` | TechnoClass::AddSensorsAt |
| +0x4F8 | 0x004556D0 | `BuildingClass::RemoveSensorArrayAt` | TechnoClass::RemoveSensorsAt (**radius bug**) |
| +0x4FC | 0x00455A80 | `BuildingClass::AddDetectDisguiseAt` | TechnoClass |
| +0x500 | 0x00455980 | `BuildingClass::RemoveDetectDisguiseAt` | TechnoClass |

Note: The task prompt's "vtable+0x414 = UpdateCloakShroud" is correct for the
main BuildingClass vtable (base 0x007E3EBC). A common confusion is the
neighboring data table at 0x007E4100 which contains a subset of the same
function pointers but with different offsets and different base -- that is a
different (possibly secondary / partial) vtable and should not be used for
this work.

---

## 11. Rust implementation notes

### Priorities

In order of retail YR relevance:

1. **SensorArray + SensorsSight + DetectDisguise** -- ACTIVE. Psychic Sensor (`[PSYC]`),
   Spy Satellite Uplink (`[SPYSAT]` / `[NACLON]`), Allied Radar need cell counter
   add/remove on construction/destruction/owner-change.
2. **GapGenerator + GapRadiusInCells** -- ACTIVE (Gap Generator building
   `[GAGAP]` or similar). Uses the `TechnoClass::UpdateCloakShroud` path (not
   the CloakGenerator path).
3. **CloakGenerator + CloakRadiusInCells** -- DORMANT in YR. Implement only for
   mod support, with a comment noting the radius-expansion one-cell-per-tick
   behavior and the `+0x6EB` direction-of-change state machine.
4. **Cloakable on buildings** -- DORMANT in YR. Skip unless mod support is
   needed.

### State machine summary for CloakGenerator field

```rust
enum CloakFieldPhase {
    Idle,          // field_0x6EB == 0
    Expanding,     // field_0x6EB == 1
    Contracting,   // field_0x6EB == -1
}

struct CloakFieldState {
    phase: CloakFieldPhase,
    current_radius: u8,     // field_0x6EC, 0..=CloakRadiusInCells
    stage: u8,              // field_0x6ED, 0..=16 (visual)
    shroud_active: bool,    // field_0x269
}

// Per tick (gated on type.cloak_generator):
match phase {
    Idle => {
        if powered && !shroud_active { phase = Expanding; }
        else if !powered && shroud_active { phase = Contracting; }
    }
    Expanding => {
        if current_radius == type.cloak_radius_cells { phase = Idle; }
        else {
            current_radius += 1;
            apply_cloak_cells_ring(current_radius);  // only the newly-added ring
        }
    }
    Contracting => {
        if current_radius == 0 {
            phase = Idle;
            try_handshake_neighbors();  // per gamemd
        }
        else {
            remove_cloak_cells_ring(current_radius);
            for object in cells_in_ring(current_radius).units() {
                object.do_uncloak();  // force visibility recheck
            }
            current_radius -= 1;
        }
    }
}
```

### Gotchas

- **Radius bug in RemoveSensorArrayAt.** Use `SensorsSight` for both Add and
  Remove in the Rust implementation. Do NOT mirror the gamemd bug.
- **One-cell-per-tick.** The cloak field is NOT applied atomically. It grows
  and shrinks over many ticks. This matters for replay/lockstep determinism
  and for visual timing.
- **Counter types.** SensorCount / DisguiseDetectCount are `short` per house.
  GapOverlayCount / GapShroudLevel / AlliedGapExclusion are `int` per cell
  (NOT per house -- the shroud is global-for-enemies, with a single
  allied-exclusion counter).
- **Visibility check is `> 0`.** Any positive counter means detected/shrouded.
- **Power gate.** `AddSensorArrayAt` / `AddDetectDisguiseAt` internally gate on
  `IsActive`. `Remove*` do NOT gate. Match this in Rust to avoid diverging.
- **Do not call `StartCloaking` on units inside a cloak field.** The cloak
  field operates at cell level only. Unit cloak state is independent.
- **Tiberian Sun ghost.** `CloakGenerator=yes` is TS legacy. Implement only if
  a mod activates it. Default retail YR has zero CloakGenerator buildings.

### Rust sim tick ordering

Insert `update_gap_generator_tick` for each building in the building animation /
cleanup phase of `World::advance_tick`. It must run AFTER combat and power-state
updates so `+0x6EB` can be toggled by the same-tick power change. The per-ring
apply/remove of cell cloak overlays should write to the shroud buffers that the
render phase reads -- keep writes confined to the sim phase.

---

## YR-active summary

| System | Active in retail YR? | Live buildings |
|--------|----------------------|----------------|
| `CloakGenerator=yes` | **No** | none |
| `SensorArray=yes` | **Yes** | Psychic Sensor (`NATA02` / `NACLON` variants), Spy Satellite (`NACLON`), Allied Radar Dome (some variants) |
| `DetectDisguise=yes` | **Yes** | Multiple buildings (Psychic Sensor, Spy Satellite, Soviet Radar, some infantry: Attack Dog, GI) |
| `GapGenerator=yes` on buildings | **Yes** | Gap Generator (`GAGAP`) |
| `Cloakable=yes` on buildings | **No** | none (only on Dolphin, Typhoon sub, Mirage Tank, etc.) |
| `PsychicDetectionRadius=` | **Yes** | Psychic Sensor (value 15) |

Prioritize implementing sensor array + disguise detect + unit-level cloak
(including Mirage tank and sub cloak) first. Defer CloakGenerator building
logic as a mod-only feature.
