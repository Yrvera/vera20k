# Foundation Parser/Table Bracket Extents - Ghidra Research Report

**Address(es):** `0x00474DA0` parser helper; `0x00461225..0x00461257` building parser call chain; `0x00464AF0` `Dimension2`; tables `0x0081B9D8`, `0x008192B8`, `0x00819310`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** BuildingTypeClass `Foundation=` string-to-id mapping, static width/height tables, defaults and invalid fallbacks as they affect selected-building bracket extents.  
**Non-Scope:** full building placement, AddOccupy/RemoveOccupy, bib footprint, pathing occupancy, and full CCINI load-order internals beyond the visible per-section read sequence.  
**Confidence:** High  
**Active in YR:** Yes - `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` is the active BuildingType parser, and selected building brackets consume the parsed id through `Dimension2 @ 0x00464AF0`.

## 1. Overview

Selected-building bracket X/Y extents are not parsed by splitting `Foundation=` as generic `WxH` text. The binary maps the string through a fixed 22-entry enum table, stores the enum id at `BuildingTypeClass+0xEF0`, then `Dimension2` indexes width/height tables and shifts the cell counts by 8 to produce lepton extents.

The parser path is two-stage for buildings: it reads `Foundation` from the art/image section first, then from the building/rules section using the art result as the default. This second pass has a zero-id edge case: because id `0` is both valid `1x1` and the parser's invalid/unmatched return value, the second pass only stores nonzero results.

## 2. Key Offsets And Globals

| Field / global | Offset / address | Purpose | Active in YR |
|---|---:|---|---|
| `BuildingTypeClass.Foundation` id | `this+0xEF0` | enum id used by bracket extent tables | Yes - read by `0x00464AF0`; written by parser at `0x00461248/0x00461257` |
| `BuildingTypeClass.Height` | `this+0xEF4` | Z extent source, independent of foundation table | Yes - parsed at `0x004610D8..0x00461101`; read by `0x00464AF6` |
| foundation string/id table | `0x0081B9D8` | 22 entries: pointer to string plus id | Yes - walked by `0x00474DA0` |
| width table | `0x008192B8` | integer cell width by foundation id | Yes - indexed at `0x00464B0B` |
| height table | `0x00819310` | integer cell height by foundation id | Yes - indexed at `0x00464B11` |

Constructor defaults in `BuildingTypeClass__constructor @ 0x0045DD90`: `Foundation` starts as id `0`, `Height` starts as `2`, and `OccupyHeight` starts as `2` (`param_1[0x3BC]=0`, `[0x3BD]=2`, `[0x3BE]=2`). Active in YR: Yes; these are constructor writes before INI parsing.

## 3. Parser Logic

`FUN_00474DA0(section, key, default_id)` does this narrow operation:

```text
default_string = foundation_table[default_id].string
value = ReadString(section, key, default_string, max_len=0x20)
for each entry in foundation_table[0..21]:
    if case_insensitive_compare(value, entry.string) == 0:
        return entry.id
return 0
```

Important details:

- The loop starts at `0x0081B9D8` and stops when the entry pointer reaches `0x0081BA88`, so it examines exactly 22 entries of 8 bytes each.
- The string buffer is 32 bytes (`0x20`), enough for `3x3Refinery`.
- Matching uses `FUN_007C8D20`, a case-insensitive comparison. `Foundation=3X3` in `ini/artmd.ini:11325` and `ini/artmd.ini:11354` therefore maps to id `6`.
- No numeric parsing occurs; `10x10` or malformed text does not become dimensions and returns `0`.
- Returning `0` is ambiguous: it is both valid id `0` (`1x1`) and the failure return.

Active in YR: Yes. Evidence: `BuildingTypeClass_ReadINI_Water @ 0x00461237` and `0x0046124E` call `0x00474DA0`; no TS-only gate appears on the building parser path.

## 4. Building Read Order And Fallbacks

The building parser calls `0x00474DA0` twice:

```text
art_id = parse_foundation(art_section, "Foundation", current_id)
this.Foundation = art_id
rules_id = parse_foundation(rules_section, "Foundation", art_id)
if rules_id != 0:
    this.Foundation = rules_id
```

Evidence:

- `0x00461225..0x00461237`: pushes current `+0xEF0`, key `0x0081A734` (`Foundation`), and `EDI`; calls `0x00474DA0`.
- `0x00461248`: stores the first result unconditionally to `+0xEF0`.
- `0x0046123C..0x0046124E`: pushes first result as the second call's default, key `Foundation`, and `EBX`; calls `0x00474DA0`.
- `0x00461253..0x00461257`: stores the second result only if it is nonzero.
- Nearby setup identifies `EDI` as the art/image section base (`LEA EDI,[EBP+0x1F8]` at `0x004610DE`) and `EBX` as the main building section base (`LEA EBX,[EBP+0x24]` earlier in the same parser).

Fallback consequences:

| Scenario | Resulting id | Active in YR |
|---|---:|---|
| no art key, no rules key | previous/default id, normally `0` | Yes - `ReadString` uses table default string |
| valid art key, no rules key | art id | Yes |
| invalid art key, no rules key | `0` (`1x1` dimensions) | Yes - first result stored unconditionally |
| valid art key, invalid rules key | art id | Yes - second zero is ignored |
| art key nonzero, rules key `1x1` | art id, not `1x1` | Conditional - only if rules section sets `Foundation=1x1`; branch treats returned zero as no update |
| art key zero/`1x1`, rules key nonzero | rules id | Yes |
| art key zero/`1x1`, rules key `1x1` | `0` | Yes |

This report found no active `Foundation=` entries in `ini/rules.ini` or `ini/rulesmd.ini`; standard content uses art files for this key. Active in YR: Yes for the parser behavior; Conditional for rules-side override edge cases because they require modded or nonstandard rules data.

## 5. Table Mapping To Bracket Dimensions

| Id | Parser string | Width | Height | `Dimension2` X,Y |
|---:|---|---:|---:|---:|
| 0 | `1x1` | 1 | 1 | 256, 256 |
| 1 | `2x1` | 2 | 1 | 512, 256 |
| 2 | `1x2` | 1 | 2 | 256, 512 |
| 3 | `2x2` | 2 | 2 | 512, 512 |
| 4 | `2x3` | 2 | 3 | 512, 768 |
| 5 | `3x2` | 3 | 2 | 768, 512 |
| 6 | `3x3` | 3 | 3 | 768, 768 |
| 7 | `3x5` | 3 | 5 | 768, 1280 |
| 8 | `4x2` | 4 | 2 | 1024, 512 |
| 9 | `3x3Refinery` | 3 | 3 | 768, 768 |
| 10 | `1x3` | 1 | 3 | 256, 768 |
| 11 | `3x1` | 3 | 1 | 768, 256 |
| 12 | `4x3` | 4 | 3 | 1024, 768 |
| 13 | `1x4` | 1 | 4 | 256, 1024 |
| 14 | `1x5` | 1 | 5 | 256, 1280 |
| 15 | `2x6` | 2 | 6 | 512, 1536 |
| 16 | `2x5` | 2 | 5 | 512, 1280 |
| 17 | `5x3` | 5 | 3 | 1280, 768 |
| 18 | `4x4` | 4 | 4 | 1024, 1024 |
| 19 | `3x4` | 3 | 4 | 768, 1024 |
| 20 | `6x4` | 6 | 4 | 1536, 1024 |
| 21 | `0x0` | 0 | 0 | 0, 0 |

`3x3Refinery` is a distinct enum id (`9`) even though its bracket dimensions match `3x3`. `0x0` is also a real enum id (`21`) and produces zero X/Y extents if parsed. In the repo INI snapshot, `Foundation=0x0` appears only commented in `ini/art.ini:9247` etc. and `ini/artmd.ini:13403` etc.; this is data evidence only, not a binary gate.

Active in YR: Yes. Evidence: parser table memory `0x0081B9D8`/strings at `0x0081BB68`; width table memory `0x008192B8`; height table memory `0x00819310`; consumer decompile `0x00464AF0`.

## 6. Dimension2 Consumer

`BuildingTypeClass::Dimension2 @ 0x00464AF0` writes:

```text
out.x = g_FoundationWidthTable[this.Foundation] << 8
out.y = g_FoundationHeightTable[this.Foundation] << 8
out.z = this.Height * g_HeightFactor
```

The same id indexes both tables. There is no bounds check in `Dimension2`, but the parser helper only returns ids `0..21`. The X/Y shift is exactly `<< 8`, so dimensions are table cell counts multiplied by 256 leptons.

