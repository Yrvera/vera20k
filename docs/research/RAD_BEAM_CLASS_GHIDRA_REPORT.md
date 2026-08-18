# RadBeam — Ghidra Research Report

**Class RTTI name (verified from binary):** `RadBeam`
(strings at `0x00839540` `.?AV?$VectorClass@PAVRadBeam@@@@` and
`0x00839570` `.?AV?$DynamicVectorClass@PAVRadBeam@@@@`)

> Note on nomenclature: YRpp and community sources sometimes call this
> "`RadBeamClass`". The binary RTTI string is literally `RadBeam`. Both struct
> layout and behavior match the YRpp-documented RadBeam.

**Allocator:** `0x00659110` — `RadBeam__Allocate` (returns initialized `RadBeam*`)
**Constructor body:** `0x006593F0` — `RadBeam__Constructor` (field zero-init)
**Allocation size:** `200` bytes (`0xC8`) via `operator_new(200)`
**Confidence:** HIGH for struct layout, both draw paths, per-tick/per-frame logic,
and all callsites. MEDIUM for exact semantic labels on a handful of floats and
the `+0x18` step-size constant.

---

## 1. Purpose & Live-in-YR Status

`RadBeam` is the visual beam drawn between an attacker and its target for weapons
whose WeaponTypeClass flag `IsRadBeam=yes` is set. **Live in standard YR
skirmishes**, used by:

- **Desolator** weapon (`RadBeamWeapon`, `CRRadBeamWeapon`, `RadBeamWeaponE`) —
  green straight beam (type 0, color = `[Radiation] RadColor=0,255,0`).
- **Chrono Legionnaire** weapon (`NeutronRifle`, `CRNeutronRifle`, `NeutronRifleE`)
  whose warhead is `ChronoBeam` with `Temporal=yes` — blue beam (type 1, color
  = `RulesClass + 0x1866` RGB).
- **RadEruption** (`IsRadEruption=yes`) spawns 8 RadBeam instances in a 3×3
  neighbor grid from the impact cell (path via `TechnoClass__SpawnRadEruption
  @ 0x006FD800`).

A third color branch selects `RulesClass + 0x1869` when type == 2; **no live
callsite currently invokes type 2**. Check for TS-only dormancy before
implementing a type-2 path.

**TS-legacy considerations:** See §10. The beam code itself is live; two
known dead sub-branches are documented there.

---

## 2. Struct Layout (0xC8 bytes)

`param_1` in the ctor is `undefined4 *` (so `param_1[N]` = byte offset `4*N`).
Every offset below is verified against a writer in the constructor, a setter,
or a read in either draw path.

