# Radiation-Glow Render — Adversarial Re-Verification

**Role:** Independent adversarial verifier. All prior worknotes treated as UNVERIFIED; every
fact below re-read from gamemd.exe via Ghidra MCP **this session** (live connection, project
`gamemd`). Default verdict was DRIFT/UNVERIFIED unless proven from the binary.
**Scope:** READ-ONLY on the binary (decompile/disassemble/read_memory only; no Ghidra writes,
no Rust edits).
**Date:** 2026-06-15.

---

## Verdict summary

| # | Claim | Verdict |
|---|---|---|
| 1 | INTENSITY = `ftol(min(RadLevel × RadLightFactor, 2000.0))` at `0x0065B580` | **VERIFIED** |
| 2 | COLOR `ftol(min((c×1000/255)×RadTintFactor, 2000.0))`; tint fades `×remaining/total`, intensity by fixed subtraction; cadence = RadLightDelay | **VERIFIED** |
| 3 | RadSite light lands in per-cell array read by terrain AND sprites; additive-then-normalize-then-clamp-0..2000 | **VERIFIED** |

**Overall: design claims hold.** No corrections required. One observation re the per-site
clamp's practical reach (does not change the formula) is noted under Claim 1. The earlier
file's supersession banner (per-site 2000 clamp IS real) is independently confirmed correct.

---

## Claim 1 — INTENSITY — VERIFIED

`RadSiteClass__Activate @ 0x0065B580`. Verify calls this session: `disassemble_function
0x0065B580`, `read_memory 0x007edae0`, `disassemble_function 0x007c5f00`.

Disassembly shows the exact operand chain claimed:
```
0065b607  FILD  dword ptr [ESI + 0x4c]     ; load RadSite->RadLevel (per-site peak, int @ +0x4C)
0065b63e  FMUL  double ptr [EDI + 0x1820]   ; × RulesClass+0x1820 (RadLightFactor, double)
0065b644  FLD   double ptr [0x007edae0]     ; clamp ceiling
0065b64a  FCOMP                             ; compare product vs ceiling
0065b682  FNSTSW AX ; TEST AH,0x1 ; JZ ...  ; product < ceiling → keep product
0065b68d  FSTP ST0 ; FLD [0x007edae0]       ; else substitute ceiling
0065b695  CALL  0x007c5f00                  ; ftol → EAX
0065b69e  MOV   EBX,EAX
0065b72d  MOV   dword ptr [ESI + 0x54],EBX  ; RadSite->LightIntensity = EBX
```

- **Operand `[ESI+0x4c]` is the per-site RadLevel**, not the per-cell decayed level. Confirmed
  by `decompile_function 0x0065B4F0` (`RadSiteClass__SetRadLevel`): `*(param_1+0x4c)=param_2`.
- **Multiply** is a single `FMUL` against `RulesClass+0x1820`. VERIFIED-FROM-BINARY.
- **Clamp ceiling `0x007edae0` = 2000.0.** `read_memory 0x007edae0` → bytes
  `00 00 00 00 00 40 9F 40` = `0x409F400000000000`. IEEE-754 decode: exponent field `0x409`
  = 1033, unbiased = 10; mantissa `1.953125`; `1.953125 × 2^10 = 2000.0`. VERIFIED.
  (Adjacent double `0x007edae8` = `00 00 00 00 00 C0 72 40` = `0x4072C00000000000` =
  `1.171875 × 2^8 = 300.0`; NOT referenced on any rad FLD — all four rad clamps use
  `0x007edae0`. `read_memory 0x007edae8` confirms.)
- **ftol target `0x007c5f00` is the MSVC `_ftol`.** `disassemble_function 0x007c5f00`:
  `FNSTCW` → load truncate control word from `0x00822d80` → `FISTP qword` → restore. Truncate
  toward zero. VERIFIED-FROM-BINARY (the rounding-mode source data word at `0x00822d80` is the
  standard `_ftol` control word; behavior = truncate toward zero).

**Formula confirmed exactly:** `LightIntensity = ftol( min( RadLevel × RadLightFactor, 2000.0 ) )`
stored at `RadSite+0x54`.

