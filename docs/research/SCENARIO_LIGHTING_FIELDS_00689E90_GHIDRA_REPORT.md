# Scenario Lighting Fields 00689E90 - Ghidra Research Report

**Address(es):** `0x00689E90` primary, `0x00484180`, `0x00483E30`, `0x00484680`, `0x005558E0` consumers/helpers  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `ScenarioClass::Read_INI_Basic` map `[Lighting]` fields, their binary scaling/defaults, and the ordinary map-lighting consumer formula used for per-cell LightConvert/Z-adjust setup.  
**Non-Scope:** `LightSourceClass` allocation/lifetime, `LightConvertClass` palette table internals, complete Lightning Storm/Psychic Dominator superweapon lifecycle, renderer draw-call auditing beyond immediate cell-field consumers.  
**Confidence:** High for field reads, offsets, scaling instructions, defaults, and ordinary formula; Medium for semantic names of the three ambient slots because only immediate consumers were traced.  
**Active in YR:** Yes. `ScenarioClass__Full_Init @ 0x00686B20` calls `ScenarioClass__Read_INI_Basic @ 0x00687853` on the standard scenario load path.

## Target Question

What does `ScenarioClass::Read_INI_Basic @ 0x00689E90` read from map `[Lighting]`, how are those values scaled into ScenarioClass fields, what defaults exist before map override, and what exact ordinary map-lighting formula consumes them?

## Evidence Needed To Mark COMPLETE

- Decompile `0x00689E90` and record all scoped `[Lighting]` key reads, ScenarioClass offsets, and defaults.
- Verify the floating-point-to-integer scale from assembly around the read sites.
- Decompile the ordinary consumer path that uses the fields during map/cell initialization.
- Separate ordinary map lighting from Ion, Nuke/Lightning, and Dominator branches.
- Scan current Rust surfaces enough to name deltas and test proposals.

## Stop Conditions

- Stop after immediate consumers of `ScenarioClass+0x3528..+0x3594` are mapped for ordinary map ambience.
- Do not follow `LightSourceClass` lifecycle or `LightConvertClass` palette construction beyond the values passed into them.
- Do not patch Rust, INI, existing docs, or Ghidra labels.

## 1. Overview

Map `[Lighting]` is a ScenarioClass-owned ambience system. The ordinary fields are read once during scenario INI load, converted from map-authored decimal values into integer fixed-scale fields, and later consumed by per-cell light-convert setup (`0x00484180` via `0x00483E30`).

The ordinary formula is additive in binary units: base ambient plus local light-source contribution, plus `Level * cell.Level`, minus `Ground`. It is not `ambient * (1 - ground)` except in the special case where external ambient is exactly `1.0`.

## 2. Class Layout / Key Offsets

All offsets are byte offsets from `ScenarioClass` (`g_ScenarioClass_Instance` / `DAT_00A8B230`).