| Offset | Size | Field | Ctor init | Confidence | Notes |
|--------|------|-------|-----------|------------|-------|
| 0x00 | 4 | `vtable` (?) | 0 | Inferred | Set to 0 by ctor; treated as opaque. No virtual calls observed — plausibly padding. |
| 0x04 | 4 | `TargetObjectPtr` | 0 | Verified | Used in temporal lock-back at end of `TechnoClass__SpawnRadBeam` when type==2 (dead in YR) |
| 0x08 | 1 | `GeometryReady` | 0 | Verified | Set to 1 on first draw pass (lazy init of dst−src delta + length) |
| 0x09 | 1 | `IsActive` | 1 | Verified | When 0, the beam is skipped in draws; when 1 and type 2 with a random chance, ctor of tick can force-deactivate (fade) |
| 0x0C | 4 | `ZOffsetAdjust` | 0 | Verified | Written by `RadBeam__SetSourceZAdjust`; used as extra Z delta inside draw (`(iVar5 - iVar4) + -2`-style compositing) |
| 0x10 | 4 | `BeamType` | 0 | Verified | 1 = straight-line (`DrawStraightBeam`), 2 = sinusoidal (`DrawSineBeam`). Set by `RadBeam__SetBeamType`. |
| 0x18 | 8 | `StepSize` (double) | 10.0 or 20.0 | Verified | Set by `RadBeam__Allocate`: `10.0` if allocator param == 1 (Chrono Legion blue), else `20.0`. Step in leptons between drawn line segments along the beam length. |
| 0x20 | 2 | `Color.BG` | 0 | Verified | Written by `RadBeam__SetColor` (2 bytes) |
| 0x22 | 1 | `Color.R` | 0 | Verified | Written by `RadBeam__SetColor` (high byte) |
| 0x24-0x2F | 12 | `SrcCoord` (X,Y,Z 3×int) | 0 | Verified | Source coords set by `RadBeam__SetSourceCoord` |
| 0x30-0x3B | 12 | `TgtCoord` (X,Y,Z 3×int) | 0 | Verified | Target coords set by `RadBeam__SetTargetCoord` |
| 0x3C | 4 | `DurationTotal` | 0 | Verified | Set by `RadBeam__SetDuration` to `0x0F` (15 ticks) for rad beam (§6) |
| 0x40 | 8 | `Amplitude` (double) | 0 | Verified | Set by `RadBeam__SetAmplitudeAndPeriod` (float param re-read as double). `0x40440000` = 3.0f converted → 12.0 (3.0 in the double, which is the amplitude scale). |
| 0x48 | 8 | `IntensityFadeParams` (double / 2×float) | 0 | Verified | Written by `RadBeam__SetIntensityFade` (2 args). Used by RadEruption only. |
| 0x50 | 4 | `AuxScalar` | 0 | Verified | Written by `RadBeam__SetIntensity`. Used only by RadEruption path. |
| 0x54 | 4 | `BeamType2` alias / sub-shape | 0 | Verified | Written by `RadBeam__SetBeamType` with value 1 in `TechnoClass__SpawnRadBeam` when param==2 (dead in YR) |
| 0x58 | 1 | `RetainEndpoint` | 0 | Verified | When set (type-2 temporal-lock path), the tick function snaps the target coord from the linked TechnoClass each frame |
| 0x5C-0x67 | 12 | `DeltaXYZ` (int[3]) | 0 | Verified | dst-src per axis, lazy-computed on first draw |
| 0x68 | 8 | `Length` (double) | 0 | Verified | Euclidean length in leptons, lazy-computed |
| 0x70-0x7B | 12 | `DirVec` (normalized dir, int[3]) | 0 | Verified | `DeltaXYZ / Length` (rounded), used by sine path |
| 0x7C | 4 | `AgeInFrames` | 0 | Verified | Incremented each tick; beam expires when `AgeInFrames ≥ DurationTotal` |
| 0x80 | 8 | `FadeFactor` (double) | 0 | Verified | `(AgeInFrames / DurationTotal) * Amplitude` — the linear growth factor for the sine envelope |
| 0x88 | 4 | `SegmentIndex` | 0 | Verified | Inner draw-loop counter (current segment along the beam) |
| 0x8C | 4 | `SegmentCount` | 0 | Verified | Total number of segments: `ftol(Length / StepSize)` rounded. For sine path, drives Cos phase. |
| 0x90-0x9B | 12 | `PerpDir` (int[3]) | 0 | Verified | For sine path: the stored perpendicular direction snapshot |
| 0x9C-0xA7 | 12 | `CurrentSegmentSrc` (int[3]) | 0 | Verified | Projected coord at start of current segment |
| 0xA8-0xB3 | 12 | `CurrentSegmentDst` (int[3]) | 0 | Verified | Projected coord at end of current segment |
| 0xB8 | 8 | `CurrentSegmentScale` (double) | 0 | Verified | For sine path: current envelope magnitude |
| 0xC0 | 1 | `Draw.R` | 0 | Verified | Per-segment drawing byte R (sine path may dither it) |
| 0xC1 | 1 | `Draw.G` | 0 | Verified | Per-segment G |
| 0xC2 | 1 | `Draw.B` | 0 | Verified | Per-segment B |

### Global container (at DAT_00B04A60…)

