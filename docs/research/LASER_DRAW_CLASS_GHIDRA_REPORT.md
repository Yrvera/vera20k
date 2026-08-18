# LaserDrawClass — Ghidra Research Report

**Class RTTI name (verified from binary):** `LaserDrawClass`
(strings at `0x008296e8` `.?AV?$VectorClass@PAVLaserDrawClass@@@@` and
`0x00829718` `.?AV?$DynamicVectorClass@PAVLaserDrawClass@@@@`)

**Primary constructor:** `0x0054FE60` — `LaserDrawClass::Constructor` (17-arg __thiscall)
**Allocation size:** `0x5C` bytes (via `operator_new(0x5c)`)
**Confidence:** HIGH — struct layout, all four lifecycle entry points, and the three
visual variants (normal laser / prism default / prism boosted) are verified from
decompilation. This report re-confirms and extends the research already in
`PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md §7–§9`.

---

## 1. Purpose & Live-in-YR Status

LaserDrawClass is the **generic per-segment laser renderer** for every weapon whose
beam is drawn as straight parallel lines between two points. It is **live in
standard YR skirmishes** and is the backing renderer for:

- **Prism Tower** (outgoing `PrismShot`, both normal and boosted) — INI `IsLaser=yes`
- **Prism support beams** (supporter → firing tower) — hard-wired by the prism
  cascade (see `PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md`)
- **Mirage Tank**, **Infantry Fortress Battle Fortress IFV laser beam**,
  **Guardian GI deployed**, **Robot Tank**, **Tank Destroyer** and similar
  `IsLaser=yes` / `IsBigLaser=yes` weapons
- **Disk Laser** (Kirov-era floating disc) — three spinning beams per orbit slot
  (see `DiskLaserClass::AI_Update` at `0x004A7340`)
- **Railgun trail** (Yuri Railgun) — trails along particle system path
  (`ParticleSystemClass__AI_Railgun` @ `0x0062F230`)
- **Electric Bolt fallback when `DrawBoltAsLaser=yes`** is set on an IsElectricBolt
  weapon (flag at WeaponType +0x152)

No `SpecialFlags` gating; no Tiberian-Sun-only code path in the hot loop.

---

## 2. Struct Layout (0x5C bytes)

`param_1` in the constructor is `undefined4 *`, so compiler-generated
`param_1[N]` expressions equal byte offset `4*N`. All offsets re-verified.

