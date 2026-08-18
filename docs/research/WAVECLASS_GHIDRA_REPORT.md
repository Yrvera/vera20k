# WaveClass — Ghidra Research Report

**Primary Address:** `0x0075E950` (full constructor), `0x0075EBE0` (default constructor)
**Allocation Size:** `0x240` (576 bytes) — `operator_new(0x240)` at every callsite in `TechnoClass::Fire_At`
**Vtables:** 4 separate vtables (`vtable__WaveClass`, `vtable__WaveClass__secondary_4/_8/_12`)
**Confidence:** HIGH for constructor signature, parameter mapping, two callsites in `TechnoClass::Fire_At`, the wave-type 0/3 selector, the `0x14`-byte point-list subobject, and the lookup-table init at `0x0075F020`. MEDIUM for the precise field naming inside the geometry helpers (`FUN_00761640`/`FUN_00762070`) — vertex math is decoded but exact rendering semantics (ramp index, color schema) need a separate Draw-pass decode.
**Active in YR:** Yes — instantiated by every weapon firing path with `IsSonic`, `IsLaser`, `IsBigLaser`, `IsRadBeam`, or `IsMagBeam` set on the WeaponTypeClass.

## 1. Overview

WaveClass is the runtime sprite-quad effect spawned by **special draw weapons** —
sonic frequency, prism/laser beams, magnetron beams, radiation beams. It is NOT a
projectile (no flight, no damage application of its own); it is a short-lived visual
effect class that owns 1–4 textured quads stretched along the beam vector and ticked
each frame to animate fade/pulse.

WaveClass extends `ObjectClass` (constructor calls `ObjectClass::Constructor()` first).
It registers itself in a global wave list at `DAT_00A8EC3C` (capacity stored at
`DAT_00A8EC40`, count at `DAT_00A8EC48`) so that the world-update tick can iterate
all live waves and call their per-tick AI.

**Two callsites** in `TechnoClass::Fire_At` (`0x006FF470`):
1. **Line ~6FF460**: triggered when `WeaponTypeClass+0x130 != 0` (corresponds to
   `IsLaser=yes` / `IsBigLaser=yes` / `IsSonic=yes` / `IsRadBeam=yes` family — all
   the "main beam" variants). Constructs with **`WaveType=0`**.
2. **Line ~6FF647**: triggered when `WeaponTypeClass+0x15c != 0` AND no existing wave
   on the firing technocraft. Constructs with **`WaveType=3`**.

The `WaveType` (`+0xB0`) selects which geometry helper runs at construction
(`FUN_00761640` for type 0/1/2, `FUN_00762070` for type 3) and which lookup table of
quad-corner offsets is used (`DAT_00B45DA8` vs `DAT_00B45CA0`).

## 2. Constructor Signature

**Full constructor (0x0075E950):**

```c
WaveClass* __thiscall WaveClass__Constructor(
    WaveClass* this,        // ECX, allocated by operator_new(0x240)
    CoordStruct* fromCoord, // beam start (3D world leptons)
    CoordStruct* toCoord,   // beam end (3D world leptons)
    TechnoClass* owner,     // firing unit (color/palette source)
    int waveType,           // 0..3 — selects geometry helper + LUT
    AbstractClass* target   // target object (used for shroud/range checks at use time)
);
```

**Default constructor (0x0075EBE0):** zero-arg form used by save-game loader. Sets
all fields to zero/sentinels, installs vtables, allocates the 0x14-byte point-list
subobject, registers in the global wave list, and lazy-initializes the shared
distance/cosine LUTs via `FUN_0075F020`. Skips geometry computation (deferred to
Load_Data).

**Verified callsites** in `TechnoClass::Fire_At` (`0x006FF470`):

```c
// Path A: WeaponType+0x130 set (main beam: IsLaser/IsSonic/IsRadBeam family)
this->Wave = WaveClass__Constructor(
    operator_new(0x240),
    &iStack_54,        // src coord (firing unit position)
    &uStack_98,        // dst coord (target position)
    this,              // owner = firing TechnoClass
    0,                 // waveType
    in_stack_00000004  // target object
);

// Path B: WeaponType+0x15c set (alternate beam shape)
this->Wave = WaveClass__Constructor(..., 3, ...);
```

