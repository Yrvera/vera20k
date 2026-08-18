# BuildingType Light Keys ReadINI Constants - Ghidra Report

Date: 2026-05-22

**Investigation Mode:** exhaustive-slice  
**Target:** `BUILDINGTYPE_LIGHT_KEYS_READINI_CONSTANTS`  
**Primary function:** `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`  
**Claimed scope:** exact read sites/constants for `LightVisibility`, `LightIntensity`, `LightRedTint`, `LightGreenTint`, `LightBlueTint`, and `ExtraLight`.  
**Non-scope:** LightSource falloff, LightConvert recompute/cache, map `[Lighting]`, spotlight rendering, and final renderer integration.  
**Overall confidence:** HIGH for reader addresses, owner section, offsets, types, constants, and `0,01` handling.  
**Active in YR:** Yes. This is the standard `BuildingTypeClass` INI reader used by retail YR building types.

## Target Question

Verify the exact `BuildingTypeClass::ReadINI` parser sites/constants for the five lamp keys and `ExtraLight`: reader address/range, target offset, field type, scale/multiply behavior, malformed float behavior where visible, and whether the key is read from the building's rules section or the image/art fallback section.

## Non-goals

- Do not investigate LightSource cell contribution or dynamic dirty scheduling.
- Do not re-open whether `LightIntensity != 0` is the allocation gate.
- Do not re-open whether `ExtraLight` is draw-depth instead of RGB ambience.
- Do not mutate Ghidra, Rust, INI, `.swarm-claims.md`, or existing docs.

## Evidence Needed To Mark COMPLETE

- String xref / immediate evidence for all six target keys.
- Disassembly around each read site, not just decompiler prose.
- Constructor/default evidence for each target field.
- Constants backing the float-to-internal conversion.
- Parser helper evidence for malformed `ReadDouble` values.
- Section-owner evidence showing rules self-section vs image/art fallback.

## Stop Conditions

- Stop after the six target key reads are classified.
- Stop if a related light/render key appears outside this target; list it as non-scope.
- Stop before following consumers beyond already-settled facts needed for handoff.

## Verified Findings

### 1. Section ownership split

| Key(s) | Reader section pointer | Evidence | Active in YR |
|---|---|---|---|
| `LightVisibility`, `LightIntensity`, `LightRedTint`, `LightGreenTint`, `LightBlueTint` | `this + 0x24` building rules section | `0x0046049C` sets `EBX = [EBP+0x24]`; all five read sites push `EBX`; Ghidra decompile uses `iVar21 = param_1 + 0x24` | Yes |
| `ExtraLight` | `this + 0x1F8` image/art section | `0x004610DE` sets `EDI = [EBP+0x1F8]`; `0x004613F6` pushes `EDI` for `ExtraLight` | Yes |

`ExtraLight` is not read from the rules building section. The five lamp keys are not read from the image/art fallback section.

### 2. Exact read sites and offsets

| Key | String VA | Read site | Helper | Default source | Store | Type / internal units | Active in YR |
|---|---:|---:|---|---|---|---|---|
| `LightVisibility` | `0x0081A92C` | `0x00460C93..0x00460CA0` | `CCINIClass__ReadInt @ 0x005276D0` | current `dword [this+0xE30]` | `dword [this+0xE30]` | signed/atoi-style int leptons, no scale | Yes |
| `LightIntensity` | `0x0081A91C` | `0x00460CA6..0x00460CE9` | `CCINIClass__ReadDouble @ 0x005283D0` | current `dword [this+0xE34] / 1000` | `dword [this+0xE34]` | integer milli-units: `ftol(value * 1000.0 + 0.1)` | Yes |
| `LightRedTint` | `0x0081A90C` | `0x00460CEF..0x00460D32` | `CCINIClass__ReadDouble @ 0x005283D0` | current `dword [this+0xE38] / 1000` | `dword [this+0xE38]` | integer milli-units on explicit reads: `ftol(value * 1000.0 + 0.1)` | Yes |
| `LightGreenTint` | `0x0081A8FC` | `0x00460D38..0x00460D7B` | `CCINIClass__ReadDouble @ 0x005283D0` | current `dword [this+0xE3C] / 1000` | `dword [this+0xE3C]` | integer milli-units on explicit reads: `ftol(value * 1000.0 + 0.1)` | Yes |
| `LightBlueTint` | `0x0081A8EC` | `0x00460D81..0x00460DC4` | `CCINIClass__ReadDouble @ 0x005283D0` | current `dword [this+0xE40] / 1000` | `dword [this+0xE40]` | integer milli-units on explicit reads: `ftol(value * 1000.0 + 0.1)` | Yes |
| `ExtraLight` | `0x0081A650` | `0x004613E9..0x00461401` | `CCINIClass__ReadInt @ 0x005276D0` | sign-extended `word [this+0x1548]` | low word of `AX` to `[this+0x1548]` | signed 16-bit draw-depth/Z adjust, no scale | Yes |