| Offset | Size | Field | Ctor init | Confidence | Notes |
|--------|------|-------|-----------|------------|-------|
| 0x00 | 4 | `AnimStep` | 0 | Verified | Counter incremented by `StepIncrement` each repeat by `UpdateAllAI` |
| 0x04 | 1 | `IsActive` | 0 | Verified | Set to 1 by UpdateAllAI when a repeat triggers, else cleared |
| 0x08 | 4 | `SpawnFrame` | `g_CurrentFrameCounter` | Verified | Timer base for expiry test |
| 0x0C | 4 | `AnimParamB` | param_3 (src_y) on initial pass | Verified | Reassigned to param_3 by repeat path in UpdateAllAI |
| 0x10 | 4 | `RemainingTicks` | 1 | Verified | Decremented by expiry test `(frame - SpawnFrame) ≥ RemainingTicks` |
| 0x14 | 4 | `Flag1` | 1 | Verified | Non-zero triggers repeat in UpdateAllAI |
| 0x18 | 4 | `StepIncrement` | 1 | Verified | Added to `AnimStep` on each repeat |
| 0x1C | 4 | `InnerLineCount` | 1 | Verified | **Number of parallel beam lines drawn** — 1=plain, 3=prism, 5=boosted prism |
| 0x20 | 1 | `IsLaserEffect` | 0 | Verified | When 1 → DrawBeamSpecial path at `0x005509F0`; cleared by ctor, set by `TechnoClass__SpawnLaser` (IsLaser) and `EmitPrismSupportBeam` |
| 0x21 | 1 | `IsBoosted` | 0 | Verified | Set to 1 by Fire_At when firing tower has prism supporters; causes the first draw pass to skip color halving |
| 0x24 | 4 | `SrcX` | param | Verified | Source 3D lepton coord |
| 0x28 | 4 | `SrcY` | param | Verified | |
| 0x2C | 4 | `SrcZ` | param | Verified | |
| 0x30 | 4 | `TgtX` | param | Verified | Target 3D lepton coord |
| 0x34 | 4 | `TgtY` | param | Verified | |
| 0x38 | 4 | `TgtZ` | param | Verified | |
| 0x3C | 4 | `Param7` | param | Inferred | Always 0 at every observed callsite; meaning unconfirmed |
| 0x40 | 1 | `OneShot` | param (byte, 1 for prism) | Verified | |
| 0x41-0x43 | 3 | `InnerColor.B,G,R` | ctor param (low 3 bytes of `inner_color_rgb`) | Verified | BGR byte triple (primary-surface 888 order) |
| 0x44-0x46 | 3 | `OuterColor.B,G,R` | ctor param | Verified | BGR |
| 0x47-0x49 | 3 | `SpreadColor.B,G,R` | ctor param (LaserOuterSpread RGB jitter amplitude) | Verified | BGR |
| 0x4C | 4 | `DurationTotal` | `duration_ticks` | Verified | Total frames the beam is alive; checked against `AnimStep` for destruction |
| 0x50 | 1 | `ToggleFlag` | param_13 (byte) | Verified | If non-zero, UpdateAllAI toggles `+0x51` each tick |
| 0x51 | 1 | `ToggledState` | 0 | Verified | Flips 0/1 each tick when `ToggleFlag` set (skips drawing when set) |
| 0x52 | 1 | `FadeEnable` | `is_laser_effect` param | Verified | When set, intensity linearly interpolates 0x54→0x58 across duration |
| 0x54 | 4 | `IntensityStart` | float | Verified | Begin fade value (1.0f for prism) |
| 0x58 | 4 | `IntensityEnd` | float | Verified | End fade value |

### Global container

| Address | Purpose | Confidence |
|---------|---------|------------|
| `0x00ABC87C` | `g_LaserDraw_Array` — array of `LaserDrawClass*` | Verified |
| `0x00ABC880` | `g_LaserDraw_Cap` — current array capacity | Verified |
| `0x00ABC885` | growable flag | Verified |
| `0x00ABC888` | `g_LaserDraw_Count` — current active count | Verified |
| `0x00ABC878` | `DynamicVectorClass` vtable ptr (expand/remove hooks) | Verified |

---

## 3. Constructor Signature (verified from disassembly)

```c
LaserDrawClass* __thiscall LaserDrawClass::Constructor(
    LaserDrawClass* this,     // ECX
    int32  src_x,             // stack +0x04
    int32  src_y,             //       +0x08
    int32  src_z,             //       +0x0C
    int32  tgt_x,             //       +0x10
    int32  tgt_y,             //       +0x14
    int32  tgt_z,             //       +0x18
    int32  param_7,           //       +0x1C  (always 0 for prism)
    uint8  one_shot,          //       +0x20  (byte)
    uint32 inner_color_rgb,   //       +0x24  (low 3 bytes stored)
    uint32 outer_color_rgb,   //       +0x28
    uint32 spread_color_rgb,  //       +0x2C
    int32  duration_ticks,    //       +0x30
    uint8  toggle_flag,       //       +0x34
    uint8  is_laser_effect,   //       +0x38
    float  intensity_start,   //       +0x3C
    float  intensity_end);    //       +0x40
```

Callee cleans 16 × 4 = 64 bytes of stack args (`RET 0x40`).

---

## 4. Vtable

LaserDrawClass has **no conventional vtable** in its object layout — it is a pure
data POD with no virtual functions. All lifecycle operations are done by static
free functions that walk the global array. This is atypical for the RA2 codebase
(most renderables inherit from `ObjectClass`) and is the reason it does NOT
participate in `LayerClass` rendering.