The result is stored on the firing TechnoClass at `this->Wave` so the firer can
manage its lifetime and prevent stacking duplicate waves on the same shooter.

## 3. WaveClass Struct Layout

**Total size:** `0x240` (576 bytes). Fields below are **partial** — only those touched
by the constructor and the two geometry helpers were directly verified.

`param_1` in the constructor is `undefined4 *` so `param_1[N]` = byte offset `4*N`.

### Verified Fields

| Offset | Size | Type | Field | Init | Evidence |
|--------|------|------|-------|------|----------|
| +0x000 | 4 | ptr | vtable_main (`vtable__WaveClass`) | install at end of ctor | `*param_1 = ...` |
| +0x004 | 4 | ptr | vtable_secondary_4 | install at end | `param_1[1] = ...` |
| +0x008 | 4 | ptr | vtable_secondary_8 | install at end | `param_1[2] = ...` |
| +0x00C | 4 | ptr | vtable_secondary_12 | install at end | `param_1[3] = ...` |
| ... | | | (ObjectClass base fields, ~0xA8 bytes) | from `ObjectClass::Constructor` | not decoded here |
| +0x0AC | 4 | AbstractClass* | TargetObject | from param_6 | `param_1[0x2b] = param_6` |
| +0x0B0 | 4 | int | **WaveType** (0..3) | from param_5 | `param_1[0x2c] = param_5` |
| +0x0B4..0x12C | ? | (ObjectClass extension) | — | various zero inits | `param_1[0x2d..0x4a]` written by geometry helpers |
| +0x0B4 | 12 | CoordStruct | LocationFromCamera | written by helper | `param_1[0x2d/0x2e/0x2f]` (see §6) |
| +0x0C0 | 12 | int[3] | TimerOrZScratch | helper-local | `param_1[0x30/0x31/0x32]` |
| +0x0CC | 16 | int[4] | ScreenSpaceFrom (X,Y pair) + ScreenSpaceTo (X,Y pair) | `TacticalClass::CoordsToClient2` outputs | `param_1[0x33..0x36]` |
| +0x0DC | 32 | int[8] | ScreenSpaceQuadCorners (4 × (X,Y)) | helper writes via CoordsToClient2 | `param_1[0x37..0x3E]` |
| +0x0FC | 48 | int[12] | WorldSpaceQuadCorners (4 × (X,Y,Z)) | from `Math__ftol` of rotated points | `param_1[0x3F..0x4A]` |
| +0x12C | 1 | bool | IsActive (active flag) | 1 (true) | byte at `+0x12C` (`*(byte*)(param_1 + 0x4b) = 1`) |
| +0x12D | 1 | bool | (flag2) | 0 | byte at `+0x12D` |
| +0x130 | 4 | int | InitialStrength / Damage | 100 | `param_1[0x4c] = 100` |
| +0x134 | 4 | int | (?) | 0 | `param_1[0x4d] = 0` |
| +0x138 | 4 | int | (counter A) | 0 | `param_1[0x4e] = 0` |
| +0x13C | 4 | int | (?) | 0 | `param_1[0x4f] = 0` |
| +0x140 | 4 | int | (counter B) | 0 | `param_1[0x50] = 0` |
| +0x144 | 4 | int | (?) | 0 | `param_1[0x51] = 0` |
| +0x148 | 4 | int | **PointListCount** | 0 (default ctor); set to 6 (type 0/1/2) or 4 (type 3) by geometry helper | `param_1[0x52] = 6/4` |
| +0x14C | 4 | int* | **PointListPtr** (points into self at +0x150 or +0x180) | 0 (default); set to `param_1+0x54` or `param_1+0x60` by helper | `param_1[0x53] = ...` |
| +0x150 | 8 | (X,Y) | ScreenPoint #1 (used by type 0/1/2) | helper writes | inferred from PointListPtr |
| +0x180 | 8 | (X,Y) | ScreenPoint #1 (used by type 3) | helper writes | inferred |
| +0x1A0 | 4 | int | RenderHeight (?) | 0 (no helper) or 0xA0 if type 1/2 | `param_1[0x68/0x69/0x6A] = 0`; helper sets `+0x1A0 = 0xA0` for type 1/2 |
| +0x1D0 | 4 | int | (counter / scratch) | 0 | `param_1[0x74] = 0` |
| +0x1D4 | 4 | TechnoClass* | **OwnerLink** (firing unit, color/palette source) | from param_4 | `param_1[0x75] = param_4` |
| +0x1D8 | 24 | int[6] | OwnerColorRamp (copied from OwnerLink+0x3A0 by geometry helper — likely RGB ramp / `ColorScheme`-like) | helper copies 6 ints | `for (i=6; i--;) param_1[0x76+i] = OwnerLink[0x3A0+i]` |
| +0x1F0 | 4 | ptr | InternalAnimList_vtable_A | `&PTR_FUN_007ED480` | `param_1[0x7c] = ...` |
| +0x1F4 | 4 | void* | InternalAnimList_storage (operator_new(0x14) — 5 ints, capacity-5 list) | from `operator_new(0x14)` | `param_1[0x7d] = pvVar4` |
| +0x1F8 | 4 | int | InternalAnimList_capacity | 5 | `param_1[0x7e] = 5` |
| +0x1FC | 1 | bool | InternalAnimList_growable | 1 | `*(byte*)(param_1 + 0x7f) = 1` |
| +0x1FD | 1 | bool | InternalAnimList_owned | 0 → 1 after alloc | `*(byte*)(param_1 + 0x1FD) = 0` then `1` |
| +0x1F0 | 4 | ptr | (re-assigned vtable) `&PTR_FUN_007ED9BC` | post-alloc | `param_1[0x7c] = &PTR_FUN_007ED9BC` |
| +0x200 | 4 | int | RemainingFrames | 0 | `param_1[0x80] = 0` |
| +0x204 | 4 | int | TotalFrames | 10 | `param_1[0x81] = 10` |