**Observation (does NOT alter the formula):** the per-site 2000 clamp is real and must be
implemented, but at stock `RadLightFactor=0.1` a single site needs summed `RadLevel ≥ 20000`
to reach the ceiling — only stacking pushes it there. The clamp exists in the binary
unconditionally; this note is about reach, not presence.

---

## Claim 2 — COLOR + DUAL DECAY + CADENCE — VERIFIED

### 2a. Per-channel tint at activation — VERIFIED
Same function `0x0065B580` (`disassemble_function 0x0065B580`; `convert_number 0x80808081`).

Color source bytes: `0065b5f5 MOV AL,[EDI+0x1830]` (R), `0065b5dc MOV DL,[EDI+0x1831]` (G),
`0065b5e2 MOV BL,[EDI+0x1832]` (B) — RulesClass RadColor.RGB.

Per-channel integer pre-scale (R block `0065b604..0065b624`): three `LEA [x+x*4]` (×5×5×5 =
×125) then `SHL 3` (×8) → **×1000**; then `IMUL 0x80808081 ; SAR ...,7 ; +sign` → **÷255**.
`convert_number 0x80808081` = signed `-2139062143` — the standard signed-divide-by-255
reciprocal-multiply magic. So integer intermediate = `RadColor_c × 1000 / 255`.

Then each channel (`0065b69a..0065b70f`): `FILD [intermediate] ; FMUL [EDI+0x1828]` (×
RulesClass+0x1828 = RadTintFactor) `; FLD [0x007edae0] ; FCOMP ; (substitute 2000 if ≥) ;
CALL 0x007c5f00` (ftol). Stored `0065b71f..0065b72a` → `+0x58/+0x5C/+0x60` (R/G/B).

**Formula confirmed exactly:**
`Tint_c = ftol( min( (RadColor_c × 1000 / 255) × RadTintFactor , 2000.0 ) )`.
The `×1000/255` rescale and the 2000 clamp are both real. VERIFIED-FROM-BINARY.

### 2b. Two distinct decay curves — VERIFIED
`RadSiteClass__AI @ 0x0065B800` (`decompile_function 0x0065B800` + `disassemble_function
0x0065B800`). On the light-timer fire (disasm `0065b871..0065b8a4`):
```
EDI = [ESI+0x70]          ; RemainingDuration
EBX = [ESI+0x6c]          ; TotalDuration
newG = [ESI+0x60]*EDI/EBX ; tint × remaining/total   (IMUL EAX,EDI ; CDQ ; IDIV EBX)
newR = [ESI+0x5c]*EDI/EBX
newB = [ESI+0x58]*EDI/EBX
newIntensity = [ECX+0x24] - [ESI+0x68]   ; LightSource.intensity − LightIntensityDecrement
FUN_00554aa0(newIntensity, newR, newG, newB, 0)
```
Decompiler agrees: `FUN_00554aa0(*(param_1[9]+0x24) - param_1[0x1a], (param_1[0x16]*iVar1)/iVar2,
(param_1[0x17]*iVar1)/iVar2, (param_1[0x18]*iVar1)/iVar2, 0)` where `iVar1 = +0x70 (remaining)`,
`iVar2 = +0x6c (total)`.

- **Tint** decays multiplicatively `× remaining/total` (ratio fade → 0 at expiry).
- **Intensity** decays by subtracting a fixed precomputed per-step decrement (`+0x68`),
  precomputed in Activate at `0065b73e..0065b749` as `LightIntensity / (TotalDuration/RadLightDelay)`.

Two different curves. VERIFIED-FROM-BINARY. The `+0x6c`/`+0x70` total/remaining come from
`SetRadLevel` (`+0x6c=+0x70=RulesClass+0x1804 × RadLevel`; confirmed `decompile 0x0065B4F0`).