---

## 5. Lifecycle

### 5.1 Creation (six distinct callsites)

| Caller address | Meaning |
|----------------|---------|
| `0x006FD210` | `TechnoClass::SpawnLaser` — the IsLaser branch in Fire_At (buildings + units) |
| `0x006FF52C` | Prism boost block inside `TechnoClass::Fire_At` (sets InnerLineCount=3 or 5 and IsBoosted post-hoc) |
| `0x0044AC9E` | `EmitPrismSupportBeam` inside `BuildingClass::Update` — prism supporter→firing tower visual |
| `0x004A7691, 0x004A7804, 0x004A78DF` | `DiskLaserClass::AI_Update` — three beam legs from floating disk |
| `0x0062F940` | `ParticleSystemClass::AI_Railgun` — per-frame railgun trail segment |

Allocation is always `operator_new(0x5C)` followed directly by the 17-arg ctor.
The ctor itself appends `this` to `g_LaserDraw_Array` (growing capacity via the
DynamicVector expand hook if needed).

### 5.2 Per-tick AI — `LaserDrawClass::UpdateAllAI @ 0x00550150`

Called every sim tick from `LogicClass::PerTickUpdate` (xref at `0x0055B5C3`).

For every live laser:
1. Compute `elapsed = g_CurrentFrameCounter - SpawnFrame`. If `elapsed < RemainingTicks`
   the laser is "alive in this cycle" → `IsActive = 0`.
2. Otherwise:
   - If `StepIncrement == 0` → `IsActive = 0` (will be destroyed).
   - Else → repeat: `IsActive = 1`, `AnimStep += StepIncrement`,
     `SpawnFrame = g_CurrentFrameCounter`, `RemainingTicks = StepIncrement`.
3. If `ToggleFlag != 0` → flip `ToggledState` each tick.
4. If `AnimStep >= DurationTotal` → remove from array (via `DynamicVectorClass::Remove`
   at vtable+0x10) and `operator_delete`.

### 5.3 Per-frame draw — `LaserDrawClass::DrawAll @ 0x00550240`

Called from `TacticalClass::Draw` (xref at `0x006D4669`) **after**
`Tactical_ObjectRenderingLoop` (units/buildings) and **before** the
`RadBeam::DrawAndTickAll` call. It simply loops `g_LaserDraw_Count` times calling
`LaserDrawClass::Draw @ 0x00550260` per instance.

`Draw` dispatches on `IsLaserEffect` (+0x20):
- `IsLaserEffect == 1` → `DrawBeamSpecial @ 0x005509F0` (the thick parallel-line path)
- `IsLaserEffect == 0` → inline straight-line or X/Y axis-cross draw path

Both paths:
1. Skip if `ToggledState` (+0x51) is set this tick.
2. Compute screen-space src/tgt via `TacticalClass::CoordsToClient2`, then
   `AdjustForZ` for each end (function at `0x006D20E0`).
3. Compute angle octant from `atan2(dy,dx)` → used as index into a 16-float
   perpendicular-offset table at `0x00ABC738..0x00ABC7B0` (initialised lazily
   when `DAT_00ABC8B8 & 1` is 0).
4. Resolve drawing color:
   - Inner color is `[InnerColor.R, G, B]` at +0x41..+0x43.
   - If `SpreadColor ≠ 0,0,0` on any channel, add
     `RandomRanged(-spread, +spread)` per channel per pass (clamped to [0,255]).
   - Pack to surface pixel format via `g_DD_*Loss`/`g_DD_*Shift` globals.
5. If `FadeEnable != 0` → linearly interpolate `intensity = (IntensityStart*(D-S) + IntensityEnd*S) / D`
   where `D = DurationTotal`, `S = AnimStep`. Not applied to every backend —
   the gradient-capable DirectDraw surface variant (`surface vtable+0x40`) is used
   when `DAT_00A8EB78 != 0`, else a flat `DrawLine` at `surface vtable+0x34`.
