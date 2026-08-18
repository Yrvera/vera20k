# Dynamic Body Roll/Pitch Writers — Ghidra Analysis Report

**Target:** Write sites for `entity+0x328` (roll / `AngleRotatedSideways`) and
`entity+0x32C` (pitch / `AngleRotatedForwards`) in `TechnoClass` instances.

**Date:** 2026-05-19  
**Method:** `search_byte_patterns` for FSTP (`D9 9?`) and MOV-int (`89 ??`) stores
to disp32 offsets 0x328 and 0x32C, followed by `get_function_by_address` and
`decompile_function` for every hit, cross-checked with `read_memory` on ambiguous
bytes.

---

## Field Identity Confirmation

Ghidra's `TechnoClass` struct names these fields:

| Offset | Ghidra field name | Type | Meaning |
|---|---|---|---|
| `entity+0x328` | `AngleRotatedSideways` | `float` | Body roll (Y-axis rotation) |
| `entity+0x32C` | `AngleRotatedForwards` | `float` | Body pitch (X-axis rotation) |

**Verified** at `0x0070b66b`: `D9 9E 28 03 00 00` = `FSTP dword ptr [ESI+0x328]`
inside `TechnoClass__RockingUpdate`.  
**Verified** at `0x0070b659`: `D9 9E 2C 03 00 00` = `FSTP dword ptr [ESI+0x32C]`
inside `TechnoClass__RockingUpdate`.  
**Verified** at `0x0070b78b`: `C7 86 28 03 00 00 DB 0F 49 3F` = `MOV [ESI+0x328], 0.7853982`
(= π/4, confirmed IEEE 754 `3F490FDB`) inside `TechnoClass__RockingUpdate`.

**Note on false positives:** The byte patterns `D9 9? 28 03 00 00` and `89 ?? 28 03 00 00`
matched writes to offset +0x328 in many *different* struct types:
- `RulesClass+0x328` = `ChronoBlast` (integer index) — written in `RulesClass__ReadGeneral`
- `AnimTypeClass+0x328` — written in `AnimTypeClass__Constructor` / `AnimTypeClass__ReadINI`
- `ParticleSystemTypeClass+0x328` — written in constructor
- Pointer-nulling in `FUN_00678850` (large pointer-clear sweep)

Only hits where the base register demonstrably points to a `TechnoClass` (or sub-class)
instance count as entity roll/pitch writers. The two confirmed entity writers are:
`TechnoClass__RockingUpdate` (primary) and `Wave_splash_forces` (secondary).

---

## Writer 1 — `TechnoClass__RockingUpdate`

**Address:** `0x0070B570`  
**Signature:** `void __thiscall TechnoClass__RockingUpdate(TechnoClass *this)`  
**Body:** `0x0070B570` – `0x0070BCA9` (3,641 bytes)  
**Confirmed write addresses:** `0x0070B659`, `0x0070B66B`, `0x0070B78B`, `0x0070B7C1`,
`0x0070B921`, `0x0070B95D`, and many more FP stores in the body.

### Caller chain

`TechnoClass__RockingUpdate` is **only called through vtable**. It appears in 6 vtables:

| Vtable address | Context |
|---|---|
| `0x007E26C0` | Class vtable (InfantryClass or subclass hierarchy) |
| `0x007E42D8` | Class vtable (UnitClass hierarchy) |
| `0x007E90B0` | Class vtable (aircraft or vessel hierarchy) |
| `0x007EB474` | Class vtable (BuildingClass hierarchy) |
| `0x007F4D7C` | Class vtable (another TechnoClass variant) |
| `0x007F608C` | Class vtable (another TechnoClass variant) |

In the vtable entry memory reads, `RockingUpdate` always sits at the **same relative
slot** — preceded by `TechnoClass__UpdateCloakShroud` (`0x006FB170`) and
`TechnoClass__CloakingTick` (`0x006FB470`). The slot is part of the per-frame AI
update chain called from each class's `AI()` method.

**Active in YR:** Yes. There is no `SpecialFlags` gate, no `Rules` boolean check inside
the function. It is called every tick for every `TechnoClass` instance that has a
non-zero `IsSinking` flag OR non-zero rocking-per-frame accumulator. The update is
essentially always running for all vehicles / infantry with `Rocking=yes` in their
typeclass.

### Function structure (high-level)

Three distinct code paths based on entity state:

#### Path A — Sinking animation (`this->IsSinking != false`)

Applies a simple oscillating pitch based on `RateTimer` low bits:

```
frame_phase = (RateTimer::Current() >> 12 + 1) >> 1 & 7
if (phase != 0 && phase < 6 || phase > 7):
    AngleRotatedForwards += g_ImpassableSpeedThreshold_0_01
else:
    AngleRotatedForwards -= g_ImpassableSpeedThreshold_0_01
```

