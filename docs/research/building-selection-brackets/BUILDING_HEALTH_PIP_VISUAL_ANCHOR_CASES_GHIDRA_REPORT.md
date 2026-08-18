# Building Health-Pip Visual Anchor Cases - Ghidra Report

**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** building health-pip anchor and rounding for GACNST, requested NATESLA, and GAREFN using `gamemd.exe` draw path plus retail INI data.  
**Non-Scope:** Rust implementation, runtime screenshot capture, SHP frame internal canvas offsets, selection line bracket geometry.  
**Confidence:** HIGH for binary formula and INI inputs; MEDIUM for the final `AdjustForZ(Height*g_HeightFactor) = Height*15` collapse because this report did not live-debug initialized tactical globals.  
**Active in YR:** Yes for GACNST and GAREFN; Conditional for requested NATESLA because stock rules use `[TESLA]` with `Image=NATSLA`, not a `[NATESLA]` section.

## 1. Verified Binary Evidence

`TechnoClass::DrawHealthBar` at `0x006F64A0` is the active health-pip renderer. Its building branch starts when `WhatAmI()` returns `6`; it calls `GetHeight()` for side effects, then `GetType()` and `BuildingTypeClass::Dimension2` via type vtable `+0x7C`.

`BuildingTypeClass::Dimension2` at `0x00464AF0` returns:

```text
+0 = g_FoundationWidthTable[Type+0xEF0] << 8
+4 = g_FoundationHeightTable[Type+0xEF0] << 8
+8 = Type+0xEF4 Height * g_HeightFactor
```

The building pip count and first draw point are computed in `0x006F64A0`:

```text
left = CoordsToClient(-width/2, +height/2, z)
top  = CoordsToClient(-width/2, -height/2, z)
numPips = (left.y - top.y) / 2       // signed integer truncation
pip0.x = pLocation.x + left.x + 3 + numPips * 4
pip0.y = pLocation.y + left.y + 4 - numPips * 2
step = (-4, +2)
```

Filled pips are drawn first; empty pips continue from the same sequence. The draw calls use `PIPS.SHP` via `DAT_00AC147C`, z/flags argument `0x600`; filled frames are `1` green, `2` yellow, `4` red, and empty frame is `0`.

`ObjectClass::GetHealthRatio` at `0x005F5C60` returns `Health / Type.Strength`; filled count is `ftol(healthRatio * numPips)`, then clamped to `[1, numPips]`. `ObjectClass::IsYellowHP` at `0x005F5D20` is true only when `ConditionRed < ratio <= ConditionYellow`; `ObjectClass::IsRedHP` at `0x005F5CD0` is true when `ratio <= ConditionRed && Health > 0`.

`BuildingClass::GetCoords` at `0x00447AC0` returns the building foundation center from the placed/NW object coordinate:

```text
pLocationWorld.x = object.x + foundationWidth * 128 - 128
pLocationWorld.y = object.y + foundationHeight * 128 - 128
pLocationWorld.z = object.z
```

`CoordsToClient` at `0x006D1F10` converts lepton offsets with signed truncation toward zero:

```text
screen.x = trunc_toward_zero((x*30 - y*30) / 256)
screen.y = trunc_toward_zero((x*15 + y*15) / 256) - Tactical_AdjustForZ(z)
```

`Tactical_AdjustForZ` at `0x006D20E0` multiplies Z by runtime `DAT_00B0CD48`, adds `1` if `z >= 0x2D8`, adds `0.5` (`0x007E1738`), then `ftol`s. The known simplified building formula assumes the initialized YR tactical globals collapse `AdjustForZ(Height*g_HeightFactor)` to `Height*15`; this report keeps the exact binary expression and lists the simplified visible offsets separately.

## 2. Retail Data Inputs

YR `artmd.ini` repeats or patches the base art data and takes priority:

| Requested case | Rules identity | Art identity | artmd.ini evidence | Foundation | Height | Active in YR |
|---|---|---|---|---:|---:|---|
| GACNST | `[GACNST]` | `[GACNST]` | `artmd.ini:1599-1602` | 4x4 | 4 | Yes |
| NATESLA | no stock `[NATESLA]` found | `[NATSLA]` via `[TESLA] Image=NATSLA` | `rulesmd.ini:12930-12938`, `artmd.ini:4441-4446` | 1x1 | 5 | Conditional: Yes if request means Soviet Tesla Coil `[TESLA]`/`NATSLA`; No for literal `[NATESLA]` |
| GAREFN | `[GAREFN]` | `[GAREFN]` | `artmd.ini:1763-1768` | 4x3 | 4 | Yes |

The foundation width/height lookup tables at `0x008192B8` and `0x00819310` include the required entries for `1x1`, `4x3`, and `4x4`; `Dimension2` consumes those table results directly.

## 3. Exact Anchor Cases

Definitions:

```text
sx, sy = projected screen position of the building's placed/NW object coordinate
ZAdj(H) = Tactical_AdjustForZ(Height * g_HeightFactor)
draw point = point passed to CC_Draw_Shape before PIPS.SHP canvas/frame centering
```

| Case | fw x fh | Height | numPips | First pip draw point from NW (`sx`,`sy`) | Last pip draw point from NW (`sx`,`sy`) | Simplified visible assumption |
|---|---:|---:|---:|---|---|---|
| GACNST | 4x4 | 4 | 30 | `sx + 3, sy - ZAdj(4) - 11` | `sx - 113, sy - ZAdj(4) + 47` | if `ZAdj(4)=60`: first `(sx+3, sy-71)`, last `(sx-113, sy-13)` |
| TESLA/NATSLA | 1x1 | 5 | 7 | `sx + 1, sy - ZAdj(5) - 10` | `sx - 23, sy - ZAdj(5) + 2` | if `ZAdj(5)=75`: first `(sx+1, sy-85)`, last `(sx-23, sy-73)` |
| GAREFN | 4x3 | 4 | 22 | `sx + 1, sy - ZAdj(4) - 10` | `sx - 83, sy - ZAdj(4) + 32` | if `ZAdj(4)=60`: first `(sx+1, sy-70)`, last `(sx-83, sy-28)` |

The familiar formula `pip0 = (sx + 3, sy - 11 - Height*15)` is exact for GACNST's even foundation height but not for NATSLA/TESLA or GAREFN. For odd foundation heights, two integer truncations matter: `numPips = (foundationHeight * 15) / 2` truncates `.5`, and `CoordsToClient` truncates the foundation-center and edge projections toward zero. The visible result is a two-pixel X shift and one-pixel Y shift for the selected odd-height cases above.

## 4. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Building branch of `TechnoClass::DrawHealthBar` | verified | `0x006F64A0`; assembly `0x006F64B1..0x006F677D` | none |
| `BuildingTypeClass::Dimension2` width/height/Z source | verified | `0x00464AF0` | none |
| `BuildingClass::GetCoords` pLocation source | verified | `0x00447AC0` | none |
| `CoordsToClient` signed truncation | verified | `0x006D1F10` | none |
| `Tactical_AdjustForZ` exact runtime collapse | touched-not-exhausted | `0x006D20E0` | live initialized `DAT_00B0CD48` / `g_HeightFactor` were not sampled in this slot |
| GACNST, GAREFN, TESLA/NATSLA INI inputs | verified | `artmd.ini`, `rulesmd.ini` line evidence above | literal `NATESLA` name does not exist in stock INI |
| SHP frame internal canvas offsets | resolved in follow-up | `BUILDING_HEALTH_PIP_FINAL_FRAMEBUFFER_ANCHOR_GHIDRA_REPORT.md` | frame-rect top-left is `draw_point + (-5,-3)` for frames 0/1/2/4 |

## 5. Open Questions - Final State

[RESOLVED] OQ-1 - Does building pip anchor come from sprite bounds or foundation geometry? It comes from `Dimension2`/foundation geometry, not sprite bounds. Evidence: `0x006F64A0`, `0x00464AF0`.

[RESOLVED] OQ-2 - Are GACNST, GAREFN, and Tesla active YR content? GACNST/GAREFN yes; Tesla yes under `[TESLA] Image=NATSLA`; literal `[NATESLA]` no stock section found. Evidence: `rulesmd.ini` / `artmd.ini` searches.

[RESOLVED] OQ-3 - Is odd foundation-height rounding visible? Yes. `numPips` truncates to 7 for 1x1 and 22 for 4x3, producing `sx+1` first-pip X and `sy-ZAdj-10` first-pip Y, not the even-foundation `sx+3` / `sy-ZAdj-11`. Evidence: `0x006F64A0` integer divisions and `0x006D1F10` truncation.

[RESOLVED] OQ-4 - What are the final framebuffer pixel coordinates after PIPS.SHP frame/canvas centering? The follow-up report `BUILDING_HEALTH_PIP_FINAL_FRAMEBUFFER_ANCHOR_GHIDRA_REPORT.md` verifies that frames 0/1/2/4 share a 16x16 canvas, frame offset `(3,5)`, and size 10x7, so the final frame-rect top-left is always `draw_point + (-5,-3)`.

## Sources

- Ghidra decompiled/disassembled: `0x006F64A0`, `0x00464AF0`, `0x00447AC0`, `0x006D1F10`, `0x006D20E0`, `0x005F5C60`, `0x005F5D20`, `0x005F5CD0`.
- Binary data read: foundation width table `0x008192B8`; foundation height table `0x00819310`.
- INI data checked: `ini/artmd.ini`, `ini/art.ini`, `ini/rulesmd.ini`, `ini/rules.ini`.
- Prior related report read for context: `C:/Users/enok/Documents/ra2-rust-game-docs/HEALTH_BAR_POSITIONING.md`.