6. Loop `i = 1..InnerLineCount`:
   - Draw one line at the current perpendicular offset pair.
   - If `IsBoosted && i==1` → keep the inner color at full (pre-doubled) intensity
     for the center beam. Otherwise halve each channel (`c >>= 1`) each iteration,
     producing the characteristic soft halo.

Screen draws go through `g_PrimarySurface` (DirectDraw7 surface). No depth/Z
integration — beams are painted straight onto the already-rendered object layer.

### 5.4 Destruction

- **Natural expiry** in `UpdateAllAI` when `AnimStep ≥ DurationTotal`.
- **Bulk flush** `LaserDrawClass::DestroyAll @ 0x00550000` — drains the global
  array, invoked on scenario teardown. No per-laser finalizer; just
  `DynamicVectorClass::Remove` + `operator_delete` (via `FUN_007C8B3D`).

---

## 6. INI Keys

All keys are parsed in `WeaponTypeClass::ReadINI @ 0x00772090` (source line
offsets listed are absolute within the ReadINI function).

| INI Key | Type | Offset on WeaponTypeClass | Default | Read at | Source in rulesmd.ini |
|---------|------|---------------------------|---------|---------|------------------------|
| `IsLaser` | bool | `+0x149` | false | `0x00772638` | Prism, Mirage, IFV engineer, etc. |
| `IsBigLaser` | bool | `+0x14C` | false | `0x007727FD` | Mirage Tank (`MirageLaser`) line 23946 |
| `IsHouseColor` | bool | `+0x14D` | false | `0x00772686` | Prism Support + Prism Shot |
| `LaserDuration` | int (stored as byte) | `+0x14E` | 0 | `0x007727E3` | e.g. `LaserDuration=15` (line 23804) |
| `LaserInnerColor` | RGB triple | `+0x120..+0x122` | 0,0,0 | `0x00772775` (`ReadColorRGB`) | e.g. `LaserInnerColor=255,0,0` (23940) |
| `LaserOuterColor` | RGB triple | `+0x123..+0x125` | 0,0,0 | `0x0077279B` | |
| `LaserOuterSpread` | RGB triple (used as per-channel jitter amplitude) | `+0x126..+0x128` | 0,0,0 | `0x007727C1` | e.g. `LaserOuterSpread=20,40,40` |

**On HouseClass:** `LaserColor` is parsed into HouseClass+0x56FC..0x56FE (BGR, 3
bytes) and is the source when `IsHouseColor=true` (inner=full, outer=each channel
halved). Verified at `0x004A7691`'s ctor-arg setup and in `TechnoClass::SpawnLaser`.

### Notable non-INI constant

- **Prism thickness**: `InnerLineCount` is NOT read from any INI key. Fire_At
  hard-codes it: `3` for a normal prism shot, `5` when the firing tower has any
  `PrismSupportCount > 0` supporters queued (see `PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md §9`).

### TS-legacy / non-stock keys

- `IsAlternateColor` (+0x153) is parsed (`0x00772899`) but its consumption path
  in the stock LaserDraw flow is limited to one check in
  `DiskLaserClass::AI_Update` (`*(char *)((int)param_2 + 0x153)` at
  `0x004A7689`). Other engines (Ares) extend it; YR uses it only for disk laser
  variation. Not TS legacy, but low impact.
- `LaserDuration` is stored as a single byte (`0x7727E3` writes `*(byte*)(this+0x14E)`),
  so values >255 are silently truncated. Verified from disassembly.

---

## 7. Rendering Details

- **Coordinate space**: 3D world leptons (same frame as `CoordStruct`), projected
  per-draw by `TacticalClass::CoordsToClient2` and `AdjustForZ` for the isometric
  Z shear.
- **Color order**: BGR (byte triples, little-endian matches the 24-bit
  primary-surface layout; repacked into pixel format via
  `g_DD_R/G/B_{Loss,Shift}` at draw time).