Active in YR: Yes. Evidence: selected-building bracket paths call the type vtable `+0x7C`, bound to `0x00464AF0`, in `DrawBehind @ 0x006F60D0` and `DrawExtras @ 0x006F5190`.

## 7. INI And Rust Status

INI:

- `Foundation=` is present throughout `ini/art.ini` and `ini/artmd.ini`.
- `Foundation=` was not found in `ini/rules.ini` or `ini/rulesmd.ini` in this repo snapshot.
- `ini/artmd.ini` contains uppercase `Foundation=3X3` entries; binary comparison accepts them because matching is case-insensitive.
- No active `3x3Refinery` entries were found in the repo INI snapshot, but the binary table supports it.

Current Rust:

- `src/sim/production/production_tech.rs:562` parses by splitting lowercase `x`, clamps both dimensions to at least `1`, and does not support enum-only strings like `3x3Refinery` or `0x0`.
- `src/app_selection_brackets.rs:18` has a separate lowercase-`x` split parser with the same broad limitation for bracket geometry.
- `src/app_render/build_instances.rs:446` currently disables selected-building bracket instance generation, so this mismatch is dormant for visible brackets until re-enabled.

No Rust files were modified.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00474DA0` string-to-id helper | verified | decompile and disassembly `0x00474DA0..0x00474E04` | none for building foundation ids |
| foundation string/id table | verified | memory `0x0081B9D8..0x0081BA88`, strings `0x0081BB68..0x0081BBC4` | none |
| width/height tables | verified | memory `0x008192B8`, `0x00819310` | none |
| building parser call order | verified | assembly context `0x00461225..0x00461257` | none |
| default constructor values | verified | decompile `0x0045DD90` | none |
| `Dimension2` consumption | verified | decompile `0x00464AF0` | none |
| YR md override mechanics | touched-not-exhausted | parser reads the active CCINI object; repo INI search shows key in art/artmd, not rules/rulesmd | exact upstream MIX/INI load precedence is outside this slot |
| terrain caller of `0x00474DA0` | deferred | caller list includes `TerrainTypeClass__ReadINI_Full @ 0x0071DEA0` | out-of-scope; not a selected-building bracket extent consumer |

## 9. Open Questions - Final State

[RESOLVED] OQ1 - Is `Foundation=` parsed as free-form `WxH`? No; it maps through a fixed string/id table. Evidence: `0x00474DA0`, table `0x0081B9D8`.

[RESOLVED] OQ2 - What are the special entries? `3x3Refinery` is id `9` with dimensions `3x3`; `0x0` is id `21` with dimensions `0x0`. Evidence: memory `0x0081B9D8`, `0x008192B8`, `0x00819310`.

[RESOLVED] OQ3 - What is the default? Constructor default is id `0` (`1x1`); missing keys read the default id's string. Evidence: `0x0045DD90`, `0x00474DA0`.

[RESOLVED] OQ4 - What happens on invalid strings? The helper returns `0`; the first building parser call stores that as `1x1`, while the second building parser call ignores zero and preserves the art/default result. Evidence: `0x00474DEE..0x00474DF5`, `0x00461248`, `0x00461253..0x00461257`.

[RESOLVED] OQ5 - Is matching case-sensitive? No. Evidence: comparison helper `0x007C8D20`; uppercase `3X3` in `ini/artmd.ini:11325` is accepted by the binary logic.

[DEFERRED] OQ6 - Does upstream CCINI merge `artmd.ini` over `art.ini` before this parser? Likely yes by engine convention and repo data layout, but this slot only verified the visible per-section reads from the active CCINI object. Category: out-of-scope.

## Sources

- Ghidra decompile/disassembly: `0x00474DA0`, `0x0045FE50`, `0x0045DD90`, `0x00464AF0`, `0x007C8D20`
- Ghidra assembly context: `0x004610DE`, `0x00461225..0x00461257`
- Static memory reads: `0x0081B9D8`, `0x0081BB68`, `0x008192B8`, `0x00819310`
- Existing report checked: `docs/research/building-selection-brackets/BUILDINGTYPE_DIMENSION2_BRACKET_EXTENTS_GHIDRA_REPORT.md`
- INI files checked: `ini/art.ini`, `ini/artmd.ini`, `ini/rules.ini`, `ini/rulesmd.ini`
- Rust files checked: `src/sim/production/production_tech.rs`, `src/app_selection_brackets.rs`, `src/app_render/build_instances.rs`
