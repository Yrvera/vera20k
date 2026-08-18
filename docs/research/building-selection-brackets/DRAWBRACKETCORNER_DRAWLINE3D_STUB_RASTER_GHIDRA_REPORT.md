# DrawBracketCorner / DrawLine3D Stub Raster - Ghidra Research Report

**Address(es):** `0x006F5EF0` (`TechnoClass::DrawBracketCorner`), `0x006DBB60` (`Tactical::DrawLine3D`), `0x004BFD30` (`Surface::DrawLine_ABufModulated_ZClipped`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** quarter-stub math, endpoint ordering, projection, clipping, depth arguments, and 1-pixel raster behavior for building selection bracket uses only.  
**Non-Scope:** full `DrawExtras` edge topology, health pips, target/rally/action lines outside building brackets, palette ownership beyond the passed line color.  
**Confidence:** High for the two primary functions and `Surface` vtable binding; Medium for exact subpixel camera constants because runtime BSS multiplier value is initialized outside this slice.  
**Active in YR:** Yes. Evidence: `DrawBracketCorner` callers are exactly `TechnoClass::DrawBehind @ 0x006F60D0` and `TechnoClass::DrawExtras @ 0x006F5190`; both gate building bracket work on `WhatAmI()==6` and selected byte `this+0x83`.

## 1. Overview

Building selection brackets are short 3D line stubs projected into tactical screen space. `DrawBracketCorner` draws two 25% stubs on one edge; `DrawExtras` also computes three single stubs manually with the same 3:1 weighted formula before calling `Tactical::DrawLine3D`.

The line ultimately lands on `g_PrimarySurface` through DSurface vtable slot `+0x34`, which resolves to `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30`. It is a clipped Bresenham-style 1-pixel line with A-buffer modulation and Z-test, but building bracket calls pass `0` for the final Z-write flag.

Follow-up `SURFACE_DRAW_LINE_BRACKET_RASTER_GHIDRA_REPORT.md` verifies the exact
surface contract: clipping is left/top inclusive and right/bottom clipped to
`x + width - 1` / `y + height - 1`; rasterization writes the start pixel and excludes
the final endpoint; integer stepping chooses among x-, y-, and depth-dominant paths.
Rust parity should therefore emit the clipped integer pixel set first, then render
those pixels as 1x1 quads.

## 2. Key Offsets / Globals

| Item | Address / offset | Meaning | Active in YR |
|---|---:|---|---|
| `g_Tactical` | `0x00887324` | Tactical instance; vtable `+0x60` calls `DrawLine3D` | Yes - used directly by bracket helpers |
| `g_PrimarySurface` | `0x00887314` | primary surface used by `DrawLine3D` | Yes - `DrawLine3D @ 0x006DBB60` loads it |
| DSurface vtable | `0x007E85D4` | vtable slot `+0x34` = `0x004BFD30` | Yes - raw vtable memory at `0x007E8608` |
| Tactical viewport X/Y | `this+0xB0`, `this+0xB4` | subtracted after projection | Yes - `DrawLine3D @ 0x006DBB60` |
| Clip rect argument | `0x00886FA0` | viewport rect passed to surface line draw | Yes - pushed by `DrawLine3D @ 0x006DBCC0` |
| Z-buffer | `0x00887644` | surface line Z-test source | Yes when non-null - `0x004C0003` guard |
| A-buffer | `0x0087E8A4` | shroud/fog modulation mask | Yes - sampled in line loops |

## 3. Core Logic

### 3.1 `DrawBracketCorner @ 0x006F5EF0`

For input endpoints `A` and `B`, the helper computes two quarter points:

```text
Q_A = trunc_to_zero((3*A + B) / 4)
Q_B = trunc_to_zero((A + 3*B) / 4)
```

Evidence: assembly `0x006F5EFB..0x006F5F40` computes `3*A + B` per component, then uses `CDQ; AND EDX,3; ADD; SAR 2` before `CoordStruct::Set @ 0x0041C230`. The same pattern repeats at `0x006F5F47..0x006F5FA0` for `A + 3*B`.

Rounding detail: the divide-by-4 is signed truncation toward zero, not mathematical floor. For non-negative building bracket coords it matches floor; for negative intermediate sums it differs from arithmetic shift by adding `3` before `SAR 2`. Active in YR: Yes - this helper is the live building-bracket helper from both building bracket phases.

### 3.2 Endpoint Ordering

`DrawBracketCorner` orders each line so the first `DrawLine3D` coordinate has the greater-or-equal Z:

```text
if A.z > Q_A.z: DrawLine3D(A, Q_A, color, 0)
else:           DrawLine3D(Q_A, A, color, 0)

if B.z > Q_B.z: DrawLine3D(B, Q_B, color, 0)
else:           DrawLine3D(Q_B, B, color, 0)
```

Evidence: first segment compare at `0x006F5FC2`; the `JLE` path passes `(Q_A, A)`, otherwise `(A, Q_A)`. Second segment compare at `0x006F5FEF`; the `JLE` path passes `(Q_B, B)`, otherwise `(B, Q_B)`.

This corrects the older shorthand that said "lower Z first." The live code passes higher Z first at the helper boundary. Active in YR: Yes - both `DrawBehind` and `DrawExtras` call this helper for buildings.

### 3.3 Manual Single-Stubs In `DrawExtras`

The building branch in `DrawExtras` repeats the same 3:1 weighted quarter formula using three `CoordStruct::VecAdd @ 0x006CE240` calls followed by `CoordStruct::VecDiv(..., 4) @ 0x00710700`, then calls `Tactical::DrawLine3D` directly with final arg `0`.

Evidence call groups:
- `0x006F5746..0x006F57A0`
- `0x006F5873..0x006F58CD`
- `0x006F5995..0x006F59F6`
- related building-path repetitions at `0x006F5ACE..0x006F5D77`

These direct calls use the same Z ordering rule: compare the visible endpoint Z against the quarter-point Z, pass the greater-or-equal Z endpoint first. Active in YR: Yes - inside the selected-building `WhatAmI()==6` branch of `DrawExtras`.

### 3.4 `Tactical::DrawLine3D @ 0x006DBB60`

`DrawLine3D` projects each endpoint as:

```text
sub_x = x*30 - y*30
sub_y = x*15 + y*15
screen_x = trunc_to_zero(sub_x / 256) - Tactical.viewport_x
screen_y = trunc_to_zero(sub_y / 256) - AdjustForZ(z) - Tactical.viewport_y
```

Evidence: `WorldToScreenSub @ 0x006D1EB0` computes `x*0x3C/2 + y*-0x3C/2` and `x*0x1E/2 + y*0x1E/2`; `DrawLine3D @ 0x006DBB80..0x006DBBC3` and `0x006DBDDF..0x006DBC24` apply signed `/256` with the same add-bias-before-`SAR 8` truncation and subtract `this+0xB0/+0xB4`.

For each endpoint, surface depth is `14 - AdjustForZ(endpoint.z)`. `DrawLine3D` passes both endpoint depths plus the caller's final argument as the surface routine's final boolean. Building brackets pass `0`, so they Z-test but do not write new Z.

Evidence: `DrawLine3D @ 0x006DBC62..0x006DBCBF` computes two `0xE - ftol(z * g_AdjustForZ_Mult + threshold + 0.5)` values, pushes color, endpoint pointers, rect `0x00886FA0`, and the incoming final arg; `Surface @ 0x004BFD30` returns with `RET 0x1C`, confirming seven stack args after `this`.

Active in YR: Yes - `g_Tactical` vtable entry `0x007F43A8` points at `0x006DBB60`; building bracket helpers call `g_Tactical->vtable+0x60`.

## 4. INI Keys

No INI key controls the stub fraction, line thickness, clipping mode, Z-write flag, or the surface rasterizer. Foundation/Height/color selection are upstream and outside this slice.

| Key | Effect on this slice | Active in YR |
|---|---|---|
| `Foundation` / `Height` | Determines endpoints before this helper sees them; not read by these functions | Conditional - active upstream |
| `PixelSelectionBracketDelta` | Not consumed by `DrawBracketCorner` or `DrawLine3D` | No for this slice |

## 5. Integration Points

| Function / site | Role | Active in YR |
|---|---|---|
| `TechnoClass::DrawBehind @ 0x006F60D0` | Calls `DrawBracketCorner` for back/behind-sprite building bracket edges | Yes - `WhatAmI()==6 && selected` |
| `TechnoClass::DrawExtras @ 0x006F5190` | Calls `DrawBracketCorner` and direct `DrawLine3D` for front/visible building bracket stubs | Yes - `this+0x3CD==0`, `this+0x83!=0`, `WhatAmI()==6` |
| `TechnoClass::DrawBracketCorner @ 0x006F5EF0` | Draws two 25% stubs with high-Z-first endpoint order | Yes |
| `Tactical::DrawLine3D @ 0x006DBB60` | Projects 3D endpoints and delegates to primary-surface line raster | Yes |
| `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30` | Clips and rasterizes one-pixel line | Yes when primary surface vtable is active |

## 6. Current Rust Implementation Status

Not investigated in this slot by request. Prior docs mention `src/app_selection_brackets.rs`, but this report did not inspect or modify Rust.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `DrawBracketCorner` quarter math | verified | `0x006F5EFB..0x006F5FA0` | none |
| signed divide rounding | verified | `CDQ; AND 3; ADD; SAR 2` at `0x006F5F1A..0x006F5F3C` and repeats | none |
| helper endpoint ordering | verified | compares/call setup at `0x006F5FC2..0x006F600F` | none |
| direct building single-stub calls in `DrawExtras` | verified | `0x006F5746..0x006F57A0`, `0x006F5873..0x006F58CD`, `0x006F5995..0x006F59F6` | exact edge naming is owned by topology reports |
| `DrawLine3D` projection | verified | `0x006DBB6D..0x006DBC24`, `WorldToScreenSub @ 0x006D1EB0` | none |
| `DrawLine3D` depth args | verified | `0x006DBC62..0x006DBCBF`, `Surface RET 0x1C` | none |
| DSurface vtable binding | verified | raw vtable `0x007E85D4 + 0x34 = 0x004BFD30` | none |
| line clipping | verified | `Surface @ 0x004BFD42..0x004BFE06` calls `ClipRect` then clip helper `0x007BC2B0` | none |
| line thickness | verified | `Surface @ 0x004C01C0`, `0x004C03AE`, `0x004C058B` loops perform one 16-bit destination write per Bresenham step | none |
| Z-write flag for building brackets | verified | bracket callsites push final arg `0`; `Surface @ 0x004C024F..0x004C0265` writes Z only if final byte arg nonzero | none |

## 8. Open Questions - Final State

[RESOLVED] Q1 - Is the stub exactly 25%? Yes: weighted points are `(3*A+B)/4` and `(A+3*B)/4`. Evidence: `0x006F5EFB..0x006F5FA0`.

[RESOLVED] Q2 - Does divide round or truncate? It truncates toward zero via signed bias before `SAR`. Evidence: `0x006F5F1A..0x006F5F3C`.

[RESOLVED] Q3 - Which endpoint is passed first? The greater-or-equal Z endpoint is passed as first coordinate. Evidence: `0x006F5FC2..0x006F600F`.

[RESOLVED] Q4 - Does `DrawLine3D` clip itself? It delegates clipping to `Surface::DrawLine_ABufModulated_ZClipped` through the primary surface vtable. Evidence: `0x006DBCC0..0x006DBCC7`, `0x004BFD42..0x004BFE06`.

[RESOLVED] Q5 - Is the line exactly 1 pixel? Yes, one 16-bit destination pixel write per raster step, no adjacent-pixel thickness loop in the active raster branches. Evidence: `0x004C01C0`, `0x004C03AE`, `0x004C058B`.

[RESOLVED] Q6 - Do building bracket lines write Z? No. The final arg is `0` at helper and direct building callsites; surface writes Z only when that byte is nonzero. Evidence: `0x006F5FCE`, `0x006F5FED`, direct sites including `0x006F5762`, `0x004C024F..0x004C0265`.

[DEFERRED] Q7 - Exact runtime value of `g_AdjustForZ_Mult @ 0x00B0CD48`. Category: out-of-scope. Existing coordinate reports cover initialization; this slice only needed the formula and call use.

## Sources

- Ghidra decompile/disassembly: `0x006F5EF0`, `0x006DBB60`, `0x006F5190`, `0x006F60D0`, `0x006D1EB0`, `0x006D20E0`, `0x004BFD30`
- Ghidra vtable memory: `0x007E85D4..0x007E8624`
- Prior docs used as seeds/cross-checks: `SELECTION_BRACKETS_GHIDRA_REPORT.md`, `SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md`, `VFX_RENDER_SUBSTRATE_AUDIT.md`, `COORDINATE_SYSTEM_GAMEMD.md`