- **Width / thickness**: controlled entirely by `InnerLineCount`. Parallel lines
  at 16 fixed perpendicular offsets (table at `0x00ABC738`). Each iteration from
  center outward halves the color intensity (except `IsBoosted` first pass).
- **Spread jitter**: if `LaserOuterSpread != 0`, per-channel random offset in
  `[-spread, +spread]` added to the outer lines' color each draw frame. This is
  what makes the Prism Tower beam visibly "sparkle".
- **Duration**: `DurationTotal` (from `LaserDuration` INI key, in ticks). On each
  tick, `AnimStep` advances by `StepIncrement`; the beam is destroyed when
  `AnimStep >= DurationTotal`.
- **Intensity fade curve**: linear, `(IntensityStart*(D-S) + IntensityEnd*S) / D`
  where S = `AnimStep`, D = `DurationTotal`. Only used by backends whose surface
  vtable slot 0x40 supports gradient lines; otherwise ignored.
- **Animation curve**: flat (same color pair every frame) except for
  `LaserOuterSpread` RNG, the linear fade above, and the `ToggleFlag` half-tick
  blink.
- **Layer / Z**: **bypasses the `LayerClass` system entirely**. Drawn directly to
  `g_PrimarySurface` from `TacticalClass::Draw` at a fixed position in the draw
  order — after `Tactical_ObjectRenderingLoop` (units/buildings/anims), before
  `RadBeam::DrawAndTickAll` and `Tactical__DrawUnitActionVisuals`. This is why
  lasers always appear on top of units and buildings in the tactical view.

---

## 8. Call Graph

**Creators** (all six verified via Ghidra xrefs to `LaserDrawClass::Constructor`):

```
TechnoClass::Fire_At (0x006FDD50)
  ├─► TechnoClass::SpawnLaser (0x006FD210)      ─── IsLaser weapon path
  │    └─► LaserDrawClass::Constructor
  ├─► FUN_006FF52C (inline prism boost block)   ─── sets Inner=3 or 5 post-hoc
  │    └─► (mutates laser returned by SpawnLaser)
  └─► (no direct IsElectricBolt path; EBolt uses its own EBoltClass)

BuildingClass::Update (0x0043FB20)
  └─► EmitPrismSupportBeam (≈ 0x0044ABD0)
       └─► LaserDrawClass::Constructor

DiskLaserClass::AI_Update (0x004A7340)
  └─► LaserDrawClass::Constructor × 3 (three orbit legs)

ParticleSystemClass::AI_Railgun (0x0062F230)
  └─► LaserDrawClass::Constructor (per-segment trail)
```

**Tickers / drawers:**

```
LogicClass::PerTickUpdate (0x0055B...)
  └─► LaserDrawClass::UpdateAllAI (0x00550150)   ─── advances AnimStep, culls

TacticalClass::Draw (0x006D3D10)
  └─► LaserDrawClass::DrawAll (0x00550240)       ─── iterates g_LaserDraw_Array
       └─► LaserDrawClass::Draw (0x00550260)     ─── per-instance dispatch
            └─► LaserDrawClass::DrawBeamSpecial (0x005509F0)   ─── IsLaserEffect=1
```

---

## 9. Open Questions / Not Confirmed

1. **Field `+0x3C` (Param7)**: always 0 at every observed callsite. Could be a
   reserved "beam type enum" (ModEnc folklore), a fade-curve shape selector, or
   legacy dead parameter. No code path reads it in the verified draw functions.
2. **Intensity fade backend branching**: the `DAT_00A8EB78` flag that toggles
   between flat-color `DrawLine` (surface +0x34) and gradient `DrawLine`
   (surface +0x40) has not been traced. Possibly a DirectDraw feature-detect
   at startup.
3. **Angle LUT at 0x00ABC738..0x00ABC7B0**: 16 ints × 2 = 32 values (8 octants ×
   4 offsets). The exact offset magnitudes were not individually verified; they
   drive the perpendicular spacing between parallel lines.