| Address | Purpose | Confidence |
|---------|---------|------------|
| `0x00B04A60` | `DynamicVectorClass` vtable ptr | Verified |
| `0x00B04A64` | `g_RadBeam_Array` — `RadBeam**` | Verified |
| `0x00B04A68` | `g_RadBeam_Cap` | Verified |
| `0x00B04A6D` | growable flag | Verified |
| `0x00B04A70` | `g_RadBeam_Count` | Verified |
| `0x00B04A74` | capacity growth increment | Verified |

---

## 3. Constructors

### 3.1 `RadBeam__Allocate @ 0x00659110`

```c
RadBeam* RadBeam__Allocate(int type_selector);
//   type_selector == 1  →  StepSize = 10.0  (used by temporal/neutron blue beam)
//   otherwise          →  StepSize = 20.0  (used by green rad beam)
```

Steps:
1. `this = operator_new(0xC8)`.
2. `RadBeam__Constructor(this, type_selector)` zero-inits every field (this is
   the function that also writes `*(TargetObjectPtr at +0x04) = type_selector`).
3. Append `this` to the global `g_RadBeam_Array`, growing capacity if needed.
4. Write `*(double*)(this + 0x18) = (type_selector == 1) ? 10.0 : 20.0`.

### 3.2 `RadBeam__Constructor @ 0x006593F0`

Pure zero-init (all fields via `param_1[N] = 0` except `+0x09 = 1` and `+0x15 = 1`).
Writes the ctor param (the "type selector" passed from `RadBeam__Allocate`) into
`param_1[4]` (offset `+0x10 = BeamType`) — **this** is the value the global
tick iterator branches on. So:

- `BeamType == 0` → inactive (no draw code path handles 0 in `DrawAndTickAll`)
- `BeamType == 1` → `DrawStraightBeam`
- `BeamType == 2` → `DrawSineBeam`

The callsite pattern sets BeamType post-ctor via `RadBeam__SetBeamType` to 1 or
2 before the beam is ticked.

---

## 4. Setters (all verified, trivial writers)

| Address | New name | Writes |
|---------|----------|--------|
| `0x00659470` | `RadBeam__SetColor` | `+0x20,+0x21` (2 bytes), `+0x22` (1 byte) — 3 color bytes BGR |
| `0x00659490` | `RadBeam__SetSourceZAdjust` | `+0x0C` int |
| `0x006594A0` | `RadBeam__SetSourceCoord` | `+0x24/+0x28/+0x2C` (X,Y,Z) |
| `0x006594C0` | `RadBeam__SetTargetCoord` | `+0x30/+0x34/+0x38` (X,Y,Z) |
| `0x006594E0` | `RadBeam__SetDuration` | `+0x3C` int |
| `0x006594F0` | `RadBeam__SetAmplitudeAndPeriod` | `+0x40/+0x44` two ints |
| `0x00659510` | `RadBeam__SetIntensityFade` | `+0x48/+0x4C` two ints (used by RadEruption only) |
| `0x00659530` | `RadBeam__SetIntensity` | `+0x50` int (RadEruption) |
| `0x00659540` | `RadBeam__SetBeamType` | `+0x54` int |
| `0x00659550` | `RadBeam__SetRetainEndpoint` | `+0x58` byte |

---

## 5. Vtable

RadBeam has **no true vtable** — like LaserDrawClass it is a POD. The `+0x00` slot
is written to 0 by the ctor and never dereferenced as a function pointer.
All dispatch happens via the `BeamType` field discriminator inside
`RadBeam__DrawAndTickAll`.

---

## 6. Lifecycle

### 6.1 Creation — `TechnoClass::Fire_At` branch

From `TechnoClass__SpawnRadBeam @ 0x006FD620` (only called when weapon
`IsRadBeam=yes`):

