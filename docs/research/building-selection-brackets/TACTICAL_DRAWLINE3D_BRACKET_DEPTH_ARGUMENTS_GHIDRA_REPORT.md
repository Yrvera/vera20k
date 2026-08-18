# Tactical DrawLine3D Bracket Depth Arguments - Ghidra Research Report

**Address(es):** `Tactical::DrawLine3D @ 0x006DBB60`; selected building bracket callers `TechnoClass::DrawBehind @ 0x006F60D0`, `TechnoClass::DrawExtras @ 0x006F5190`; helper `TechnoClass::DrawBracketCorner @ 0x006F5EF0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** endpoint depth formula, screen-Y Z adjustment, line color argument, final Z-write flag argument, and how selected building bracket call sites feed endpoints into `DrawLine3D`.  
**Non-Scope:** full surface raster contract, A-buffer/shroud value meanings, unrelated action/rally lines, health pips, object sorting beyond reachability.  
**Confidence:** High for scoped binary findings; medium only for human-readable corner names inherited from prior topology docs.  
**Active in YR:** Yes for the normal selected-building path; conditional only for the dim-color branch.

## 1. Overview

`Tactical::DrawLine3D` is not passed a single flat bracket depth by selected building callers. It projects both 3D endpoints to 2D, subtracts a Z-derived screen-Y adjustment from each endpoint, computes a separate Z-buffer depth for each endpoint from that endpoint's own Z, and forwards the caller's final argument as the surface Z-write flag.

For selected building brackets, both the helper and direct callers pass the final flag as `0`, so bracket pixels can Z-test against the existing Z-buffer but do not write replacement Z values.

## 2. Key Inputs And Offsets

| Source | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `DrawLine3D` arg1 / ECX | `TacticalClass *this` | `0x006DBB60`; reads `this+0xB0/+0xB4` | Yes |
| `DrawLine3D` stack arg1 | endpoint A `CoordStruct*` | `0x006DBB65..0x006DBB79` | Yes |
| `DrawLine3D` stack arg2 | endpoint B `CoordStruct*` | `0x006DBBA0`, `0x006DBBCC..0x006DBBD8` | Yes |
| `DrawLine3D` stack arg3 | surface line color | pushed by callers; forwarded at `0x006DBCB9` | Yes |
| `DrawLine3D` stack arg4 | final surface Z-write flag | pushed by callers as `0`; forwarded at `0x006DBC70` | Yes, value is zero for brackets |
| `Tactical+0xB0/+0xB4` | viewport scroll X/Y subtracted after projection | `0x006DBBB7..0x006DBBC3`, `0x006DBC16..0x006DBC24` | Yes |
| `g_AdjustForZ_Mult @ 0x00B0CD48` | Z-to-screen/depth multiplier | `0x006D2103`, inline at `0x006DBC71`, `0x006DBC93` | Yes |
| `g_PrimarySurface @ 0x00887314` | target surface; vtable `+0x34` | `0x006DBC66`, `0x006DBCC7` | Yes |

## 3. DrawLine3D Projection And Depth Contract

### 3.1 World X/Y Projection

`DrawLine3D` calls `WorldToScreenSub @ 0x006D1EB0` for each endpoint:

```text
sub_x = (x * 0x3C) / 2 + (y * -0x3C) / 2
sub_y = (x * 0x1E) / 2 + (y *  0x1E) / 2
```

The returned subpixel values are converted to screen pixels with signed truncation toward zero by adding `0xFF` to negative values before `SAR 8`. Active in YR: Yes. Evidence: `0x006D1EB0`, first endpoint `0x006DBB80..0x006DBBC3`, second endpoint `0x006DBBDF..0x006DBC24`.

### 3.2 Screen-Y Z Adjustment

For both endpoints, screen Y is:

```text
screen_y = trunc_to_zero(sub_y / 256) - AdjustForZ(endpoint.z) - Tactical.viewport_y
```

`screen_x` is:

```text
screen_x = trunc_to_zero(sub_x / 256) - Tactical.viewport_x
```

`AdjustForZ(z)` returns:

```text
ftol(z * g_AdjustForZ_Mult + (z >= 0x2D8 ? 1 : 0) + 0.5)
```

Active in YR: Yes. Evidence: `Tactical::AdjustForZ @ 0x006D20E0..0x006D211B`; screen-Y subtracts at `0x006DBBB5..0x006DBBC1` and `0x006DBC14..0x006DBC24`.

### 3.3 Endpoint Depth Formula

The surface receives two endpoint depth arguments. Each is computed from the matching endpoint's Z:

```text
surface_depth(endpoint) = 0x0E - ftol(endpoint.z * g_AdjustForZ_Mult
                                      + (endpoint.z >= 0x2D8 ? 1 : 0)
                                      + 0.5)