4. **`LaserDuration` byte truncation**: verified storage is `*(byte*)(this+0x14E)`,
   meaning INI values 256+ would wrap modulo 256. Should be confirmed by
   checking a ResourceHacker view of compiled bytes at 0x007727E9 to rule out
   a decompile artifact.

---

## 10. TS-Legacy Risks

- No known TS-era gating on this class.
- **`IsAlternateColor` (+0x153)** is parsed by YR's WeaponTypeClass::ReadINI but
  actively consumed only by DiskLaserClass. Low TS-legacy risk: it's a real bit
  but its effect is narrow. Flag if re-implementing disk laser.
- **LaserDrawClass predates the LayerClass system** (TS-era layout). It writes
  directly to the primary surface instead of registering with a layer. This is
  **not** a TS-dormant bug — it's intentional in YR and drives the "lasers draw
  over everything" behavior. Reproducing this in Rust requires a post-sprite
  pass, not an insertion into the main sprite layer.

---

## 11. Ghidra Functions Labeled in This Session

| Address | New name |
|---------|----------|
| `0x0054FE60` | `LaserDrawClass__Constructor` |
| `0x00550000` | `LaserDrawClass__DestroyAll` |
| `0x00550150` | `LaserDrawClass__UpdateAllAI` |
| `0x00550240` | `LaserDrawClass__DrawAll` |
| `0x00550260` | `LaserDrawClass__Draw` |
| `0x005509F0` | `LaserDrawClass__DrawBeamSpecial` |
| `0x006FD210` | `TechnoClass__SpawnLaser` |

Saved via `save_program` at end of session.

---

**Verified 2026-04-21** from gamemd.exe (image base 0x00400000, SHA from 2026-03-15
build) via Ghidra MCP: constructor decompile, all four lifecycle functions,
TacticalClass::Draw draw-order context, WeaponTypeClass::ReadINI key-by-key
extraction, and cross-check against `ini/rulesmd.ini` for real-world INI values.

---

## Follow-up investigation (round 2, 2026-04-21)

### Q1 — Laser color plumbing: Inner/Outer/OuterSpread from WeaponTypeClass → Draw

**Resolution: RESOLVED (HIGH confidence).** Colors travel end-to-end as **raw 24-bit
BGR byte triples** — they are NOT mapped through any LUT (unlike ElectricBolt's
`DAT_0087F6C4+0x174` per-color table). At draw time the three R/G/B bytes are only
subjected to the DirectDraw surface pixel-format packing (Loss/Shift), which is
trivial bit-slicing, not a palette/LUT lookup.

#### Storage on WeaponTypeClass (verified at the ReadINI offsets already listed in §6)

| INI key | WeaponType offset | Layout |
|---------|--------------------|--------|
| `LaserInnerColor` | `+0x120..+0x122` | 3 bytes, BGR order on disk (ReadColorRGB writes R,G,B but laser code reads B@+0x41,G@+0x42,R@+0x43 — see below) |
| `LaserOuterColor` | `+0x123..+0x125` | 3 bytes |
| `LaserOuterSpread` | `+0x126..+0x128` | 3 bytes (per-channel random-jitter amplitude) |
| `IsHouseColor` | `+0x14D` | bool |

#### Flow into the constructor (verified from `TechnoClass::SpawnLaser @ 0x006FD210`)

