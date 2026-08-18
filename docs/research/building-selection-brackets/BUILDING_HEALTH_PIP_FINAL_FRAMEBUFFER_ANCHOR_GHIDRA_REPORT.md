# Building Health Pip Final Framebuffer Anchor - Ghidra Report

Target: PIPS.SHP health-pip final framebuffer anchor.

Scope: inspect `PIPS.SHP` frame canvas/centering and `CC_Draw_Shape` behavior to convert building health-pip draw points into final framebuffer pixel positions for frames `0`, `1`, `2`, and `4`.

Non-scope: Rust implementation changes, INI edits, screenshot capture, exact transparent/nontransparent pixel mask inside the 10x7 frame rect.

Status: COMPLETE.

Active in YR: Yes, when the standard selected/hover health path reaches `TechnoClass::DrawHealthBar` for a building. Evidence: `TechnoClass::DrawExtras @ 0x006F5190` calls vtable `+0x44C`, and `TechnoClass::DrawHealthBar @ 0x006F64A0` draws `PIPS.SHP @ 0x00AC147C` when `WhatAmI()==6`.

## Verified Binary Evidence

`TechnoClass::DrawHealthBar @ 0x006F64A0` building branch computes each health-pip draw point and calls:

```text
0x006F66BA filled pips: CC_Draw_Shape(PIPS.SHP, frame 1/2/4, draw_point, bounds, flags 0x600, ..., 1000, ...)
0x006F675F empty pips:  CC_Draw_Shape(PIPS.SHP, frame 0,     draw_point, bounds, flags 0x600, ..., 1000, ...)
```

The draw-point sequence is:

```text
D_i.x = pLocation.x + A.x + 3 + N*4 - i*4
D_i.y = pLocation.y + A.y + 4 - N*2 + i*2
```

where `A = CoordsToClient((-half_width, +half_height, z))`, `N = total_pips`, and `i` is the pip index from left/top sequence order.

`CC_Draw_Shape @ 0x004AED70` handles the flags and SHP frame rect as follows:

1. It reads the per-frame rect via `SHP_frame_rect_getter @ 0x0069E7E0`.
2. If flag `0x200` is set, it subtracts `canvas_width/2` and `canvas_height/2` from the caller position.
3. It then adds the frame rect `x`/`y` offset before dispatching to the SHP blitter.

Therefore `0x600` is not a top-left draw. It includes `0x200`, so the caller's point is the SHP canvas-center anchor. The final frame-rect top-left is:

```text
final_x = draw_x - floor(canvas_width / 2) + frame_x
final_y = draw_y - floor(canvas_height / 2) + frame_y
```

`SHP_frame_rect_getter @ 0x0069E7E0` reads frame rect fields as signed shorts from the SHP header (`frame_x`, `frame_y`, `frame_width`, `frame_height`) at `shp + 8 + frame*0x18`.

## Verified Asset Evidence

Retail `PIPS.SHP` metadata from the existing retail asset load log:

Source: `<local>/Documents/ra2-engine-research/logs/bridge_stderr.txt`, lines around the `Pip atlas source` block.

```text
Pip atlas source: pips.shp (16x16, 21 frames)
pip frame  0: pos=(3,5) size=10x7 pixels=70
pip frame  1: pos=(3,5) size=10x7 pixels=70
pip frame  2: pos=(3,5) size=10x7 pixels=70
pip frame  4: pos=(3,5) size=10x7 pixels=70
```

All scoped health-pip frames share the same canvas and frame rect:

| PIPS frame | Use | Canvas | Frame rect offset | Frame rect size | Active in YR |
|---:|---|---:|---:|---:|---|
| `0` | empty building pip | `16x16` | `(3,5)` | `10x7` | Yes |
| `1` | green filled building pip | `16x16` | `(3,5)` | `10x7` | Yes |
| `2` | yellow filled building pip | `16x16` | `(3,5)` | `10x7` | Yes |
| `4` | red filled building pip | `16x16` | `(3,5)` | `10x7` | Yes |

For these frames:

```text
canvas_center = (8, 8)
frame_offset  = (3, 5)
final_frame_top_left_delta_from_draw_point = (3 - 8, 5 - 8) = (-5, -3)
```

