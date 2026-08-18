# House Color Remap Pipeline — Ghidra Report

**Primary addresses:**
`Init_Color_Schemes_INI 0x0066D3A0` (the `[Colors]` reader / doubler) ·
`ColorScheme__ctor 0x0068C710` · `ColorScheme__BuildRampPalette 0x0068C3B0` (the 16-shade ramp builder) ·
`HSV_to_RGB 0x00517440` · `Sin_lookup 0x004CACB0` · `Cos_lookup 0x004CAD00` · `Math__ftol 0x007C5F00` ·
`SessionClass__PriorityToColorScheme 0x0069A310` · `ScenarioClass__Create_Houses 0x00687F10` ·
`HouseClass__InitColor 0x50B840` · `HouseClass__ComputeRemap 0x50BA00` ·
`TechnoClass__SelectRemap 0x00705D70` · standard remap blitter `0x00491740` ·
`RadarClass__RenderCellPixel 0x00655C50` · `DSurface__Constructor 0x004BA9D6` (g_DD_* setter) ·
`ConvertClass palette→16bit table builder 0x004BBB00` · `VXL_BuildRemapTable 0x00758B70`.
Globals: `g_ColorSchemeArray 0x00B054D4` (count `0x00B054E0`) · priority LUT `0x0083ED14` ·
g_DD_* `0x008A0DD0..0x008A0DE4` · ComputeRemap consts `0x007E5F78 / 0x007EAA50 / 0x007EAA48`.

**Investigation Mode:** **Exhaustive-slice** for the ramp construction, ColorScheme layout,
priority→index flow, and ComputeRemap constants (all decoded + byte-verified this session).
**Coverage-map** for the live g_DD_* values and the ConvertClass `+0x174` LUT population (both
runtime-only / out of lane — see Coverage Ledger §7).

**Claimed Scope:** how each house's player color is chosen (priority → ColorScheme index),
how the 16-entry unit remap band is constructed, the full ColorScheme object layout, the
single radar/UI color path (InitColor + ComputeRemap), the DirectDraw 16-bit format
dependency, and the exact reproducible ComputeRemap constants/rounding.

**Non-Scope:** the ConvertClass `+0x174` 16-bit pixel-table byte layout population from a live
palette (runtime), the `Sqrt_Approx` mantissa LUT contents (`0x008650BC`), the exact live
g_DD_* shift/loss values (computed at surface creation), per-type `TypeClass+0xDF0` custom
scheme arrays, and the type-1 (non-player) scheme variant's downstream consumers.

**Overall Confidence:** HIGH for the ramp algorithm, ColorScheme layout, priority flow, and
ComputeRemap constants/rounding. MEDIUM for stock live format = RGB565 (derived, not read off a
live surface) and for the [Colors] decl-order → priority-name mapping (math-verified, names from INI).

**2026-07-27 correction:** a live identity audit disproved the earlier lookup
labels in this report. `0x004CACB0` is SINE and `0x004CAD00` is COSINE.
Consequently S uses the 50°→90° schedule, V uses the 20°→90° schedule, and the
shade-0 π/16 override belongs to the cosine/V path. Any older `cos-V 50→90 /
sin-S 20→90` wording is superseded by the corrected sections below. Native
`ftol` rounding at this ramp-constructor call remains UNCHECKED without a live
control-word trace.

**Active in YR:** **Yes** — every skirmish/MP match. `Init_Color_Schemes_INI` runs at rules load;
`Create_Houses`→priority LUT→InitColor/ComputeRemap runs once per house at scenario start; the
blit-time remap runs every techno draw every frame. No FogOfWar/SpecialFlags gate anywhere on this
path. The g_DD_* 16-bit conversion is Conditional (active only when the display surface is 16-bit,
the stock retail default).

---

## 1. Overview

gamemd does **not** build a 16-entry RGB "house ramp" from a single base color the way the
current Rust port does. The pipeline is:

1. **Rules load:** `Init_Color_Schemes_INI` reads every `[Colors]` entry (`Name=H,S,V`) and
   builds **two** `ColorScheme` objects per entry (the "all doubled" the INI comment warns about).