```c
int TechnoClass__SpawnRadBeam(TechnoClass* target, int beam_type) {
    RadBeam* beam = RadBeam__Allocate(beam_type);
    // source coord from firer's FireFLH-adjusted position (vtable+0xB0)
    RadBeam__SetSourceCoord(beam, fire_src);
    // target coord from target->GetCoords (vtable+0xAC)
    RadBeam__SetTargetCoord(beam, target_coord);
    // compute screen Z-delta
    ...
    // color source:
    uint8* rgb = (beam_type == 1) ? &Rules.BlueBoltColor[0x1866]
               : (beam_type == 2) ? &Rules.RedBoltColor [0x1869]   // unreachable in YR
               :                    &Rules.RadColor    [0x1830];
    RadBeam__SetColor(beam, rgb);
    // Fixed constants for temporal / rad beam:
    RadBeam__SetDuration(beam, 0x0F);               // 15 frames = ~1.5 sec @ 10fps sim
    RadBeam__SetAmplitudeAndPeriod(beam, 0, 0x40440000); // 3.0f
    RadBeam__SetSourceZAdjust(beam, iVar3);
    if (beam_type == 2) {
        RadBeam__SetBeamType(beam, 1);               // dead in YR
        RadBeam__SetRetainEndpoint(beam, 1);
        // link to target for endpoint snap
    }
    return beam;  // keep as list entry only; caller doesn't store ptr
}
```

From `TechnoClass__SpawnRadEruption @ 0x006FD800` (only called when weapon
`IsRadEruption=yes`):

Iterates an 8-neighbor 3×3 ring around the firing unit's cell. For each,
allocates a RadBeam with random intensity (`5..20`), random duration (`100..500`),
and random amplitude/phase. Used by no stock YR weapon (see §10).

### 6.2 Per-frame tick + draw — `RadBeam__DrawAndTickAll @ 0x006591B0`

Called from `TacticalClass::Draw` (xref at `0x006D4678`), AFTER LaserDraw::DrawAll
and BEFORE `Tactical__DrawUnitActionVisuals`. **This function does BOTH the
tick and draw in one pass** — unlike LaserDrawClass which splits them.

For each live RadBeam (iterated backward):
1. Count how many `BeamType==2` beams are active AND `IsActive==1`
   (for load-based render-skip heuristic).
2. If the count > a threshold (pre-computed from
   `DAT_00ABCD44 / DAT_00A8B558` cubed), start dropping type-2 draws stochastically.
3. For each beam, dispatch on `BeamType`:
   - `BeamType == 1` → `RadBeam__DrawStraightBeam @ 0x00659650`:
     - Lazy-init `DeltaXYZ`, `Length` from src/tgt.
     - If `RetainEndpoint` set, snap TgtCoord from the linked target each tick.
     - For each segment `i = 0..ceil(Length/StepSize)`:
       - Compute segment endpoints along the straight line.
       - Project to screen via `CoordsToClient2`.
       - Clip via `FUN_007BC2B0`.
       - `AdjustForZ` on both ends.
       - Draw line via `g_PrimarySurface->vtable[0x34]` (DrawLine, flat color).
     - Advance fade animation: `AgeInFrames += CurrentSegmentScale`. Reverse
       direction when reaching start/end of duration (ping-pong).
   - `BeamType == 2` → `RadBeam__DrawSineBeam @ 0x00659CA0`:
     - Random-death chance per tick (for the RadEruption cluster fade-out).
     - `RadBeam__InitSineBeamGeometry` on first tick.
     - `RadBeam__UpdateSineTaper` — envelope is `((age*2/D) or (D - age*2/D))`
       triangle-wave × `Amplitude`.
     - For each segment `i = 0..SegmentCount`:
       - `RadBeam__ComputeSineSegment` — cos-wave offset perpendicular to the
         beam direction via `Cos_lookup(DAT_00839530 * (age/SegCount) * DAT_007E5F30)`.
       - Per-segment random color dither (`Draw.R/G/B` at +0xC0..+0xC2).
       - Draw with screen-coord + Z clip.
     - `AgeInFrames++`.
4. If `AgeInFrames >= DurationTotal` → remove from global array, delete.

### 6.3 Destruction

Natural expiry at `AgeInFrames ≥ DurationTotal` inside `DrawAndTickAll`. There is
no bulk-destroy function — the class relies on individual expiry. If the
beam holds a back-pointer (type 2 retain-endpoint), the tick function also
clears the target's `+0x510` back-ref on destroy.