So every scoped building health pip frame blits its 10x7 frame rectangle at:

```text
final_rect_top_left = draw_point + (-5, -3)
final_rect_extent   = x [draw_x - 5, draw_x + 4], y [draw_y - 3, draw_y + 3]
```

## Final Framebuffer Formula

For pip index `i` in the building health-pip sequence:

```text
draw_i.x = pLocation.x + A.x + 3 + N*4 - i*4
draw_i.y = pLocation.y + A.y + 4 - N*2 + i*2

frame_rect_i.x = pLocation.x + A.x - 2 + N*4 - i*4
frame_rect_i.y = pLocation.y + A.y + 1 - N*2 + i*2
```

The final formula is identical for frames `0`, `1`, `2`, and `4`; only palette pixels differ by frame content. The final step between adjacent pips remains `(-4,+2)`.

Using the concrete draw-point cases from `BUILDING_HEALTH_PIP_VISUAL_ANCHOR_CASES_GHIDRA_REPORT.md`:

| Case | First pip draw point from NW screen point `(sx,sy)` | First frame rect top-left | Last frame rect top-left |
|---|---|---|---|
| `GACNST`, 4x4, `Height=4`, `N=30` | `(sx+3, sy-ZAdj(4)-11)` | `(sx-2, sy-ZAdj(4)-14)` | `(sx-118, sy-ZAdj(4)+44)` |
| `TESLA/NATSLA`, 1x1, `Height=5`, `N=7` | `(sx+1, sy-ZAdj(5)-10)` | `(sx-4, sy-ZAdj(5)-13)` | `(sx-28, sy-ZAdj(5)-1)` |
| `GAREFN`, 4x3, `Height=4`, `N=22` | `(sx+1, sy-ZAdj(4)-10)` | `(sx-4, sy-ZAdj(4)-13)` | `(sx-88, sy-ZAdj(4)+29)` |

If using the simplified initialized-YR assumption `ZAdj(H)=H*15`, these become:

| Case | First frame rect top-left |
|---|---|
| `GACNST` | `(sx-2, sy-74)` |
| `TESLA/NATSLA` | `(sx-4, sy-88)` |
| `GAREFN` | `(sx-4, sy-73)` |

## Interpretation

Verified:

- The building health-pip caller's draw point is not the visible frame top-left.
- The draw point is a centered SHP canvas anchor because flags `0x600` include `0x200`.
- The scoped frames are affected by SHP frame offsets, but frames `0`, `1`, `2`, and `4` have identical offsets and dimensions.
- The final framebuffer frame-rect top-left is always `draw_point + (-5,-3)` for these four frames.

Inference:

- Treating the 10x7 frame rect as the "final visible pip rectangle" is correct for frame placement, but this pass did not inspect per-pixel transparency inside that 10x7 rect. If exact first opaque pixel per frame is needed, decode the four frame pixel masks.

## Open Questions

- Do frames `0`, `1`, `2`, and `4` contain transparent pixels inside the 10x7 rect that would make the first opaque pixel differ from the frame-rect top-left? This is a pixel-mask question, not an anchor question.
- The previous report's `ZAdj(H)=H*15` simplification still depends on initialized runtime tactical globals; this slot did not live-debug those globals.

## Sources

- Ghidra decompile: `TechnoClass::DrawHealthBar @ 0x006F64A0`.
- Ghidra decompile: `CC_Draw_Shape @ 0x004AED70`.
- Ghidra decompile: `SHP_frame_rect_getter @ 0x0069E7E0`, `SHP_frame_data_getter @ 0x0069E740`, `SHP_frame_flag_check @ 0x0069E900`.
- Retail asset metadata log: `<local>/Documents/ra2-engine-research/logs/bridge_stderr.txt` (`Pip atlas source: pips.shp (16x16, 21 frames)` and frame rows).
- Prior scoped reports: `TECHNO_DRAWHEALTHBAR_BUILDING_BRANCH_GHIDRA_REPORT.md`, `BUILDING_HEALTH_PIP_VISUAL_ANCHOR_CASES_GHIDRA_REPORT.md`.