The two-step vtable swap on the inner anim list (`PTR_FUN_007ED480` → `PTR_FUN_007ED9BC`)
is the standard YR pattern for a polymorphic VectorClass<T> that promotes from "no
storage" to "owns 5-cap dynamic array" once `operator_new(0x14)` succeeds.

### Outstanding Layout Gaps

- The fields between `+0xA8` (end of ObjectClass base) and `+0xAC` (first WaveClass
  field) need an `ObjectClass` overlay to fully resolve.
- Fields at `+0x148`+ relating to per-frame fade/pulse animation parameters were not
  individually mapped — the WaveClass per-tick AI function (a vtable slot) needs to
  be decoded to confirm.
- The fields at `+0x68..+0x6A` (zeroed in ctor) look like a 12-byte CoordStruct slot
  but no helper writes to them — possibly a per-tick offset scratch.

## 4. Wave Type Routing

```c
// In constructor, after geometry distance check passes:
if (param_1[0x2c] == 3) {           // WaveType == 3
    FUN_00762070(this, fromCoord, toCoord);
} else {
    FUN_00761640(this, fromCoord, toCoord);
}
```

Then **only for WaveType in {1, 2}** (NOT 0 and NOT 3):
```c
if ((0 < (int)param_1[0x2c]) && ((int)param_1[0x2c] < 3)) {
    // initialize secondary point-list at (param_1+0xD4 → +0xE0):
    *PointListPtr     = param_1[0x37];  // ScreenSpaceQuadCorner #1.X
    *(PointListPtr+1) = param_1[0x38];  // .Y
    *(PointListPtr+2) = param_1[0x35];  // ScreenSpaceTo.X
    *(PointListPtr+3) = param_1[0x36];  // .Y
    *(PointListPtr+4) = param_1[0x39];  // QuadCorner #2.X
    *(PointListPtr+5) = param_1[0x3a];  // .Y
    param_1[0x4e] = 0;        // counter A
    param_1[0x4f] = 0x3FF00000; // double-1.0 high bits → likely scale = 1.0
    param_1[0x74] = 0xA0;     // RenderHeight = 160 leptons (?)
}
```