---

## 7. INI Keys

**RadBeam has NO per-weapon INI configuration keys in stock YR.** The
"activator" is `IsRadBeam=yes` on the WeaponTypeClass (offset +0x154, parsed at
`0x007728B3` in `WeaponTypeClass::ReadINI`), plus the warhead's `Temporal=yes`
flag (offset +0x15A on WarheadTypeClass) which flips `beam_type` 0 → 1.

**Color sources** (verified from binary strings and xrefs into
`RulesClass::ReadRadiation` and related):

| Beam type | Rules offset | INI section / key | Default | Source line in rulesmd.ini |
|-----------|--------------|-------------------|---------|-----------------------------|
| 0 (rad beam — Desolator) | `RulesClass + 0x1830` | `[Radiation] RadColor=R,G,B` | `0,255,0` (green) | line 932 |
| 1 (temporal beam — Chrono Legion) | `RulesClass + 0x1866` | *(not stock YR INI-parsed; hardcoded constant in binary)* | TS-era blue | — |
| 2 (red bolt, dead) | `RulesClass + 0x1869` | *(not stock YR INI-parsed)* | TS-era red | — |

**Keys searched in binary and confirmed NOT present:**
`RadBeamAmplitude`, `RadBeamColor`, `RadOuterSpread`, `RadBeamPeriod`,
`RadBeamDuration`. These are Ares/community extensions — stock YR has only
`RadColor` (via the radiation system) and the hardcoded beam properties
(duration=15, amplitude=3.0f) inside `TechnoClass__SpawnRadBeam`.

**WarheadTypeClass flags that steer RadBeam:**
| INI Key | Offset | Purpose |
|---------|--------|---------|
| `Temporal` | `+0x15A` | When set on warhead of an `IsRadBeam` weapon, beam_type flips 0 → 1 (blue) |

---

## 8. Rendering Details

- **Coordinate space**: 3D world leptons, projected per segment via
  `TacticalClass::CoordsToClient2` and `AdjustForZ` for the isometric Z shear.
- **Primary color**: single RGB triple (from `RadColor` for Desolator, or a
  hardcoded Rules constant for temporal/bolt variants). No inner/outer
  distinction, unlike LaserDrawClass.
- **Width / thickness**: always 1-pixel per-segment line. The beam appears
  "thick" only via the sine envelope (BeamType 2) or density of overlapping
  segments.
- **Animation curve**:
  - BeamType 1 (straight): segment walk + ping-pong intensity modulation
    (no sine). Duration = 15 ticks hardcoded.
  - BeamType 2 (sine): cosine-wave perpendicular displacement with triangular
    amplitude envelope (grows from 0 to Amplitude, then back to 0).
- **Step size**: fixed at 10 (type 1, blue) or 20 (type 0 green) leptons per
  segment, set by `RadBeam__Allocate`.
- **Duration**: 15 ticks for main beam; 100..500 random for RadEruption clusters.
- **Layer / Z**: **bypasses the `LayerClass` system**, writing directly to
  `g_PrimarySurface` (vtable slot 0x34 = DrawLine). Drawn in `TacticalClass::Draw`
  AFTER `LaserDrawClass::DrawAll` and BEFORE `Tactical__DrawUnitActionVisuals`.
  This means RadBeam draws on top of lasers but under action-visual overlays.

---

## 9. Call Graph