The six string immediates have one direct `.text` hit each in the checked retail binary: `0x00460C94`, `0x00460CCE`, `0x00460D17`, `0x00460D60`, `0x00460DA7`, and `0x004613F2` are the immediate bytes inside the corresponding `PUSH imm32` instructions.

### 3. Float scaling constants

The five double-valued lamp fields use the same pattern:

1. Convert the current stored integer to a default double by signed division by `1000`.
2. Call `CCINIClass__ReadDouble`.
3. Multiply ST0 by the double at `0x007E4658`.
4. Add the double at `0x007E3860`.
5. Call `Math__ftol @ 0x007C5F00`.
6. Store the returned integer to the field.

Verified constants:

| VA | Bytes | Double value | Use | Active in YR |
|---:|---|---:|---|---|
| `0x007E4658` | `00 00 00 00 00 40 8F 40` | `1000.0` | storage multiplier after `ReadDouble` | Yes |
| `0x007E3860` | `9A 99 99 99 99 99 B9 3F` | `0.1` | positive bias before `ftol` | Yes |

The default division uses `IMUL 0x10624DD3`, `SAR EDX,0x6`, and sign correction before `FILD`, which is the compiler's signed divide-by-1000 sequence. Active in YR: Yes.

Examples implied by the binary:

| INI value | Stored internal int |
|---:|---:|
| `0.2` | `200` |
| `0.01` / `.01` | `10` |
| `1.5` | `1500` |
| `-0.15` | approximately `-149` after `*1000 + 0.1` and `ftol` |
| `0,01` | `0`, because `ReadDouble` parses the leading `0` |

Tiny detail: `+0.1` is applied even for negative values. Do not replace this with symmetric rounding.

### 4. Constructor defaults

`BuildingTypeClass__constructor @ 0x0045DD90` initializes the target fields before INI reads:

| Offset | Constructor write | Default | Active in YR |
|---:|---|---:|---|
| `+0xE30` | `0x0045DE12: MOV [ESI+0xE30],0x1388` | `5000` | Yes |
| `+0xE34` | `0x0045DE21: MOV [ESI+0xE34],EBX` | `0` | Yes |
| `+0xE38` | `0x0045DE27: MOV [ESI+0xE38],EAX`, after `EAX=0xF4240` | `1000000` | Yes |
| `+0xE3C` | `0x0045DE2D: MOV [ESI+0xE3C],EAX` | `1000000` | Yes |
| `+0xE40` | `0x0045DE33: MOV [ESI+0xE40],EAX` | `1000000` | Yes |
| `+0x1548` | `0x0045DFCC` word zero write, per prior ctor report | `0` | Yes |

Important oddity: if a tint key is missing, the reader's default path divides the constructor value by `1000` and then stores `default * 1000`, preserving `1000000`. If the key is explicitly present as `1.0`, the store becomes `1000`. This is a verified parser quirk; this report does not claim the later light-compute normalization semantics.

### 5. Malformed float handling

`CCINIClass__ReadDouble @ 0x005283D0` uses `sscanf(raw, "%f", &temp)` and does not require full-string consumption. It then checks for a percent sign and scales by `0.01` if present.