This means **WaveType 0** (the most common — `IsLaser/IsSonic/IsRadBeam`) skips the
secondary-point setup entirely; it draws the 4-corner quad straight from the
geometry helper output. **WaveType 3** uses the smaller LUT and a different point
list at +0x180. **WaveTypes 1 and 2** are intermediate variants — likely Magnetron
Beam (1) and one other (2 = unknown, possibly an unused test type).

The mapping is consistent with the comment in the V3 cruise-missile section: the
ModEnc/community-known mapping (Sonic=0, Magbeam=1, Tesla=2, Special=3) is plausible
but not verified directly from the binary in this report.

## 5. Geometry Helpers (`FUN_00761640`, `FUN_00762070`)

Both helpers share the same algorithm; they only differ in:

| Aspect | FUN_00761640 (types 0/1/2) | FUN_00762070 (type 3) |
|--------|----------------------------|------------------------|
| Vertex LUT base | `DAT_00B45DA8` (4 corners × 4 variants) | `DAT_00B45CA0` (4 corners × 4 variants) |
| LUT init guard | `DAT_00B45DA0 & 1` | `DAT_00B45D88 & 1` |
| `+0x148 PointListCount` set to | 6 | 4 |
| `+0x14C PointListPtr` set to | `&this[0x150]` | `&this[0x180]` |

**Algorithm** (both):
1. Lazy-init the per-WaveType vertex LUT (4 quad corners × 12 floats each, indexed
   by `WaveType * 0x30`).
2. Convert from/to world coords through `Math__ftol` (no-op when already int).
3. If `WaveType == 0` or `WaveType == 3`, add `0x32` (50 leptons) to the wave's Z —
   probably "raise above terrain" so the beam doesn't clip into the ground.
4. Compute beam direction via `Sqrt_Approx(dx² + dy²)`.
5. Compute beam-rotation angle via `Acos_lookup(dy/length)` (signed by `dx` polarity).
6. Build a Z-rotation matrix via `Matrix3x4_SetIdentity` + `Matrix3x4_RotateZ`.
7. Transform each of the 4 base quad corners through the rotation
   (`Matrix3x4_TransformPoint`).
8. `Math__ftol` each transformed corner into world-space coords (stored at
   `+0xFC..+0x12B` as 12 ints = 4 × (X,Y,Z)).
9. Project each corner to screen space via `TacticalClass::CoordsToClient2`
   (stored at `+0xDC..+0xFB` as 8 ints = 4 × (X,Y)).
10. Copy 6 ints (24 bytes) from `OwnerLink+0x3A0` to `+0x1D8` — the firing unit's
    color ramp / palette indices for tinting.

Note that `param_1[0x52]` is set to 6 / 4 *before* the rotation math but the helper
only writes 4 corners — the count value is likely the **per-line vertex count**
including line cap segments, not the corner count.

## 6. Lookup-Table Init (`FUN_0075F020`)

Called once when `DAT_00B725CC == 0`. Builds three shared LUTs used by all
WaveClass instances during draw:

1. **Distance LUT** at `DAT_00B4669C`: 300×300 short table of
   `Sqrt_Approx(x² + y²)` — for very fast pixel-distance lookup during the
   per-frame line-drawing pass.
2. **Cosine LUT** at `DAT_00B46254`: ~250-entry table of
   `Cos_lookup(i * scale_A)` where `scale_A = _DAT_007F6DE0`.
3. **Cosine LUT** at `DAT_00B45E68`: ~250-entry table of
   `Cos_lookup(i * scale_B)` where `scale_B = _DAT_007E3860` (different scale).
4. **Brightness ramp** at `DAT_00B46648`: 18 ints from `0x6E` (110) to `0xCE`
   (206) in steps of 8 — palette brightness levels for the beam fade animation.

After init, sets `DAT_00B725CC = 1` so subsequent waves skip the rebuild.

## 7. Self-Registration in Global List

```c
// At the end of the constructor (both forms):
if (DAT_00A8EC40 <= DAT_00A8EC48) {       // capacity reached
    if (cant_grow_or_zero) goto skip;     // bail without registering
    grow_array(DAT_00A8EC4C + DAT_00A8EC40, 0);  // capacity += chunk
}
DAT_00A8EC3C[DAT_00A8EC48++] = this;      // append to global wave array
```

