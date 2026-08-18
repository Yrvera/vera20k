# BuildingTypeClass::Dimension2 Bracket Extents - Ghidra Research Report

**Address(es):** `0x00464AF0` primary; related `0x0045EC90`, `0x0045ECA0`, `0x0045B080`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact `Dimension2` contract used by building selection brackets and building health pips: foundation table dimensions, Z extent, defaults, bib exclusion, and exclusion of other Z fields.  
**Non-Scope:** bracket edge topology, line rasterization, body draw depth, general `BuildingTypeClass` layout, and full art/rules parser behavior beyond the fields that feed this function.  
**Confidence:** High  
**Active in YR:** Yes. Evidence: `TechnoClass::DrawBehind` (`0x006F60D0`) and `TechnoClass::DrawExtras` (`0x006F5190`) call the object type vtable `+0x7C` after checking `WhatAmI()==6` and selected state; no TS-only flag gates this call path.

## 1. Overview

`BuildingTypeClass::Dimension2` returns a three-int coordinate extent used by the building bracket and building pip code. It is a pure field/table read: X and Y come from foundation width/height tables scaled to leptons, and Z comes from the art `Height` field multiplied by the runtime `g_HeightFactor`.

For player-visible building selection boxes, this means the line-drawn wireframe uses only the art foundation footprint and art height. Bibs, `ExtraZAdjust`/`ZAdjust`, draw offsets, animation ZAdjust fields, and occupied-height fields do not enlarge or shrink the box returned by this function.

## 2. Key Offsets And Globals

| Field / global | Offset / address | Use in `Dimension2` | Active in YR |
|---|---:|---|---|
| `BuildingTypeClass.Foundation` id | `this+0xEF0` | direct index into width and height tables | Yes - read at `0x00464B07..0x00464B11` |
| `BuildingTypeClass.Height` | `this+0xEF4` | multiplied by `g_HeightFactor` for Z extent | Yes - read at `0x00464AF6` |
| `g_FoundationWidthTable` | `0x008192B8` | table result shifted left 8 | Yes - read at `0x00464B0B` |
| `g_FoundationHeightTable` | `0x00819310` | table result shifted left 8 | Yes - read at `0x00464B11` |
| `g_HeightFactor` | `0x0089DDB8` | runtime scalar for `Height` -> Z leptons | Yes - read at `0x00464AFC`, written by `HeightFactor_Init` |
| `HasBib` | `this+0x1570` | not read by `Dimension2` | No for this function - only separate `GetFoundationHeight` can use it |
| body `ZAdjust` / `ExtraZAdjust` | known body-depth field around `this+0x1548` from DrawBody docs | not read by `Dimension2` | No for this function |
| `OccupyHeight` | `this+0xEF8` | not read by `Dimension2` | No for this function |

Constructor defaults verified in `BuildingTypeClass__constructor` (`0x0045DD90`): `Foundation` defaults to `0`, `Height` defaults to `2`, and `OccupyHeight` defaults to `2` (`param_1[0x3BC]=0`, `[0x3BD]=2`, `[0x3BE]=2`).

## 3. Core Contract

`Dimension2(this, out)` writes:

```text
out.x = g_FoundationWidthTable[this.Foundation] << 8
out.y = g_FoundationHeightTable[this.Foundation] << 8
out.z = this.Height * g_HeightFactor
return out
```

Important exact details:

- The same `Foundation` id at `this+0xEF0` indexes both width and height tables.
- There is no bounds check before indexing either table.
- X/Y scaling is `<< 8`, so cell counts become leptons with `256` leptons per cell.
- Z is not shifted by 8. It is a direct integer multiply of the parsed `Height` field by runtime `g_HeightFactor`.
- The function does not inspect the building instance; it uses only the type pointer.
- The output pointer passed by the caller is returned unchanged after the three writes.
- There are no conditional branches inside `Dimension2`.

**Active in YR:** Yes. Evidence: DrawBehind and DrawExtras both reach vtable `+0x7C` for selected buildings; the vtable entry at `0x007E45EC` contains `0x00464AF0`.

## 4. Foundation Table Mapping

The foundation string parser helper at `0x00474DA0` reads `Foundation` and returns the id from the table at `0x0081B9D8`. `Dimension2` itself only consumes the numeric id.

| Id | Parser string | Width table | Height table | `Dimension2` X,Y leptons |
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

**Evidence:** static memory at `0x008192B8` and `0x00819310`; parser table/string memory at `0x0081B9D8` / `0x0081BB68`; parser helper decompile `0x00474DA0`.

## 5. Height And Runtime Factor

`Height` is read from art data into `this+0xEF4` in the `BuildingTypeClass_ReadINI_Water` parser path: assembly around `0x004610D8..0x00461101` loads the previous `+0xEF4`, pushes the `Height` string at `0x0081A7A8`, calls the INI read helper, and stores the result back to `+0xEF4`.

`g_HeightFactor` is not a compile-time table value. Static memory at `0x0089DDB8` is zero in the program image, and `HeightFactor_Init` (`0x0045B080`) writes it at runtime after a sine lookup and `Math__ftol`. `Dimension2` reads that runtime global directly.

**Active in YR:** Yes. Evidence: no YR/TS flag in `Dimension2`; runtime height factor is read on the selected-building bracket paths in `DrawBehind`, `DrawExtras`, and building health pips.

## 6. Bib And Other Z Fields