```
TechnoClass::Fire_At (0x006FDD50)
  ├── IsRadBeam && warhead.Temporal==no   ─► TechnoClass__SpawnRadBeam(target, 0)
  │                                          └─► RadBeam__Allocate(0) [green, step=20]
  │                                          └─► setters → BeamType=1 straight
  │
  ├── IsRadBeam && warhead.Temporal==yes  ─► TechnoClass__SpawnRadBeam(target, 1)
  │                                          └─► RadBeam__Allocate(1) [blue, step=10]
  │                                          └─► setters → BeamType=1 straight
  │                                          └─► (linked to target via +0x510)
  │
  └── IsRadEruption                        ─► TechnoClass__SpawnRadEruption(center)
                                             └─► 8 × RadBeam__Allocate(0)
                                                  └─► BeamType=2 sine, random amp/dur

TacticalClass::Draw (0x006D3D10)
  └── RadBeam__DrawAndTickAll (0x006591B0)
       ├── BeamType==1 ─► RadBeam__DrawStraightBeam (0x00659650)
       │                   └── RadBeam__ComputeStraightSegment (0x00659AC0)
       │
       └── BeamType==2 ─► RadBeam__DrawSineBeam (0x00659CA0)
                           ├── RadBeam__InitSineBeamGeometry (0x00659E30)
                           ├── RadBeam__UpdateSineTaper (0x00659EE0)
                           └── RadBeam__ComputeSineSegment (0x00659FF0)
```

---

## 10. Open Questions / Not Confirmed

1. **Field `+0x00` (leading int, init to 0)**: written once but never read as
   either a vtable or a typed field. Could be padding/alignment, a reserved
   vtable slot, or a dead legacy "flags" word.
2. **Field `+0x48/+0x4C` (IntensityFadeParams)**: setter exists and is used
   only in the RadEruption callsite. The exact semantic role during
   `DrawSineBeam` was not individually traced.
3. **Type-2 color branch (`Rules+0x1869`)**: no live callsite sets beam_type=2
   in stock YR Fire_At. This branch is reachable only if a weapon sets both
   `IsRadBeam=yes` and some other flag that drives beam_type past 1 — I did
   not enumerate all possible callers. Likely TS-dormant.
4. **`DAT_00839530` and `DAT_007E5F30`**: the amplitude and period constants
   used inside `RadBeam__ComputeSineSegment` were observed but their numeric
   values were not individually verified from memory — they are lazy-init
   floats set up at game startup.
5. **RadEruption is not used by any stock YR weapon**: `IsRadEruption=yes` does
   not appear in `rulesmd.ini` outside example/commented sections. The
   `SpawnRadEruption` path is almost certainly TS-dormant but is still alive
   (reachable if a mod sets the flag).
6. **Threshold RNG for type-2 skip in `DrawAndTickAll`**: the formula
   `cubed(DAT_00ABCD44 / DAT_00A8B558)` was observed in the counting loop but
   its meaning (load-adaptive culling?) was not verified.

---

## 11. TS-Legacy Risks

- **RadEruption (3×3 random-beam burst)**: the `SpawnRadEruption` code path
  exists and compiles cleanly, but no stock YR weapon uses `IsRadEruption=yes`.
  It is TS-era or Ares-era only. If implementing from scratch, skip until a
  real weapon triggers it — treat as a potential TS dormant feature per
  CLAUDE.md guidance.
- **BeamType 2 via `TechnoClass__SpawnRadBeam`**: the code writes
  `RadBeam__SetBeamType(1)` inside the `beam_type == 2` branch (which looks
  non-sensical in isolation). Paired with the type-2 `Rules+0x1869` color
  slot that has no known INI parser, this is a strong indicator that type 2
  came from TS and is dead in YR. Flag anything that instantiates beam_type=2
  as suspect.
- **Type-2 load-adaptive culling**: tuned for TS-era frame budgets, may not
  kick in at all in typical YR play.
- The class itself is **live** for Desolator and Chrono Legion; both weapons
  use `IsRadBeam=yes` in stock `rulesmd.ini`.

---

## 12. Ghidra Functions Labeled in This Session