| Offset | Field role | Reset default | External value represented | Read key | Active in YR |
|---:|---|---:|---:|---|---|
| `+0x3528` | ordinary ambient slot A | `100` | `1.0` | `Ambient` | Yes |
| `+0x352C` | ordinary ambient slot B, immediate consumer base | `100` | `1.0` | `Ambient` writes same value | Yes |
| `+0x3530` | ordinary ambient slot C | `100` | `1.0` | `Ambient` writes same value | Yes |
| `+0x3534` | ordinary red channel | `100` | `1.0` | `Red` | Yes |
| `+0x3538` | ordinary green channel | `100` | `1.0` | `Green` | Yes |
| `+0x353C` | ordinary blue channel | `100` | `1.0` | `Blue` | Yes |
| `+0x3540` | ordinary ground offset | `50` | `0.2` if key absent | `Ground` | Yes |
| `+0x3544` | ordinary height step | `8` | `0.032` | `Level` | Yes |
| `+0x3548` | Ion ambient | `87` | `0.87` | `IonAmbient` | Conditional: TS/Ion branch only |
| `+0x354C` | Ion red | `30` | `0.30` | `IonRed` | Conditional |
| `+0x3550` | Ion green | `40` | `0.40` | `IonGreen` | Conditional |
| `+0x3554` | Ion blue | `75` | `0.75` | `IonBlue` | Conditional |
| `+0x3558` | Ion ground | `0` | `0.0` | `IonGround` | Conditional |
| `+0x355C` | Ion level | `0` | `0.0` | `IonLevel` | Conditional |
| `+0x3570` | Lightning/Nuke ground-like dynamic offset | `100` | not read from `[Lighting]` in this function | none here | Conditional: Lightning branch |
| `+0x3574` | Lightning/Nuke level-like dynamic step | `100` | not read from `[Lighting]` in this function | none here | Conditional: Lightning branch |
| `+0x3578` | Nuke ambient change rate | `1` | integer, no 100/250 scale at this read site | `NukeAmbientChangeRate` | Conditional |
| `+0x357C` | Dominator ambient | `150` | `1.5` | `DominatorAmbient` | Conditional: Psychic Dominator |
| `+0x3580` | Dominator red | `85` | `0.85` | `DominatorRed` | Conditional |
| `+0x3584` | Dominator green | `20` | `0.20` | `DominatorGreen` | Conditional |
| `+0x3588` | Dominator blue | `30` | `0.30` | `DominatorBlue` | Conditional |
| `+0x358C` | Dominator ground | `0` | `0.0` | `DominatorGround` | Conditional |
| `+0x3590` | Dominator level | `0` | `0.0` | `DominatorLevel` | Conditional |
| `+0x3594` | Dominator ambient change rate | `1` | `0.004` if key absent | `DominatorAmbientChangeRate` | Conditional |

Default evidence: `FUN_00683610 @ 0x00683610` writes all defaults above. Active in YR: Yes, constructor `0x006832C0` calls it during ScenarioClass initialization.

## 3. Core Logic

### 3.1 INI read scaling

`ScenarioClass__Read_INI_Basic @ 0x00689E90` reads `[Lighting]` after `[Ranking]`.

Ordinary `Ambient`, `Red`, `Green`, `Blue` use this pattern:

1. Push default as `current_internal * 0.01`.
2. Call `CCINIClass__ReadDouble`.
3. Multiply returned value by `100.0`.
4. Add `0.01`.
5. Call `Math__ftol`.
6. Store the integer field.

Evidence: assembly around `0x0068A817..0x0068A846` for `Ambient`, `0x0068A84B..0x0068A88C` for `Red`, repeated for `Green`/`Blue`.

Active in YR: Yes. This path is in the standard scenario load call from `0x00687853`.

`Ground` and `Level` use a different scale:

1. Push default as `current_internal * 0.004` (`float ptr [0x007F0E78]`, inferred from `8 -> 0.032` and final multiply).
2. Call `CCINIClass__ReadDouble`.
3. Multiply returned value by `250.0`.
4. Add `0.01`.
5. `ftol` and store.

Evidence: `Ground` assembly `0x0068A905..0x0068A93F`; `Level` assembly `0x0068A93F..0x0068A984`.

Active in YR: Yes.

Critical detail: `Ambient` writes the same parsed integer to `+0x3528`, `+0x352C`, and `+0x3530` in that order. Immediate ordinary consumers in this slice read `+0x352C`.

### 3.2 Ordinary per-cell formula

`FUN_00484180 @ 0x00484180` computes the full cell lighting package used by `FUN_00483E30`.

For a real map cell (`cell.MapCoord` neither `(0,0)` nor `(-1,-1)`), ordinary path with no active superweapon:

```text
10A-like ground brightness =
  ((Scenario+0x352C * 1000) / 100)
  + local_light_intensity
  + (Scenario+0x3544 * signed_cell_level)
  - Scenario+0x3540

+10E-like bridge brightness =
  ((Scenario+0x352C * 1000) / 100)
  + local_light_intensity
  + (Scenario+0x3544 * (signed_cell_level + 4))
  - Scenario+0x3540
```

Evidence: decompile `0x00484180`, ordinary branch after `FUN_0053A100`, `FUN_0053B400`, and `FUN_0053A110` all return false.

Active in YR: Yes. `MapClass__InitCellAttributes @ 0x00568C90` calls `FUN_00483E30` for every cell during scenario load; `FUN_00483E30` calls `FUN_00484180` for non-sentinel cells.

Tiny details:

- Cell level is read as signed byte from `CellClass+0x11B`.
- Bridge brightness uses `cell_level + 4`.
- The ground/bridge brightness values clamp high when `value > 1999` to exactly `2000`.
- They clamp low by zeroing values `< 1`, so `1` survives and `0`/negative become `0`.
- Sentinel/off-map cells `(0,0)` and `(-1,-1)` bypass the formula and receive neutral defaults: scale `0x10000`, base `0`, and brightness/color slots `1000`.

### 3.3 RGB normalization helper

`FUN_005558E0 @ 0x005558E0` clamps red/green/blue components to `[0,2000]`, normalizes the dominant RGB component to `1000`, and moves excess color intensity into a 16.16 scale value that is then applied to brightness.

Evidence:

- Lower clamp uses `TEST value; SETLE; DEC; AND`, so `<= 0` becomes `0`.
- Upper clamp compares with `0x7D0` (`2000`).
- If RGB is exactly `1000,1000,1000`, it leaves scale as `0x10000`.
- If the computed scale is less than `0x42` (`66`), it resets RGB to neutral `1000,1000,1000` and brightness offset to `0`.
- Assembly around `0x00555951..0x00555A9E` shows the dominant-channel selection, `1000` normalization, and `scale * brightness >> 16`.

Active in YR: Yes. `FUN_00484180` calls it for every non-sentinel cell lighting computation.

### 3.4 Superweapon branch separation

The ordinary branch is selected only when all three checks are false:

- `FUN_0053A100 @ 0x0053A100` returns `DAT_00A9FAB4`. If nonzero, the code uses Ion/PsychicDominator-full offsets `+0x3558/+0x355C`.
- `FUN_0053B400 @ 0x0053B400` returns `DAT_00A9FAC0 != 0`. If true, `0x00484180` uses `+0x358C/+0x3590` for Dominator transition lighting.
- `FUN_0053A110 @ 0x0053A110` returns `DAT_00A9FABC == 1`. If true, it uses `+0x3570/+0x3574`, not ordinary `+0x3540/+0x3544`.

Active in YR: Conditional. These are active only during their superweapon visual states. They are not ordinary map ambience.

## 4. INI Keys

| Section | Key | Binary read? | Scale into field | Default source | Active in YR |
|---|---|---:|---|---|---|
| `[Lighting]` | `Ambient` | Yes | `ftol(value * 100 + 0.01)`, copied to `+0x3528/+0x352C/+0x3530` | reset `100`; FinalAlert template `1.000000` | Yes |
| `[Lighting]` | `Red` | Yes | `ftol(value * 100 + 0.01)` | reset `100`; template `1.000000` | Yes |
| `[Lighting]` | `Green` | Yes | `ftol(value * 100 + 0.01)` | reset `100`; template `1.000000` | Yes |
| `[Lighting]` | `Blue` | Yes | `ftol(value * 100 + 0.01)` | reset `100`; template `1.000000` | Yes |
| `[Lighting]` | `Ground` | Yes | `ftol(value * 250 + 0.01)` | reset `50`; template `0.000000` | Yes |
| `[Lighting]` | `Level` | Yes | `ftol(value * 250 + 0.01)` | reset `8`; template `0.032000` | Yes |
| `[Lighting]` | `IonAmbient`, `IonRed`, `IonGreen`, `IonBlue` | Yes | `ftol(value * 100 + 0.01)` | reset values in `0x00683610`; template has Ion keys | Conditional |
| `[Lighting]` | `IonGround`, `IonLevel` | Yes | `ftol(value * 250 + 0.01)` | reset `0`, `0`; template has Ion keys | Conditional |
| `[Lighting]` | `NukeAmbientChangeRate` | Yes | `ftol(value)` at this read site | reset `1`; not present in FinalAlert template found | Conditional |
| `[Lighting]` | `DominatorAmbient`, `DominatorRed`, `DominatorGreen`, `DominatorBlue` | Yes | `ftol(value * 100 + 0.01)` | reset `150/85/20/30`; template has Dominator keys | Conditional |
| `[Lighting]` | `DominatorGround`, `DominatorLevel`, `DominatorAmbientChangeRate` | Yes | `ftol(value * 250 + 0.01)` | reset `0/0/1`; template has Dominator keys | Conditional |