Bib is excluded from this function. `Dimension2` does not call `BuildingTypeClass__GetFoundationHeight` and does not read `this+0x1570`. The separate helper at `0x0045ECA0` has a `param_2 != 0 && HasBib` branch that returns table height + 1, but `Dimension2` bypasses that helper and reads `g_FoundationHeightTable[Foundation]` directly.

Other building type Z-like fields are excluded from this function. The only type fields read are `+0xEF0` and `+0xEF4`. In particular, body depth/Z-adjust fields used by DrawBody are not part of the selection bracket extent returned by `Dimension2`; they may affect render ordering elsewhere, but not the geometry dimensions returned here.

**Active in YR:** Yes for the exclusion. Evidence: primary decompile `0x00464AF0` has only the three reads above; `GetFoundationHeight` bib logic at `0x0045ECA0` is not called from this function.

## 7. Integration Points

| Caller context | Status | Evidence | What remains |
|---|---|---|---|
| Building bracket back edges | verified | `TechnoClass::DrawBehind` `0x006F60D0` calls object type vtable `+0x7C` after RTTI 6 and selected checks | none for extents |
| Building bracket front edges | verified | `TechnoClass::DrawExtras` `0x006F5190` calls object type vtable `+0x7C` after selected and RTTI 6 checks | none for extents |
| Building health pips | verified | `TechnoClass::DrawHealthBar` `0x006F64A0` calls object type vtable `+0x7C` when `WhatAmI()==6` | none for extents |
| Vtable binding | verified | memory at `0x007E45EC` begins with `F0 4A 46 00`, i.e. `0x00464AF0` at vtable `+0x7C` | none |

## 8. Current Rust Implementation Status

`src/app_selection_brackets.rs` currently builds building bracket boxes from parsed foundation strings and art height. That broadly matches the contract that X/Y derive from `Foundation` and Z derives from art `Height`, but the Rust path parses `Foundation` as generic `WxH` text rather than using the original foundation id table, including the special `3x3Refinery` and `0x0` entries. The source comment in `src/rules/art_data.rs` currently says `Dimension2.Z = (fh + Height) * 256`; that does not match this binary slice, where `Dimension2.z = Height * g_HeightFactor`.

No Rust files were changed in this investigation.

## 9. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingTypeClass::Dimension2` | verified | decompile `0x00464AF0` | none |
| Foundation width lookup | verified | `0x00464B0B`, static table `0x008192B8` | none |
| Foundation height lookup | verified | `0x00464B11`, static table `0x00819310` | none |
| Height-derived Z extent | verified | `0x00464AF6..0x00464AFC`, parser evidence `0x004610D8..0x00461101` | none |
| Runtime height factor | verified | `0x0045B080` writes `g_HeightFactor`; `0x00464AFC` reads it | exact runtime numeric value depends on initialized tactical math context |
| Bib exclusion | verified | `0x00464AF0` does not read `+0x1570`; separate `0x0045ECA0` bib branch not called | none |
| ExtraZAdjust / other Z fields exclusion | verified | `0x00464AF0` reads no field besides `+0xEF0/+0xEF4` | none for this function |
| Active YR bracket call path | verified | `0x006F60D0`, `0x006F5190`, vtable `0x007E45EC` | none for extents |

## 10. Open Questions - Final State

[RESOLVED] OQ1 - Does `Dimension2` include bib height? No; it directly reads `g_FoundationHeightTable[Foundation]` and never reads `HasBib` (`+0x1570`). Evidence: `0x00464AF0`, contrast `0x0045ECA0`.

[RESOLVED] OQ2 - Does `Dimension2` use `ExtraZAdjust`, `ZAdjust`, `OccupyHeight`, draw offsets, or animation ZAdjust fields? No; only `+0xEF0`, `+0xEF4`, `g_HeightFactor`, and the two foundation tables are read. Evidence: `0x00464AF0`.

[RESOLVED] OQ3 - Is the bracket path active in standard YR? Yes; selected RTTI 6 objects in DrawBehind/DrawExtras call vtable `+0x7C` with no TS-only gate. Evidence: `0x006F60D0`, `0x006F5190`, `0x007E45EC`.

[RESOLVED] OQ4 - Is the foundation table just rectangular string parsing? No; parser maps strings through a fixed id table, including `3x3Refinery` and `0x0`; `Dimension2` consumes the id and table dimensions. Evidence: `0x00474DA0`, `0x0081B9D8`, `0x0081BB68`.

[RESOLVED] OQ5 - Is the Z extent `(foundation height + Height) * something`? No for `Dimension2`; Z is `Height * g_HeightFactor`. Evidence: `0x00464AF6..0x00464AFC`.

## Sources

- Ghidra decompile `0x00464AF0` - `BuildingTypeClass::Dimension2`
- Ghidra decompile `0x0045EC90` - foundation width helper
- Ghidra decompile `0x0045ECA0` - foundation height helper with separate bib branch
- Ghidra decompile `0x0045B080` - `HeightFactor_Init`
- Ghidra decompile `0x00474DA0` - foundation string-to-id helper
- Ghidra decompile `0x006F60D0`, `0x006F5190`, `0x006F64A0` - bracket/pip call contexts
- Static memory reads: `0x008192B8`, `0x00819310`, `0x0081B9D8`, `0x0081BB68`, `0x007E45EC`
- Existing docs checked: `SELECTION_BRACKETS_GHIDRA_REPORT.md`, `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md`
- Rust implementation checked: `src/app_selection_brackets.rs`, `src/rules/art_data.rs`