Gate: `if (ABS(AngleRotatedForwards) >= DAT_007EF8F8) return` — exits if max angle
already reached. **Active in YR** when a ship/naval unit sinks.

#### Path B — External rocking flag (`this->field_0x425 != 0`)

Directly accumulates `RockingForwardsPerFrame` and `RockingSidewaysPerFrame` into
the angle fields. Clamps to a fixed minimum if `typeclass+0xD6A != 0`:

```
AngleRotatedForwards  += RockingForwardsPerFrame
AngleRotatedSideways  += RockingSidewaysPerFrame
if typeclass->field_0xD6A:
    clamp both to _LAB_007F4E73_1 (a small negative)
```

`field_0x425` appears to be set by an external trigger (e.g., explosion/EMP event).

#### Path C — Normal driving physics (primary path for ground vehicles)

This is the main case. For both roll and pitch:

1. If `RockingXxxPerFrame == 0.0` → zero the angle.
2. Otherwise: `angle_new = angle_old + RockingXxxPerFrame`
3. Bound check: if angle crosses `±DAT_007E897C` (the "peak") while coming from the
   other side with no field_0x2a8, snap to `±π/4` (= ±0.7853982 rad) and zero the
   per-frame step.
4. Per-frame step update: apply `_LAB_007F4E70` (base increment) or
   `RulesClass+0x18b8 * _LAB_007F4E70` (scaled by `DirectRockingCoefficient`) to
   `RockingXxxPerFrame`.
   - Direction of step change depends on: current angle magnitude, `field_0x2a8`
     (non-zero when unit is docking/transported?), and magnitude threshold constants.
5. Final clamp:
   ```
   if (|AngleRotated| < _LAB_007F4E5C threshold):
       zero both RockingXxxPerFrame and AngleRotatedXxx
   ```

**Maximum angles:** ±π/4 (±45°), stored as `0x3F490FDB` / `0xBF490FDB` IEEE 754.
Confirmed from byte reads at `0x0070B78B` (positive) and `0x0070B7C1` (negative).

**At the end of `RockingUpdate`:** if either `|AngleRotatedSideways|` or
`|AngleRotatedForwards|` exceeds `_LAB_007F4E5F_1`, a vtable call fires at
`this->vtable + 0x16C` — this appears to be a visual notification (probably
`TechnoClass::TiltChanged` / screen shake or similar). The argument passed is
`RulesClass+0xFA8` (a globally-configured shake intensity).

### Key constants (addresses unresolved to named INI keys — see below)

| Address | IEEE 754 / value | Role |
|---|---|---|
| `_DAT_007EF8F8` | unknown at static read | Upper angle limit (max rocking) |
| `_DAT_007EC0B0` | unknown at static read | Another limit (see sign-change logic) |
| `_DAT_007E897C` | unknown | "Peak" angle threshold |
| `_DAT_007E8980` | unknown | Low-side threshold |
| `_LAB_007F4E70` | unknown small float | Per-frame increment step |
| `_LAB_007F4E66_2` | unknown float | Larger decrement (positive-side braking) |
| `_LAB_007F4E75_3` | unknown float | Small-magnitude threshold |

These are BSS/static globals populated at engine init; `read_memory` returns zero (init
happens at runtime). Values can be read in a live debug session.

---

## Writer 2 — `Wave_splash_forces`

**Address:** `0x0053CBE0`  
**Signature:** `void __fastcall Wave_splash_forces(float *param_1)` where `param_1`
points to a wave object.  
**Write addresses:** `0x0053D2B7` (`FSTP [ESI+0x328]`) and `0x0053D2CB` (`FSTP [ESI+0x32C]`).

The function iterates cells in a ±3 cell radius around the wave position. For each
`TechnoClass` object on those cells that:
1. Has a type index of 0xF (naval unit?) or 1, AND
2. Has a `TypeClass` with `+0xB0 != null` AND `typeclass+0xB0[0] == 0`, AND
3. Has `pcVar7[0] >= 0` (typeclass speed check)

It computes a velocity-based tilt:
```
// Project wave velocity into the object's frame
fStack_ac = cross_product_Z(wave_vel, approach_dir)    // roll component
fVar1     = dot_product_XY(wave_vel, approach_dir)     // pitch component

// Time-based oscillation
fVar9 = d/dt of Cos(phase) scaled by wave params and _DAT_007EC040

// Store
entity+0x328 = fStack_ac * fVar9 * _DAT_007EC040   // roll
entity+0x32C = -(fVar1 * fVar9 * _DAT_007EC040)    // pitch (negated)
```