### 2c. RadLightDelay cadence — VERIFIED
Activate sets the light timer from `RulesClass+0x1814` (disasm `0065b599 MOV EAX,[EAX+0x1814]`,
stored to `RadSite+0x3c`). `RadSiteClass__AI` re-arms it from `RulesClass+0x1814` after each
light push (`0065b8b9 MOV ECX,[ECX+0x1814]`). The level/damage timer is the separate
`RulesClass+0x1810`. So light updates step every `RadLightDelay` ticks. VERIFIED-FROM-BINARY.

(`RadSiteClass__AI` only xref is `007f086c [DATA]` = vtable slot +0x5C — `get_xrefs_to
0x0065B800` — reached only through the LogicClass loop; see Claim 3 / gating.)

---

## Claim 3 — COMPOSITING — VERIFIED

### 3a. RadSite creates/drives a LightSource (no bespoke primitive) — VERIFIED
`0x0065B580` disasm: `0065b77c PUSH 0x4c ; CALL 0x007c8e17` (operator new 0x4C), then
`0065b7b2 CALL 0x00554760` (`LightSourceClass__Constructor`, confirmed `decompile_function
0x00554760`), ptr stored to `RadSite+0x24`. Then `0065b7be MOV dword [EAX+0x34],0x0` — forces
the detail threshold to **0** (ctor default is `2`, per `decompile 0x00554760` `param_1[0xd]=2`),
so radiation is never DetailLevel-culled. `0065b7c8 PUSH 0; 0065b7ca CALL 0x00554a60` — enable
in immediate mode (0). On the already-exists branch (`0065b7de`), `CALL 0x00554aa0` (update).
VERIFIED-FROM-BINARY.

### 3b. Per-cell additive accumulation then normalize then clamp 0..2000 — VERIFIED
`FUN_00484180` body, `disassemble_function 0x004842B0` (covers `0x00484180..0x00484675`):

- **Source loop** over `DAT_00abca14[0..DAT_00abca20)` — the global LightSource vector the ctor
  appends to (`decompile 0x00554760` tail: `*(DAT_00abca14 + count*4) = this; count++`). Skip
  unless `source+0x48 != 0` (active, `0048429f..00484299`) AND `source+0x34 <= [0x00a8eb78]`
  (DetailLevel, `0048429f CMP; JL skip`).
- **Linear falloff** `0048439a..004843b8`: `factor = ((radius - distance) × 1000) / radius`
  (the `×1000` is `LEA×3 ; SHL 3`, then `DIV EDI`=radius).
- **Additive accumulation** `004843ba..0048443e`: for each of intensity `+0x24`, R `+0x28`,
  G `+0x2c`, B `+0x30`: `add = source_field × factor / 1000` (`IMUL ; magic 0x10624dd3 ; SAR 6 ;
  +sign` = signed ÷1000; `convert_number 0x10624dd3` = `274877907`, the divide-by-1000 magic),
  then `ADD` into the running per-channel accumulator (`MOV EDI,[EAX] ; ADD EDI,EDX ; MOV [EAX],EDI`).
  **All active sources sum BEFORE any clamp.** VERIFIED-FROM-BINARY.
- **Normalize + clamp** `004845a2..00484615`: high-clamp the ground/intensity term to `0x7d0`
  (=2000); `CALL 0x005558e0`; final high-clamp intensity to 2000 again (`004845dd CMP EAX,0x7d0`)
  and low-clamp all three channels to 0 (`SETLE/DEC/AND` triples). `decompile_function 0x005558e0`
  confirms: low-clamp each RGB to 0, high-clamp each to 2000, normalize the max channel to 1000
  with excess folded into the 16.16 scale, final intensity high-clamp 2000.

So the blend is **purely additive → normalize-max-to-1000 → clamp 0..2000**, exactly as claimed.
VERIFIED-FROM-BINARY.

### 3c. Terrain AND sprites read the SAME CellClass light fields — VERIFIED
- **Terrain tile** `CellOverlay_TileDraw @ 0x00480350` (`decompile_function 0x00480350`):
  lazy-inits `cell+0x34` via `FUN_00483e30` if null, then passes `*(undefined4*)(param_1+0x34)`
  (palette convert profile) AND `*(short*)(param_1+0x10c)` (brightness scalar) into `TMP_TileBlitter`.