For the stock typo `LightGreenTint=0,01`, standard C `%f` parses the leading `0` and stops at the comma, so the value returned to the caller is `0.0`. The key is present, so the caller does not fall back to the default. Active in YR: Yes, for stock lamp entries containing `0,01` in `rules.ini` and `rulesmd.ini`.

For completely non-numeric strings, this slice did not prove a stable fallback because the decompile shows no explicit `sscanf` return check. That case is a mod edge case and remains outside this target.

### 6. `ExtraLight` integer semantics

The `ExtraLight` reader sequence is:

- `0x004613E9`: `MOVSX EDX, word ptr [EBP+0x1548]`
- `0x004613F0`: push signed current value as default
- `0x004613F1`: push `"ExtraLight"`
- `0x004613F6`: push `EDI` (`this+0x1F8`, image/art section)
- `0x004613FC`: call `CCINIClass__ReadInt`
- `0x00461401`: `MOV word ptr [EBP+0x1548], AX`

This proves signed-default input and 16-bit truncating storage. `ReadInt` itself supports `$` hex strings, trailing `h` hex strings, and decimal `atoi` fallback. Active in YR: Yes.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|---|
| `LightVisibility` is a rules-section int, default `5000`, no scaling | `0x00460C93..0x00460CA0`; ctor `0x0045DE12` | Rust defaults to `0` | `src/rules/object_type.rs` | A building section missing `LightVisibility=` but with light intensity inherits radius `5000` | `building_light_visibility_missing_defaults_to_5000` | Do not use default visibility as the allocation gate |
| `LightIntensity` and tint explicit values store as `ftol(value * 1000.0 + 0.1)` | `0x00460CA6..0x00460DC4`; constants `0x007E4658/0x007E3860` | Rust stores raw `f32` | rules parser/light data model | Lamp values like `0.2`, `0.01`, and `1.5` round to `200`, `10`, and `1500` internal units before later light math | `building_light_double_keys_store_milli_units_with_point_one_bias` | Do not use symmetric rounding or direct floats for parity-sensitive tests |
| Missing tint keys preserve ctor default `1000000`; explicit `1.0` stores `1000` | ctor `0x0045DE27..0x0045DE33`; reader divide/multiply path | Rust treats both as visible `1.0` | rules parser/light data model | A mod fixture can distinguish missing `LightRedTint` from explicit `LightRedTint=1.0` in raw parsed data | `building_light_tint_missing_default_differs_from_explicit_one` | Do not silently normalize this away if later binary math depends on raw units |
| `LightGreenTint=0,01` parses to `0`, not `0.01` and not default | `ReadDouble @ 0x005283D0`; stock `rulesmd.ini` typo | Rust `parse::<f32>()` fails and falls back | `src/rules/ini_parser.rs`, object light parser | Stock green invisible lamp typo produces zero green tint | `building_light_comma_decimal_parses_leading_zero` | Do not "fix" Westwood's comma typo for gamemd parity |
| `ExtraLight` is image/art-section signed i16, no `/1000` scale, stored by truncating to word | `0x004613E9..0x00461401`; prior drawbody report | Rust parses `i32` and applies RGB brightness | `src/rules/art_data.rs`, `src/map/lighting.rs`, `src/app_init.rs`, render depth code | `GAARTY ExtraLight=350` affects building depth only and does not change RGB lighting | `building_extra_light_is_signed_i16_depth_not_rgb` | Do not feed `ExtraLight` into `LightSource`, `LightConvert`, or `LightingGrid` |

## Negative Facts / Do Not Do

- Do not read the five lamp keys from art/image fallback; the binary pushes `this+0x24`.
- Do not read `ExtraLight` from the rules building section; the binary pushes `this+0x1F8`.
- Do not treat `ExtraLight` as a float or divide it by `1000`.
- Do not parse `0,01` as `0.01` for gamemd parity.
- Do not assume missing tint and explicit `1.0` have identical raw `BuildingTypeClass` storage.
- Do not replace the binary's `value * 1000.0 + 0.1` with ordinary `round(value * 1000.0)`.

## Remaining Uncertainty