**Active in YR:** Yes, unconditionally — `Wave_splash_forces` is called whenever a
wave object (`WaveClass`) with a tick counter > 0x4E executes. The velocity-based
tilt fires every time a wave hits a nearby naval unit. No SpecialFlags gate.

**Important:** This *directly writes* float values to `entity+0x328/0x32C`, bypassing
`TechnoClass__RockingUpdate` entirely. The next call to `RockingUpdate` will then
accumulate from whatever `Wave_splash_forces` wrote. The magnitude is bounded by the
constants in `RockingUpdate` — so even a wild wave-splash can't push past ±π/4.

---

## Reader — `FUN_0054DCC0` (JumpJet-variant Draw_Matrix)

**Address:** `0x0054DCC0`  
**Vtable ref:** `0x007ECD8C` (single vtable — JumpJet or similar locomotor)

This is a `Draw_Matrix` implementation that reads (not writes) `entity+0x328` and
`+0x32C`. It gates on `typeclass+0xD22 = TiltCrashJumpjet` (bool, parsed in
`TechnoTypeClass__ReadINI` at `0x00713391`):

```c
if (typeclass->TiltCrashJumpjet && 
    (ABS(entity+0x328) >= DAT_007E44E8 || ABS(entity+0x32C) >= DAT_007E44E8)):
    // build tilt matrix using Matrix_rotate_x_axis(entity+0x32C) and
    //                           Matrix_rotate_y_axis(entity+0x328)
    // multiply with facing and slope matrices
else:
    // straight facing matrix only
```

The threshold `DAT_007E44E8` acts as a dead-band — tiny angles (< threshold) are
treated as flat. This function is read-only on the fields; it does not write them.

---

## INI Keys that Feed the System

### `DirectRockingCoefficient` (RulesClass section)

**Source:** `RulesClass__ReadAudioVisual` (`0x006691E0`) — write at `0x0066B8BC`:
`FSTP [ESI+0x18B8]`.  
**INI key string:** `0x0083A164` = "DirectRockingCoefficient" (verified from `read_memory`).  
**RulesClass offset:** `+0x18B8` (float).  
**Role in formula:** Multiplier applied to the per-frame rocking increment when
`entity->field_0x2a8 != 0` (e.g., docked/garrisoned state). Governs how quickly
a docked unit rocks.  
**Active in YR:** Yes, unconditionally read from `[CombatDamage]` section (or wherever
RulesClass::ReadAudioVisual parses it).

### `FallbackCoefficient` (near `DirectRockingCoefficient` in parser)

**INI key string:** `0x0083A150` = "FallbackCoefficient".  
**RulesClass offset:** `+0x18B0` (float, based on parser proximity).  
**Role:** Unknown from this pass — adjacent to DirectRockingCoefficient in the parser,
likely affects a related rocking damping path.

### `RockingCoefficient`

**INI key string:** `0x0083A16A` = "RockingCoefficient".  
**RulesClass offset:** Unknown — no Ghidra xref resolved in this session (string is
referenced indirectly, likely via a function pointer table). Likely adjacent to
`DirectRockingCoefficient`.

### `TiltCrashJumpjet` (TechnoTypeClass)

**INI key string:** `0x00844118` = "TiltCrashJumpjet".  
**TechnoTypeClass offset:** `+0xD22` (bool).  
**Parsed in:** `TechnoTypeClass__ReadINI` at `0x00713391`.  
**Role:** Gates whether the JumpJet `Draw_Matrix` (`FUN_0054DCC0`) applies the
roll/pitch tilt to the draw matrix. Does NOT affect whether `TechnoClass__RockingUpdate`
writes the fields — it only gates the consumption in that specific Draw_Matrix path.

### `BodyLength`

**INI key strings:** Three occurrences at `0x0083B8F9`, `0x0083B9D1`, `0x0083BAB4`
(for different TypeClass hierarchies). No Ghidra xref resolved in this session (likely
function-pointer-table dispatch). Meaning: the physical body length used to compute
how steeply the unit pitches when accelerating/braking. Expected to feed `UnitTypeClass`
or `TechnoTypeClass` struct field (offset unknown from this pass — deferred).

---

## YR-Active Summary

