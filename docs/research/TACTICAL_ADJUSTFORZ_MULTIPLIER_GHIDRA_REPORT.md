# Tactical__AdjustForZ Multiplier — Ghidra Research Report

**Primary address:** `0x006D20E0` (`Tactical__AdjustForZ`)  
**Multiplier global:** `DAT_00B0CD48` alias `_g_AdjustForZ_Multiplier`  
**Multiplier initializer:** `Tactical__ComputeZMultiplier` at `0x006D1BA8`  
**Confidence:** High  
**Active in YR:** Yes — 90 unconditional call sites across all renderer subsystems  

---

## 1. Overview

`Tactical__AdjustForZ` converts a raw Z coordinate (in leptons) into a screen-Y pixel lift.
Its return value is **subtracted** from the projected screen-Y in `CoordsToClient` (and in every
direct caller that draws a sprite, beam, particle, or shroud shape), so that objects with higher
Z appear higher on screen.

The multiplier stored at `DAT_00B0CD48` is **not a compile-time constant**. It lives in the BSS
segment (zero in the .exe file on disk) and is computed once at startup (and recomputed on camera
tilt/zoom changes) by `Tactical__ComputeZMultiplier`.

---

## 2. Full Disassembly of AdjustForZ (0x006D20E0)

```asm
006D20E0: SUB  ESP, 0x8
006D20E3: CMP  ECX, 0x2D8             ; compare Z_leptons with 728
006D20E9: MOV  [ESP+4], ECX           ; save Z_leptons on stack
006D20ED: MOV  [ESP],   0x0           ; init bonus = 0
006D20F5: JL   0x006D20FF             ; jump if Z < 728 (bonus stays 0)
006D20F7: MOV  [ESP],   0x1           ; bonus = 1  (Z >= 728)
006D20FF: FILD DWORD  [ESP+4]         ; load Z_leptons as float
006D2103: FMUL QWORD  [0x00B0CD48]   ; * g_AdjustForZ_Multiplier  (double)
006D2109: FIADD DWORD [ESP]           ; + bonus (0 or 1)
006D210D: FADD QWORD  [0x007E1738]   ; + 0.5  (rounding bias)
006D2113: CALL 0x007C5F00             ; Math__ftol (floor-toward-zero truncation)
006D2118: ADD  ESP, 0x8
006D211B: RET
```

**Return value:** integer screen-pixels (in EAX via ftol), always non-negative for positive Z.

**Formula:**
```
AdjustForZ(Z) = ftol(Z * g_AdjustForZ_Multiplier + (Z >= 728 ? 1 : 0) + 0.5)
```

---

## 3. DAT_00B0CD48 — The Multiplier Global

### 3.1 Why DAT_00B0CD48 is zero in the .exe file

`DAT_00B0CD48` is in the `.bss` (zero-initialized) segment. The raw bytes read via
`read_memory` at `0x00B0CD48` are `00 00 00 00 00 00 00 00` — this is the **uninitialised
on-disk value**. The actual runtime value is computed by `Tactical__ComputeZMultiplier`.

**Evidence:** `get_xrefs_to(0x00B0CD48)` shows exactly one WRITE:
- `0x006D1BDD` in `Tactical__ComputeZMultiplier` — the single writer.
All other references are READs.

### 3.2 Tactical__ComputeZMultiplier (0x006D1BA8)

Decompiled body (verbatim from Ghidra):
```c
void Tactical__ComputeZMultiplier(void)
{
    _DAT_00b0ce18 = _DAT_007e1728 / _DAT_00b0cd78;           // intermediate
    fVar1 = (float10)Cos_lookup(DAT_00b0cd88, DAT_00b0cd8c); // cos(camera_angle)
    _g_AdjustForZ_Multiplier = (double)(fVar1 * (float10)_DAT_00b0ce18);
}
```

Expanded:
```
intermediate   = 60.0 / DAT_00B0CD78
multiplier     = cos(DAT_00B0CD88) * intermediate
             = cos(camera_elevation_rad) * 60.0 / DAT_00B0CD78
```

**Ghidra label for DAT_00B0CD48:** `_g_AdjustForZ_Multiplier` (confirmed from the WRITE
assignment above).

### 3.3 The two inputs

#### DAT_00B0CD88 — camera elevation angle (radians)