```

The inline depth setup duplicates `AdjustForZ`'s formula instead of reusing the earlier screen-Y helper result. It computes the second endpoint depth first, then the first endpoint depth, before pushing color, endpoint pointers, and clip rect to the surface call.

Active in YR: Yes. Evidence: endpoint Z threshold setup `0x006DBC2C..0x006DBC5E`; first inline `FILD/FMUL/FIADD/FADD/ftol` sequence and `0x0E - result` push at `0x006DBC62..0x006DBC99`; second sequence and push at `0x006DBC88..0x006DBCB8`; surface call at `0x006DBCC0..0x006DBCC7`.

### 3.4 Final Argument Is Z-Write Flag

The caller's final `DrawLine3D` argument is pushed unchanged to the surface before the two depth arguments. For building brackets that value is `0`.

At `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30`, the surface still performs the Z-test, but the Z-buffer write is guarded by the final byte flag. Prior surface-slot audit confirms guarded writes at the three raster branches: `0x004C024F`, `0x004C043A`, and `0x004C062B`.

Active in YR: Yes. Evidence: final flag forwarded at `0x006DBC70`; helper pushes `0` at `0x006F5FCE` and `0x006F5FED`; direct `DrawExtras` pushes `0` at `0x006F5762`, `0x006F58B1`, `0x006F59D3`.

## 4. Building Bracket Color Argument

Both `DrawBehind` and `DrawExtras` use the same palette-index choice before drawing building brackets:

```text
palette_index = 0x0F
if vtable+0x1C8 result < -4:
    palette_index = 0x0C
```

The palette index is converted through `g_PaletteData + 0x174`: if `*(g_PaletteData+4) == 1`, one byte is read; otherwise a 16-bit value at `index * 2` is read. The converted value is passed as the `DrawLine3D` color argument and then forwarded to the surface line draw.

Active in YR: Yes for normal `0x0F` selected-building lines; Conditional for `0x0C` only when the height-like virtual `+0x1C8` return is below `-4`. Evidence: `DrawExtras @ 0x006F53AD..0x006F542F`; `DrawBehind @ 0x006F6109..0x006F6186`; color forwarded by `DrawLine3D @ 0x006DBCB9`.

## 5. Endpoint Feeding From Building Bracket Call Sites

### 5.1 Shared Helper: DrawBracketCorner

`TechnoClass::DrawBracketCorner @ 0x006F5EF0` receives an edge's two endpoints `A` and `B` plus the converted color. It computes:

```text
Q_A = trunc_to_zero((3*A + B) / 4)
Q_B = trunc_to_zero((A + 3*B) / 4)
```

The divide-by-4 uses signed truncation toward zero (`CDQ; AND 3; ADD; SAR 2`), not floor. Active in YR: Yes. Evidence: `0x006F5EFB..0x006F5F40` and `0x006F5F47..0x006F5FA0`.

For each 25% stub, the helper passes the greater-or-equal Z endpoint first:

```text
if A.z > Q_A.z: DrawLine3D(A, Q_A, color, 0)
else:           DrawLine3D(Q_A, A, color, 0)