Template evidence: `C:/Users/enok/Documents/Command and Conquer Red Alert II/FinalAlert2/StdMapRA2.ini:34-53`.

## 5. Integration Points

| Function | Role | Active in YR | Evidence |
|---|---|---|---|
| `ScenarioClass__Full_Init @ 0x00686B20` | Standard scenario load path; calls `Read_INI_Basic` before map/cell initialization | Yes | call at `0x00687853` |
| `ScenarioClass__Read_INI_Basic @ 0x00689E90` | Reads `[Lighting]` fields | Yes | decompile and assembly around `0x0068A817..0x0068AC9A` |
| `MapClass__InitCellAttributes @ 0x00568C90` | Calls `FUN_00483E30` for every playfield cell during load | Yes | decompile `0x00568C90` |
| `FUN_00483E30 @ 0x00483E30` | Writes cell light/LightConvert fields; calls `FUN_00484180` for non-sentinel cells | Yes | decompile/xrefs |
| `FUN_00484180 @ 0x00484180` | Computes ordinary and conditional superweapon cell-lighting packages | Yes/Conditional by branch | decompile |
| `Cell_ComputeZAdjust @ 0x00484680` | Recomputes a subset during dynamic superweapon updates | Conditional | xrefs only from `0x004AE4C0`; ordinary branch exists but function is not the normal load initializer |
| `FUN_0053AD00 @ 0x0053AD00` | Propagates dynamic lighting changes to LightConvert/color schemes, then calls `0x004AE4C0` | Conditional | xrefs from superweapon/reset helpers |

## 6. Current Rust Implementation Status

Rust parses ordinary `[Lighting]` in [src/map/lighting.rs](C:/Users/enok/Documents/ra2-rust-game/src/map/lighting.rs:65) and builds the grid from [src/app_init.rs](C:/Users/enok/Documents/ra2-rust-game/src/app_init.rs:339).

Observed deltas:

- `cell_tint` uses `ambient * (1.0 - ground) + level * z`. Binary ordinary load formula is additive: `ambient + level*z - ground`, plus local light contributions, in fixed integer units.
- Rust stores direct `f32` values, while binary stores `Ambient/RGB` as value*100 integers and `Ground/Level` as value*250 integers before deriving 1000-scale cell fields.
- Rust default `Ground=0.0` matches FinalAlert template but not ScenarioClass reset default if the key is absent (`+0x3540=50`, external `0.2`).
- Rust cap `TOTAL_AMBIENT_CAP = 2.0` matches the binary's 2000 cap in external units for brightness/RGB, but binary clamp and RGB normalization order are more specific.
- Rust currently gives terrain a uniform ground-level tint in `app_init.rs`; binary computes per-cell cell fields, but full terrain draw/palette consumer parity was not part of this slice.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00689E90` ordinary `[Lighting]` reads | verified | decompile + assembly `0x0068A817..0x0068A984` | none |
| ScenarioClass lighting defaults | verified | `FUN_00683610 @ 0x00683610` | none |
| Ordinary formula in `0x00484180` | verified | decompile `0x00484180` | none for immediate formula |
| RGB normalization helper | verified | decompile/assembly `0x005558E0` | exact floating constants not memory-dumped; semantic formula inferred from instructions |
| Initial cell setup caller | verified | `MapClass__InitCellAttributes @ 0x00568C90`, `FUN_00483E30 @ 0x00483E30` | none |
| Dynamic recompute path | touched-not-exhausted | `0x00484680`, `0x004AE4C0`, `0x0053AD00` | full superweapon timing belongs to LS/PD investigations |
| LightConvert palette internals | deferred | xrefs show handoff to `FUN_00544E70`/constructor | out-of-scope |
| Rust parse/current formula | verified | `src/map/lighting.rs`, `src/app_init.rs` scan | implementation not modified |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is `0x00689E90` active in standard YR scenario load? -> Yes, called from `ScenarioClass__Full_Init @ 0x00687853`.` (evidence: `get_function_xrefs 0x00689E90`, decompile `0x00686B20`)
- `[RESOLVED] OQ-2 - Which ordinary `[Lighting]` keys are read? -> `Ambient`, `Red`, `Green`, `Blue`, `Ground`, `Level`.` (evidence: `0x0068A817..0x0068A984`)
- `[RESOLVED] OQ-3 - What are ordinary offsets? -> `+0x3528/+0x352C/+0x3530`, `+0x3534`, `+0x3538`, `+0x353C`, `+0x3540`, `+0x3544`.` (evidence: `0x00689E90`)
- `[RESOLVED] OQ-4 - Are default values verified? -> Yes, `FUN_00683610` writes defaults before INI reads.` (evidence: `0x00683610`)
- `[RESOLVED] OQ-5 - What is the external-to-internal scale? -> Ambient/RGB scale by 100; Ground/Level scale by 250; each adds `0.01` before `ftol`.` (evidence: assembly `0x0068A83A`, `0x0068A92E`, `0x0068A968`)
- `[RESOLVED] OQ-6 - Does ordinary consumer use `ambient * (1-ground)`? -> No, it adds ambient and subtracts ground.` (evidence: `0x00484180`)
- `[RESOLVED] OQ-7 - Is `Level` cell height signed? -> Yes, `CellClass+0x11B` is read as signed char.` (evidence: `0x00484180`, `0x00484680`)
- `[RESOLVED] OQ-8 - Are Ion fields ordinary map ambience? -> No, they are read but consumed only behind dynamic branch checks in this slice.` (evidence: `0x00484180`, `0x0053A100`)
- `[RESOLVED] OQ-9 - Does this read Nuke color fields? -> No scoped read found; only `NukeAmbientChangeRate` is read here.` (evidence: `0x0068AAD5..0x0068AB08`)
- `[RESOLVED] OQ-10 - Is Dominator separate from ordinary lighting? -> Yes, separate offsets and branch checks.` (evidence: `0x0068AB26..0x0068AC9A`, `0x0053B400`)
- `[RESOLVED] OQ-11 - How are cells initialized at load? -> `MapClass__InitCellAttributes` calls `FUN_00483E30`, which calls `FUN_00484180`.` (evidence: `0x00568C90`, `0x00483E30`)
- `[RESOLVED] OQ-12 - What are clamp boundaries? -> High clamp `>1999 -> 2000`; low clamp `<1 -> 0`.` (evidence: `0x00484180`, `0x005558E0`)
- `[DEFERRED] OQ-13 - What exact palette table does `LightConvertClass` build from normalized RGB?` (category: `out-of-scope`; reason: this slice stops at ScenarioClass field consumers and cell-field values; next-step-if-pursued: investigate `FUN_00544E70` and `LightConvertClass__Constructor`)
- `[DEFERRED] OQ-14 - What are the complete Lightning Storm/Psychic Dominator transition timelines?` (category: `out-of-scope`; reason: this slot only separates variants from ordinary map lighting; next-step-if-pursued: use LS/PD superweapon reports)
- `[RESOLVED] OQ-15 - Does current Rust implement the same ordinary formula? -> No, Rust multiplies ambient by `(1-ground)`.` (evidence: `src/map/lighting.rs:81`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary map brightness is additive: `ambient + level*z - ground`, scaled through binary integer fields | `0x00484180`, `0x0068A817..0x0068A984` | mismatch | `src/map/lighting.rs::cell_tint` | Use additive semantics for ordinary map ambience | `lighting_adds_ground_offset_instead_of_multiplying_ambient`: Ambient `0.8`, Ground `0.2`, Level `0`, z `0` should produce `0.6`, not `0.64` | Do not keep `ambient * (1-ground)` as a parity formula |
| `Ambient` is copied into three ScenarioClass slots; immediate ordinary consumer uses `+0x352C` | `0x0068A846..0x0068A868`, `0x00484180` | missing model distinction | lighting config/data model | Keep room for current/target ambient if dynamic transitions are added later | `lighting_ambient_parse_updates_all_internal_slots` as data-model unit test | Do not collapse dynamic transition slots if implementing superweapon lighting |
| `Ground` and `Level` use 250-scale, not 100-scale | assembly `0x0068A905..0x0068A984` | Rust direct f32; no exact integer tests | `src/map/lighting.rs::parse_lighting` tests | Add tests around binary-equivalent conversion boundaries | `lighting_level_0032_maps_to_internal_8`; `lighting_ground_0004_maps_to_internal_1` | Do not round with Rust default float formatting assumptions; binary adds `0.01` then `ftol` |
| Missing `[Lighting] Ground` binary reset default is external `0.2`, while FinalAlert template supplies `0.0` | `0x00683610`, FinalAlert `StdMapRA2.ini` | Rust default is `0.0` | `LightingConfig::default` / parse fallback | Decide whether fallback should emulate constructor or map-template behavior | `lighting_missing_ground_uses_binary_constructor_default` if emulating binary | Do not claim `0.0` is the binary missing-key default |
| Brightness and RGB clamp to external `2.0`, lower values `<=0` become `0`, and RGB normalization can move dominant color into scale | `0x00484180`, `0x005558E0` | partial: cap exists, order differs | `src/map/lighting.rs` | Preserve clamp order if implementing cell LightConvert parity | `lighting_rgb_dominant_channel_normalizes_to_1000_scale` | Do not treat RGB channels as independent unclamped floats through render |
| Ion/Nuke/Dominator fields are not ordinary map ambience | `0x00484180`, `0x0053A100`, `0x0053B400`, `0x0053A110` | Rust ignores variants, okay for ordinary static maps | future superweapon lighting surface | Keep variants separate until dynamic superweapon system needs them | `ordinary_lighting_ignores_dominator_fields_without_active_pd_flag` | Do not let Dominator defaults tint ordinary maps |

Stale Docs / Follow-up Docs:

- Any doc or code comment saying ordinary map lighting is `ambient * (1-ground)` should be replaced with: "gamemd computes ordinary cell brightness as ambient plus local light contribution plus `Level * signed cell level` minus `Ground`, then clamps in 1000-scale units."
- `CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md` is still broadly useful, but its statement that `Cell_ComputeZAdjust` is the normal map-load path should not be inferred. Normal load uses `FUN_00483E30` / `FUN_00484180`; `Cell_ComputeZAdjust` is the dynamic recompute path.

## Negative Facts / Do Not Do

- Do not implement ordinary `[Lighting]` as Lightning Storm, Ion Storm, or Psychic Dominator lighting.
- Do not use `ambient * (1-ground)` as the binary parity formula.
- Do not assume the missing-key `Ground` default is `0.0`; that is the FinalAlert template value, not the constructor reset.
- Do not let `Ion*` or `Dominator*` fields affect normal maps unless the matching dynamic branch is active.
- Do not use unsigned cell level for lighting; the binary reads `CellClass+0x11B` as signed.

## Remaining Uncertainty

- Exact `LightConvertClass` palette construction from normalized brightness/RGB was not investigated here.
- Semantic distinction between `+0x3528`, `+0x352C`, and `+0x3530` needs a dynamic transition investigation if superweapon lighting interpolation is implemented.
- Full draw-call use of cell fields `+0x104..+0x114` is covered by other docs only partially; this report stops at producing those fields.

## Sources

- Ghidra decompiled/read-only: `0x00689E90`, `0x00686B20`, `0x006832C0`, `0x00683610`, `0x00484180`, `0x00483E30`, `0x00484050`, `0x00484680`, `0x004AE450`, `0x004AE4C0`, `0x005558E0`, `0x00568C90`, `0x00545000`, `0x00554AF0`, `0x00554D50`, `0x0053A100`, `0x0053B400`, `0x0053A110`, `0x0053AD00`.
- Assembly contexts: `0x0068A817..0x0068AC9A`, `0x005558E0..0x00555A9E`.
- Existing docs referenced: `CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md`, `MAP_LIGHTING_AND_LIGHT_POSTS_SYSTEM_MODEL_SYNTHESIS.md`, `FUN_00483E30_BRIDGE_Z_AT_MAP_LOAD_GHIDRA_REPORT.md`.
- INI/template checked: `C:/Users/enok/Documents/Command and Conquer Red Alert II/FinalAlert2/StdMapRA2.ini`.
- Rust scanned: `C:/Users/enok/Documents/ra2-rust-game/src/map/lighting.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_init.rs`.