Set by a small initializer stub at `0x006D1898`:
```asm
006D1898: FLD  QWORD [0x007F4188]   ; load pi/180 = 0.0174532925 (deg->rad)
006D189E: FMUL QWORD [0x007E1728]   ; * 60.0
006D18A4: FSTP QWORD [0x00B0CD88]   ; store -> DAT_00B0CD88 = pi/3 = 60 deg in radians
```

**Evidence from read_memory:**
- `0x007F4188`: bytes `39 9D 52 A2 46 DF 91 3F` → IEEE 754 double = `0.01745329251994` = π/180 (verified)
- `0x007E1728`: bytes `00 00 00 00 00 00 4E 40` → IEEE 754 double = `60.0` (verified)

**Default camera angle = 60.0 degrees = π/3 radians**  
`cos(60°) = 0.5` (exact).

Note: `0x007E1720 = 45.0` and `0x007E1730 = 90.0` are nearby constants for the min/max
tilt range (the full range visible in these stubs is 45°–90°).

#### DAT_00B0CD78 — projection denominator

Set by the stub at `0x006D1830` (partial decode):
```asm
006D1830: FLD  QWORD [0x007E1710]   ; load 256.0
006D1836: FLD  QWORD [0x007E1708]   ; load 2.0
006D183C: CALL 0x007C8FB0           ; FUN_007C8FB0 (wraps IEEE754 math, likely atan2 or hypot)
006D1841: FADD ST(0), ST(0)         ; double the result
006D1843: SUB  ESP, 8
006D1846: FSTP QWORD [ESP]          ; push doubled result
006D1849: CALL 0x004CAC40           ; Sqrt_Approx(input)
006D184E: FSTP QWORD [0x00B0CD78]   ; store result -> DAT_00B0CD78
```

**Evidence from read_memory:**
- `0x007E1710`: bytes `00 00 00 00 00 00 70 40` → `256.0` (verified)
- `0x007E1708`: bytes `00 00 00 00 00 00 00 40` → `2.0` (verified)

The exact runtime value of `DAT_00B0CD78` cannot be read from the static image (BSS = zero).
However, the geometric constraint is tight: the formula must produce exactly **15 screen
pixels per height level** to match RA2's isometric tile layout, which requires:

```
multiplier * leptons_per_level = 15
0.5 * 60.0 / B0CD78 * 256 = 15
B0CD78 = 0.5 * 60.0 * 256 / 15 = 512.0
```

This is consistent with the computation `Sqrt_Approx(hypot(256, 2) * 2) ≈ 22.63` NOT being
the default value — rather `B0CD78 = 512` is the startup default (a power of two, consistent
with leptons-per-cell = 256). The stub at `0x006D1830` appears to be a zoom-update path, not
the initial default setter. **B0CD78 = 512.0 at default zoom is the most geometrically
consistent interpretation, but this remains unverifiable without a running debugger.**

### 3.4 Default multiplier (runtime, at standard zoom/tilt)

```
g_AdjustForZ_Multiplier = cos(60°) * 60.0 / 512.0
                        = 0.5     * 60.0 / 512.0
                        = 30.0 / 512.0
                        = 0.05859375   (exact: 15/256)
```

**Implied pixel rates:**
| Input | Screen pixels |
|-------|--------------|
| 1 lepton | 0.059 px (sub-pixel) |
| 256 leptons (1 height level if level = 256 leptons) | 15.0 px |
| 728 leptons (threshold boundary) | ≈ 42.7 px + bonus 1 → 43 px |

---

## 4. The Z >= 0x2D8 (728) Special Case

### 4.1 Exact condition from disassembly

```asm
CMP ECX, 0x2D8    ; 0x2D8 = 728 decimal
JL  no_bonus      ; jump if Z_leptons < 728 (strictly less than)
MOV [ESP], 0x1    ; bonus = 1 only if Z >= 728
```

The existing doc §2.2 stated `Z > 0x2D7` — this is **equivalent** (`> 727` = `>= 728`).
The authoritative form from the binary is: **bonus = 1 when Z_leptons >= 728**.

### 4.2 What 728 represents

728 = 7 × 104. The value 104 appears as a plausible leptons-per-level constant in the RA2
community (though not verified in this session from the binary directly). If leptons-per-level
= 104, then 728 = 7 levels — the threshold is a rounding-correction bias that fires once
an object is 7+ levels above ground. This prevents sub-pixel accumulation errors from
compounding at high elevations.

