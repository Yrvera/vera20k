# gamemd RadSite Light / Glow Mechanism — Binary-Verified Worknote

**Lane:** radiation-specific light mechanism for the "radiation green glow" render feature
(Cell/Map substrate open item #4, `SUBSTRATE_OPEN_ITEMS_20260610.md`).
**Date:** 2026-06-15 (live-Ghidra pass — supersedes the earlier doc-only draft, see banner)
**Scope:** READ-ONLY (no Rust edits, no Ghidra writes).
**Authority:** binary → Ghidra → docs/research → ini.

---

## ⚠️ Supersession banner — READ FIRST

An **earlier 2026-06-15 draft of this file ran with Ghidra UNREACHABLE** and concluded — from
docs only — that the open-items `min(level×RadLightFactor, 2000)` was a "conflation" and that the
**2000 clamp does NOT exist at the per-site level** (only on accumulated cell channels downstream).

**This pass had a live Ghidra connection and REFUTES that conclusion.** Reading the actual
`RadSiteClass__Activate` assembly: the per-site intensity AND each per-site tint channel **ARE
clamped to the double `2000.0`** (`FCOMP` against `0x007edae0` before every `ftol`). The
open-items doc's `min(…, 2000)` is **VERIFIED**, not a conflation. The prior draft's §1b/§1c/§10
"do not clamp per-site" guidance was wrong; corrected below. (The downstream cell-accumulation
clamp the prior draft cited is a *separate, additional* 2000 clamp — both exist.)

Every load-bearing fact below cites the Ghidra MCP call run **this session**.

---

## TL;DR

| Claim (open-items #4) | Verdict | Evidence |
|---|---|---|
| intensity `min(level × RadLightFactor, 2000)` | **VERIFIED** (real per-site clamp) | `disassemble 0x0065B580` FMUL+FCOMP+ftol; `read_memory 0x007edae0`=2000.0 |
| RadColor tint × `remaining/duration` | **VERIFIED** (tint faded by `remaining/total`) | `decompile 0x0065B800` |
| RadLightDelay stepping | **VERIFIED** | `decompile 0x0065B580` + `0x0065B800` |
| RadSite → LightSource hand-off | **VERIFIED** | `decompile 0x0065B580`, `0x00554760`, `0x00554AA0`, `0x0055AFB0` |
| stock-active in YR (not TS/SpecialFlags-gated) | **VERIFIED — not gated** | `decompile 0x0055AFB0` (unconditional loop) |

**One correction to the tint formula** (also missed by the prior radiation report): the RadColor
byte is **pre-scaled `byte × 1000 / 255`** before `× RadTintFactor` and the 2000 clamp.

---

## 1. Intensity formula — VERIFIED-FROM-BINARY (with the 2000 clamp)

**Function:** `RadSiteClass__Activate @ 0x0065B580`.
Verified via `decompile_function 0x0065B580` and `disassemble_function 0x0065B580` this session.

The decompiler hides the FPU math behind four opaque `Math__ftol()` calls; the assembly is ground
truth. Intensity = the first converted value (stored at `RadSite+0x54`):

```
0065b607  FILD  dword ptr [ESI + 0x4c]      ; load RadSite->RadLevel (int @ +0x4C)
0065b63e  FMUL  double ptr [EDI + 0x1820]    ; *= RulesClass->RadLightFactor (RulesClass+0x1820)
0065b644  FLD   double ptr [0x007edae0]      ; load clamp ceiling
0065b64a  FCOMP                              ; compare product vs ceiling
0065b682  FNSTSW AX ; TEST AH,0x1 ; JZ ...   ; if product < ceiling keep product...
0065b68d  FSTP ST0 ; FLD [0x007edae0]        ; ...else substitute the ceiling
0065b695  CALL  0x007c5f00                   ; ftol(...) -> EAX
0065b69e  MOV   EBX,EAX                       ; LightIntensity
0065b72d  MOV   dword ptr [ESI + 0x54],EBX    ; RadSite->LightIntensity = EBX
```

**Clamp ceiling `0x007edae0` = double `2000.0`.** `read_memory 0x007edae0` → little-endian
`00 00 00 00 00 40 9F 40` = `0x409F400000000000`; decode: exp `0x409`=1033 (unbiased 10), mantissa
1.953125, `1.953125 × 2^10 = 2000.0`. (Adjacent double `0x007edae8` = 300.0, **not** referenced on
any rad path — all four rad clamps use `0x007edae0`.)

### Verified formula
```
LightIntensity = ftol( min( RadSite.RadLevel * RadLightFactor , 2000.0 ) )      (stored @ +0x54)
```
- **"level" =** `RadSite+0x4C`, the **per-site peak RadLevel** set at creation from the weapon's
  `RadLevel` (capped by `RadLevelMax` in the Detonate path), summed on overlap (§5). It is **NOT**
  the per-cell decayed level (`CellClass+0xF0`). Verified: `FILD [ESI+0x4c]`; `SetRadLevel
  @ 0x0065B4F0` writes `+0x4C` (`decompile 0x0065B4F0`).
- **multiply:** single FPU `FMUL` against `RulesClass+0x1820` = `RadLightFactor` (double).
- **`ftol`** = `0x007c5f00` (MSVC `_ftol`, truncate-toward-zero) — INFERRED on the exact rounding
  mode (standard `_ftol`); everything else VERIFIED.

> Correction to `RADIATION_EMP_GHIDRA_REPORT.md §1.6`, which states `LightIntensity =
> ftol(RadLevel * RadLightFactor)` with **no clamp** — that report is incomplete; a real per-site
> `min(…, 2000.0)` exists. Same omission for its tint lines (§2 below).

---

## 2. Color (tint) application — VERIFIED-FROM-BINARY (with rescale correction)

Same function `0x0065B580`. The three tint channels (`+0x58/+0x5C/+0x60`) are
**pre-scaled, × RadTintFactor, clamped to 2000.0, then ftol**.

RadColor source bytes (from the disasm):
- `0065b5f5 MOV AL,[EDI+0x1830]` = RadColor.R (`RulesClass+0x1830`)
- `0065b5dc MOV DL,[EDI+0x1831]` = RadColor.G (`+0x1831`)
- `0065b5e2 MOV BL,[EDI+0x1832]` = RadColor.B (`+0x1832`)

Per-channel integer pre-scale (R at `0065b604..0065b624`):
```
LEA ×5 ; LEA ×5 ; LEA ×5 ; SHL 3   -> channel × 1000
IMUL 0x80808081 ; SAR 7            -> signed divide-by-255  (÷255 magic; convert_number 0x80808081 = -2139062143)
```
→ integer intermediate = `RadColor_c * 1000 / 255`. Then (`0065b6bd..`):
```
FILD [ESP+0x1?] ; FMUL [EDI+0x1828]   ; *= RadTintFactor (RulesClass+0x1828)
FLD [0x007edae0] ; FCOMP ...          ; clamp to 2000.0 (same constant)
CALL 0x007c5f00                       ; ftol
```
Stores `+0x58/+0x5C/+0x60` = Tint R/G/B (`0065b71f..0065b72a`).

### Verified per-channel tint-at-activation
```
Tint_c = ftol( min( (RadColor_c * 1000 / 255) * RadTintFactor , 2000.0 ) )    c ∈ {R,G,B}
```
**Correction:** the doc's plain "RadColor × RadTintFactor" omits the `× 1000 / 255` byte rescale and
the 2000 clamp. (Stock `RadColor=0,255,0`, `RadTintFactor=1.0` → green channel pre-scale
`255*1000/255 = 1000`, ×1.0 = 1000, < 2000 → tint `(0,1000,0)`. The "1000" base matters: it is the
unit the per-cell pipeline treats as 1.0.)

### Time-decay of tint — VERIFIED (`remaining/duration`)
**Function:** `RadSiteClass__AI @ 0x0065B800` (`decompile 0x0065B800`,
`get_function_by_address 0x0065B800`). On each light-step:
```
remaining = RadSite+0x70 ; total = RadSite+0x6c
newR = TintR(+0x58) * remaining / total      ; integer division
newG = TintG(+0x5C) * remaining / total
newB = TintB(+0x60) * remaining / total
newIntensity = LightSource->intensity(+0x24) - LightIntensityDecrement(RadSite+0x68)
FUN_00554aa0(newIntensity, newR, newG, newB, 0)
```
**Tint decays multiplicatively by `remaining/total`** (full at spawn → 0 at expiry); **intensity
decays by subtracting a fixed per-step decrement** — two different curves; render must reproduce
both, not one shared scalar.

---

## 3. Stepping / decay cadence — VERIFIED-FROM-BINARY

Two timers, both off RulesClass `[Radiation]`:
- `RadLevelDelay` = `RulesClass+0x1810` → damage / per-cell level-decay (`RadSite+0x28..0x30`).
- `RadLightDelay` = `RulesClass+0x1814` → **light update** (`RadSite+0x34..0x3C`).

**Precomputed at activation** (`0x0065B580`, `0065b714..0065b749`, integer `CDQ/IDIV`):
```
TotalDuration         = +0x6C = RadLevel * RadDurationMultiple   (RulesClass+0x1804; SetRadLevel)
RadLevelPerStep       = +0x50 = TotalDuration / RadLevelDelay
LightIntensityPerStep = +0x64 = TotalDuration / RadLightDelay
LightIntensityDecrement = +0x68 = LightIntensity / LightIntensityPerStep
```

**Per tick** (`RadSiteClass__AI @ 0x0065B800`):
1. `RemainingDuration(+0x70) -= 1` every tick.
2. Light timer (`RadLightDelay`): push faded tint + decremented intensity to the LightSource
   (the `FUN_00554aa0` call, §2); re-arm from `RulesClass+0x1814`.
3. Level timer (`RadLevelDelay`): `RadSiteClass__ApplyRadDamage` decreases each covered cell's
   `+0xF0` by `radAmount / RadLevelPerStep`; re-arm from `RulesClass+0x1810`.
4. `RemainingDuration < 1` → self-destruct via vtable `+0x20` (flag 1); LightSource torn down with it.

**Decay shapes for render:** intensity = stepwise linear **subtract** (changes only on light-timer
fires); tint = **ratio fade** `Tint * remaining/total` (recomputed each light-step). Both update once
per `RadLightDelay`, not continuously.

---

## 4. RadSite → LightSource hand-off — VERIFIED-FROM-BINARY

**Creation (first activation), `RadSiteClass__Activate @ 0x0065B580`:**
- If `RadSite+0x24` (LightSource ptr) is null → `operator_new(0x4c)` then
  `LightSourceClass__Constructor @ 0x00554760` (`decompile 0x00554760`).
- Ctor args from the push block `0065b78a..0065b7b0` (push order): `tintB`, `tintG`, `tintR`,
  `intensity`, `SpreadInLeptons(+0x48)`, then the cell's 3D world coords (CellClass vtable `+0x48`
  getter at `0065b75e`) → **LightSource(coordX,Y,Z, spreadLeptons, intensity, tintR,G,B)**.
- Ctor zeroes `LightSource+0x34`; `0065b7c8 CALL 0x00554a60(0)` enables/registers it; ptr stored
  back to `RadSite+0x24`. Then `RadSiteClass__SetCellRadLevels @ 0x0065B9C0` seeds per-cell levels.

**Update path (LightSource already exists, AND every AI light-step):**
- `FUN_00554aa0(intensity, tintR, tintG, tintB, 0)` (`decompile 0x00554aa0`) writes light fields
  (`+0x24` intensity, `+0x28/+0x2c/+0x30` tint) and, **if active flag `+0x48` set, calls
  `FUN_00554af0`** — the per-light **dirty recompute** that re-stamps affected cells
  (`get_function_by_address 0x00554af0` = the `0x00554AF0` slice in
  `LIGHTSOURCE_DIRTY_SCHEDULING_…REPORT.md`).

**Per-frame batch flush (global hand-off into rendering):**
`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` (`decompile 0x0055AFB0`;
`get_function_by_address 0x0055B5CD` → this fn; `get_function_by_address 0x00554D50`):
```
iVar6 = DAT_00b04be0;                 // g_RadSiteClass_Array_Count (0x00B04BE0)
while (--iVar6 >= 0)                   // reverse loop over all live RadSites
    (* RadSite[i]->vtable[0x5C] )();   // RadSiteClass__AI  (mutates its LightSource fields)
FUN_00554d50();                        // 0x00554D50 batch LightSource dirty-flush
```
RadSite array globals: data `0x00B04BD4`, count `0x00B04BE0` (matches `RADIATION_EMP §1.12`).

**Render takeaway:** gamemd keeps **no bespoke "rad glow" primitive** — the green light is an
ordinary `LightSourceClass` carrying `(world coord, spreadInLeptons radius, intensity, tintRGB)`,
driven by the RadSite and composited through the same cell-lighting pipeline as building lamps.
Rust render should model one dynamic point light per active RadSite (pos = center cell 3D coord,
radius from `SpreadInLeptons`, intensity/tint per §1–§2, stepped per §3) and feed it through the
cell-light accumulation.

### Downstream cell-light pipeline (the SECOND 2000 clamp — folded in from the prior draft, DOC-SOURCED)
The per-cell compute `0x00484180` (`MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`, DOC-SOURCED,
**not re-verified this session**): starts from `[Lighting]` ambient (`×1000/100`); for each active
source within radius adds `source_field × (radius - dist)/radius`; `FUN_005558E0` normalizes RGB and
**high-clamps accumulated channels + additive intensity to 2000, low-clamp 0**. So there are **two**
2000 clamps: (a) per-site at activation (VERIFIED this session, §1–§2), and (b) per-cell on the
accumulated sum (DOC-SOURCED). Render must apply BOTH — clamp per-site intensity/tint at 2000, and
clamp the accumulated per-cell channels at 0..2000.

---

## 5. Stacking — VERIFIED

`RadSiteClass__AddRadLevel @ 0x0065B530` (`decompile 0x0065B530`; `get_function_callers 0x0065B580`):
a detonation on a cell with an existing RadSite **sums** the level (`current_decayed + new`),
recomputes TotalDuration/Remaining, and re-runs `Activate` — re-hitting the 2000 clamp. Only callers
of `Activate` are `AddRadLevel` and `WarheadTypeClass__Detonate @ 0x004690B0` (creation). Stacked
Desolator deploys can push per-site intensity to the 2000 ceiling.

---

## 6. TS-legacy / SpecialFlags gating — VERIFIED NOT GATED

**The radiation glow is live in stock YR — not TS-legacy, not SpecialFlags-gated.**
`decompile 0x0055AFB0`: the RadSite per-tick loop and the trailing `FUN_00554d50()` light flush run
**unconditionally** every tick — no `if (SpecialFlags & …)`, no scenario-flag guard around the
RadSite iteration or the flush. (Contrast: the lightning/ion-storm blocks earlier in the same
function ARE gated, e.g. `*g_ScenarioClass & 0x1000` at `0055b2e8`; the RadSite/LightSource block is
not.) `RadSiteClass__AI`'s only xref is vtable slot `0x007F086C` (`+0x5C`) — reached purely through
that live loop (`get_xrefs_to 0x0065B800`).

**DetailLevel gate (DOC-SOURCED, render-relevant, not re-verified this session):** a default-created
LightSource writes `+0x34 = 2` and is culled when `+0x34 > [Options] DetailLevel`
(`MAP_LIGHTING §3.3`; default DetailLevel = 2). So on default settings the glow shows; reduced detail
suppresses it. A faithful render should honor this gate.

---

## 7. RadSiteClass fields the render layer reads (VERIFIED offsets)

| Offset | Field | Render relevance |
|---|---|---|
| +0x24 | LightSource* | the glow object |
| +0x40 / +0x42 | CellX / CellY | site center (light position) |
| +0x44 | Spread (cells) | radius |
| +0x48 | SpreadInLeptons (= Spread*256+128) | radius in leptons for falloff |
| +0x4C | RadLevel (per-site peak) | drives intensity & tint base |
| +0x54 | LightIntensity = `ftol(min(RadLevel*RadLightFactor, 2000))` | initial brightness |
| +0x58/+0x5C/+0x60 | Tint R/G/B = `ftol(min((RadColor*1000/255)*RadTintFactor, 2000))` | base color |
| +0x64 | LightIntensityPerStep = TotalDuration/RadLightDelay | step count |
| +0x68 | LightIntensityDecrement = LightIntensity/PerStep | per-step intensity drop |
| +0x6C | TotalDuration = RadLevel*RadDurationMultiple | tint fade denominator |
| +0x70 | RemainingDuration | tint fade numerator; tick countdown |

---

## 8. Stock numeric walkthrough — Desolator (INI inputs from `rulesmd.ini`, formulas VERIFIED)

Stock `[Radiation]`: `RadDurationMultiple=1`, `RadLevelDelay=90`, `RadLightDelay=90`,
`RadLightFactor=0.1`, `RadTintFactor=1.0`, `RadColor=0,255,0`; Desolator weapon `RadLevel=500`.
- `TotalDuration = 500 × 1 = 500` frames (~33 s @ 15 fps); light steps every 90 frames (~5–6 steps).
- `LightIntensity = ftol(min(500 × 0.1, 2000)) = 50`.  ← single Desolator site stays well under 2000.
- `LightIntensityPerStep = 500/90 = 5`; `LightIntensityDecrement = 50/5 = 10` → intensity ≈ `50−10k`.
- tint at spawn = `(0, min(255*1000/255 × 1.0, 2000), 0) = (0, 1000, 0)`; then `(0, 1000*remaining/500, 0)`.
- The 2000 per-site clamp only bites when stacked levels push `RadLevel × RadLightFactor ≥ 2000`
  (i.e. summed `RadLevel ≥ 20000` at stock `RadLightFactor=0.1`) — rare for a single site, reachable
  by stacking; the per-cell accumulation clamp (§4) bites sooner on overlapping sites.

---

## Address index (verified this session unless tagged DOC-SOURCED)

| Address | Role | Verify call |
|---|---|---|
| `0x0065B580` | `RadSiteClass__Activate` — intensity/tint precompute + 2000 clamp + LightSource create/update | decompile + disassemble |
| `0x0065B800` | `RadSiteClass__AI` — per-tick decay + light push (vtable +0x5C) | decompile, get_function_by_address, get_xrefs_to |
| `0x0065B4F0` | `RadSiteClass__SetRadLevel` — TotalDuration = RadLevel × RadDurationMultiple | decompile |
| `0x0065B530` | `RadSiteClass__AddRadLevel` — stacking (sum, re-Activate) | decompile, get_function_callers |
| `0x00554760` | `LightSourceClass__Constructor` | decompile |
| `0x00554AA0` | LightSource field-update (calls dirty recompute if active) | decompile |
| `0x00554AF0` | per-light dirty recompute | get_function_by_address |
| `0x00554D50` | per-frame batch LightSource flush | get_function_by_address |
| `0x0055AFB0` | `LogicClassPerTickUpdateLiveVector` — unconditional RadSite loop + flush | decompile |
| `0x007EDAE0` | double `2000.0` — per-site intensity/tint clamp ceiling | read_memory |
| `0x00484180` | per-cell light compute (downstream 0..2000 accumulation clamp) | DOC-SOURCED |
| RulesClass `+0x1804/+0x1810/+0x1814/+0x1820/+0x1828/+0x1830` | RadDurationMultiple / RadLevelDelay / RadLightDelay / RadLightFactor / RadTintFactor / RadColor | Activate disasm operands |
| `0x00B04BD4` / `0x00B04BE0` | RadSite array data ptr / count | LogicClass loop decompile |

---

## Negative / do-not-do

- **DO clamp a single RadSite's intensity (and each tint channel) to 2000** at activation — this is
  real and VERIFIED. (This reverses the earlier doc-only draft's "do not clamp per-site" guidance.)
- **AND** clamp the accumulated per-cell channels to 0..2000 (downstream, DOC-SOURCED). Two clamps.
- **Do NOT** use one decay scalar for tint and intensity: tint = `remaining/total` ratio; intensity
  = fixed per-step subtraction.
- **Do NOT** build a bespoke green overlay; model a `LightSourceClass`-equivalent dynamic point light.
- **Do NOT** ignore the DetailLevel gate (suppress below DetailLevel 2).
- **Do NOT** drive intensity/tint from the per-cell decayed level — use the per-site `RadLevel` (+0x4C).