2. **Per-scheme construction:** each `ColorScheme` embeds its own 256-entry RGB palette
   (`+0x04`). At construction, `ColorScheme__BuildRampPalette` writes a **16-shade team-color band
   into palette indices 16..31** of that embedded palette, via a **fixed-hue Saturation/Value
   trig sweep** (S on a sine 50°→90°, V on a cosine 20°→90°, with shade-0
   cosine = π/16) through a 6-sextant integer HSV→RGB.
   Indices 0..15 and 32..255 are the base theater palette, untouched.
3. **House color assignment:** at scenario start, each house's player priority maps through a
   9-byte LUT (`PriorityToColorScheme`) to a scheme-array index, stored at `House+0x16054`.
4. **Single radar/UI color:** `InitColor` samples palette index **16** of that scheme (one color)
   into `House+0x56F9..0x56FB` (HouseColorRGB). `ComputeRemap` then normalizes that triple to
   vector-length 240 (with a 255 cap and a 96 floor) into `House+0x56FC..0x56FE` (a second color).
   Neither builds a 16-entry ramp.
5. **Draw-time unit remap:** at blit time, every techno-body pixel does
   `screen = convert_base[ remap_lut[ source_palette_byte ] ]`, where `remap_lut` is the
   ColorScheme's ConvertClass remap table (the embedded palette's band lives at indices 16..31).
   The source byte indexes the LUT **directly** — no `byte-16` arithmetic, no per-house RGB ramp.

The player-visible "team color" of a unit therefore comes from the ColorScheme's embedded
indices-16..31 band, applied as an 8-bit→8-bit palette substitution and then converted to the
display format by the one universal ConvertClass table — exactly like every other pixel.

---

## 2. ColorScheme Layout / Key Offsets

Object size **0x33C** (`operator_new(0x33c)` in the create-or-find helper). All offsets HIGH —
read from the constructor `decompile_function 0x0068c710` and cross-referenced where noted.

| Offset | Size | Field | Meaning | Evidence |
|---|---|---|---|---|
| `+0x000` | int | SelfIndex | array index, set to `DAT_00B054E0` on append | ctor 0x0068c710 tail |
| `+0x004` | 768B | EmbeddedPalette | 256×3 RGB; **unit band = indices 16..31** (bytes 0x34..0x5B of region) | ctor zero-loop 0x100×3; `BuildRampPalette 0x0068c3b0` writes i+16 |
| `+0x304` | char* | Name | strdup of `[Colors]` key (`0x007D5408`) | ctor; destructor frees; `FindColorSchemeIndex` compares |
| `+0x308` | u16 | H,S | base hue+sat word (`*param_3`) | ctor `*(this+0xC2)=*param_3` |
| `+0x30A` | u8 | V | base value byte (`param_3[1]`) | ctor `*(this+0x30A)=param_3[1]` |
| `+0x30C` | ptr | ConvertClass* | LightConvertClass built from the 768B palette + tint(1000,1000,1000); its 16-bit pixel table at `+0x04`, remap LUT at `+0x174` | ctor `=BuildRampPalette(...)`; `InitColor 0x50B840` reads it |
| `+0x310` | int | Type | **doubling discriminator**: `1` or `0x35`(53) | ctor `=param_6`; `FindColorSchemeIndex`/create-helper compare `==type` |
| `+0x314` | int | 0x10 (16) | unit-remap band start | ctor const |
| `+0x318` | int | 0x0F (15) | unit-remap band span (16 entries, 16..31) | ctor const |
| `+0x31C` | int | 0x19 (25) | palette-index anchor | ctor const |
| `+0x320` | int | 0x18 (24) | palette-index anchor | ctor const |
| `+0x324` | int | 0x16 (22) | palette-index anchor | ctor const |
| `+0x328` | int | 0x10 (16) | palette-index anchor | ctor const |
| `+0x32C` | int | 0x13 (19) | palette-index anchor | ctor const |
| `+0x330` | int | **0x10 (16)** | the single index `InitColor` samples for the radar/UI color | ctor `=0x10`; `InitColor` reads `scheme+0x330` |
| `+0x334` | int | 0x10 (16) | palette-index anchor | ctor const |
| `+0x338` | int | 0x15 (21) | palette-index anchor | ctor const |

Notes:
- `+0x330 = 16` is why the single radar/UI color = the scheme's palette **index 16 = the
  brightest unit shade** (HIGH — ctor const + `InitColor` cross-ref).
- The destructor `0x0068C8D0` frees only `+0x304` (name) and `+0x30C` (ConvertClass); the 768B
  palette is **inline** in the object, not a separate alloc (HIGH).