| Address | New name |
|---------|----------|
| `0x00659110` | `RadBeam__Allocate` |
| `0x006593F0` | `RadBeam__Constructor` |
| `0x00659470` | `RadBeam__SetColor` |
| `0x00659490` | `RadBeam__SetSourceZAdjust` |
| `0x006594A0` | `RadBeam__SetSourceCoord` |
| `0x006594C0` | `RadBeam__SetTargetCoord` |
| `0x006594E0` | `RadBeam__SetDuration` |
| `0x006594F0` | `RadBeam__SetAmplitudeAndPeriod` |
| `0x00659510` | `RadBeam__SetIntensityFade` |
| `0x00659530` | `RadBeam__SetIntensity` |
| `0x00659540` | `RadBeam__SetBeamType` |
| `0x00659550` | `RadBeam__SetRetainEndpoint` |
| `0x00659650` | `RadBeam__DrawStraightBeam` |
| `0x00659AC0` | `RadBeam__ComputeStraightSegment` |
| `0x00659CA0` | `RadBeam__DrawSineBeam` |
| `0x00659E30` | `RadBeam__InitSineBeamGeometry` |
| `0x00659EE0` | `RadBeam__UpdateSineTaper` |
| `0x00659FF0` | `RadBeam__ComputeSineSegment` |
| `0x006591B0` | `RadBeam__DrawAndTickAll` |
| `0x006FD620` | `TechnoClass__SpawnRadBeam` |
| `0x006FD800` | `TechnoClass__SpawnRadEruption` |

Saved via `save_program` at end of session.

---

**Verified 2026-04-21** from gamemd.exe (image base 0x00400000, build 2026-03-15)
via Ghidra MCP: allocator + ctor decompile, both draw-path dispatch functions,
all 9 setters, both spawn-path callers from `TechnoClass::Fire_At`, the
`TacticalClass::Draw` tick-order context, `WeaponTypeClass::ReadINI` for
`IsRadBeam` offset, `RulesClass::ReadRadiation` for `RadColor`, and
cross-check against `ini/rulesmd.ini` for live `IsRadBeam=yes` weapons.

---

## Follow-up investigation (round 2, 2026-04-21)

### Q4 — Is the sine-beam branch (BeamType=2) LIVE or TS-DORMANT in YR? — **RESOLVED**

**Answer: Technically reachable in live YR code, but stock YR has ZERO
weapons that trigger it. Treat as DORMANT BY DEFAULT (mod-opt-in only).**

Key findings from the dataflow trace:

1. **`RadBeam__DrawAndTickAll` dispatches on field `+0x10` (BeamType)**,
   NOT on field `+0x54`. Re-reading the constructor confirms this: the
   ctor writes its `param_2` arg into `param_1[4]` which is byte offset
   0x10. The `+0x54` field (written by `RadBeam__SetBeamType`) is a
   *separate* sub-shape flag that the dead type-2 path in
   `TechnoClass__SpawnRadBeam` overwrites to `1`. **The original report
   documenting `+0x54` as BeamType was imprecise** — dispatch is on +0x10.

2. **`RadBeam__SetBeamType @ 0x00659540` is only ever called from
   `TechnoClass__SpawnRadBeam`**, and only in the dead `beam_type == 2`
   branch. This setter writes `+0x54 = 1` (not 2), so it never drives the
   draw dispatch.

3. **`RadBeam__Allocate @ 0x00659110` is called with constant argument 2
   only from `TechnoClass__SpawnRadEruption @ 0x006FD800`.** Confirmed via
   direct disassembly: `0x006FD929: MOV ECX, 0x2` → `CALL 0x00659110`.
   That's the ONLY path that creates a BeamType=2 (sine) RadBeam in live
   YR code.

4. **`TechnoClass__SpawnRadEruption` is called from
   `TechnoClass__Fire_At` in the `WeaponType+0x155` (`IsRadEruption`)
   branch** — verified in the Fire_At decompile. So the call chain is
   live:
   ```
   Fire_At → [WeaponType+0x155 == IsRadEruption] → SpawnRadEruption
          → RadBeam__Allocate(2) → ctor sets BeamType=2
          → DrawAndTickAll dispatches to DrawSineBeam
   ```

5. **Stock YR has ZERO weapons with `IsRadEruption=yes`.** Grep of
   `ini/rules.ini` and `ini/rulesmd.ini` finds the only definition:
   ```
   [RadEruptionWeapon]
   ...
   IsRadEruption=no ; SJM: we're not using this effect anymore
   ```
   Westwood developer SJM explicitly disabled this weapon in stock YR.
   The `RadEruptionWeapon` itself is still referenced as a secondary for
   some obsolete unit, but the beam effect is opt-out by default.