- Exact downstream light-compute interpretation of the raw tint quirk (`1000000` missing default vs `1000` explicit `1.0`) belongs to the cell-light compute and LightSource reports. This report only proves parser storage.
- Completely non-numeric `ReadDouble` strings are not pinned to a deterministic fallback in this slice because the helper does not visibly check `sscanf`'s return value.
- `ReadInt` out-of-range `ExtraLight` mod values are stored low-word and later sign-extended by consumers; this is verified for storage shape, but no runtime mod acceptance case was tested.

## Coverage Ledger

| Area | Status | Evidence | What remains |
|---|---|---|---|
| `LightVisibility` reader | verified | `0x00460C93..0x00460CA0` | none |
| `LightIntensity` reader | verified | `0x00460CA6..0x00460CE9` | none |
| `LightRedTint` reader | verified | `0x00460CEF..0x00460D32` | none |
| `LightGreenTint` reader | verified | `0x00460D38..0x00460D7B` | none |
| `LightBlueTint` reader | verified | `0x00460D81..0x00460DC4` | none |
| `ExtraLight` reader | verified | `0x004613E9..0x00461401` | none |
| Constructor defaults | verified | `0x0045DE12..0x0045DE33`; prior ctor report for `+0x1548` | none |
| `ReadDouble` malformed comma behavior | verified for `0,01` | `0x005283D0`, `%f` parse | totally non-numeric strings deferred |
| Current Rust scan | verified | `src/rules/object_type.rs`, `src/rules/ini_parser.rs`, `src/rules/art_data.rs`, `src/map/lighting.rs`, `src/app_init.rs` | no code edits made |

## Open Questions - Final State

- `[RESOLVED] OQ-1 - Which function reads the six keys? -> BuildingTypeClass_ReadINI_Water @ 0x0045FE50.` (evidence: read sites listed above)
- `[RESOLVED] OQ-2 - Are lamp keys self/rules or image/art? -> self/rules section at this+0x24.` (evidence: `0x0046049C`, `0x00460C93..0x00460DAE`)
- `[RESOLVED] OQ-3 - Is ExtraLight self/rules or image/art? -> image/art section at this+0x1F8.` (evidence: `0x004610DE`, `0x004613F6`)
- `[RESOLVED] OQ-4 - What constants scale LightIntensity/tints? -> divide stored default by 1000; store `ftol(ReadDouble * 1000.0 + 0.1)`.` (evidence: `0x00460CAC..0x00460DC4`, `0x007E4658`, `0x007E3860`)
- `[RESOLVED] OQ-5 - What happens to stock `0,01`? -> `ReadDouble` returns leading `0.0`; caller stores `0`.` (evidence: `0x005283D0`)
- `[RESOLVED] OQ-6 - What is ExtraLight type? -> signed current default, 16-bit stored field.` (evidence: `0x004613E9..0x00461401`)
- `[DEFERRED] OQ-7 - How does cell compute normalize the raw tint default quirk?` (category: out-of-scope; reason: downstream consumer, not parser constants; next-step-if-pursued: reconcile against `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-8 - What does totally invalid `ReadDouble` input do?` (category: bounded-cost-too-high; reason: not a stock case and helper lacks return check; next-step-if-pursued: runtime debugger or controlled binary fixture)

## Sources

- Ghidra read-only decompile/disassembly:
  - `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`
  - `CCINIClass__ReadDouble @ 0x005283D0`
  - `CCINIClass__ReadInt @ 0x005276D0`
  - `Math__ftol @ 0x007C5F00`
  - `BuildingTypeClass__constructor @ 0x0045DD90`
- Binary constants checked at VA:
  - `0x007E4658 = 1000.0`
  - `0x007E3860 = 0.1`
- Existing docs referenced:
  - `LIGHT_RULES_ART_PARSER_DEFAULTS_GHIDRA_REPORT.md`
  - `BUILDINGTYPE_EXTRALIGHT_0X1548_CONSUMER_GHIDRA_REPORT.md`
  - `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`
  - `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`
- INI/Rust scanned:
  - `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
  - `src/rules/object_type.rs`
  - `src/rules/ini_parser.rs`
  - `src/rules/art_data.rs`
  - `src/map/lighting.rs`
  - `src/app_init.rs`

## Status

COMPLETE for the bounded parser/constants target.