- The `+0x308` decompiler labeling shifts H/S/V; the **assembly is authoritative** in
  `BuildRampPalette`: H = byte 0, S = byte 1, V = byte 2 (`disassemble_function 0x0068c3b0`:
  `MOV BL,[ECX]; MOV AL,[ECX+1]; MOV DL,[ECX+2]`).

---

## 3. Core Logic

### 3.1 The 16-entry unit remap ramp — `ColorScheme__BuildRampPalette 0x0068C3B0` (HIGH)

Verified against `disassemble_function 0x0068c3b0`. Inputs: HSV triple ptr (ECX), dest palette
(= `scheme+0x04`), base theater palette (`DAT_00887308`), type, tint(1000,1000,1000).

Angle constants (decoded from `read_memory 0x007f0e80`, IEEE-754 LE doubles, radians):

| addr | value (rad) | degrees | role |
|---|---|---|---|
| `0x007F0E80` | 0.8726646260 | **50°** | sine base (drives S) |
| `0x007F0E88` | 0.0465421134 | **2.6667° = 40°/15** | sine per-step (S sweeps 50°→90°) |
| `0x007F0E90` | 0.3490658504 | **20°** | cosine base (drives V) |
| `0x007F0E98` | 0.0814486984 | **4.6667° = 70°/15** | cosine per-step (V sweeps 20°→90°) |
| i==0 override | 0.1963495408 | **11.25° = π/16** | cosine angle for shade 0 only |

```
H = hsv[0] ; S = hsv[1] ; V = hsv[2]           # bytes, assembly-confirmed order
copy 768B base theater palette into dest        # indices 0..15, 32..255 stay = base
for i in 0..=15:                                # CMP ESI,0x10; JL  → exactly 16 iterations
    sinAngle = i*(40°/15) + 50°                 # 50° → 90°
    cosAngle = i*(70°/15) + 20°                 # 20° → 90°
    if i == 0: cosAngle = π/16 (11.25°)         # shade-0 special-case
    modS = ftol( Sin_lookup(sinAngle) * S )     # S rides the sine curve
    modV = ftol( Cos_lookup(cosAngle) * V )     # V rides the cosine curve
    (r,g,b) = HSV_to_RGB( H, modS, modV )       # 0x00517440, constant hue
    dest[ (i+16)*3 .. +3 ] = (r,g,b)            # writes palette indices 16..31
build LightConvertClass(dest, base, tint=1000/1000/1000) → scheme+0x30C
```

Resolution of the prior Q1 ambiguity (corrected HIGH via live function-identity
and call-flow audit): `Sin_lookup 0x004CACB0` consumes the 50° schedule and
multiplies S; `Cos_lookup 0x004CAD00` consumes the 20° schedule and multiplies
V. Hue is constant across all 16 shades. Net: a smooth
desaturate-and-darken team-color band. The lookup functions use 8192-entry
trig tables. `HSV_to_RGB 0x00517440` is a 6-sextant
integer conversion: `frac=(H*6)%255`, `sextant=(H*6)/255`, `p=(255-S)*V/255`,
`q=(255-frac*S/255)*V/255`, `t=(255-(255-frac)*S/255)*V/255`, all truncating `/0xFF`.

**Caveat for bit-exact repro (MEDIUM→do-before-claiming-parity):**
`Sin_lookup`/`Cos_lookup`/`Math__ftol` are table/x87 operations; a Rust `f64`
port can drift. The live rounding-control state at this constructor call is
UNCHECKED. To lock the exact 16 output triples for a known H,S,V, a gated
runtime trace or equivalent executable oracle is required.

### 3.2 Priority → scheme index — `PriorityToColorScheme 0x0069A310` (HIGH)

```
PriorityToColorScheme(p):
    if p == 0xFFFFFFFE:   return (s8) LUT[8]      # random sentinel → 9th byte = 5
    if p < 9:             p = (s8) LUT[p]         # MOVSX, signed byte read
    return p                                       # p>=9 (and != -2): passthrough, no clamp
```

LUT @ `0x0083ED14` = `03 0B 15 1D 0D 19 11 0F 05` = **{3,11,21,29,13,25,17,15,5}**
(`read_memory 0x0083ED14`). Corrections vs decompile: the random case is a **MOVSX of LUT[8]**
(the 9th table byte), **not** a separate dword `DAT_0083ED1C` (the decompile mislabeled a
byte-of-table read; value 5 is identical either way). Both reads are **signed byte** (`MOVSX`);
all values ≤29 so sign is moot, but the **read width is 1 byte**. No upper clamp.