if B.z > Q_B.z: DrawLine3D(B, Q_B, color, 0)
else:           DrawLine3D(Q_B, B, color, 0)
```

Active in YR: Yes. Evidence: first compare/call setup `0x006F5FC2..0x006F5FE3`; second compare/call setup `0x006F5FE6..0x006F6015`.

### 5.2 DrawBehind Back Brackets

`TechnoClass::DrawBehind @ 0x006F60D0` is active when `WhatAmI()==6` and selected byte `this+0x83` is nonzero; it rejects `WhatAmI()==0x0F` before geometry. It computes dimensions via `vtable+0x84` then type `+0x7C`, reads object coords through `vtable+0x48`, converts the bracket color, and calls `DrawBracketCorner` five times.

The five helper-fed edges are the back/left selected-building bracket edges documented by the prior topology report:

| Edge | Endpoint pair fed to helper | Evidence | Active in YR |
|---|---|---|---|
| Back-left vertical | `BL ground -> BL roof` | helper call `0x006F623C` | Yes |
| Back ground | `BR ground -> BL ground` | helper call `0x006F62D8` | Yes |
| Left ground | `BL ground -> FL ground` | helper call `0x006F6378` | Yes |
| Left roof | `FL roof -> BL roof` | helper call `0x006F6406` | Yes |
| Back roof | `BR roof -> BL roof` | helper call `0x006F648F` | Yes |

No direct `DrawLine3D` calls were found in `DrawBehind`; all selected-building bracket endpoints go through the helper. Active in YR: Yes. Evidence: `0x006F60D0..0x006F649B`.

### 5.3 DrawExtras Front Brackets

`TechnoClass::DrawExtras @ 0x006F5190` enters the selected building bracket block when entry byte `this+0x3CD` is zero, selected byte `this+0x83` is nonzero, and `WhatAmI()==6`. It repeats the dimension/coordinate/color setup and first calls `DrawBracketCorner` four times for the front/right helper-fed edges:

| Edge | Endpoint pair fed to helper | Evidence | Active in YR |
|---|---|---|---|
| Front ground | `FL ground -> FR ground` | selected-building helper block after `0x006F5458` | Yes |
| Right ground | `BR ground -> FR ground` | same block | Yes |
| Front-left vertical | `FL roof -> FL ground` | same block | Yes |
| Back-right vertical | `BR roof -> BR ground` | same block | Yes |

After the gated `vtable+0x448` hook, stock `BuildingClass` reaches three direct `DrawLine3D` calls. Each direct call computes a single quarter-point with the same 3:1 weighted formula via three `CoordStruct::VecAdd @ 0x006CE240` calls and `CoordStruct::VecDiv(..., 4) @ 0x00710700`, compares Z, passes the greater-or-equal Z endpoint first, forwards the converted color, and pushes final flag `0`.

| Direct stub | Formula / endpoint feed | Evidence | Active in YR |
|---|---|---|---|
| `FL roof` toward hidden `FR roof` | visible endpoint plus 25% quarter point; high-Z first | `0x006F5746..0x006F57A0`; final flag push `0x006F5762` | Yes |
| `BR roof` toward hidden `FR roof` | visible endpoint plus 25% quarter point; high-Z first | `0x006F5873..0x006F58CD`; final flag push `0x006F58B1` | Yes |
| `FR ground` toward `FR roof` | visible endpoint plus 25% quarter point; high-Z first | `0x006F5995..0x006F59F6`; final flag push `0x006F59D3` | Yes |

Active in YR: Yes for selected buildings. Evidence: `BuildingClass::WhatAmI @ 0x00459EC0` returns `6`; the standard render loop calls `DrawBehind` and `DrawExtras` for visible selected buildings (`Tactical_ObjectRenderingLoop @ 0x006D8DB0`, per prior interleaving report).

## 6. INI Keys

No INI key directly controls `DrawLine3D`'s endpoint depth formula, screen-Y Z subtraction, line final flag, or endpoint ordering. `Foundation` and `Height` affect upstream dimensions through `BuildingTypeClass::Dimension2`, but this slot did not re-investigate the parser or foundation tables.

| Key | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|
| `Foundation` | upstream source for width/depth endpoints, consumed through type `vtable+0x7C` | callers `0x006F53CD..0x006F53DF`, `0x006F6122..0x006F6131` | Yes upstream |
| `Height` | upstream source for roof Z extent, consumed through type `vtable+0x7C` | same as above | Yes upstream |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Tactical::DrawLine3D @ 0x006DBB60` projection | verified | `0x006DBB6D..0x006DBC24`, `0x006D1EB0` | none |
| screen-Y Z adjustment | verified | `0x006D20E0..0x006D211B`, `0x006DBBB5`, `0x006DBC14` | none |
| endpoint depth formula | verified | `0x006DBC2C..0x006DBCB8` | none |
| final Z-write flag forwarding | verified | `0x006DBC70`; surface guards `0x004C024F`, `0x004C043A`, `0x004C062B` | none |
| bracket color argument | verified | `0x006F53AD..0x006F542F`, `0x006F6109..0x006F6186` | live negative-height content remains out of scope |
| `DrawBracketCorner` endpoint math/order | verified | `0x006F5EFB..0x006F6015` | none |
| `DrawBehind` endpoint feeding | verified | `0x006F60D0..0x006F649B`; helper calls listed above | exact corner labels rely on prior topology naming |
| `DrawExtras` direct endpoint feeding | verified | `0x006F5746..0x006F59F6` | exact corner labels rely on prior topology naming |
| TS legacy gate check | verified | no SpecialFlags/FogOfWar/TS-only gate in scoped bracket blocks; standard `WhatAmI()==6 && selected` path | none |