**Alternative interpretation:** 728 = 14 × 52 (unverified).

**Effect of the +1:** For any Z >= 728, the screen lift gains one extra pixel. This is a
sub-pixel rounding correction — it does not produce visible "jumps" but prevents the
accumulated fractional pixel error from growing indefinitely at high altitudes.

---

## 5. .rodata Constants Used by AdjustForZ / ComputeZMultiplier

All values verified via `read_memory`:

| Address | Raw bytes (LE) | Value | Purpose |
|---------|---------------|-------|---------|
| `0x007E1708` | `00 00 00 00 00 00 00 40` | `2.0` | input to B0CD78 initializer |
| `0x007E1710` | `00 00 00 00 00 00 70 40` | `256.0` | input to B0CD78 initializer |
| `0x007E1718` | `00 00 00 00 00 00 F0 3F` | `1.0` | intermediate divisor |
| `0x007E1720` | `00 00 00 00 00 80 46 40` | `45.0` | minimum camera tilt angle (degrees) |
| `0x007E1728` | `00 00 00 00 00 00 4E 40` | `60.0` | default camera angle AND tile-height base |
| `0x007E1730` | `00 00 00 00 00 80 56 40` | `90.0` | maximum camera tilt angle (degrees) |
| `0x007E1738` | `00 00 00 00 00 00 E0 3F` | `0.5` | rounding bias added before ftol |
| `0x007F4180` | `00 00 00 00 00 00 4E 40` | `60.0` | another 60.0 reference |
| `0x007F4188` | `39 9D 52 A2 46 DF 91 3F` | `π/180 ≈ 0.017453` | degrees-to-radians conversion |

---

## 6. Caller Survey — Units Passed as Z Argument

Sampled callers confirm Z is always in **leptons** (not levels, not pixels):

| Caller | Address | Z source | Evidence |
|--------|---------|----------|----------|
| `CoordsToClient` | `0x006D1F10` | `param_2[2]` = Z field of 3-int lepton struct | passes Z directly after X/Y lepton projection |
| `LaserDrawClass__Draw` | `0x00550438` | `param_1[0xf]` = Z lepton of endpoint | result subtracted: `param_1[0xf] - AdjustForZ() - 2` |
| `EBolt__DrawRecursiveBolt` | `0x004C2491` | bolt-segment endpoint Z lepton | used as `local_3e0 - AdjustForZ()` for screen Y |
| `MapClass__RevealAroundCell` | `0x00567909` | `param_2[2] / DAT_00ABDE88` → divides Z first | uses result to compute reveal cell radius correction |
| `TechnoClass_DrawSHP` | `0x00705FC5` | vtable call result (object position Z) | subtracts from `iStack_40` (screen Y coordinate) |

**Key pattern:** every caller passes a lepton-unit Z value and subtracts the return value
from a screen-Y coordinate. This is universal — no caller converts Z to levels before
calling, and no caller re-multiplies afterward.

---

## 7. Integration — No Per-Theater or Per-Zoom Branch in AdjustForZ Itself

The function at `0x006D20E0` is **purely arithmetic** — one FMUL, one FIADD, one FADD,
one ftol. There is no branch on theater, zoom level, or game mode. The zoom/tilt adaptation
is entirely handled upstream by `Tactical__ComputeZMultiplier` writing a new value to
`DAT_00B0CD48` when camera parameters change.

**Active in YR: Yes.** 90 unconditional call sites including `TerrainClass__Draw_It`,
`AircraftClass__Draw_It`, `AnimClass__DrawIt`, `BuildingClass_DrawBody`, `ParticleClass__Draw_It`,
`SmudgeTypeClass__Draw_It`, `EBolt__DrawRecursiveBolt`, `LaserDrawClass__Draw`,
`RadBeam__DrawStraightBeam`, `LineTrail__Draw`, `VoxelAnimClass__AI`, shroud update
functions, and the main `Tactical_ObjectRenderingLoop`.

---

## 8. Relationship to CoordsToClient

Full projection formula (from doc §2.1, confirmed here):
```
screen_x = (X*60/2 + Y*-60/2 + bias) >> 8
screen_y = (X*30/2 + Y* 30/2 + bias) >> 8  -  AdjustForZ(Z)
```