### 3.3 InitColor — single radar/UI color — `0x50B840` (HIGH, prior session, re-confirmed)

Reads `House+0x16054` (ColorSchemeIndex; if <0 forced to 5), `scheme = g_ColorSchemeArray[idx*4]`
(`0x00B054D4`, pointer stride 4), samples one pixel from the scheme's ConvertClass
(`scheme+0x30C → +0x174` pixel data, element size from `+0x30C[+0x04]`: 1=byte else u16) at
**index `scheme+0x330` = 16**, decodes R/G/B via runtime DD masks (g_DD_*Shift/*Loss), stores
**HouseColorRGB at House+0x56F9 (R) / 0x56FA (G) / 0x56FB (B)**. Produces ONE color.

### 3.4 ComputeRemap — second (bright) color — `0x50BA00` — exact reproducible constants (HIGH)

Constants byte-verified this session (`read_memory`, IEEE-754 LE double):

| Symbol | Address | Value | Role |
|---|---|---|---|
| multiplier | `0x007E5F78` | **240.0** (`0x406E000000000000`) | target vector length |
| high cap | `0x007EAA50` | **255.0** (`0x406FE00000000000`) | per-channel cap |
| low floor | `0x007EAA48` | **96.0** (`0x4058000000000000`) | per-channel floor (below → set 0) |

**Correction to the session lead-in:** multiplier is **240.0, NOT 255.0**; the older docs' literal
`255` for the multiplier is WRONG. Only the cap is 255.0; the floor is 96.0.

`Math__ftol 0x007C5F00`: **truncate toward zero** (control word forced to `0x0E7F`, RC=11b =
round-to-zero; `read_memory 0x00822D80 = 7f 0e 00 00`), low byte consumed (`MOV [..],AL`). No
rounding, no +0.5. `Sqrt_Approx 0x004CAC40` is a **mantissa-LUT approximation** (table at
`0x008650BC`, contents UNREAD) — not `f64::sqrt`; sqrt(0) returns exactly 0.0.

The disassembly shows **two normalization passes** (the decompile collapsed pass 2):

```
normalize(r,g,b):                               # M=240, CAP=255, FLOOR=96
    len = Sqrt_Approx(r*r + g*g + b*b)
    if len == 0:  return (255, 255, 255)        # all three forced to 255
    for each channel c:
        c = c * 240.0 / len
        if c > 255.0: c = 255.0                  # cap applied first (strict >)
        if c < 96.0:  c = 0.0                    # floor applied second (strict <; ==96 kept)
    return (r,g,b)

(r1,g1,b1) = normalize(R,G,B)                    # R,G,B = House+0x56F9..0x56FB
(r2,g2,b2) = normalize(r1,g1,b1)                 # pass 2 over pass-1 doubles, NOT re-read
House+0x56FC = trunc(r2)   # R
House+0x56FD = trunc(g2)   # G   (channel order preserved, no R↔B swap)
House+0x56FE = trunc(b2)   # B
```

Internal math is 80-bit x87; a strict-lockstep port must replicate the two passes, the truncation,
the 240/255/96 constants, and ideally the `Sqrt_Approx` LUT. Single caller: `Create_Houses 0x00687F10`.

---

## 4. INI Keys

`[Colors]` in `ini/rulesmd.ini` (YR, authoritative), 21 entries; `ini/rules.ini` (base) has 19.
Each entry is `Name=H,S,V` (**HSV bytes**, parsed by `sscanf "%d,%d,%d"` in `0x00474C70`, default
0,0,0). The trailing `;`-comment is the artist's plain-name description, not data. HSV
interpretation is HIGH (binary `FUN_00474C70` parses 3 ints to a 3-byte triple; values like
`NeonGreen=0,0,0`→"Black" and `LightGrey=0,0,240`→"White" only make sense as HSV).

[Colors] decl order (rulesmd): 0 LightGold, 1 Gold(MP), 2 LightGrey, 3 Grey, 4 Red, 5 DarkRed(MP),
6 Orange(MP), 7 Magenta(MP), 8 Purple(MP), 9 LightBlue, 10 DarkBlue(MP), 11 NeonBlue, 12 DarkSky(MP),
13 Green, 14 DarkGreen(MP), 15 NeonGreen, 16 Yellow, 17 Purple2, 18 Purple3, 19 AlliedLoad, 20 SovietLoad.

Load-order drift (HIGH): `Gold` = `43,239,255` in rulesmd vs `41,240,230` in rules (YR wins →
`43,239,255`). `Purple2`/`Purple3` are YR-only additions absent from rules.ini.

**The doubling** (HIGH, in-INI confirmation + binary): rulesmd comment line 3388
`;gs note to coders, these entries are all doubled, so ColorSchemes[5] is White, not 2.`
Binary: `Init_Color_Schemes_INI` calls the create-or-find helper **twice per entry** — type `1`
and type `0x35`(53) — so runtime scheme index `R` ↔ [Colors] decl entry `R/2` (integer div, which
is the Rust `scheme_for_priority` `/2` quirk). Runtime scheme count = 2× [Colors] count
(21 → 42 in rulesmd).

Priority → name (priority LUT × doubling; all LUT values odd → every player picks the `2N+1` =
type-`0x35` variant): p0→idx3→Gold, p1→11→DarkRed, p2→21→DarkBlue, p3→29→DarkGreen, p4→13→Orange,
p5→25→DarkSky, p6→17→Purple, p7→15→Magenta, random(-2)→5→LightGrey. The 8 MP colors match the 8
MP-flagged [Colors] entries; "random→white-ish" matches the "ColorSchemes[5] is White" note. HIGH
that LUT values are odd; MEDIUM on the names (R/2 mapping math + INI decl order).

Not part of the player remap (flagged so they're not conflated): `[ColorAdd]` (16-bit additive
tints), `[General] ChronoBeamColor/MagnaBeamColor/LaserTargetColor/IronCurtainColor/BerserkColor/
ForceShieldColor/LocalRadarColor/RadColor`, per-unit `LaserColor`, `IsHouseColor=`/`IsAlternateColor=`
(consumer flags, not the remap definition).

---

## 5. Integration Points

**Build time (rules load):** `RulesClass__Process 0x00668BF0` → `Init_Color_Schemes_INI 0x0066D3A0`
→ per `[Colors]` entry: parse H,S,V, stage into side tables `0x00886380`(names)/`0x00885780`(HSV),
then two `create-or-find(name, hsv, 1)` and `(name, hsv, 0x35)` calls → `ColorScheme__ctor 0x0068C710`
→ `BuildRampPalette 0x0068C3B0` builds the 16..31 band → append to `g_ColorSchemeArray 0x00B054D4`,
bump count `0x00B054E0`.

**House assignment (scenario start):** `Create_Houses 0x00687F10` — for each human/MP player and
each AI house: `idx = PriorityToColorScheme(player_priority)` → `House+0x16054 = idx` →
`InitColor()` → `ComputeRemap()`. Neutral/civilian tail uses `FindColorSchemeIndex()` (by name+type)
→ `House+0x16054` → `InitColor` **only** (no ComputeRemap). HouseColorRGB (+0x56F9) and bright triple
(+0x56FC) are cached per house here.

**Draw time (every frame):**
- *SHP / building bodies:* `TechnoClass_DrawSHP 0x00705E00` calls vtable `+0x1E4`
  (`TechnoClass__SelectRemap 0x00705D70`): remap = `g_ColorSchemeArray[ Owner->+0x16054 *4 ]->+0x30C`
  (the ConvertClass), with overrides for per-type `TypeClass+0xDF0` scheme array and for captured
  units (use captor's scheme). This ConvertClass lands on `surface+0x178`; the blitter reads it.
- *Blitter:* standard opaque body `0x00491740`:
  `if (b != 0) *dst = convert_base[ remap_lut[b] ]` where `remap_lut = surface+0x178`,
  `convert_base = surface+0x174`. **Source byte indexes the LUT directly — no `byte-16` math.**
  Color-0 transparency tested on the raw source byte (so `remap_lut[0]` never read).
- *Voxel bodies:* the house tint enters only as blit flags; the lighting/shading ramp is a
  separate 32-shade × 256-index byte LUT built at file load by `VXL_BuildRemapTable 0x00758B70`
  (shades 0..15 unclamped, 16..31 clamped to 255 before nearest-palette match), sourced from the
  scheme RGB band — not a 16-entry house ramp.
- *Radar/minimap dot:* `RadarClass__RenderCellPixel 0x00655C50` reads **HouseColorRGB
  (House+0x56F9..0x56FB)** — the InitColor color, **not** the bright triple +0x56FC — and packs it
  through g_DD_*. Local-player selection/flash inverts the color (`~color`). Fallback (no owner)
  uses default scheme's `+0x330` index.

**DirectDraw format dependency (verdict):** the unit remap is **not distinctly format-dependent**.
The remap is an 8-bit→8-bit palette substitution; the 16-bit/g_DD_* conversion is the single
**universal** palette→display step (`ConvertClass table builder 0x004BBB00`) that every pixel
(remapped or not, unit/terrain/UI) passes through identically. g_DD_* (`0x008A0DD0..0x008A0DE4`,
set by `DSurface__Constructor 0x004BA9D6`: shift = trailing-zero count of channel mask, loss =
8−bit-width) is shared infrastructure used by both the universal sprite conversion AND the radar/UI
single-color packing. Stock live values are runtime-only (all zero in the static image); typical
stock 16-bit desktop = **RGB565** (R: shift 11/loss 3, G: shift 5/loss 2, B: shift 0/loss 3) —
MEDIUM (derived from the format-recognition branch, not read off a live surface). If the Rust port
renders RGBA8 it is *more* precise than gamemd's 16-bit surface (a global display-precision delta
affecting all pixels equally, not unit-specific).

---

## 6. Current Rust Implementation Status

**Unit/radar ramp path (fully synthetic — the DRIFT):**
- `src/rules/house_colors.rs:61` `SCHEME_BASES` — 9 hardcoded RGB base triples (invented, not from `[Colors]`).
- `src/rules/house_colors.rs:156` `generate_ramp(r,g,b)` — linear brightness sweep
  `brightness_100 = 140 - i*110/15`, `channel*brightness/100`, clamp 255 (shade 0 brightest, 15 darkest).
- `src/rules/house_colors.rs:75` `SCHEMES` — `[[Color;16];9]` ramps at compile time;
  `:95` `house_color_ramp(idx)` accessor (out-of-range/NO_REMAP=255 → Gold).
- `src/rules/house_colors.rs:108` `color_index_for_name(name)` — maps `Color=` string → one of 9 scheme indices.
- Consumers: GPU ramp texture `src/render/palette_textures.rs:263` `build_house_ramp_bytes`
  (`RAMP_SIZE=16 × MAX_HOUSES=32`; row 0 = palette `[16,32)`, rows 1.. = `house_color_ramp`);
  SHP/CPU remap `src/render/sprite_atlas.rs:1224` + `src/assets/pal_file.rs:154`
  (`with_house_colors` copies 16 colors into palette `[16,32)`); target lines
  `src/app_target_lines.rs:287` uses `house_color_ramp(idx)[0]`; radar dot
  `src/render/minimap_helpers.rs:277` `owner_dot_color` uses `house_color_ramp(idx)[0]`.

**Loading/lobby path (the only one that reads `[Colors]`, but scoped to load bar + lobby tints):**
- `src/rules/color_scheme.rs:54` `parse_color_schemes` reads `[Colors]` `Name=H,S,V`;
  `:31` `PRIORITY_TO_SCHEME_INDEX = [3,11,21,29,13,25,17,15,5]` (matches the binary LUT);
  `:88` `scheme_for_priority` (the `/2` doubling quirk, correct); `:94` `hsv_to_rgb`.
  Consumers: load bar `src/app_loading.rs:238`; lobby slot tint
  `src/app_skirmish_shell_render/controls.rs:244`.

The gamemd `InitColor`/`ComputeRemap` single-color outputs (HouseColorRGB +0x56F9, bright triple
+0x56FC) are represented in **neither** Rust path. The two color paths are disjoint: the unit/radar
ramp is fully synthetic and reads no `[Colors]` data; the load/lobby path parses `[Colors]` but
never feeds units/radar.

---

## 7. Coverage Ledger

| Area | Status | Evidence | Remains |
|---|---|---|---|
| 16-shade ramp construction (algorithm, constants, loop, index band) | verified | disassemble_function 0x0068c3b0; read_memory 0x007f0e80 | bit-exact channel values need emulate_function 0x0068c3b0 |
| Sin/Cos schedule ownership | verified | live identities: 0x004CACB0=SINE, 0x004CAD00=COSINE; 50° schedule→S, 20° schedule→V | ramp-call `ftol` RC remains UNCHECKED |
| ColorScheme object layout (size 0x33C, all offsets) | verified | decompile_function 0x0068c710; InitColor 0x50B840 cross-ref | — |
| Doubling mechanism (type 1 / 0x35, two create calls) | verified | decompile_function 0x0066d3a0; create-or-find compares +0x310 | — |
| Priority LUT + PriorityToColorScheme | verified | disassemble_function 0x0069A310; read_memory 0x0083ED14 | decl-order→names is MEDIUM (INI) |
| ColorSchemeIndex write path (+0x16054) | verified | decompile_function 0x00687F10 | — |
| InitColor single-color (index 16, +0x56F9) | verified | prior session 0x50B840 + +0x330 ctor const | — |
| ComputeRemap constants (240/255/96), 2 passes, trunc | verified | read_memory 0x007E5F78/0x007EAA50/0x007EAA48; disassemble 0x50BA00; CW 0x00822D80 | Sqrt_Approx LUT 0x008650BC contents UNREAD |
| Blit-time remap index math (no -16) | verified | decompile_function 0x00491740 / 0x00491590 | exact writer of surface+0x178 = ConvertClass install (MEDIUM) |
| g_DD_* setter + algorithm (trailing-zeros/8−width) | verified | decompile_function 0x004BA9D6; list_globals | live values runtime-only (zero in static image) |
| Universal palette→16bit table (g_DD_* packer) | verified | decompile_function 0x004BBB00 | — |
| VXL 32×256 ramp + clamp asymmetry | verified | decompile_function 0x00758B70 | NearestColorMatch source-RGB origin (MEDIUM) |
| Radar dot uses +0x56F9 not +0x56FC | verified | decompile_function 0x00655C50 | — |
| Stock live display format = RGB565 | touched-not-exhausted | format-recognition branch in 0x004BA9D6 | needs runtime surface read |
| ConvertClass +0x174 LUT population from scheme RGB | not-touched | flagged DOC-HIGH only | constructor decode (out of lane) |
| type-1 (non-player) scheme variant consumers | not-touched | only type-0x35 used by players | downstream trace |

---

## 8. Open Questions — final state

- [RESOLVED: where is the 16-entry unit ramp built] — `ColorScheme__BuildRampPalette 0x0068C3B0`,
  a fixed-hue trig S/V sweep into embedded-palette indices 16..31 (disassemble_function 0x0068c3b0).
- [RESOLVED: ramp source] — the `[Colors]` H,S,V bytes per scheme, not a synthesized base RGB
  (decompile_function 0x0066d3a0 + 0x0068c710).
- [RESOLVED: sine/cosine schedule ownership] — SINE `0x004CACB0`
  drives S with 50°→90°; COSINE `0x004CAD00` drives V with 20°→90° and
  shade-0 π/16 (2026-07-27 live identity audit).
- [RESOLVED: ComputeRemap constants] — 240.0 / 255.0 / 96.0 (read_memory 0x007E5F78/0x007EAA50/0x007EAA48);
  multiplier is 240, NOT 255.
- [PARTIAL: ftol rounding] — ComputeRemap has the documented 0x0E7F evidence;
  live rounding control at the ramp-constructor call remains UNCHECKED.
- [RESOLVED: priority→index] — 9-byte LUT {3,11,21,29,13,25,17,15,5}, random→LUT[8]=5
  (disassemble_function 0x0069A310; read_memory 0x0083ED14).
- [RESOLVED: radar dot field] — HouseColorRGB +0x56F9, not bright triple +0x56FC
  (decompile_function 0x00655C50).
- [RESOLVED: format dependency] — not distinct to units; universal palette→display conversion only
  (decompile_function 0x004BBB00 / 0x004BA9D6).
- [DEFERRED: bit-exact 16 ramp triples — category: emulation] — needs `emulate_function 0x0068C3B0`
  for a known H,S,V (read-only/no-emulate mandate this session). Blocks byte-parity claim, not D9 planning.
- [DEFERRED: Sqrt_Approx LUT contents — category: runtime/data] — `read_memory 0x008650BC len 4096`;
  needed only for strict ComputeRemap bit-parity, not for the ramp.
- [DEFERRED: live g_DD_* values / RGB565-vs-555 — category: runtime] — computed at surface creation;
  static image is zero. Global precision question, not unit-specific.
- [DEFERRED: ConvertClass +0x174 LUT population — category: out-of-lane] — the constructor that
  writes the 16-bit pixel table from the embedded palette; needed to know stored band is 8-bit vs 16-bit.

---

## 9. Visual / UI Composition note (the remap draw path)

Player-visible team coloring is produced by an 8-bit palette substitution at blit, then one
universal palette→display conversion — **not** by compositing a per-house RGB ramp. Observable
consequences to preserve for parity: (1) the unit band is exactly 16 shades at palette indices
16..31, each a desaturate-and-darken step (sin-S 50→90° / cos-V 20→90°
curve), so the *shading character* differs
from a linear brightness ramp — shades cluster differently, especially the shade-0 (π/16 cosine)
brightest entry that becomes the radar/UI/target-line color; (2) the radar dot and the unit body
draw from the **same** scheme band (dot = index 16), so dot and unit team-color are consistent in
gamemd; (3) on a 16-bit surface, all pixels lose per-channel precision (RGB565: G keeps 6 bits,
R/B 5) at the universal conversion — an RGBA8 Rust renderer is uniformly more precise (a global, not
unit-specific, delta the user may or may not choose to quantize).

---

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Unit ramp = `[Colors]` H,S,V → 16-shade fixed-hue trig S/V sweep into palette idx 16..31 (sin-S 50→90°, cos-V 20→90°, i0 cos=π/16, 6-sextant HSV→RGB) | disassemble_function 0x0068c3b0; read_memory 0x007f0e80; live identity audit 2026-07-27; HSV_to_RGB 0x00517440 | `house_colors.rs` had the two angle schedules swapped | `src/rules/house_colors.rs` (ramp build), feeds `palette_textures.rs`, `pal_file.rs`, `sprite_atlas.rs`, loading chrome, minimap, and target lines | Assign the 50° schedule to sine/S and the 20° schedule plus shade-0 π/16 to cosine/V | Pick a player color (e.g. Gold), compare the 16 ramp entries idx16..31 to a gamemd runtime oracle before claiming exact bytes | Do not swap lookup identities back; do not claim exact `ftol` bytes without live RC evidence. |
| Color source = `[Colors]` HSV, doubled (type 1 / 0x35), priority LUT {3,11,21,29,13,25,17,15,5}, random→5 | decompile_function 0x0066d3a0; disassemble_function 0x0069A310; read_memory 0x0083ED14 | `house_colors.rs` reads no `[Colors]`; `color_scheme.rs` parses it but only for load/lobby | unify `color_scheme.rs` (already parses `[Colors]`+LUT+`/2`) as the source for the unit ramp too | Feed the unit ramp builder from the parsed `[Colors]` scheme selected by priority LUT (reuse `scheme_for_priority` `/2`) | Priority 0..7 + random select Gold/DarkRed/DarkBlue/DarkGreen/Orange/DarkSky/Purple/Magenta/LightGrey | Keep the `/2` doubling quirk (it is correct); don't re-derive a 9-entry table that can't reach idx 11/21/29. |
| Radar/minimap dot = HouseColorRGB = scheme palette index 16 (InitColor), NOT the bright triple +0x56FC, NOT ramp shade 0 of a synthetic base | decompile_function 0x50B840 (+0x330=16); decompile_function 0x00655C50 | `minimap_helpers.rs:277` uses synthetic `house_color_ramp(idx)[0]`; target lines `app_target_lines.rs:287` same | `src/render/minimap_helpers.rs`, `src/app_target_lines.rs` | Source dot/target-line color from the real scheme band index 16 (= the corrected ramp's brightest entry) | After ramp fix, dot color == unit team-color brightest shade == scheme index 16 | Once the ramp is correct, `ramp[0]` is the right value (index 16) — no separate ComputeRemap color needed for the dot. |

**Notes for the D9 slice:** the 8-bit palette-substitution architecture (`pal_file.rs` band [16,32),
GPU/shader universal conversion) is **structurally correct and matches gamemd** — keep it; only the
*contents* of indices 16..31 are wrong. Do NOT introduce a per-house 16-bit RGB565/555 round-trip
for units (gamemd does only the one universal display conversion the GPU already does in RGBA8).
ComputeRemap's bright triple (+0x56FC) is a *separate* single color (used for some UI/effects, e.g.
the default-scheme fallback `+0x330` and selection inversion) — it is **not** needed for the unit
ramp or the radar dot; do not wire it into either.