The world-update tick presumably iterates `DAT_00A8EC3C[0..DAT_00A8EC48]` and calls
each wave's `AI`/`Update` virtual. Waves remove themselves from the array when their
animation completes (mechanism not decoded in this report).

## 8. INI Bindings (Active Flags Triggering Wave Creation)

WaveClass is not directly INI-driven — it is created in response to the firing
weapon's flags. The flags that lead to `WaveClass__Constructor` calls in
`TechnoClass::Fire_At` are:

| WeaponTypeClass byte field | INI key | Triggers WaveType | Used by |
|---------------------------|---------|-------------------|---------|
| `+0x130` | `IsLaser=yes` / `IsBigLaser=yes` | 0 | Prism Tower, Mirage Tank, IFV Allied Engineer ramp |
| `+0x130` | `IsSonic=yes` | 0 | Sonic Tank, Sonic Frequency |
| `+0x130` | `IsRadBeam=yes` | 0 | Initiate radiation, Yuri Clone radbeam |
| `+0x15c` | (different beam variant) | 3 | (specific units — needs callsite verification) |

Note: the exact field-bit-mask within `+0x130` that distinguishes Laser vs Sonic vs
RadBeam is presumably checked by the **per-frame Draw** function to choose the
shader/palette, not by the `Fire_At` constructor call. All three flags route to the
same `WaveType=0` constructor.

## 9. Tiberian Sun Legacy Notes

- The full 4-vtable layout (main + 3 secondary at +0x4/+0x8/+0xC) is the standard
  YR pattern for an `ObjectClass` with multiple inherited interfaces; it is **not**
  TS-only and is active in YR.
- The vertex LUTs in `FUN_00761640` use 4 distinct "corner sets" each (16 floats × 4
  = 64 floats per WaveType). The `WaveType=2` slot is populated with valid data but
  no callsite for `WaveType=2` was found in `TechnoClass::Fire_At`. **WaveType 2 may
  be a TS-only wave variant** — confirm by xref-tracing `WaveClass__Constructor`
  callers comprehensively before implementing wave type 2.
- The default constructor at `0x0075EBE0` has the same registration logic but no
  geometry computation; this is the **save-game load path**, not a TS-only branch.

## 10. Open Questions / Unverified

- **WaveType enum**: only values 0 and 3 confirmed from `Fire_At` callsites. Values
  1 and 2 are referenced in the constructor's "if (0 < type < 3)" branch and have
  populated LUT data, but no instantiating callsite was decoded here. Need to
  follow `xrefs to WaveClass__Constructor` more broadly (look at
  `BulletClass::AI`, weapon-on-impact paths, `LightningStorm`, etc.).
- **Per-tick AI function**: not decoded. Lives in one of the 4 vtable slots at
  `0x007F4E00`-ish (vtable address inferred but not read). Need to find
  `vtable__WaveClass` data and follow slot ~7-9 for the `AI` virtual, then trace
  what RemainingFrames/TotalFrames does.
- **Field at `+0x1D8` (24 bytes copied from `OwnerLink+0x3A0`)**: assumed to be the
  unit's color ramp for beam tinting, but `TechnoClass+0x3A0` was not separately
  verified. Could also be a palette pointer + sub-fields. Need a TechnoClass struct
  pass to confirm.
- **Inner 5-element list at `+0x1F0`** (the polymorphic `operator_new(0x14)` block)
  is likely an `AnimClass*` list (5 sub-anims attached to the wave for sparkle/fade
  effects). Not decoded.
- **`+0x130` and `+0x15c` of WeaponTypeClass**: identified as the trigger flags but
  not separately decoded. Need a WeaponTypeClass field-map pass to confirm exact
  semantics (which is `IsLaser` vs `IsSonic` vs `IsRadBeam`).

---

**Verified 2026-04-19** from gamemd.exe (image base 0x00400000) via Ghidra MCP:
constructor decompile (full and default forms), `FUN_0075F020` decompile,
`FUN_00761640` decompile, `FUN_00762070` decompile, `TechnoClass::Fire_At` decompile
(both wave construction call sites identified), and `rulesmd.ini` cross-check at
lines 23291-24316 for the IsLaser/IsSonic/IsRadBeam flag set.