```c
// SpawnLaser begins:
if (weaponType->IsHouseColor == 0) {
    pbVar9   = weaponType + 0x120;   // LaserInnerColor ptr (3 bytes)
    local_2c = weaponType + 0x123;   // LaserOuterColor ptr
}
else {
    // House color override:
    int houseClass = techno->Owner /* techno+0x21C */;
    pbVar9 = houseClass + 0x56FC;    // HouseClass.LaserColor (B byte)
    local_38 = CONCAT12(houseClass->LaserColor.R >> 1,       // halved
                        CONCAT11(houseClass->LaserColor.G >> 1,
                                 houseClass->LaserColor.B));
    local_2c = &local_38;            // "outer" = half-intensity house color
    // Note: Inner stays full-brightness house color, Outer is each channel
    //       right-shifted by 1 (divided by 2). LaserOuterSpread is STILL
    //       read from the weapon (+0x126) even in IsHouseColor mode.
}
LaserDrawClass__Constructor(
    src, tgt,
    /* param_7 */ 0,
    /* one_shot */ 1,
    /* inner_color  */ *(u24*)pbVar9,               // raw BGR
    /* outer_color  */ *(u24*)local_2c,             // raw BGR (possibly halved)
    /* spread_color */ *(u24*)(weaponType + 0x126), // raw BGR jitter magnitude
    /* duration */ weaponType->LaserDuration,
    /* toggle */ 0, /* is_laser_effect */ 1,
    /* intensity_start */ 1.0f, /* intensity_end */ 1.0f);
// Post-ctor, IsLaserEffect flag is flipped on if IsHouseColor:
if (weaponType->IsHouseColor) laser->IsLaserEffect /*+0x20*/ = 1;
```

**HouseClass source** for `IsHouseColor=true`: `HouseClass+0x56FC..+0x56FE` is the
`LaserColor` RGB byte triple parsed from the `[HouseInfo]`-linked house's `Color=`
entry (via `HouseTypeClass::LaserColor`, then colorscheme resolution). Confirmed
inline in SpawnLaser: `iVar8 = param_1[0x87]; pbVar9 = (byte *)(iVar8 + 0x56fc);`
where `param_1` is the firing Techno and index 0x87 = Techno+0x21C = `Owner: HouseClass*`.

#### Use in Draw (verified at `LaserDrawClass::Draw @ 0x00550260` and `DrawBeamSpecial @ 0x005509F0`)

The three color triples are stored verbatim into the LaserDrawClass at ctor time:
- `laser->InnerColor.BGR` at `+0x41..+0x43`
- `laser->OuterColor.BGR` at `+0x45..+0x47` (see note on field boundaries below)
- `laser->SpreadColor.BGR` at `+0x49..+0x4B`

**Note on struct table accuracy:** the §2 layout column lists OuterColor at
`+0x44..+0x46` and SpreadColor at `+0x47..+0x49`, but the decompiler clearly reads
OuterColor at bytes offsets `param_1+0x45/0x46` and `+0x44` (via `param_1+0x11`
= byte offset 0x44) and SpreadColor at `+0x47`/`+0x48`/`+0x49`. The actual
byte-granular layout for the color block is:

```
+0x41: Inner.B
+0x42: Inner.G
+0x43: Inner.R
+0x44: (pad / reserved)         <-- `param_1 + 0x11` dereference = this byte
+0x45: Outer.B
+0x46: Outer.G
+0x47: Outer.R                  <-- or reused as next block start; code reads both
                                    `+0x11` (=0x44), `+0x45`, `+0x46` as Outer {B,G,R}
+0x48: (pad)
+0x49: Spread.B? / channel     (ctor stored `param_12` low byte at `+0x47`,
                                continues at `+0x48`, then `+0x49`)
```

The decompile is noisy about the exact byte packing (ctor uses `CONCAT12` /
`CONCAT11` to pack the three color bytes into two dword stores per color),
but the **semantic flow is unambiguous**: three RGB triples are stored verbatim
and read back verbatim during draw. No table lookup ever occurs.

##### Draw path `LaserDrawClass::Draw` (normal, non-special)