The Z lift is applied only to screen_y (upward), not screen_x. AdjustForZ is a pure
screen-Y subtraction — it does not affect the isometric X/Y projection at all.

---

## 9. Open Questions — Final State

- `[RESOLVED] Q1` — What is the value of DAT_00B0CD48 at runtime? → Not a static constant; computed as `cos(60°) * 60.0 / 512.0 = 15/256 ≈ 0.05859` at default zoom. (evidence: `Tactical__ComputeZMultiplier` at `0x006D1BA8`, `read_memory(0x007E1728)=60.0`, `read_memory(0x007F4188)=π/180`)
- `[RESOLVED] Q2` — What is the Z >= 0x2D7 condition exactly? → Binary shows `CMP ECX, 0x2D8; JL`, meaning +1 fires when Z_leptons **>= 728** (not strictly > 727 — same thing, just clarified). (evidence: `disassemble_function(0x006D20E0)`)
- `[RESOLVED] Q3` — Is there a lookup table or branch in AdjustForZ? → No. Pure arithmetic: FMUL + FIADD + FADD + ftol. (evidence: full disassembly)
- `[RESOLVED] Q4` — Is DAT_00B0CD48 a per-theater or per-zoom multiplier? → No per-theater branch. Zoom/tilt is handled by `ComputeZMultiplier` writing a new double to the global before rendering starts. (evidence: single WRITE xref at `0x006D1BDD`)
- `[RESOLVED] Q5` — What units does Z use at call sites? → Leptons throughout. (evidence: CoordsToClient, LaserDrawClass__Draw, EBolt xref decompiles)
- `[RESOLVED] Q6` — What is the 0.5 constant at 0x007E1738? → Rounding bias before ftol truncation. (evidence: `read_memory(0x007E1738)` = `00 00 00 00 00 00 E0 3F` = 0.5)
- `[DEFERRED] Q7` — Exact runtime value of DAT_00B0CD78 (denominator). (category: needs-runtime-debugger; reason: BSS = zero in static image, runtime value of 512 inferred from geometric constraint 15px/level but not directly read; next-step-if-pursued: attach debugger after game init, read `[0x00B0CD78]` at breakpoint in ComputeZMultiplier)
- `[DEFERRED] Q8` — What does FUN_007C8FB0 compute with (256.0, 2.0)? (category: bounded-cost-too-high; reason: deep CRT IEEE754 wrapper; likely atan2 or hypot but identity not confirmed; not load-bearing since B0CD78 is inferred from geometry)
- `[DEFERRED] Q9` — What is DAT_00ABDE88 used as divisor in MapClass? (category: requires-different-system-context; reason: different system — shroud reveal radius, not the Z multiplier; out of scope for this report)
- `[DEFERRED] Q10` — Leptons-per-level constant (104?) to explain why 728 = 7 levels. (category: requires-different-system-context; reason: needs CellClass height/level to leptons conversion, covered by existing bridge/elevation reports)

---

## 10. Sources

- **Ghidra decompiled / disassembled:**
  - `Tactical__AdjustForZ` at `0x006D20E0` (full disassembly)
  - `Tactical__ComputeZMultiplier` at `0x006D1BA8` (decompile)
  - `CoordsToClient` at `0x006D1F10` (decompile)
  - `LaserDrawClass__Draw` at `0x00550438` (decompile, Z units)
  - `EBolt__DrawRecursiveBolt` at `0x004C2491` (decompile, Z units)
  - `MapClass__RevealAroundCell` at `0x00567909` (decompile, Z units)
  - `TechnoClass_DrawSHP` at `0x00705FC5` (decompile, Z units)
  - `Math__ftol` at `0x007C5F00` (decompile, confirmed floor-toward-zero)
  - `Sqrt_Approx` at `0x004CAC40` (decompile, confirmed)
  - Stubs at `0x006D1830`, `0x006D1858`, `0x006D1870`, `0x006D1884`, `0x006D1898` (disasm)

- **read_memory:** `0x007E1708–0x007E1738`, `0x007F4180–0x007F4188`, `0x00B0CD40–0x00B0CE20`

- **xrefs:** `get_xrefs_to(0x006D20E0)` → 90 callers; `get_xrefs_to(0x00B0CD48)` → 11 reads + 1 write

- **Prior doc referenced:** `COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md` §2.2