**Implication for the Rust port:**

- The SINE path (`DrawSineBeam`, `InitSineBeamGeometry`, `UpdateSineTaper`,
  `ComputeSineSegment`) is reachable code in gamemd.exe, but NO STOCK
  WEAPON exercises it.
- A faithful Rust engine should **not implement BeamType=2** on the
  initial pass. If mod support becomes a goal later, the sine path can
  be added with `IsRadEruption=yes` as the trigger flag.
- `DrawStraightBeam` (BeamType=1) is the only path that needs to work for
  Desolator/Chrono Legion parity.

### Q5 — Community keys absence confirmation — **RESOLVED**

Re-ran `search_strings` for each candidate key:

| Key | Hits in gamemd.exe |
|-----|--------------------|
| `RadBeamAmplitude` | 0 |
| `RadBeamColor`     | 0 |
| `RadOuterSpread`   | 0 |
| `RadBeamPeriod`    | 0 |
| `RadBeamDuration`  | 0 |
| `DurationMultiple` | 0 (only `RadDurationMultiple`, see below) |
| `OuterSpread`      | 0 (only `LaserOuterSpread`, laser-only) |

Only strings starting with `RadBeam` in the entire binary:
- `.?AV?$VectorClass@PAVRadBeam@@@@` (RTTI)
- `.?AV?$DynamicVectorClass@PAVRadBeam@@@@` (RTTI)
- `IsRadBeam` (WeaponType flag)

**`RadDurationMultiple`** (at `0x0083B330`) exists but is part of the
**Radiation gameplay system** (`RulesClass__ReadRadiation @ 0x0066CFA9`,
writes to `RulesClass + 0x1804`), controlling how long radiation clouds
linger on the ground — **unrelated to the RadBeam visual**. It does NOT
drive beam duration (which is hardcoded to 15 ticks in
`TechnoClass__SpawnRadBeam`).

**Confirmed: RadBeam has NO tuning INI keys whatsoever in stock YR.** The
only knobs are:
- `[WeaponType] IsRadBeam=` (WeaponType+0x154) — enable/disable.
- `[WarheadType] Temporal=` (WarheadType+0x15A) — flips color (green→blue).
- `[Radiation] RadColor=R,G,B` (RulesClass+0x1830) — source of the
  green-beam color for BeamType=1 when warhead has Temporal=no.

The Ares/community extensions `RadBeamAmplitude`, `RadBeamColor`,
`RadOuterSpread`, `RadBeamPeriod`, `RadBeamDuration` are NOT parsed by
stock gamemd.exe. Hardcoded values inside `TechnoClass__SpawnRadBeam`:
- Duration = 15 frames (passed to `RadBeam__SetDuration`).
- Amplitude = 3.0f (passed to `RadBeam__SetAmplitudeAndPeriod`).
- Period = 0 (amplitude-and-period pair).
- Step size = 10 or 20 leptons (via `RadBeam__Allocate` param).

### Correction to the original struct table

Based on the ctor re-decompile and the dispatch evidence above:

| Offset | Field name (old) | Field name (corrected) |
|-------:|------------------|------------------------|
| 0x10   | `BeamType`       | `BeamType` (dispatch key: 1=straight, 2=sine) |
| 0x54   | `BeamType2` alias| `RetainEndpointModifier` — set to 1 only in the dead type-2 `SpawnRadBeam` branch; does NOT steer `DrawAndTickAll` |

The `RadBeam__SetBeamType` function name is retained but be aware it
writes to the secondary `+0x54` field, not the dispatch field at `+0x10`.

### Notes on label changes

No new RadBeam-related labels applied in this pass (all relevant
functions were already named in the prior session). The single new label
added during this round — `RulesClass__ReadCombatDamage @ 0x0066BBB0`
— is documented in the sibling EBolt report (Q1).

`save_program` called at end of session.