- **Techno SHP (units/buildings/infantry)** `0x00705E00` (`decompile_function 0x00705E00`):
  fetches the unit's cell via `Get_CellClass`, lazy-inits `cell+0x34` with the same
  `FUN_00483e30(0,0x10000,0,1000,1000,1000)` when null, then reads `*(short*)(iVar4+0x10c)`
  into the draw brightness param (`param_7`/`param_8`).

Both draw paths read `CellClass+0x34` (convert profile) and `CellClass+0x10C` (brightness scalar)
from the same per-cell state `FUN_00484180`/`FUN_00483E30` write. A green RadSite light therefore
tints the ground tile and the unit standing on it identically. VERIFIED-FROM-BINARY.

---

## TS-legacy / SpecialFlags gate — NOT GATED (VERIFIED)

`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` (`decompile_function 0x0055AFB0`):
```
iVar6 = DAT_00b04be0;                                  // RadSite count
while (iVar6 = iVar6 + -1, -1 < iVar6)
    (**(code **)(**(int **)(DAT_00b04bd4 + iVar6*4) + 0x5c))();  // RadSite vtable +0x5C = AI
FUN_00554d50();                                        // batch LightSource dirty-flush
```
The RadSite loop and the `FUN_00554d50()` flush run **unconditionally** — no `if (SpecialFlags
& …)`, no scenario-flag guard. By contrast the earlier ion/lightning-storm blocks in the SAME
function ARE gated (`(*g_ScenarioClass & 0x1000) != 0` at `LAB_0055b29a`), and the
weather/dominator blocks check Rules doubles — confirming the RadSite/LightSource path is
deliberately ungated. Radiation glow is live in stock YR, not TS-legacy, not SpecialFlags-gated.
VERIFIED-FROM-BINARY.

DetailLevel gate: building lamps use threshold 2 and ARE culled below DetailLevel 2; radiation
forces threshold 0 (Claim 3a) so it shows at every detail level. The per-cell loop's only live
gates are the active flag, the DetailLevel comparison, and the master logic flag — none of which
suppress radiation at default settings.

---

## Calls run this session (evidence index)

| Call | Established |
|---|---|
| `decompile_function 0x0065B580` / `disassemble_function 0x0065B580` | intensity FILD×FMUL×FCOMP×ftol chain; tint ×1000/255 ×RadTintFactor ×clamp; LightSource create/update; detail-threshold 0; enable mode 0 |
| `read_memory 0x007edae0` | clamp ceiling = 2000.0 |
| `read_memory 0x007edae8` | adjacent double = 300.0 (not on rad path) |
| `disassemble_function 0x007c5f00` | ftol = MSVC `_ftol`, truncate toward zero |
| `decompile_function 0x0065B800` / `disassemble_function 0x0065B800` | dual decay: tint ×remaining/total, intensity − fixed decrement; RadLightDelay re-arm |
| `get_xrefs_to 0x0065B800` | AI reached only via vtable slot `007f086c` |
| `decompile_function 0x0065B4F0` | RadLevel @ +0x4C; TotalDuration/Remaining = Rules+0x1804 × RadLevel |
| `decompile_function 0x00554760` | LightSource ctor; threshold default 2; appends to global vector `DAT_00abca14` |
| `decompile_function 0x00554aa0` | LightSource field-update path |
| `disassemble_function 0x004842B0` | per-cell additive accumulation + normalize + clamp 0..2000 |
| `decompile_function 0x005558e0` | normalize max→1000, clamp 0..2000 |
| `decompile_function 0x00480350` | terrain tile reads cell+0x34 / cell+0x10c |
| `decompile_function 0x00705E00` | techno SHP reads cell+0x34 / cell+0x10c |
| `decompile_function 0x0055AFB0` | unconditional RadSite loop + flush (no SpecialFlags gate) |
| `convert_number 0x80808081` | ÷255 magic (signed) |
| `convert_number 0x10624dd3` | ÷1000 magic |
| `get_function_callers 0x0065B580` | Activate callers = AddRadLevel, WarheadTypeClass__Detonate |