## 8. Open Questions - Final State

[RESOLVED] OQ-TDL3D-001 - Is the final `DrawLine3D` argument the bracket line depth? No. Endpoint depths are computed internally from endpoint Z; the final caller argument is forwarded as the surface Z-write flag. Evidence: `0x006DBC62..0x006DBCB8`, `0x006DBC70`.

[RESOLVED] OQ-TDL3D-002 - What is the endpoint depth formula? `0x0E - ftol(z * g_AdjustForZ_Mult + (z >= 0x2D8 ? 1 : 0) + 0.5)`. Evidence: `0x006DBC2C..0x006DBCB8`.

[RESOLVED] OQ-TDL3D-003 - Does screen Y use the same Z adjustment? Yes; each endpoint subtracts `AdjustForZ(endpoint.z)` before viewport Y subtraction. Evidence: `0x006DBB96..0x006DBBC1`, `0x006DBBF9..0x006DBC24`, helper formula `0x006D20E0..0x006D211B`.

[RESOLVED] OQ-TDL3D-004 - What color do bracket lines pass? Converted palette index `0x0F`, or `0x0C` when `vtable+0x1C8 < -4`, passed unchanged to `DrawLine3D`. Evidence: `0x006F53AD..0x006F542F`, `0x006F6109..0x006F6186`.

[RESOLVED] OQ-TDL3D-005 - Do selected building bracket calls Z-write? No; every scoped helper/direct building bracket call pushes final flag `0`. Evidence: `0x006F5FCE`, `0x006F5FED`, `0x006F5762`, `0x006F58B1`, `0x006F59D3`.

[RESOLVED] OQ-TDL3D-006 - Is this path active in standard YR? Yes for selected visible buildings. Evidence: `DrawBehind`/`DrawExtras` selected building gates at `0x006F60D0` and `0x006F5190`, `BuildingClass::WhatAmI @ 0x00459EC0`, render loop reachability from prior `0x006D8DB0` reports.

[DEFERRED] OQ-TDL3D-007 - Which stock runtime object states, if any, make `vtable+0x1C8 < -4` for a selected building? Category: out-of-scope. The branch and color argument are verified, but runtime content/state reachability of the dim color was not part of this slot.

## Sources

- Ghidra decompile/assembly: `Tactical::DrawLine3D @ 0x006DBB60`
- Ghidra decompile/assembly: `Tactical::WorldToScreenSub @ 0x006D1EB0`
- Ghidra decompile/assembly: `Tactical::AdjustForZ @ 0x006D20E0`
- Ghidra decompile/assembly: `TechnoClass::DrawBracketCorner @ 0x006F5EF0`
- Ghidra decompile/assembly: `TechnoClass::DrawBehind @ 0x006F60D0`
- Ghidra decompile/assembly: `TechnoClass::DrawExtras @ 0x006F5190`
- Ghidra decompile: `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30`
- Prior docs cross-checked: `DRAWBRACKETCORNER_DRAWLINE3D_STUB_RASTER_GHIDRA_REPORT.md`, `BUILDING_BRACKET_ABUFFER_ZTEST_DEPTH_SEMANTICS_GHIDRA_REPORT.md`, `TECHNO_DRAWBEHIND_BUILDING_BRACKET_EDGES_GHIDRA_REPORT.md`, `TECHNO_DRAWEXTRAS_BUILDING_BRACKET_BLOCK_GHIDRA_REPORT.md`