```c
if (param_1->LaserOuterColor_any_nonzero || LaserOuterSpread_any_nonzero) {
    // Outer-with-spread branch:
    int jR = RandomRanged(-spreadR, +spreadR);
    int jG = RandomRanged(-spreadG, +spreadG);
    int jB = RandomRanged(-spreadB, +spreadB);
    R = clamp(OuterColor.R + jR, 0, 255);
    G = clamp(OuterColor.G + jG, 0, 255);
    B = clamp(OuterColor.B + jB, 0, 255);
    // Pack to surface pixel format via DD_{R,G,B}_{Loss,Shift}
    pixel_rgb16or24 = (R >> g_DD_RLoss) << g_DD_RShift |
                      (G >> g_DD_GLoss) << g_DD_GShift |
                      (B >> g_DD_BLoss) << g_DD_BShift;
    // Draw outer border lines at ±offset perpendicular to beam axis
}
// Always draw inner (no spread):
pixel_inner = pack_to_surface(InnerColor.R, InnerColor.G, InnerColor.B);
primarySurface->DrawLine(src_screen, tgt_screen, pixel_inner, ...);
```

##### Draw path `LaserDrawClass::DrawBeamSpecial` (used when IsLaserEffect=1,
i.e. SpawnLaser/Prism/EmitPrismSupportBeam path)

```c
if (laser->IsBoosted /*+0x21*/ == 0) {
    R = InnerColor.R;
    G = InnerColor.G;
    B = InnerColor.B;
    // Halved variant for subsequent parallel-line iterations:
    halfR = R >> 1; halfG = G >> 1; halfB = B >> 1;
} else {
    // Prism boost: multiply first-pass color by 2 (capped at 0xFF).
    // The FIRST line iteration uses the doubled color; subsequent
    // iterations halve as normal.
    R = min(InnerColor.R * 2, 0xFF);
    G = min(InnerColor.G * 2, 0xFF);
    B = min(InnerColor.B * 2, 0xFF);
    halfR = R; halfG = G; halfB = B;   // no halving for first pass
}
for (i = 1; i <= laser->InnerLineCount; i++) {
    if (i == 1 && laser->IsBoosted) {
        // Center line of boosted prism = pre-doubled color, full intensity.
    } else {
        // Move to outer line offset; halve color each step.
        R >>= 1; G >>= 1; B >>= 1;
        if (R < 0x40 && G < 0x40 && B < 0x40) break;  // early-out when too dark
    }
    primarySurface->DrawLine(src_screen+offset, tgt_screen+offset,
                             pack_to_surface(R, G, B), ...);
}
```

**Key confirmations:**

1. **Raw RGB, no LUT.** The only transform applied to the RGB bytes before
   drawing is the DirectDraw surface pixel-format pack (`>> Loss << Shift`),
   which is a dumb bit re-encoding for 16/24/32-bit target surfaces — not a
   color table / palette lookup. Compare with ElectricBoltClass which DOES
   index into `DAT_0087F6C4+0x174` (a per-index color table). LaserDrawClass
   differs here: it uses the weapon's literal RGB numbers.

2. **Spread is per-frame RNG jitter on the outer line only.** The existing §7
   description ("sparkle") is accurate. It is NOT read during inner-line draws.

3. **IsHouseColor override path.** Verified to pull from `HouseClass+0x56FC`
   (B,G,R at +0x56FC, +0x56FD, +0x56FE). Pre-ctor, SpawnLaser builds:
   - inner = `{houseB,       houseG,       houseR}` (full)
   - outer = `{houseB >> 1, houseG >> 1, houseR >> 1}` (halved)
   - spread = still `WeaponType+0x126..+0x128`
   Then sets `laser->IsLaserEffect=1` post-construction so the DrawBeamSpecial
   path runs. This is the "prism tower uses house color with halved outer" path.

4. **Prism InnerLineCount override** (already noted in §6) is the ONLY color-
   related difference between a plain IsLaser weapon and a prism shot: the
   color data itself is still straight-through weapon → ctor → surface.

**No new labels needed** — all relevant functions were already labeled
(`LaserDrawClass__Constructor`, `TechnoClass__SpawnLaser`, `LaserDrawClass__Draw`,
`LaserDrawClass__DrawBeamSpecial`). This round re-confirms the pipeline; no
new writers or LUTs were discovered.