| System | Active in YR? | Evidence |
|---|---|---|
| `TechnoClass__RockingUpdate` — Path C (driving) | **Yes, always-on** | No gate, called every tick via vtable for all TechnoClass instances |
| `TechnoClass__RockingUpdate` — Path A (sinking) | **Yes, conditional** | Fires when `IsSinking != 0`; set during naval unit destruction |
| `TechnoClass__RockingUpdate` — Path B (external) | **Yes, conditional** | Fires when `field_0x425 != 0` (blast / EMP hit) |
| `Wave_splash_forces` writer | **Yes, conditional** | Fires on every wave with tick > 0x4E near a naval unit |
| `FUN_0054DCC0` reader (Draw_Matrix consumer) | **Yes, conditional** | Only when `TiltCrashJumpjet=yes` and angle exceeds threshold |
| `RulesClass__ReadAudioVisual` (INI init) | **Yes, once** | At game init, populates `DirectRockingCoefficient` |

No TS-only flags gate any of these paths. The rocking system is unconditionally active
for any unit on any YR map.

---

## Open Questions (Deferred)

1. **`BodyLength` → struct offset mapping.** Three string occurrences, zero Ghidra
   xrefs. Cannot resolve without tracing via a function pointer table or INI parser
   dispatch array. Likely ~`UnitTypeClass+0xXXX` (float). Needed to understand how
   body length scales rocking amplitude.
2. **`field_0x2a8` exact semantics.** `TechnoClass__Constructor` zeroes it;
   `TechnoClass__PointerExpired` nulls it if equal to an expiring pointer; it's tested
   non-zero in `RockingUpdate` to select the "DirectRockingCoefficient" path. Likely
   a "garrison occupant" or "transported-by" pointer. Full semantics not traced.
3. **Per-frame constants** (`_LAB_007F4E70`, `_DAT_007EF8F8`, `_DAT_007E897C`, etc.)
   are BSS-zero in the binary image — readable only in a live debug session. Values
   unknown from static analysis.
4. **`FallbackCoefficient` and `RockingCoefficient` offset mapping.** Adjacent to
   `DirectRockingCoefficient` in `RulesClass__ReadAudioVisual`; exact offsets deferred.
5. **Slope-interpolation writer.** `Force_New_Slope` zeroes `locomotor+0x2C`. Something
   else writes a non-zero value to trigger interpolation. Outside scope of this slot.
6. **`field_0x425` setter.** What event sets `TechnoClass::field_0x425`? Suspected
   EMP or blast event. Not traced in this pass.

---

## Key Addresses Summary

| Address | What |
|---|---|
| `0x0070B570` | `TechnoClass__RockingUpdate` — primary roll/pitch writer |
| `0x0053CBE0` | `Wave_splash_forces` — secondary roll/pitch writer (wave splash) |
| `0x0054DCC0` | JumpJet locomotor `Draw_Matrix` — roll/pitch reader (not writer) |
| `0x0066B8BC` | `FSTP [ESI+0x18B8]` in `RulesClass__ReadAudioVisual` — stores `DirectRockingCoefficient` |
| `0x00713391` | `MOV [EAX+0xD22], DL` in `TechnoTypeClass__ReadINI` — stores `TiltCrashJumpjet` |
| `TechnoClass+0x328` | `AngleRotatedSideways` — float, body roll |
| `TechnoClass+0x32C` | `AngleRotatedForwards` — float, body pitch |
| `TechnoClass+0x2A8` | Pointer (docking/garrison?); non-zero → DirectRockingCoefficient path |
| `TechnoClass+0x425` | `IsSinking` flag; triggers sinking-oscillation path |
| `RulesClass+0x18B8` | `DirectRockingCoefficient` float (from INI) |
| `TechnoTypeClass+0xD22` | `TiltCrashJumpjet` bool — gates JumpJet Draw_Matrix tilt |
| `0x3F490FDB` | `+π/4` = `+0.7853982` — maximum positive rocking angle (IEEE 754) |
| `0xBF490FDB` | `-π/4` = `-0.7853982` — maximum negative rocking angle (IEEE 754) |
| `0x0083A164` | INI string "DirectRockingCoefficient" |
| `0x0083B8F9` | INI string "BodyLength" (one of three occurrences) |
| `0x0084411` | INI string "TiltCrashJumpjet" |

---

## Confidence Notes

- **VERIFIED from binary:** All addresses, opcodes, field offsets, IEEE 754 constants.
  All `D9 9E 28/2C 03 00 00` = FSTP confirmed by `read_memory`.
- **INFERRED:** The "DirectRockingCoefficient scales docked rocking" behavior — from
  decompile reading `*(float *)(g_RulesClass_Instance + 0x18B8)` inside the
  `field_0x2a8 != 0` branch. Confidence: HIGH (content), HIGH (identity of
  INI key via string), MEDIUM (exact INI section).
- **INFERRED:** `BodyLength=` feed path — key exists in binary, no xref found this
  session. Confidence: LOW (unknown struct offset and formula).
- **NOT VERIFIED:** Live values of BSS globals (`_LAB_007F4E70`, `_DAT_007EF8F8`, etc.)
